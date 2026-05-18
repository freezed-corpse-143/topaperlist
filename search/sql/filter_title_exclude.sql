-- 标题排除关键词过滤（大小写不敏感子串匹配）。
-- 多个关键词为 AND 关系：标题不能包含任一关键词。
-- 占位符: {inner} 替换为内层查询，{not_like_clauses} 替换为 NOT LIKE 条件列表。
SELECT * FROM {inner}
WHERE {not_like_clauses}
