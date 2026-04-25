# RemodelCore

[![Crates.io](https://img.shields.io/crates/v/remodel-core.svg)](https://crates.io/crates/remodel-core)
[![Docs.rs](https://docs.rs/remodel-core/badge.svg)](https://docs.rs/remodel-core)
[![License: GPL v3+](https://img.shields.io/badge/license-GPLv3%2B-blue.svg)](LICENSE)

`remodel-core` is the engine behind **Remodel**, a modern database-modeling
toolkit. It provides the data structures and algorithms required to author
**Entity-Relationship** (conceptual) diagrams, normalize them into **relational
schemas** (logical), and emit **SQL DDL** (physical) for the major dialects.

The crate is a Rust reimplementation of the modeling engine originally written
in Java for [brModelo](https://github.com/sis4com/brModelo) by Carlos Henrique
Cândido and contributors. Full credit to the original authors; this port is
released under the same GPL-3.0-or-later license.

## Features

- **Conceptual model** — entities, attributes (simple, primary, composite,
  multivalued, derived), relationships of arbitrary arity with cardinalities,
  specializations (total/partial × disjoint/overlapping), unions, and
  associative entities.
- **Logical model** — tables, columns with SQL types and nullability, primary,
  unique and foreign-key constraints with referential actions.
- **Conceptual → logical transform** — propagates foreign keys for 1:N,
  generates associative tables for N:M, and applies user-controlled strategies
  for specializations and complex attributes.
- **SQL DDL exporter** — PostgreSQL, MySQL and SQLite dialects.
- **Validation** — surfaces modeling errors (entity without attributes, missing
  primary keys, dangling links, …) with structured diagnostics.
- **Serialization** — every model derives `serde::{Serialize, Deserialize}`
  for use in JSON-based persistence formats (`.remodel`).

## Quick start

```rust
use remodel_core::prelude::*;

let mut model = ConceptualModel::new("library");

let book = model.add_entity("Book");
model.add_primary_attribute(book, "id", DataType::Integer);
model.add_attribute(book, "title", DataType::Varchar(255));

let author = model.add_entity("Author");
model.add_primary_attribute(author, "id", DataType::Integer);
model.add_attribute(author, "name", DataType::Varchar(120));

model.relate("wrote", book, Cardinality::ZeroToMany)
     .with(author, Cardinality::OneToMany);

let logical = model.to_logical()?;
let sql = logical.to_sql(SqlDialect::Postgres);
println!("{sql}");
# Ok::<(), remodel_core::Error>(())
```

## License

GPL-3.0-or-later, inherited from the upstream brModelo project.
