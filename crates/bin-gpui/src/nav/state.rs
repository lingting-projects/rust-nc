//! 当前路由状态与页面缓存。

use std::collections::HashMap;

use gpui_kit::*;

use super::route::Router;

/// 应用级路由表。必须在创建窗口之前通过 [`init`] 注册。
struct RouteTable(Router);

impl Global for RouteTable {}

/// 页面缓存。
///
/// `AnyView` 内部持有实体的强引用，因此页面一旦创建就会一直存活，
/// 切换页面再切回来时页面内部状态不会丢失。
#[derive(Default)]
struct PageCache(HashMap<String, AnyView>);

impl Global for PageCache {}

/// 当前路由状态。
///
/// 对外可见是为了让 [`super::router::Outlet`] 能订阅它的变化。
pub struct RoutePath {
    path: SharedString,
}

impl Global for RoutePath {}

/// 注册全局路由表。必须在创建窗口之前调用。
pub fn init(cx: &mut App, configure: impl FnOnce(&mut Router)) {
    let mut router = Router::default();
    configure(&mut router);
    cx.set_global(RouteTable(router));
    cx.set_global(PageCache::default());
    cx.set_global(RoutePath {
        path: SharedString::default(),
    });
}

/// 当前路径。
pub fn current_path(cx: &App) -> SharedString {
    cx.global::<RoutePath>().path.clone()
}

/// 切换页面。
///
/// 未注册的路径会被忽略，避免渲染出空白内容。
pub fn navigate(cx: &mut App, path: impl Into<SharedString>) {
    let path = path.into();
    if !cx.global::<RouteTable>().0.contains(&path) {
        return;
    }
    if cx.global::<RoutePath>().path == path {
        return;
    }
    cx.global_mut::<RoutePath>().path = path;
    cx.refresh_windows();
}

/// 取得当前路径对应的页面，必要时创建并缓存。
pub(crate) fn page_for_current_path(window: &mut Window, cx: &mut App) -> Option<AnyView> {
    let path = current_path(cx);
    let route = cx.global::<RouteTable>().0.find(&path)?;
    let key = route.cache_key();

    if let Some(view) = cx.global::<PageCache>().0.get(&key) {
        return Some(view.clone());
    }

    let view = (route.factory())(window, cx);
    cx.global_mut::<PageCache>().0.insert(key, view.clone());
    Some(view)
}
