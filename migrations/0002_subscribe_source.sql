-- 订阅源类型
-- 说明: 与节点一起落库, 决定已保存节点的键如何解析。
-- 旧库新增该列时历史数据为 NULL, 读取时视为 unknown, 由节点自身内容判定来源。
alter table tbl_subscribe
    add column source text;
