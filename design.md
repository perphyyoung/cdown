# design.md — 设计约定

UI / 交互层面的硬约定，新增功能前先对照；与《cdown起步方案.md》（方案与取舍）互补。

## 交互

- **破坏性操作必须二次确认**：重置（如分级恢复默认）、删除（如删除分级、删除倒计时项）、导入覆盖等会破坏现有数据的操作，执行前必须弹出确认，文案需说明影响的范围；确认后只覆盖确认范围内的数据。
- **确认弹窗必须用自定义 ConfirmDialog**（`src/components/ConfirmDialog.vue`，参考 paim 同名组件）：Tauri 的 WebView2 下原生 `window.confirm` / `window.alert` 不生效，禁止使用。
