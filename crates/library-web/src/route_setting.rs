use axum::routing::get;
use axum::Router;

async fn hello() -> String {
    String::from("hello")
}

pub fn fill(router: Router) -> Router {
    router.route("/", get(hello))
}
