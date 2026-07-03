//! SYNTHETIC gap-filler fixtures — the ONLY hand-built test data in the
//! `openehr-serde` acceptance suite.
//!
//! Every other class is exercised against REAL canonical JSON: the vendored
//! `ehrbase/openEHR_SDK` corpus plus four in-repo EHRbase resources (see
//! `real_world_round_trip.rs` and `class_coverage.rs`). Fifteen ITS-JSON RM
//! classes have NO real-world canonical-JSON oracle, for two reasons:
//!
//!   * the whole `rm.demographic` package (`PERSON`, `ORGANISATION`, `ROLE`,
//!     `AGENT`, `GROUP`, `CONTACT`, `ADDRESS`, `CAPABILITY`, `PARTY_IDENTITY`,
//!     `PARTY_RELATIONSHIP`) — openEHR deployments keep demographics in a
//!     separate repository, and archie / specifications-RM / `openEHR_SDK` ship
//!     no demographic canonical-JSON corpus; and
//!   * a handful of rare data-value types absent from the composition corpus
//!     (`DV_STATE`, `DV_PARAGRAPH`, `DV_SCALE`,
//!     `DV_PERIODIC_TIME_SPECIFICATION`, `DV_GENERAL_TIME_SPECIFICATION`).
//!
//! These fifteen (and only these — they are exactly the `★`-marked entries in
//! `class_coverage.rs`'s `DOCUMENTED_UNCOVERED`) are covered here with MINIMAL
//! synthetic instances. Each fixture carries only what the ITS-JSON schema
//! `required` set and our own non-`Option` fields demand — no padding.
//!
//! Each fixture runs the same oracle the real-world suite uses, minus the
//! byte-compare against a source file (there is no source file):
//!
//!   1. build the minimal Rust instance (mirroring each class's own
//!      `#[cfg(test)]` constructors — the authoritative, non-stale APIs);
//!   2. serialize to a [`serde_json::Value`];
//!   3. schema-validate that value against the class's ITS-JSON definition,
//!      via the shared `corpus::schema_errors` helper (draft-07, pinned
//!      commit `5acae056248e917a4b4c56f7e712f4fcfeb616a6`); and
//!   4. deserialize it back and assert `serialize → deserialize → equal`.
//!
//! `synthetic_gap_fixtures_round_trip_and_validate` is data-driven: it runs
//! all fifteen and reports every failure at once (class name + failure), like
//! `real_world_corpus_round_trips` does. The instances are small, so no
//! large-stack worker thread is needed.

mod corpus;

use serde::Serialize;
use serde::de::DeserializeOwned;

use openehr_base::identification::hier_object_id::HierObjectId;
use openehr_base::identification::object_id::{ObjectId, ObjectIdData};
use openehr_base::identification::party_ref::PartyRef;
use openehr_base::identification::terminology_id::TerminologyId;
use openehr_base::identification::uid_based_id::{UidBasedId, UidBasedIdData};
use openehr_foundation::primitive_types::real::Real;
use openehr_foundation::serde_support::TypeTag;

use openehr_rm::common::archetyped::locatable::LocatableData;
use openehr_rm::data_structures::item_structure::ItemStructure;
use openehr_rm::data_structures::item_structure::data_structure::DataStructureData;
use openehr_rm::data_structures::item_structure::item_structure::ItemStructureData;
use openehr_rm::data_structures::item_structure::item_tree::ItemTree;
use openehr_rm::data_types::basic::dv_state::DvState;
use openehr_rm::data_types::encapsulated::dv_encapsulated::DvEncapsulatedData;
use openehr_rm::data_types::encapsulated::dv_parsable::DvParsable;
use openehr_rm::data_types::quantity::dv_ordered::DvOrderedData;
use openehr_rm::data_types::quantity::dv_scale::DvScale;
use openehr_rm::data_types::text::code_phrase::CodePhrase;
use openehr_rm::data_types::text::dv_coded_text::DvCodedText;
use openehr_rm::data_types::text::dv_paragraph::DvParagraph;
use openehr_rm::data_types::text::dv_text::{DvText, DvTextData};
use openehr_rm::data_types::time_specification::dv_general_time_specification::DvGeneralTimeSpecification;
use openehr_rm::data_types::time_specification::dv_periodic_time_specification::DvPeriodicTimeSpecification;
use openehr_rm::demographic::actor::ActorData;
use openehr_rm::demographic::address::Address;
use openehr_rm::demographic::agent::Agent;
use openehr_rm::demographic::capability::Capability;
use openehr_rm::demographic::contact::Contact;
use openehr_rm::demographic::group::Group;
use openehr_rm::demographic::organisation::Organisation;
use openehr_rm::demographic::party::PartyData;
use openehr_rm::demographic::party_identity::PartyIdentity;
use openehr_rm::demographic::party_relationship::PartyRelationship;
use openehr_rm::demographic::person::Person;
use openehr_rm::demographic::role::Role;

// ── Minimal builders (mirroring each class's own #[cfg(test)] constructors) ──

/// Bare `DV_TEXT` carrying only the schema-required `value`.
fn dv_text(value: &str) -> DvText {
    DvText::Text {
        type_tag: TypeTag::new(),
        data: DvTextData {
            value: value.to_string(),
            hyperlink: None,
            formatting: None,
            mappings: None,
            language: None,
            encoding: None,
        },
    }
}

/// `TERMINOLOGY_ID` with only its `value`.
fn terminology_id(value: &str) -> TerminologyId {
    TerminologyId {
        type_tag: TypeTag::new(),
        object_id: ObjectIdData {
            value: value.to_string(),
        },
    }
}

/// `CODE_PHRASE` (schema-required `terminology_id` + `code_string`).
fn code_phrase(terminology: &str, code: &str) -> CodePhrase {
    CodePhrase {
        type_tag: TypeTag::new(),
        terminology_id: terminology_id(terminology),
        code_string: code.to_string(),
        preferred_term: None,
    }
}

/// `DV_CODED_TEXT` (schema-required `value` + `defining_code`).
fn coded_text(terminology: &str, code: &str, text: &str) -> DvCodedText {
    DvCodedText {
        type_tag: TypeTag::new(),
        text: DvTextData {
            value: text.to_string(),
            hyperlink: None,
            formatting: None,
            mappings: None,
            language: None,
            encoding: None,
        },
        defining_code: code_phrase(terminology, code),
    }
}

/// A `HIER_OBJECT_ID`-backed `UID_BASED_ID` (satisfies the `LOCATABLE.uid`
/// slot, whose schema requires `_type ∈ {HIER_OBJECT_ID, OBJECT_VERSION_ID}`).
fn hier_uid(value: &str) -> UidBasedId {
    UidBasedId::HierObjectId(HierObjectId {
        type_tag: TypeTag::new(),
        uid_based_id: UidBasedIdData {
            value: value.to_string(),
        },
    })
}

/// Minimal `LOCATABLE` state: only `name` + `archetype_node_id` (+ optional
/// `uid` where a class's schema requires it).
fn locatable(name: &str, node_id: &str, uid: Option<UidBasedId>) -> LocatableData {
    LocatableData {
        name: dv_text(name),
        archetype_node_id: node_id.to_string(),
        uid,
        links: None,
        archetype_details: None,
        feeder_audit: None,
        parent: None,
    }
}

/// Minimal `ITEM_STRUCTURE`: an empty `ITEM_TREE` (the spec permits an empty
/// tree; the schema requires only `name` + `archetype_node_id`). Used for the
/// several demographic `details`/`credentials` slots.
fn item_tree(name: &str, node_id: &str) -> ItemStructure {
    ItemStructure::Tree(ItemTree {
        type_tag: TypeTag::new(),
        item_structure: ItemStructureData {
            data_structure: DataStructureData {
                locatable: locatable(name, node_id, None),
            },
        },
        items: None,
    })
}

/// `PARTY_REF` (schema-required `id`, `namespace`, `type`).
fn party_ref(id: &str) -> PartyRef {
    PartyRef {
        type_tag: TypeTag::new(),
        namespace: "demographic".to_string(),
        r#type: "PERSON".to_string(),
        id: ObjectId::UidBased(hier_uid(id)),
    }
}

/// `PARTY_IDENTITY` (schema-required `name`, `archetype_node_id`, `details`).
fn party_identity(name: &str, node_id: &str) -> PartyIdentity {
    PartyIdentity {
        type_tag: TypeTag::new(),
        locatable: locatable(name, node_id, None),
        details: item_tree("identity details", "at0001"),
    }
}

/// Minimal `PARTY` state: `identities` must be non-empty (schema `minItems: 1`)
/// and carries the given `uid` where the concrete class requires one.
fn party_data(name: &str, node_id: &str, uid: Option<UidBasedId>) -> PartyData {
    PartyData {
        locatable: locatable(name, node_id, uid),
        identities: vec![party_identity("legal name", "at0002")],
        contacts: None,
        details: None,
        reverse_relationships: None,
        relationships: None,
    }
}

/// Minimal `ACTOR` state (used by `PERSON`/`ORGANISATION`/`GROUP`/`AGENT`,
/// each of whose schema requires `uid`).
fn actor_data(name: &str, node_id: &str, uid: &str) -> ActorData {
    ActorData {
        party: party_data(name, node_id, Some(hier_uid(uid))),
        languages: None,
        roles: None,
    }
}

/// `DV_PARSABLE` (schema-required `value` + `formalism`); backs the two
/// time-specification `value` slots.
fn dv_parsable(formalism: &str, value: &str) -> DvParsable {
    DvParsable {
        type_tag: TypeTag::new(),
        encapsulated: DvEncapsulatedData {
            charset: None,
            language: None,
        },
        value: value.to_string(),
        formalism: formalism.to_string(),
    }
}

// ─────────────────────────── The oracle ───────────────────────────

/// One fixture's pipeline: serialize → schema-validate the output →
/// deserialize back → assert value-equality with the original. Returns a
/// human-readable failure string on any step, `Ok(())` on a clean pass.
fn check<T>(class: &str, instance: &T) -> Result<(), String>
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    // 2. serialize to a Value.
    let value = serde_json::to_value(instance).map_err(|e| format!("serialize failed: {e}"))?;

    // 3. the serialized output must validate against the ITS-JSON definition.
    let errors = corpus::schema_errors(class, &value);
    if !errors.is_empty() {
        return Err(format!(
            "serialized output does NOT validate against the ITS-JSON {class} schema:\n{}",
            errors.join("\n")
        ));
    }

    // 4. deserialize back (path-diagnosed) and assert value-equality.
    let back: T = serde_path_to_error::deserialize(&value).map_err(|e| {
        format!(
            "deserialization failed\n    at path: {}\n    error: {}",
            e.path(),
            e.inner()
        )
    })?;
    if &back != instance {
        return Err(format!(
            "round-trip is not value-identical:\n    original: {instance:?}\n    back:     {back:?}"
        ));
    }
    Ok(())
}

/// Runs `check` for one fixture, pushing a labelled failure onto `failures`.
macro_rules! run {
    ($failures:ident, $class:expr, $instance:expr) => {{
        let class: &str = $class;
        if let Err(msg) = check(class, &$instance) {
            $failures.push(format!("[{class}]\n  {msg}"));
        }
    }};
}

/// The data-driven gate: all fifteen synthetic fixtures, every failure
/// reported together.
// The fifteen fixtures are inlined here (each is a minimal struct literal),
// which pushes the function over clippy's default line budget — accepted for a
// data-driven test whose whole point is to enumerate the cases in one place.
#[allow(clippy::too_many_lines)]
#[test]
fn synthetic_gap_fixtures_round_trip_and_validate() {
    let mut failures: Vec<String> = Vec::new();

    // ── rm.demographic (10) ──
    run!(
        failures,
        "PERSON",
        Person {
            type_tag: TypeTag::new(),
            actor: actor_data("PERSON", "at0000", "8849182c-82ad-4088-a07f-48ead4180515"),
        }
    );
    run!(
        failures,
        "ORGANISATION",
        Organisation {
            type_tag: TypeTag::new(),
            actor: actor_data(
                "ORGANISATION",
                "at0000",
                "0ec2f2b0-9e97-4b1a-8f6c-1a2b3c4d5e6f"
            ),
        }
    );
    run!(
        failures,
        "ROLE",
        Role {
            type_tag: TypeTag::new(),
            party: party_data(
                "ROLE",
                "at0000",
                Some(hier_uid("1b2d0f9a-3c4e-4d5f-9a8b-7c6d5e4f3a2b")),
            ),
            time_validity: None,
            performer: party_ref("2c3e1a8b-4d5f-4e6a-8b9c-0d1e2f3a4b5c"),
            capabilities: None,
        }
    );
    run!(
        failures,
        "AGENT",
        Agent {
            type_tag: TypeTag::new(),
            actor: actor_data("AGENT", "at0000", "3d4f2b9c-5e6a-4f7b-9c0d-1e2f3a4b5c6d"),
        }
    );
    run!(
        failures,
        "GROUP",
        Group {
            type_tag: TypeTag::new(),
            actor: actor_data("GROUP", "at0000", "4e5a3c0d-6f7b-4a8c-0d1e-2f3a4b5c6d7e"),
        }
    );
    run!(
        failures,
        "CONTACT",
        Contact {
            type_tag: TypeTag::new(),
            locatable: locatable("contact purpose", "at0000", None),
            addresses: vec![Address {
                type_tag: TypeTag::new(),
                locatable: locatable("address type", "at0001", None),
                details: item_tree("address details", "at0002"),
            }],
            time_validity: None,
        }
    );
    run!(
        failures,
        "ADDRESS",
        Address {
            type_tag: TypeTag::new(),
            locatable: locatable("address type", "at0000", None),
            details: item_tree("address details", "at0001"),
        }
    );
    run!(
        failures,
        "CAPABILITY",
        Capability {
            type_tag: TypeTag::new(),
            locatable: locatable("capability", "at0000", None),
            credentials: item_tree("credentials", "at0001"),
            time_validity: None,
        }
    );
    run!(
        failures,
        "PARTY_IDENTITY",
        party_identity("legal name", "at0000")
    );
    run!(
        failures,
        "PARTY_RELATIONSHIP",
        PartyRelationship {
            type_tag: TypeTag::new(),
            locatable: locatable("relationship type", "at0000", None),
            details: None,
            target: party_ref("5f6a4d1e-7a8c-4b9d-1e2f-3a4b5c6d7e8f"),
            time_validity: None,
            source: party_ref("6a7b5e2f-8b9d-4c0e-2f3a-4b5c6d7e8f90"),
        }
    );

    // ── rare data-value types (5) ──
    run!(
        failures,
        "DV_STATE",
        DvState {
            type_tag: TypeTag::new(),
            value: coded_text("local", "at0001", "active"),
            is_terminal: false,
        }
    );
    run!(
        failures,
        "DV_PARAGRAPH",
        DvParagraph {
            type_tag: TypeTag::new(),
            items: vec![dv_text("first line of prose")],
        }
    );
    run!(
        failures,
        "DV_SCALE",
        DvScale {
            type_tag: TypeTag::new(),
            ordered: DvOrderedData {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: None,
            },
            symbol: coded_text("borg_cr10", "at0002", "very slight"),
            value: Real(1.0),
        }
    );
    run!(
        failures,
        "DV_PERIODIC_TIME_SPECIFICATION",
        DvPeriodicTimeSpecification {
            type_tag: TypeTag::new(),
            value: dv_parsable("HL7:PIVL", "[200004181100;200004181110]/(7d)@DW"),
        }
    );
    run!(
        failures,
        "DV_GENERAL_TIME_SPECIFICATION",
        DvGeneralTimeSpecification {
            type_tag: TypeTag::new(),
            value: dv_parsable("HL7:GTS", "[200004181100;200004181110]"),
        }
    );

    let total = 15;
    let passed = total - failures.len();
    // Visible under `--nocapture` so the pass count is easy to confirm.
    eprintln!("synthetic gap fixtures: {passed}/{total} classes clean");

    assert!(
        failures.is_empty(),
        "{}/{} synthetic gap fixtures failed:\n\n{}",
        failures.len(),
        total,
        failures.join("\n\n")
    );
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ITS-JSON pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6 + ADR-002; synthetic gap fixtures for rm.demographic + rare DV types
//   source_loc: n/a
//   confidence: high
//   todos: 0
//   note: the ONLY hand-built test data in openehr-serde; minimal synthetic instances for the 15 ITS-JSON classes no real corpus reaches (rm.demographic + DV_STATE/DV_PARAGRAPH/DV_SCALE/DV_PERIODIC_TIME_SPECIFICATION/DV_GENERAL_TIME_SPECIFICATION), each serialize→schema-validate→deserialize→equal via the shared corpus::schema_errors helper; data-driven, all failures reported at once. Every other class is covered by real vendored data.
// ─────────────────────────────────────────────
