-- 新结构: 前缀统一为 tbl2, 业务主键由文本编码改为整型雪花 ID。
-- 说明: 旧表原样保留, 本脚本只新建新表并把旧数据复制过来, 便于回滚核对。
-- 复制口径: 默认全部字段同步, 特例见各表的注释。

-- 应用键值配置
create table if not exists tbl2_app_config
(
    key   text primary key,
    value text
);

-- 订阅
create table if not exists tbl2_subscribe
(
    id           integer primary key,
    name         text,
    -- 订阅地址
    url          text,
    -- 订阅地址返回的完整内容
    content      text,
    -- 订阅解析后的所有节点
    nodes        text,
    -- 订阅源类型
    source       text,
    -- 刷新间隔, 单位: 毫秒
    interval     integer,
    -- 更新时间: 毫秒级别时间戳
    update_time  integer,
    -- 创建时间: 毫秒级别时间戳
    create_time  integer,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time integer,
    -- 下载流量, 单位: Bytes
    download     integer,
    -- 上传流量, 单位: Bytes
    upload       integer,
    -- 最大可用流量, 单位: Bytes
    max          integer,
    -- 过期时间: 毫秒级别时间戳
    expire_time  integer
);

-- 规则
create table if not exists tbl2_rule
(
    id            integer primary key,
    name          text,
    -- 规则订阅地址, 空表示本地规则
    url           text,
    -- 订阅完整内容(url返回或者本地编辑的)
    content       text,
    -- 刷新间隔, 单位: 毫秒
    interval      integer,
    -- 更新时间: 毫秒级别时间戳
    update_time   integer,
    -- 创建时间: 毫秒级别时间戳
    create_time   integer,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time  integer,
    -- 可用规则数量
    count         integer,
    -- 进程规则数量
    count_process integer,
    -- IP 规则数量
    count_ip      integer,
    -- 其他规则数量
    count_other   integer
);

-- 内核使用的配置
create table if not exists tbl2_config
(
    id                     integer primary key,
    name                   text,
    tun                    integer,
    fake_ip                integer,
    ipv6                   integer,
    -- 订阅
    subscribe_id           integer,
    -- geo cn 直连
    geo_cn                 integer,
    -- 规则 id 集合, 以整型 JSON 数组存储
    rule_direct_ids        text,
    rule_proxy_ids         text,
    rule_reject_ids        text,
    -- 包含
    include_area_non       integer,
    -- 包含指定区域 json字符串
    include_area           text,
    -- 包含名称中存在关键字的 json字符串
    include_name_contains  text,
    -- 排除
    -- 排除指定区域 json字符串
    exclude_area           text,
    -- 排除名称中存在关键字的 json字符串
    exclude_name_contains  text,
    -- 刷新间隔, 单位: 毫秒
    interval               integer,
    -- 刷新时间: 毫秒级别时间戳
    refresh_time           integer,
    -- 更新时间: 毫秒级别时间戳
    update_time            integer,
    -- 创建时间: 毫秒级别时间戳
    create_time            integer
);

-- 内核内置规则, 名称为 `geoip-cn` 这类语义键, 文件名与内核引用都按名称
create table if not exists tbl2_inner_rule
(
    id    integer primary key,
    -- 语义键: 规则集名称, 同时是本地文件名
    name  text,
    -- 原始下载地址, 不写入加速镜像前缀
    url   text,
    -- 最近一次下载成功时远端返回的 etag
    etag  text,
    -- 是否已就绪: 0 未就绪, 1 本地文件可用
    ready integer not null default 0
);

-- 刷新轮询按 (refresh_time, interval) 过滤, 建立覆盖索引避免全表扫描
create index if not exists idx_tbl2_subscribe_refresh on tbl2_subscribe (refresh_time, interval);
create index if not exists idx_tbl2_rule_refresh on tbl2_rule (refresh_time, interval);
create index if not exists idx_tbl2_config_refresh on tbl2_config (refresh_time, interval);
-- 内置规则按名称登记, 名称唯一; 配置侧只按引用校验, 内置规则自始至终只登记一次
create unique index if not exists idx_tbl2_inner_rule_name on tbl2_inner_rule (name);

-- 应用配置: 键值全部同步
insert into tbl2_app_config (`key`, `value`)
select `key`, `value`
from app_config;

-- 订阅: 不同步 nodes 与 refresh_time, source 统一为 unknown(由节点自身内容重新识别)
insert into tbl2_subscribe (`id`, `name`, `url`, `content`, `source`, `interval`, `update_time`,
                            `create_time`, `download`, `upload`, `max`, `expire_time`)
select cast(`id` as integer), `name`, `url`, `content`, 'unknown', `interval`, `update_time`,
       `create_time`, `download`, `upload`, `max`, `expire_time`
from tbl_subscribe;

-- 规则: 不同步 refresh_time
insert into tbl2_rule (`id`, `name`, `url`, `content`, `interval`, `update_time`, `create_time`,
                       `count`, `count_process`, `count_ip`, `count_other`)
select cast(`id` as integer), `name`, `url`, `content`, `interval`, `update_time`, `create_time`,
       `count`, `count_process`, `count_ip`, `count_other`
from tbl_rule;

-- 配置: 不同步 refresh_time; 订阅与规则 id 转为整型, 空订阅转为 NULL
insert into tbl2_config (`id`, `name`, `tun`, `fake_ip`, `ipv6`, `subscribe_id`, `geo_cn`,
                         `rule_direct_ids`, `rule_proxy_ids`, `rule_reject_ids`,
                         `include_area_non`, `include_area`, `include_name_contains`,
                         `exclude_area`, `exclude_name_contains`,
                         `interval`, `update_time`, `create_time`)
select cast(`id` as integer),
       `name`,
       `tun`,
       `fake_ip`,
       `ipv6`,
       case
           when `subscribe_id` is null or trim(`subscribe_id`) = '' then null
           else cast(`subscribe_id` as integer)
           end,
       `geo_cn`,
       case
           when `rule_direct_ids` is null or trim(`rule_direct_ids`) = '' then '[]'
           else coalesce((select json_group_array(cast(`value` as integer))
                          from json_each(`tbl_config`.`rule_direct_ids`)), '[]')
           end,
       case
           when `rule_proxy_ids` is null or trim(`rule_proxy_ids`) = '' then '[]'
           else coalesce((select json_group_array(cast(`value` as integer))
                          from json_each(`tbl_config`.`rule_proxy_ids`)), '[]')
           end,
       case
           when `rule_reject_ids` is null or trim(`rule_reject_ids`) = '' then '[]'
           else coalesce((select json_group_array(cast(`value` as integer))
                          from json_each(`tbl_config`.`rule_reject_ids`)), '[]')
           end,
       `include_area_non`,
       `include_area`,
       `include_name_contains`,
       `exclude_area`,
       `exclude_name_contains`,
       `interval`,
       `update_time`,
       `create_time`
from tbl_config;

-- 内置规则: 旧 id 是语义键, 迁移到 name 列; 新 id 由 rowid 提供, 与雪花 ID 不会冲突
insert into tbl2_inner_rule (`id`, `name`, `url`, `etag`, `ready`)
select `rowid`, `id`, `url`, `etag`, `ready`
from tbl_inner_rule;
