# AGENTS.md

## 长度单位

适用于 `crates/bin-gpui`：GPUI 自绘界面的长度一律用 **`Rems`**（`rems()`）。
GPUI 的 rem 以 `window.rem_size()`（由 `gpui-component` 的 Root 设为 `theme.font_size`）为基准，
与 `gap_2`、`px_3`、`text_sm`、`rounded_lg` 等同源；主题字号一变，字号、间距、控件尺寸、图标一起缩放。

- 尺寸、内边距、间距、圆角、图标几何、列宽、弹窗宽度等**长度相关**的取值，一律用 `rems()` 或
  `gap_*` / `p_*` / `h_*` / `size_*` / `text_*` / `rounded_*` 这类 rem 缩放助手，不要用 `px()`。
- 常量声明为 `Rems`，需要 0 时写 `rems(0.)`，不要写 `px(0.)`。
- 与既有 rem 数值保持同一比例（1rem = 16px 基准）：32px 写 `rems(2.)`，14px 写 `rems(0.875)`。
- 与框架交互而必须传 `Pixels` 时（例如 `Dialog::w`、`Column::width`），在调用点用
  `长度.to_pixels(window.rem_size())` 换算，不要退回写死像素。

### 例外

以下位置继续用 `px()` / `Pixels`，并且要写注释说明为什么脱离缩放：

1. **标题栏**：标题栏与系统摆放的窗口控制按钮对齐，跟随字号缩放会错位，因此标题栏自身的高度、
   图标、菜单项尺寸、窗口控制按钮宽度允许用 `px()`。标题栏之外的内容区不受此例外影响。
2. **GPUI 强制 `Pixels` 的类型**：`BoxShadow` 的 `offset` / `blur_radius` / `spread_radius`。
3. **平台物理量**：窗口 `size` / `size_min`（`Size<DevicePixels>`）、屏幕 `visible_bounds` 与
   `scale_factor`（`f32`）、`titlebar.traffic_light_position`（GPUI 要求 `Point<Pixels>`）。

公共组件库 `lingting-gpui-kit` 有同一份规则的详细说明（含 `Menu` 收 `AbsoluteLength` 的原因），
改动那里时以该仓库的 `AGENTS.md` 为准。
