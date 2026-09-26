// 分层目录即层名：commands(2) → domain(1) → infra(0)，单向依赖
pub mod commands;
pub mod domain;
pub mod infra;

use tauri::Manager;

/// tauri-specta 命令注册表：单一事实源，同时供 invoke_handler 与 TS 绑定导出使用。
/// 新增命令必须：① `#[specta::specta]` 标注；② 在此注册；③ 重新构建（pnpm dev 或
/// pnpm check 链路）自动复写 src/bindings.ts。
fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        // i64/u64 统一导出为 TS number（本应用值域极小，安全）
        .dangerously_cast_bigints_to_number()
        // 错误走 Promise reject（bindings 返回 Promise<T>），前端 try/catch 即可
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .commands(tauri_specta::collect_commands![
            commands::items::list_items,
            commands::items::add_item,
            commands::items::update_item,
            commands::items::delete_item,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::open_settings,
        ])
}

/// debug 构建启动时自动导出到 ../src/bindings.ts
#[cfg(debug_assertions)]
fn export_bindings(builder: &tauri_specta::Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("导出 TypeScript 绑定失败");
}

/// 供 pnpm check 复写 bindings：环境变量 CDOWN_EXPORT_BINDINGS 触发「导出即退」。
/// 路径锚定 CARGO_MANIFEST_DIR（编译期绝对路径），与进程工作目录无关。
#[cfg(debug_assertions)]
fn export_bindings_standalone(builder: &tauri_specta::Builder<tauri::Wry>) {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts");
    builder
        .export(specta_typescript::Typescript::default(), &out)
        .expect("导出 TypeScript 绑定失败");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = specta_builder();

    // 导出即退模式：pnpm check 直接复写 src/bindings.ts（不建窗口、不连
    // webview、不初始化任何子系统），导出后立即退出。
    #[cfg(debug_assertions)]
    if std::env::var_os("CDOWN_EXPORT_BINDINGS").is_some() {
        export_bindings_standalone(&specta_builder);
        return;
    }

    // debug 启动自动导出 bindings
    #[cfg(debug_assertions)]
    export_bindings(&specta_builder);

    tauri::Builder::default()
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);

            // 存储初始化：<app_config_dir>/cdown.json
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;
            app.manage(infra::store::Store::new(config_dir.join("cdown.json")));

            // 单例：二次启动不出新实例，唤起已有实例（可能正藏在托盘）。
            // 官方要求该插件最先注册。
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.unminimize();
                        let _ = w.set_focus();
                    }
                }))?;

            // 托盘常驻：小组件 skipTaskbar 没有任务栏图标，托盘是唯一出口。
            // 左键单击切换显示/隐藏，右键菜单「显示 / 退出」。
            #[cfg(desktop)]
            {
                use tauri::{
                    menu::{Menu, MenuItem},
                    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
                };

                let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
                let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &settings, &quit])?;

                TrayIconBuilder::with_id("cdown-tray")
                    .icon(app.default_window_icon().expect("缺少应用图标").clone())
                    .tooltip("cdown")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                        // 设置：唤起主窗口（供对照）并打开独立设置窗口。
                        // 菜单事件处理器在主线程，build() 必须丢到独立线程，
                        // 否则 Windows 上死锁（官方文档警告，同 async 命令）。
                        "settings" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                            let settings_app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) =
                                    crate::commands::settings::open_settings_window(&settings_app)
                                {
                                    log::warn!("打开设置窗口失败：{e}");
                                }
                            });
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        // 左键单击按可见性 toggle：隐藏 → 显示并聚焦；可见 → 隐藏。
                        // 不能用 is_focused 参与判断：点击托盘时窗口已先失焦，恒走显示分支。
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                if w.is_visible().unwrap_or(false) {
                                    let _ = w.hide();
                                } else {
                                    let _ = w.show();
                                    let _ = w.unminimize();
                                    let _ = w.set_focus();
                                }
                            }
                        }
                    })
                    .build(app)?;
            }

            // 日志插件仅 debug 构建注册（终端输出）；应用日志量小，release 不落盘
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
