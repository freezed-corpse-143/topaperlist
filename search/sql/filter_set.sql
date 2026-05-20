-- Generic set membership filter (exact match, optional case folding).
-- Placeholders:
--   {inner}       Inner subquery (or "papers" for the first filter)
--   {column_expr} Column reference, may include LOWER() wrapper
--   {op}          IN or NOT IN
--   {values}      Comma-separated bind placeholders (e.g. ?1,?2)
--
-- Used for: level, conference, year — both include and exclude.
SELECT * FROM {inner}
WHERE {column_expr} {op} ({values})
