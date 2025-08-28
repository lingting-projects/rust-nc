use library_core::core::AnyResult;
use std::collections::HashMap;
use worker::{Fetch, Method, Request, Response};

pub async fn http_get(url: &str) -> AnyResult<Response> {
    let mut request = Request::new(url, Method::Get)?;
    let headers = request.headers_mut()?;
    headers.set("User-Agent", "lingting rust work cloudflare")?;
    let fetch = Fetch::Request(request);
    let response = fetch.send().await?;
    Ok(response)
}

pub trait RequestExt {
    fn query_map(&self) -> AnyResult<HashMap<String, Vec<String>>>;
}

impl RequestExt for Request {
    fn query_map(&self) -> AnyResult<HashMap<String, Vec<String>>> {
        let url = self.url()?;
        let query = url.query().unwrap_or("");
        let mut source: HashMap<String, Vec<String>> = HashMap::new();

        for item in query.split("&") {
            if item.is_empty() {
                continue;
            }
            let mut key = "";
            let mut value = "".to_string();
            let mut i = 0;
            item.split("=").for_each(|arg| {
                i += 1;
                if i == 1 {
                    key = arg;
                } else if i == 2 {
                    value = arg.to_string();
                } else {
                    value = format!("{}={}", value, arg);
                }
            });

            let option = source.get_mut(key);
            match option {
                None => {
                    source.insert(key.to_string(), vec![value]);
                }
                Some(vec) => vec.push(value),
            }
        }

        Ok(source)
    }
}
