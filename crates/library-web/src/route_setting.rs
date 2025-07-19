use axum::Router;
use axum::routing::get;

async fn hello() -> String {
    String::from("hello")
}

pub fn fill(router: Router) -> Router {
    router.route("/", get(hello))
}
