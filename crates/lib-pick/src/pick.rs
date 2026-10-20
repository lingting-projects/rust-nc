use anyhow::{Result, bail};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use indexmap::IndexMap;
use log::warn;
use percent_encoding::percent_decode_str;
use serde_json::Value;
use serde_yaml::Value as Yml;

use crate::node::{KEY_NAME, KEY_PORT, KEY_SERVER, PickNode, PickNodes};
use crate::remote::PickRemote;
use crate::value::{JsonValueExt, yml_to_json};

/// clash 配置中协议所在字段。
pub const KEY_TYPE: &str = "type";
/// 分享链接中协议所在字段。
pub const KEY_SCHEME: &str = "scheme";

/// 自动识别来源并提取节点。
///
/// `user_info` 为 `Subscription-Userinfo` 响应头, 缺失时从节点名称解析流量与到期信息。
pub fn pick(source: &str, user_info: Option<&str>) -> Result<PickRemote> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(PickRemote::resolve(PickNodes::default(), user_info));
    }

    // 单行内容多为整体 Base64 编码的订阅
    if !source.contains('\n')
        && let Ok(decoded) = decode_base64(source)
    {
        let decoded = decoded.trim();
        if !decoded.is_empty() && decoded != source {
            return pick(decoded, user_info);
        }
    }

    let nodes = match pick_clash(source) {
        Ok(nodes) if !nodes.is_empty() => nodes,
        _ => pick_pure(source)?,
    };

    Ok(PickRemote::resolve(nodes, user_info))
}

/// 从 clash 配置中提取节点。
pub fn pick_clash(source: &str) -> Result<PickNodes> {
    let root: IndexMap<String, Yml> = serde_yaml::from_str(source)?;
    let proxies = root
        .get("proxies")
        .and_then(Yml::as_sequence)
        .cloned()
        .unwrap_or_default();

    let mut nodes = Vec::new();
    for proxy in proxies {
        let Some(mapping) = proxy.as_mapping() else {
            continue;
        };

        let mut map = IndexMap::new();
        for (key, value) in mapping {
            let Some(key) = yml_to_json(key).string() else {
                continue;
            };
            map.insert(key, yml_to_json(value));
        }

        nodes.push(PickNode::new(map, KEY_TYPE));
    }

    Ok(PickNodes::new(nodes))
}

/// 从纯净订阅中提取节点, 每行一个分享链接。
pub fn pick_pure(source: &str) -> Result<PickNodes> {
    let mut nodes = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match pick_link(line) {
            Ok(Some(node)) => nodes.push(node),
            Ok(None) => {}
            Err(e) => log_skip(line, &e),
        }
    }

    Ok(PickNodes::new(nodes))
}

/// 解析单个分享链接, 无法识别时返回 `Ok(None)`。
fn pick_link(source: &str) -> Result<Option<PickNode>> {
    let Some((scheme, rest)) = source.split_once("://") else {
        return Ok(None);
    };

    let protocol = scheme.trim().to_lowercase();
    let mut map = IndexMap::new();
    map.insert(KEY_SCHEME.to_string(), Value::String(protocol.clone()));

    match protocol.as_str() {
        "ss" | "shadowsocks" => parse_shadow_socks(rest, &mut map)?,
        "trojan" => parse_trojan(rest, &mut map)?,
        "vless" => parse_vless(rest, &mut map)?,
        "hysteria2" | "hy2" => parse_trojan(rest, &mut map)?,
        "anytls" => parse_trojan(rest, &mut map)?,
        _ => return Ok(None),
    }

    Ok(Some(PickNode::new(map, KEY_SCHEME)))
}

/// 解析 shadowsocks 分享链接: `ss://base64(method:password)@host:port#name`。
fn parse_shadow_socks(rest: &str, map: &mut IndexMap<String, Value>) -> Result<()> {
    let (secret, config) = split_once(rest, '@')?;
    let secret = decode_base64(secret).unwrap_or_else(|_| secret.to_string());
    let (cipher, password) = split_once(&secret, ':')?;

    map.insert(
        "cipher".to_string(),
        Value::String(cipher.trim().to_string()),
    );
    map.insert(
        "password".to_string(),
        Value::String(password.trim().to_string()),
    );

    parse_host(config, map)
}

/// 解析 trojan、hysteria2 分享链接: `scheme://password@host:port?params#name`。
fn parse_trojan(rest: &str, map: &mut IndexMap<String, Value>) -> Result<()> {
    let (password, config) = split_once(rest, '@')?;
    map.insert(
        "password".to_string(),
        Value::String(decode(password.trim())?),
    );

    parse_host(config, map)
}

/// 解析 vless 分享链接: `vless://uuid@host:port?params#name`。
fn parse_vless(rest: &str, map: &mut IndexMap<String, Value>) -> Result<()> {
    let (uuid, config) = split_once(rest, '@')?;
    map.insert("uuid".to_string(), Value::String(decode(uuid.trim())?));

    parse_host(config, map)
}

/// 解析 `host:port?params#name` 部分。
fn parse_host(source: &str, map: &mut IndexMap<String, Value>) -> Result<()> {
    let (host, rest) = split_once(source, '#')?;
    let (host, params) = match host.split_once('?') {
        Some((host, params)) => (host, params),
        None => (host, ""),
    };

    // 部分订阅把端口写成 `host:port/`, 末尾斜杠不属于端口
    let (server, port) = match host.rsplit_once(':') {
        Some((server, port)) => (server, port.trim_end_matches('/')),
        None => (host, "443"),
    };

    map.insert(
        KEY_SERVER.to_string(),
        Value::String(server.trim().to_string()),
    );
    map.insert(KEY_PORT.to_string(), Value::String(port.trim().to_string()));
    map.insert(KEY_NAME.to_string(), Value::String(decode(rest.trim())?));

    for (key, value) in parse_params(params) {
        map.insert(key, value);
    }

    Ok(())
}

/// 解析查询参数, 同名参数合并为数组。
fn parse_params(source: &str) -> IndexMap<String, Value> {
    let mut map: IndexMap<String, Value> = IndexMap::new();
    for param in source.split('&') {
        let Some((key, value)) = param.split_once('=') else {
            continue;
        };
        if key.is_empty() {
            continue;
        }

        let value = Value::String(value.to_string());
        match map.get_mut(key) {
            None => {
                map.insert(key.to_string(), value);
            }
            Some(Value::Array(values)) => values.push(value),
            Some(current) => {
                let first = current.clone();
                *current = Value::Array(vec![first, value]);
            }
        }
    }
    map
}

/// 按第一个分隔符切分, 缺失时返回错误。
fn split_once(source: &str, separator: char) -> Result<(&str, &str)> {
    match source.split_once(separator) {
        Some((left, right)) => Ok((left, right)),
        None => bail!("缺少分隔符[{}]: {}", separator, source),
    }
}

/// 解析 Base64, 兼容 URL 安全字符集与缺失的填充。
fn decode_base64(source: &str) -> Result<String> {
    let mut source = source.trim().replace('-', "+").replace('_', "/");
    let remainder = source.len() % 4;
    if remainder > 0 {
        source.push_str(&"=".repeat(4 - remainder));
    }

    let bytes = BASE64_STANDARD.decode(source)?;
    Ok(String::from_utf8(bytes)?)
}

/// URL 解码。
fn decode(source: &str) -> Result<String> {
    Ok(percent_decode_str(source).decode_utf8()?.into_owned())
}

/// 记录无法解析的行, 不影响其余节点。
fn log_skip(line: &str, error: &anyhow::Error) {
    warn!("节点解析失败! {}; {}", line, error);
}
