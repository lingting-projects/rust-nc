use axum::Router;
use tower_http::cors::{AllowHeaders, AllowMethods, Any, CorsLayer, ExposeHeaders};

pub fn fill(router: Router) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .expose_headers(ExposeHeaders::any());
    router.route_layer(cors)
}
