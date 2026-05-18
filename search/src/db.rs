use std::path::Path;

use crate::models::{Direction, Field, IndexedRecord, SortSpec};

/// Fixed provenance columns (derived from directory structure).
pub const FIXED_COLUMNS: &[&str] = &["level", "conference", "year"];

pub type Result<T> = std::result::Result<T, String>;

/// Log a debug message if RUST_LOG=debug is set.
macro_rules! debug {
    ($($arg:tt)*) => {
        if std::env::var("RUST_LOG").unwrap_or_default().to_ascii_lowercase() == "debug" {
            eprintln!("[DEBUG] {}", format!($($arg)*));
        }
    };
}
pub(crate) use debug;

/// Open (or create) the SQLite database.
pub fn open_db(db_path: &Path) -> Result<rusqlite::Connection> {
    debug!("打开数据库: {}", db_path.display());
    let conn = rusqlite::Connection::open(db_path).map_err(|e| format!("无法打开数据库文件: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| format!("无法设置 WAL 模式: {e}"))?;
    Ok(conn)
}

/// Create (or recreate) the papers table with given data columns.
pub fn create_table(conn: &rusqlite::Connection, data_columns: &[String]) -> Result<()> {
    debug!("建表，字段: {:?}", data_columns);

    conn.execute_batch("DROP TABLE IF EXISTS papers;")
        .map_err(|e| format!("无法删除旧表: {e}"))?;

    let mut col_defs: Vec<String> = vec!["id INTEGER PRIMARY KEY AUTOINCREMENT".to_string()];
    for fixed in FIXED_COLUMNS {
        col_defs.push(format!("`{fixed}` TEXT NOT NULL"));
    }
    for col in data_columns {
        let safe_col = col.replace('`', "``");
        col_defs.push(format!("`{safe_col}` TEXT NOT NULL DEFAULT ''"));
    }

    let create_sql = format!("CREATE TABLE papers ({});", col_defs.join(", "));
    debug!("建表 SQL: {create_sql}");

    conn.execute(&create_sql, [])
        .map_err(|e| format!("无法创建 papers 表: {e}"))?;

    // Indexes on provenance columns + title
    for fixed in FIXED_COLUMNS {
        conn.execute(
            &format!("CREATE INDEX IF NOT EXISTS idx_{fixed} ON papers(`{fixed}`);"),
            [],
        )
        .ok();
    }
    if data_columns.iter().any(|c| c == "title") {
        conn.execute("CREATE INDEX IF NOT EXISTS idx_title ON papers(title);", [])
            .ok();
    }

    debug!("表创建完成");
    Ok(())
}

/// Insert a batch of records into the database.
pub fn insert_records(
    conn: &rusqlite::Connection,
    records: &[IndexedRecord],
    data_columns: &[String],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("无法开启事务: {e}"))?;

    let all_columns: Vec<String> = FIXED_COLUMNS
        .iter()
        .map(|c| format!("`{c}`"))
        .chain(data_columns.iter().map(|c| format!("`{}`", c.replace('`', "``"))))
        .collect();

    let placeholders: Vec<String> = (1..=all_columns.len())
        .map(|i| format!("?{i}"))
        .collect();

    let insert_sql = format!(
        "INSERT INTO papers ({}) VALUES ({});",
        all_columns.join(", "),
        placeholders.join(", ")
    );

    let mut count = 0;
    {
        let mut stmt = tx
            .prepare(&insert_sql)
            .map_err(|e| format!("无法准备 INSERT 语句: {e}"))?;

        for record in records {
            let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            values.push(Box::new(record.level.clone()));
            values.push(Box::new(record.conference.clone()));
            values.push(Box::new(record.year.clone()));
            for col in data_columns {
                let val = record.data.get(col).cloned().unwrap_or_default();
                values.push(Box::new(val));
            }

            let params_refs: Vec<&dyn rusqlite::types::ToSql> =
                values.iter().map(|v| v.as_ref()).collect();

            stmt.execute(rusqlite::params_from_iter(params_refs))
                .map_err(|e| format!("插入记录失败: {e}"))?;
            count += 1;
        }
    }

    tx.commit()
        .map_err(|e| format!("无法提交事务: {e}"))?;

    debug!("成功插入 {count} 条记录");
    Ok(count)
}

/// Drop the papers table.
pub fn clear_db(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch("DROP TABLE IF EXISTS papers;")
        .map_err(|e| format!("清空数据库失败: {e}"))?;
    debug!("已清空数据库旧表");
    Ok(())
}

/// Build a query by nesting active filters as subquery layers
/// (see sql/filter_*.sql for the SQL templates corresponding to each filter).
///
/// The pipeline starts from `papers` and wraps each active filter as:
///     SELECT * FROM ({inner}) WHERE {filter_clause}
///
/// At the end, columns are selected and ordering is applied:
///     SELECT {columns} FROM ({inner}) {order_by}
pub fn query_records(
    conn: &rusqlite::Connection,
    title_include: &[String],
    title_exclude: &[String],
    level_include: &[String],
    level_exclude: &[String],
    conference_include: &[String],
    conference_exclude: &[String],
    year_include: &[String],
    year_exclude: &[String],
    sort_specs: &[SortSpec],
    display_fields: &[Field],
) -> Result<Vec<String>> {
    let mut bind_values: Vec<String> = Vec::new();
    // Start from the base table
    let mut inner = "papers".to_string();

    // ── Title include filter (see sql/filter_title_include.sql) ──
    if !title_include.is_empty() {
        let conditions: Vec<String> = title_include
            .iter()
            .map(|kw| {
                bind_values.push(format!("%{}%", kw));
                let idx = bind_values.len();
                format!("LOWER(title) LIKE ?{idx}")
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE {}",
            conditions.join(" AND ")
        );
        debug!("应用 filter_title_include: [{}]", title_include.join(", "));
    }

    // ── Title exclude filter (see sql/filter_title_exclude.sql) ──
    if !title_exclude.is_empty() {
        let conditions: Vec<String> = title_exclude
            .iter()
            .map(|kw| {
                bind_values.push(format!("%{}%", kw));
                let idx = bind_values.len();
                format!("LOWER(title) NOT LIKE ?{idx}")
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE {}",
            conditions.join(" AND ")
        );
        debug!("应用 filter_title_exclude: [{}]", title_exclude.join(", "));
    }

    // ── Level include filter (see sql/filter_level_include.sql) ──
    if !level_include.is_empty() {
        let values: Vec<String> = level_include
            .iter()
            .map(|v| {
                bind_values.push(v.clone());
                format!("?{}", bind_values.len())
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE LOWER(level) IN ({})",
            values.join(",")
        );
        debug!("应用 filter_level_include: [{}]", level_include.join(", "));
    }

    // ── Level exclude filter (see sql/filter_level_exclude.sql) ──
    if !level_exclude.is_empty() {
        let values: Vec<String> = level_exclude
            .iter()
            .map(|v| {
                bind_values.push(v.clone());
                format!("?{}", bind_values.len())
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE LOWER(level) NOT IN ({})",
            values.join(",")
        );
        debug!("应用 filter_level_exclude: [{}]", level_exclude.join(", "));
    }

    // ── Conference include filter (see sql/filter_conference_include.sql) ──
    if !conference_include.is_empty() {
        let values: Vec<String> = conference_include
            .iter()
            .map(|v| {
                bind_values.push(v.clone());
                format!("?{}", bind_values.len())
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE LOWER(conference) IN ({})",
            values.join(",")
        );
        debug!(
            "应用 filter_conference_include: [{}]",
            conference_include.join(", ")
        );
    }

    // ── Conference exclude filter (see sql/filter_conference_exclude.sql) ──
    if !conference_exclude.is_empty() {
        let values: Vec<String> = conference_exclude
            .iter()
            .map(|v| {
                bind_values.push(v.clone());
                format!("?{}", bind_values.len())
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE LOWER(conference) NOT IN ({})",
            values.join(",")
        );
        debug!(
            "应用 filter_conference_exclude: [{}]",
            conference_exclude.join(", ")
        );
    }

    // ── Year include filter (see sql/filter_year_include.sql) ──
    if !year_include.is_empty() {
        let values: Vec<String> = year_include
            .iter()
            .map(|v| {
                bind_values.push(v.clone());
                format!("?{}", bind_values.len())
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE year IN ({})",
            values.join(",")
        );
        debug!("应用 filter_year_include: [{}]", year_include.join(", "));
    }

    // ── Year exclude filter (see sql/filter_year_exclude.sql) ──
    if !year_exclude.is_empty() {
        let values: Vec<String> = year_exclude
            .iter()
            .map(|v| {
                bind_values.push(v.clone());
                format!("?{}", bind_values.len())
            })
            .collect();
        inner = format!(
            "SELECT * FROM ({inner}) WHERE year NOT IN ({})",
            values.join(",")
        );
        debug!("应用 filter_year_exclude: [{}]", year_exclude.join(", "));
    }

    // ── Final projection: columns + ordering (see sql/query.sql) ──
    let select_cols: Vec<String> = display_fields
        .iter()
        .map(|f| format!("`{}`", f.as_str()))
        .collect();

    let order_clause = if sort_specs.is_empty() {
        String::new()
    } else {
        let orders: Vec<String> = sort_specs
            .iter()
            .map(|s| format!("`{}` {}", s.field.as_str(), s.direction.as_sql()))
            .collect();
        format!("ORDER BY {}", orders.join(", "))
    };

    let sql = format!(
        "SELECT {} FROM ({inner}) {order_clause}",
        select_cols.join(", ")
    );
    debug!("最终查询: {sql}");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("查询准备失败: {e}"))?;

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = bind_values
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let num_cols = display_fields.len();
    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(params_refs.iter()),
            move |row| {
                let mut parts: Vec<String> = Vec::with_capacity(num_cols);
                for i in 0..num_cols {
                    let val: String = row.get(i).unwrap_or_default();
                    parts.push(val);
                }
                Ok(parts.join("\t"))
            },
        )
        .map_err(|e| format!("查询执行失败: {e}"))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| format!("读取查询结果失败: {e}"))?);
    }

    debug!("查询返回 {} 条结果", results.len());
    Ok(results)
}

/// Parse sort spec "field:direction"
pub fn parse_sort_spec(value: &str) -> Result<SortSpec> {
    let (field_raw, order_raw) = value
        .split_once(':')
        .ok_or_else(|| format!("无效的排序格式: {value}（应为 field:asc 或 field:desc）"))?;

    let field = Field::parse(field_raw);
    let direction = Direction::parse(order_raw)?;

    Ok(SortSpec { field, direction })
}

/// Parse columns "title,year,conference"
pub fn parse_columns(value: &str) -> Result<Vec<Field>> {
    let mut selected = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let field = Field::parse(trimmed);
        if !selected.contains(&field) {
            selected.push(field);
        }
    }

    if selected.is_empty() {
        return Err("至少需要选择一个列".to_string());
    }

    Ok(crate::models::canonical_fields()
        .into_iter()
        .filter(|field| selected.contains(field))
        .collect())
}
