-- lingting-nc 初始化脚本
-- 说明：结构与历史 sqlite 库保持一致，全部使用 if not exists，旧库可直接升级。
-- 时间字段统一为毫秒级时间戳，布尔字段使用 0/1 整数，字符串数组使用 JSON 文本。

-- 应用键值配置
create table if not exists app_config
(
    key   text primary key,
    value text
);

-- 订阅
create table if not exists tbl_subscribe
(
    id   text primary key,
    name text,
    -- 订阅地址
    url text,
    -- 订阅地址返回的完整内容
    content text,
    -- 订阅解析后的所有节点
    nodes text,
    -- 刷新间隔, 单位: 毫秒
    interval integer,
    -- 更新时间: 毫秒级别时间戳
    update_time integer,
    -- 创建时间: 毫秒级别时间戳
    create_time integer,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time integer,
    -- 下载流量, 单位: Bytes
    download integer,
    -- 上传流量, 单位: Bytes
    upload integer,
    -- 最大可用流量, 单位: Bytes
    max integer,
    -- 过期时间: 毫秒级别时间戳
    expire_time integer
);

-- 规则
create table if not exists tbl_rule
(
    id   text primary key,
    name text,
    -- 规则订阅地址, 空表示本地规则
    url text,
    -- 订阅完整内容(url返回或者本地编辑的)
    content text,
    -- 刷新间隔, 单位: 毫秒
    interval integer,
    -- 更新时间: 毫秒级别时间戳
    update_time integer,
    -- 创建时间: 毫秒级别时间戳
    create_time integer,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time integer,
    -- 可用规则数量
    count integer,
    -- 进程规则数量
    count_process integer,
    -- IP 规则数量
    count_ip integer,
    -- 其他规则数量
    count_other integer
);

-- 内核使用的配置
create table if not exists tbl_config
(
    id   text primary key,
    name text,
    tun integer,
    fake_ip integer,
    ipv6 integer,
    -- 订阅
    subscribe_id text,
    -- geo cn 直连
    geo_cn integer,
    -- 规则 id1,id2
    rule_direct_ids text,
    rule_proxy_ids text,
    rule_reject_ids text,
    -- 包含
    include_area_non integer,
    -- 包含指定区域 json字符串
    include_area text,
    -- 包含名称中存在关键字的 json字符串
    include_name_contains text,
    -- 排除
    -- 排除指定区域 json字符串
    exclude_area text,
    -- 排除名称中存在关键字的 json字符串
    exclude_name_contains text,
    -- 刷新间隔, 单位: 毫秒
    interval integer,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time integer,
    -- 更新时间: 毫秒级别时间戳
    update_time integer,
    -- 创建时间: 毫秒级别时间戳
    create_time integer
);

-- 刷新轮询按 (refresh_time, interval) 过滤, 建立覆盖索引避免全表扫描
create index if not exists idx_tbl_subscribe_refresh on tbl_subscribe (refresh_time, interval);
create index if not exists idx_tbl_rule_refresh on tbl_rule (refresh_time, interval);
create index if not exists idx_tbl_config_refresh on tbl_config (refresh_time, interval);
