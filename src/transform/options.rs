//! Knobs that control how [`conceptual_to_logical`](super::conceptual_to_logical)
//! resolves ambiguous modeling decisions.
//!
//! brModelo prompts the user with a dialog at each ambiguous step (composite
//! attribute, specialization, N:M relationship, …). RemodelCore replaces each
//! dialog with an explicit option in [`ConvertOptions`]. The defaults reflect
//! the choices that brModelo recommends in its built-in help.

use serde::{Deserialize, Serialize};

/// How to resolve a relationship into tables/foreign keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RelationshipResolution {
    /// Decide automatically from cardinalities. This is the default.
    ///
    /// - `(1,1) ↔ (1,1)` → merge tables
    /// - `(0..=1, 1) ↔ many` → FK on the *many* side, NULL if the many side
    ///    has minimum 0
    /// - `(many) ↔ (many)` → associative table
    #[default]
    Auto,
    /// Always emit an associative (junction) table, even for 1:1 / 1:N.
    AlwaysAssociative,
    /// Always merge the two tables. Only valid for 1:1; ignored otherwise.
    AlwaysMerge,
}

/// How to fold a specialization hierarchy into the relational model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SpecializationStrategy {
    /// One table per class: parent stays, each child gets its own table with
    /// an FK to the parent. Always safe.
    #[default]
    OneTablePerClass,
    /// Single table: parent absorbs all child columns, plus a discriminator
    /// column. Only valid for total + disjoint specializations.
    SingleTable,
    /// One table per child: parent disappears, each child gets the parent's
    /// columns inlined. Suitable for total specializations.
    OneTablePerChild,
}

/// How to fold a complex attribute (composite or multivalued).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ComplexAttributeStrategy {
    /// Create a separate table joined by FK to the owning entity. Always
    /// safe and is the brModelo default for multivalued attributes.
    #[default]
    SeparateTable,
    /// Flatten into the owner table by prefixing column names with the
    /// composite/multivalued attribute's name. Only valid when the maximum
    /// cardinality is small and known.
    Flatten,
}

/// All knobs for the conceptual → logical conversion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConvertOptions {
    /// Relationship resolution policy.
    pub relationship: RelationshipResolution,
    /// Specialization strategy.
    pub specialization: SpecializationStrategy,
    /// Complex attribute strategy (composite + multivalued).
    pub complex_attribute: ComplexAttributeStrategy,
    /// If `true`, generate `_id` suffix instead of `_pk`/`_fk` in synthesized
    /// column names. Defaults to `false` to mirror brModelo's naming.
    pub modern_naming: bool,
    /// If `true`, drop non-alphanumeric characters from generated identifiers
    /// (matches brModelo's `removerCaracteresEspeciais`).
    pub sanitize_identifiers: bool,
}

impl ConvertOptions {
    /// Suffix used for primary-key columns synthesized by the converter.
    pub fn pk_suffix(&self) -> &'static str {
        if self.modern_naming { "_id" } else { "_pk" }
    }

    /// Suffix used for foreign-key columns synthesized by the converter.
    pub fn fk_suffix(&self) -> &'static str {
        if self.modern_naming { "_id" } else { "_fk" }
    }
}
