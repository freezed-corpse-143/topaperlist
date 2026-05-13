-- 会议等级排除过滤（大小写不敏感精确匹配）。
-- 排除所有给定等级。
-- 占位符: {inner} 替换为内层查询，{values} 替换为 != 条件列表。
SELECT * FROM {inner}
WHERE {conditions}
