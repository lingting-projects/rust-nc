-- 内核内置规则
-- 内核配置中引用的远程规则集, 由 InnerRuleSync 提前下载到本地,
-- 配置只引用本地文件, 内核启动时不再联网下载, 避免拖住 API 服务启动。
create table if not exists tbl_inner_rule
(
    id    text primary key,
    -- 原始下载地址, 不写入加速镜像前缀, 镜像在下载时按需拼接
    url   text,
    -- 最近一次下载成功时远端返回的 etag
    etag  text,
    -- 是否已就绪: 0 未就绪, 1 本地文件可用; 内核启动前要求全部为 1
    ready integer not null default 0
);
