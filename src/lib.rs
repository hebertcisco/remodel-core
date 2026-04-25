//! # RemodelCore
//!
//! Engine for database modeling: build conceptual (Entity-Relationship) models,
//! normalize them into logical (relational) models, and emit SQL DDL for the
//! major dialects.
//!
//! `remodel-core` is a Rust reimplementation of the modeling engine of
//! [brModelo](https://github.com/sis4com/brModelo) by Carlos Henrique Cândido.
//! See the crate `README.md` for credits and licensing.
//!
//! The crate is organized in three layers, mirroring the three model "levels"
//! used in database design:
//!
//! - [`models::conceptual`] — Entity-Relationship structures
//! - [`models::logical`] — relational (table-and-constraint) structures
//! - [`sql`] — physical DDL generation per [`SqlDialect`](sql::SqlDialect)
//!
//! Transformations between levels live in [`transform`], and structural
//! checks in [`validation`].

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod format;
pub mod models;
pub mod sql;
pub mod transform;
pub mod validation;

pub use error::{Error, Result};

/// Convenient re-exports for downstream users.
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::models::cardinality::Cardinality;
    pub use crate::models::conceptual::{
        Attribute, AttributeKind, ConceptualModel, Entity, EntityId, Relationship,
        RelationshipId, Specialization, SpecializationKind, Union,
    };
    pub use crate::models::logical::{
        Column, ColumnId, Constraint, ConstraintKind, ForeignKey, LogicalModel, ReferentialAction,
        Table, TableId,
    };
    pub use crate::models::types::DataType;
    pub use crate::sql::SqlDialect;
    pub use crate::transform::{ConvertOptions, RelationshipResolution, SpecializationStrategy};
    pub use crate::validation::{Diagnostic, Severity};
}

/// Crate version, useful when persisting model files that need a writer tag.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
