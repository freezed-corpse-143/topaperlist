-- 标题包含关键词过滤（大小写不敏感子串匹配）。
-- 多个关键词为 AND 关系：标题必须同时包含所有关键词。
-- 占位符: {inner} 替换为内层查询（首次为 papers），{conditions} 替换为 LIKE 条件列表。
SELECT * FROM {inner}
WHERE {conditions}
