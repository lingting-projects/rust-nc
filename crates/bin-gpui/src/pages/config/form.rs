use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::IndexPath;
use gpui_kit::component::combobox::ComboboxState;
use gpui_kit::component::searchable_list::{
    SearchableListChange, SearchableListDelegate, SearchableListItem,
};
use gpui_kit::component::select::SearchableVec;
use gpui_kit::*;

use types_core::dto::config::ConfigUpsertDTO;
use types_core::entity::config::Config;
use types_core::entity::rule::Rule;
use types_core::entity::subscribe::Subscribe;

use crate::utils::format;

/// 下拉项的取值：下拉组件按文本值比对，id 在界面上一律用文本形式承载。
pub fn id_value(id: i64) -> SharedString {
    id.to_string().into()
}

/// 订阅下拉的一行：名称 + 剩余流量，选择时能区分同名订阅。
///
/// 值保存订阅 id 的文本形式：下拉组件要求值可比对，id 本身是整型。
#[derive(Clone)]
pub struct SubscribeItem {
    id: SharedString,
    name: SharedString,
    traffic: SharedString,
}

impl SearchableListItem for SubscribeItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        row(&self.name, &self.traffic, cx)
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// 订阅下拉的数据源。
pub type SubscribeDelegate = SearchableVec<SubscribeItem>;

/// 由订阅列表构建下拉数据源。
pub fn subscribe_delegate(items: &[Subscribe]) -> SubscribeDelegate {
    SearchableVec::new(
        items
            .iter()
            .map(|item| SubscribeItem {
                id: id_value(item.id),
                name: item.name.clone().into(),
                traffic: format::remaining(item.max, item.upload + item.download).into(),
            })
            .collect::<Vec<_>>(),
    )
}

/// 规则下拉的一行：名称 + 规则数量。
#[derive(Clone)]
pub struct RuleItem {
    id: SharedString,
    name: SharedString,
    count: SharedString,
}

impl SearchableListItem for RuleItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        row(&self.name, &self.count, cx)
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// 一行下拉内容：主要文字 + 次要信息。
fn row(name: &SharedString, hint: &SharedString, cx: &App) -> impl IntoElement {
    h_flex().gap_2().child(name.clone()).child(
        div()
            .text_color(cx.theme().muted_foreground)
            .child(hint.clone()),
    )
}

/// 三个规则框互相持有的弱引用：每个框都能读到其它框当前已选的规则。
///
/// 读实时值而不是缓存快照，清空按钮等不走选中回调的路径也不会漏。
pub type RulePeers = Rc<RefCell<Vec<Option<WeakEntity<ComboboxState<RuleDelegate>>>>>>;

/// 规则框的槽位数量：直连、代理、拒绝。
pub const RULE_SLOTS: usize = 3;
/// 直连规则框的槽位。
pub const RULE_SLOT_DIRECT: usize = 0;
/// 代理规则框的槽位。
pub const RULE_SLOT_PROXY: usize = 1;
/// 拒绝规则框的槽位。
pub const RULE_SLOT_REJECT: usize = 2;

/// 建立三个规则框互相引用的表，槽位在状态创建后依次登记。
pub fn rule_peers() -> RulePeers {
    Rc::new(RefCell::new(vec![None; RULE_SLOTS]))
}

/// 把刚建好的规则框登记到互相引用的表里。
pub fn rule_register(peers: &RulePeers, slot: usize, state: &Entity<ComboboxState<RuleDelegate>>) {
    if let Some(entry) = peers.borrow_mut().get_mut(slot) {
        *entry = Some(state.downgrade());
    }
}

/// 规则下拉的数据源。
///
/// 已被其它规则框选中的规则仍然列出，但置灰、不可再选：用户能看到规则被哪个框占用。
#[derive(Clone)]
pub struct RuleDelegate {
    items: SearchableVec<RuleItem>,
    /// 三个规则框互相引用的表。
    peers: RulePeers,
    /// 当前规则框在表中的槽位。
    slot: usize,
    /// 最近一次渲染时算出的、被其它框占用的规则 id。
    ///
    /// 选中回调拿不到 `cx`，读不了别的下拉，只能用渲染时算好的这份。
    taken: Rc<RefCell<Vec<SharedString>>>,
}

impl RuleDelegate {
    /// 由其它两个规则框当前的选中值算出被占用的规则。
    fn occupied(&self, cx: &App) -> Vec<SharedString> {
        self.peers
            .borrow()
            .iter()
            .enumerate()
            .filter(|(slot, _)| *slot != self.slot)
            .filter_map(|(_, peer)| peer.as_ref()?.upgrade())
            .flat_map(|peer| peer.read(cx).selected_values())
            .collect()
    }

    /// 该规则是否已被其它规则框选中。
    fn taken(&self, id: &SharedString) -> bool {
        self.taken.borrow().contains(id)
    }
}

impl SearchableListDelegate for RuleDelegate {
    type Item = RuleItem;

    fn items_count(&self, section: usize) -> usize {
        self.items.items_count(section)
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.items.item(ix)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        self.items.position(value)
    }

    fn perform_search(&mut self, query: &str, window: &mut Window, cx: &mut App) -> Task<()> {
        self.items.perform_search(query, window, cx)
    }

    /// 每次渲染都重算占用表，顺带刷新供选中回调使用的副本。
    fn is_item_enabled(&self, _: IndexPath, item: &Self::Item, cx: &App) -> bool {
        let occupied = self.occupied(cx);
        let enabled = !occupied.contains(&item.id);
        *self.taken.borrow_mut() = occupied;
        enabled
    }

    /// 已被其它规则框选中的规则不允许选中：选中请求直接丢弃。
    ///
    /// 置灰的行走不到键盘确认，但鼠标点击仍会进来，因此这里再拦一次。
    fn on_will_change(
        &mut self,
        selection: &mut Vec<(IndexPath, Self::Item)>,
        changes: &[SearchableListChange],
    ) {
        for change in changes {
            let SearchableListChange::Select { index } = change else {
                continue;
            };

            if self.item(*index).is_some_and(|item| self.taken(&item.id)) {
                return;
            }
        }

        SearchableVec::on_will_change(&mut self.items, selection, changes);
    }
}

/// 由规则列表构建下拉数据源，`slot` 指明这是第几个规则框。
pub fn rule_delegate(items: &[Rule], peers: RulePeers, slot: usize) -> RuleDelegate {
    RuleDelegate {
        items: SearchableVec::new(
            items
                .iter()
                .map(|item| RuleItem {
                    id: id_value(item.id),
                    name: item.name.clone().into(),
                    count: format!("{} 条", item.count).into(),
                })
                .collect::<Vec<_>>(),
        ),
        peers,
        slot,
        taken: Rc::new(RefCell::new(Vec::new())),
    }
}

/// 建立多选规则下拉的状态，`selected` 为已选规则 id。
///
/// 三个规则框共用一份 [`RulePeers`]，`slot` 指明当前框的槽位：
/// 已被其它框选中的规则会置灰，同一条规则只能被一个规则框选中。
pub fn rule_state(
    items: &[Rule],
    peers: RulePeers,
    slot: usize,
    selected: &[i64],
    window: &mut Window,
    cx: &mut App,
) -> Entity<ComboboxState<RuleDelegate>> {
    let delegate = rule_delegate(items, peers, slot);
    let indices = selected
        .iter()
        .filter_map(|id| delegate.position(&id_value(*id)))
        .collect::<Vec<_>>();

    cx.new(|cx| {
        ComboboxState::new(delegate, indices, window, cx)
            .multiple(true)
            .searchable(true)
    })
}

/// 表单里除输入框以外的字段。
///
/// 布尔开关与区域多选都在弹窗内就地修改，因此需要一份可变的实体承载，
/// 下拉框的选中值直接读它们自己的状态，不在这里重复保存。
pub struct ConfigForm {
    pub tun: bool,
    pub ipv6: bool,
    pub fake_ip: bool,
    pub geo_cn: bool,
    /// 包含: 是否包含无区域节点
    pub include_area_non: bool,
    /// 包含区域，ISO 代码
    pub include_area: Vec<String>,
    /// 排除区域，ISO 代码
    pub exclude_area: Vec<String>,
}

impl ConfigForm {
    /// 由实体裁剪出编辑所需的字段。
    pub fn of(config: &Config) -> Self {
        Self {
            tun: config.tun,
            ipv6: config.ipv6,
            fake_ip: config.fake_ip,
            geo_cn: config.geo_cn,
            include_area_non: config.include_area_non,
            include_area: config.include_area.clone(),
            exclude_area: config.exclude_area.clone(),
        }
    }

    /// 新增配置的默认值，与业务层的默认入参一致。
    pub fn new() -> Self {
        let dto = ConfigUpsertDTO::default();
        Self {
            tun: dto.tun,
            ipv6: dto.ipv6,
            fake_ip: dto.fake_ip,
            geo_cn: dto.geo_cn,
            include_area_non: dto.include_area_non,
            include_area: dto.include_area,
            exclude_area: dto.exclude_area,
        }
    }
}
