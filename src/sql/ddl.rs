//! DDL rendering shared across dialects.

use std::fmt::Write;

use crate::models::logical::{
    ColumnId, ConstraintKind, ForeignKey, LogicalModel, Table,
};
use crate::sql::dialect::{Dialect, SqlDialect};

/// Render `model` as a SQL DDL script in the given `dialect`.
pub(crate) fn render(model: &LogicalModel, dialect: SqlDialect) -> String {
    let d = dialect.renderer();
    let mut out = String::new();

    for table in model.tables.values() {
        write_create_table(&mut out, table, &*d);
        out.push('\n');
    }

    for table in model.tables.values() {
        for c in table.constraints.values() {
            if let ConstraintKind::ForeignKey(fk) = &c.kind {
                let Some(target) = model.tables.get(&fk.references_table) else {
                    continue;
                };
                write_alter_add_fk(&mut out, table, c.name.as_deref(), fk, target, &*d);
            }
        }
    }

    out
}

fn write_create_table(out: &mut String, table: &Table, d: &dyn Dialect) {
    let _ = writeln!(out, "CREATE TABLE {} (", d.quote_ident(&table.name));

    let cols: Vec<String> = table
        .columns
        .values()
        .map(|c| {
            let mut line = format!(
                "    {} {}",
                d.quote_ident(&c.name),
                d.render_type(&c.data_type)
            );
            if !c.nullable {
                line.push_str(" NOT NULL");
            }
            if let Some(default) = &c.default {
                line.push_str(" DEFAULT ");
                line.push_str(default);
            }
            if c.is_unique && !c.is_primary {
                line.push_str(" UNIQUE");
            }
            line
        })
        .collect();
    let pk_cols = table.primary_key_columns();
    let unique_constraints: Vec<&Vec<ColumnId>> = table
        .constraints
        .values()
        .filter_map(|c| match &c.kind {
            ConstraintKind::Unique { columns } if columns.len() > 1 => Some(columns),
            _ => None,
        })
        .collect();
    let check_constraints: Vec<&str> = table
        .constraints
        .values()
        .filter_map(|c| match &c.kind {
            ConstraintKind::Check { expression } => Some(expression.as_str()),
            _ => None,
        })
        .collect();

    let mut parts: Vec<String> = cols;

    if !pk_cols.is_empty() {
        let names: Vec<String> = pk_cols
            .iter()
            .filter_map(|id| table.column(*id).map(|c| d.quote_ident(&c.name)))
            .collect();
        parts.push(format!("    PRIMARY KEY ({})", names.join(", ")));
    }
    for cols in unique_constraints {
        let names: Vec<String> = cols
            .iter()
            .filter_map(|id| table.column(*id).map(|c| d.quote_ident(&c.name)))
            .collect();
        parts.push(format!("    UNIQUE ({})", names.join(", ")));
    }
    for expr in check_constraints {
        parts.push(format!("    CHECK ({expr})"));
    }

    out.push_str(&parts.join(",\n"));
    let _ = writeln!(out, "\n){}", d.terminator());
}

fn write_alter_add_fk(
    out: &mut String,
    table: &Table,
    name: Option<&str>,
    fk: &ForeignKey,
    target: &Table,
    d: &dyn Dialect,
) {
    let local: Vec<String> = fk
        .columns
        .iter()
        .filter_map(|id| table.column(*id).map(|c| d.quote_ident(&c.name)))
        .collect();
    let foreign: Vec<String> = fk
        .references_columns
        .iter()
        .filter_map(|id| target.column(*id).map(|c| d.quote_ident(&c.name)))
        .collect();
    let constraint_name = name
        .map(str::to_string)
        .unwrap_or_else(|| format!("fk_{}_{}", table.name, target.name));

    let _ = writeln!(
        out,
        "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}) ON UPDATE {} ON DELETE {}{}",
        d.quote_ident(&table.name),
        d.quote_ident(&constraint_name),
        local.join(", "),
        d.quote_ident(&target.name),
        foreign.join(", "),
        d.render_action(fk.on_update),
        d.render_action(fk.on_delete),
        d.terminator(),
    );
}
