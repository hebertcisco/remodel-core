//! Transformations between modeling levels.
//!
//! The flagship operation is [`conceptual_to_logical`], which mirrors the
//! `conversorConceitualParaLogico` algorithm from brModelo:
//!
//! 1. Each entity becomes a table.
//! 2. Attributes become columns; complex attributes (composite, multivalued)
//!    are expanded according to [`ConvertOptions`].
//! 3. Specializations are folded according to a [`SpecializationStrategy`].
//! 4. Each binary relationship is resolved into either an FK on one side, an
//!    associative table, or a table merge based on its cardinalities.
//! 5. Ternary and higher-arity relationships always become associative
//!    tables.
//! 6. Self-relationships are resolved into either a self-referencing FK or an
//!    associative table.

mod conceptual_to_logical;
mod options;

pub use conceptual_to_logical::conceptual_to_logical;
pub use options::{ComplexAttributeStrategy, ConvertOptions, RelationshipResolution, SpecializationStrategy};

use crate::error::Result;
use crate::models::conceptual::ConceptualModel;
use crate::models::logical::LogicalModel;

impl ConceptualModel {
    /// Convert this conceptual model into a logical model using the default
    /// options. See [`conceptual_to_logical`] for the underlying algorithm
    /// and [`ConvertOptions`] for how to tune it.
    pub fn to_logical(&self) -> Result<LogicalModel> {
        conceptual_to_logical(self, &ConvertOptions::default())
    }
}
