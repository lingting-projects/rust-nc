-- 配置新增「进程优先」开关。
-- 默认 0 保持原行为: dns 只按 ip/域名规则集分流, 进程访问直连域名时按域名规则直连。
-- 开启后 dns 追加进程规则集, 规则顺序与内核路由一致:
-- 拒绝 -> 直连进程 -> 代理进程 -> 直连其他 -> 代理其他。
alter table tbl2_config add column process_first integer not null default 0;
