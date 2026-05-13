-- 最终投影：选择要显示的列并应用排序。
-- 占位符: {inner} 替换为内层查询，{columns} 替换为逗号分隔的列名，{order} 替换为 ORDER BY 子句。
-- 列名可选: level, conference, year, title 以及 JSONL 中的任意数据字段。
SELECT {columns}
FROM ({inner})
{order}
