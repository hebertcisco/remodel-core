//! Implementation of the conceptual → logical transformation.
//!
//! High-level flow (mirrors brModelo's `conversorConceitualParaLogico`):
//!
//! ```text
//! 1. validate the conceptual model
//! 2. for each entity → create a table with its primary attributes
//! 3. for each non-primary attribute → fold into the table
//!     (composite/multivalued via ConvertOptions)
//! 4. for each specialization → fold per SpecializationStrategy
//! 5. for each binary relationship → resolve cardinalities to FK / merge / N:M
//! 6. for each n-ary or self-relationship → emit an associative table
//! 7. for each union → emit FK from each parent into the category
//! 8. for each associative entity → ensure its underlying relationship was
//!    materialized as a table and rename it
//! ```

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::models::conceptual::{
    Attribute, AttributeId, AttributeKind, ConceptualModel, EntityId, RelationshipEndpoint,
    RelationshipId, Specialization, SpecializationKind,
};
use crate::models::logical::{
    ColumnId, ForeignKey, LogicalModel, ReferentialAction, TableId,
};
use crate::models::types::DataType;
use crate::transform::options::{
    ComplexAttributeStrategy, ConvertOptions, RelationshipResolution, SpecializationStrategy,
};
use crate::validation::{validate_conceptual, Severity};

/// Convert a [`ConceptualModel`] into a [`LogicalModel`] using the given
/// [`ConvertOptions`].
///
/// The function is pure: it neither mutates the input nor performs I/O. It
/// returns [`Error::Validation`] if the conceptual model fails structural
/// validation with at least one [`Severity::Error`] diagnostic.
pub fn conceptual_to_logical(
    conceptual: &ConceptualModel,
    options: &ConvertOptions,
) -> Result<LogicalModel> {
    let diagnostics = validate_conceptual(conceptual);
    let errors = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    if errors > 0 {
        return Err(Error::Validation(errors));
    }

    let mut ctx = Ctx::new(conceptual, options);
    ctx.create_tables_for_entities()?;
    ctx.add_simple_attributes()?;
    ctx.handle_complex_attributes()?;
    ctx.handle_specializations()?;
    ctx.handle_unions()?;
    ctx.handle_relationships()?;
    ctx.handle_associative_entities()?;
    Ok(ctx.into_logical())
}

/// Internal mutable state for one conversion run.
struct Ctx<'a> {
    src: &'a ConceptualModel,
    opts: &'a ConvertOptions,
    out: LogicalModel,
    /// Maps an entity to the table it became.
    entity_to_table: HashMap<EntityId, TableId>,
    /// Maps an entity's primary attribute to the column it became (in its
    /// own table). Needed to mint matching FK columns elsewhere.
    primary_columns: HashMap<EntityId, Vec<(AttributeId, ColumnId, DataType)>>,
    /// Maps a relationship to the associative table it became, if any.
    relationship_table: HashMap<RelationshipId, TableId>,
}

impl<'a> Ctx<'a> {
    fn new(src: &'a ConceptualModel, opts: &'a ConvertOptions) -> Self {
        let out = LogicalModel::new(src.name.clone());
        Self {
            src,
            opts,
            out,
            entity_to_table: HashMap::new(),
            primary_columns: HashMap::new(),
            relationship_table: HashMap::new(),
        }
    }

    fn into_logical(self) -> LogicalModel {
        self.out
    }

    fn create_tables_for_entities(&mut self) -> Result<()> {
        for entity in self.src.entities.values() {
            let name = self.identifier(&entity.name);
            let tid = self.out.add_table(name);
            self.entity_to_table.insert(entity.id, tid);

            let mut pk_cols: Vec<(AttributeId, ColumnId, DataType)> = Vec::new();
            for aid in &entity.attributes {
                let attr = self.src.attribute(*aid)?;
                if attr.is_primary && matches!(attr.kind, AttributeKind::Simple) {
                    let col = self.out.add_column(
                        tid,
                        self.identifier(&attr.name),
                        attr.data_type.clone(),
                    )?;
                    pk_cols.push((*aid, col, attr.data_type.clone()));
                }
            }
            if !pk_cols.is_empty() {
                let ids: Vec<ColumnId> = pk_cols.iter().map(|(_, c, _)| *c).collect();
                self.out.set_primary_key(tid, ids)?;
            }
            self.primary_columns.insert(entity.id, pk_cols);
        }
        Ok(())
    }

    fn add_simple_attributes(&mut self) -> Result<()> {
        for entity in self.src.entities.values() {
            let tid = self.entity_to_table[&entity.id];
            for aid in &entity.attributes {
                let attr = self.src.attribute(*aid)?;
                if attr.is_primary && matches!(attr.kind, AttributeKind::Simple) {
                    continue;
                }
                if !matches!(attr.kind, AttributeKind::Simple | AttributeKind::Derived) {
                    continue;
                }
                if matches!(attr.kind, AttributeKind::Derived) {
                    continue;
                }
                let col = self.out.add_column(
                    tid,
                    self.identifier(&attr.name),
                    attr.data_type.clone(),
                )?;
                let column = self
                    .out
                    .table_mut(tid)?
                    .columns
                    .get_mut(&col)
                    .ok_or_else(|| Error::Internal("just-added column missing".into()))?;
                column.nullable = attr.is_optional;
            }
        }
        Ok(())
    }

    fn handle_complex_attributes(&mut self) -> Result<()> {
        let mut work: Vec<(EntityId, AttributeId)> = Vec::new();
        for entity in self.src.entities.values() {
            for aid in &entity.attributes {
                let attr = self.src.attribute(*aid)?;
                if matches!(
                    attr.kind,
                    AttributeKind::Composite | AttributeKind::Multivalued { .. }
                ) {
                    work.push((entity.id, *aid));
                }
            }
        }

        for (eid, aid) in work {
            let attr = self.src.attribute(aid)?.clone();
            let owner_tid = self.entity_to_table[&eid];
            match (self.opts.complex_attribute, &attr.kind) {
                (ComplexAttributeStrategy::Flatten, AttributeKind::Composite) => {
                    self.flatten_composite(owner_tid, &attr)?
                }
                (
                    ComplexAttributeStrategy::Flatten,
                    AttributeKind::Multivalued { max: Some(max), .. },
                ) if *max > 0 && *max <= 8 => self.flatten_multivalued(owner_tid, &attr, *max)?,
                _ => self.spin_off_attribute_table(eid, owner_tid, &attr)?,
            }
        }
        Ok(())
    }

    fn flatten_composite(&mut self, owner: TableId, attr: &Attribute) -> Result<()> {
        for cid in &attr.children {
            let child = self.src.attribute(*cid)?;
            let name = format!("{}_{}", attr.name, child.name);
            let col = self.out.add_column(owner, self.identifier(&name), child.data_type.clone())?;
            self.out.table_mut(owner)?.columns.get_mut(&col).unwrap().nullable =
                attr.is_optional || child.is_optional;
        }
        Ok(())
    }

    fn flatten_multivalued(&mut self, owner: TableId, attr: &Attribute, max: u32) -> Result<()> {
        for i in 1..=max {
            let name = format!("{}_{}", attr.name, i);
            let col = self.out.add_column(owner, self.identifier(&name), attr.data_type.clone())?;
            let nullable = match attr.kind {
                AttributeKind::Multivalued { min, .. } => i > min,
                _ => true,
            };
            self.out.table_mut(owner)?.columns.get_mut(&col).unwrap().nullable = nullable;
        }
        Ok(())
    }

    fn spin_off_attribute_table(
        &mut self,
        owner_entity: EntityId,
        owner: TableId,
        attr: &Attribute,
    ) -> Result<()> {
        let table_name = self.identifier(&format!("{}_{}", self.entity_name(owner_entity)?, attr.name));
        let new_tid = self.out.add_table(table_name);

        let pk_cols = self.primary_columns.get(&owner_entity).cloned().unwrap_or_default();
        let mut local_fk: Vec<ColumnId> = Vec::new();
        let mut foreign_pk: Vec<ColumnId> = Vec::new();
        for (_aid, owner_col_id, ty) in &pk_cols {
            let owner_col_name =
                self.out.table(owner)?.column(*owner_col_id).unwrap().name.clone();
            let fk_name = format!("{}{}", owner_col_name, self.opts.fk_suffix());
            let new_col = self.out.add_column(new_tid, self.identifier(&fk_name), ty.clone())?;
            local_fk.push(new_col);
            foreign_pk.push(*owner_col_id);
        }

        let mut value_cols: Vec<ColumnId> = Vec::new();
        if matches!(attr.kind, AttributeKind::Composite) && !attr.children.is_empty() {
            for cid in &attr.children {
                let child = self.src.attribute(*cid)?;
                let id = self.out.add_column(
                    new_tid,
                    self.identifier(&child.name),
                    child.data_type.clone(),
                )?;
                value_cols.push(id);
            }
        } else {
            let id = self.out.add_column(
                new_tid,
                self.identifier(&attr.name),
                attr.data_type.clone(),
            )?;
            value_cols.push(id);
        }

        let mut pk = local_fk.clone();
        pk.extend(value_cols.iter().copied());
        if !pk.is_empty() {
            self.out.set_primary_key(new_tid, pk)?;
        }

        if !local_fk.is_empty() {
            self.out.add_foreign_key(
                new_tid,
                ForeignKey {
                    columns: local_fk,
                    references_table: owner,
                    references_columns: foreign_pk,
                    on_update: ReferentialAction::Cascade,
                    on_delete: ReferentialAction::Cascade,
                },
            )?;
        }
        Ok(())
    }

    fn handle_specializations(&mut self) -> Result<()> {
        let specs: Vec<Specialization> = self.src.specializations.values().cloned().collect();
        for spec in specs {
            match self.choose_strategy(&spec) {
                SpecializationStrategy::OneTablePerClass => self.spec_one_per_class(&spec)?,
                SpecializationStrategy::SingleTable => self.spec_single_table(&spec)?,
                SpecializationStrategy::OneTablePerChild => self.spec_one_per_child(&spec)?,
            }
        }
        Ok(())
    }

    /// Choose a strategy for `spec`, falling back to `OneTablePerClass` when
    /// the user-selected strategy would be unsafe for the specialization
    /// kind (e.g. SingleTable on a partial+overlapping spec).
    fn choose_strategy(&self, spec: &Specialization) -> SpecializationStrategy {
        match self.opts.specialization {
            SpecializationStrategy::SingleTable
                if spec.kind == SpecializationKind::TOTAL_DISJOINT =>
            {
                SpecializationStrategy::SingleTable
            }
            SpecializationStrategy::OneTablePerChild if spec.kind.total => {
                SpecializationStrategy::OneTablePerChild
            }
            _ => SpecializationStrategy::OneTablePerClass,
        }
    }

    fn spec_one_per_class(&mut self, spec: &Specialization) -> Result<()> {
        let parent_tid = self.entity_to_table[&spec.parent];
        let parent_pk = self.primary_columns.get(&spec.parent).cloned().unwrap_or_default();
        for child in &spec.children {
            let child_tid = self.entity_to_table[child];
            let mut local_fk: Vec<ColumnId> = Vec::new();
            let mut foreign_pk: Vec<ColumnId> = Vec::new();
            for (_aid, parent_col_id, ty) in &parent_pk {
                let parent_col_name =
                    self.out.table(parent_tid)?.column(*parent_col_id).unwrap().name.clone();
                let new_col = self.out.add_column(
                    child_tid,
                    self.identifier(&parent_col_name),
                    ty.clone(),
                )?;
                local_fk.push(new_col);
                foreign_pk.push(*parent_col_id);
            }
            if !local_fk.is_empty() {
                self.out.set_primary_key(child_tid, local_fk.clone())?;
                self.out.add_foreign_key(
                    child_tid,
                    ForeignKey {
                        columns: local_fk,
                        references_table: parent_tid,
                        references_columns: foreign_pk,
                        on_update: ReferentialAction::Cascade,
                        on_delete: ReferentialAction::Cascade,
                    },
                )?;
            }
        }
        Ok(())
    }

    fn spec_single_table(&mut self, spec: &Specialization) -> Result<()> {
        let parent_tid = self.entity_to_table[&spec.parent];
        let child_tids: Vec<TableId> =
            spec.children.iter().map(|c| self.entity_to_table[c]).collect();
        for child_tid in &child_tids {
            let child_table = self.out.table(*child_tid)?.clone();
            for (_, col) in &child_table.columns {
                if col.is_primary {
                    continue;
                }
                let id = self.out.add_column(parent_tid, col.name.clone(), col.data_type.clone())?;
                let dst = self.out.table_mut(parent_tid)?.columns.get_mut(&id).unwrap();
                dst.nullable = true;
            }
            self.out.tables.shift_remove(child_tid);
            self.entity_to_table.retain(|_, t| t != child_tid);
        }
        let disc = self.out.add_column(parent_tid, "type", DataType::Varchar(64))?;
        self.out.table_mut(parent_tid)?.columns.get_mut(&disc).unwrap().nullable = false;
        for child in &spec.children {
            self.entity_to_table.insert(*child, parent_tid);
        }
        Ok(())
    }

    fn spec_one_per_child(&mut self, spec: &Specialization) -> Result<()> {
        let parent_tid = self.entity_to_table[&spec.parent];
        let parent_table = self.out.table(parent_tid)?.clone();
        for child in &spec.children {
            let child_tid = self.entity_to_table[child];
            let mut new_pk: Vec<ColumnId> = Vec::new();
            for (_, col) in &parent_table.columns {
                let id = self.out.add_column(child_tid, col.name.clone(), col.data_type.clone())?;
                let dst = self.out.table_mut(child_tid)?.columns.get_mut(&id).unwrap();
                dst.nullable = col.nullable;
                if col.is_primary {
                    new_pk.push(id);
                }
            }
            if !new_pk.is_empty() {
                self.out.set_primary_key(child_tid, new_pk)?;
            }
        }
        self.out.tables.shift_remove(&parent_tid);
        if let Some(first_child) = spec.children.first() {
            let first_tid = self.entity_to_table[first_child];
            self.entity_to_table.insert(spec.parent, first_tid);
        }
        Ok(())
    }

    fn handle_unions(&mut self) -> Result<()> {
        let unions: Vec<_> = self.src.unions.values().cloned().collect();
        for u in unions {
            let cat_tid = self.entity_to_table[&u.category];
            let _ = self
                .out
                .add_column(cat_tid, "category_source", DataType::Varchar(64))?;
            for parent in &u.parents {
                let parent_tid = self.entity_to_table[parent];
                let parent_pk =
                    self.primary_columns.get(parent).cloned().unwrap_or_default();
                let mut local: Vec<ColumnId> = Vec::new();
                let mut foreign: Vec<ColumnId> = Vec::new();
                for (_aid, pcol, ty) in &parent_pk {
                    let parent_name = self.entity_name(*parent)?.to_string();
                    let parent_col_name =
                        self.out.table(parent_tid)?.column(*pcol).unwrap().name.clone();
                    let name = format!("{}_{}{}", parent_name, parent_col_name, self.opts.fk_suffix());
                    let id = self.out.add_column(cat_tid, self.identifier(&name), ty.clone())?;
                    self.out.table_mut(cat_tid)?.columns.get_mut(&id).unwrap().nullable = true;
                    local.push(id);
                    foreign.push(*pcol);
                }
                if !local.is_empty() {
                    self.out.add_foreign_key(
                        cat_tid,
                        ForeignKey {
                            columns: local,
                            references_table: parent_tid,
                            references_columns: foreign,
                            on_update: ReferentialAction::Cascade,
                            on_delete: ReferentialAction::SetNull,
                        },
                    )?;
                }
            }
        }
        Ok(())
    }

    fn handle_relationships(&mut self) -> Result<()> {
        let rels: Vec<RelationshipId> = self.src.relationships.keys().copied().collect();
        for rid in rels {
            let rel = self.src.relationship(rid)?.clone();
            if rel.is_self() {
                self.handle_self_relationship(rid, &rel)?;
            } else if rel.is_binary() {
                self.handle_binary_relationship(rid, &rel)?;
            } else {
                self.handle_nary_relationship(rid, &rel)?;
            }
        }
        Ok(())
    }

    fn handle_binary_relationship(
        &mut self,
        rid: RelationshipId,
        rel: &crate::models::conceptual::Relationship,
    ) -> Result<()> {
        let a = &rel.endpoints[0];
        let b = &rel.endpoints[1];

        match self.opts.relationship {
            RelationshipResolution::AlwaysAssociative => {
                self.emit_associative_table(rid, rel)?
            }
            RelationshipResolution::AlwaysMerge if !a.cardinality.is_many() && !b.cardinality.is_many() => {
                self.merge_tables(a.entity, b.entity, &rel.name)?;
            }
            _ => {
                match (a.cardinality.is_many(), b.cardinality.is_many()) {
                    (true, true) => self.emit_associative_table(rid, rel)?,
                    (true, false) => self.emit_fk(b, a, rel)?,
                    (false, true) => self.emit_fk(a, b, rel)?,
                    (false, false) => {
                        if a.cardinality.is_mandatory() && b.cardinality.is_mandatory() {
                            self.merge_tables(a.entity, b.entity, &rel.name)?;
                        } else {
                            let (receiver, target) = if !a.cardinality.is_mandatory() {
                                (a, b)
                            } else {
                                (b, a)
                            };
                            self.emit_fk(receiver, target, rel)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Emit an FK column on `receiver.entity`'s table, referencing
    /// `target.entity`'s PK.
    ///
    /// `receiver`'s cardinality (which describes how many `target` tuples
    /// participate per `receiver` tuple) governs the FK's nullability.
    fn emit_fk(
        &mut self,
        receiver: &RelationshipEndpoint,
        target: &RelationshipEndpoint,
        rel: &crate::models::conceptual::Relationship,
    ) -> Result<()> {
        let many = receiver;
        let one = target;
        let many_tid = self.entity_to_table[&many.entity];
        let one_tid = self.entity_to_table[&one.entity];
        let one_pk = self.primary_columns.get(&one.entity).cloned().unwrap_or_default();

        let one_name = self.entity_name(one.entity)?.to_string();
        let mut local: Vec<ColumnId> = Vec::new();
        let mut foreign: Vec<ColumnId> = Vec::new();
        for (_aid, pcol, ty) in &one_pk {
            let pcol_name = self.out.table(one_tid)?.column(*pcol).unwrap().name.clone();
            let name = format!("{}_{}{}", one_name, pcol_name, self.opts.fk_suffix());
            let id = self.out.add_column(many_tid, self.identifier(&name), ty.clone())?;
            self.out.table_mut(many_tid)?.columns.get_mut(&id).unwrap().nullable =
                !many.cardinality.is_mandatory();
            local.push(id);
            foreign.push(*pcol);
        }
        if !local.is_empty() {
            self.out.add_foreign_key(
                many_tid,
                ForeignKey {
                    columns: local,
                    references_table: one_tid,
                    references_columns: foreign,
                    on_update: ReferentialAction::Cascade,
                    on_delete: if many.cardinality.is_mandatory() {
                        ReferentialAction::Cascade
                    } else {
                        ReferentialAction::SetNull
                    },
                },
            )?;
        }
        for aid in &rel.attributes {
            let attr = self.src.attribute(*aid)?;
            let id = self.out.add_column(
                many_tid,
                self.identifier(&attr.name),
                attr.data_type.clone(),
            )?;
            self.out.table_mut(many_tid)?.columns.get_mut(&id).unwrap().nullable = attr.is_optional;
        }
        Ok(())
    }

    fn merge_tables(&mut self, a: EntityId, b: EntityId, _rel_name: &str) -> Result<()> {
        let a_tid = self.entity_to_table[&a];
        let b_tid = self.entity_to_table[&b];
        if a_tid == b_tid {
            return Ok(());
        }
        let b_table = self.out.table(b_tid)?.clone();
        for (_, col) in &b_table.columns {
            let id = self.out.add_column(a_tid, col.name.clone(), col.data_type.clone())?;
            let dst = self.out.table_mut(a_tid)?.columns.get_mut(&id).unwrap();
            dst.nullable = col.nullable;
        }
        self.out.tables.shift_remove(&b_tid);
        self.entity_to_table.insert(b, a_tid);
        Ok(())
    }

    fn handle_self_relationship(
        &mut self,
        rid: RelationshipId,
        rel: &crate::models::conceptual::Relationship,
    ) -> Result<()> {
        let a = &rel.endpoints[0];
        let b = &rel.endpoints[1];
        if a.cardinality.is_many() && b.cardinality.is_many() {
            return self.emit_associative_table(rid, rel);
        }
        let tid = self.entity_to_table[&a.entity];
        let entity_name = self.entity_name(a.entity)?.to_string();
        let pk = self.primary_columns.get(&a.entity).cloned().unwrap_or_default();
        let mut local: Vec<ColumnId> = Vec::new();
        let mut foreign: Vec<ColumnId> = Vec::new();
        for (_aid, pcol, ty) in &pk {
            let pcol_name = self.out.table(tid)?.column(*pcol).unwrap().name.clone();
            let role = b.role.as_deref().unwrap_or(&rel.name);
            let name = format!("{}_{}_{}{}", entity_name, role, pcol_name, self.opts.fk_suffix());
            let id = self.out.add_column(tid, self.identifier(&name), ty.clone())?;
            self.out.table_mut(tid)?.columns.get_mut(&id).unwrap().nullable = true;
            local.push(id);
            foreign.push(*pcol);
        }
        if !local.is_empty() {
            self.out.add_foreign_key(
                tid,
                ForeignKey {
                    columns: local,
                    references_table: tid,
                    references_columns: foreign,
                    on_update: ReferentialAction::Cascade,
                    on_delete: ReferentialAction::SetNull,
                },
            )?;
        }
        Ok(())
    }

    fn handle_nary_relationship(
        &mut self,
        rid: RelationshipId,
        rel: &crate::models::conceptual::Relationship,
    ) -> Result<()> {
        self.emit_associative_table(rid, rel)
    }

    /// Emit a junction/associative table for `rel`, with FKs to each endpoint
    /// and any descriptive attributes promoted to columns.
    fn emit_associative_table(
        &mut self,
        rid: RelationshipId,
        rel: &crate::models::conceptual::Relationship,
    ) -> Result<()> {
        let table_name = self.identifier(&rel.name);
        let new_tid = self.out.add_table(table_name);
        let mut composite_pk: Vec<ColumnId> = Vec::new();

        for (idx, ep) in rel.endpoints.iter().enumerate() {
            let target_tid = self.entity_to_table[&ep.entity];
            let target_name = self.entity_name(ep.entity)?.to_string();
            let target_pk = self.primary_columns.get(&ep.entity).cloned().unwrap_or_default();
            let mut local: Vec<ColumnId> = Vec::new();
            let mut foreign: Vec<ColumnId> = Vec::new();
            for (_aid, pcol, ty) in &target_pk {
                let pcol_name =
                    self.out.table(target_tid)?.column(*pcol).unwrap().name.clone();
                let prefix = if rel.is_self() {
                    format!("{}_{}", target_name, idx + 1)
                } else {
                    target_name.clone()
                };
                let name = format!("{}_{}{}", prefix, pcol_name, self.opts.fk_suffix());
                let id = self.out.add_column(new_tid, self.identifier(&name), ty.clone())?;
                self.out.table_mut(new_tid)?.columns.get_mut(&id).unwrap().nullable = false;
                local.push(id);
                composite_pk.push(id);
                foreign.push(*pcol);
            }
            if !local.is_empty() {
                self.out.add_foreign_key(
                    new_tid,
                    ForeignKey {
                        columns: local,
                        references_table: target_tid,
                        references_columns: foreign,
                        on_update: ReferentialAction::Cascade,
                        on_delete: ReferentialAction::Cascade,
                    },
                )?;
            }
        }
        for aid in &rel.attributes {
            let attr = self.src.attribute(*aid)?;
            let id = self.out.add_column(
                new_tid,
                self.identifier(&attr.name),
                attr.data_type.clone(),
            )?;
            self.out.table_mut(new_tid)?.columns.get_mut(&id).unwrap().nullable = attr.is_optional;
        }
        if !composite_pk.is_empty() {
            self.out.set_primary_key(new_tid, composite_pk)?;
        }
        self.relationship_table.insert(rid, new_tid);
        Ok(())
    }

    fn handle_associative_entities(&mut self) -> Result<()> {
        for assoc in self.src.associative_entities.values() {
            let tid = match self.relationship_table.get(&assoc.relationship) {
                Some(t) => *t,
                None => {
                    let rel = self.src.relationship(assoc.relationship)?.clone();
                    self.emit_associative_table(assoc.relationship, &rel)?;
                    self.relationship_table[&assoc.relationship]
                }
            };
            let new_name = self.identifier(&assoc.name);
            self.out.table_mut(tid)?.name = new_name;
        }
        Ok(())
    }

    fn entity_name(&self, eid: EntityId) -> Result<&str> {
        Ok(self.src.entity(eid)?.name.as_str())
    }

    fn identifier(&self, raw: &str) -> String {
        if !self.opts.sanitize_identifiers {
            return raw.to_string();
        }
        raw.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cardinality::Cardinality;
    use crate::models::conceptual::{AttributeOwner, ConceptualModel, SpecializationKind};
    use crate::models::types::DataType;
    use pretty_assertions::assert_eq;

    fn library() -> ConceptualModel {
        let mut m = ConceptualModel::new("library");
        let book = m.add_entity("Book");
        let author = m.add_entity("Author");
        m.add_primary_attribute(book, "id", DataType::Integer).unwrap();
        m.add_attribute(AttributeOwner::Entity(book), "title", DataType::Varchar(255)).unwrap();
        m.add_primary_attribute(author, "id", DataType::Integer).unwrap();
        m.relate("wrote", book, Cardinality::ZeroToMany)
            .with(author, Cardinality::OneToMany)
            .id();
        m
    }

    #[test]
    fn n_to_m_creates_junction_table() {
        let m = library();
        let logical = conceptual_to_logical(&m, &ConvertOptions::default()).unwrap();
        assert_eq!(logical.tables.len(), 3);
        let names: Vec<&str> =
            logical.tables.values().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"Book"));
        assert!(names.contains(&"Author"));
        assert!(names.contains(&"wrote"));
        let junction = logical
            .tables
            .values()
            .find(|t| t.name == "wrote")
            .unwrap();
        let fk_count = junction
            .constraints
            .values()
            .filter(|c| c.is_foreign_key())
            .count();
        assert_eq!(fk_count, 2);
        assert_eq!(junction.primary_key_columns().len(), 2);
    }

    #[test]
    fn one_to_many_propagates_fk() {
        let mut m = ConceptualModel::new("hr");
        let dept = m.add_entity("Department");
        let emp = m.add_entity("Employee");
        m.add_primary_attribute(dept, "id", DataType::Integer).unwrap();
        m.add_primary_attribute(emp, "id", DataType::Integer).unwrap();
        m.add_attribute(AttributeOwner::Entity(emp), "name", DataType::Varchar(120)).unwrap();
        m.relate("works_for", emp, Cardinality::OneToOne)
            .with(dept, Cardinality::ZeroToMany)
            .id();

        let l = conceptual_to_logical(&m, &ConvertOptions::default()).unwrap();
        let employee = l.tables.values().find(|t| t.name == "Employee").unwrap();
        let fks: Vec<_> = employee
            .constraints
            .values()
            .filter(|c| c.is_foreign_key())
            .collect();
        assert_eq!(fks.len(), 1);
        let department = l.tables.values().find(|t| t.name == "Department").unwrap();
        assert!(!department.constraints.values().any(|c| c.is_foreign_key()));
    }

    #[test]
    fn one_to_one_optional_propagates_fk() {
        let mut m = ConceptualModel::new("auth");
        let user = m.add_entity("User");
        let prof = m.add_entity("Profile");
        m.add_primary_attribute(user, "id", DataType::Integer).unwrap();
        m.add_primary_attribute(prof, "id", DataType::Integer).unwrap();
        m.relate("has_profile", user, Cardinality::ZeroToOne)
            .with(prof, Cardinality::OneToOne)
            .id();

        let l = conceptual_to_logical(&m, &ConvertOptions::default()).unwrap();
        let user_t = l.tables.values().find(|t| t.name == "User").unwrap();
        assert!(user_t.constraints.values().any(|c| c.is_foreign_key()));
    }

    #[test]
    fn ternary_creates_junction() {
        let mut m = ConceptualModel::new("supply");
        let supplier = m.add_entity("Supplier");
        let part = m.add_entity("Part");
        let project = m.add_entity("Project");
        for e in [supplier, part, project] {
            m.add_primary_attribute(e, "id", DataType::Integer).unwrap();
        }
        m.relate("supplies", supplier, Cardinality::ZeroToMany)
            .with(part, Cardinality::ZeroToMany)
            .with(project, Cardinality::ZeroToMany)
            .carry("quantity", DataType::Integer)
            .id();

        let l = conceptual_to_logical(&m, &ConvertOptions::default()).unwrap();
        let junction = l.tables.values().find(|t| t.name == "supplies").unwrap();
        assert_eq!(junction.primary_key_columns().len(), 3);
        let fk_count = junction
            .constraints
            .values()
            .filter(|c| c.is_foreign_key())
            .count();
        assert_eq!(fk_count, 3);
        assert!(junction.columns.values().any(|c| c.name == "quantity"));
    }

    #[test]
    fn specialization_one_per_class() {
        let mut m = ConceptualModel::new("vehicles");
        let v = m.add_entity("Vehicle");
        m.add_primary_attribute(v, "id", DataType::Integer).unwrap();
        let car = m.add_entity("Car");
        m.add_attribute(AttributeOwner::Entity(car), "wheels", DataType::Integer).unwrap();
        let boat = m.add_entity("Boat");
        m.add_attribute(AttributeOwner::Entity(boat), "tonnage", DataType::Integer).unwrap();
        m.add_specialization("kind", v, vec![car, boat], SpecializationKind::PARTIAL_DISJOINT)
            .unwrap();

        let l = conceptual_to_logical(&m, &ConvertOptions::default()).unwrap();
        assert_eq!(l.tables.len(), 3);
        let car_t = l.tables.values().find(|t| t.name == "Car").unwrap();
        assert_eq!(car_t.primary_key_columns().len(), 1);
        assert!(car_t.constraints.values().any(|c| c.is_foreign_key()));
    }
}
