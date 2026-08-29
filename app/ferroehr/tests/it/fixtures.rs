// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The shared SM commit-fixture family the topic modules import.
//!
//! Every suite that drives a write through the service layer needs the same
//! pieces: a `PARTY_IDENTIFIED` committer, an `openehr` change-type
//! `DV_CODED_TEXT`, a minimal valid RM COMPOSITION, and the SM
//! `UPDATE_VERSION` envelope carrying them (SM
//! `UML/classes/update_version.adoc`). They live here once, so the per-suite
//! copies cannot drift apart.
//!
//! A suite whose fixture identity is load-bearing (an assertion reads the
//! composer or committer name) takes the parameterized builder rather than a
//! private copy.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this module's \
              fixture helpers; a failing fixture must panic at the fixture (the \
              Rust Book ch11)"
)]

use serde_json::{Value, json};

use ferroehr::service::version_update::{change_type_coded, lifecycle_state_coded};
use openehr_its::rest::generated::common::{UpdateAudit, UpdateAuditData, UpdateVersion};
use openehr_rm::prelude::PartyProxy;

/// The identity the shared fixtures write as — both the COMPOSITION `composer`
/// and the commit `committer`.
const TESTER: &str = "conformance tester";

/// Returns a `PARTY_IDENTIFIED` committer named `name`, as canonical JSON.
pub(crate) fn committer(name: &str) -> Value {
    json!({ "_type": "PARTY_IDENTIFIED", "name": name })
}

/// Returns a `PARTY_IDENTIFIED` committer named `name`, as the typed RM value.
pub(crate) fn committer_proxy(name: &str) -> PartyProxy {
    openehr_its::json::from_canonical_value(&committer(name)).expect("committer")
}

/// Returns the wire `DV_CODED_TEXT` naming `openehr` terminology `code` with
/// rubric `value`.
///
/// The `audit_change_type` group codes this carries are listed in RM common
/// `master06-change_control_package.adoc` §Contributions.
pub(crate) fn change_type(code: &str, value: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT", "value": value,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
            "code_string": code
        }
    })
}

/// Returns a minimal *valid* RM COMPOSITION named `name`, composed by
/// `composer`.
///
/// `language`, `territory`, `category` and `composer` are all `1..1` (RM ehr,
/// COMPOSITION class), so typed RM validation rejects a fixture without them.
/// No template is referenced, so the fixture needs no `template_store` row.
pub(crate) fn composition_by(name: &str, composer: &str) -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": {
                "_type": "ARCHETYPE_ID",
                "value": "openEHR-EHR-COMPOSITION.encounter.v1"
            },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": name },
        "language": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" },
            "code_string": "en"
        },
        "territory": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" },
            "code_string": "NL"
        },
        "category": {
            "_type": "DV_CODED_TEXT",
            "value": "event",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "433"
            }
        },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": composer }
    })
}

/// Returns a minimal *valid* RM COMPOSITION named `name`.
pub(crate) fn composition(name: &str) -> Value {
    composition_by(name, TESTER)
}

/// Returns the SM `UPDATE_VERSION` commit envelope for a bare-RM write.
///
/// `change_code` is the `openehr` `audit_change_type` code the version's audit
/// records (`249` creation, `251` modification, `523` deleted); `preceding`
/// names the version this one supersedes (RM common
/// `master06-change_control_package.adoc` §Contributions).
pub(crate) fn uv<T: serde::de::DeserializeOwned>(
    data: &Value,
    change_code: &str,
    preceding: Option<&str>,
) -> UpdateVersion<T> {
    UpdateVersion {
        preceding_version_uid: preceding.map(|p| p.parse().expect("OBJECT_VERSION_ID")),
        lifecycle_state: lifecycle_state_coded("532"),
        attestations: None,
        data: openehr_its::json::from_canonical_value(data)
            .expect("the fixture commit body decodes as its RM type"),
        commit_audit: UpdateAudit::UpdateAudit(UpdateAuditData {
            _type: None,
            system_id: None,
            change_type: change_type_coded(change_code),
            description: None,
            committer: committer_proxy(TESTER),
        }),
        signature: None,
    }
}

/// The committed version's `uid/value` string off a served body.
pub(crate) fn uid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
}

/// The bare versioned-object UUID of an `OBJECT_VERSION_ID` — everything
/// before the first separator (`object_version_id = object_id, '::',
/// creating_system_id, '::', version_tree_id`; BASE
/// `base_types/master05-identification_package.adoc` §Syntaxes).
pub(crate) fn vo_of(ovid: &str) -> &str {
    ovid.split("::").next().expect("vo uuid")
}

/// A minimal valid root FOLDER (RM ehr master04 §Folders).
pub(crate) fn folder(name: &str) -> Value {
    json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": name }
    })
}
