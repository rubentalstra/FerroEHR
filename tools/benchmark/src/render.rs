//! Seeded instance-payload rendering over the vendored fixture skeletons.
//!
//! The module is `render` rather than `gen` because `gen` is a reserved keyword
//! in Rust edition 2024.
//!
//! # Constraint-aware variation (the fairness guarantee)
//!
//! Every payload starts from a committed corpus skeleton (the CKM example
//! skeletons and the ECC-corpus canonical-JSON fixtures) and receives
//! **deterministic seeded variation of values only, never structure**. The
//! variation is **constraint-aware by construction**: it is applied in FLAT
//! (simSDT) space against the template's `WebTemplate`, so every leaf is
//! addressable against its web-template input and every jittered value is kept
//! inside the leaf's AOM constraint (magnitude/count/numerator ranges honoured;
//! `C_INTEGER`/`C_REAL` list-constrained leaves left untouched; temporal leaves
//! stamped only within their `C_DATE_TIME` pattern + `timezone_validity`). A
//! naive raw-JSON jitter (the prior approach) pushed range-clamped leaves in the
//! newly-populated CKM skeletons outside their constraints, which a conformant
//! server correctly answers `422` — voiding the run. Variation in FLAT space
//! cannot produce that.
//!
//! ## Two rendering modes, chosen empirically per template
//!
//! * **`Flat` (constraint-aware).** The template's OPT builds a `WebTemplate`
//!   and the skeleton round-trips through `to_flat`/`from_flat` *faithfully*
//!   (same populated-leaf count, no new validation messages). The skeleton is
//!   decomposed to a FLAT map, numeric/temporal leaves are jittered against
//!   their web-template inputs, and `from_flat` reassembles the canonical
//!   composition. The five CKM templates — the only compositions the measured
//!   workload commits — are `/example`-generated and round-trip byte-identically,
//!   so this is lossless for them.
//! * **`Raw` (structure-preserving).** For a template whose skeleton does *not*
//!   round-trip faithfully through FLAT (the ECC-corpus fixtures are richer than
//!   their compacted web-template — e.g. a persistent composition has no
//!   `EVENT_CONTEXT` that `from_flat` would inject, and a deeply nested corpus
//!   example carries content the compacted tree drops), the raw JSON is
//!   preserved and only the composition-context times + composer name (the
//!   unconstrained RM housekeeping the workload's ordering/attribution needs)
//!   are stamped; no leaf value is jittered, so no constraint can be breached.
//!   The corpus templates are **provisioning-only** — the measured workload
//!   uploads their OPTs (E10) but never commits their compositions — so raw
//!   variation is measurement-neutral and honest (it never distorts a payload
//!   nor asserts a shape the fixture does not have).
//!
//! CKM templates *must* render `Flat` (a CKM skeleton that failed to round-trip
//! is a committed-payload defect and is a hard error, not silently downgraded).
//!
//! The same `(kind, subject_id, event_time, seed, salt)` inputs always render
//! byte-identically, so both SUTs receive an identical request sequence.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;

use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::{Map, Value};

use crate::sutclient::fixtures;
use openehr_flat::convert::{composition_from_flat, composition_to_flat};
use openehr_flat::validation::validate_composition;
use openehr_flat::webtemplate::{
    WebTemplate, WebTemplateNode, WebTemplateRange, build_web_template,
};

use crate::pack;
use crate::{BenchError, TemplateKind};

/// Fixed `ctx/time` default for the FLAT rebuild direction (ITS-REST
/// `simplified_formats` master04 §Context). The benchmark renders deterministic
/// skeletons, so a constant instant keeps a rebuild reproducible.
const NOW: &str = "2024-01-01T00:00:00Z";

/// The subject-id namespace stamped into every rendered `EHR_STATUS`
/// (our own value — no openEHR spec governs the benchmark subject namespace).
pub const SUBJECT_NAMESPACE: &str = "ehrbase-bench";

/// The vendored contribution envelope reused (and re-filled) for batch commits —
/// a proven both-SUT-accepted `CONTRIBUTION` shape. The
/// `contribution.valid` corpus-dir file.
pub const CONTRIBUTION_ENVELOPE: &str = "minimal/minimal_observation.contribution.json";

/// The `ehr-status.valid` corpus-dir file used for every `EHR_STATUS` body.
pub const EHR_STATUS_FIXTURE: &str = "000_ehr_status.json";

/// Every [`TemplateKind`], in a stable order (corpus fixtures first, then the
/// CKM pack — the same order provisioning uploads in).
const ALL_KINDS: [TemplateKind; 8] = [
    TemplateKind::Vitals,
    TemplateKind::Nested,
    TemplateKind::Persistent,
    TemplateKind::CkmVitalSigns,
    TemplateKind::CkmLabResult,
    TemplateKind::CkmMedicationOrder,
    TemplateKind::CkmSummary,
    TemplateKind::CkmSynopsis,
];

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
/// (which are sourced from [`crate::pack`], not the vendored CNF fixtures).
///
/// NOTE: no openEHR spec governs the benchmark's template selection. The
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

// ── prepared templates (WebTemplate + skeleton, built once) ─────────────────────

/// A template prepared for rendering: its build-once state (the mode chosen by
/// the faithfulness gate) plus everything a render needs.
enum Prepared {
    /// Constraint-aware FLAT variation: the skeleton round-trips faithfully, so
    /// jitter is applied in FLAT space against `WebTemplate`.
    Flat(Box<WebTemplate>, Value),
    /// Structure-preserving raw variation (context times + composer only) for a
    /// provisioning-only corpus template whose skeleton does not round-trip
    /// faithfully through FLAT (or whose OPT built no template).
    Raw(Value),
    /// Preparation failed for a kind that requires it (a CKM committed payload).
    Failed(String),
}

/// Prepared templates, built once (`WebTemplate` building is the expensive step;
/// the skeleton read + `to_flat`/`from_flat` per render are cheap and happen at
/// schedule-build time, off the timed loop). Deterministic: the same vendored
/// inputs always prepare identically.
static PREPARED: LazyLock<HashMap<TemplateKind, Prepared>> =
    LazyLock::new(|| ALL_KINDS.iter().map(|&k| (k, prepare(k))).collect());

/// Force the prepared-template cache and fail if any template a measured commit
/// depends on could not be prepared (a CKM `Failed`). Called at schedule-build
/// time so a broken pack surfaces as a build error, never a silently-null
/// payload in the hot loop.
///
/// # Errors
/// [`BenchError::Fixture`] listing every template that failed to prepare.
pub fn preflight() -> Result<(), BenchError> {
    let failed: Vec<&str> = ALL_KINDS
        .iter()
        .filter_map(|k| match PREPARED.get(k) {
            Some(Prepared::Failed(msg)) => Some(msg.as_str()),
            _ => None,
        })
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        Err(BenchError::Fixture(format!(
            "template preparation failed: {}",
            failed.join("; ")
        )))
    }
}

/// Prepare one template: read the skeleton, build the `WebTemplate`, and choose
/// the rendering mode by the faithfulness gate.
fn prepare(kind: TemplateKind) -> Prepared {
    let skeleton = match read_composition_skeleton(kind) {
        Ok(s) => s,
        Err(e) => return Prepared::Failed(format!("{kind:?} skeleton: {e}")),
    };
    let is_ckm = pack::is_ckm(kind);
    let wt = match build_template(kind) {
        Ok(wt) => wt,
        Err(e) => {
            return if is_ckm {
                Prepared::Failed(format!(
                    "{kind:?} web-template build failed (a committed CKM payload requires it): {e}"
                ))
            } else {
                // Corpus, provisioning-only: keep the raw skeleton, stamp only
                // housekeeping.
                Prepared::Raw(skeleton)
            };
        }
    };
    if is_faithful(&skeleton, &wt) {
        Prepared::Flat(Box::new(wt), skeleton)
    } else if is_ckm {
        Prepared::Failed(format!(
            "{kind:?} CKM skeleton does not round-trip faithfully through FLAT — refusing to \
             commit a distorted payload"
        ))
    } else {
        Prepared::Raw(skeleton)
    }
}

/// Build the `WebTemplate` for a kind from its OPT 1.4 XML.
fn build_template(kind: TemplateKind) -> Result<WebTemplate, BenchError> {
    let xml = opt_text(kind)?;
    let opt = openehr_its::opt14::from_xml(&xml)
        .map_err(|e| BenchError::Fixture(format!("OPT parse: {e}")))?;
    build_web_template(&opt).map_err(|e| BenchError::Fixture(format!("web-template build: {e}")))
}

/// Whether `to_flat`/`from_flat` round-trips `skeleton` faithfully under `wt`:
/// the reassembled composition preserves the populated `DATA_VALUE`-leaf count
/// and introduces no validation message the skeleton did not already carry. A
/// faithful round-trip is the precondition for jittering in FLAT space and
/// reassembling without distorting the committed payload.
fn is_faithful(skeleton: &Value, wt: &WebTemplate) -> bool {
    let Ok(flat) = composition_to_flat(skeleton, wt) else {
        return false;
    };
    let map: Map<String, Value> = flat.into_iter().collect();
    let Ok(rebuilt) = composition_from_flat(&map, wt, NOW) else {
        return false;
    };
    if dv_leaf_count(&rebuilt) != dv_leaf_count(skeleton) {
        return false;
    }
    let baseline = message_set(&validate_composition(skeleton, wt));
    validate_composition(&rebuilt, wt)
        .iter()
        .all(|m| baseline.contains(&message_signature(m)))
}

/// Count populated `DATA_VALUE` leaves (a coarse structural-fidelity metric).
fn dv_leaf_count(v: &Value) -> usize {
    fn rec(v: &Value, n: &mut usize) {
        match v {
            Value::Object(m) => {
                if m.get("_type")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t.starts_with("DV_"))
                {
                    *n += 1;
                }
                for x in m.values() {
                    rec(x, n);
                }
            }
            Value::Array(a) => {
                for x in a {
                    rec(x, n);
                }
            }
            _ => {}
        }
    }
    let mut n = 0;
    rec(v, &mut n);
    n
}

/// Read (and parse) the composition skeleton: the committed CKM example for a
/// CKM kind, else the canonical-JSON corpus fixture.
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

// ── rendering ───────────────────────────────────────────────────────────────

/// Render a composition body for a template kind: the skeleton with seeded,
/// constraint-aware value variation applied (see the module docs).
///
/// # Errors
/// [`BenchError::Fixture`] if the template could not be prepared, or the FLAT
/// decomposition/reassembly fails.
pub fn composition(kind: TemplateKind, params: &VaryParams) -> Result<Value, BenchError> {
    render_prepared(kind, params, 0)
}

/// Render a `kind` composition with the given per-call `salt` (differentiating
/// the compositions inside one CONTRIBUTION batch).
fn render_prepared(
    kind: TemplateKind,
    params: &VaryParams,
    salt: u64,
) -> Result<Value, BenchError> {
    match PREPARED
        .get(&kind)
        .ok_or_else(|| BenchError::Fixture(format!("no prepared template for {kind:?}")))?
    {
        Prepared::Flat(wt, skeleton) => render_flat(wt, skeleton, params, salt),
        Prepared::Raw(skeleton) => Ok(render_raw(skeleton, params)),
        Prepared::Failed(msg) => Err(BenchError::Fixture(msg.clone())),
    }
}

/// Constraint-aware render: decompose to FLAT, jitter leaves inside their
/// web-template constraints, reassemble.
fn render_flat(
    wt: &WebTemplate,
    skeleton: &Value,
    params: &VaryParams,
    salt: u64,
) -> Result<Value, BenchError> {
    let flat = composition_to_flat(skeleton, wt).map_err(|e| BenchError::Fixture(e.to_string()))?;
    let mut map: Map<String, Value> = flat.into_iter().collect();
    let mut rng = StdRng::seed_from_u64(derive_seed(params, salt));
    jitter_flat(&mut map, wt, params, &mut rng);
    composition_from_flat(&map, wt, NOW).map_err(|e| BenchError::Fixture(e.to_string()))
}

/// Structure-preserving render: stamp only the composition-context times and the
/// composer name (unconstrained RM housekeeping), leaving every leaf value.
fn render_raw(skeleton: &Value, params: &VaryParams) -> Value {
    let mut comp = skeleton.clone();
    for ptr in ["/context/start_time/value", "/context/end_time/value"] {
        if let Some(slot @ Value::String(_)) = comp.pointer_mut(ptr) {
            *slot = Value::String(params.event_time.clone());
        }
    }
    if let Some(slot @ Value::String(_)) = comp.pointer_mut("/composer/name/value") {
        *slot = Value::String(params.composer.clone());
    } else if let Some(slot @ Value::String(_)) = comp.pointer_mut("/composer/name") {
        *slot = Value::String(params.composer.clone());
    }
    comp
}

/// Derive the per-render RNG seed from the variation inputs (deterministic:
/// the same inputs always seed identically).
fn derive_seed(params: &VaryParams, salt: u64) -> u64 {
    fnv1a(params.subject_id.as_bytes())
        ^ fnv1a(params.event_time.as_bytes())
        ^ params.seed
        ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Jitter the numeric/temporal leaves of a FLAT map in place, each inside its
/// web-template constraint, and stamp the workload housekeeping (`ctx/time`,
/// `ctx/composer_name`). Keys are visited in sorted order so the RNG draw order
/// is deterministic regardless of the FLAT map's own iteration order.
fn jitter_flat(
    flat: &mut Map<String, Value>,
    wt: &WebTemplate,
    params: &VaryParams,
    rng: &mut StdRng,
) {
    let leaves = leaf_index(wt);
    let mut keys: Vec<String> = flat.keys().cloned().collect();
    keys.sort();
    for key in keys {
        if key.starts_with("ctx/") {
            continue; // ctx housekeeping is stamped after the leaf pass
        }
        let (base, suffix) = match key.split_once('|') {
            Some((b, s)) => (b, Some(s)),
            None => (key.as_str(), None),
        };
        let Some(node) = resolve(&leaves, base) else {
            continue;
        };
        let rm = node.rm_type.split('<').next().unwrap_or(&node.rm_type);
        match (rm, suffix) {
            ("DV_QUANTITY", Some("magnitude")) => {
                if has_numeric_list(node, "magnitude") {
                    continue; // C_REAL/C_INTEGER list-constrained: leave unchanged
                }
                let Some(current) = flat.get(&key).and_then(Value::as_f64) else {
                    continue;
                };
                let unit = flat
                    .get(&format!("{base}|unit"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let range = quantity_range(node, unit.as_deref());
                let jittered = round3(current * decimal_factor(rng));
                let value = constrain_decimal(current, jittered, range.as_ref());
                set_number(flat, &key, value);
            }
            ("DV_COUNT", None) => {
                if has_numeric_list(node, "magnitude") {
                    continue;
                }
                let Some(current) = flat.get(&key).and_then(Value::as_i64) else {
                    continue;
                };
                let range = input_range(node, None);
                let jittered = current + rng.random_range(-5..=5);
                let value = constrain_count(current, jittered, range.as_ref());
                flat.insert(key.clone(), Value::Number(value.into()));
            }
            ("DV_PROPORTION", Some("numerator")) => {
                if has_numeric_list(node, "numerator") {
                    continue;
                }
                // Conservative: only jitter when a numerator range is declared,
                // and never for the integral proportion kinds (fraction /
                // integer_fraction), whose RM integrality invariant a float
                // jitter would break. `type` codes: ratio 0, unitary 1,
                // percent 2, fraction 3, integer_fraction 4 (RM DV_PROPORTION;
                // master05 §DV_PROPORTION).
                let Some(range) = input_range(node, Some("numerator")) else {
                    continue;
                };
                if matches!(
                    flat.get(&format!("{base}|type")).and_then(Value::as_i64),
                    Some(3 | 4)
                ) {
                    continue;
                }
                let Some(current) = flat.get(&key).and_then(Value::as_f64) else {
                    continue;
                };
                let jittered = round3(current * decimal_factor(rng));
                let value = constrain_decimal(current, jittered, Some(&range));
                set_number(flat, &key, value);
            }
            ("DV_DATE_TIME" | "DV_DATE" | "DV_TIME", None) => {
                if let Some(stamped) = stamp_temporal(rm, node, &params.event_time) {
                    flat.insert(key.clone(), Value::String(stamped));
                }
            }
            _ => {}
        }
    }
    // Workload housekeeping (unconstrained RM context): the start-time drives the
    // workload's ordering and the composer its attribution.
    flat.insert(
        "ctx/time".to_owned(),
        Value::String(params.event_time.clone()),
    );
    flat.insert(
        "ctx/composer_name".to_owned(),
        Value::String(params.composer.clone()),
    );
}

/// A random multiplicative jitter factor in `[0.85, 1.15)` (±15%).
fn decimal_factor(rng: &mut StdRng) -> f64 {
    rng.random_range(0.85..1.15)
}

/// Round to 3 decimal places.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Insert a finite `f64` as a JSON number at `key` (a non-finite result is
/// dropped, leaving the original value).
fn set_number(flat: &mut Map<String, Value>, key: &str, value: f64) {
    if let Some(n) = serde_json::Number::from_f64(value) {
        flat.insert(key.to_owned(), Value::Number(n));
    }
}

/// Whether the node carries a `C_INTEGER`/`C_REAL` list constraint on the RM
/// attribute `attr` (a list-constrained leaf must stay on an enumerated value,
/// so it is never jittered).
fn has_numeric_list(node: &WebTemplateNode, attr: &str) -> bool {
    node.numeric_lists.iter().any(|(a, _)| a == attr)
}

/// The web-template input matching `suffix` (or the first input when `None`),
/// and its validation range (cloned to avoid entangling the node borrow with
/// the FLAT-map mutation).
fn input_range(node: &WebTemplateNode, suffix: Option<&str>) -> Option<WebTemplateRange> {
    node.inputs
        .iter()
        .find(|i| i.suffix.as_deref() == suffix)
        .and_then(|i| i.validation.as_ref())
        .and_then(|v| v.range.clone())
}

/// The magnitude range for a `DV_QUANTITY` leaf: the magnitude input's own
/// range, else the range scoped to the instance's unit (mirroring the
/// validator's `check_quantity`).
fn quantity_range(node: &WebTemplateNode, unit: Option<&str>) -> Option<WebTemplateRange> {
    input_range(node, Some("magnitude")).or_else(|| {
        let unit = unit?;
        node.inputs
            .iter()
            .find(|i| i.suffix.as_deref() == Some("unit"))?
            .list
            .iter()
            .find(|cv| cv.value == unit)?
            .validation
            .as_ref()?
            .range
            .clone()
    })
}

/// Constrain a jittered decimal into `range` (honouring open bounds); if the
/// result cannot satisfy the range (e.g. it lands on an excluded boundary), fall
/// back to the known-valid original.
fn constrain_decimal(original: f64, jittered: f64, range: Option<&WebTemplateRange>) -> f64 {
    let Some(range) = range else {
        return jittered;
    };
    let mut v = jittered;
    if let Some(min) = range.min.as_ref().and_then(Value::as_f64)
        && v < min
    {
        v = min;
    }
    if let Some(max) = range.max.as_ref().and_then(Value::as_f64)
        && v > max
    {
        v = max;
    }
    if range_ok(v, range) { v } else { original }
}

/// Constrain a jittered integer count into `range` (integer open bounds are
/// exact: `>` min ⇒ min+1, `<` max ⇒ max−1).
fn constrain_count(original: i64, jittered: i64, range: Option<&WebTemplateRange>) -> i64 {
    let Some(range) = range else {
        return jittered;
    };
    let lo = range.min.as_ref().and_then(Value::as_i64).map(|m| {
        if range.min_op.as_deref() == Some(">") {
            m + 1
        } else {
            m
        }
    });
    let hi = range.max.as_ref().and_then(Value::as_i64).map(|m| {
        if range.max_op.as_deref() == Some("<") {
            m - 1
        } else {
            m
        }
    });
    if let (Some(lo), Some(hi)) = (lo, hi)
        && lo > hi
    {
        return original; // contradictory range — leave the valid original
    }
    let mut v = jittered;
    if let Some(lo) = lo
        && v < lo
    {
        v = lo;
    }
    if let Some(hi) = hi
        && v > hi
    {
        v = hi;
    }
    v
}

/// Whether `value` satisfies a web-template numeric range (mirrors the
/// `openehr-flat` validator's `in_range`: `>`/`<` exclude the bound; a missing
/// bound is unbounded). BASE `Interval.has(v)` semantics over the Better wire
/// representation (`org.openehr.base.foundation_types.interval.adoc`).
fn range_ok(value: f64, range: &WebTemplateRange) -> bool {
    if let Some(min) = range.min.as_ref().and_then(Value::as_f64) {
        let ok = match range.min_op.as_deref() {
            Some(">") => value > min,
            _ => value >= min,
        };
        if !ok {
            return false;
        }
    }
    if let Some(max) = range.max.as_ref().and_then(Value::as_f64) {
        let ok = match range.max_op.as_deref() {
            Some("<") => value < max,
            _ => value <= max,
        };
        if !ok {
            return false;
        }
    }
    true
}

/// Stamp a temporal leaf from the event time, honouring the leaf's
/// `C_DATE_TIME`/`C_DATE`/`C_TIME` pattern (partial-precision segments truncate
/// the stamped value) and `timezone_validity`. Returns `None` — keep the
/// original — when the leaf carries a temporal *range* constraint (the skeleton
/// value is known-valid; a stamped instant could fall outside it).
fn stamp_temporal(rm: &str, node: &WebTemplateNode, event_time: &str) -> Option<String> {
    let validation = node.inputs.first().and_then(|i| i.validation.as_ref());
    if validation.and_then(|v| v.range.as_ref()).is_some() {
        return None;
    }
    let pattern = validation.and_then(|v| v.pattern.as_deref());
    Some(build_temporal(rm, pattern, node.tz_validity, event_time))
}

/// Build a temporal value from the event time truncated to the leaf's allowed
/// precision (mirrors `openehr_flat::example::example_temporal`, the validated
/// example generator, but stamped from the workload's event time rather than a
/// fixed instant): year always; month/day/hour/minute/second kept unless the
/// pattern segment prohibits them (`XX`); the timezone omitted iff
/// `timezone_validity == 1003` (disallowed).
fn build_temporal(
    rm: &str,
    pattern: Option<&str>,
    tz_validity: Option<i32>,
    event: &str,
) -> String {
    let (ev_date, ev_time_raw) = event.split_once('T').unwrap_or((event, ""));
    let ev_time = trim_timezone(ev_time_raw);
    let (pat_date, pat_time) = match pattern {
        Some(p) if p.contains('T') => p.split_once('T').unwrap_or((p, "")),
        Some(p) if p.contains(':') => ("", p),
        Some(p) => (p, ""),
        None => ("", ""),
    };
    let date_full: Vec<&str> = ev_date.splitn(3, '-').collect();
    let build_date = |pat: &str| -> String {
        let segs: Vec<&str> = if pat.is_empty() {
            Vec::new()
        } else {
            pat.splitn(3, '-').collect()
        };
        let mut out = date_full.first().copied().unwrap_or("2024").to_owned();
        for (i, part) in date_full.iter().enumerate().skip(1) {
            if segs.get(i).is_some_and(|s| *s == "XX") {
                break;
            }
            out.push('-');
            out.push_str(part);
        }
        out
    };
    let time_full: Vec<&str> = ev_time
        .splitn(3, ':')
        .map(|s| s.split('.').next().unwrap_or(s))
        .collect();
    let build_time = |pat: &str| -> String {
        let segs: Vec<&str> = if pat.is_empty() {
            Vec::new()
        } else {
            pat.splitn(3, ':').collect()
        };
        let mut out = time_full.first().copied().unwrap_or("00").to_owned();
        for (i, part) in time_full.iter().enumerate().skip(1) {
            if segs.get(i).is_some_and(|s| *s == "XX") {
                break;
            }
            out.push(':');
            out.push_str(part);
        }
        out
    };
    let tz = if tz_validity == Some(1003) { "" } else { "Z" };
    match rm {
        "DV_DATE" => build_date(pat_date),
        "DV_TIME" => format!("{}{tz}", build_time(pat_time)),
        // DV_DATE_TIME: a time part prohibited outright (pattern `…TXX:…`)
        // truncates to the date.
        _ => {
            if pat_time.starts_with("XX") {
                build_date(pat_date)
            } else {
                format!("{}T{}{tz}", build_date(pat_date), build_time(pat_time))
            }
        }
    }
}

/// Strip a trailing timezone designator (`Z`, or a `+hh:mm`/`-hh:mm` offset)
/// from a time part.
fn trim_timezone(time: &str) -> &str {
    if let Some(stripped) = time.strip_suffix(['Z', 'z']) {
        return stripped;
    }
    // A `+`/`-` offset: split at the last such sign (the date part is already
    // removed, so any sign here is the timezone).
    match time.rfind(['+', '-']) {
        Some(i) => &time[..i],
        None => time,
    }
}

// ── leaf resolution (flat key → web-template node) ─────────────────────────────

/// Build the map from a leaf's de-indexed json-id path (`root/child/.../leaf`,
/// matching the FLAT key structure `to_flat` emits) to its web-template node.
fn leaf_index(wt: &WebTemplate) -> HashMap<String, &WebTemplateNode> {
    fn walk<'a>(
        node: &'a WebTemplateNode,
        prefix: &str,
        out: &mut HashMap<String, &'a WebTemplateNode>,
    ) {
        if !node.inputs.is_empty() {
            out.insert(prefix.to_owned(), node);
            return;
        }
        for child in &node.children {
            let child_prefix = format!("{prefix}/{}", child.id);
            walk(child, &child_prefix, out);
        }
    }
    let mut out = HashMap::new();
    walk(&wt.tree, &wt.tree.id, &mut out);
    out
}

/// Resolve a FLAT key's base path (already stripped of its `|suffix`) to its
/// leaf node: drop each segment's `:index`, then look up the de-indexed path.
fn resolve<'a>(
    leaves: &HashMap<String, &'a WebTemplateNode>,
    base: &str,
) -> Option<&'a WebTemplateNode> {
    let deindexed: String = base
        .split('/')
        .map(|seg| seg.split(':').next().unwrap_or(seg))
        .collect::<Vec<_>>()
        .join("/");
    leaves.get(&deindexed).copied()
}

// ── validation-message set helpers (faithfulness gate + tests) ─────────────────

/// A comparable signature for a validation message (path + kind + text).
fn message_signature(m: &openehr_flat::validation::ValidationMessage) -> String {
    format!("{}|{:?}|{}", m.path, m.kind, m.message)
}

/// The set of message signatures for a validation result.
fn message_set(msgs: &[openehr_flat::validation::ValidationMessage]) -> HashSet<String> {
    msgs.iter().map(message_signature).collect()
}

// ── other bodies (EHR_STATUS, directory, contribution) ─────────────────────────

/// Render an `EHR_STATUS` body carrying the patient's subject id (E1 create,
/// E9 status update). The corpus `EHR_STATUS` fixture is adapted to RM 1.2.0 by
/// the [`fixtures::adapt_ehr_status`] overlay; we only stamp the subject id +
/// namespace.
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
/// varied composition of `kind` (constraint-aware, differentiated by a per-slot
/// salt), and the committer name is the params composer.
///
/// # Errors
/// [`BenchError`] if the envelope or the composition cannot be read/rendered.
pub fn contribution(
    kind: TemplateKind,
    params: &VaryParams,
    n: usize,
) -> Result<Value, BenchError> {
    let n = n.clamp(1, 3);
    let text = fixtures::read_from("contribution.valid", CONTRIBUTION_ENVELOPE)
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
    let mut envelope: Value = serde_json::from_str(&text).map_err(BenchError::Json)?;

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
            obj.insert("data".to_owned(), render_prepared(kind, params, i as u64)?);
        }
        versions.push(version);
    }

    if let Some(obj) = envelope.as_object_mut() {
        obj.insert("versions".to_owned(), Value::Array(versions));
    }
    set_committer_name(&mut envelope, "audit", &params.composer);
    Ok(envelope)
}

/// Set the committer name inside an `AUDIT_DETAILS` field of `node`.
fn set_committer_name(node: &mut Value, audit_field: &str, name: &str) {
    if let Some(Value::String(s)) = node
        .get_mut(audit_field)
        .and_then(|a| a.get_mut("committer"))
        .and_then(|c| c.get_mut("name"))
    {
        name.clone_into(s);
    }
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
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
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

    /// The `WebTemplate` for a kind, for validating renders. `Flat` kinds reuse
    /// the prepared template; `Raw` (corpus) kinds rebuild it (their OPT builds a
    /// template — it is only the FLAT round-trip that is unfaithful).
    fn web_template(kind: TemplateKind) -> WebTemplate {
        match PREPARED.get(&kind).expect("prepared") {
            Prepared::Flat(wt, _) => (**wt).clone(),
            Prepared::Raw(_) => build_template(kind).expect("corpus OPT builds a template"),
            Prepared::Failed(msg) => panic!("{msg}"),
        }
    }

    fn skeleton(kind: TemplateKind) -> &'static Value {
        match PREPARED.get(&kind).expect("prepared") {
            Prepared::Flat(_, s) | Prepared::Raw(s) => s,
            Prepared::Failed(msg) => panic!("{msg}"),
        }
    }

    /// Every `DV_QUANTITY` magnitude in a composition (deep), in document order.
    fn magnitudes(v: &Value) -> Vec<f64> {
        fn rec(v: &Value, out: &mut Vec<f64>) {
            match v {
                Value::Object(m) => {
                    if m.get("_type").and_then(Value::as_str) == Some("DV_QUANTITY")
                        && let Some(mag) = m.get("magnitude").and_then(Value::as_f64)
                    {
                        out.push(mag);
                    }
                    for x in m.values() {
                        rec(x, out);
                    }
                }
                Value::Array(a) => {
                    for x in a {
                        rec(x, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        rec(v, &mut out);
        out
    }

    #[test]
    fn every_template_prepares() {
        // No CKM hard-failure; each kind is renderable.
        preflight().expect("all templates prepare");
        for kind in ALL_KINDS {
            let _ = composition(kind, &params()).expect("render");
        }
    }

    #[test]
    fn ckm_templates_render_via_the_constraint_aware_flat_path() {
        // The committed workload payloads must be constraint-aware (Flat), never
        // the raw fallback — that is the whole point of the fix.
        for tpl in pack::all() {
            assert!(
                matches!(PREPARED.get(&tpl.kind), Some(Prepared::Flat(..))),
                "CKM {} must render via the Flat path",
                tpl.slug
            );
        }
    }

    /// The core regression net: for ~100 distinct (seed, `event_time`, salt)
    /// inputs, a rendered payload introduces **no** validation message beyond
    /// its skeleton's own baseline; where the skeleton is itself clean, the
    /// render is clean too. This is the guarantee that variation can never
    /// produce the server-rejected (`422`) payloads that voided the earlier run.
    #[test]
    fn variation_never_introduces_a_validation_error() {
        let times = [
            "2024-06-01T00:00:00.000Z",
            "2024-06-01T06:30:15.500Z",
            "2024-06-01T08:15:00.000Z",
            "2024-06-01T12:45:30.250Z",
            "2024-06-01T14:30:00.000Z",
            "2024-06-01T18:20:59.999Z",
            "2024-06-01T21:05:45.100Z",
            "2024-06-01T23:59:59.000Z",
            "2024-06-01T03:03:03.030Z",
            "2024-06-01T16:16:16.160Z",
        ];
        for kind in ALL_KINDS {
            let wt = web_template(kind);
            let baseline = message_set(&validate_composition(skeleton(kind), &wt));
            for (si, &time) in times.iter().enumerate() {
                for salt in 0..10u64 {
                    let p = VaryParams {
                        subject_id: format!("bench-patient-{si:06}"),
                        composer: format!("Dr. Bench {salt}"),
                        event_time: time.to_owned(),
                        seed: 0x1234_5678u64
                            .wrapping_mul(si as u64 + 1)
                            .wrapping_add(salt),
                    };
                    let rendered =
                        render_prepared(kind, &p, salt).expect("render for validity net");
                    let msgs = validate_composition(&rendered, &wt);
                    for m in &msgs {
                        assert!(
                            baseline.contains(&message_signature(m)),
                            "{kind:?} render introduced a new validation message \
                             (time={time}, salt={salt}): {m:?}"
                        );
                    }
                    if baseline.is_empty() {
                        assert!(
                            msgs.is_empty(),
                            "{kind:?} render must validate clean (time={time}, salt={salt}): \
                             {msgs:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn render_is_deterministic() {
        for kind in ALL_KINDS {
            let a = composition(kind, &params()).expect("render");
            let b = composition(kind, &params()).expect("render");
            assert_eq!(a, b, "{kind:?}: same inputs must render identically");
        }
        // Determinism also through the salted contribution path.
        let c1 = contribution(TemplateKind::CkmLabResult, &params(), 3).expect("contribution");
        let c2 = contribution(TemplateKind::CkmLabResult, &params(), 3).expect("contribution");
        assert_eq!(c1, c2, "same inputs must produce an identical contribution");
    }

    #[test]
    fn jitter_actually_varies_numeric_leaves() {
        // Across CKM kinds (constraint-aware), a change of seed alone (fixed
        // subject/composer/event-time) must move at least one DV_QUANTITY
        // magnitude — proof the jitter is real, not silently disabled.
        let base = params();
        let mut other = params();
        other.seed = base.seed ^ 0xFFFF_FFFF;
        let varied = pack::all().iter().any(|tpl| {
            let a = composition(tpl.kind, &base).expect("render a");
            let b = composition(tpl.kind, &other).expect("render b");
            magnitudes(&a) != magnitudes(&b)
        });
        assert!(
            varied,
            "no CKM template varied a magnitude across seeds — jitter is not firing"
        );
    }

    #[test]
    fn jitter_stays_within_a_constrained_range() {
        // A synthetic proof that the clamp honours bounds: a value jittered from
        // near an inclusive max never exceeds it.
        let range = WebTemplateRange {
            min_op: None,
            min: Some(serde_json::json!(0.0)),
            max_op: None,
            max: Some(serde_json::json!(10.0)),
        };
        for raw in [-5.0, 0.0, 9.9, 10.0, 12.0, 100.0] {
            let v = constrain_decimal(5.0, raw, Some(&range));
            assert!(
                (0.0..=10.0).contains(&v),
                "clamped {raw} → {v} out of [0,10]"
            );
        }
        // Exclusive bound landing falls back to the valid original.
        let excl = WebTemplateRange {
            min_op: Some(">".to_owned()),
            min: Some(serde_json::json!(0.0)),
            max_op: None,
            max: None,
        };
        assert!((constrain_decimal(3.0, -1.0, Some(&excl)) - 3.0).abs() < 1e-12);
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
        let c = contribution(TemplateKind::CkmLabResult, &params(), 3).expect("contribution");
        assert_eq!(c["_type"], "CONTRIBUTION");
        let versions = c["versions"].as_array().expect("versions array");
        assert_eq!(versions.len(), 3);
        for v in versions {
            assert_eq!(v["_type"], "ORIGINAL_VERSION");
            let data = v.get("data").expect("each version wraps a composition");
            assert_eq!(
                data.get("_type").and_then(Value::as_str),
                Some("COMPOSITION")
            );
        }
        // Each version's composition is independently salted, so batches differ
        // whenever the template carries jitterable content. `CkmLabResult` here
        // does not (its single DV_QUANTITY skeleton magnitude is 0.0, and
        // 0.0 × factor = 0.0), so identical datas are correct — the salt path is
        // proven to differentiate separately in `salt_differentiates_a_batch`.
    }

    #[test]
    fn salt_differentiates_a_batch() {
        // The salt genuinely differentiates the compositions in a batch for a
        // template with jitterable magnitudes (proof the per-slot salt is wired,
        // independent of any single template's constrained content).
        let differentiated = pack::all().iter().any(|tpl| {
            let batch = contribution(tpl.kind, &params(), 3).expect("contribution");
            let datas: Vec<Value> = batch["versions"]
                .as_array()
                .expect("versions")
                .iter()
                .filter_map(|v| v.get("data").cloned())
                .collect();
            datas.windows(2).any(|w| w[0] != w[1])
        });
        assert!(
            differentiated,
            "no CKM contribution batch differed across salts — the per-slot salt is not wired"
        );
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
