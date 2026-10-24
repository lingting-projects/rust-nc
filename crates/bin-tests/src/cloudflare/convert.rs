//! worker 转换接口的校验。
//!
//! `/clash` 与 `/sing-box` 返回配置文本, 校验分两层: 一是响应本身(状态码、内容类型、
//! 文件名), 二是配置结构(出口、代理组、规则、dns 与入口)。结构校验只依赖配置自身的
//! 字段约定, 与 `lib-converter` 生成的形态保持一致。

use anyhow::{Context, Result, bail};
use lib_fetch::{FetchClient, FetchRequest, FetchResponse, UA};
use serde_json::Value;
use serde_yaml::Value as Yml;

/// 转换接口对应的平台。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// clash 配置, 响应为 yaml
    Clash,
    /// sing-box 配置, 响应为 json
    SingBox,
}

impl Platform {
    /// 全部平台。
    pub const ALL: [Self; 2] = [Self::Clash, Self::SingBox];

    /// 接口路径。
    pub fn path(&self) -> &'static str {
        match self {
            Self::Clash => "/clash",
            Self::SingBox => "/sing-box",
        }
    }

    /// 响应内容类型。
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Clash => "application/yaml",
            Self::SingBox => "application/json",
        }
    }

    /// 响应文件扩展名。
    pub fn ext(&self) -> &'static str {
        match self {
            Self::Clash => "yaml",
            Self::SingBox => "json",
        }
    }

    /// 请求一次转换接口。
    ///
    /// worker 会把来源请求头转发给订阅源, 而订阅源普遍按 UA 判断来源, 因此这里
    /// 固定带 Chrome UA, 与真实浏览器访问 worker 的请求保持一致。
    pub async fn request(&self, worker: &str, remote: &str) -> Result<FetchResponse> {
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        serializer.append_pair("remote", remote);

        let url = format!("{}{}?{}", worker, self.path(), serializer.finish());
        FetchClient::new()
            .get(FetchRequest::new(url).with_ua(UA))
            .await
            .with_context(|| format!("请求 {} 失败", self.path()))
    }

    /// 校验响应头与响应体。
    pub fn verify(&self, response: &FetchResponse) -> Result<()> {
        if !response.is_success() {
            bail!(
                "{} 返回异常: {}, 内容: {}",
                self.path(),
                response.status,
                response.string()
            );
        }

        self.headers(response)?;
        let body = response.string();
        match self {
            Self::Clash => clash(&body),
            Self::SingBox => sing_box(&body),
        }
    }

    /// 校验响应头。
    fn headers(&self, response: &FetchResponse) -> Result<()> {
        let content_type = response
            .content_type()
            .with_context(|| format!("{} 缺少 content-type", self.path()))?;
        if content_type != self.content_type() {
            bail!(
                "{} 的 content-type 应为 {}, 实际为 {}",
                self.path(),
                self.content_type(),
                content_type
            );
        }

        let disposition = response
            .content_disposition()
            .with_context(|| format!("{} 缺少 content-disposition", self.path()))?;
        let ext = format!(".{}", self.ext());
        if !disposition.contains(&ext) {
            bail!(
                "{} 的 content-disposition 缺少扩展名 {}: {}",
                self.path(),
                ext,
                disposition
            );
        }

        Ok(())
    }
}

/// clash 配置结构校验。
fn clash(body: &str) -> Result<()> {
    let root: Yml = serde_yaml::from_str(body).context("clash 配置不是合法 yaml")?;

    let proxies = yml_sequence(&root, "proxies")?;
    if proxies.is_empty() {
        bail!("clash 配置没有节点");
    }
    for proxy in proxies {
        let name = yml_string(proxy, "name")?;
        yml_string(proxy, "type")?;
        yml_string(proxy, "server")?;
        if proxy.get("port").and_then(Yml::as_u64).is_none() {
            bail!("clash 节点[{name}]缺少 port");
        }
    }

    if yml_sequence(&root, "proxy-groups")?.is_empty() {
        bail!("clash 配置没有代理组");
    }
    if yml_sequence(&root, "rules")?.is_empty() {
        bail!("clash 配置没有规则");
    }

    Ok(())
}

/// sing-box 配置结构校验。
fn sing_box(body: &str) -> Result<()> {
    let root: Value = serde_json::from_str(body).context("sing-box 配置不是合法 json")?;

    let outbounds = json_array(&root, "outbounds")?;
    let nodes: Vec<&Value> = outbounds
        .iter()
        .filter(|item| !is_group(item) && !is_direct(item))
        .collect();
    if nodes.is_empty() {
        bail!("sing-box 配置没有节点出口");
    }
    for node in nodes {
        let tag = json_string(node, "tag")?;
        json_string(node, "server")?;
        if node.get("server_port").and_then(Value::as_u64).is_none() {
            bail!("sing-box 出口[{tag}]缺少 server_port");
        }
    }

    let route = root.get("route").context("sing-box 配置缺少 route")?;
    json_string(route, "final")?;
    if json_array(route, "rules")?.is_empty() {
        bail!("sing-box 路由没有规则");
    }

    let dns = root.get("dns").context("sing-box 配置缺少 dns")?;
    if json_array(dns, "servers")?.is_empty() {
        bail!("sing-box dns 没有服务器");
    }

    if json_array(&root, "inbounds")?.is_empty() {
        bail!("sing-box 配置没有入口");
    }

    Ok(())
}

/// 是否为选择类出口。
fn is_group(item: &Value) -> bool {
    matches!(
        item.get("type").and_then(Value::as_str),
        Some("selector") | Some("urltest")
    )
}

/// 是否为直连出口。
fn is_direct(item: &Value) -> bool {
    item.get("type").and_then(Value::as_str) == Some("direct")
}

/// 取出必须存在的 json 数组字段。
fn json_array<'a>(root: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    root.get(key)
        .and_then(Value::as_array)
        .with_context(|| format!("配置缺少数组字段 {key}"))
}

/// 取出必须存在的 json 字符串字段。
fn json_string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("配置缺少字符串字段 {key}"))
}

/// 取出必须存在的 yaml 序列字段。
fn yml_sequence<'a>(root: &'a Yml, key: &str) -> Result<&'a Vec<Yml>> {
    root.get(key)
        .and_then(Yml::as_sequence)
        .with_context(|| format!("配置缺少序列字段 {key}"))
}

/// 取出必须存在的 yaml 字符串字段。
fn yml_string<'a>(value: &'a Yml, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Yml::as_str)
        .with_context(|| format!("配置缺少字符串字段 {key}"))
}
