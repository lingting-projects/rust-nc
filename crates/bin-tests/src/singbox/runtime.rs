//! 真实运行配置。
//!
//! 生成链路产出的配置只用于结构校验, 直接拿来启动有两处不合适: 端口是固定值, 可能与
//! 正在运行的应用抢占同一端口; 默认出口是默认选择组, 而该组默认直连, 流量不会经过
//! 订阅节点。这里把配置改成可运行形态:
//!
//! - 入口换成 Rust 侧申请的随机端口, 入口类型仍是混合入口, 同时接受 socks5 与 http;
//! - 内核 API 服务同样换成随机端口;
//! - 默认出口指向节点选择组, 请求才会真正经过订阅节点。

use std::net::{Ipv4Addr, TcpListener};

use anyhow::{Context, Result};
use lib_converter::constants::TAG_SELECTOR;
use serde_json::Value;

use super::pipeline::Scenario;

/// 入口类型: 混合入口同时接受 socks5 与 http。
const INBOUND_MIXED: &str = "mixed";
/// 内核 API 服务类型。
const SERVICE_API: &str = "api";

/// 可真实运行的场景配置。
pub struct Runtime {
    /// 来源场景, 决定内核的工作目录
    scenario: Scenario,
    /// 改写后的配置文本
    body: String,
    /// 内核 API 地址
    api: String,
    /// 入口的代理地址
    proxy: String,
}

impl Runtime {
    /// 由已校验的场景配置构建可运行配置。
    pub fn new(scenario: Scenario, body: &str) -> Result<Self> {
        let mut root: Value = serde_json::from_str(body).context("配置不是合法 json")?;
        let (api_port, proxy_port) = free_ports()?;

        let listen = inbound(&mut root, proxy_port)?;
        service(&mut root, api_port)?;
        route(&mut root)?;

        Ok(Self {
            scenario,
            body: serde_json::to_string(&root)?,
            api: format!("{listen}:{api_port}"),
            proxy: format!("socks5://{listen}:{proxy_port}"),
        })
    }

    /// 场景名称, 同时也是内核的工作目录名。
    pub fn scenario(&self) -> &str {
        self.scenario.name
    }

    /// 改写后的配置文本。
    pub fn body(&self) -> &str {
        &self.body
    }

    /// 内核 API 地址。
    pub fn api(&self) -> &str {
        &self.api
    }

    /// 入口的代理地址, 交给 `lib-fetch` 直接使用。
    pub fn proxy(&self) -> &str {
        &self.proxy
    }
}

/// 申请两个互不相同的空闲端口, 返回 `(API 端口, 入口端口)`。
///
/// 绑定 0 端口让系统分配, 同时占住两个再释放, 避免两次分配拿到同一个端口。
/// 端口在之后才由内核绑定, 因此仍存在被其他进程抢占的极小概率。
fn free_ports() -> Result<(u16, u16)> {
    let api = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("申请空闲端口失败")?;
    let proxy = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("申请空闲端口失败")?;
    let api = port(&api)?;
    let proxy = port(&proxy)?;
    Ok((api, proxy))
}

/// 读取监听端口。
fn port(listener: &TcpListener) -> Result<u16> {
    let address = listener.local_addr().context("读取监听地址失败")?;
    Ok(address.port())
}

/// 入口换成随机端口, 返回入口监听地址。
fn inbound(root: &mut Value, port: u16) -> Result<String> {
    let inbound = array_mut(root, "inbounds")?
        .iter_mut()
        .find(|item| item.get("type").and_then(Value::as_str) == Some(INBOUND_MIXED))
        .with_context(|| format!("配置缺少 {INBOUND_MIXED} 入口"))?;
    inbound["listen_port"] = Value::from(port);

    let listen = inbound
        .get("listen")
        .and_then(Value::as_str)
        .with_context(|| format!("{INBOUND_MIXED} 入口缺少 listen"))?;
    Ok(listen.to_string())
}

/// 内核 API 服务换成随机端口。
fn service(root: &mut Value, port: u16) -> Result<()> {
    let service = array_mut(root, "services")?
        .iter_mut()
        .find(|item| item.get("type").and_then(Value::as_str) == Some(SERVICE_API))
        .context("配置缺少 api 服务")?;
    service["listen_port"] = Value::from(port);
    Ok(())
}

/// 默认出口改为节点选择组, 否则流量按默认组直连, 请求测不到节点。
fn route(root: &mut Value) -> Result<()> {
    let route = root.get_mut("route").context("配置缺少 route")?;
    route["final"] = Value::from(TAG_SELECTOR);
    Ok(())
}

/// 取出必须存在的数组字段。
fn array_mut<'a>(root: &'a mut Value, key: &str) -> Result<&'a mut Vec<Value>> {
    root.get_mut(key)
        .and_then(Value::as_array_mut)
        .with_context(|| format!("配置缺少 {key}"))
}
