# remodel-core

[![Crates.io](https://img.shields.io/crates/v/remodel-core.svg)](https://crates.io/crates/remodel-core)
[![Docs.rs](https://docs.rs/remodel-core/badge.svg)](https://docs.rs/remodel-core)
[![License: GPL v3+](https://img.shields.io/badge/license-GPLv3%2B-blue.svg)](LICENSE)

`remodel-core` is a Rust library for database modeling. It lets you build
conceptual ER models, convert them into logical relational schemas, and render
SQL DDL for supported dialects.

This crate is a Rust reimplementation of the core modeling engine from
[brModelo](https://github.com/sis4com/brModelo) by Carlos Henrique Candido and
contributors. The port keeps the same GPL-3.0-or-later licensing model.

## What it provides

- Conceptual modeling primitives: entities, attributes, relationships,
  cardinalities, specializations, unions, and associative entities.
- Logical modeling primitives: tables, columns, primary keys, unique
  constraints, and foreign keys.
- Conceptual-to-logical transformation with configurable strategies for
  relationships, specializations, and complex attributes.
- SQL DDL rendering for PostgreSQL, MySQL, and SQLite.
- Validation diagnostics for structurally invalid diagrams.
- `serde` support for persistence and interchange formats.

## Installation

```bash
cargo add remodel-core
```

## Quick start

```rust
use remodel_core::models::conceptual::AttributeOwner;
use remodel_core::prelude::*;

fn main() -> Result<()> {
    let mut model = ConceptualModel::new("library");

    let book = model.add_entity("Book");
    model.add_primary_attribute(book, "id", DataType::Integer)?;
    model.add_attribute(
        AttributeOwner::Entity(book),
        "title",
        DataType::Varchar(255),
    )?;

    let author = model.add_entity("Author");
    model.add_primary_attribute(author, "id", DataType::Integer)?;
    model.add_attribute(
        AttributeOwner::Entity(author),
        "name",
        DataType::Varchar(120),
    )?;

    model
        .relate("wrote", book, Cardinality::ZeroToMany)
        .with(author, Cardinality::OneToMany)
        .id();

    let logical = model.to_logical()?;
    let sql = logical.to_sql(SqlDialect::Postgres);

    println!("{sql}");
    Ok(())
}
```

This produces a relational model with `Book`, `Author`, and a junction table
for `wrote`.

## Modeling pipeline

`remodel-core` follows the usual database-design flow:

1. Build a conceptual model.
2. Validate and convert it into a logical model.
3. Render SQL DDL for the target database dialect.

The main APIs are:

- `ConceptualModel` for ER authoring
- `ConceptualModel::to_logical()` for default conversion
- `transform::conceptual_to_logical()` when you need custom `ConvertOptions`
- `LogicalModel::to_sql()` for final DDL generation

## Supported SQL dialects

- PostgreSQL
- MySQL / MariaDB
- SQLite

## Validation

The library validates conceptual models before conversion. Structural problems
such as missing references or invalid identifiers are reported through
diagnostics, and conversion returns `Error::Validation` when errors are present.

## Release

Publishing to crates.io is automated through GitHub Actions:

- Push a tag named `remodel-core-vX.Y.Z`, or
- Run the `Publish remodel-core` workflow manually.

The workflow runs the crate tests and then publishes `RemodelCore/` using the
repository secret `CARGO_REGISTRY_TOKEN`.

## License

GPL-3.0-or-later.
