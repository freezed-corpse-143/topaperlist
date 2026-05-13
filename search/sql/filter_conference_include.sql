-- 会议名称包含过滤（大小写不敏感精确匹配）。
-- 匹配任一给定会议名即为命中。
-- 占位符: {inner} 替换为内层查询，{values} 替换为逗号分隔的占位符列表。
SELECT * FROM {inner}
WHERE LOWER(conference) IN ({values})
