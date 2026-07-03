//! `VERSIONED_COMPOSITION` — version-controlled composition abstraction.
//!
//! openEHR class: `VERSIONED_COMPOSITION`, package `rm.ehr`.
//! Inherits: `VERSIONED_OBJECT<T>` (bound to `T = COMPOSITION`).
//!
//! Version-controlled composition abstraction, defined by inheriting
//! `VERSIONED_OBJECT<COMPOSITION>`. Unlike the other `VERSIONED_*` bindings
//! declared by this chapter (`VERSIONED_EHR_ACCESS`, `VERSIONED_EHR_STATUS`),
//! this class is not a bare binding — the published table adds one function
//! (`is_persistent()`) and two invariants of its own.
//!
//! Ground truth: `docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_composition.adoc`
//! (RM Release-1.1.0 @ 3cbd85b).

use crate::common::change_control::version::VersionApi;
use crate::common::change_control::versioned_object::VersionedObject;

// TODO(port): forward-reference — `COMPOSITION` lives in rm.ehr.composition
// (PORT_MASTER_PLAN.md §7.1: "EHR (20): EHR, EHR_STATUS, EHR_ACCESS,
// COMPOSITION, ..."). A sibling transcription pass owns the
// composition/content/entry classes in this same `crates/openehr-rm/src/ehr/`
// directory; this file forward-references `Composition` rather than
// defining it.
use super::composition::Composition;
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class per the spec's
/// class naming.
///
/// PORT NOTE (ADR-002, resolved): `VERSIONED_X` binding classes never emit
/// their own `_type`. The pinned ITS-JSON schema (commit
/// `5acae056248e917a4b4c56f7e712f4fcfeb616a6`,
/// `openehr_rm_1.1.0_all.json`) defines only `VERSIONED_OBJECT` — which
/// self-tags via its own `TypeTag` (sibling `common.change_control` wave) —
/// and has no `VERSIONED_COMPOSITION`/`VERSIONED_EHR_*` entries at all. So
/// this newtype keeps `#[serde(transparent)]`, gets no `TypeName`/`TypeTag`
/// of its own, and this const exists only as the spec class name for
/// non-serde callers (e.g. `OBJECT_REF.type` comparisons).
pub const TYPE_NAME: &str = "VERSIONED_COMPOSITION";

/// `VERSIONED_COMPOSITION` — `VERSIONED_OBJECT<COMPOSITION>` plus its own
/// `is_persistent()` function and two invariants.
///
/// See `versioned_ehr_access::VersionedEhrAccess` for the rationale behind
/// the newtype-wrapper (rather than bare type-alias) shape used for
/// `VERSIONED_OBJECT<T>` bindings in general; this class additionally
/// carries real behaviour of its own (below), reinforcing that the
/// newtype-with-inherent-impl shape (not a type alias, which could not
/// carry inherent methods distinct from `VersionedObject<Composition>`'s
/// own) is the right one here.
///
/// PORT NOTE: `#[serde(transparent)]` makes this newtype serialize/
/// deserialize identically to its single field,
/// `VersionedObject<Composition>` — whose own `TypeTag` emits
/// `_type: "VERSIONED_OBJECT"`. Per ADR-002 this binding never emits a
/// `_type: "VERSIONED_COMPOSITION"` of its own: the pinned ITS-JSON schema
/// defines no `VERSIONED_X` entries, only `VERSIONED_OBJECT` (see the
/// `TYPE_NAME` doc comment). Resolved — not a deferred P17 question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VersionedComposition(pub VersionedObject<Composition>);

impl VersionedComposition {
    /// Function `is_persistent` (): `Boolean`.
    ///
    /// Indicates whether this composition set is persistent; derived from
    /// the first version's `COMPOSITION.is_persistent`.
    ///
    /// Cardinality: `1..1`.
    ///
    /// PORT NOTE: `False` when the container has no versions, or the first
    /// version carries no `data` (an `IMPORTED_VERSION` whose wrapped data is
    /// absent), mirroring `Composition::is_persistent`'s "False otherwise".
    #[must_use]
    pub fn is_persistent(&self) -> bool {
        self.0
            .all_versions()
            .first()
            .and_then(|version| version.data())
            .is_some_and(Composition::is_persistent)
    }

    /// Invariant `Archetype_node_id_valid`:
    /// `for_all v in all_versions | v.archetype_node_id.is_equal(all_versions.first.archetype_node_id)`.
    ///
    /// All versions of the same `VERSIONED_COMPOSITION` share one
    /// archetype node id — the identity of the composition's archetype does
    /// not change across its version history (ADR-003 §8).
    ///
    /// PORT NOTE: `VERSION<T>` has no `archetype_node_id` attribute of its
    /// own, so this reads it from the version's `data`
    /// (`COMPOSITION.archetype_node_id`, inherited from `LOCATABLE`); the
    /// reference is `all_versions.first`'s data, and versions with absent
    /// `data` are skipped (nothing to compare). An empty container, or a
    /// first version without data, is vacuously valid.
    #[must_use]
    pub fn invariant_archetype_node_id_valid(&self) -> bool {
        let versions = self.0.all_versions();
        let Some(reference) = versions
            .first()
            .and_then(|version| version.data())
            .map(|composition| composition.locatable.archetype_node_id.as_str())
        else {
            return true;
        };
        versions
            .iter()
            .filter_map(|version| version.data())
            .all(|composition| composition.locatable.archetype_node_id == reference)
    }

    /// Invariant `Persistent_validity`:
    /// `for_all v in all_versions | v.is_persistent = all_versions.first.data.is_persistent`.
    ///
    /// PORT NOTE: as with [`Self::invariant_archetype_node_id_valid`],
    /// `v.is_persistent` is read from each version's `data`
    /// (`COMPOSITION.is_persistent`); versions with absent `data` are
    /// skipped, and an empty/dataless-first container is vacuously valid.
    #[must_use]
    pub fn invariant_persistent_validity(&self) -> bool {
        let versions = self.0.all_versions();
        let Some(reference) = versions
            .first()
            .and_then(|version| version.data())
            .map(Composition::is_persistent)
        else {
            return true;
        };
        versions
            .iter()
            .filter_map(|version| version.data())
            .all(|composition| composition.is_persistent() == reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::common::change_control::original_version::OriginalVersion;
    use crate::common::change_control::version::{Version, VersionData};
    use crate::common::change_control::versioned_object::test_support;
    use crate::common::generic::party_proxy::{PartyProxy, PartyProxyData};
    use crate::common::generic::party_self::PartySelf;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;
    use openehr_foundation::serde_support::TypeTag;

    fn code_phrase(terminology: &str, code: &str) -> CodePhrase {
        CodePhrase {
            type_tag: TypeTag::new(),
            terminology_id: TerminologyId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: terminology.to_string(),
                },
            },
            code_string: code.to_string(),
            preferred_term: None,
        }
    }

    fn composition(archetype_node_id: &str, category_code: &str) -> Composition {
        Composition {
            type_tag: TypeTag::new(),
            locatable: LocatableData {
                name: DvText::Text {
                    type_tag: TypeTag::new(),
                    data: DvTextData {
                        value: "Encounter".to_string(),
                        hyperlink: None,
                        formatting: None,
                        mappings: None,
                        language: None,
                        encoding: None,
                    },
                },
                archetype_node_id: archetype_node_id.to_string(),
                uid: None,
                links: None,
                archetype_details: None,
                feeder_audit: None,
                parent: None,
            },
            language: code_phrase("ISO_639-1", "en"),
            territory: code_phrase("ISO_3166-1", "NL"),
            category: DvCodedText {
                type_tag: TypeTag::new(),
                text: DvTextData {
                    value: "category".to_string(),
                    hyperlink: None,
                    formatting: None,
                    mappings: None,
                    language: None,
                    encoding: None,
                },
                defining_code: code_phrase("openehr", category_code),
            },
            context: None,
            composer: PartyProxy::PartySelf(PartySelf {
                type_tag: TypeTag::new(),
                party_proxy: PartyProxyData { external_ref: None },
            }),
            content: None,
        }
    }

    fn original(uid: &str, comp: Composition) -> Version<Composition> {
        Version::Original(OriginalVersion {
            type_tag: TypeTag::new(),
            version: VersionData {
                contribution: test_support::object_ref(
                    "CONTRIBUTION",
                    "b0e6c17c-7b78-4de1-a1a4-fd2988c8d3a1",
                ),
                signature: None,
                commit_audit: test_support::audit("2020-01-01T00:00:01"),
            },
            uid: test_support::ovid(uid),
            preceding_version_uid: None,
            other_input_version_uids: None,
            lifecycle_state: test_support::coded("532", "complete"),
            attestations: None,
            data: Some(comp),
        })
    }

    fn versioned(versions: Vec<Version<Composition>>) -> VersionedComposition {
        VersionedComposition(VersionedObject {
            type_tag: TypeTag::new(),
            uid: test_support::hier("87284370-2d4b-4e3d-a3f3-f303d2f4f34b"),
            owner_id: test_support::object_ref("EHR", "b5a56f4c-4574-4759-9bd5-b09be2f0e532"),
            time_created: test_support::date_time("2020-01-01T00:00:00"),
            versions,
        })
    }

    #[test]
    fn is_persistent_derives_from_the_first_version() {
        let persistent = versioned(vec![original(
            "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::sys::1",
            composition("openEHR-EHR-COMPOSITION.encounter.v1", "431"),
        )]);
        assert!(persistent.is_persistent());

        let event = versioned(vec![original(
            "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::sys::1",
            composition("openEHR-EHR-COMPOSITION.encounter.v1", "433"),
        )]);
        assert!(!event.is_persistent());

        // Empty container: not persistent.
        assert!(!versioned(vec![]).is_persistent());
    }

    #[test]
    fn archetype_node_id_and_persistent_validity_hold_across_versions() {
        let node = "openEHR-EHR-COMPOSITION.encounter.v1";
        let consistent = versioned(vec![
            original(
                "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::sys::1",
                composition(node, "431"),
            ),
            original(
                "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::sys::2",
                composition(node, "431"),
            ),
        ]);
        assert!(consistent.invariant_archetype_node_id_valid());
        assert!(consistent.invariant_persistent_validity());

        // A version with a different archetype node id / persistence breaks
        // the invariants.
        let divergent = versioned(vec![
            original(
                "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::sys::1",
                composition(node, "431"),
            ),
            original(
                "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::sys::2",
                composition("openEHR-EHR-COMPOSITION.report.v1", "433"),
            ),
        ]);
        assert!(!divergent.invariant_archetype_node_id_valid());
        assert!(!divergent.invariant_persistent_validity());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_composition.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/versioned_composition.adoc §VERSIONED_COMPOSITION Class
//   confidence: high
//   todos: 1
//   note: newtype over VersionedObject<Composition>. P5/ADR-003 §8: is_persistent() (from first version's data.is_persistent), Archetype_node_id_valid and Persistent_validity all implemented over all_versions()/VersionApi::data(); PORT NOTE flags that VERSION carries no archetype_node_id/is_persistent of its own so both are read from the version's Composition data (versions with absent data skipped). Pinned by unit tests. Sole remaining TODO(port) is the COMPOSITION forward-ref import comment. P4/ADR-002 resolved: #[serde(transparent)], never emits its own _type (schema defines only VERSIONED_OBJECT).
// ─────────────────────────────────────────────
