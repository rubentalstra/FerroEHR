//! The journey template pack: the CKM OPTs + committed example skeletons
//! the journey stages commit against, and the deterministic payload
//! stamping that turns a committed skeleton into one arrival's body.
//!
//! Payload ground rules: the skeletons are committed artifacts (generated
//! once from the SUT's example endpoint and vendored byte-identical —
//! `corpus/templates/ckm/PROVENANCE.md`), so every SUT receives the same
//! bytes; stamping mutates only the event-context times and the composer
//! name, deterministically from the arrival's planned instant — variation
//! never introduces a validation error a conformant server would reject.
// TODO: port the benchmark lab's constraint-aware FLAT-leaf value jitter
// (tools/benchmark/src/render.rs) as a richer stamping mode once the
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

/// The loaded pack: every template the journey catalogue names.
#[derive(Debug, Clone)]
pub struct JourneyPack {
    pub templates: Vec<PackTemplate>,
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
        let mut templates = Vec::with_capacity(keys.len());
        for key in keys {
            let entry = |k: &str| {
                crate::ids::CorpusKey::parse(k)
                    .ok()
                    .and_then(|parsed| manifest.get(&parsed).cloned())
                    .ok_or_else(|| format!("corpus manifest has no entry {k}"))
            };
            let opt_entry = entry(&key)?;
            let example_entry = entry(&format!("{key}.example"))?;
            let template_id = opt_entry
                .template_id
                .clone()
                .ok_or_else(|| format!("manifest entry {key} carries no template_id"))?;
            let read = |source: Option<&String>, what: &str| {
                let source = source.ok_or_else(|| format!("{what} entry has no source"))?;
                std::fs::read_to_string(corpus_dir.join(source))
                    .map_err(|e| format!("cannot read {source}: {e}"))
            };
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
        Ok(Self { templates })
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
    let body = serde_json::json!({
        "_type": "EHR_STATUS",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
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

/// The `ITEM_TAG` set the tagging journey replaces (ITS-REST TAGS API).
pub(crate) fn tags_body(offset_s: u64) -> Vec<u8> {
    let body = serde_json::json!([
        { "key": "cnf.workflow", "value": "ward-round" },
        { "key": "cnf.touched", "value": sim_time(offset_s) }
    ]);
    serde_json::to_vec(&body).unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
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
