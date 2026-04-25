//! SQL dialects supported by the DDL exporter.

use serde::{Deserialize, Serialize};

use crate::models::logical::ReferentialAction;
use crate::models::types::DataType;

/// One of the dialects RemodelCore knows how to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SqlDialect {
    /// PostgreSQL 12+.
    Postgres,
    /// MySQL 8 / MariaDB 10+.
    MySql,
    /// SQLite 3.
    Sqlite,
}

impl SqlDialect {
    /// Get a [`Dialect`] trait object for this enum value.
    pub fn renderer(self) -> Box<dyn Dialect> {
        match self {
            SqlDialect::Postgres => Box::new(Postgres),
            SqlDialect::MySql => Box::new(MySql),
            SqlDialect::Sqlite => Box::new(Sqlite),
        }
    }
}

/// Dialect-specific rendering hooks.
pub trait Dialect {
    /// Render a [`DataType`] as a SQL type expression.
    fn render_type(&self, ty: &DataType) -> String;

    /// Quote an identifier (column / table name).
    fn quote_ident(&self, ident: &str) -> String;

    /// Render a referential action verbatim.
    fn render_action(&self, action: ReferentialAction) -> &'static str {
        match action {
            ReferentialAction::NoAction => "NO ACTION",
            ReferentialAction::Restrict => "RESTRICT",
            ReferentialAction::Cascade => "CASCADE",
            ReferentialAction::SetNull => "SET NULL",
            ReferentialAction::SetDefault => "SET DEFAULT",
        }
    }

    /// Statement terminator (almost always `;`).
    fn terminator(&self) -> &'static str {
        ";"
    }
}

/// PostgreSQL renderer.
pub struct Postgres;
impl Dialect for Postgres {
    fn render_type(&self, ty: &DataType) -> String {
        match ty {
            DataType::Integer => "INTEGER".into(),
            DataType::BigInt => "BIGINT".into(),
            DataType::SmallInt => "SMALLINT".into(),
            DataType::Real => "DOUBLE PRECISION".into(),
            DataType::Decimal(p, s) => format!("NUMERIC({p},{s})"),
            DataType::Boolean => "BOOLEAN".into(),
            DataType::Varchar(n) => format!("VARCHAR({n})"),
            DataType::Char(n) => format!("CHAR({n})"),
            DataType::Text => "TEXT".into(),
            DataType::Date => "DATE".into(),
            DataType::Time => "TIME".into(),
            DataType::Timestamp => "TIMESTAMP".into(),
            DataType::Uuid => "UUID".into(),
            DataType::Bytes => "BYTEA".into(),
            DataType::Custom(s) => s.clone(),
        }
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("\"{}\"", ident.replace('"', "\"\""))
    }
}

/// MySQL / MariaDB renderer.
pub struct MySql;
impl Dialect for MySql {
    fn render_type(&self, ty: &DataType) -> String {
        match ty {
            DataType::Integer => "INT".into(),
            DataType::BigInt => "BIGINT".into(),
            DataType::SmallInt => "SMALLINT".into(),
            DataType::Real => "DOUBLE".into(),
            DataType::Decimal(p, s) => format!("DECIMAL({p},{s})"),
            DataType::Boolean => "TINYINT(1)".into(),
            DataType::Varchar(n) => format!("VARCHAR({n})"),
            DataType::Char(n) => format!("CHAR({n})"),
            DataType::Text => "TEXT".into(),
            DataType::Date => "DATE".into(),
            DataType::Time => "TIME".into(),
            DataType::Timestamp => "DATETIME".into(),
            DataType::Uuid => "CHAR(36)".into(),
            DataType::Bytes => "BLOB".into(),
            DataType::Custom(s) => s.clone(),
        }
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("`{}`", ident.replace('`', "``"))
    }
}

/// SQLite 3 renderer.
pub struct Sqlite;
impl Dialect for Sqlite {
    fn render_type(&self, ty: &DataType) -> String {
        match ty {
            DataType::Integer | DataType::SmallInt | DataType::BigInt => "INTEGER".into(),
            DataType::Real | DataType::Decimal(_, _) => "REAL".into(),
            DataType::Boolean => "INTEGER".into(),
            DataType::Varchar(_) | DataType::Char(_) | DataType::Text | DataType::Uuid => "TEXT".into(),
            DataType::Date | DataType::Time | DataType::Timestamp => "TEXT".into(),
            DataType::Bytes => "BLOB".into(),
            DataType::Custom(s) => s.clone(),
        }
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("\"{}\"", ident.replace('"', "\"\""))
    }
}
