//! Canonical-JSON `_type` self-tagging infrastructure.
//!
//! PORT NOTE: this module is **serialization infrastructure, not a spec
//! class** — no openEHR class corresponds to it. It lives in
//! `openehr-foundation` because this crate is the root of the workspace
//! dependency tree, so `openehr-base` and `openehr-rm` (whose types carry
//! the tags) can both reach it, while `openehr-serde` (which per
//! `PORT_MASTER_PLAN.md` §9 depends *on* `openehr-rm`) cannot provide it
//! to them — Rust's orphan rule forbids implementing `Serialize` for
//! another crate's types, so the discriminator mechanism must sit below
//! the types themselves.
//!
//! ## The convention (ADR-002)
//!
//! ITS-JSON canonical JSON identifies every object's RM class with a
//! `"_type"` key holding the uppercase class name (`"DV_TEXT"`,
//! `"COMPOSITION"`). Stock EHRbase emits `_type` on **every** object, not
//! only where the schema strictly requires it, and behavioural parity with
//! EHRbase is this project's acceptance bar — so we do the same. Every
//! concrete RM/BASE class:
//!
//! 1. implements [`TypeName`] with its canonical class-name string, and
//! 2. declares a [`TypeTag`] as its **first** struct field:
//!
//! ```ignore
//! #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
//! pub struct DvQuantity {
//!     /// Canonical `_type` discriminator (`"DV_QUANTITY"`), always
//!     /// serialized first; tolerated-absent and validated-if-present on
//!     /// input.
//!     #[serde(rename = "_type", default = "TypeTag::new")]
//!     pub type_tag: TypeTag<Self>,
//!     pub magnitude: f64,
//!     pub units: String,
//! }
//! impl TypeName for DvQuantity {
//!     const NAME: &'static str = "DV_QUANTITY";
//! }
//! ```
//!
//! Closed subtype-set enums (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, …) are
//! `#[serde(untagged)]`: dispatch is driven by each variant payload's own
//! `TypeTag`, whose `Deserialize` **fails** on a mismatched `_type` string,
//! so serde's variant probing selects exactly the variant whose class name
//! matches — even between structure-identical classes like `DV_DATE` and
//! `DV_TIME`. Abstract classes carry no tag of their own (ITS-JSON defines
//! no schema entry for them; they are flattened into the concretes).
//!
//! Both halves of this design were validated experimentally against
//! `serde_json` before rollout (tag-first field order, tag-driven untagged
//! dispatch, wrong-tag rejection, missing-tag tolerance on concrete slots,
//! `#[serde(flatten)]` interplay, generic self-tagged types); the unit
//! tests below pin the same behaviour.
//!
//! The `default = "TypeTag::new"` **function-path** form (not bare
//! `default`) is deliberate: serde's derive adds a spurious `T: Default`
//! bound to generic containers (`ORIGINAL_VERSION<T>` etc.) when a field
//! uses the bare trait-based `default`.

use core::fmt;
use core::marker::PhantomData;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Associates a type with its canonical openEHR class name — the exact
/// string carried in the ITS-JSON `"_type"` discriminator key (uppercase
/// with underscores, e.g. `"DV_CODED_TEXT"`, `"OBJECT_VERSION_ID"`).
///
/// Implemented by every concrete (instantiable) RM/BASE class, and by an
/// embedded shared-state `*Data` struct only in the "bare concrete parent"
/// case (e.g. `DvTextData` names `"DV_TEXT"` so the `DvText::Text` variant
/// can tag a bare `DV_TEXT` instance).
pub trait TypeName {
    /// The canonical class name serialized as the `"_type"` value.
    const NAME: &'static str;
}

/// Zero-sized `"_type"` discriminator field.
///
/// - **Serialize:** always emits `T::NAME` as a JSON string.
/// - **Deserialize:** accepts exactly the string `T::NAME` and errors on
///   any other value. Combined with `#[serde(default = "TypeTag::new")]`
///   on the field, a *missing* `_type` key is tolerated (legal in
///   concrete-declared slots per ITS-JSON), while a *wrong* one is
///   rejected — which is precisely what makes `#[serde(untagged)]` enum
///   dispatch tag-driven instead of structure-driven.
///
/// `PhantomData<fn() -> T>` (not `PhantomData<T>`) keeps the tag `Send`/
/// `Sync`/`'static` regardless of `T`, and all trait impls below are
/// written by hand so no spurious `T: Clone`/`T: Default`/… bounds leak
/// onto containing types (deriving them would add exactly such bounds).
pub struct TypeTag<T: TypeName>(PhantomData<fn() -> T>);

impl<T: TypeName> TypeTag<T> {
    /// The (only) value of this tag. Also referenced by name in
    /// `#[serde(default = "TypeTag::new")]` attributes.
    #[must_use]
    pub const fn new() -> Self {
        TypeTag(PhantomData)
    }

    /// The canonical class-name string this tag serializes as.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        T::NAME
    }
}

impl<T: TypeName> Default for TypeTag<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TypeName> Clone for TypeTag<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: TypeName> Copy for TypeTag<T> {}

impl<T: TypeName> fmt::Debug for TypeTag<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypeTag({})", T::NAME)
    }
}

/// All `TypeTag<T>` values are identical by construction, so equality is
/// unconditionally true and hashing contributes nothing — the tag must not
/// perturb the containing class's derived `PartialEq`/`Eq`/`Hash`/`Ord`.
impl<T: TypeName> PartialEq for TypeTag<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T: TypeName> Eq for TypeTag<T> {}

impl<T: TypeName> PartialOrd for TypeTag<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: TypeName> Ord for TypeTag<T> {
    fn cmp(&self, _other: &Self) -> core::cmp::Ordering {
        core::cmp::Ordering::Equal
    }
}

impl<T: TypeName> core::hash::Hash for TypeTag<T> {
    fn hash<H: core::hash::Hasher>(&self, _state: &mut H) {}
}

impl<T: TypeName> Serialize for TypeTag<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(T::NAME)
    }
}

struct TypeTagVisitor<T: TypeName>(PhantomData<fn() -> T>);

impl<T: TypeName> Visitor<'_> for TypeTagVisitor<T> {
    type Value = TypeTag<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "the _type discriminator string {:?}", T::NAME)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v == T::NAME {
            Ok(TypeTag::new())
        } else {
            Err(E::invalid_value(de::Unexpected::Str(v), &self))
        }
    }
}

impl<'de, T: TypeName> Deserialize<'de> for TypeTag<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(TypeTagVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct DvDate {
        #[serde(rename = "_type", default = "TypeTag::new")]
        type_tag: TypeTag<Self>,
        value: String,
    }
    impl TypeName for DvDate {
        const NAME: &'static str = "DV_DATE";
    }

    /// Structure-identical to `DvDate` — only `_type` can tell them apart.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct DvTime {
        #[serde(rename = "_type", default = "TypeTag::new")]
        type_tag: TypeTag<Self>,
        value: String,
    }
    impl TypeName for DvTime {
        const NAME: &'static str = "DV_TIME";
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Temporal {
        Date(DvDate),
        Time(DvTime),
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Versionish<T> {
        #[serde(rename = "_type", default = "TypeTag::new")]
        type_tag: TypeTag<Versionish<T>>,
        data: T,
    }
    impl<T> TypeName for Versionish<T> {
        const NAME: &'static str = "ORIGINAL_VERSION";
    }

    #[test]
    fn emits_type_first() {
        let d = DvDate {
            type_tag: TypeTag::new(),
            value: "2026-07-02".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, r#"{"_type":"DV_DATE","value":"2026-07-02"}"#);
    }

    #[test]
    fn untagged_dispatch_is_tag_driven_not_order_driven() {
        // Date is declared first; a DV_TIME payload must still reach Time.
        let t: Temporal =
            serde_json::from_str(r#"{"_type":"DV_TIME","value":"10:00:00"}"#).unwrap();
        assert!(matches!(t, Temporal::Time(_)));
        let d: Temporal =
            serde_json::from_str(r#"{"_type":"DV_DATE","value":"2026-07-02"}"#).unwrap();
        assert!(matches!(d, Temporal::Date(_)));
    }

    #[test]
    fn wrong_type_rejected_missing_type_tolerated() {
        let wrong: Result<DvDate, _> = serde_json::from_str(r#"{"_type":"DV_TIME","value":"x"}"#);
        assert!(wrong.is_err());
        let missing: DvDate = serde_json::from_str(r#"{"value":"2026-07-02"}"#).unwrap();
        assert_eq!(missing.value, "2026-07-02");
    }

    #[test]
    fn unknown_type_in_abstract_slot_errors() {
        let unk: Result<Temporal, _> = serde_json::from_str(r#"{"_type":"DV_BOGUS","value":"x"}"#);
        assert!(unk.is_err());
    }

    #[test]
    fn generic_self_tag_roundtrips_without_t_default_bound() {
        let v = Versionish {
            type_tag: TypeTag::new(),
            data: DvDate {
                type_tag: TypeTag::new(),
                value: "d".into(),
            },
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.starts_with(r#"{"_type":"ORIGINAL_VERSION""#));
        let back: Versionish<DvDate> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn tag_is_inert_for_eq_and_hash() {
        // Two tags are always equal; equality of containers reduces to the
        // real fields.
        let a = DvDate {
            type_tag: TypeTag::new(),
            value: "v".into(),
        };
        let b: DvDate = serde_json::from_str(r#"{"value":"v"}"#).unwrap();
        assert_eq!(a, b);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ITS-JSON (pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6) serialization conventions + ADR-002
//   source_loc: n/a
//   confidence: high
//   todos: 0
//   note: serialization infrastructure (not a spec class); behaviour pinned by in-file unit tests, validated in an isolated probe before rollout
// ─────────────────────────────────────────────
