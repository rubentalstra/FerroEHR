//! `PROPORTION_KIND` — class of enumeration constants defining types of
//! proportion for the `DV_PROPORTION` class.
//!
//! openEHR class: `PROPORTION_KIND`, package `rm.data_types.quantity`.
//! Inherits: none listed (spec table has no `Inherit` row).
//!
//! Class of enumeration constants defining types of proportion for the
//! `DV_PROPORTION` class.
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `PROPORTION_KIND` is a **constants-only** class: its per-class table has
/// no `Attributes` section at all, only a `Constants` section (five named
/// `Integer` values) and a single `Functions` section (`valid_proportion_kind`).
///
/// Transcribed as a Rust `enum` with explicit discriminants matching the
/// spec's assigned integer values exactly, rather than as a zero-sized
/// constants-holder struct (the pattern used for genuinely field-less
/// "namespace" classes like `Time_Definitions` or `BASIC_DEFINITIONS`) —
/// because `PROPORTION_KIND`'s five constants form a closed, mutually
/// exclusive set of *values* a `DV_PROPORTION.type_` attribute actually
/// holds (see `dv_proportion.rs::DvProportion::type_`), not merely
/// documentation-time named literals referenced occasionally. A closed enum
/// makes `Type_validity` (`valid_proportion_kind (type)`) true by
/// construction for any `ProportionKind` value, matching ADR-001 §4's
/// "closed subtype set → enum" pattern applied here to an *enumeration*
/// class rather than a class hierarchy — a reasonable extension of the same
/// principle, flagged as a judgment call since ADR-001 §4 itself only
/// discusses class hierarchies (`DATA_VALUE`, `ITEM`, etc.), not
/// Eiffel-style named-integer-constant classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// PORT NOTE: schema-verified — `openehr_rm_1.1.0_all.json` (ITS-JSON @
// 5acae05), `#/definitions/DV_PROPORTION/properties/type`, types this field
// `{"type": "integer"}`, a JSON number, matching the spec's own literal
// `Integer` typing of `DV_PROPORTION.type`. A plain serde derive would emit
// the variant NAME as a string (`"Percent"`) — wrong on the wire — so
// `Serialize`/`Deserialize` are written by hand below: serialize as the raw
// `i32` discriminant, deserialize from an `i32` through the existing
// `TryFrom<i32>` validity machinery. Per ADR-002, enumerations carry no
// `_type` discriminator of their own (they are values, not RM objects).
#[repr(i32)]
pub enum ProportionKind {
    /// `pk_ratio` = 0.
    ///
    /// Ratio type. Numerator and denominator may be any value.
    Ratio = 0,

    /// `pk_unitary` = 1.
    ///
    /// Denominator must be 1.
    Unitary = 1,

    /// `pk_percent` = 2.
    ///
    /// Denominator is 100, numerator is understood as a percentage value.
    Percent = 2,

    /// `pk_fraction` = 3.
    ///
    /// Numerator and denominator are integral, and the presentation method
    /// uses a slash, e.g. 1/2.
    Fraction = 3,

    /// `pk_integer_fraction` = 4.
    ///
    /// Numerator and denominator are integral, and the presentation method
    /// uses a slash, e.g. 1/2; if the numerator is greater than the
    /// denominator, e.g. n=3, d=2, the presentation is 1 1/2.
    IntegerFraction = 4,
}

impl ProportionKind {
    /// `valid_proportion_kind(nq: Integer) -> Boolean`.
    ///
    /// True if `nq` is one of the defined types.
    ///
    /// PORT NOTE: the spec's own signature takes a raw `Integer` parameter
    /// (`nq`), consistent with `type` being declared `Integer` on
    /// `DV_PROPORTION` rather than this enum type itself. Transcribed as a
    /// free function taking an `i32` here (rather than a `ProportionKind`,
    /// which would make the check trivially always-`true` and defeat the
    /// purpose of a runtime validity check against an arbitrary integer),
    /// so callers holding a raw `Integer`/`i32` — as the spec's own
    /// `DV_PROPORTION.type` attribute is declared, before this file's own
    /// enum-narrowing judgment call in `dv_proportion.rs` — can still
    /// validate it before converting to [`ProportionKind`].
    pub fn valid_proportion_kind(nq: i32) -> bool {
        matches!(nq, 0..=4)
    }

    /// Attempts to convert a raw `Integer` value into a [`ProportionKind`],
    /// returning `None` if `nq` is not one of the five defined values.
    ///
    /// PORT NOTE: not itself a row in the spec's table — added as the
    /// natural companion to [`Self::valid_proportion_kind`] given this
    /// file's own enum-narrowing judgment call (see the struct-level doc
    /// comment); every RM attribute/function beyond this point is
    /// transcribed literally, this one conversion helper is not.
    pub fn from_i32(nq: i32) -> Option<Self> {
        match nq {
            0 => Some(ProportionKind::Ratio),
            1 => Some(ProportionKind::Unitary),
            2 => Some(ProportionKind::Percent),
            3 => Some(ProportionKind::Fraction),
            4 => Some(ProportionKind::IntegerFraction),
            _ => None,
        }
    }
}

/// Manual canonical-JSON serialization: emits the raw `i32` discriminant
/// (schema-verified `DV_PROPORTION.type: {"type": "integer"}`), never the
/// variant name string a derive would produce. See the enum-level PORT NOTE.
impl Serialize for ProportionKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

/// Manual canonical-JSON deserialization: reads a bare integer and validates
/// it through the existing [`TryFrom<i32>`] machinery (itself delegating to
/// [`ProportionKind::from_i32`]), so an out-of-range value fails with
/// [`InvalidProportionKind`] rather than silently mapping.
impl<'de> Deserialize<'de> for ProportionKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = i32::deserialize(deserializer)?;
        ProportionKind::try_from(raw).map_err(serde::de::Error::custom)
    }
}

/// Integer-conversion companion to the manual `Serialize` impl above —
/// canonical-JSON serialization target, per the enum-level PORT NOTE
/// (schema-verified `DV_PROPORTION.type: {"type": "integer"}`).
impl From<ProportionKind> for i32 {
    fn from(kind: ProportionKind) -> Self {
        kind as i32
    }
}

/// Integer-conversion source for the manual `Deserialize` impl above.
/// Delegates to [`ProportionKind::from_i32`]; the rejected `i32` is
/// returned as-is in `Self::Error` (serde requires a `Display`-able error
/// type for its error-context wrapping — see [`InvalidProportionKind`]).
impl TryFrom<i32> for ProportionKind {
    type Error = InvalidProportionKind;

    fn try_from(nq: i32) -> Result<Self, Self::Error> {
        ProportionKind::from_i32(nq).ok_or(InvalidProportionKind(nq))
    }
}

/// Error returned by `TryFrom<i32> for ProportionKind` when the input is
/// not one of the five legal `PROPORTION_KIND` values.
///
/// PORT NOTE: not itself a spec-declared type — the minimal
/// `std::error::Error` wrapper the manual `Deserialize` impl needs around
/// the rejected value, since `serde::de::Error::custom` requires a
/// `std::fmt::Display` value for its deserialization error message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidProportionKind(pub i32);

impl std::fmt::Display for InvalidProportionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid PROPORTION_KIND value: {}", self.0)
    }
}

impl std::error::Error for InvalidProportionKind {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/proportion_kind.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / proportion_kind.adoc §PROPORTION_KIND Class
//   confidence: high
//   todos: 0
//   note: discriminants verified against the published Constants table (pk_ratio=0, pk_unitary=1, pk_percent=2, pk_fraction=3, pk_integer_fraction=4); valid_proportion_kind kept as a raw-i32 free function (not a ProportionKind method, which would be trivially always-true) plus an added from_i32 conversion helper not itself drawn from the spec table. P4/ADR-002: hand-written Serialize (raw i32 discriminant) and Deserialize (i32 via TryFrom validity machinery) — a plain derive would emit the variant name string, wrong against the schema (DV_PROPORTION.type: integer); enumerations carry no _type tag per ADR-002; InvalidProportionKind is the Display-able error the manual Deserialize needs.
// ─────────────────────────────────────────────
