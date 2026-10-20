use std::cmp::Ordering;

use lib_core::constants;
use lib_pick::{PickNode, PickNodes};

/// 节点筛选条件, 多个参数值之间为并集。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeContains {
    /// 区域
    pub area: Vec<String>,
    /// 名称中包含的关键字
    pub name_contains: Vec<String>,
    /// 是否匹配无区域节点
    pub non_area: bool,
    /// 是否匹配无名称关键字的节点
    pub non_name: bool,
}

impl NodeContains {
    /// 包含条件默认值: 主区域, 保留无区域节点。
    pub fn default_include() -> Self {
        Self {
            area: constants::NODE_INCLUDE_AREA
                .iter()
                .map(|item| item.to_string())
                .collect(),
            name_contains: Vec::new(),
            non_area: constants::NODE_INCLUDE_AREA_NON,
            non_name: false,
        }
    }

    /// 排除条件默认值: 移除无区域与无名称关键字的节点。
    pub fn default_exclude() -> Self {
        Self {
            area: constants::NODE_EXCLUDE_AREA
                .iter()
                .map(|item| item.to_string())
                .collect(),
            name_contains: constants::NODE_EXCLUDE_NAME_CONTAINS
                .iter()
                .map(|item| item.to_string())
                .collect(),
            non_area: constants::NODE_EXCLUDE_AREA_NON,
            non_name: constants::NODE_EXCLUDE_NAME_NON,
        }
    }

    /// 条件数量。
    pub fn size(&self) -> usize {
        self.area.len() + self.name_contains.len()
    }

    /// 是否没有条件。
    pub fn is_empty(&self) -> bool {
        self.area.is_empty() && self.name_contains.is_empty()
    }

    /// 节点是否命中条件, `and` 为真时要求全部命中。
    pub fn is_match(&self, node: &PickNode, and: bool) -> bool {
        if self.is_empty() {
            return true;
        }

        if and {
            self.match_area(node) && self.match_name(node)
        } else {
            self.match_area(node) || self.match_name(node)
        }
    }

    fn match_area(&self, node: &PickNode) -> bool {
        if self.area.is_empty() {
            return true;
        }

        match node.area {
            None => self.non_area,
            Some(area) => self.area.iter().any(|code| code == area.iso),
        }
    }

    fn match_name(&self, node: &PickNode) -> bool {
        // 未设置名称关键字时不限制名称, 否则包含条件会因该维度无关键字而过滤掉全部节点
        if self.name_contains.is_empty() {
            return true;
        }

        let name = node.name();
        if self.name_contains.iter().any(|key| name.contains(key)) {
            return true;
        }

        // 名称未命中任何关键字时, 由 non_name 决定是否视为命中
        self.non_name
    }
}

/// 按包含与排除条件筛选, 并按主区域优先排序。
pub(crate) fn filter(
    nodes: PickNodes,
    include: &NodeContains,
    exclude: &NodeContains,
) -> Vec<PickNode> {
    let mut nodes = nodes.nodes;

    // 条件少的一侧先执行, 减少比较次数
    if include.size() > exclude.size() {
        nodes.retain(|node| include.is_match(node, true) && !exclude.is_match(node, false));
    } else {
        nodes.retain(|node| !exclude.is_match(node, false) && include.is_match(node, true));
    }

    nodes.sort_by(compare);
    nodes
}

/// 无区域节点在最前, 其次是主区域, 其余按区域码排序。
fn compare(left: &PickNode, right: &PickNode) -> Ordering {
    let main = constants::NODE_INCLUDE_AREA;

    let (Some(left_area), Some(right_area)) = (left.area, right.area) else {
        return match (left.area.is_some(), right.area.is_some()) {
            (false, false) => Ordering::Equal,
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            (true, true) => Ordering::Equal,
        };
    };

    let left_main = main.contains(&left_area.iso);
    let right_main = main.contains(&right_area.iso);
    if left_main != right_main {
        return if left_main {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    if left_main {
        let left_index = main.iter().position(|code| *code == left_area.iso);
        let right_index = main.iter().position(|code| *code == right_area.iso);
        return left_index.cmp(&right_index);
    }

    left_area.iso.cmp(right_area.iso)
}
