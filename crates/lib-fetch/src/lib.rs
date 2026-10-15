mod client;
#[cfg(all(
    not(any(feature = "wreq", feature = "reqwest")),
    not(target_arch = "wasm32")
))]
mod none_backend;
mod request;
#[cfg(any(feature = "reqwest", target_arch = "wasm32"))]
mod reqwest_backend;
mod response;
#[cfg(any(feature = "wreq", feature = "reqwest", target_arch = "wasm32"))]
mod util;
#[cfg(all(
    feature = "wreq",
    not(feature = "reqwest"),
    not(target_arch = "wasm32")
))]
mod wreq_backend;

pub use client::{FetchClient, UA};
pub use request::{FetchRequest, HEADER_UA};
pub use response::FetchResponse;
