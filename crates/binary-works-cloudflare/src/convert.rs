use crate::core::{http_get, RequestExt};
use library_core::boolean::is_true;
use library_core::core::AnyResult;
use library_nc::http::pick_host;
use library_nc::kernel::{
    dns_default_cn, dns_default_proxy, exclude_default, include_main, KernelConfig, NodeContains,
};
use library_nc::rule::{Rule, RuleType};
use library_nc::subscribe::{Subscribe, HEADER_INFO};
use std::collections::HashMap;
use worker::wasm_bindgen::UnwrapThrowExt;
use worker::Error::RustError;
use worker::{console_debug, Request, Response};

struct ConvertParams {
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
        Some(is_true(first))
    }

    fn get_all(source: &HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
        let mut r = Vec::new();

        source.get(key).map(|vec| {
            vec.iter().for_each(|s| {
                let trim = s.trim();
                if !trim.is_empty() {
                    r.push(trim.to_string())
                }
            })
        });

        r
    }

    fn contains(
        prefix: &str,
        source: &HashMap<String, Vec<String>>,
        default: &NodeContains,
    ) -> NodeContains {
        let area = Self::get_all(source, &format!("{}.area", prefix));
        let name_contains = Self::get_all(source, &format!("{}.name_contains", prefix));

        NodeContains {
            area,
            name_contains,
            non_area: default.non_area,
            non_name: default.non_name,
        }
    }

    fn first(source: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
        let vec = source.get(key)?;
        let first = vec.first()?;
        Some(first.clone())
    }

    pub fn from_fetch(req: Request) -> AnyResult<Self> {
        Self::from_map(req.query_map()?)
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
            Self::contains("include", &source, &include_main)
        };

        let mut exclude = Self::contains("exclude", &source, &exclude_default);
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
            debug: self.debug,
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
        .with_default(&self.include, &self.exclude);

        Ok(config)
    }
}

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
    let info = headers.get(HEADER_INFO)?;
    let remote = response.text().await?;
    console_debug!("解析远程数据: {}", &params.remote);
    Subscribe::resolve(&remote, info)
}

struct Remote {
    pub config: KernelConfig,
    pub disposition: String,
    pub info: Option<String>,
}

const gist_prefix: &str =
    "https://gist.githubusercontent.com/lingting/93a4a9ff5d1134aab8ca286bec969436/raw/";

async fn build_remote(
    req: Request,
    rules_direct: Vec<Rule>,
    rules_proxy: Vec<Rule>,
    rules_reject: Vec<Rule>,
) -> AnyResult<Remote> {
    console_debug!("解析参数");
    let params = ConvertParams::from_fetch(req)?;
    console_debug!("解析远程地址: {}", &params.remote);
    let host = pick_host(&params.remote).expect_throw("invalid remote");
    console_debug!("远程域名: {}", &host);
    let subscribe = subscribe(&params).await?;
    let info = subscribe.info();
    console_debug!("构造配置");
    let config = params.build_config(subscribe, rules_direct, rules_proxy, rules_reject)?;

    let disposition = format!("inline; filename={}.json", &host);
    Ok(Remote {
        config,
        disposition,
        info,
    })
}

pub async fn sing_box(req: Request) -> AnyResult<Response> {
    let remote = build_remote(
        req,
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
    )
    .await?;
    let config = remote.config;
    let disposition = remote.disposition;
    let info = remote.info;
    console_debug!("配置转换");
    let json = config.sing_box_default()?;
    console_debug!("返回配置");

    let mut builder = Response::builder().with_header("Content-Disposition", &disposition)?;

    if let Some(v) = info {
        builder = builder.with_header("Subscription-Userinfo", &v)?
    }

    Ok(builder.ok(json)?)
}

pub async fn clash(req: Request) -> AnyResult<Response> {
    let remote = build_remote(
        req,
        vec![
            Rule::from_remote(RuleType::Other, format!("{}clash.direct", gist_prefix)),
            Rule::from_remote(RuleType::Ip, format!("{}clash.direct.ip", gist_prefix)),
        ],
        vec![
            Rule::from_remote(RuleType::Other, format!("{}clash.proxy", gist_prefix)),
            Rule::from_remote(RuleType::Ip, format!("{}clash.proxy.ip", gist_prefix)),
        ],
        vec![
            Rule::from_remote(RuleType::Process, format!("{}clash.reject", gist_prefix)),
            Rule::from_remote(RuleType::Ip, format!("{}clash.reject.ip", gist_prefix)),
        ],
    )
    .await?;
    let config = remote.config;
    let disposition = remote.disposition;
    let info = remote.info;
    console_debug!("配置转换");
    let json = config.clash_default()?;
    console_debug!("返回配置");

    let mut builder = Response::builder().with_header("Content-Disposition", &disposition)?;

    if let Some(v) = info {
        builder = builder.with_header("Subscription-Userinfo", &v)?
    }

    Ok(builder.ok(json)?)
}
