use std::collections::HashMap;

/// A paper record with provenance + dynamic data fields for DB insertion.
#[derive(Clone, Debug)]
pub struct IndexedRecord {
    pub level: String,
    pub conference: String,
    pub year: String,
    pub data: HashMap<String, String>,
}

/// A display/sort field — wraps a column name (dynamic, not hardcoded).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Field(pub String);

impl Field {
    pub const LEVEL: &'static str = "level";
    pub const CONFERENCE: &'static str = "conference";
    pub const YEAR: &'static str = "year";
    pub const TITLE: &'static str = "title";

    pub fn parse(value: &str) -> Self {
        let normalized = value.trim().to_ascii_lowercase();
        let mapped = match normalized.as_str() {
            "conf" | "name" => Self::CONFERENCE,
            "paper" => Self::TITLE,
            other => other,
        };
        Field(mapped.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn canonical_fields() -> Vec<Field> {
    vec![
        Field(Field::LEVEL.to_string()),
        Field(Field::CONFERENCE.to_string()),
        Field(Field::YEAR.to_string()),
        Field(Field::TITLE.to_string()),
    ]
}

/// Sort direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Asc,
    Desc,
}

impl Direction {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "asc" => Ok(Self::Asc),
            "desc" => Ok(Self::Desc),
            other => Err(format!("unsupported order: {other}")),
        }
    }

    pub fn as_sql(&self) -> &'static str {
        match self {
            Direction::Asc => "ASC",
            Direction::Desc => "DESC",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortSpec {
    pub field: Field,
    pub direction: Direction,
}
