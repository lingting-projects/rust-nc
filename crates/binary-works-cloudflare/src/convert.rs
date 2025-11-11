use crate::core::{http_get, RequestExt};
use crate::share;
use library_core::boolean::is_true;
use library_core::core::{AnyResult, BizError};
use library_nc::http::pick_host;
use library_nc::kernel::{
    dns_default_cn, dns_default_proxy, exclude_default, include_main, KernelConfig, NodeContains,
};
use library_nc::rule::{Rule, RuleType};
use library_nc::subscribe::{Subscribe, HEADER_INFO};
use std::collections::HashMap;
use worker::wasm_bindgen::UnwrapThrowExt;
use worker::Error::RustError;
use worker::{console_debug, Env, Request, Response, ResponseBuilder};

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

    pub fn from_fetch(req: Request, env: Env) -> AnyResult<Self> {
        Self::from_map(req.query_map()?, env)
    }

    pub fn from_map(source: HashMap<String, Vec<String>>, env: Env) -> AnyResult<Self> {
        let remote = Self::first(&source, "remote").expect_throw("remote not found");
        let tun = Self::convert_bool(source.get("tun")).unwrap_or(true);
        let ipv6 = Self::convert_bool(source.get("ipv6")).unwrap_or(true);
        let fake_ip = Self::convert_bool(source.get("fake_ip")).unwrap_or(true);
        let debug = Self::convert_bool(source.get("debug")).unwrap_or(false);
        let geo_cn = Self::convert_bool(source.get("geo_cn")).unwrap_or(true);

        let uo = if remote.starts_with("s:") {
            let source = &remote[2..];
            console_debug!("分享源: {}", source);
            let segments: Vec<&str> = source.split('?').collect();
            let key = segments[0];
            let p = if segments.len() > 1 {
                segments[1].to_string()
            } else {
                "".to_string()
            };
            share::find_url(env, key, Some(&p))
        } else {
            Some(remote)
        };
        if uo == None {
            return Err(Box::new(BizError::SubscribeNotFound));
        }
        let url = &uo.unwrap();

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
            remote: url.clone(),
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
    let remote = &params.remote;
    console_debug!("从远程[{}]获取数据", remote);
    let mut response = http_get(remote).await?;
    if response.status_code() != 200 {
        match response.text().await {
            Ok(body) => {
                console_debug!("远程返回内容: {}", body)
            }
            Err(_) => {}
        }
        return Err(Box::new(RustError(format!(
            "远程[{}]返回异常! {}",
            remote,
            response.status_code()
        ))));
    }
    let headers = response.headers();
    let info = headers.get(HEADER_INFO)?;
    let remote = response.text().await?;
    console_debug!("解析远程[{}]数据, 长度: {}", &params.remote, remote.len());
    Subscribe::resolve(&remote, info)
}

struct Remote {
    pub config: KernelConfig,
    pub filename: String,
    pub info: Option<String>,
}

const gist_prefix: &str =
    "https://gist.githubusercontent.com/lingting/93a4a9ff5d1134aab8ca286bec969436/raw/";

async fn build_remote(
    req: Request,
    env: Env,
    rules_direct: Vec<Rule>,
    rules_proxy: Vec<Rule>,
    rules_reject: Vec<Rule>,
) -> AnyResult<Remote> {
    console_debug!("解析参数");
    let params = ConvertParams::from_fetch(req, env)?;
    console_debug!("解析远程地址: {}", &params.remote);
    let host = pick_host(&params.remote).expect_throw("invalid remote");
    console_debug!("远程域名: {}", &host);
    let subscribe = subscribe(&params).await?;
    let info = subscribe.info();
    console_debug!("构造配置");
    let config = params.build_config(subscribe, rules_direct, rules_proxy, rules_reject)?;

    Ok(Remote {
        config,
        filename: host,
        info,
    })
}

pub async fn sing_box(req: Request, env: Env) -> AnyResult<Response> {
    let remote = build_remote(
        req,
        env,
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
    let config = &remote.config;
    console_debug!("配置转换");
    let json = config.sing_box_default()?;
    console_debug!("返回配置");

    let builder = Response::builder();

    ok(builder, remote, "json", json)
}

pub async fn clash(req: Request, env: Env) -> AnyResult<Response> {
    let remote = build_remote(
        req,
        env,
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
    let config = &remote.config;
    console_debug!("配置转换");
    let yml = config.clash_default()?;
    console_debug!("返回配置");

    let builder = Response::builder();
    ok(builder, remote, "yaml", yml)
}

fn ok(
    mut builder: ResponseBuilder,
    remote: Remote,
    ext: &str,
    body: String,
) -> AnyResult<Response> {
    let _type = format!("application/{}", ext);
    let disposition = format!("inline; filename=\"{}.{}\"", remote.filename, ext);
    builder = builder.with_header("Content-Disposition", &disposition)?;

    if let Some(v) = &remote.info {
        builder = builder.with_header("Subscription-Userinfo", v)?
    }

    builder = builder.with_header("content-type", &_type)?;
    let r = builder.fixed(body.into_bytes());
    Ok(r)
}
