//! `EHR_ACCESS` — EHR-wide access control object.
//!
//! openEHR class: `EHR_ACCESS`, package `rm.ehr`.
//! Inherits: `LOCATABLE`.
//!
//! EHR-wide access control object. All access decisions to data in the EHR
//! must be made in accordance with the policies and rules in this object.
//!
//! NOTE (spec): it is strongly recommended that the inherited attribute
//! `_uid_` be populated in `EHR_ACCESS` objects, using the UID copied from
//! the `object_id()` of the `_uid_` field of the enclosing `VERSION` object.
//! For example, the `ORIGINAL_VERSION.uid`
//! `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2` would be copied to
//! the `_uid_` field of the `EHR_ACCESS` object.
//!
//! Ground truth: `docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr_access.adoc`
//! (RM Release-1.1.0 @ 3cbd85b).

// TODO(port): forward-reference — `common` package (rm.common), not yet
// transcribed. `LocatableData` is the ADR-001 §3 embedded-struct half of the
// abstract `LOCATABLE` class.
//
// TODO(port): P4 — `#[serde(flatten)]` on `EhrAccess::locatable` below
// requires `LocatableData` to itself derive `Serialize`/`Deserialize`; that
// derive is a sibling P4 wave over `common/`, not yet landed.
use crate::common::archetyped::locatable::LocatableData;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized form.
///
/// Single-sourced into the `TypeName` impl below (ADR-002).
pub const TYPE_NAME: &str = "EHR_ACCESS";

/// `ACCESS_CONTROL_SETTINGS` — abstract parent of the concrete access
/// control scheme classes referenced by `EHR_ACCESS.settings`.
///
/// **Spec ambiguity, flagged rather than guessed at:** the published RM
/// 1.1.0 class table for `ACCESS_CONTROL_SETTINGS`
/// (`docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/access_control_settings.adoc`)
/// declares the class abstract with **zero** attributes, **zero**
/// functions, and **zero** invariants — only a one-line description
/// ("Access Control Settings for the EHR and components. Intended to
/// support multiple access control schemes. Currently implementation
/// dependent."). The `EHR_STATUS`/`EHR_ACCESS` chapter narrative
/// (`master04-ehr_package.adoc` §EHR Access) further states that "[e]ach
/// scheme is defined by an instance of a subclass of the abstract class
/// `ACCESS_CONTROL_SETTINGS`, defined in the {openEHR RM} Security
/// Information Model" — a chapter/model that is not part of the RM 1.1.0
/// `ehr` package and is not cached or transcribed by this pass. This
/// mirrors the known-hazard list's "`ACCESS_GROUP_REF` was not migrated to
/// BASE 1.2.0" precedent: the type is declared and referenced, but its
/// substantive content lives in a security model this crate does not (yet)
/// have ground truth for.
///
/// Transcribed here as a genuinely empty marker struct — the literal
/// content of the published table, no more — rather than inventing fields
/// or a concrete subtype enum the spec does not itself provide. Per
/// ADR-001 §4, if/when the Security Information Model is transcribed and
/// yields a closed subtype set, this should become the `XxxData` struct
/// half of an `XxxData`/`Xxx`-enum/`XxxApi`-trait cluster (ADR-001
/// Refinements) rather than staying a bare marker.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AccessControlSettings {
    // Deliberately empty: the published class table declares no attribute,
    // function, or invariant of its own. See the struct doc comment above.
}

/// `EHR_ACCESS` — EHR-wide access control object.
///
/// Per ADR-001 §3, `LOCATABLE`'s state is embedded as
/// `pub locatable: LocatableData` rather than simulated via a Rust
/// supertrait. `#[serde(flatten)]` folds those six attributes into
/// `EHR_ACCESS`'s own JSON object per the ITS-JSON abstract-class-flattening
/// rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EhrAccess {
    /// Canonical `_type` discriminator (`"EHR_ACCESS"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `LOCATABLE` state.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `settings`: access control settings for the EHR. Instance is a
    /// subtype of the type `ACCESS_CONTROL_SETTINGS`, allowing for the use
    /// of different access control schemes.
    ///
    /// Cardinality: `0..1`.
    ///
    /// TODO(port): the spec's declared type here is the abstract
    /// `ACCESS_CONTROL_SETTINGS`, used polymorphically ("Instance is a
    /// subtype of..."). Since the Security Information Model that would
    /// supply the closed subtype set is out of scope for this pass (see the
    /// `AccessControlSettings` doc comment), this field is typed against
    /// the bare marker struct rather than a proper enum of concrete
    /// schemes. Revisit once/if that model is transcribed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<AccessControlSettings>,
}

impl TypeName for EhrAccess {
    const NAME: &'static str = TYPE_NAME;
}

impl EhrAccess {
    /// Function `scheme` (): `String`.
    ///
    /// The name of the access control scheme in use; corresponds to the
    /// concrete instance of the `settings` attribute.
    ///
    /// Cardinality: `1..1`.
    ///
    /// TODO(port): cannot be derived from a bare `AccessControlSettings`
    /// marker (see its doc comment) — needs the concrete scheme subtype's
    /// own name once the Security Information Model is transcribed.
    pub fn scheme(&self) -> String {
        todo!(
            "port: scheme name is only knowable once ACCESS_CONTROL_SETTINGS has concrete subtypes (Security Information Model, out of scope for this pass)"
        )
    }

    /// Invariant `Scheme_valid`: `not scheme.is_empty`.
    ///
    /// TODO(port): depends on `scheme()`, above.
    pub fn invariant_scheme_valid(&self) -> bool {
        todo!("port: depends on scheme(), see above")
    }

    /// Invariant `Is_archetype_root`: `is_archetype_root`.
    ///
    /// Inherited unchanged from `LOCATABLE`. Implemented per ADR-003 §8 as
    /// the derived value `LOCATABLE.is_archetype_root` computes —
    /// `archetype_details /= Void` (see [`LocatableData`] /
    /// `LocatableApi::is_archetype_root`).
    #[must_use]
    pub fn invariant_is_archetype_root(&self) -> bool {
        self.locatable.archetype_details.is_some()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr_access.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/ehr_access.adoc §EHR_ACCESS Class; access_control_settings.adoc §ACCESS_CONTROL_SETTINGS Class
//   confidence: medium
//   todos: 5
//   note: ACCESS_CONTROL_SETTINGS is a genuinely near-empty spec class whose concrete subtypes live in an out-of-scope Security Information Model — flagged, not invented; scheme() stubbed until that model exists. P4/ADR-002: EhrAccess self-tags (TypeTag<Self> first field + TypeName impl; no-op struct-level rename removed); AccessControlSettings stays untagged (abstract class — the schema defines no entry for it).
// ─────────────────────────────────────────────
