//! Structural validation of conceptual and logical models.
//!
//! Validation is *non-fatal* by default: [`validate_conceptual`] returns a
//! `Vec<Diagnostic>`, and the caller chooses whether any diagnostic is severe
//! enough to abort. The transform pipeline rejects models that produce one or
//! more [`Severity::Error`] diagnostics.

use serde::{Deserialize, Serialize};

use crate::models::conceptual::{ConceptualModel, EntityId};
use crate::models::logical::{LogicalModel, TableId};

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Informational: the model is valid; this is just a hint.
    Info,
    /// Warning: the model is technically valid but may not behave as expected.
    Warning,
    /// Error: the model violates a structural invariant.
    Error,
}

/// One validation finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity classifier.
    pub severity: Severity,
    /// Stable code, e.g. `"E001"`. Useful for filtering and i18n.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
}

impl Diagnostic {
    /// Construct a new error diagnostic.
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, code, message: message.into() }
    }

    /// Construct a new warning diagnostic.
    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, code, message: message.into() }
    }
}

/// Validate a [`ConceptualModel`] and return all findings.
pub fn validate_conceptual(model: &ConceptualModel) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    for entity in model.entities.values() {
        if entity.attributes.is_empty() && !is_in_specialization_child(model, entity.id) {
            out.push(Diagnostic::warning(
                "W001",
                format!("entity `{}` has no attributes", entity.name),
            ));
        }
    }

    for entity in model.entities.values() {
        if !entity.attributes.is_empty()
            && !entity.attributes.iter().any(|a| {
                model.attributes.get(a).map(|x| x.is_primary).unwrap_or(false)
            })
            && !is_in_specialization_child(model, entity.id)
            && !entity.weak
        {
            out.push(Diagnostic::error(
                "E002",
                format!("entity `{}` has no primary-key attribute", entity.name),
            ));
        }
    }

    for rel in model.relationships.values() {
        if rel.endpoints.len() < 2 {
            out.push(Diagnostic::error(
                "E003",
                format!(
                    "relationship `{}` has only {} endpoint(s); at least 2 required",
                    rel.name,
                    rel.endpoints.len()
                ),
            ));
        }
    }

    for rel in model.relationships.values() {
        for ep in &rel.endpoints {
            if !model.entities.contains_key(&ep.entity) {
                out.push(Diagnostic::error(
                    "E004",
                    format!(
                        "relationship `{}` references unknown entity #{}",
                        rel.name, ep.entity.0
                    ),
                ));
            }
        }
    }

    out
}

/// Validate a [`LogicalModel`].
pub fn validate_logical(model: &LogicalModel) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    for table in model.tables.values() {
        if table.columns.is_empty() {
            out.push(Diagnostic::error(
                "L001",
                format!("table `{}` has no columns", table.name),
            ));
            continue;
        }
        if table.primary_key().is_none() {
            out.push(Diagnostic::warning(
                "L002",
                format!("table `{}` has no primary key", table.name),
            ));
        }
        for c in table.constraints.values() {
            if let crate::models::logical::ConstraintKind::ForeignKey(fk) = &c.kind {
                if !model.tables.contains_key(&fk.references_table) {
                    out.push(Diagnostic::error(
                        "L003",
                        format!(
                            "table `{}` has FK referencing unknown table #{}",
                            table.name, fk.references_table.0
                        ),
                    ));
                }
                if fk.columns.len() != fk.references_columns.len() {
                    out.push(Diagnostic::error(
                        "L004",
                        format!(
                            "table `{}` has FK with mismatched arity ({} local vs {} referenced)",
                            table.name,
                            fk.columns.len(),
                            fk.references_columns.len()
                        ),
                    ));
                }
            }
        }
    }

    let _ = (TableId(0),);
    out
}

fn is_in_specialization_child(model: &ConceptualModel, eid: EntityId) -> bool {
    model
        .specializations
        .values()
        .any(|s| s.children.iter().any(|c| *c == eid))
}
