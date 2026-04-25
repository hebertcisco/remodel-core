//! SQL DDL generation for the supported dialects.
//!
//! Each dialect implements the [`Dialect`] trait, which knows how to render
//! [`DataType`](crate::models::types::DataType) values, identifier quoting,
//! and dialect-specific clauses. The top-level entry point is
//! [`LogicalModel::to_sql`].

mod dialect;
mod ddl;

pub use dialect::{Dialect, MySql, Postgres, Sqlite, SqlDialect};

use crate::models::logical::LogicalModel;

impl LogicalModel {
    /// Render this logical model as a SQL DDL script for the given dialect.
    ///
    /// Tables are emitted in author order, with primary keys inline and
    /// foreign keys deferred to `ALTER TABLE` statements at the bottom of the
    /// script (so that forward references between tables resolve cleanly).
    pub fn to_sql(&self, dialect: SqlDialect) -> String {
        ddl::render(self, dialect)
    }
}
