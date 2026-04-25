//! On-disk serialization helpers.
//!
//! The `.remodel` file format is a JSON document containing a writer tag, a
//! conceptual model, and (optionally) a cached logical model. This module
//! exposes thin helpers around `serde_json` so callers don't have to assemble
//! the envelope by hand.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::conceptual::ConceptualModel;
use crate::models::logical::LogicalModel;

/// On-disk envelope for a `.remodel` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemodelFile {
    /// Writer tag, typically `"remodel-core/<version>"`. Useful for migrations.
    pub writer: String,
    /// The conceptual (ER) model — the source of truth.
    pub conceptual: ConceptualModel,
    /// Cached logical model, if any. May be regenerated from `conceptual`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub logical: Option<LogicalModel>,
}

impl RemodelFile {
    /// Wrap the given conceptual model in a fresh envelope tagged with the
    /// current crate version.
    pub fn new(conceptual: ConceptualModel) -> Self {
        Self {
            writer: format!("remodel-core/{}", crate::VERSION),
            conceptual,
            logical: None,
        }
    }

    /// Serialize to a pretty JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::Internal(format!("serialize: {e}")))
    }

    /// Parse from a JSON string.
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(|e| crate::Error::Internal(format!("parse: {e}")))
    }
}
