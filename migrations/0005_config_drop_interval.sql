-- 配置不再有刷新间隔: 配置只靠新增/修改、手动同步、依赖的订阅或规则更新时自动刷新。
-- 索引先删, 否则列上还有索引会让 drop column 失败。
drop index if exists idx_tbl2_config_refresh;
alter table tbl2_config drop column `interval`;
