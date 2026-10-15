use framework_proc_auto::auto_type;

use crate::entity::subscribe::Subscribe;

/// 新增或修改订阅的入参。
///
/// `id` 为空表示新增，`content` 为空表示由 `url` 拉取。
#[auto_type]
pub struct SubscribeUpsertDTO {
    pub id: Option<String>,
    pub name: String,
    pub url: String,
    pub content: Option<String>,
    pub interval: i64,
}

/// 订阅刷新所需的字段，不含解析后的节点。
#[auto_type]
pub struct SubscribeRefreshDTO {
    pub id: String,
    pub name: String,
    pub url: String,
    /// 远端订阅时为空，避免把已保存的内容读回内存
    pub content: String,
}

impl SubscribeRefreshDTO {
    /// 由完整实体裁剪出刷新所需字段。
    pub fn of(entity: Subscribe) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            url: entity.url,
            content: entity.content,
        }
    }
}
