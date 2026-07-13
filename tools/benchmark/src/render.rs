//! Seeded instance-payload rendering over the vendored fixture skeletons
//! (`docs/design/benchmark/00-workload-model.md` §4).
//!
//! The module is `render` rather than `gen` because `gen` is a reserved keyword
//! in Rust edition 2024.
//!
//! Every payload starts from a committed corpus skeleton (the same OPT /
//! canonical-JSON fixtures the ECC suite provisions) and receives **deterministic
//! seeded variation of values only, never structure**: numeric leaves move
//! within a plausible band, `DV_DATE_TIME` leaves advance to the event's
//! simulated time, the composer is drawn from the ward's staff pool, and the
//! `EHR_STATUS` subject carries the patient's external id. The same
//! `(skeleton, seed, params)` triple always renders byte-identically, so both
//! SUTs receive an identical request sequence.
//!
//! [`vary`] is the variation core exposed so that skeletons obtained at run time
//! (e.g. a SUT's `GET …/template/{id}/example`, wired in at B3 for the CKM
//! template pack) can flow through the identical machinery — this module never
//! fetches templates or calls a SUT itself.

use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::Value;

use conformance::testdata::fixtures;

use crate::pack;
use crate::{BenchError, TemplateKind};

/// The subject-id namespace stamped into every rendered `EHR_STATUS`
/// (our own value — no openEHR spec governs the benchmark subject namespace).
pub const SUBJECT_NAMESPACE: &str = "ehrbase-bench";

/// The vendored contribution envelope reused (and re-filled) for batch commits —
/// a proven both-SUT-accepted `CONTRIBUTION` shape (register 00 §4; E4). The
/// `contribution.valid` corpus-dir file.
pub const CONTRIBUTION_ENVELOPE: &str = "minimal/minimal_observation.contribution.json";

/// The `ehr-status.valid` corpus-dir file used for every `EHR_STATUS` body.
pub const EHR_STATUS_FIXTURE: &str = "000_ehr_status.json";

/// The corpus provenance of one [`TemplateKind`]: the OPT to provision and the
/// canonical-JSON composition skeleton to vary. The `template_id` is the id the
/// OPT registers under (the `GET …/template/adl1.4/{id}` key).
#[derive(Debug, Clone, Copy)]
pub struct TemplateSource {
    /// The manifest `corpus-dir` key holding the OPT (always `template.valid`).
    pub opt_dir_key: &'static str,
    /// The OPT path within that directory.
    pub opt_rel: &'static str,
    /// The manifest `corpus-dir` key holding the composition skeleton.
    pub comp_dir_key: &'static str,
    /// The composition skeleton file within that directory.
    pub comp_file: &'static str,
    /// The template id the OPT registers under.
    pub template_id: &'static str,
}

/// The ECC-corpus source for a template kind, or `None` for the CKM-pack kinds
/// (which are sourced from [`crate::pack`], not the conformance fixtures).
///
/// PORT NOTE: no openEHR spec governs the benchmark's template selection. The
/// ECC-corpus kinds are retained as proven both-server-accepted payloads; the
/// CKM-pack kinds (E1–E4/E7/E9 clinical events) are the official openEHR CKM
/// templates in `templates/ckm/`.
#[must_use]
pub fn template_source(kind: TemplateKind) -> Option<TemplateSource> {
    Some(match kind {
        TemplateKind::Vitals => TemplateSource {
            opt_dir_key: "template.valid",
            opt_rel: "validation/composition_evaluation_test.opt",
            comp_dir_key: "composition.canonical-json",
            comp_file: "composition_evaluation_test__full.json",
            template_id: "composition_evaluation_test",
        },
        TemplateKind::Nested => TemplateSource {
            opt_dir_key: "template.valid",
            opt_rel: "nested/nested.opt",
            comp_dir_key: "composition.canonical-json",
            comp_file: "nested.en.v1__full.json",
            template_id: "nested.en.v1",
        },
        TemplateKind::Persistent => TemplateSource {
            opt_dir_key: "template.valid",
            opt_rel: "minimal_persistent/persistent_minimal.opt",
            comp_dir_key: "composition.canonical-json",
            comp_file: "persistent_minimal.en.v1__full.json",
            template_id: "persistent_minimal.en.v1",
        },
        TemplateKind::CkmVitalSigns
        | TemplateKind::CkmLabResult
        | TemplateKind::CkmMedicationOrder
        | TemplateKind::CkmSummary
        | TemplateKind::CkmSynopsis => return None,
    })
}

/// The seeded variation inputs for one rendered payload.
#[derive(Debug, Clone)]
pub struct VaryParams {
    /// The patient's stable subject id (stamped into `EHR_STATUS`).
    pub subject_id: String,
    /// The composer name (drawn from the ward staff pool).
    pub composer: String,
    /// The event's simulated wall-clock time (RFC 3339), advanced along the day.
    pub event_time: String,
    /// The workload seed (combined with a per-call salt for value variation).
    pub seed: u64,
}

/// Read the raw OPT text for a template kind (for provisioning uploads).
///
/// # Errors
/// [`BenchError`] if the OPT cannot be read.
pub fn opt_text(kind: TemplateKind) -> Result<String, BenchError> {
    if let Some(tpl) = pack::get(kind) {
        return tpl.opt_text();
    }
    let src = template_source(kind)
        .ok_or_else(|| BenchError::Fixture(format!("no OPT source for {kind:?}")))?;
    fixtures::read_from(src.opt_dir_key, src.opt_rel)
        .map_err(|e| BenchError::Fixture(e.to_string()))
}

/// Render a composition body for a template kind: the corpus skeleton with
/// seeded value variation applied.
///
/// # Errors
/// [`BenchError::Fixture`] if the skeleton cannot be read or parsed.
pub fn composition(kind: TemplateKind, params: &VaryParams) -> Result<Value, BenchError> {
    let skeleton = read_composition_skeleton(kind)?;
    Ok(vary(&skeleton, params))
}

/// Read (and cache-free parse) the composition skeleton: the committed CKM
/// example for a CKM kind, else the canonical-JSON corpus fixture.
fn read_composition_skeleton(kind: TemplateKind) -> Result<Value, BenchError> {
    if let Some(tpl) = pack::get(kind) {
        return tpl.skeleton();
    }
    let src = template_source(kind)
        .ok_or_else(|| BenchError::Fixture(format!("no skeleton source for {kind:?}")))?;
    let text = fixtures::read_from(src.comp_dir_key, src.comp_file)
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
    serde_json::from_str(&text).map_err(BenchError::Json)
}

/// Render an `EHR_STATUS` body carrying the patient's subject id (E1 create,
/// E9 status update). The corpus `EHR_STATUS` fixture is adapted to RM 1.2.0 by
/// the conformance loader; we only stamp the subject id + namespace.
///
/// # Errors
/// [`BenchError::Fixture`] if the fixture cannot be read.
pub fn ehr_status(subject_id: &str, seed: u64) -> Result<Value, BenchError> {
    let text = fixtures::read_from("ehr-status.valid", EHR_STATUS_FIXTURE)
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
    let base: Value = serde_json::from_str(&text).map_err(BenchError::Json)?;
    // is_modifiable / is_queryable jitter would change semantics; keep the
    // fixture's booleans. The seed only participates so a caller may vary the
    // "other_details" leaves in a future revision without changing this shape.
    let _ = seed;
    // adapt_ehr_status defaults subject `_type` to PARTY_SELF, which carries the
    // external_ref and is what strict servers require.
    Ok(fixtures::adapt_ehr_status(
        base,
        SUBJECT_NAMESPACE,
        subject_id,
    ))
}

/// Render a directory (root `FOLDER`) body (E6 directory establishment/update).
/// The structure is fixed; the folder name carries the patient's subject id so
/// the two SUTs' directories are labelled identically per patient.
#[must_use]
pub fn folder(params: &VaryParams) -> Value {
    // `archetype_node_id` is RM-mandatory on every LOCATABLE
    // (RM common `LOCATABLE.Archetype_node_id_valid`); the same generic FOLDER
    // archetype the ECC directory suite commits.
    serde_json::json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "root" },
        "folders": [
            {
                "_type": "FOLDER",
                "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
                "name": {
                    "_type": "DV_TEXT",
                    "value": format!("episodes-{}", params.subject_id)
                }
            }
        ]
    })
}

/// Render a `CONTRIBUTION` batch of `n` (1–3) result compositions (E4). The
/// vendored envelope is reused; each version's `data` is an independently
/// varied composition of `kind`, and the committer name is the params composer.
///
/// # Errors
/// [`BenchError`] if the envelope or the composition skeleton cannot be read.
pub fn contribution(
    kind: TemplateKind,
    params: &VaryParams,
    n: usize,
) -> Result<Value, BenchError> {
    let n = n.clamp(1, 3);
    let text = fixtures::read_from("contribution.valid", CONTRIBUTION_ENVELOPE)
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
    let mut envelope: Value = serde_json::from_str(&text).map_err(BenchError::Json)?;
    let skeleton = read_composition_skeleton(kind)?;

    let template_version = envelope
        .get("versions")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| {
            BenchError::Fixture("contribution envelope has no versions[0]".to_owned())
        })?;

    let mut versions = Vec::with_capacity(n);
    for i in 0..n {
        let mut version = template_version.clone();
        set_committer_name(&mut version, "commit_audit", &params.composer);
        if let Some(obj) = version.as_object_mut() {
            obj.insert(
                "data".to_owned(),
                vary_with_salt(&skeleton, params, i as u64),
            );
        }
        versions.push(version);
    }

    if let Some(obj) = envelope.as_object_mut() {
        obj.insert("versions".to_owned(), Value::Array(versions));
    }
    set_committer_name(&mut envelope, "audit", &params.composer);
    Ok(envelope)
}

/// The variation core: return a copy of `skeleton` with values (numeric leaves,
/// `DV_DATE_TIME`/`DV_DATE`/`DV_TIME` leaves, composer name) varied
/// deterministically from `params`; structure is preserved exactly.
#[must_use]
pub fn vary(skeleton: &Value, params: &VaryParams) -> Value {
    vary_with_salt(skeleton, params, 0)
}

/// [`vary`] with an extra per-call salt (used to differentiate the compositions
/// inside one CONTRIBUTION batch).
fn vary_with_salt(skeleton: &Value, params: &VaryParams, salt: u64) -> Value {
    let mut value = skeleton.clone();
    let seed = fnv1a(params.subject_id.as_bytes())
        ^ fnv1a(params.event_time.as_bytes())
        ^ params.seed
        ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut rng = StdRng::seed_from_u64(seed);
    vary_value(&mut value, &mut rng, params);
    value
}

/// Recursively vary the values of a JSON node in place.
fn vary_value(value: &mut Value, rng: &mut StdRng, params: &VaryParams) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(ty)) = map.get("_type") {
                match ty.as_str() {
                    "DV_QUANTITY" => vary_number(map, "magnitude", rng),
                    "DV_COUNT" => vary_count(map, "magnitude", rng),
                    "DV_PROPORTION" => vary_number(map, "numerator", rng),
                    "DV_DATE_TIME" => set_string(map, "value", &params.event_time),
                    "DV_DATE" => set_string(map, "value", date_part(&params.event_time)),
                    "DV_TIME" => set_string(map, "value", time_part(&params.event_time)),
                    _ => {}
                }
            }
            // Keys are visited in insertion order (serde_json `preserve_order`),
            // keeping the RNG draw order deterministic.
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if key == "composer"
                    && let Some(child) = map.get_mut(&key)
                {
                    set_name_field(child, &params.composer);
                }
                if let Some(child) = map.get_mut(&key) {
                    vary_value(child, rng, params);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                vary_value(item, rng, params);
            }
        }
        _ => {}
    }
}

/// Vary a floating-point leaf within ±15% of its original, rounded to 3 dp.
fn vary_number(map: &mut serde_json::Map<String, Value>, field: &str, rng: &mut StdRng) {
    if let Some(current) = map.get(field).and_then(Value::as_f64) {
        let factor: f64 = rng.random_range(0.85..1.15);
        let varied = (current * factor * 1000.0).round() / 1000.0;
        if let Some(num) = serde_json::Number::from_f64(varied) {
            map.insert(field.to_owned(), Value::Number(num));
        }
    }
}

/// Vary an integer count leaf within ±5 of its original (never negative).
fn vary_count(map: &mut serde_json::Map<String, Value>, field: &str, rng: &mut StdRng) {
    if let Some(current) = map.get(field).and_then(Value::as_i64) {
        let low = (current - 5).max(0);
        let high = current + 5;
        let varied: i64 = rng.random_range(low..=high);
        map.insert(field.to_owned(), Value::Number(varied.into()));
    }
}

/// Set a string field only if it already exists as a string (no structural add).
fn set_string(map: &mut serde_json::Map<String, Value>, field: &str, val: &str) {
    if let Some(slot @ Value::String(_)) = map.get_mut(field) {
        *slot = Value::String(val.to_owned());
    }
}

/// Set a `name.value` (or `name` string) child to the composer name, if present.
fn set_name_field(node: &mut Value, name: &str) {
    if let Some(obj) = node.as_object_mut() {
        match obj.get_mut("name") {
            Some(Value::String(s)) => name.clone_into(s),
            Some(Value::Object(name_obj)) => {
                if let Some(Value::String(v)) = name_obj.get_mut("value") {
                    name.clone_into(v);
                }
            }
            _ => {}
        }
    }
}

/// Set the committer name inside an `AUDIT_DETAILS` field of `node`.
fn set_committer_name(node: &mut Value, audit_field: &str, name: &str) {
    if let Some(committer) = node
        .get_mut(audit_field)
        .and_then(|a| a.get_mut("committer"))
    {
        set_name_field_direct(committer, name);
    }
}

/// Set a `committer.name` string directly (the `AUDIT_DETAILS` committer name is
/// a plain string, not a `DV_TEXT`).
fn set_name_field_direct(committer: &mut Value, name: &str) {
    if let Some(Value::String(s)) = committer.get_mut("name") {
        name.clone_into(s);
    }
}

fn date_part(rfc3339: &str) -> &str {
    rfc3339.split('T').next().unwrap_or(rfc3339)
}

fn time_part(rfc3339: &str) -> &str {
    rfc3339.split_once('T').map_or(rfc3339, |(_, t)| t)
}

/// 64-bit FNV-1a over bytes (RNG salting only — not a cryptographic hash).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> VaryParams {
        VaryParams {
            subject_id: "bench-patient-000007".to_owned(),
            composer: "Dr. Bench Three".to_owned(),
            event_time: "2024-06-01T08:15:00.000Z".to_owned(),
            seed: 0xDEAD_BEEF,
        }
    }

    /// The recursive key/type signature of a JSON value — structure, ignoring
    /// scalar values. Two payloads with the same signature are structurally
    /// identical.
    fn signature(v: &Value) -> String {
        match v {
            Value::Object(map) => {
                let mut parts: Vec<String> = map
                    .iter()
                    .map(|(k, val)| format!("{k}:{}", signature(val)))
                    .collect();
                parts.sort();
                format!("{{{}}}", parts.join(","))
            }
            Value::Array(items) => {
                let inner: Vec<String> = items.iter().map(signature).collect();
                format!("[{}]", inner.join(","))
            }
            Value::String(_) => "s".to_owned(),
            Value::Number(_) => "n".to_owned(),
            Value::Bool(_) => "b".to_owned(),
            Value::Null => "z".to_owned(),
        }
    }

    #[test]
    fn vary_preserves_structure_for_every_template() {
        for kind in [
            TemplateKind::Vitals,
            TemplateKind::Nested,
            TemplateKind::Persistent,
        ] {
            let skeleton = read_composition_skeleton(kind).expect("skeleton reads");
            let rendered = vary(&skeleton, &params());
            assert_eq!(
                signature(&skeleton),
                signature(&rendered),
                "structure changed for {kind:?}"
            );
        }
    }

    #[test]
    fn vary_preserves_structure_for_every_ckm_template() {
        for tpl in pack::all() {
            let skeleton = read_composition_skeleton(tpl.kind).expect("CKM skeleton reads");
            let rendered = vary(&skeleton, &params());
            assert_eq!(
                signature(&skeleton),
                signature(&rendered),
                "structure changed for CKM {}",
                tpl.slug
            );
        }
    }

    #[test]
    fn ckm_render_is_deterministic_and_varies() {
        // The CKM skeletons come from the pack module, then through the same
        // `vary` core: same params render identically, a changed event time
        // changes the payload (the DV_DATE_TIME leaves advance).
        let a = composition(TemplateKind::CkmVitalSigns, &params()).expect("render");
        let b = composition(TemplateKind::CkmVitalSigns, &params()).expect("render");
        assert_eq!(a, b, "same params must render identically");
        let mut p2 = params();
        p2.event_time = "2024-06-01T14:30:00.000Z".to_owned();
        let c = composition(TemplateKind::CkmVitalSigns, &p2).expect("render");
        assert_ne!(a, c, "different event time must change the CKM payload");
    }

    #[test]
    fn render_is_deterministic() {
        let a = composition(TemplateKind::Nested, &params()).expect("render");
        let b = composition(TemplateKind::Nested, &params()).expect("render");
        assert_eq!(a, b, "same params must render identically");
    }

    #[test]
    fn render_varies_with_params() {
        let a = composition(TemplateKind::Vitals, &params()).expect("render");
        let mut p2 = params();
        p2.event_time = "2024-06-01T14:30:00.000Z".to_owned();
        let b = composition(TemplateKind::Vitals, &p2).expect("render");
        // The event time is stamped into DV_DATE_TIME leaves, so the payloads
        // must differ.
        assert_ne!(a, b, "different event time must change the payload");
    }

    #[test]
    fn datetime_leaves_advance_to_event_time() {
        let rendered = composition(TemplateKind::Vitals, &params()).expect("render");
        // composition_evaluation_test has context.start_time / end_time.
        let start = rendered
            .pointer("/context/start_time/value")
            .and_then(Value::as_str)
            .expect("start_time present");
        assert_eq!(start, "2024-06-01T08:15:00.000Z");
    }

    #[test]
    fn ehr_status_carries_subject_id() {
        let status = ehr_status("bench-patient-000042", 1).expect("status");
        let id = status
            .pointer("/subject/external_ref/id/value")
            .and_then(Value::as_str)
            .expect("subject id present");
        assert_eq!(id, "bench-patient-000042");
        let ns = status
            .pointer("/subject/external_ref/namespace")
            .and_then(Value::as_str)
            .expect("namespace present");
        assert_eq!(ns, SUBJECT_NAMESPACE);
    }

    #[test]
    fn contribution_batches_and_preserves_envelope() {
        let c = contribution(TemplateKind::Vitals, &params(), 3).expect("contribution");
        assert_eq!(c["_type"], "CONTRIBUTION");
        let versions = c["versions"].as_array().expect("versions array");
        assert_eq!(versions.len(), 3);
        for v in versions {
            assert_eq!(v["_type"], "ORIGINAL_VERSION");
            assert!(v.get("data").is_some(), "each version wraps a composition");
        }
    }

    #[test]
    fn folder_labels_per_patient() {
        let f = folder(&params());
        let label = f
            .pointer("/folders/0/name/value")
            .and_then(Value::as_str)
            .expect("subfolder name");
        assert!(label.contains("bench-patient-000007"), "label {label}");
    }
}
