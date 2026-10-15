//! 路由表：路径与页面工厂的映射。

use std::rc::Rc;

use gpui_kit::*;

/// 创建页面实体的工厂，返回类型擦除后的 [`AnyView`] 便于统一缓存。
///
/// 用 `Rc` 而非裸函数指针，是因为工厂需要捕获页面的构造闭包。
pub type RouteFactory = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>;

/// 一条路由记录。
pub struct Route {
    path: SharedString,
    factory: RouteFactory,
}

impl Route {
    /// 注册一个有状态页面。
    ///
    /// 页面在首次进入时通过 [`App::new`] 创建，之后由页面缓存复用，
    /// 所以再次进入同一路由时页面内部状态仍然保留。
    pub fn component<T, F>(path: impl Into<SharedString>, create: F) -> Self
    where
        T: Render + 'static,
        F: Fn(&mut Window, &mut App) -> T + 'static,
    {
        Self {
            path: path.into(),
            factory: Rc::new(move |window, cx| cx.new(|cx| create(window, cx)).into()),
        }
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// 页面缓存的键。同一个路由始终对应同一个键，因此只创建一份页面。
    pub(crate) fn cache_key(&self) -> String {
        self.path.to_string()
    }

    pub(crate) fn factory(&self) -> RouteFactory {
        self.factory.clone()
    }
}

/// 已注册的路由表。
#[derive(Default)]
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    /// 注册一条路由。
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    pub(crate) fn find(&self, path: &str) -> Option<&Route> {
        self.routes.iter().find(|route| route.path() == path)
    }

    pub(crate) fn contains(&self, path: &str) -> bool {
        self.find(path).is_some()
    }
}
