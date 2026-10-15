//! 页面路由。
//!
//! 页面统一通过 [`Route::component`] 注册：页面是实现了 `Render` 的实体，
//! 首次进入时创建，之后一直缓存复用，因此切换页面不会丢失页面内部状态。
//!
//! 全局路由表由 [`init`] 注册，当前路径存放在全局状态里，
//! [`Outlet`] 负责把当前路径对应的页面渲染出来。

mod route;
mod router;
mod state;

pub use route::Route;
pub use router::{Outlet, init};
pub use state::{current_path, navigate};
