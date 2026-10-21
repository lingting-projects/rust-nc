use anyhow::{Context as _, Result, bail};
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};

/// 由 `proto/started_service.proto` 生成的 gRPC 客户端代码。
pub mod proto {
    tonic::include_proto!("daemon");
}

pub use proto::started_service_client::StartedServiceClient;
pub use proto::{
    CloseConnectionRequest, Connection, ConnectionEvent, ConnectionEventType, Group, GroupItem,
    SelectOutboundRequest, Status, SubscribeConnectionsRequest, SubscribeStatusRequest,
    UrlTestRequest, Version,
};

/// 连接信息快照。
///
/// 连接订阅是流式的，每次推送都包含增量事件；这里把首个推送解析为
/// 「当前仍处于打开状态的连接」列表，供界面一次性展示。
#[derive(Debug, Clone, Default)]
pub struct ConnectionSnapshot {
    /// 当前打开的连接。
    pub connections: Vec<Connection>,
}

/// sing-box 内核的 gRPC 客户端。
///
/// 内核从 1.14.0 起提供 gRPC API 服务（配置中的 `services[].type = "api"`），
/// 监听明文 HTTP/2，因此这里按 `http://<host>:<port>` 建连；
/// 若配置了密钥，则以 `authorization: Bearer <secret>` 元数据认证。
#[derive(Clone)]
pub struct SingBoxGrpc {
    /// 已建连的通道，由 tonic 自行复用底层连接。
    channel: Channel,
    /// API 密钥，为空表示无需认证。
    secret: Option<MetadataValue<tonic::metadata::Ascii>>,
}

impl SingBoxGrpc {
    /// 按监听地址创建客户端。
    ///
    /// `address` 形如 `127.0.0.1:9090`，缺少协议前缀时按 `http://` 处理。
    pub fn new(address: &str) -> Result<Self> {
        Self::with_secret(address, None)
    }

    /// 按监听地址与密钥创建客户端。
    pub fn with_secret(address: &str, secret: Option<&str>) -> Result<Self> {
        let url = Self::url(address)?;
        let endpoint = Endpoint::from_shared(url.clone())
            .with_context(|| format!("内核 API 地址非法: {url}"))?
            // 连接按需建立, 避免内核刚启动尚未监听时直接失败
            .connect_timeout(std::time::Duration::from_secs(3));

        let secret = match secret.filter(|value| !value.is_empty()) {
            Some(value) => Some(
                format!("Bearer {value}")
                    .parse()
                    .context("内核 API 密钥含非法字符")?,
            ),
            None => None,
        };

        Ok(Self {
            channel: endpoint.connect_lazy(),
            secret,
        })
    }

    /// 规范化监听地址为完整的 URL。
    fn url(address: &str) -> Result<String> {
        let address = address.trim();
        if address.is_empty() {
            bail!("内核 API 地址为空");
        }
        if address.contains("://") {
            return Ok(address.to_string());
        }
        Ok(format!("http://{address}"))
    }

    /// 为请求附加认证元数据。
    fn request<T>(&self, payload: T) -> tonic::Request<T> {
        let mut request = tonic::Request::new(payload);
        if let Some(secret) = &self.secret {
            request
                .metadata_mut()
                .insert("authorization", secret.clone());
        }
        request
    }

    /// 取得服务客户端。
    fn client(&self) -> StartedServiceClient<Channel> {
        StartedServiceClient::new(self.channel.clone())
    }

    /// 读取内核版本，用于确认内核 API 已就绪。
    pub async fn version(&self) -> Result<Version> {
        let version = self
            .client()
            .get_version(self.request(()))
            .await
            .context("读取内核版本失败")?
            .into_inner();
        Ok(version)
    }

    /// 读取一次内核运行状态快照。
    ///
    /// 内核的 `SubscribeStatus` 是流式接口，但界面按秒轮询即可，
    /// 因此这里只取首个推送后立即断开，避免长期持有一条流。
    pub async fn status(&self) -> Result<Status> {
        let request = SubscribeStatusRequest {
            interval: std::time::Duration::from_secs(1).as_millis() as i64,
        };
        let mut stream = self
            .client()
            .subscribe_status(self.request(request))
            .await
            .context("订阅内核状态失败")?
            .into_inner();

        let status = stream
            .message()
            .await
            .context("读取内核状态失败")?
            .context("内核未推送状态")?;
        Ok(status)
    }

    /// 读取出站分组，分组内的节点即节点切换的候选项。
    pub async fn groups(&self) -> Result<Vec<Group>> {
        let mut stream = self
            .client()
            .subscribe_groups(self.request(()))
            .await
            .context("订阅内核分组失败")?
            .into_inner();

        let groups = stream
            .message()
            .await
            .context("读取内核分组失败")?
            .map(|groups| groups.group)
            .unwrap_or_default();
        Ok(groups)
    }

    /// 在分组 `group_tag` 中切换到节点 `outbound_tag`。
    pub async fn select(&self, group_tag: &str, outbound_tag: &str) -> Result<()> {
        let request = SelectOutboundRequest {
            group_tag: group_tag.to_string(),
            outbound_tag: outbound_tag.to_string(),
        };
        self.client()
            .select_outbound(self.request(request))
            .await
            .with_context(|| format!("切换节点失败: {group_tag} -> {outbound_tag}"))?;
        Ok(())
    }

    /// 对节点 `outbound_tag` 触发一次延迟测试。
    pub async fn url_test(&self, outbound_tag: &str) -> Result<()> {
        let request = UrlTestRequest {
            outbound_tag: outbound_tag.to_string(),
        };
        self.client()
            .url_test(self.request(request))
            .await
            .with_context(|| format!("节点测速失败: {outbound_tag}"))?;
        Ok(())
    }

    /// 读取当前连接信息快照。
    ///
    /// `interval` 为内核推送增量事件的间隔，取 0 表示由内核决定。
    pub async fn connections(&self, interval: std::time::Duration) -> Result<ConnectionSnapshot> {
        let request = SubscribeConnectionsRequest {
            interval: interval.as_millis() as i64,
        };
        let mut stream = self
            .client()
            .subscribe_connections(self.request(request))
            .await
            .context("订阅内核连接失败")?
            .into_inner();

        let events = stream
            .message()
            .await
            .context("读取内核连接失败")?
            .map(|events| events.events)
            .unwrap_or_default();

        // 首个推送中的新增事件即当前打开的连接, 已关闭的忽略
        let connections = events
            .into_iter()
            .filter(|event| event.kind() != ConnectionEventType::ConnectionEventClosed)
            .filter_map(|event| event.connection)
            .collect();
        Ok(ConnectionSnapshot { connections })
    }

    /// 关闭指定连接。
    pub async fn close_connection(&self, id: &str) -> Result<()> {
        let request = CloseConnectionRequest { id: id.to_string() };
        self.client()
            .close_connection(self.request(request))
            .await
            .with_context(|| format!("关闭连接失败: {id}"))?;
        Ok(())
    }

    /// 关闭全部连接。
    pub async fn close_all_connections(&self) -> Result<()> {
        self.client()
            .close_all_connections(self.request(()))
            .await
            .context("关闭全部连接失败")?;
        Ok(())
    }
}

impl ConnectionEvent {
    /// 事件类型，未知取值按新增处理。
    fn kind(&self) -> ConnectionEventType {
        ConnectionEventType::try_from(self.r#type)
            .unwrap_or(ConnectionEventType::ConnectionEventNew)
    }
}

impl std::fmt::Debug for SingBoxGrpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingBoxGrpc")
            .field("secret", &self.secret.is_some())
            .finish_non_exhaustive()
    }
}
