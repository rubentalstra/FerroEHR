//! Fixtures for rm.common classes.

use openehr_base::identification::archetype_id::ArchetypeId;
use openehr_base::identification::object_id::ObjectIdData;
use openehr_foundation::serde_support::TypeTag;
use openehr_rm::common::archetyped::archetyped::Archetyped;
use openehr_rm::common::archetyped::feeder_audit::FeederAudit;
use openehr_rm::common::archetyped::feeder_audit_details::FeederAuditDetails;
use openehr_rm::common::archetyped::link::Link;
use openehr_rm::common::change_control::contribution::Contribution;
use openehr_rm::common::change_control::imported_version::ImportedVersion;
use openehr_rm::common::change_control::original_version::OriginalVersion;
use openehr_rm::common::change_control::version::VersionData;
use openehr_rm::common::change_control::versioned_object::VersionedObject;
use openehr_rm::common::directory::folder::Folder;
use openehr_rm::common::generic::attestation::Attestation;
use openehr_rm::common::generic::participation::Participation;
use openehr_rm::common::generic::party_identified::{PartyIdentified, PartyIdentifiedData};
use openehr_rm::common::generic::party_proxy::PartyProxyData;
use openehr_rm::common::generic::party_related::PartyRelated;
use openehr_rm::common::generic::revision_history::RevisionHistory;
use openehr_rm::common::generic::revision_history_item::RevisionHistoryItem;
use openehr_rm::data_types::text::dv_text::DvText;

use super::helpers::{
    audit, audit_data, coded, hier, object_ref, object_version_id, party_self, text,
};
use super::{Vector, vector};

fn original_version(data: &str) -> OriginalVersion<DvText> {
    OriginalVersion {
        type_tag: TypeTag::new(),
        version: VersionData {
            contribution: object_ref(
                "local",
                "CONTRIBUTION",
                "b6704e19-1b6e-4b73-9baf-4c4a7bf6e9b2",
            ),
            signature: None,
            commit_audit: audit(),
        },
        uid: object_version_id("939cec48-d629-4a3f-89f1-28c573387680::ehrbase.example.org::1"),
        preceding_version_uid: None,
        other_input_version_uids: None,
        lifecycle_state: coded("complete", "openehr", "532"),
        attestations: None,
        data: Some(text(data)),
    }
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        vector(
            "ARCHETYPED",
            &Archetyped {
                type_tag: TypeTag::new(),
                archetype_id: ArchetypeId {
                    type_tag: TypeTag::new(),
                    object_id: ObjectIdData {
                        value: "openEHR-EHR-COMPOSITION.encounter.v1".to_string(),
                    },
                },
                template_id: None,
                rm_version: "1.1.0".to_string(),
            },
        ),
        vector(
            "LINK",
            &Link {
                type_tag: TypeTag::new(),
                meaning: text("problem"),
                r#type: text("issue"),
                target: openehr_rm::data_types::uri::dv_ehr_uri::DvEhrUri {
                    type_tag: TypeTag::new(),
                    uri: openehr_rm::data_types::uri::dv_uri::DvUriData {
                        value: "ehr://ehr.example.org/ehr1".to_string(),
                    },
                },
            },
        ),
        vector(
            "FEEDER_AUDIT_DETAILS",
            &FeederAuditDetails {
                type_tag: TypeTag::new(),
                system_id: "legacy.example.org".to_string(),
                location: None,
                subject: None,
                provider: None,
                time: None,
                version_id: None,
                other_details: None,
            },
        ),
        vector(
            "FEEDER_AUDIT",
            &FeederAudit {
                type_tag: TypeTag::new(),
                originating_system_item_ids: None,
                feeder_system_item_ids: None,
                original_content: None,
                originating_system_audit: FeederAuditDetails {
                    type_tag: TypeTag::new(),
                    system_id: "legacy.example.org".to_string(),
                    location: None,
                    subject: None,
                    provider: None,
                    time: None,
                    version_id: None,
                    other_details: None,
                },
                feeder_system_audit: None,
            },
        ),
        vector("PARTY_SELF", &party_self()),
        vector(
            "PARTY_IDENTIFIED",
            &PartyIdentified {
                type_tag: TypeTag::new(),
                data: PartyIdentifiedData {
                    party_proxy: PartyProxyData { external_ref: None },
                    name: Some("Dr. A. Jansen".to_string()),
                    identifiers: None,
                },
            },
        ),
        vector(
            "PARTY_RELATED",
            &PartyRelated {
                type_tag: TypeTag::new(),
                party_identified: PartyIdentifiedData {
                    party_proxy: PartyProxyData { external_ref: None },
                    name: Some("J. Jansen".to_string()),
                    identifiers: None,
                },
                relationship: coded("mother", "openehr", "10"),
            },
        ),
        vector(
            "PARTICIPATION",
            &Participation {
                type_tag: TypeTag::new(),
                function: text("nurse"),
                mode: None,
                performer: super::helpers::committer(),
                time: None,
            },
        ),
        vector("AUDIT_DETAILS", &audit()),
        vector(
            "ATTESTATION",
            &Attestation {
                type_tag: TypeTag::new(),
                audit_details: audit_data(),
                attested_view: None,
                proof: None,
                items: None,
                reason: text("signed"),
                is_pending: false,
            },
        ),
        vector(
            "REVISION_HISTORY_ITEM",
            &RevisionHistoryItem {
                type_tag: TypeTag::new(),
                version_id: object_version_id(
                    "939cec48-d629-4a3f-89f1-28c573387680::ehrbase.example.org::1",
                ),
                audits: vec![audit()],
            },
        ),
        vector(
            "REVISION_HISTORY",
            &RevisionHistory {
                type_tag: TypeTag::new(),
                items: vec![RevisionHistoryItem {
                    type_tag: TypeTag::new(),
                    version_id: object_version_id(
                        "939cec48-d629-4a3f-89f1-28c573387680::ehrbase.example.org::1",
                    ),
                    audits: vec![audit()],
                }],
            },
        ),
        vector("ORIGINAL_VERSION", &original_version("payload")),
        vector(
            "IMPORTED_VERSION",
            &ImportedVersion {
                type_tag: TypeTag::new(),
                version: VersionData {
                    contribution: object_ref(
                        "local",
                        "CONTRIBUTION",
                        "1f9f4a15-3b8b-4a26-9d63-c519db2a4a26",
                    ),
                    signature: None,
                    commit_audit: audit(),
                },
                item: Box::new(original_version("imported payload")),
            },
        ),
        vector(
            "CONTRIBUTION",
            &Contribution {
                type_tag: TypeTag::new(),
                uid: hier("b6704e19-1b6e-4b73-9baf-4c4a7bf6e9b2"),
                versions: vec![object_ref(
                    "local",
                    "VERSION",
                    "939cec48-d629-4a3f-89f1-28c573387680",
                )],
                audit: audit(),
            },
        ),
        vector(
            "VERSIONED_OBJECT",
            &VersionedObject::<DvText> {
                type_tag: TypeTag::new(),
                uid: hier("939cec48-d629-4a3f-89f1-28c573387680"),
                owner_id: object_ref("local", "EHR", "7d44b88c-4199-4bad-97dc-d78268e01398"),
                time_created: super::helpers::date_time("2026-07-02T10:00:00Z"),
                versions: vec![],
            },
        ),
        vector(
            "FOLDER",
            &Folder {
                type_tag: TypeTag::new(),
                locatable: super::helpers::locatable("root", "openEHR-EHR-FOLDER.generic.v1"),
                items: None,
                folders: None,
                details: None,
            },
        ),
    ]
}
