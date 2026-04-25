//! Validation diagnostics produced by the conceptual model.

use remodel_core::prelude::*;
use remodel_core::validation::{validate_conceptual, validate_logical, Severity};

#[test]
fn warns_on_entity_without_attributes() {
    let mut m = ConceptualModel::new("x");
    m.add_entity("Lonely");
    let diag = validate_conceptual(&m);
    assert!(
        diag.iter().any(|d| d.code == "W001" && d.severity == Severity::Warning),
        "expected W001 warning, got: {diag:?}"
    );
}

#[test]
fn errors_on_entity_without_primary_key() {
    let mut m = ConceptualModel::new("x");
    let e = m.add_entity("Customer");
    m.add_attribute(
        remodel_core::models::conceptual::AttributeOwner::Entity(e),
        "name",
        DataType::Varchar(120),
    )
    .unwrap();
    let diag = validate_conceptual(&m);
    assert!(
        diag.iter().any(|d| d.code == "E002"),
        "expected E002 error, got: {diag:?}"
    );
}

#[test]
fn convert_rejects_invalid_model() {
    let mut m = ConceptualModel::new("x");
    let e = m.add_entity("Customer");
    m.add_attribute(
        remodel_core::models::conceptual::AttributeOwner::Entity(e),
        "name",
        DataType::Varchar(120),
    )
    .unwrap();
    let err = m.to_logical().unwrap_err();
    assert!(matches!(err, Error::Validation(n) if n >= 1));
}

#[test]
fn logical_validation_passes_on_well_formed_schema() {
    let mut m = ConceptualModel::new("x");
    let e = m.add_entity("Customer");
    m.add_primary_attribute(e, "id", DataType::Integer).unwrap();
    let logical = m.to_logical().unwrap();
    let diag = validate_logical(&logical);
    assert!(
        diag.iter().all(|d| d.severity != Severity::Error),
        "unexpected errors: {diag:?}"
    );
}
