use framework_proc_auto::auto_type;

use crate::entity::rule::Rule;

/// 新增或修改规则的入参。
///
/// `id` 为空表示新增，`content` 为空表示由 `url` 拉取。
#[auto_type]
pub struct RuleUpsertDTO {
    pub id: Option<i64>,
    pub name: String,
    pub url: String,
    pub content: Option<String>,
    pub interval: i64,
}

/// 规则刷新所需的字段，不含统计数量。
#[auto_type]
pub struct RuleRefreshDTO {
    pub id: i64,
    pub name: String,
    pub url: String,
    /// 远端规则时为空，避免把已保存的内容读回内存
    pub content: String,
}

impl RuleRefreshDTO {
    /// 由完整实体裁剪出刷新所需字段。
    pub fn of(entity: Rule) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            url: entity.url,
            content: entity.content,
        }
    }
}
