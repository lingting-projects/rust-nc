mod dns;
mod outbound;
mod route;
mod rule;

pub use rule::SingBoxRule;

use anyhow::Result;
use serde::Serialize;

use crate::constants::{
    FAKE_IPV4, FAKE_IPV6, INNER_IPV4, INNER_IPV6, LOOPBACK_IPV4, LOOPBACK_IPV6, OUT_DIRECT,
    ROUTE_IPV4, TAG_HTTP_CLIENT,
    ROUTE_IPV6, TAG_AUTO, TAG_FALLBACK, TAG_SELECTOR, VIRTUAL_IPV4, VIRTUAL_IPV6,
};
use crate::platform::Kernel;
use crate::source::Source;

use dns::DnsConfig;
use outbound::Outbound;
use route::RouteConfig;

/// 生成 sing-box 配置。
pub(crate) fn build(kernel: &Kernel, source: Source) -> Result<String> {
    let route = route::build(kernel);
    let dns = dns::build(kernel, &route);

    let config = SingBoxConfig {
        log: LogConfig {
            level: if kernel.config.debug { "debug" } else { "info" }.to_string(),
            timestamp: true,
        },
        experimental: ExperimentalConfig {
            cache_file: ExperimentalCache {
                enabled: true,
                // 1.14 起 store_rdrc 废弃, 由 store_dns 持久化完整 DNS 缓存
                store_dns: true,
                store_fakeip: kernel.config.fake_ip,
            },
        },
        services: services(kernel),
        http_clients: http_clients(),
        inbounds: inbounds(kernel),
        outbounds: outbounds(kernel, source),
        route,
        dns,
    };

    Ok(serde_json::to_string(&config)?)
}

#[derive(Serialize)]
struct SingBoxConfig {
    log: LogConfig,
    experimental: ExperimentalConfig,
    /// 内核 API 等内核自身提供的服务
    services: Vec<Service>,
    /// 供远程规则集等内部组件复用的 http 客户端
    http_clients: Vec<HttpClient>,
    inbounds: Vec<Inbound>,
    outbounds: Vec<Outbound>,
    route: RouteConfig,
    dns: DnsConfig,
}

#[derive(Serialize)]
struct LogConfig {
    level: String,
    timestamp: bool,
}

#[derive(Serialize)]
struct ExperimentalConfig {
    cache_file: ExperimentalCache,
}

#[derive(Serialize)]
struct ExperimentalCache {
    enabled: bool,
    /// 持久化完整 DNS 缓存, 替代 1.14 起废弃的 store_rdrc
    store_dns: bool,
    store_fakeip: bool,
}

/// 内核自身提供的服务。
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum Service {
    /// sing-box API, 提供 gRPC 接口
    Api {
        tag: String,
        listen: String,
        listen_port: u16,
    },
}

/// 按内核设置生成服务列表。
fn services(kernel: &Kernel) -> Vec<Service> {
    vec![Service::Api {
        tag: "api".to_string(),
        listen: kernel.config.ui_listen.clone(),
        listen_port: kernel.config.ui_port,
    }]
}

/// http 客户端，供远程规则集等内部组件下载资源。
///
/// 1.14 起为远程规则集隐式创建默认客户端会告警，因此显式声明一个直连客户端。
#[derive(Serialize)]
struct HttpClient {
    tag: String,
    detour: String,
}

/// 生成 http 客户端列表。
fn http_clients() -> Vec<HttpClient> {
    vec![HttpClient {
        tag: TAG_HTTP_CLIENT.to_string(),
        detour: OUT_DIRECT.to_string(),
    }]
}
#[derive(Serialize)]
#[serde(untagged)]
enum Inbound {
    Tun {
        #[serde(rename = "type")]
        kind: String,
        tag: String,
        interface_name: String,
        auto_route: bool,
        strict_route: bool,
        udp_timeout: String,
        stack: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        address: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        route_address: Option<Vec<String>>,
        loopback_address: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        route_exclude_address: Option<Vec<String>>,
    },
    Mixed {
        #[serde(rename = "type")]
        kind: String,
        tag: String,
        set_system_proxy: bool,
        listen: String,
        listen_port: u16,
        tcp_fast_open: bool,
        tcp_multi_path: bool,
        udp_fragment: bool,
    },
}

fn inbounds(kernel: &Kernel) -> Vec<Inbound> {
    let mut inbounds = Vec::new();
    if kernel.config.tun {
        inbounds.push(tun(kernel));
    }
    inbounds.push(mixed(kernel));
    inbounds
}

fn tun(kernel: &Kernel) -> Inbound {
    let config = &kernel.config;

    let (address, route_address, route_exclude_address) = if config.fake_ip {
        let mut address = vec![VIRTUAL_IPV4.to_string()];
        let mut route = vec![FAKE_IPV4.to_string()];
        if config.ipv6 {
            address.push(VIRTUAL_IPV6.to_string());
            route.push(FAKE_IPV6.to_string());
        }
        (Some(address), Some(route), None)
    } else {
        let mut route = vec![ROUTE_IPV4.to_string()];
        let mut exclude = INNER_IPV4.clone();
        if config.ipv6 {
            route.push(ROUTE_IPV6.to_string());
            exclude.extend(INNER_IPV6.iter().cloned());
        }
        (Some(route), None, Some(exclude))
    };

    let mut loopback_address = vec![LOOPBACK_IPV4.to_string()];
    if config.ipv6 {
        loopback_address.push(LOOPBACK_IPV6.to_string());
    }

    Inbound::Tun {
        kind: "tun".to_string(),
        tag: "tun-in".to_string(),
        interface_name: "NcRustTunBySingBox".to_string(),
        auto_route: true,
        strict_route: true,
        udp_timeout: "5m".to_string(),
        stack: "system".to_string(),
        address,
        route_address,
        loopback_address,
        route_exclude_address,
    }
}

fn mixed(kernel: &Kernel) -> Inbound {
    Inbound::Mixed {
        kind: "mixed".to_string(),
        tag: "mixed-in".to_string(),
        set_system_proxy: false,
        listen: kernel.config.mixed_listen.clone(),
        listen_port: kernel.config.mixed_port,
        tcp_fast_open: true,
        tcp_multi_path: true,
        udp_fragment: false,
    }
}

fn outbounds(kernel: &Kernel, source: Source) -> Vec<Outbound> {
    let areas = auto_area_outbounds(kernel);
    let area_tags: Vec<String> = areas.iter().map(|outbound| outbound.tag.clone()).collect();

    let auto = Outbound::url_test(TAG_AUTO, area_tags.clone());
    let default = area_tags
        .first()
        .cloned()
        .unwrap_or_else(|| OUT_DIRECT.to_string());

    let selector_group = selector(kernel, TAG_SELECTOR, default, &area_tags);
    let fallback = selector(kernel, TAG_FALLBACK, OUT_DIRECT.to_string(), &area_tags);

    let mut outbounds = vec![selector_group, auto, fallback];
    outbounds.extend(areas);
    for node in &kernel.nodes {
        outbounds.push(Outbound::node(node, source));
    }
    outbounds.push(Outbound::direct(OUT_DIRECT));
    outbounds
}

/// 按区域生成自动选择组。
fn auto_area_outbounds(kernel: &Kernel) -> Vec<Outbound> {
    kernel
        .map_area()
        .into_iter()
        .map(|(code, nodes)| {
            let area = nodes[0].area.expect("区域分组中的节点必然有区域");
            let tag = format!("[{}] {}自动", code, area.name.zh);
            let outbounds = nodes.iter().map(|node| node.name()).collect();
            Outbound::url_test(&tag, outbounds)
        })
        .collect()
}

fn selector(kernel: &Kernel, tag: &str, default: String, area_tags: &[String]) -> Outbound {
    let mut outbounds = vec![OUT_DIRECT.to_string(), TAG_AUTO.to_string()];
    outbounds.extend(area_tags.iter().cloned());
    outbounds.extend(kernel.nodes.iter().map(|node| node.name()));
    Outbound::selector(tag, default, outbounds)
}
