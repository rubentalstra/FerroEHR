//! `VERSION_STATUS` — status of a versioned artefact.
//!
//! openEHR class: `VERSION_STATUS` (enumeration), package
//! `base.base_types.definitions`.
//!
//! Status of a versioned artefact, as one of a number of possible values:
//! uncontrolled, prerelease, release, build. Each value corresponds to a
//! specific rendering of a semantic-versioning-style `N.M.P` string, as
//! documented per variant below.
use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Closed five-value enumeration, transcribed directly as a Rust `enum`.
/// The spec's exact lower-case/snake_case symbol names are preserved by
/// [`VersionStatus::symbol`] and by the canonical JSON `value` field.
///
/// P4 update: as with `ValidityKind` in this same package, the pinned
/// ITS-JSON schema exposes an object definition for this enumeration, so
/// serde emits `{_type: "VERSION_STATUS", value: <symbol>}` and accepts
/// the older bare symbol string for compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionStatus {
    /// `alpha` — a version which is 'unstable', i.e. contains an unknown
    /// size of change with respect to its base version. Rendered with the
    /// build number as a string in the form `N.M.P-alpha.B`, e.g.
    /// `2.0.1-alpha.154`.
    Alpha,

    /// `beta` — a version which is 'beta', i.e. contains an unknown but
    /// reducing size of change with respect to its base version. Rendered
    /// with the build number as a string in the form `N.M.P-beta.B`, e.g.
    /// `2.0.1-beta.154`.
    Beta,

    /// `release_candidate` — a version which is 'release candidate', i.e.
    /// contains only patch-level changes on the base version. Rendered as a
    /// string as `N.M.P-rc.B`, e.g. `2.0.1-rc.27`.
    ReleaseCandidate,

    /// `released` — a version which is 'released', i.e. is the definitive
    /// base version. Rendered as `N.M.P`, e.g. `2.0.1`.
    Released,

    /// `build` — a version which is a build of the current base release.
    /// Rendered with the build number as a string in the form `N.M.P+B`,
    /// e.g. `2.0.1+33`.
    Build,
}

impl VersionStatus {
    /// The spec's own snake_case symbol name for this enumeration value.
    pub const fn symbol(self) -> &'static str {
        match self {
            VersionStatus::Alpha => "alpha",
            VersionStatus::Beta => "beta",
            VersionStatus::ReleaseCandidate => "release_candidate",
            VersionStatus::Released => "released",
            VersionStatus::Build => "build",
        }
    }

    fn from_symbol(value: &str) -> Option<Self> {
        match value {
            "alpha" => Some(VersionStatus::Alpha),
            "beta" => Some(VersionStatus::Beta),
            "release_candidate" => Some(VersionStatus::ReleaseCandidate),
            "released" => Some(VersionStatus::Released),
            "build" => Some(VersionStatus::Build),
            _ => None,
        }
    }
}

impl Serialize for VersionStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VERSION_STATUS", 2)?;
        state.serialize_field("_type", "VERSION_STATUS")?;
        state.serialize_field("value", self.symbol())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for VersionStatus {
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
            .is_some_and(|name| name != "VERSION_STATUS")
        {
            return Err(D::Error::custom("expected _type \"VERSION_STATUS\""));
        }
        VersionStatus::from_symbol(&value)
            .ok_or_else(|| D::Error::custom(format!("unknown VERSION_STATUS value {value:?}")))
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/version_status.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-definitions_package.adoc §Class Definitions / version_status.adoc §VERSION_STATUS Enumeration
//   confidence: high
//   todos: 0
//   note: closed 5-value enum with a symbol() method carrying the spec's own snake_case name; render-format strings (N.M.P-alpha.B etc.) are documentation only, not implemented as a formatting method since the spec table does not define one as a class function. P4 — canonical JSON emits object form `{_type:"VERSION_STATUS",value}` to satisfy the pinned ITS-JSON schema while preserving the enum symbol.
// ─────────────────────────────────────────────
