# HydCraft Updater 项目约定

## UI 技术栈与视觉风格

- UI 统一使用 Nuxt UI 组件和 Tailwind CSS，不要为已有控件重新手写等价组件。
- 优先使用 `UButton`、`USelect`、`UPopover`、`UIcon`、`USkeleton` 等 Nuxt UI 组件；只有窗口拖动区域、纯布局容器和确实需要原生语义的元素才使用原生 HTML。
- 页面以 slate 色系作为背景、边框和文字基础色，以 `primary` 作为少量交互强调色；状态色使用 Nuxt UI 的 semantic color。
- 所有颜色必须同时考虑 light/dark 模式，优先使用成对的 Tailwind class，例如 `text-slate-600 dark:text-slate-300`，不要只为单一主题写颜色。
- 视觉样式优先对齐 `E:/Project/hydcraft-portal` 的 `PageHeader`、`HeaderMenu` 和账户菜单，特别是 active 状态、Popover 菜单、选中态、间距和过渡效果。
- 品牌图片、图标和字体优先从本地资源导入，不在运行时直接请求远程资源。

## 窗口与布局边界

- 保持 updater 原有左右分栏：左侧 aside 负责品牌、外观和账户操作，右侧负责当前 tab 内容；不要把 tabs 或窗口按钮放回 aside。
- 无边框窗口的最小化、关闭和拖动必须集中在标题栏组件中。标题栏只能覆盖所属内容窗口，不能跨越 aside。
- 拖动区域可以调用 Tauri `startDragging`，但不要把 cursor 改成 `grab`；按钮、Popover、链接、表单控件和其他交互元素必须阻止拖动。
- 认证、版本选择、需要独立生命周期的确认流程优先使用真实的 Tauri 子窗口（例如 `auth`、`version`），不要把所有内容堆到主窗口或伪造 Web modal。
- 主窗口只承载稳定的状态和 tab 内容；流程性、阻塞性或需要独立关闭/聚焦的内容应拆成窗口。

## i18n

- 所有用户可见文本都必须经过 `t()`，组件、Rust 命令返回值和错误包装中不得新增硬编码中文或英文 UI 文案。
- 语言文件按语言和功能拆分，目录结构固定为：

  ```text
  src/locales/{locale}/common.json
  src/locales/{locale}/updater.json
  src/locales/{locale}/auth.json
  src/locales/{locale}/version.json
  src/locales/{locale}/errors.json
  ```

- 当前语言必须同时维护 `zh-CN`、`zh-TW`、`ja-JP`、`en-US`；新增 key 必须同步写入全部语言文件。
- 动态文案使用参数插值（例如 `{seconds}`、`{error}`），不要在模板中拼接自然语言句子；版本号、Hydroline ID、路径等数据可以作为参数或后端返回值保留。
- `src/locales/index.ts` 负责按功能合并语言文件，组件只通过 composable 暴露的 `t()` 读取文案。

## 错误与信息处理

- 错误、登录结果、更新失败、版本选择失败、下载源读取失败等信息统一使用 Tauri 系统 dialog 展示，不要把错误堆在主页面中伪装成 Web 应用。
- 错误 dialog 的固定前缀和标题必须走 i18n；底层错误可以作为 `{error}` 技术详情传入。
- 页面内只保留必要的中性流程状态（例如正在检查、正在更新、倒计时），不要同时渲染一套重复的错误卡片。
- Console 返回的真实列表优先展示；没有列表时只能使用明确的占位项，不能伪造真实版本或下载源。

## 交互与实现

- 版本、客户端和下载源选择统一使用 `USelect`，选中态、禁用态和登录要求必须清晰可见。
- Bootstrap 与手动打开模式必须保持边界：手动模式不能启动 Minecraft，Bootstrap 才能在流程结束后继续启动客户端。
- 状态流转优先通过 Tauri command 和 `updater-status` 事件解耦，组件负责渲染和用户交互，不直接复制更新业务逻辑。
- 不使用 mock 数据；如果 Console 没有数据，使用明确的 fallback/占位逻辑并在代码和 UI 中保持可识别。

## 修改后的验证

- 前端修改后运行 `pnpm format` 和 `pnpm build`。
- Rust 修改后运行 `cargo fmt --check` 和 `cargo check`（工作目录为 `src-tauri`）。
- 不要为了验证而默认启动 dev server；只有用户明确要求时才启动。
- 完成后检查新增文本是否全部进入语言文件、所有语言 key 是否一致，以及 `git diff --check` 是否通过。
