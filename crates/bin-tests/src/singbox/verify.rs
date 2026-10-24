use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use lib_pick::PickNodes;
use serde_json::Value;

use super::pipeline::Scenario;

/// 出口类型: 节点选择组。
const TYPE_SELECTOR: &str = "selector";
/// 出口类型: 延迟测试组。
const TYPE_URL_TEST: &str = "urltest";
/// 出口类型: 直连。
const TYPE_DIRECT: &str = "direct";
/// fake ip dns 服务器标签, 该类型没有 `server` 字段。
const TAG_DNS_FAKE: &str = "dns-fake";

/// 提取结果校验: 节点必须完整可用。
pub fn nodes(nodes: &PickNodes) -> Result<()> {
    if nodes.is_empty() {
        bail!("未提取到任何节点");
    }

    for node in &nodes.nodes {
        if node.name().trim().is_empty() {
            bail!("节点缺少名称: {node:?}");
        }
        if node.server().trim().is_empty() {
            bail!("节点[{}]缺少服务地址", node.name());
        }
        if node.port().is_none() {
            bail!("节点[{}]端口无效", node.name());
        }
    }

    Ok(())
}

/// 配置结构校验, 检查生成链路的通用约束。
///
/// 只校验结构关联, 协议字段与内核语义交给内核 `check`。
pub fn config(config: &str) -> Result<()> {
    let root: Value = serde_json::from_str(config).context("配置不是合法 json")?;

    let outbounds = array(&root, "outbounds")?;
    if outbounds.is_empty() {
        bail!("配置缺少 outbounds");
    }

    let tags = outbound_tags(outbounds)?;
    let names = node_tags(outbounds)?;
    check_outbounds(outbounds, &tags)?;
    check_route(&root, &tags, &names)?;
    check_dns(&root)?;
    check_inbounds(&root)?;

    Ok(())
}

/// 取出必须存在的数组字段。
fn array<'a>(root: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    root.get(key)
        .and_then(Value::as_array)
        .with_context(|| format!("配置缺少 {key}"))
}

/// 取出必须存在的字符串字段。
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("缺少字符串字段 {key}"))
}

/// 是否为选择类出口。
fn is_group(kind: &str) -> bool {
    matches!(kind, TYPE_SELECTOR | TYPE_URL_TEST)
}

/// 出口标签, 同时校验标签唯一。
fn outbound_tags(outbounds: &[Value]) -> Result<HashSet<String>> {
    let mut tags = HashSet::new();
    for item in outbounds {
        let tag = required(item, "tag")?;
        if !tags.insert(tag.to_string()) {
            bail!("出口标签重复: {tag}");
        }
    }
    Ok(tags)
}

/// 节点出口标签, 名称重复会让选择组引用到错误的出口。
fn node_tags(outbounds: &[Value]) -> Result<HashSet<String>> {
    let mut names = HashSet::new();
    for item in outbounds {
        let kind = required(item, "type")?;
        // 组与直连出口由转换器生成, 只校验节点名称
        if is_group(kind) || kind == TYPE_DIRECT {
            continue;
        }

        let tag = required(item, "tag")?;
        if !names.insert(tag.to_string()) {
            bail!("节点名称重复: {tag}");
        }
    }

    if names.is_empty() {
        bail!("配置没有任何节点出口, 节点可能全部被筛选条件过滤");
    }

    Ok(names)
}

/// 校验出口自身与其引用的标签。
fn check_outbounds(outbounds: &[Value], tags: &HashSet<String>) -> Result<()> {
    for item in outbounds {
        let tag = required(item, "tag")?;
        let kind = required(item, "type")?;

        if is_group(kind) {
            for member in members(item, tag)? {
                if !tags.contains(&member) {
                    bail!("出口组[{tag}]引用了不存在的出口: {member}");
                }
            }
            continue;
        }

        if kind == TYPE_DIRECT {
            continue;
        }

        if item.get("server").and_then(Value::as_str).is_none() {
            bail!("出口[{tag}]缺少 server");
        }
        if item.get("server_port").and_then(Value::as_u64).is_none() {
            bail!("出口[{tag}]缺少 server_port");
        }
    }

    Ok(())
}

/// 出口组引用的成员。
fn members(item: &Value, tag: &str) -> Result<Vec<String>> {
    let values = item
        .get("outbounds")
        .and_then(Value::as_array)
        .with_context(|| format!("出口组[{tag}]缺少 outbounds"))?;
    if values.is_empty() {
        bail!("出口组[{tag}]没有成员");
    }

    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .with_context(|| format!("出口组[{tag}]成员不是字符串"))
        })
        .collect()
}

/// 校验路由规则、规则集与默认出口。
fn check_route(root: &Value, tags: &HashSet<String>, names: &HashSet<String>) -> Result<()> {
    let route = root.get("route").context("配置缺少 route")?;

    let final_ = required(route, "final").context("路由缺少 final")?;
    if !tags.contains(final_) {
        bail!("路由 final 指向不存在的出口: {final_}");
    }

    let rule_set = rule_set_tags(route)?;
    let rules = route
        .get("rules")
        .and_then(Value::as_array)
        .context("路由缺少 rules")?;
    if rules.is_empty() {
        bail!("路由没有规则");
    }

    for rule in rules {
        if let Some(tag) = rule.get("rule_set").and_then(Value::as_str)
            && !rule_set.contains(tag)
        {
            bail!("路由规则引用了不存在的规则集: {tag}");
        }

        let Some(outbound) = rule.get("outbound").and_then(Value::as_str) else {
            continue;
        };
        if !tags.contains(outbound) {
            bail!("路由规则指向不存在的出口: {outbound}");
        }
    }

    check_selectors(root, names)
}

/// 规则集标签, 同时校验本地规则集文件存在。
fn rule_set_tags(route: &Value) -> Result<HashSet<String>> {
    let mut tags = HashSet::new();
    let values = route
        .get("rule_set")
        .and_then(Value::as_array)
        .context("路由缺少 rule_set")?;

    for item in values {
        let tag = required(item, "tag")?;
        if required(item, "type")? == "local" {
            let path = required(item, "path")?;
            if !std::path::Path::new(path).exists() {
                bail!("本地规则集[{tag}]文件不存在: {path}");
            }
        }
        tags.insert(tag.to_string());
    }

    Ok(tags)
}

/// 选择组必须包含全部节点, 否则节点无法被手动选中。
fn check_selectors(root: &Value, names: &HashSet<String>) -> Result<()> {
    for item in array(root, "outbounds")? {
        if item.get("type").and_then(Value::as_str) != Some(TYPE_SELECTOR) {
            continue;
        }

        let tag = required(item, "tag")?;
        let members: HashSet<String> = members(item, tag)?.into_iter().collect();
        for name in names {
            if !members.contains(name) {
                bail!("选择组[{tag}]缺少节点: {name}");
            }
        }
    }

    Ok(())
}

/// 校验 dns 服务器与默认服务器。
fn check_dns(root: &Value) -> Result<()> {
    let dns = root.get("dns").context("配置缺少 dns")?;

    let servers = dns
        .get("servers")
        .and_then(Value::as_array)
        .context("dns 缺少 servers")?;
    if servers.is_empty() {
        bail!("dns 没有服务器");
    }

    let mut tags = HashSet::new();
    for item in servers {
        let tag = required(item, "tag")?;
        required(item, "type")?;
        // 1.14 起 dns 服务器只能使用 type + server 的新格式, fake ip 没有 server
        if tag != TAG_DNS_FAKE && item.get("server").and_then(Value::as_str).is_none() {
            bail!("dns 服务器[{tag}]缺少 server");
        }
        tags.insert(tag.to_string());
    }

    let final_ = required(dns, "final").context("dns 缺少 final")?;
    if !tags.contains(final_) {
        bail!("dns final 指向不存在的服务器: {final_}");
    }

    Ok(())
}

/// 校验入口。
fn check_inbounds(root: &Value) -> Result<()> {
    let inbounds = array(root, "inbounds")?;
    if inbounds.is_empty() {
        bail!("配置缺少 inbounds");
    }

    for item in inbounds {
        let kind = required(item, "type")?;
        let tag = required(item, "tag")?;
        if kind == "tun" && item.get("interface_name").is_none() {
            bail!("tun 入口[{tag}]缺少 interface_name");
        }
        if kind == "mixed" && item.get("listen_port").and_then(Value::as_u64).is_none() {
            bail!("mixed 入口[{tag}]缺少 listen_port");
        }
    }

    Ok(())
}

/// 按场景校验生成结果, 确认场景开关确实作用到了配置上。
pub fn scenario(config: &str, scenario: &Scenario) -> Result<()> {
    let root: Value = serde_json::from_str(config).context("配置不是合法 json")?;

    check_tun(&root, scenario)?;
    check_fake_ip(&root, scenario)?;
    check_ipv6(&root, scenario)?;

    Ok(())
}

/// tun 入口只在启用时存在。
fn check_tun(root: &Value, scenario: &Scenario) -> Result<()> {
    let exists = array(root, "inbounds")?
        .iter()
        .any(|inbound| inbound.get("type").and_then(Value::as_str) == Some("tun"));

    match (scenario.tun, exists) {
        (true, false) => bail!("场景[{}]缺少 tun 入口", scenario.name),
        (false, true) => bail!("场景[{}]不应包含 tun 入口", scenario.name),
        _ => Ok(()),
    }
}

/// fake ip dns 服务器只在启用时存在。
fn check_fake_ip(root: &Value, scenario: &Scenario) -> Result<()> {
    let dns = root.get("dns").context("配置缺少 dns")?;
    let exists = dns
        .get("servers")
        .and_then(Value::as_array)
        .context("dns 缺少 servers")?
        .iter()
        .any(|server| server.get("tag").and_then(Value::as_str) == Some(TAG_DNS_FAKE));

    match (scenario.fake_ip, exists) {
        (true, false) => bail!("场景[{}]缺少 fake ip dns 服务器", scenario.name),
        (false, true) => bail!("场景[{}]不应包含 fake ip dns 服务器", scenario.name),
        _ => Ok(()),
    }
}

/// ipv6 开关影响 dns 策略与 tun 地址。
fn check_ipv6(root: &Value, scenario: &Scenario) -> Result<()> {
    let strategy = root
        .get("dns")
        .and_then(|dns| dns.get("strategy"))
        .and_then(Value::as_str)
        .context("dns 缺少 strategy")?;
    let expect = match scenario.ipv6 {
        true => "prefer_ipv6",
        false => "ipv4_only",
    };
    if strategy != expect {
        bail!(
            "场景[{}] dns 策略应为 {expect}, 实际为 {strategy}",
            scenario.name
        );
    }

    Ok(())
}
