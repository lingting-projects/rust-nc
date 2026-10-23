use framework_proc_auto::{auto_enum, auto_type};

/// 订阅源类型, 决定已保存的节点按键如何解析。
///
/// `unknown` 为默认值, 表示尚未识别出类型, 此时由节点自身内容判定。
/// strum 只取 `Display`, 文本还原由 [`SubscribeSource::parse`] 统一处理, 避免出现两套解析口径。
#[auto_enum(strum = false)]
#[derive(strum_macros::Display, Default)]
pub enum SubscribeSource {
    /// 未知
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
    /// clash 配置, 节点键为 clash 原生字段
    #[strum(serialize = "clash")]
    #[serde(rename = "clash")]
    Clash,
    /// 分享链接, 节点键为链接参数
    #[strum(serialize = "pure")]
    #[serde(rename = "pure")]
    Pure,
}

impl SubscribeSource {
    /// 由库中保存的文本还原源类型, 任何异常取值都归为 [`SubscribeSource::Unknown`]。
    ///
    /// 旧数据的该字段可能是 `NULL`、空串、前后带空白或其它无法识别的文本,
    /// 统一按未知处理, 由节点自身内容判定来源。
    pub fn parse(source: &str) -> Self {
        match source.trim().to_ascii_lowercase().as_str() {
            "clash" => Self::Clash,
            "pure" => Self::Pure,
            _ => Self::Unknown,
        }
    }
}

/// 订阅, 对应 tbl_subscribe。
///
/// `nodes` 为订阅解析后的节点 JSON 文本, 结构由订阅解析逻辑决定,
/// 键的解析方式见 [`SubscribeSource`]。
/// `download`、`upload`、`max` 单位为 Bytes, 时间字段为毫秒级时间戳。
#[auto_type(default = false)]
pub struct Subscribe {
    pub id: String,
    pub name: String,
    /// 订阅地址, 为空表示本地订阅
    pub url: String,
    /// 订阅地址返回的完整内容
    pub content: String,
    /// 订阅解析后的所有节点, JSON 文本
    pub nodes: String,
    /// 订阅源类型, 与节点一起落库
    pub source: SubscribeSource,
    /// 刷新间隔, 单位: 毫秒
    pub interval: i64,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: i64,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: i64,
    /// 刷新时间: 毫秒级别时间戳
    pub refresh_time: i64,
    /// 下载流量, 单位: Bytes
    pub download: u64,
    /// 上传流量, 单位: Bytes
    pub upload: u64,
    /// 最大可用流量, 单位: Bytes
    pub max: u64,
    /// 过期时间: 毫秒级别时间戳
    pub expire_time: i64,
}
