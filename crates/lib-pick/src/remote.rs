use chrono::NaiveDateTime;

use crate::node::PickNodes;

/// 剩余流量前缀。
pub const PREFIX_REMAIN_TRAFFIC: &[&str] = &["剩余流量："];
/// 套餐到期前缀。
pub const PREFIX_EXPIRE: &[&str] = &["套餐到期："];
/// 节点名称中的到期时间格式。
pub const EXPIRE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// 订阅远程内容。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickRemote {
    /// 解析出的节点
    pub nodes: PickNodes,
    /// 已使用下行流量, 单位: bytes
    pub download: Option<u64>,
    /// 已使用上行流量, 单位: bytes
    pub upload: Option<u64>,
    /// 最多可用流量, 单位: bytes
    pub max: Option<u64>,
    /// 过期时间, 毫秒级时间戳
    pub expire: Option<i64>,
}

impl PickRemote {
    /// 由节点与响应头信息构建。
    ///
    /// `user_info` 为 `Subscription-Userinfo` 响应头, 缺失时从节点名称解析流量与到期信息。
    pub fn resolve(nodes: PickNodes, user_info: Option<&str>) -> Self {
        match user_info {
            Some(info) => Self::of_header(nodes, info),
            None => Self::of_nodes(nodes),
        }
    }

    /// 由响应头解析流量与到期信息。
    fn of_header(nodes: PickNodes, info: &str) -> Self {
        let mut remote = Self {
            nodes,
            ..Default::default()
        };

        for item in info.split("; ") {
            let Some((key, value)) = item.split_once('=') else {
                continue;
            };

            match key.trim() {
                "download" => remote.download = value.trim().parse().ok(),
                "upload" => remote.upload = value.trim().parse().ok(),
                "total" => remote.max = value.trim().parse().ok(),
                // 响应头中的到期时间为秒级时间戳
                "expire" => {
                    remote.expire = value
                        .trim()
                        .parse::<i64>()
                        .ok()
                        .map(|seconds| seconds * 1000)
                }
                _ => {}
            }
        }

        remote
    }

    /// 由节点名称解析流量与到期信息。
    fn of_nodes(nodes: PickNodes) -> Self {
        let mut remote = Self {
            nodes,
            ..Default::default()
        };

        for node in &remote.nodes.nodes {
            let name = node.name();
            if name.is_empty() {
                continue;
            }

            if let Some(max) = parse_remain_traffic(&name) {
                remote.max = Some(max);
                continue;
            }

            if let Some(expire) = parse_expire(&name) {
                remote.expire = Some(expire);
            }
        }

        remote
    }

    /// 是否为无流量与到期信息的订阅。
    pub fn is_empty_info(&self) -> bool {
        self.download.is_none()
            && self.upload.is_none()
            && self.max.is_none()
            && self.expire.is_none()
    }

    /// 生成响应头内容, 无信息时返回 `None`。
    pub fn info(&self) -> Option<String> {
        if self.is_empty_info() {
            return None;
        }

        let download = self.download.unwrap_or(0);
        let upload = self.upload.unwrap_or(0);
        let used = lib_core::data_size::DataSize::of_bytes(download + upload);
        let expire = match self.expire {
            Some(millis) if millis > 0 => format!("{:.2}", millis as f64 / 1000.0),
            _ => "0".to_string(),
        };

        Some(format!(
            "download={}; upload={}; used={}; total={}; expire={}",
            download,
            upload,
            used,
            self.max.unwrap_or(0),
            expire
        ))
    }
}

/// 从节点名称解析剩余流量, 单位 bytes。
fn parse_remain_traffic(name: &str) -> Option<u64> {
    let prefix = PREFIX_REMAIN_TRAFFIC
        .iter()
        .find(|p| name.starts_with(**p))?;
    let source = name.strip_prefix(prefix)?;
    lib_core::data_size::DataSize::parse(source)
        .ok()
        .map(|size| size.bytes)
}

/// 从节点名称解析到期时间, 返回毫秒级时间戳。
fn parse_expire(name: &str) -> Option<i64> {
    let prefix = PREFIX_EXPIRE.iter().find(|p| name.starts_with(**p))?;
    let source = name.strip_prefix(prefix)?.trim();
    let time = NaiveDateTime::parse_from_str(source, EXPIRE_FORMAT).ok()?;
    Some(time.and_utc().timestamp_millis())
}
