-- 会议名称排除过滤（大小写不敏感精确匹配）。
-- 排除所有给定会议名。
-- 占位符: {inner} 替换为内层查询，{values} 替换为逗号分隔的占位符列表。
SELECT * FROM {inner}
WHERE LOWER(conference) NOT IN ({values})
