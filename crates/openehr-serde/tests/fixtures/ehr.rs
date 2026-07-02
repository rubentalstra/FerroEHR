//! Fixtures for rm.ehr and rm.integration classes.

use openehr_base::identification::locatable_ref::LocatableRef;
use openehr_base::identification::uid_based_id::UidBasedId;
use openehr_foundation::serde_support::TypeTag;
use openehr_rm::common::generic::party_proxy::PartyProxy;
use openehr_rm::data_structures::history::history::History;
use openehr_rm::data_structures::item_structure::data_structure::DataStructureData;
use openehr_rm::data_structures::item_structure::item_structure::ItemStructure;
use openehr_rm::ehr::action::Action;
use openehr_rm::ehr::activity::Activity;
use openehr_rm::ehr::admin_entry::AdminEntry;
use openehr_rm::ehr::care_entry::CareEntryData;
use openehr_rm::ehr::composition::Composition;
use openehr_rm::ehr::content_item::ContentItemData;
use openehr_rm::ehr::ehr::Ehr;
use openehr_rm::ehr::ehr_access::EhrAccess;
use openehr_rm::ehr::ehr_status::EhrStatus;
use openehr_rm::ehr::entry::EntryData;
use openehr_rm::ehr::evaluation::Evaluation;
use openehr_rm::ehr::event_context::EventContext;
use openehr_rm::ehr::instruction::Instruction;
use openehr_rm::ehr::instruction_details::InstructionDetails;
use openehr_rm::ehr::ism_transition::IsmTransition;
use openehr_rm::ehr::observation::Observation;
use openehr_rm::ehr::section::Section;
use openehr_rm::integration::generic_entry::GenericEntry;

use super::helpers::{
    code_phrase, coded, date_time, item_structure, item_tree, locatable, object_ref,
    object_version_id, party_self, text,
};
use super::{Vector, vector};

fn entry_data(name: &str, node: &str) -> EntryData {
    EntryData {
        content_item: ContentItemData {
            locatable: locatable(name, node),
        },
        language: code_phrase("ISO_639-1", "en"),
        encoding: code_phrase("IANA_character-sets", "UTF-8"),
        other_participations: None,
        workflow_id: None,
        subject: PartyProxy::PartySelf(party_self()),
        provider: None,
    }
}

fn care_entry_data(name: &str, node: &str) -> CareEntryData {
    CareEntryData {
        entry: entry_data(name, node),
        protocol: None,
        guideline_id: None,
    }
}

fn history(name: &str) -> History<ItemStructure> {
    History {
        type_tag: TypeTag::new(),
        data_structure: DataStructureData {
            locatable: locatable(name, "at0001"),
        },
        origin: date_time("2026-07-02T10:00:00Z"),
        period: None,
        duration: None,
        summary: None,
        events: None,
    }
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        vector(
            "EHR",
            &Ehr {
                type_tag: TypeTag::new(),
                system_id: super::helpers::hier("ehrbase.example.org"),
                ehr_id: super::helpers::hier("7d44b88c-4199-4bad-97dc-d78268e01398"),
                contributions: Some(vec![]),
                ehr_status: object_ref(
                    "local",
                    "VERSIONED_EHR_STATUS",
                    "d61338e1-0ea1-45a5-9bcf-04b71b41e3a9",
                ),
                ehr_access: object_ref(
                    "local",
                    "VERSIONED_EHR_ACCESS",
                    "59a8a3ae-e9ca-4c11-a270-fbcfd0d10012",
                ),
                compositions: Some(vec![]),
                directory: None,
                time_created: date_time("2026-07-02T09:00:00Z"),
                folders: None,
            },
        ),
        vector(
            "EHR_STATUS",
            &EhrStatus {
                type_tag: TypeTag::new(),
                locatable: locatable("EHR Status", "openEHR-EHR-EHR_STATUS.generic.v1"),
                subject: party_self(),
                is_queryable: true,
                is_modifiable: true,
                other_details: None,
            },
        ),
        vector(
            "EHR_ACCESS",
            &EhrAccess {
                type_tag: TypeTag::new(),
                locatable: locatable("EHR Access", "openEHR-EHR-EHR_ACCESS.generic.v1"),
                settings: None,
            },
        ),
        vector(
            "COMPOSITION",
            &Composition {
                type_tag: TypeTag::new(),
                locatable: locatable("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1"),
                language: code_phrase("ISO_639-1", "en"),
                territory: code_phrase("ISO_3166-1", "NL"),
                category: coded("event", "openehr", "433"),
                context: None,
                composer: PartyProxy::PartySelf(party_self()),
                content: None,
            },
        ),
        vector(
            "EVENT_CONTEXT",
            &EventContext {
                type_tag: TypeTag::new(),
                start_time: date_time("2026-07-02T09:30:00Z"),
                end_time: None,
                location: None,
                setting: coded("primary medical care", "openehr", "228"),
                other_context: None,
                health_care_facility: None,
                participations: None,
            },
        ),
        vector(
            "SECTION",
            &Section {
                type_tag: TypeTag::new(),
                content_item: ContentItemData {
                    locatable: locatable("Vital signs", "openEHR-EHR-SECTION.vital_signs.v1"),
                },
                items: None,
            },
        ),
        vector(
            "ADMIN_ENTRY",
            &AdminEntry {
                type_tag: TypeTag::new(),
                entry: entry_data("Admission", "openEHR-EHR-ADMIN_ENTRY.admission.v1"),
                data: item_structure("data", "at0001"),
            },
        ),
        vector(
            "OBSERVATION",
            &Observation {
                type_tag: TypeTag::new(),
                care_entry: care_entry_data(
                    "Blood pressure",
                    "openEHR-EHR-OBSERVATION.blood_pressure.v2",
                ),
                data: history("data"),
                state: None,
            },
        ),
        vector(
            "EVALUATION",
            &Evaluation {
                type_tag: TypeTag::new(),
                care_entry: care_entry_data(
                    "Problem",
                    "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
                ),
                data: item_structure("data", "at0001"),
            },
        ),
        vector(
            "INSTRUCTION",
            &Instruction {
                type_tag: TypeTag::new(),
                care_entry: care_entry_data(
                    "Medication order",
                    "openEHR-EHR-INSTRUCTION.medication_order.v3",
                ),
                narrative: text("Aspirin 100mg daily"),
                expiry_time: None,
                wf_definition: None,
                activities: None,
            },
        ),
        vector(
            "ACTIVITY",
            &Activity {
                type_tag: TypeTag::new(),
                locatable: locatable("Order", "at0001"),
                timing: None,
                action_archetype_id: "/.*/".to_string(),
                description: item_structure("description", "at0002"),
            },
        ),
        vector(
            "ACTION",
            &Action {
                type_tag: TypeTag::new(),
                care_entry: care_entry_data(
                    "Medication action",
                    "openEHR-EHR-ACTION.medication.v1",
                ),
                time: date_time("2026-07-02T10:15:00Z"),
                ism_transition: IsmTransition {
                    type_tag: TypeTag::new(),
                    current_state: coded("completed", "openehr", "532"),
                    transition: None,
                    careflow_step: None,
                    reason: None,
                },
                instruction_details: None,
                description: item_structure("description", "at0001"),
            },
        ),
        vector(
            "ISM_TRANSITION",
            &IsmTransition {
                type_tag: TypeTag::new(),
                current_state: coded("planned", "openehr", "526"),
                transition: None,
                careflow_step: None,
                reason: None,
            },
        ),
        vector(
            "INSTRUCTION_DETAILS",
            &InstructionDetails {
                type_tag: TypeTag::new(),
                instruction_id: LocatableRef {
                    type_tag: TypeTag::new(),
                    namespace: "local".to_string(),
                    r#type: "INSTRUCTION".to_string(),
                    id: UidBasedId::ObjectVersionId(object_version_id(
                        "939cec48-d629-4a3f-89f1-28c573387680::ehrbase.example.org::1",
                    )),
                    path: Some("/content[openEHR-EHR-INSTRUCTION.medication_order.v3]".to_string()),
                },
                activity_id: "activities[at0001]".to_string(),
                wf_details: None,
            },
        ),
        vector(
            "GENERIC_ENTRY",
            &GenericEntry {
                type_tag: TypeTag::new(),
                locatable: locatable("Imported", "openEHR-EHR-GENERIC_ENTRY.generic.v1"),
                data: item_tree("data", "at0001"),
            },
        ),
    ]
}
