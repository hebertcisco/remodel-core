//! Conceptual (Entity-Relationship) model.
//!
//! The conceptual model is a directed multigraph of entities, relationships,
//! attributes, specializations, and unions. Elements are referenced by typed
//! [`u32`] handles (à la ECS) rather than by `Rc`/`Arc`, which keeps the model
//! easy to serialize, clone, and mutate without lifetime pain.
//!
//! ## Mapping from brModelo
//!
//! | brModelo class           | RemodelCore type                 |
//! |--------------------------|----------------------------------|
//! | `Entidade`               | [`Entity`]                       |
//! | `Atributo`               | [`Attribute`]                    |
//! | `Relacionamento`         | [`Relationship`]                 |
//! | `Especializacao`         | [`Specialization`]               |
//! | `Uniao`                  | [`Union`]                        |
//! | `EntidadeAssociativa`    | [`AssociativeEntity`]            |
//! | `Cardinalidade`          | [`crate::models::cardinality::Cardinality`] |
//! | `DiagramaConceitual`     | [`ConceptualModel`]              |

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::models::cardinality::Cardinality;
use crate::models::types::DataType;

/// Strongly-typed handle for an [`Entity`] inside a [`ConceptualModel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityId(pub u32);

/// Strongly-typed handle for an [`Attribute`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AttributeId(pub u32);

/// Strongly-typed handle for a [`Relationship`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationshipId(pub u32);

/// Strongly-typed handle for a [`Specialization`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SpecializationId(pub u32);

/// Strongly-typed handle for a [`Union`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UnionId(pub u32);

/// Strongly-typed handle for an [`AssociativeEntity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AssociativeEntityId(pub u32);

/// What kind of attribute this is.
///
/// Mirrors brModelo's flags on `Atributo`. A single attribute can be both
/// `Primary` and another kind in the original Java model (an attribute is
/// identified by a `boolean isIdentificador`); here that is split into
/// `is_primary` (a flag on [`Attribute`]) and `kind` (the structural kind).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AttributeKind {
    /// A plain single-valued attribute.
    #[default]
    Simple,
    /// A composite attribute, made of nested sub-attributes.
    Composite,
    /// A multivalued attribute. The pair carries the cardinality bounds
    /// `(min, max)`; `max == None` means unbounded.
    Multivalued {
        /// Minimum number of values; `0` makes the attribute optional.
        min: u32,
        /// Maximum number of values, or `None` for unbounded (`*`).
        max: Option<u32>,
    },
    /// A derived attribute, computed from other attributes.
    Derived,
}

/// An ER entity. Holds owned attributes and a name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    /// Unique handle within the owning model.
    pub id: EntityId,
    /// Display name (e.g. `"Customer"`).
    pub name: String,
    /// Free-form note shown in the inspector panel.
    pub note: String,
    /// IDs of attributes owned by this entity, in author order.
    pub attributes: Vec<AttributeId>,
    /// `true` for *weak entities*, which require an identifying relationship
    /// to be uniquely identified.
    pub weak: bool,
}

/// An attribute belonging to either an [`Entity`] or a [`Relationship`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Unique handle within the owning model.
    pub id: AttributeId,
    /// Display name (e.g. `"first_name"`).
    pub name: String,
    /// Logical data type (mapped to a column type during conversion).
    pub data_type: DataType,
    /// Whether this attribute participates in the entity's primary identifier.
    pub is_primary: bool,
    /// Whether this attribute is a *partial* key — combined with the
    /// owning entity's identifying relationship to form a key (typical for
    /// weak entities).
    pub is_partial_key: bool,
    /// Whether the attribute admits a null value.
    pub is_optional: bool,
    /// Structural kind of the attribute.
    pub kind: AttributeKind,
    /// For composite attributes, the IDs of the sub-attributes that compose
    /// this one. Empty for non-composite attributes.
    pub children: Vec<AttributeId>,
}

/// One endpoint of a [`Relationship`].
///
/// The cardinality annotation follows the **"look-here"** convention used in
/// Heuser's textbook and most modern UML tools: the cardinality next to
/// entity *E* describes **how many tuples of the other entity** participate
/// per tuple of *E*.
///
/// For example, in a `wrote` relationship between `Book` and `Author`,
/// recording `Book.cardinality = ZeroToMany` means *"each book has 0..N
/// authors"* (so `Author` is the many side from `Book`'s perspective).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipEndpoint {
    /// The entity participating at this endpoint.
    pub entity: EntityId,
    /// How many tuples of the *other* entity participate per tuple of
    /// `entity`. See the type-level docs for the convention.
    pub cardinality: Cardinality,
    /// Optional role label (brModelo's `Papel`).
    pub role: Option<String>,
}

/// A relationship between two or more entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique handle within the owning model.
    pub id: RelationshipId,
    /// Display name (e.g. `"works_for"`).
    pub name: String,
    /// Endpoints, in author order. A relationship is *binary* iff
    /// `endpoints.len() == 2`, *self* iff all endpoints share an entity.
    pub endpoints: Vec<RelationshipEndpoint>,
    /// Attributes carried by the relationship itself (descriptive attributes).
    pub attributes: Vec<AttributeId>,
}

impl Relationship {
    /// `true` if every endpoint refers to the same entity.
    pub fn is_self(&self) -> bool {
        if self.endpoints.is_empty() {
            return false;
        }
        let first = self.endpoints[0].entity;
        self.endpoints.iter().all(|e| e.entity == first)
    }

    /// `true` for binary relationships (exactly 2 endpoints).
    pub fn is_binary(&self) -> bool {
        self.endpoints.len() == 2
    }

    /// `true` if at least three distinct entity endpoints participate.
    pub fn is_nary(&self) -> bool {
        self.endpoints.len() >= 3
    }
}

/// Whether a specialization is total (every parent instance is in some child)
/// or partial (some parent instances may be in no child), and whether it is
/// disjoint (an instance can be in at most one child) or overlapping
/// (an instance can be in several children).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SpecializationKind {
    /// `true` if every parent instance must belong to some child.
    pub total: bool,
    /// `true` if a parent instance may belong to multiple children at once.
    pub overlapping: bool,
}

impl SpecializationKind {
    /// Partial + disjoint: the most permissive variant (the default).
    pub const PARTIAL_DISJOINT: Self = Self { total: false, overlapping: false };
    /// Total + disjoint.
    pub const TOTAL_DISJOINT: Self = Self { total: true, overlapping: false };
    /// Partial + overlapping.
    pub const PARTIAL_OVERLAPPING: Self = Self { total: false, overlapping: true };
    /// Total + overlapping.
    pub const TOTAL_OVERLAPPING: Self = Self { total: true, overlapping: true };
}

/// IS-A specialization (generalization) connecting one parent entity to
/// several child entities. brModelo calls this `Especializacao`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Specialization {
    /// Unique handle within the owning model.
    pub id: SpecializationId,
    /// Optional name shown next to the specialization gadget.
    pub name: String,
    /// The supertype.
    pub parent: EntityId,
    /// The subtypes.
    pub children: Vec<EntityId>,
    /// Discriminator flags.
    pub kind: SpecializationKind,
}

/// Union/category construct: several parent entities feed a single category
/// child entity (brModelo's `Uniao`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Union {
    /// Unique handle within the owning model.
    pub id: UnionId,
    /// Optional name shown next to the union gadget.
    pub name: String,
    /// Parent entities whose union forms `category`.
    pub parents: Vec<EntityId>,
    /// The category entity (the union of `parents`).
    pub category: EntityId,
}

/// An associative entity wraps a relationship so that it can itself
/// participate in further relationships (brModelo's `EntidadeAssociativa`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssociativeEntity {
    /// Unique handle within the owning model.
    pub id: AssociativeEntityId,
    /// The relationship that this associative entity wraps.
    pub relationship: RelationshipId,
    /// Display name; defaults to the wrapped relationship's name.
    pub name: String,
}

/// The full conceptual model: entities, attributes, relationships, and the
/// higher-level constructs (specialization, union, associative).
///
/// Element ordering is deterministic (author order) so that round-tripping
/// through serialization preserves the diagram.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptualModel {
    /// Diagram name (typically the project or schema name).
    pub name: String,
    /// Monotonic counter used to mint unique IDs.
    next_id: u32,
    /// Entities, keyed for O(1) lookup; iteration order is insertion order.
    pub entities: IndexMap<EntityId, Entity>,
    /// Attributes shared between entities and relationships.
    pub attributes: IndexMap<AttributeId, Attribute>,
    /// Relationships.
    pub relationships: IndexMap<RelationshipId, Relationship>,
    /// Specializations.
    pub specializations: IndexMap<SpecializationId, Specialization>,
    /// Unions / categories.
    pub unions: IndexMap<UnionId, Union>,
    /// Associative entities.
    pub associative_entities: IndexMap<AssociativeEntityId, AssociativeEntity>,
}

impl ConceptualModel {
    /// Create a new empty model with the given diagram name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    fn mint(&mut self) -> u32 {
        self.next_id = self.next_id.checked_add(1).expect("ID space exhausted");
        self.next_id
    }

    /// Add a new entity with the given name. Returns its handle.
    pub fn add_entity(&mut self, name: impl Into<String>) -> EntityId {
        let id = EntityId(self.mint());
        self.entities.insert(
            id,
            Entity {
                id,
                name: name.into(),
                note: String::new(),
                attributes: Vec::new(),
                weak: false,
            },
        );
        id
    }

    /// Add a new *weak* entity. Functionally identical to
    /// [`Self::add_entity`] but flips the `weak` flag, which downstream
    /// conversion uses to require an identifying relationship.
    pub fn add_weak_entity(&mut self, name: impl Into<String>) -> EntityId {
        let id = self.add_entity(name);
        self.entity_mut(id).expect("just inserted").weak = true;
        id
    }

    /// Borrow an entity by its handle.
    pub fn entity(&self, id: EntityId) -> Result<&Entity> {
        self.entities
            .get(&id)
            .ok_or_else(|| Error::UnknownReference { kind: "entity", id: format!("{}", id.0) })
    }

    /// Mutably borrow an entity by its handle.
    pub fn entity_mut(&mut self, id: EntityId) -> Result<&mut Entity> {
        self.entities
            .get_mut(&id)
            .ok_or_else(|| Error::UnknownReference { kind: "entity", id: format!("{}", id.0) })
    }

    /// Add a new attribute and attach it to `owner`.
    ///
    /// `owner` may be either an [`EntityId`] or a [`RelationshipId`]; the
    /// attribute is appended to that owner's attribute list.
    pub fn add_attribute(
        &mut self,
        owner: AttributeOwner,
        name: impl Into<String>,
        data_type: DataType,
    ) -> Result<AttributeId> {
        let id = AttributeId(self.mint());
        self.attributes.insert(
            id,
            Attribute {
                id,
                name: name.into(),
                data_type,
                is_primary: false,
                is_partial_key: false,
                is_optional: false,
                kind: AttributeKind::Simple,
                children: Vec::new(),
            },
        );
        match owner {
            AttributeOwner::Entity(eid) => self.entity_mut(eid)?.attributes.push(id),
            AttributeOwner::Relationship(rid) => self.relationship_mut(rid)?.attributes.push(id),
        }
        Ok(id)
    }

    /// Convenience: add a primary-key attribute to an entity.
    pub fn add_primary_attribute(
        &mut self,
        entity: EntityId,
        name: impl Into<String>,
        data_type: DataType,
    ) -> Result<AttributeId> {
        let id = self.add_attribute(AttributeOwner::Entity(entity), name, data_type)?;
        let attr = self.attribute_mut(id)?;
        attr.is_primary = true;
        Ok(id)
    }

    /// Borrow an attribute by its handle.
    pub fn attribute(&self, id: AttributeId) -> Result<&Attribute> {
        self.attributes
            .get(&id)
            .ok_or_else(|| Error::UnknownReference { kind: "attribute", id: format!("{}", id.0) })
    }

    /// Mutably borrow an attribute.
    pub fn attribute_mut(&mut self, id: AttributeId) -> Result<&mut Attribute> {
        self.attributes
            .get_mut(&id)
            .ok_or_else(|| Error::UnknownReference { kind: "attribute", id: format!("{}", id.0) })
    }

    /// Start building a relationship. The returned [`RelationshipBuilder`]
    /// records each endpoint and is finalized by dropping it (the relationship
    /// is inserted into the model immediately on construction; `with` mutates
    /// it in place).
    pub fn relate(
        &mut self,
        name: impl Into<String>,
        first: EntityId,
        first_card: Cardinality,
    ) -> RelationshipBuilder<'_> {
        let id = RelationshipId(self.mint());
        let rel = Relationship {
            id,
            name: name.into(),
            endpoints: vec![RelationshipEndpoint {
                entity: first,
                cardinality: first_card,
                role: None,
            }],
            attributes: Vec::new(),
        };
        self.relationships.insert(id, rel);
        RelationshipBuilder { model: self, id }
    }

    /// Borrow a relationship by its handle.
    pub fn relationship(&self, id: RelationshipId) -> Result<&Relationship> {
        self.relationships.get(&id).ok_or_else(|| Error::UnknownReference {
            kind: "relationship",
            id: format!("{}", id.0),
        })
    }

    /// Mutably borrow a relationship.
    pub fn relationship_mut(&mut self, id: RelationshipId) -> Result<&mut Relationship> {
        self.relationships.get_mut(&id).ok_or_else(|| Error::UnknownReference {
            kind: "relationship",
            id: format!("{}", id.0),
        })
    }

    /// Add a new specialization.
    pub fn add_specialization(
        &mut self,
        name: impl Into<String>,
        parent: EntityId,
        children: Vec<EntityId>,
        kind: SpecializationKind,
    ) -> Result<SpecializationId> {
        if children.len() < 2 {
            return Err(Error::InvalidSpecialization(format!(
                "specialization `{}` must have at least 2 children, got {}",
                name.into(),
                children.len()
            )));
        }
        let _ = self.entity(parent)?;
        for c in &children {
            let _ = self.entity(*c)?;
        }
        let id = SpecializationId(self.mint());
        let name = name.into();
        self.specializations.insert(id, Specialization { id, name, parent, children, kind });
        Ok(id)
    }

    /// Add a new union/category construct.
    pub fn add_union(
        &mut self,
        name: impl Into<String>,
        parents: Vec<EntityId>,
        category: EntityId,
    ) -> Result<UnionId> {
        if parents.len() < 2 {
            return Err(Error::InvalidSpecialization(format!(
                "union `{}` must have at least 2 parents, got {}",
                name.into(),
                parents.len()
            )));
        }
        let _ = self.entity(category)?;
        for p in &parents {
            let _ = self.entity(*p)?;
        }
        let id = UnionId(self.mint());
        self.unions
            .insert(id, Union { id, name: name.into(), parents, category });
        Ok(id)
    }

    /// Wrap a relationship as an associative entity.
    pub fn add_associative_entity(
        &mut self,
        relationship: RelationshipId,
    ) -> Result<AssociativeEntityId> {
        let rel = self.relationship(relationship)?;
        let name = rel.name.clone();
        let id = AssociativeEntityId(self.mint());
        self.associative_entities
            .insert(id, AssociativeEntity { id, relationship, name });
        Ok(id)
    }
}

/// Owner reference passed to [`ConceptualModel::add_attribute`].
#[derive(Debug, Clone, Copy)]
pub enum AttributeOwner {
    /// Attach the attribute to an entity.
    Entity(EntityId),
    /// Attach the attribute to a relationship (descriptive attribute).
    Relationship(RelationshipId),
}

/// Fluent builder returned by [`ConceptualModel::relate`].
///
/// Use [`with`](Self::with) to add additional endpoints (binary, ternary, …)
/// and [`carry`](Self::carry) to attach descriptive attributes.
pub struct RelationshipBuilder<'m> {
    model: &'m mut ConceptualModel,
    id: RelationshipId,
}

impl<'m> RelationshipBuilder<'m> {
    /// Add another endpoint to this relationship.
    pub fn with(self, entity: EntityId, cardinality: Cardinality) -> Self {
        if let Some(rel) = self.model.relationships.get_mut(&self.id) {
            rel.endpoints.push(RelationshipEndpoint { entity, cardinality, role: None });
        }
        self
    }

    /// Set a role label on the most recently added endpoint.
    pub fn with_role(self, role: impl Into<String>) -> Self {
        if let Some(rel) = self.model.relationships.get_mut(&self.id) {
            if let Some(last) = rel.endpoints.last_mut() {
                last.role = Some(role.into());
            }
        }
        self
    }

    /// Attach a descriptive attribute to the relationship.
    pub fn carry(self, name: impl Into<String>, data_type: DataType) -> Self {
        let _ = self
            .model
            .add_attribute(AttributeOwner::Relationship(self.id), name, data_type);
        self
    }

    /// Finalize the builder and return the relationship's handle.
    pub fn id(self) -> RelationshipId {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConceptualModel {
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
    fn build_basic_model() {
        let m = sample();
        assert_eq!(m.entities.len(), 2);
        assert_eq!(m.attributes.len(), 3);
        assert_eq!(m.relationships.len(), 1);
        let rel = m.relationships.values().next().unwrap();
        assert!(rel.is_binary());
        assert!(!rel.is_self());
    }

    #[test]
    fn relationship_self_check() {
        let mut m = ConceptualModel::new("hr");
        let person = m.add_entity("Person");
        let rel = m
            .relate("manages", person, Cardinality::ZeroToOne)
            .with(person, Cardinality::ZeroToMany)
            .id();
        assert!(m.relationship(rel).unwrap().is_self());
    }

    #[test]
    fn specialization_requires_two_children() {
        let mut m = ConceptualModel::new("vehicles");
        let v = m.add_entity("Vehicle");
        let car = m.add_entity("Car");
        let err = m
            .add_specialization("kind", v, vec![car], SpecializationKind::PARTIAL_DISJOINT)
            .unwrap_err();
        assert!(matches!(err, Error::InvalidSpecialization(_)));
    }

    #[test]
    fn unknown_entity_error() {
        let m = ConceptualModel::new("x");
        let err = m.entity(EntityId(99)).unwrap_err();
        assert!(matches!(err, Error::UnknownReference { kind: "entity", .. }));
    }

    #[test]
    fn json_round_trip() {
        let m = sample();
        let s = serde_json::to_string(&m).unwrap();
        let back: ConceptualModel = serde_json::from_str(&s).unwrap();
        assert_eq!(m.entities.len(), back.entities.len());
        assert_eq!(m.attributes.len(), back.attributes.len());
        assert_eq!(m.relationships.len(), back.relationships.len());
    }
}
