-- Generic substring match filter (LIKE / NOT LIKE with % wildcards).
-- Placeholders:
--   {inner}   Inner subquery (or "papers" for the first filter)
--   {clauses} AND-connected LIKE or NOT LIKE conditions
--             (e.g. "LOWER(title) LIKE ?1 AND LOWER(title) NOT LIKE ?2")
--
-- Used for: title — both include and exclude.
SELECT * FROM {inner}
WHERE {clauses}
