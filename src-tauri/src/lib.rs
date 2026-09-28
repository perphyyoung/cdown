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
        .events(tauri_specta::collect_events![
            infra::logging::LogLevelChanged
        ])
        .commands(tauri_specta::collect_commands![
            commands::items::list_items,
            commands::items::add_item,
            commands::items::update_item,
            commands::items::delete_item,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::open_settings,
            commands::data::export_items,
            commands::data::import_items,
            commands::data::export_settings,
            commands::data::import_settings,
            commands::date_picker::open_date_picker,
            commands::date_picker::get_date_picker_payload,
            commands::date_picker::close_date_picker,
            infra::logging::log_msg,
            infra::logging::get_log_level,
            infra::logging::set_log_level,
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

    // 文件日志初始化（级别：CDOWN_LOG > cdown-config.toml > 内置默认）
    infra::logging::init_from_config();
    log_info!(
        "cdown 启动（version {}，debug={}），日志级别 {}",
        env!("PACKAGE_VERSION"),
        cfg!(debug_assertions),
        infra::logging::level_str()
    );

    tauri::Builder::default()
        .plugin({
            // 官方窗口状态插件：窗口创建时自动恢复上次尺寸/位置，退出时自动保存。
            // 排除 VISIBLE：主窗口常隐藏到托盘，可见性不参与持久化（否则托盘态退出后
            // 下次启动窗口不显示）。
            let mut state = tauri_plugin_window_state::Builder::default().with_state_flags(
                tauri_plugin_window_state::StateFlags::all()
                    & !tauri_plugin_window_state::StateFlags::VISIBLE
                    // 边框形态由代码决定（主/设置窗口均无边框自绘），不参与持久化，
                    // 否则插件会把旧的原生边框状态恢复回来
                    & !tauri_plugin_window_state::StateFlags::DECORATIONS,
            );
            // dev/release 状态文件分离：release 用插件默认名（带前置点，插件内硬编码），
            // dev 单独命名，避免两边共享同一份窗口几何
            if cfg!(debug_assertions) {
                state = state.with_filename("window-state.dev.json");
            }
            state.build()
        })
        .plugin(
            // 全局热键插件：注册/注销全在 Rust 侧（见 commands/hotkey.rs），capabilities 无需开权限。
            // 键位不在构建期注册（构建期注册失败会让启动直接失败），而是 setup 里按设置注册；
            // with_handler 是「任意已注册热键被按下」的统一入口。
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    let held = app.state::<commands::hotkey::HotkeyHeld>();
                    match event.state() {
                        // press() 为 true 才是「松手后的新一次按下」，长按的自动重复被忽略
                        ShortcutState::Pressed => {
                            if held.press() {
                                commands::hotkey::toggle_main_window(app);
                            }
                        }
                        ShortcutState::Released => held.release(),
                    }
                })
                .build(),
        )
        .plugin(
            // 开机自启插件：Windows 写 HKCU Run 注册表项，注册表即唯一状态源
            //（不进 Settings/导出导入）；前端设置页开关直调插件 JS API
            tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None),
        )
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);

            // 存储初始化：数据目录分离（dev=<项目根>/cdown-data，release=<app_config_dir>）
            let data_dir = infra::store::data_dir(app.handle());
            std::fs::create_dir_all(&data_dir)?;
            app.manage(infra::store::Store::new(data_dir.join("cdown.json")));
            app.manage(commands::hotkey::RegisteredHotkey::default());
            app.manage(commands::hotkey::HotkeyHeld::default());
            app.manage(commands::date_picker::DatePickerPayload::default());
            // 原生文件对话框（设置页的导出/导入选路径用）
            app.handle().plugin(tauri_plugin_dialog::init())?;

            // 单例：二次启动不出新实例，唤起已有实例（可能正藏在托盘）。
            // 官方要求该插件最先注册。
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                    commands::hotkey::show_main_window(app);
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
                        "show" => commands::hotkey::show_main_window(app),
                        // 设置：唤起主窗口（供对照）并打开独立设置窗口。
                        // 菜单事件处理器在主线程，build() 必须丢到独立线程，
                        // 否则 Windows 上死锁（官方文档警告，同 async 命令）。
                        "settings" => {
                            commands::hotkey::show_main_window(app);
                            let settings_app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) =
                                    crate::commands::settings::open_settings_window(&settings_app)
                                {
                                    log::warn!("打开设置窗口失败：{e}");
                                }
                            });
                        }
                        "quit" => {
                            // 退出前显式保存窗口状态（插件在应用退出时也会自动保存）
                            use tauri_plugin_window_state::{AppHandleExt, StateFlags};
                            let _ = app.save_window_state(
                                StateFlags::all() & !StateFlags::VISIBLE & !StateFlags::DECORATIONS,
                            );
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        // 左键单击按可见性 toggle：隐藏 → 显示并聚焦；可见 → 隐藏。
                        // 不能用 is_focused 参与判断：点击托盘时窗口已先失焦，恒走显示分支。
                        // （热键的切换判定不同，见 commands::hotkey::toggle_main_window）
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                if w.is_visible().unwrap_or(false) {
                                    commands::hotkey::hide_main_window(app);
                                } else {
                                    commands::hotkey::show_main_window(app);
                                }
                            }
                        }
                    })
                    .build(app)?;
            }

            // 主窗口配置为 visible:false（避免几何恢复前的尺寸闪变），此处亮相
            let main_window = app.get_webview_window("main").expect("主窗口不存在");
            // 置顶偏好存 Settings（标题栏图钉切换），conf 的 alwaysOnTop:true 仅为
            // 首次启动兜底；此处以 Settings 为准。读取失败按默认设置处理（与 conf 一致）
            let settings = app
                .state::<infra::store::Store>()
                .read()
                .map(|d| d.settings.clone())
                .unwrap_or_default();
            let _ = main_window.set_always_on_top(settings.always_on_top);
            let _ = main_window.show();

            // 全局热键：按设置注册（键被别的程序占用时只记日志，不影响启动）
            if let Err(e) = commands::hotkey::apply(app.handle(), settings.hotkey.as_deref()) {
                log_warn!("全局热键注册失败：{e}");
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
