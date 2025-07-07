use crate::core::http_get;
use library_nc::core::AnyResult;
use library_nc::http::pick_host;
use library_nc::kernel::{
    dns_default_cn, dns_default_proxy, exclude_default, include_main, KernelConfig, NodeContains,
};
use library_nc::rule::{Rule, RuleType};
use library_nc::subscribe::Subscribe;
use std::collections::HashMap;
use worker::wasm_bindgen::UnwrapThrowExt;
use worker::Error::RustError;
use worker::{console_debug, Request, Response};

pub struct ConvertParams {
    remote: String,
    tun: bool,
    ipv6: bool,
    fake_ip: bool,
    geo_cn: bool,
    debug: bool,
    include: NodeContains,
    exclude: NodeContains,
}

impl ConvertParams {
    fn convert_bool(option: Option<&Vec<String>>) -> Option<bool> {
        let vec = option?;
        let first = vec.first()?;
        let f = match first.to_lowercase().as_ref() {
            "1" | "true" | "t" | "y" | "ok" => true,
            _ => false,
        };
        Some(f)
    }

    fn contains(prefix: &str, source: &HashMap<String, Vec<String>>) -> NodeContains {
        let mut area = Vec::new();
        let mut name_contains = Vec::new();

        source.get(&format!("{}.area", prefix)).map(|vec| {
            vec.iter().for_each(|s| {
                let trim = s.trim();
                if !trim.is_empty() {
                    area.push(trim.to_string())
                }
            })
        });
        source.get(&format!("{}.name_contains", prefix)).map(|vec| {
            vec.iter().for_each(|s| {
                let trim = s.trim();
                if !trim.is_empty() {
                    name_contains.push(trim.to_string())
                }
            })
        });

        NodeContains {
            area,
            name_contains,
        }
    }

    fn first(source: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
        let vec = source.get(key)?;
        let first = vec.first()?;
        Some(first.clone())
    }

    pub fn from_fetch(req: Request) -> AnyResult<Self> {
        let url = req.url()?;
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

        Self::from_map(source)
    }

    pub fn from_map(source: HashMap<String, Vec<String>>) -> AnyResult<Self> {
        let remote = Self::first(&source, "remote").expect_throw("remote not found");
        let tun = Self::convert_bool(source.get("tun")).unwrap_or(true);
        let ipv6 = Self::convert_bool(source.get("ipv6")).unwrap_or(true);
        let fake_ip = Self::convert_bool(source.get("fake_ip")).unwrap_or(true);
        let debug = Self::convert_bool(source.get("debug")).unwrap_or(false);
        let geo_cn = Self::convert_bool(source.get("geo_cn")).unwrap_or(true);

        let only_main = Self::convert_bool(source.get("include.only_main")).unwrap_or(false);
        let include: NodeContains = if only_main {
            include_main.clone()
        } else {
            Self::contains("include", &source)
        };

        let mut exclude = Self::contains("exclude", &source);
        if exclude.is_empty() {
            exclude = exclude_default.clone()
        }

        Ok(Self {
            remote,
            tun,
            ipv6,
            fake_ip,
            geo_cn,
            debug,
            include,
            exclude,
        })
    }

    pub fn build_config(
        &self,
        subscribe: Subscribe,
        rules_direct: Vec<Rule>,
        rules_proxy: Vec<Rule>,
        rules_reject: Vec<Rule>,
    ) -> AnyResult<KernelConfig> {
        let config = KernelConfig {
            nodes: subscribe.nodes,
            tun: self.tun,
            fake_ip: self.fake_ip,
            ipv6: self.ipv6,
            geo_cn_direct: self.geo_cn,
            rules_direct,
            rules_proxy,
            rules_reject,
            dns_cn: dns_default_cn.clone(),
            dns_proxy: dns_default_proxy.clone(),
        }
        .with_include(&self.include, true)
        .with_exclude(&self.exclude, false)
        .with_sort();
        Ok(config)
    }
}

const gist_prefix: &str =
    "https://gist.githubusercontent.com/lingting/93a4a9ff5d1134aab8ca286bec969436/raw/";

async fn subscribe(params: &ConvertParams) -> AnyResult<Subscribe> {
    console_debug!("从远程获取数据: {}", &params.remote);
    let mut response = http_get(&params.remote).await?;
    if response.status_code() != 200 {
        return Err(Box::new(RustError(format!(
            "远程返回异常! {}",
            response.status_code()
        ))));
    }
    let headers = response.headers();
    let info = headers.get("Subscription-Userinfo")?;
    let remote = response.text().await?;
    console_debug!("解析远程数据: {}", &params.remote);
    Subscribe::resolve(&remote, info)
}

pub async fn sing_box(req: Request) -> AnyResult<Response> {
    console_debug!("解析参数");
    let params = ConvertParams::from_fetch(req)?;
    console_debug!("解析远程地址: {}", &params.remote);
    let host = pick_host(&params.remote).expect_throw("invalid remote");
    console_debug!("远程域名: {}", &host);
    let subscribe = subscribe(&params).await?;
    let infoOption = subscribe.info();
    console_debug!("构造配置");
    let config = params.build_config(
        subscribe,
        vec![
            Rule::from_remote(RuleType::Process, format!("{}sing.direct.p", gist_prefix)),
            Rule::from_remote(RuleType::Other, format!("{}sing.direct.np", gist_prefix)),
            Rule::from_remote(RuleType::Ip, format!("{}sing.direct.ip", gist_prefix)),
        ],
        vec![
            Rule::from_remote(RuleType::Other, format!("{}sing.proxy", gist_prefix)),
            Rule::from_remote(RuleType::Ip, format!("{}sing.proxy.ip", gist_prefix)),
        ],
        vec![
            Rule::from_remote(RuleType::Process, format!("{}sing.reject", gist_prefix)),
            Rule::from_remote(RuleType::Ip, format!("{}sing.reject.ip", gist_prefix)),
        ],
    )?;

    console_debug!("配置转换");
    let json = config.sing_box_default()?;
    console_debug!("返回配置");

    let mut builder = Response::builder().with_header(
        "Content-Disposition",
        &format!("inline; filename={}.json", host),
    )?;

    if let Some(info) = infoOption {
        builder = builder.with_header("Subscription-Userinfo", &info)?
    }

    Ok(builder.ok(json)?)
}
