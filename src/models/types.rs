//! Generic data types used by attributes (conceptual) and columns (logical).
//!
//! These types are dialect-agnostic; the [`crate::sql`] module renders them
//! into the appropriate per-dialect SQL keyword.

use serde::{Deserialize, Serialize};

/// Logical data type for an attribute or column.
///
/// `Custom` allows users to bypass the abstraction and write a verbatim type
/// string for a specific dialect (e.g. `"jsonb"`, `"GEOGRAPHY(POINT)"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    /// 32-bit signed integer.
    Integer,
    /// 64-bit signed integer.
    BigInt,
    /// 16-bit signed integer.
    SmallInt,
    /// IEEE-754 double-precision floating point.
    Real,
    /// Fixed-precision decimal — `Decimal(precision, scale)`.
    Decimal(u8, u8),
    /// Boolean.
    Boolean,
    /// Variable-length text up to `n` characters.
    Varchar(u32),
    /// Fixed-length text of exactly `n` characters.
    Char(u32),
    /// Unbounded text.
    Text,
    /// Calendar date with no time component.
    Date,
    /// Time of day with no date component.
    Time,
    /// Date and time.
    Timestamp,
    /// Universally unique identifier.
    Uuid,
    /// Opaque binary blob.
    Bytes,
    /// Verbatim, dialect-specific type string.
    Custom(String),
}

impl DataType {
    /// Default type used when the user does not pick one (matches brModelo's
    /// default of `INT` for new key columns).
    pub fn default_key() -> Self {
        Self::Integer
    }
}

impl Default for DataType {
    fn default() -> Self {
        Self::Varchar(255)
    }
}
