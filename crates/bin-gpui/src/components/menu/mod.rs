//! 菜单组件。
//!
//! 菜单项有两种渲染方式：`menu` 用普通 `div` 自绘（导航列表），
//! `menu_button` 基于组件库的 `Button`（带自身交互状态的命令按钮）。
//! 多项如何排列由使用它的容器决定。

mod basic;

pub use basic::{MenuItem, menu};
