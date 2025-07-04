use axum::Router;
use tower_http::cors::{AllowHeaders, AllowMethods, Any, CorsLayer};

pub fn fill(router: Router) -> Router {
    // 配置 CORS 中间件 - 允许所有来源、方法和请求头
    let cors = CorsLayer::new()
        // 允许所有来源
        .allow_origin(Any)
        // 允许所有常见 HTTP 方法
        .allow_methods(AllowMethods::any())
        // 允许常见请求头
        .allow_headers(AllowHeaders::any());

    router.route_layer(cors)
}
