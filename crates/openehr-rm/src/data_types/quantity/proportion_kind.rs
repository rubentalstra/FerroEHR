//! `PROPORTION_KIND` — class of enumeration constants defining types of
//! proportion for the `DV_PROPORTION` class.
//!
//! openEHR class: `PROPORTION_KIND`, package `rm.data_types.quantity`.
//! Inherits: none listed (spec table has no `Inherit` row).
//!
//! Class of enumeration constants defining types of proportion for the
//! `DV_PROPORTION` class.

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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/proportion_kind.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / proportion_kind.adoc §PROPORTION_KIND Class
//   confidence: high
//   todos: 0
//   note: discriminants verified against the published Constants table (pk_ratio=0, pk_unitary=1, pk_percent=2, pk_fraction=3, pk_integer_fraction=4); valid_proportion_kind kept as a raw-i32 free function (not a ProportionKind method, which would be trivially always-true) plus an added from_i32 conversion helper not itself drawn from the spec table.
// ─────────────────────────────────────────────
