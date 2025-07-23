use axum::routing::get;
use axum::Router;

pub static key_config_selected: &str = "config_selected";

async fn hello() -> String {
    String::from("hello")
}

pub fn fill(router: Router) -> Router {
    router.route("/", get(hello))
}
