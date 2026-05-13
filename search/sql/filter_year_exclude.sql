-- 年份排除过滤（精确匹配）。
-- 排除所有给定年份。
-- 占位符: {inner} 替换为内层查询，{values} 替换为 != 条件列表。
SELECT * FROM {inner}
WHERE {conditions}
