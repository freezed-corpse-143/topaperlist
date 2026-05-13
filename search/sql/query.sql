-- ============================================================
-- 组合查询管道：通过嵌套子查询串联各过滤器，最后投影选列。
-- ============================================================
--
-- 过滤器文件:
--   filter_title_include.sql       标题包含关键词 (LIKE '%kw%')
--   filter_title_exclude.sql       标题排除关键词 (NOT LIKE '%kw%')
--   filter_level_include.sql       等级包含 (IN)
--   filter_level_exclude.sql       等级排除 (!=)
--   filter_conference_include.sql  会议包含 (IN)
--   filter_conference_exclude.sql  会议排除 (!=)
--   filter_year_include.sql        年份包含 (IN)
--   filter_year_exclude.sql        年份排除 (!=)
--   projection.sql                  最终投影：选列 + 排序
--
-- 管道组合示例（level=A, conf=ICML,NeurIPS, year=2024, 标题含 diffusion 不含 survey）:
--
--   -- 1. 标题包含 (filter_title_include.sql)
--   SELECT * FROM papers WHERE LOWER(title) LIKE '%diffusion%'
--
--   -- 2. 标题排除 (filter_title_exclude.sql)
--   SELECT * FROM (...) WHERE LOWER(title) NOT LIKE '%survey%'
--
--   -- 3. 等级包含 (filter_level_include.sql)
--   SELECT * FROM (...) WHERE LOWER(level) IN ('a')
--
--   -- 4. 会议包含 (filter_conference_include.sql)
--   SELECT * FROM (...) WHERE LOWER(conference) IN ('icml','neurips')
--
--   -- 5. 年份包含 (filter_year_include.sql)
--   SELECT * FROM (...) WHERE year IN ('2024')
--
--   -- 6. 最终投影 (projection.sql)
--   SELECT conference, year, title FROM (...) ORDER BY year DESC, conference ASC
--
-- 新增过滤: 在 sql/ 下新增 .sql 文件，在 db.rs 的 query_records 中添加对应分支即可。

SELECT {columns}
FROM ({inner})
{order}
