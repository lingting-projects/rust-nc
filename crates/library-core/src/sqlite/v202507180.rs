use crate::app_config::AppConfig;
use crate::core::AnyResult;
use sqlite::ConnectionThreadSafe;

pub(super) fn init(conn: &ConnectionThreadSafe) -> AnyResult<()> {
    conn.execute(
        "
CREATE TABLE tbl_subscribe
(
    id   TEXT PRIMARY KEY,
    name TEXT,
    -- 订阅地址
    url TEXT,
    -- 订阅地址返回完整内容
    content TEXT,
    -- 订阅解析后的所有节点
    nodes TEXT,
    -- 刷新间隔, 单位: 毫秒
    interval INTEGER,
    -- 更新时间: 毫秒级别时间戳
    update_time INTEGER,
    -- 创建时间: 毫秒级别时间戳
    create_time INTEGER,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time INTEGER,
    -- 下载流量, 单位: Bytes
    download INTEGER,
    -- 上传流量, 单位: Bytes
    upload INTEGER,
    -- 最大可用流量, 单位: Bytes
    max INTEGER,
    -- 过期时间: 毫秒级别时间戳
    expire_time INTEGER
);

CREATE TABLE tbl_rule
(
    id   TEXT PRIMARY KEY,
    name TEXT,
    -- 规则订阅地址, 空表示本地规则
    url TEXT,
    -- 订阅完整内容(url返回或者本地编辑的)
    content TEXT,
    -- 刷新间隔, 单位: 毫秒
    interval INTEGER,
    -- 更新时间: 毫秒级别时间戳
    update_time INTEGER,
    -- 创建时间: 毫秒级别时间戳
    create_time INTEGER,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time INTEGER,
    -- 可用规则数量
    count INTEGER,
    -- 进程规则数量
    count_process INTEGER,
    -- IP规则数量
    count_ip INTEGER,
    -- 其他规则数量
    count_other INTEGER
);

-- 内核使用的配置
CREATE TABLE tbl_config
(
    id   TEXT PRIMARY KEY,
    name TEXT,
    tun INTEGER,
    fake_ip INTEGER,
    ipv6 INTEGER,
    -- 订阅
    subscribe_id TEXT,
    -- geo cn 直连
    geo_cn INTEGER,
    -- 规则 id1,id2
    rule_direct_ids TEXT,
    rule_proxy_ids TEXT,
    rule_reject_ids TEXT,
    -- 包含
    include_area_non INTEGER,
    -- 包含指定区域 json字符串
    include_area TEXT,
    -- 包含名称中存在关键的 json字符串
    include_name_contains TEXT,
    -- 排除
    -- 排除指定区域 json字符串
    exclude_area TEXT,
    -- 排除名称中存在关键的 json字符串
    exclude_name_contains TEXT,
    -- 刷新间隔, 单位: 毫秒
    interval INTEGER,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time INTEGER,
    -- 更新时间: 毫秒级别时间戳
    update_time INTEGER,
    -- 创建时间: 毫秒级别时间戳
    create_time INTEGER
);
        ",
    )?;

    AppConfig::version_set(20250718)
}
