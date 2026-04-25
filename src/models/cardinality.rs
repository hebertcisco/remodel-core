//! Cardinality of a relationship endpoint.
//!
//! Mirrors the four cardinality kinds used by brModelo
//! (`C01`, `C11`, `C0N`, `C1N`).

use serde::{Deserialize, Serialize};

/// Cardinality of one end of a relationship.
///
/// The four variants correspond, in order, to the integer codes used by
/// brModelo's persistence format:
///
/// | code | variant         | min | max  | notation |
/// |------|-----------------|-----|------|----------|
/// | 0    | `ZeroToOne`     | 0   | 1    | `0..1`   |
/// | 1    | `OneToOne`      | 1   | 1    | `1..1`   |
/// | 2    | `ZeroToMany`    | 0   | n    | `0..N`   |
/// | 3    | `OneToMany`     | 1   | n    | `1..N`   |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cardinality {
    /// Optional, single — `0..1`.
    ZeroToOne,
    /// Mandatory, single — `1..1`.
    OneToOne,
    /// Optional, multiple — `0..N`.
    ZeroToMany,
    /// Mandatory, multiple — `1..N`.
    OneToMany,
}

impl Cardinality {
    /// `true` when the maximum side allows more than one occurrence.
    pub fn is_many(self) -> bool {
        matches!(self, Self::ZeroToMany | Self::OneToMany)
    }

    /// `true` when the minimum side requires at least one occurrence
    /// (i.e. the foreign key produced for this side must be `NOT NULL`).
    pub fn is_mandatory(self) -> bool {
        matches!(self, Self::OneToOne | Self::OneToMany)
    }

    /// brModelo wire format integer code (0..=3).
    pub fn code(self) -> u8 {
        match self {
            Self::ZeroToOne => 0,
            Self::OneToOne => 1,
            Self::ZeroToMany => 2,
            Self::OneToMany => 3,
        }
    }

    /// Inverse of [`Self::code`]. Returns `None` for unknown codes.
    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::ZeroToOne,
            1 => Self::OneToOne,
            2 => Self::ZeroToMany,
            3 => Self::OneToMany,
            _ => return None,
        })
    }

    /// Human-readable notation, e.g. `"0..N"`.
    pub fn notation(self) -> &'static str {
        match self {
            Self::ZeroToOne => "0..1",
            Self::OneToOne => "1..1",
            Self::ZeroToMany => "0..N",
            Self::OneToMany => "1..N",
        }
    }
}

impl std::fmt::Display for Cardinality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.notation())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_codes() {
        for c in [
            Cardinality::ZeroToOne,
            Cardinality::OneToOne,
            Cardinality::ZeroToMany,
            Cardinality::OneToMany,
        ] {
            assert_eq!(Cardinality::from_code(c.code()), Some(c));
        }
        assert_eq!(Cardinality::from_code(99), None);
    }

    #[test]
    fn is_many_and_mandatory() {
        assert!(!Cardinality::ZeroToOne.is_many());
        assert!(!Cardinality::ZeroToOne.is_mandatory());
        assert!(Cardinality::OneToOne.is_mandatory());
        assert!(Cardinality::OneToMany.is_many());
        assert!(Cardinality::OneToMany.is_mandatory());
        assert!(Cardinality::ZeroToMany.is_many());
    }
}
