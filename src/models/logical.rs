//! Logical (relational) model.
//!
//! Tables, columns, and constraints. Constraints carry their own definition
//! (PK / UNIQUE column lists, FK source-and-target references) which the SQL
//! exporter renders as either inline `CREATE TABLE` clauses or out-of-line
//! `ALTER TABLE … ADD CONSTRAINT` statements.
//!
//! ## Mapping from brModelo
//!
//! | brModelo class    | RemodelCore type     |
//! |-------------------|----------------------|
//! | `Tabela`          | [`Table`]            |
//! | `Campo`           | [`Column`]           |
//! | `Constraint`      | [`Constraint`]       |
//! | `DataBaseModel`   | [`LogicalModel`]     |

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::models::types::DataType;

/// Strongly-typed handle for a [`Table`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TableId(pub u32);

/// Strongly-typed handle for a [`Column`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ColumnId(pub u32);

/// Strongly-typed handle for a [`Constraint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConstraintId(pub u32);

/// A column on a [`Table`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    /// Unique handle within the owning model.
    pub id: ColumnId,
    /// Column name.
    pub name: String,
    /// Logical data type.
    pub data_type: DataType,
    /// Whether the column admits NULL.
    pub nullable: bool,
    /// `true` when this column is part of the primary key.
    pub is_primary: bool,
    /// `true` when this column is part of (any) foreign key.
    pub is_foreign: bool,
    /// `true` when this column is part of a single-column UNIQUE constraint.
    pub is_unique: bool,
    /// Optional default value, rendered verbatim into DDL.
    pub default: Option<String>,
    /// Optional descriptive comment.
    pub comment: String,
}

impl Column {
    /// Construct a non-key, non-null column.
    pub fn new(id: ColumnId, name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            id,
            name: name.into(),
            data_type,
            nullable: false,
            is_primary: false,
            is_foreign: false,
            is_unique: false,
            default: None,
            comment: String::new(),
        }
    }
}

/// What action a referential constraint takes when the parent row changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ReferentialAction {
    /// `NO ACTION` — the default; the operation is rejected if it would
    /// orphan a referencing row.
    #[default]
    NoAction,
    /// `RESTRICT` — like `NO ACTION` but checked immediately.
    Restrict,
    /// `CASCADE` — propagate the change to the referencing rows.
    Cascade,
    /// `SET NULL` — set the FK columns to NULL.
    SetNull,
    /// `SET DEFAULT` — set the FK columns to their default values.
    SetDefault,
}

/// A foreign key payload, attached to a [`Constraint`] of kind
/// [`ConstraintKind::ForeignKey`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKey {
    /// Local column IDs (ordered) that constitute the FK.
    pub columns: Vec<ColumnId>,
    /// The table that this FK references.
    pub references_table: TableId,
    /// Column IDs of the referenced primary/unique key, in matching order.
    pub references_columns: Vec<ColumnId>,
    /// Action on parent UPDATE.
    pub on_update: ReferentialAction,
    /// Action on parent DELETE.
    pub on_delete: ReferentialAction,
}

/// What kind of constraint this is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    /// Primary key over the listed columns.
    PrimaryKey {
        /// Columns that compose the primary key, in order.
        columns: Vec<ColumnId>,
    },
    /// Uniqueness constraint over the listed columns.
    Unique {
        /// Columns that must be jointly unique.
        columns: Vec<ColumnId>,
    },
    /// Foreign key. See [`ForeignKey`].
    ForeignKey(ForeignKey),
    /// CHECK constraint with a verbatim predicate (rendered as-is).
    Check {
        /// SQL expression for the predicate.
        expression: String,
    },
}

/// A named (or unnamed) constraint on a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    /// Unique handle within the owning model.
    pub id: ConstraintId,
    /// Optional explicit name. If `None`, the SQL exporter will emit it
    /// inline (for `PRIMARY KEY` / `UNIQUE`) or synthesise a name (for `FK`).
    pub name: Option<String>,
    /// Constraint kind and payload.
    pub kind: ConstraintKind,
}

impl Constraint {
    /// `true` if this is a primary-key constraint.
    pub fn is_primary_key(&self) -> bool {
        matches!(self.kind, ConstraintKind::PrimaryKey { .. })
    }

    /// `true` if this is a foreign-key constraint.
    pub fn is_foreign_key(&self) -> bool {
        matches!(self.kind, ConstraintKind::ForeignKey(_))
    }
}

/// A relational table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    /// Unique handle within the owning model.
    pub id: TableId,
    /// Table name.
    pub name: String,
    /// Columns, in author order.
    pub columns: IndexMap<ColumnId, Column>,
    /// Constraints attached to this table (PK, UNIQUE, FK, CHECK).
    pub constraints: IndexMap<ConstraintId, Constraint>,
    /// Optional descriptive comment.
    pub comment: String,
}

impl Table {
    /// Iterate the columns in author order.
    pub fn columns_iter(&self) -> impl Iterator<Item = &Column> {
        self.columns.values()
    }

    /// Find the table's primary-key constraint, if any.
    pub fn primary_key(&self) -> Option<&Constraint> {
        self.constraints.values().find(|c| c.is_primary_key())
    }

    /// IDs of the columns that compose the primary key. Empty if there is no
    /// PK or the PK is empty.
    pub fn primary_key_columns(&self) -> &[ColumnId] {
        match self.primary_key().map(|c| &c.kind) {
            Some(ConstraintKind::PrimaryKey { columns }) => columns,
            _ => &[],
        }
    }

    /// Borrow a column by ID.
    pub fn column(&self, id: ColumnId) -> Option<&Column> {
        self.columns.get(&id)
    }
}

/// The full logical (relational) model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogicalModel {
    /// Database / schema name.
    pub name: String,
    /// Monotonic counter used to mint unique IDs.
    next_id: u32,
    /// Tables, keyed for O(1) lookup; iteration order is insertion order.
    pub tables: IndexMap<TableId, Table>,
}

impl LogicalModel {
    /// Construct a new empty logical model.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Self::default() }
    }

    pub(crate) fn mint(&mut self) -> u32 {
        self.next_id = self.next_id.checked_add(1).expect("ID space exhausted");
        self.next_id
    }

    /// Create a new empty table.
    pub fn add_table(&mut self, name: impl Into<String>) -> TableId {
        let id = TableId(self.mint());
        self.tables.insert(
            id,
            Table {
                id,
                name: name.into(),
                columns: IndexMap::new(),
                constraints: IndexMap::new(),
                comment: String::new(),
            },
        );
        id
    }

    /// Borrow a table by handle.
    pub fn table(&self, id: TableId) -> Result<&Table> {
        self.tables
            .get(&id)
            .ok_or_else(|| Error::UnknownReference { kind: "table", id: format!("{}", id.0) })
    }

    /// Mutably borrow a table by handle.
    pub fn table_mut(&mut self, id: TableId) -> Result<&mut Table> {
        self.tables
            .get_mut(&id)
            .ok_or_else(|| Error::UnknownReference { kind: "table", id: format!("{}", id.0) })
    }

    /// Append a new column to `table`.
    pub fn add_column(
        &mut self,
        table: TableId,
        name: impl Into<String>,
        data_type: DataType,
    ) -> Result<ColumnId> {
        let id = ColumnId(self.mint());
        let column = Column::new(id, name, data_type);
        self.table_mut(table)?.columns.insert(id, column);
        Ok(id)
    }

    /// Add (or replace) a primary-key constraint on `table`.
    pub fn set_primary_key(&mut self, table: TableId, columns: Vec<ColumnId>) -> Result<ConstraintId> {
        {
            let t = self.table_mut(table)?;
            for c in &columns {
                let col = t.columns.get_mut(c).ok_or_else(|| Error::UnknownReference {
                    kind: "column",
                    id: format!("{}", c.0),
                })?;
                col.is_primary = true;
                col.nullable = false;
            }
            let existing_pk: Vec<ConstraintId> = t
                .constraints
                .iter()
                .filter_map(|(id, c)| c.is_primary_key().then_some(*id))
                .collect();
            for id in existing_pk {
                t.constraints.shift_remove(&id);
            }
        }
        let id = ConstraintId(self.mint());
        let t = self.table_mut(table)?;
        t.constraints
            .insert(id, Constraint { id, name: None, kind: ConstraintKind::PrimaryKey { columns } });
        Ok(id)
    }

    /// Add a foreign-key constraint on `table`.
    pub fn add_foreign_key(&mut self, table: TableId, fk: ForeignKey) -> Result<ConstraintId> {
        {
            let t = self.table_mut(table)?;
            for c in &fk.columns {
                let col = t.columns.get_mut(c).ok_or_else(|| Error::UnknownReference {
                    kind: "column",
                    id: format!("{}", c.0),
                })?;
                col.is_foreign = true;
            }
        }
        let id = ConstraintId(self.mint());
        let t = self.table_mut(table)?;
        t.constraints
            .insert(id, Constraint { id, name: None, kind: ConstraintKind::ForeignKey(fk) });
        Ok(id)
    }

    /// Add a UNIQUE constraint over the given columns.
    pub fn add_unique(&mut self, table: TableId, columns: Vec<ColumnId>) -> Result<ConstraintId> {
        let id = ConstraintId(self.mint());
        let t = self.table_mut(table)?;
        if columns.len() == 1 {
            if let Some(col) = t.columns.get_mut(&columns[0]) {
                col.is_unique = true;
            }
        }
        t.constraints
            .insert(id, Constraint { id, name: None, kind: ConstraintKind::Unique { columns } });
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_table_with_pk_and_fk() {
        let mut m = LogicalModel::new("shop");
        let customer = m.add_table("Customer");
        let cid = m.add_column(customer, "id", DataType::Integer).unwrap();
        m.add_column(customer, "name", DataType::Varchar(120)).unwrap();
        m.set_primary_key(customer, vec![cid]).unwrap();

        let order = m.add_table("Order");
        let oid = m.add_column(order, "id", DataType::Integer).unwrap();
        let ocust = m.add_column(order, "customer_id", DataType::Integer).unwrap();
        m.set_primary_key(order, vec![oid]).unwrap();
        m.add_foreign_key(
            order,
            ForeignKey {
                columns: vec![ocust],
                references_table: customer,
                references_columns: vec![cid],
                on_update: ReferentialAction::NoAction,
                on_delete: ReferentialAction::NoAction,
            },
        )
        .unwrap();

        let order_t = m.table(order).unwrap();
        assert_eq!(order_t.primary_key_columns(), &[oid]);
        assert!(order_t.column(ocust).unwrap().is_foreign);
        assert!(!order_t.column(ocust).unwrap().nullable);
    }
}
