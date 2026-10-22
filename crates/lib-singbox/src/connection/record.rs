//! 单条连接的数据。

use crate::grpc::{Connection, ConnectionEvent, ConnectionEventType};

/// 一条内核连接。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    /// 连接 id，断开连接时使用。
    pub id: String,
    /// 入站标签，即流量从哪个入站进入。
    pub inbound: String,
    /// 入站类型，用于在详情里说明入站形态。
    pub inbound_type: String,
    /// 网络类型，例如 `tcp`、`udp`。
    pub network: String,
    /// 进程名，取不到时为空。
    pub process: String,
    /// 目标地址，域名请求优先展示域名。
    pub destination: String,
    /// 命中的规则，未命中时为空。
    pub rule: String,
    /// 出站链路，从第一个出站到最后一个出站。
    pub chain: Vec<String>,
    /// 源地址。
    pub source: String,
    /// 连接建立时间，毫秒时间戳。
    pub created_at: i64,
    /// 连接关闭时间，毫秒时间戳，仍打开时为 0。
    pub closed_at: i64,
    /// 累计上传字节数。
    pub upload: u64,
    /// 累计下载字节数。
    pub download: u64,
}

impl ConnectionInfo {
    /// 由内核连接快照转换。
    pub(crate) fn of(connection: Connection) -> Self {
        // 域名请求的 destination 是解析后的地址，域名本身在 domain 里，展示时优先用域名。
        let destination = match connection.domain.is_empty() {
            true => connection.destination,
            false => connection.domain,
        };

        Self {
            id: connection.id,
            inbound: connection.inbound,
            inbound_type: connection.inbound_type,
            network: connection.network,
            process: connection
                .process_info
                .map(|info| info.process_path)
                .unwrap_or_default(),
            destination,
            rule: connection.rule,
            chain: connection.chain_list,
            source: connection.source,
            created_at: connection.created_at,
            closed_at: connection.closed_at,
            upload: connection.uplink_total.max(0) as u64,
            download: connection.downlink_total.max(0) as u64,
        }
    }

    /// 事件类型，未知取值按新增处理。
    pub(crate) fn kind(event: &ConnectionEvent) -> ConnectionEventType {
        ConnectionEventType::try_from(event.r#type)
            .unwrap_or(ConnectionEventType::ConnectionEventNew)
    }
}
