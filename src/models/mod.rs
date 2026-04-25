//! Core data structures shared by the modeling layers.
//!
//! - [`cardinality`] — the four classical cardinalities used in ER modeling.
//! - [`types`] — generic SQL data types used by columns and attributes.
//! - [`conceptual`] — the Entity-Relationship (conceptual) model.
//! - [`logical`] — the relational (logical) model.

pub mod cardinality;
pub mod conceptual;
pub mod logical;
pub mod types;
