// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The journey template pack.
//!
//! Carries the CKM OPTs + committed example skeletons the journey stages
//! commit against, and the deterministic payload stamping that turns a
//! committed skeleton into one arrival's body.
//!
//! Payload ground rules: the skeletons are committed artifacts (generated
//! once from the SUT's example endpoint and vendored byte-identical —
//! `corpus/templates/ckm/PROVENANCE.md`), so every SUT receives the same
//! bytes; stamping mutates only the event-context times and the composer
//! name, deterministically from the arrival's planned instant — variation
//! never introduces a validation error a conformant server would reject.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

// TODO(#1451): port the benchmark lab's constraint-aware FLAT-leaf value jitter
// (the retired benchmark lab's renderer, in git history) as a richer stamping mode once the
// benchmark crate migrates into the runner; time/composer stamping is the
// committed baseline until then.

use std::path::Path;

use serde_json::Value;

use crate::model::corpus::CorpusManifest;
use crate::perf::JourneyCatalogue;

/// The rotating composer pool (staff are modelled as event rates; the
/// composer label is the only per-arrival identity).
const STAFF: [&str; 8] = [
    "Nurse Amara Okafor",
    "Dr. Ingrid Larsen",
    "Nurse Tomas Novak",
    "Dr. Priya Sharma",
    "Nurse Lucia Romero",
    "Dr. Sean Murphy",
    "Nurse Mei Chen",
    "Dr. Kwame Mensah",
];

/// One template of the pack: the OPT (the constraint carrier the seeder
/// uploads) and the committed example skeleton (the payload ground).
#[derive(Debug, Clone)]
pub struct PackTemplate {
    /// The corpus key (`cnf.ckm.vital_signs`).
    pub key: String,
    /// The OPT's `template_id` (the wire identifier).
    pub template_id: String,
    /// The vendored OPT 1.4 XML.
    pub opt_xml: String,
    /// The committed example composition skeleton.
    pub skeleton: Value,
}

/// The Simplified-FLAT payload one journey stage commits.
///
/// Carries the OPT that
/// constrains it, that OPT's `template_id` (the `openehr-template-id`
/// channel — ITS-REST overview `Requests_and_responses` §openehr-template-id)
/// and the committed FLAT body.
#[derive(Debug, Clone)]
pub struct FlatPayload {
    /// The `template_id` the commit names in `openehr-template-id`.
    pub template_id: String,
    /// The operational template XML the seeder uploads first.
    pub opt_xml: String,
    /// The FLAT body committed at each stage arrival.
    pub body: Value,
}

/// The TDD-import stage's payload: the document plus the operational
/// template it names, which the seeder uploads before any window opens.
#[derive(Debug, Clone)]
pub struct TddPayload {
    /// The operational template the document instantiates.
    pub opt_xml: String,
    /// The Template Data Document text the stage imports.
    pub document: String,
}

/// The committed payloads the journey stages that do NOT commit a CKM
/// COMPOSITION carry.
///
/// Every one is a corpus fixture the functional catalogue already adjudicates
/// — the load instrument invents no payload of its own. A field is `Some`
/// exactly when the catalogue names an operation that needs it (see
/// [`crate::perf::PerfOp::aux_payload`]).
#[derive(Debug, Clone, Default)]
pub struct AuxPayloads {
    /// The Simplified-FLAT commit payload, when a stage commits FLAT.
    pub flat: Option<FlatPayload>,
    /// The Template Data Document the TDD-import stage commits, with the
    /// operational template it instantiates.
    pub tdd: Option<TddPayload>,
    /// `PERSON`, first content state (the create body).
    pub person: Option<Value>,
    /// `PERSON`, amended content state (the versioned update body).
    pub person_amended: Option<Value>,
    /// `PARTY_RELATIONSHIP` body, when a stage creates one.
    pub party_relationship: Option<Value>,
}

/// The corpus keys the auxiliary payloads come from.
///
/// Fixed, because they are the payloads the functional batteries already
/// adjudicate; the `journey-envelope` validate gate checks the manifest
/// carries them whenever the catalogue names an operation that needs one.
pub const FLAT_OPT_KEY: &str = "cnf.opt.minimal_action";
/// The FLAT body the FLAT-commit stage posts.
pub const FLAT_BODY_KEY: &str = "cnf.flat.vitals.minimal_ctx";
/// The `PERSON` create body the demographic stage posts.
pub const PERSON_KEY: &str = "cnf.demographic.person.v1";
/// The amended `PERSON` body the demographic update stage puts.
pub const PERSON_AMENDED_KEY: &str = "cnf.demographic.person.v2";
/// The `PARTY_RELATIONSHIP` body the demographic stage creates.
pub const PARTY_RELATIONSHIP_KEY: &str = "cnf.demographic.party_relationship.v1";
/// The TDD stage's operational template.
///
/// `nested.en.v1` is category `433|event|`, so a sustained arrival commits a
/// fresh COMPOSITION each time; a `431|persistent|` template would hold
/// exactly one per EHR (RM ehr master04 §COMPOSITION category) and every
/// arrival after the first would be a conflict the instrument manufactured.
pub const TDD_OPT_KEY: &str = "cnf.opt.nested";
/// The Template Data Document the TDD-import stage sends.
pub const TDD_BODY_KEY: &str = "cnf.messaging.tdd.nested";

/// The loaded pack: every template the journey catalogue names, plus the
/// auxiliary payloads its non-COMPOSITION stages carry.
#[derive(Debug, Clone)]
pub struct JourneyPack {
    /// Every operational template the journey catalogue names.
    pub templates: Vec<PackTemplate>,
    /// The payloads the non-COMPOSITION stages carry.
    pub aux: AuxPayloads,
}

impl JourneyPack {
    /// Load every template the catalogue's stages name: the manifest's OPT
    /// entry (`<key>`) and example entry (`<key>.example`), both resolved
    /// against the corpus directory.
    ///
    /// # Errors
    /// A message naming the missing manifest entry, file, or field.
    pub fn load(
        corpus_dir: &Path,
        manifest: &CorpusManifest,
        catalogue: &JourneyCatalogue,
    ) -> Result<Self, String> {
        let mut keys: Vec<String> = Vec::new();
        for (_, journey) in &catalogue.0 {
            for stage in &journey.stages {
                if let Some(template) = &stage.template
                    && !keys.contains(template)
                {
                    keys.push(template.clone());
                }
            }
        }
        keys.sort();
        let entry = |k: &str| {
            crate::ids::CorpusKey::parse(k)
                .ok()
                .and_then(|parsed| manifest.get(&parsed).cloned())
                .ok_or_else(|| format!("corpus manifest has no entry {k}"))
        };
        let read = |source: Option<&String>, what: &str| {
            let source = source.ok_or_else(|| format!("{what} entry has no source"))?;
            std::fs::read_to_string(corpus_dir.join(source))
                .map_err(|e| format!("cannot read {source}: {e}"))
        };
        let read_json = |key: &str| -> Result<Value, String> {
            let e = entry(key)?;
            serde_json::from_str(&read(e.source.as_ref(), key)?)
                .map_err(|error| format!("corpus fixture {key}: {error}"))
        };
        let mut templates = Vec::with_capacity(keys.len());
        for key in keys {
            let opt_entry = entry(&key)?;
            let example_entry = entry(&format!("{key}.example"))?;
            let template_id = opt_entry
                .template_id
                .clone()
                .ok_or_else(|| format!("manifest entry {key} carries no template_id"))?;
            let opt_xml = read(opt_entry.source.as_ref(), &key)?;
            let skeleton: Value = serde_json::from_str(&read(example_entry.source.as_ref(), &key)?)
                .map_err(|e| format!("example skeleton {key}: {e}"))?;
            templates.push(PackTemplate {
                key,
                template_id,
                opt_xml,
                skeleton,
            });
        }
        if templates.is_empty() {
            return Err("the journey catalogue names no templates".to_owned());
        }

        // The auxiliary payloads: loaded exactly when the catalogue names an
        // operation that carries one, so a party's catalogue never pays for
        // fixtures its journeys do not touch.
        let mut needed: Vec<crate::perf::AuxPayloadKind> = Vec::new();
        for (_, journey) in &catalogue.0 {
            for stage in &journey.stages {
                if let Some(kind) = crate::perf::PerfOp::parse(&stage.op)
                    .ok()
                    .and_then(crate::perf::PerfOp::aux_payload)
                    && !needed.contains(&kind)
                {
                    needed.push(kind);
                }
            }
        }
        let mut aux = AuxPayloads::default();
        for kind in needed {
            match kind {
                crate::perf::AuxPayloadKind::Flat => {
                    let opt_entry = entry(FLAT_OPT_KEY)?;
                    aux.flat = Some(FlatPayload {
                        template_id: opt_entry.template_id.clone().ok_or_else(|| {
                            format!("manifest entry {FLAT_OPT_KEY} carries no template_id")
                        })?,
                        opt_xml: read(opt_entry.source.as_ref(), FLAT_OPT_KEY)?,
                        body: read_json(FLAT_BODY_KEY)?,
                    });
                }
                crate::perf::AuxPayloadKind::Person => {
                    aux.person = Some(read_json(PERSON_KEY)?);
                    aux.person_amended = Some(read_json(PERSON_AMENDED_KEY)?);
                }
                crate::perf::AuxPayloadKind::PartyRelationship => {
                    aux.party_relationship = Some(read_json(PARTY_RELATIONSHIP_KEY)?);
                }
                crate::perf::AuxPayloadKind::Tdd => {
                    let opt_entry = entry(TDD_OPT_KEY)?;
                    let body_entry = entry(TDD_BODY_KEY)?;
                    aux.tdd = Some(TddPayload {
                        opt_xml: read(opt_entry.source.as_ref(), TDD_OPT_KEY)?,
                        document: read(body_entry.source.as_ref(), TDD_BODY_KEY)?,
                    });
                }
            }
        }
        Ok(Self { templates, aux })
    }

    /// Look up a template by corpus key.
    #[must_use]
    pub fn index_of(&self, key: &str) -> Option<usize> {
        self.templates.iter().position(|t| t.key == key)
    }

    /// The template at a schedule-resolved index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&PackTemplate> {
        self.templates.get(index)
    }
}

/// The deterministic simulated clock: planned offsets map onto a fixed
/// base date, so renders are byte-identical across SUTs and runs.
#[expect(
    clippy::integer_division,
    reason = "whole hours/minutes/days of the simulated clock: exact integer split, \
              which is what makes the rendered timestamp byte-identical across runs"
)]
pub(crate) fn sim_time(offset_s: u64) -> String {
    let day_s = offset_s % 86_400;
    let (h, m, s) = (day_s / 3600, (day_s % 3600) / 60, day_s % 60);
    let day = 1 + (offset_s / 86_400) % 27;
    format!("2024-06-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn staff(arrival: u64) -> &'static str {
    let index = usize::try_from(arrival % 8).unwrap_or(0);
    STAFF.get(index).copied().unwrap_or(STAFF[0])
}

/// Stamp a skeleton for one arrival: event-context times to the planned
/// instant's simulated clock, composer name from the rotating staff pool.
fn stamped(template: &PackTemplate, offset_s: u64, arrival: u64) -> Value {
    let mut body = template.skeleton.clone();
    let time = sim_time(offset_s);
    if let Some(context) = body.get_mut("context") {
        for field in ["start_time", "end_time"] {
            if let Some(Value::String(value)) =
                context.get_mut(field).and_then(|t| t.get_mut("value"))
            {
                value.clone_from(&time);
            }
        }
    }
    if let Some(Value::String(name)) = body.get_mut("composer").and_then(|c| c.get_mut("name")) {
        staff(arrival).clone_into(name);
    }
    body
}

/// A stamped composition body (canonical JSON bytes).
///
/// # Errors
/// A serialization failure message.
pub(crate) fn composition_body(
    template: &PackTemplate,
    offset_s: u64,
    arrival: u64,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&stamped(template, offset_s, arrival)).map_err(|e| e.to_string())
}

/// A one-version CONTRIBUTION envelope around a stamped composition (the
/// ITS contribution schema: `ORIGINAL_VERSION` members carrying `data` +
/// `commit_audit` + `lifecycle_state`; RM common §`change_control`).
///
/// # Errors
/// A serialization failure message.
pub(crate) fn contribution_body(
    template: &PackTemplate,
    offset_s: u64,
    arrival: u64,
) -> Result<Vec<u8>, String> {
    let audit = |change: &str, code: &str| {
        serde_json::json!({
            "_type": "AUDIT_DETAILS",
            "system_id": "cnf-runner",
            "committer": { "_type": "PARTY_IDENTIFIED", "name": staff(arrival) },
            "change_type": { "_type": "DV_CODED_TEXT", "value": change,
                "defining_code": { "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": code } }
        })
    };
    let envelope = serde_json::json!({
        "_type": "CONTRIBUTION",
        "versions": [{
            "_type": "ORIGINAL_VERSION",
            "lifecycle_state": {
                "_type": "DV_CODED_TEXT",
                "value": "complete",
                "defining_code": { "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": "532" }
            },
            "commit_audit": audit("creation", "249"),
            "data": stamped(template, offset_s, arrival)
        }],
        "audit": audit("creation", "249")
    });
    serde_json::to_vec(&envelope).map_err(|e| e.to_string())
}

/// A replacement `EHR_STATUS` body: queryable + modifiable, anonymous
/// subject, an `other_details` note stamped with the simulated clock (the
/// admission/discharge status touch).
pub(crate) fn ehr_status_body(offset_s: u64) -> Vec<u8> {
    // An EHR_STATUS is an archetype root, so the ARCHETYPED block is
    // mandatory and the root archetype_node_id equals its archetype_id (RM
    // common locatable.adoc §Invariants Archetyped_valid +
    // §archetype_node_id) — the 2026-07-29 POC window sent a root-less body
    // and the server rightly 422'd every update arrival.
    let body = serde_json::json!({
        "_type": "EHR_STATUS",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID",
                               "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
            "rm_version": "1.1.0"
        },
        "subject": { "_type": "PARTY_SELF" },
        "is_queryable": true,
        "is_modifiable": true,
        "other_details": {
            "_type": "ITEM_TREE",
            "name": { "_type": "DV_TEXT", "value": "status" },
            "archetype_node_id": "at0001",
            "items": [{
                "_type": "ELEMENT",
                "name": { "_type": "DV_TEXT", "value": "last ADT touch" },
                "archetype_node_id": "at0002",
                "value": { "_type": "DV_DATE_TIME", "value": sim_time(offset_s) }
            }]
        }
    });
    serde_json::to_vec(&body).unwrap_or_default()
}

/// A directory FOLDER body: the per-episode tree (`episodes` open on
/// admission; a `closed` marker folder appended by the discharge
/// close-out). `archetype_node_id` on EVERY node — RM common
/// `LOCATABLE.Archetype_node_id_valid` (mandatory on each LOCATABLE, the
/// subfolders included).
pub(crate) fn folder_body(closed: bool) -> Vec<u8> {
    let mut folders = vec![serde_json::json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "episodes" }
    })];
    if closed {
        folders.push(serde_json::json!({
            "_type": "FOLDER",
            "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
            "name": { "_type": "DV_TEXT", "value": "closed" }
        }));
    }
    let body = serde_json::json!({
        "_type": "FOLDER",
        "name": { "_type": "DV_TEXT", "value": "root" },
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "folders": folders
    });
    serde_json::to_vec(&body).unwrap_or_default()
}

/// A demographic `PERSON` body for one arrival: the committed corpus
/// fixture with its legal-identity name stamped from the arrival index, so
/// successive registrations are distinct records and the render stays
/// deterministic. Nothing structural is touched — the payload's RM validity
/// is the fixture's (RM demographic §PARTY `Identities_valid`).
///
/// # Errors
/// A serialization failure message.
pub(crate) fn person_body(person: &Value, arrival: u64) -> Result<Vec<u8>, String> {
    let mut body = person.clone();
    if let Some(Value::String(name)) = body
        .get_mut("identities")
        .and_then(|i| i.get_mut(0))
        .and_then(|identity| identity.get_mut("details"))
        .and_then(|details| details.get_mut("items"))
        .and_then(|items| items.get_mut(0))
        .and_then(|item| item.get_mut("value"))
        .and_then(|value| value.get_mut("value"))
    {
        *name = format!("{} (registration {arrival})", staff(arrival));
    }
    serde_json::to_vec(&body).map_err(|e| e.to_string())
}

/// A `PARTY_RELATIONSHIP` body: the committed corpus fixture with its
/// `source` pointed at the party this journey instance just registered (RM
/// demographic master02 §Party Relationships — the relationship names the
/// parties it relates).
///
/// # Errors
/// A serialization failure message.
pub(crate) fn party_relationship_body(
    relationship: &Value,
    source_uid: &str,
) -> Result<Vec<u8>, String> {
    let mut body = relationship.clone();
    if let Some(Value::String(id)) = body
        .get_mut("source")
        .and_then(|source| source.get_mut("id"))
        .and_then(|id| id.get_mut("value"))
    {
        source_uid.clone_into(id);
    }
    serde_json::to_vec(&body).map_err(|e| e.to_string())
}

/// The Simplified-FLAT composition body (the committed fixture verbatim —
/// FLAT paths are template-derived, so nothing in it may be stamped).
///
/// # Errors
/// A serialization failure message.
pub(crate) fn flat_body(payload: &FlatPayload) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&payload.body).map_err(|e| e.to_string())
}

/// The `ITEM_TAG` set the tagging journey replaces (ITS-REST TAGS API).
pub(crate) fn tags_body(offset_s: u64) -> Vec<u8> {
    let body = serde_json::json!([
        { "key": "cnf.workflow", "value": "ward-round" },
        { "key": "cnf.touched", "value": sim_time(offset_s) }
    ]);
    serde_json::to_vec(&body).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template() -> PackTemplate {
        PackTemplate {
            key: "cnf.ckm.vital_signs".to_owned(),
            template_id: "Vital signs".to_owned(),
            opt_xml: "<template/>".to_owned(),
            skeleton: serde_json::json!({
                "_type": "COMPOSITION",
                "context": {
                    "_type": "EVENT_CONTEXT",
                    "start_time": { "_type": "DV_DATE_TIME", "value": "2020-01-01T00:00:00Z" }
                },
                "composer": { "_type": "PARTY_IDENTIFIED", "name": "original" }
            }),
        }
    }

    #[test]
    fn stamping_is_deterministic_and_touches_only_time_and_composer() {
        let t = template();
        let a = composition_body(&t, 3661, 5).unwrap();
        let b = composition_body(&t, 3661, 5).unwrap();
        assert_eq!(a, b);
        let value: Value = serde_json::from_slice(&a).unwrap();
        assert_eq!(
            value["context"]["start_time"]["value"],
            "2024-06-01T01:01:01Z"
        );
        assert_eq!(value["composer"]["name"], STAFF[5]);
        // A different planned instant yields a different render.
        let c = composition_body(&t, 7200, 5).unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn the_contribution_envelope_wraps_one_original_version() {
        let t = template();
        let bytes = contribution_body(&t, 60, 1).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["_type"], "CONTRIBUTION");
        let versions = value["versions"].as_array().unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0]["_type"], "ORIGINAL_VERSION");
        assert_eq!(
            versions[0]["lifecycle_state"]["defining_code"]["code_string"],
            "532"
        );
        assert_eq!(versions[0]["data"]["_type"], "COMPOSITION");
    }

    #[test]
    fn constructed_bodies_parse_and_carry_their_shape() {
        let status: Value = serde_json::from_slice(&ehr_status_body(0)).unwrap();
        assert_eq!(status["_type"], "EHR_STATUS");
        assert_eq!(status["subject"]["_type"], "PARTY_SELF");
        let folder: Value = serde_json::from_slice(&folder_body(true)).unwrap();
        assert_eq!(folder["folders"].as_array().unwrap().len(), 2);
        // RM common LOCATABLE.Archetype_node_id_valid on every node.
        assert!(folder["archetype_node_id"].is_string());
        for sub in folder["folders"].as_array().unwrap() {
            assert!(
                sub["archetype_node_id"].is_string(),
                "subfolder without archetype_node_id"
            );
        }
        let tags: Value = serde_json::from_slice(&tags_body(0)).unwrap();
        assert_eq!(tags.as_array().unwrap().len(), 2);
        assert!(sim_time(90_061).starts_with("2024-06-02T01:01:01"));
    }
}
