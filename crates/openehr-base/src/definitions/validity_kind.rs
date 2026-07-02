//! `VALIDITY_KIND` — presence/absence constraint enumeration.
//!
//! openEHR class: `VALIDITY_KIND` (enumeration), package
//! `base.base_types.definitions`.
//!
//! An enumeration of three values that may commonly occur in constraint
//! models. Used as the type of any attribute within a reference model that
//! expresses a constraint on some attribute in a class in that reference
//! model — for example, to indicate the validity of Date/Time fields.
use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Closed three-value enumeration, transcribed directly as a Rust `enum`.
/// The spec's exact lower-case symbol names are preserved by
/// [`ValidityKind::symbol`] and by the canonical JSON `value` field.
///
/// P4 update: the pinned ITS-JSON schema exposes an object definition for
/// this enumeration, so serde emits
/// `{_type: "VALIDITY_KIND", value: <symbol>}` and accepts the older bare
/// symbol string for compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidityKind {
    /// `mandatory` — constant to indicate mandatory presence of something.
    Mandatory,

    /// `optional` — constant to indicate optional presence of something.
    Optional,

    /// `prohibited` — constant to indicate disallowed presence of something.
    Prohibited,
}

impl ValidityKind {
    /// The spec's own lower-case symbol name for this enumeration value.
    pub const fn symbol(self) -> &'static str {
        match self {
            ValidityKind::Mandatory => "mandatory",
            ValidityKind::Optional => "optional",
            ValidityKind::Prohibited => "prohibited",
        }
    }

    fn from_symbol(value: &str) -> Option<Self> {
        match value {
            "mandatory" => Some(ValidityKind::Mandatory),
            "optional" => Some(ValidityKind::Optional),
            "prohibited" => Some(ValidityKind::Prohibited),
            _ => None,
        }
    }
}

impl Serialize for ValidityKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VALIDITY_KIND", 2)?;
        state.serialize_field("_type", "VALIDITY_KIND")?;
        state.serialize_field("value", self.symbol())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ValidityKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Object {
                #[serde(rename = "_type")]
                type_name: Option<String>,
                value: String,
            },
            Bare(String),
        }

        let (type_name, value) = match Wire::deserialize(deserializer)? {
            Wire::Object { type_name, value } => (type_name, value),
            Wire::Bare(value) => (None, value),
        };
        if type_name
            .as_deref()
            .is_some_and(|name| name != "VALIDITY_KIND")
        {
            return Err(D::Error::custom("expected _type \"VALIDITY_KIND\""));
        }
        ValidityKind::from_symbol(&value)
            .ok_or_else(|| D::Error::custom(format!("unknown VALIDITY_KIND value {value:?}")))
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/validity_kind.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-definitions_package.adoc §Class Definitions / validity_kind.adoc §VALIDITY_KIND Enumeration
//   confidence: high
//   todos: 0
//   note: closed 3-value enum with a symbol() method carrying the spec's own lower-case name; P4 — canonical JSON emits object form `{_type:"VALIDITY_KIND",value}` to satisfy the pinned ITS-JSON schema while preserving the enum symbol.
// ─────────────────────────────────────────────
