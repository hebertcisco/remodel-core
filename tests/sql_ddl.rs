//! End-to-end tests: build a small library schema, convert it, and assert
//! the rendered SQL DDL contains the expected statements for each dialect.

use remodel_core::prelude::*;
use remodel_core::models::conceptual::AttributeOwner;

fn library() -> ConceptualModel {
    let mut m = ConceptualModel::new("library");
    let book = m.add_entity("Book");
    let author = m.add_entity("Author");
    m.add_primary_attribute(book, "id", DataType::Integer).unwrap();
    m.add_attribute(AttributeOwner::Entity(book), "title", DataType::Varchar(255)).unwrap();
    m.add_primary_attribute(author, "id", DataType::Integer).unwrap();
    m.add_attribute(AttributeOwner::Entity(author), "name", DataType::Varchar(120)).unwrap();
    m.relate("wrote", book, Cardinality::ZeroToMany)
        .with(author, Cardinality::OneToMany)
        .id();
    m
}

#[test]
fn postgres_dialect_renders_expected_statements() {
    let model = library();
    let logical = model.to_logical().unwrap();
    let sql = logical.to_sql(SqlDialect::Postgres);

    assert!(sql.contains("CREATE TABLE \"Book\""), "missing CREATE TABLE Book in:\n{sql}");
    assert!(sql.contains("CREATE TABLE \"Author\""), "missing CREATE TABLE Author");
    assert!(sql.contains("CREATE TABLE \"wrote\""), "missing junction table");

    assert!(sql.contains("INTEGER"));
    assert!(sql.contains("VARCHAR(255)"));

    assert!(sql.contains("PRIMARY KEY"));

    assert!(sql.contains("ALTER TABLE \"wrote\" ADD CONSTRAINT"));
    assert!(sql.contains("REFERENCES \"Book\""));
    assert!(sql.contains("REFERENCES \"Author\""));
}

#[test]
fn mysql_dialect_uses_backticks_and_int() {
    let model = library();
    let logical = model.to_logical().unwrap();
    let sql = logical.to_sql(SqlDialect::MySql);
    assert!(sql.contains("CREATE TABLE `Book`"));
    assert!(sql.contains("`id` INT"));
    assert!(sql.contains("ALTER TABLE `wrote`"));
}

#[test]
fn sqlite_collapses_types_to_sqlite_storage_classes() {
    let model = library();
    let logical = model.to_logical().unwrap();
    let sql = logical.to_sql(SqlDialect::Sqlite);
    assert!(sql.contains("INTEGER"));
    assert!(sql.contains("TEXT"));
    assert!(!sql.contains("VARCHAR"));
}

#[test]
fn one_to_many_emits_fk_on_many_side() {
    let mut m = ConceptualModel::new("hr");
    let dept = m.add_entity("Department");
    let emp = m.add_entity("Employee");
    m.add_primary_attribute(dept, "id", DataType::Integer).unwrap();
    m.add_primary_attribute(emp, "id", DataType::Integer).unwrap();
    m.relate("works_for", emp, Cardinality::OneToOne)
        .with(dept, Cardinality::ZeroToMany)
        .id();

    let logical = m.to_logical().unwrap();
    let sql = logical.to_sql(SqlDialect::Postgres);
    assert!(sql.contains("ALTER TABLE \"Employee\" ADD CONSTRAINT"));
    assert!(sql.contains("REFERENCES \"Department\""));
}
