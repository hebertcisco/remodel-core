//! Error types returned by RemodelCore.
//!
//! All fallible operations return [`Result<T>`], an alias for
//! `std::result::Result<T, Error>`. Errors are intentionally categorized so
//! callers can react to specific failure modes (e.g. surface a validation
//! issue to the user vs. abort the whole conversion).

use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error type for RemodelCore operations.
#[derive(Debug, Error)]
pub enum Error {
    /// A reference (entity, attribute, table, …) does not exist in the model.
    #[error("unknown {kind} `{id}`")]
    UnknownReference {
        /// Kind of element that was missing (e.g. `"entity"`).
        kind: &'static str,
        /// Stringified identifier the lookup was attempted with.
        id: String,
    },

    /// A relationship does not have enough endpoints to be meaningful.
    #[error("relationship `{name}` has {found} endpoint(s); at least 2 required")]
    InsufficientEndpoints {
        /// Relationship name as it appears in the model.
        name: String,
        /// Number of endpoints actually attached.
        found: usize,
    },

    /// A specialization is malformed (no parent, no children, or cycle).
    #[error("invalid specialization: {0}")]
    InvalidSpecialization(String),

    /// The model failed structural validation prior to a transform.
    #[error("model validation failed with {0} error(s)")]
    Validation(usize),

    /// Conversion was aborted by the caller (e.g. a user rejected a strategy).
    #[error("conversion cancelled")]
    Cancelled,

    /// Catch-all for unexpected internal invariants.
    #[error("internal error: {0}")]
    Internal(String),
}
