//! The FHIR-connector **mapping definition** schema + the pure FHIR→FLAT
//! transform.
//!
//! A mapping definition binds one FHIR R4 resource profile to one openEHR
//! template. Its `entries` each read a value out of the incoming FHIR resource
//! (a **`FHIRPath`-lite** dot-path — see [`resolve`]) and write it to a
//! template-relative **openEHR FLAT path** (the Better/simSDT `id[:i]/…|suffix`
//! key `openehr-flat` consumes — `crates/openehr-flat/src/flat/sub.rs`). The
//! resulting flat map is handed to [`openehr_flat::from_flat`] with the
//! template's `WebTemplate` to build a canonical COMPOSITION, which then commits
//! through the platform's NORMAL validated path. This
//! module is protocol-free and DB-free: it is the deterministic transform, unit
//! tested here; the orchestration (mapping-store lookup, EHR resolution,
//! commit) lives in the parent [`super`] module on `EhrbaseService`.
//!
//! PORT NOTE: FHIR↔openEHR mapping is spec-silent, so this schema is
//! a design decision, not a transcription. The FHIR side is a deliberate
//! **subset** of `FHIRPath` — object-field navigation and array indexing only
//! (`code.coding[0].code`, `component[1].valueQuantity.value`) — NOT the full
//! `FHIRPath` language (no functions, filters, `where()`, `resolve()`, unions,
//! `$this`, arithmetic). That subset is sufficient for the flat, single-value
//! leaf extraction openEHR FLAT paths need; anything richer is out of the
//! starter scope. Cross-terminology *code-value* translation is likewise
//! deferred: `code_map` binds a FHIR system URL to an openEHR `terminology_id`
//! and passes the code through (the `TerminologyService` seam is where value
//! translation would plug in) — the built COMPOSITION's own terminology
//! validation is the authority on the result.

use std::collections::BTreeMap;

use ehrbase_sm::SubjectRef;
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// The `system_id` recorded in the built COMPOSITION's `FEEDER_AUDIT`
/// originating-system audit (RM common `FEEDER_AUDIT_DETAILS`), naming the
/// import channel.
pub(super) const ORIGINATING_SYSTEM: &str = "fhir-connector";

/// A validated FHIR→openEHR mapping definition (the `definition` JSON stored in
/// `fhir_mapping.definition`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FhirMappingDefinition {
    /// The FHIR resource type this mapping consumes (`Observation`, `Patient`, …).
    pub resource_type: String,
    /// The FHIR profile canonical URL this mapping binds (matched against the
    /// resource's `meta.profile`). Absent = the default mapping for the type.
    #[serde(default)]
    pub profile_url: Option<String>,
    /// The openEHR template (OPT) the built COMPOSITION targets.
    pub template_id: String,
    /// How to resolve the target EHR's subject from the FHIR resource.
    pub subject: SubjectMapping,
    /// Seed FLAT keys written before the entries (composition context defaults:
    /// `ctx/language`, `ctx/territory`, `ctx/composer_name`, `ctx/time`, …). An
    /// entry may overwrite a seed key.
    #[serde(default)]
    pub context: Map<String, Value>,
    /// The field-binding entries.
    pub entries: Vec<MappingEntry>,
}

/// How the connector resolves the target EHR's subject from the resource.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SubjectMapping {
    /// `FHIRPath`-lite path to the subject identifier string (e.g.
    /// `subject.reference` on an Observation, or `id` on a Patient).
    pub reference_path: String,
    /// The openEHR subject-id namespace to record on the resolved EHR.
    pub namespace: String,
    /// An optional prefix to strip from the resolved reference (e.g.
    /// `Patient/` → the bare logical id).
    #[serde(default)]
    pub strip_prefix: Option<String>,
}

/// One field binding: read `fhir_path` (or a `constant`) out of the resource and
/// write it to `openehr_path` (a template-relative FLAT key), shaped by
/// `transform`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MappingEntry {
    /// The template-relative openEHR FLAT path (the `id[:i]/…` key; a
    /// value-shaping transform appends the `|suffix`).
    pub openehr_path: String,
    /// The `FHIRPath`-lite source path (omitted for a `constant`).
    #[serde(default)]
    pub fhir_path: Option<String>,
    /// A literal value to write verbatim to `openehr_path` (ignores `fhir_path`).
    #[serde(default)]
    pub constant: Option<Value>,
    /// The value-shaping transform (default: plain text/scalar).
    #[serde(default)]
    pub transform: Transform,
    /// For a `coded` transform: FHIR code-system URL → openEHR `terminology_id`.
    /// The key `*` is the fallback for any unmatched system.
    #[serde(default)]
    pub code_map: BTreeMap<String, String>,
    /// When true, an absent source value is an error (else the entry is skipped).
    #[serde(default)]
    pub required: bool,
}

/// How an entry's source value is shaped into FLAT leaf(s).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum Transform {
    /// Plain scalar → the bare FLAT key (`DV_TEXT` value, or any bare leaf).
    #[default]
    Text,
    /// An ISO-8601 date/date-time string → the bare FLAT key (`DV_DATE_TIME` etc.).
    Date,
    /// A quantity: the source number → `{openehr_path}|magnitude`, the unit →
    /// `{openehr_path}|unit` (from `unit_path` in the resource, or the literal
    /// `unit`).
    Quantity {
        /// `FHIRPath`-lite path to the unit string.
        #[serde(default)]
        unit_path: Option<String>,
        /// A literal unit string (wins over `unit_path`).
        #[serde(default)]
        unit: Option<String>,
    },
    /// A coded value: the source code → `{openehr_path}|code`, the resolved
    /// `terminology_id` → `{openehr_path}|terminology`, and (optional) the
    /// display → `{openehr_path}|value`.
    Coded {
        /// `FHIRPath`-lite path to the code's system URL (looked up in `code_map`).
        #[serde(default)]
        system_path: Option<String>,
        /// `FHIRPath`-lite path to the human-readable display text.
        #[serde(default)]
        display_path: Option<String>,
    },
}

/// A FHIR→openEHR mapping transform failure.
#[derive(Debug, thiserror::Error)]
pub(super) enum FhirMapError {
    /// A required source field was absent (or not a scalar where one was
    /// expected).
    #[error("required FHIR field '{fhir_path}' (→ '{openehr_path}') is absent")]
    MissingField {
        /// The `FHIRPath`-lite source path.
        fhir_path: String,
        /// The FLAT target path.
        openehr_path: String,
    },
    /// A source value was not the scalar kind the transform needs.
    #[error("FHIR field '{fhir_path}' (→ '{openehr_path}') is not a {expected}")]
    WrongType {
        /// The `FHIRPath`-lite source path.
        fhir_path: String,
        /// The FLAT target path.
        openehr_path: String,
        /// The expected scalar kind (`number`, `string`, …).
        expected: &'static str,
    },
    /// The resource carried no subject reference at the configured path.
    #[error("FHIR resource has no subject reference at '{0}'")]
    MissingSubject(String),
    /// A `coded` entry was declared without a `fhir_path`.
    #[error("coded mapping entry for '{0}' has no fhir_path")]
    CodedWithoutSource(String),
    /// The reverse transform could not flatten the COMPOSITION (its
    /// `WebTemplate` walk failed) — a server-side error (the stored
    /// COMPOSITION should always flatten).
    #[error("could not flatten COMPOSITION for reverse mapping: {0}")]
    Reverse(String),
}

/// Resolve a **`FHIRPath`-lite** dot-path against a JSON value.
///
/// Grammar (a deliberate subset of `FHIRPath` — see the module PORT NOTE):
///
/// ```text
/// path    := segment ('.' segment)*
/// segment := name index? | index
/// name    := [A-Za-z_][A-Za-z0-9_]*
/// index   := '[' digits ']'
/// ```
///
/// A `name` selects an object field; an `index` selects an array element. So
/// `component[0].valueQuantity.value`, `code.coding[0].code`,
/// `subject.reference`, and `meta.profile[0]` all resolve. Anything richer
/// (functions, filters, unions) is out of scope and yields `None`.
pub(super) fn resolve<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for seg in path.split('.') {
        let (name, index) = parse_segment(seg)?;
        if !name.is_empty() {
            cur = cur.get(name)?;
        }
        if let Some(i) = index {
            cur = cur.get(i)?;
        }
    }
    Some(cur)
}

/// Split one path segment into its `(name, index)` parts, e.g. `component[0]`
/// → `("component", Some(0))`, `reference` → `("reference", None)`. Returns
/// `None` for a malformed segment (unbalanced/broken index).
fn parse_segment(seg: &str) -> Option<(&str, Option<usize>)> {
    match seg.split_once('[') {
        None => Some((seg, None)),
        Some((name, rest)) => {
            let digits = rest.strip_suffix(']')?;
            let idx = digits.parse::<usize>().ok()?;
            Some((name, Some(idx)))
        }
    }
}

/// Build the FLAT map for a resource under a mapping definition: the seed
/// `context` keys, then each entry's produced leaf(s).
pub(super) fn build_flat(
    resource: &Value,
    def: &FhirMappingDefinition,
) -> Result<Map<String, Value>, FhirMapError> {
    let mut flat = Map::new();
    for (key, value) in &def.context {
        flat.insert(key.clone(), value.clone());
    }
    for entry in &def.entries {
        apply_entry(resource, entry, &mut flat)?;
    }
    Ok(flat)
}

/// Apply one entry, writing zero or more FLAT leaves into `flat`.
fn apply_entry(
    resource: &Value,
    entry: &MappingEntry,
    flat: &mut Map<String, Value>,
) -> Result<(), FhirMapError> {
    // A constant short-circuits any source read (any transform).
    if let Some(c) = &entry.constant {
        flat.insert(entry.openehr_path.clone(), c.clone());
        return Ok(());
    }
    match &entry.transform {
        Transform::Text | Transform::Date => {
            if let Some(v) = source(resource, entry)? {
                flat.insert(entry.openehr_path.clone(), scalar(v, entry)?);
            }
        }
        Transform::Quantity { unit_path, unit } => {
            if let Some(v) = source(resource, entry)? {
                let magnitude = number(v, entry)?;
                flat.insert(format!("{}|magnitude", entry.openehr_path), magnitude);
            }
            let unit_val = unit.clone().or_else(|| {
                unit_path
                    .as_deref()
                    .and_then(|p| resolve(resource, p))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
            if let Some(u) = unit_val {
                flat.insert(format!("{}|unit", entry.openehr_path), Value::String(u));
            }
        }
        Transform::Coded {
            system_path,
            display_path,
        } => {
            let fhir_path = entry
                .fhir_path
                .as_deref()
                .ok_or_else(|| FhirMapError::CodedWithoutSource(entry.openehr_path.clone()))?;
            let Some(code) = resolve(resource, fhir_path).and_then(Value::as_str) else {
                if entry.required {
                    return Err(FhirMapError::MissingField {
                        fhir_path: fhir_path.to_owned(),
                        openehr_path: entry.openehr_path.clone(),
                    });
                }
                return Ok(());
            };
            flat.insert(format!("{}|code", entry.openehr_path), json!(code));
            let system = system_path
                .as_deref()
                .and_then(|p| resolve(resource, p))
                .and_then(Value::as_str);
            let terminology = system
                .and_then(|s| entry.code_map.get(s))
                .or_else(|| entry.code_map.get("*"))
                .map_or("local", String::as_str);
            flat.insert(
                format!("{}|terminology", entry.openehr_path),
                json!(terminology),
            );
            if let Some(dp) = display_path
                && let Some(display) = resolve(resource, dp).and_then(Value::as_str)
            {
                flat.insert(format!("{}|value", entry.openehr_path), json!(display));
            }
        }
    }
    Ok(())
}

/// Resolve an entry's source value, honouring `required`.
fn source<'a>(
    resource: &'a Value,
    entry: &MappingEntry,
) -> Result<Option<&'a Value>, FhirMapError> {
    let Some(fhir_path) = entry.fhir_path.as_deref() else {
        return Ok(None);
    };
    match resolve(resource, fhir_path) {
        Some(v) => Ok(Some(v)),
        None if entry.required => Err(FhirMapError::MissingField {
            fhir_path: fhir_path.to_owned(),
            openehr_path: entry.openehr_path.clone(),
        }),
        None => Ok(None),
    }
}

/// Coerce a source value to a scalar FLAT value (string / number / bool); reject
/// objects and arrays (a FLAT leaf is a single value).
fn scalar(v: &Value, entry: &MappingEntry) -> Result<Value, FhirMapError> {
    match v {
        Value::String(_) | Value::Number(_) | Value::Bool(_) => Ok(v.clone()),
        _ => Err(FhirMapError::WrongType {
            fhir_path: entry.fhir_path.clone().unwrap_or_default(),
            openehr_path: entry.openehr_path.clone(),
            expected: "scalar",
        }),
    }
}

/// Coerce a source value to a JSON number (FLAT `|magnitude`); accept a numeric
/// string too (FHIR sometimes serialises decimals as strings).
fn number(v: &Value, entry: &MappingEntry) -> Result<Value, FhirMapError> {
    if v.is_number() {
        return Ok(v.clone());
    }
    if let Some(s) = v.as_str()
        && let Ok(n) = s.parse::<f64>()
        && let Some(num) = serde_json::Number::from_f64(n)
    {
        return Ok(Value::Number(num));
    }
    Err(FhirMapError::WrongType {
        fhir_path: entry.fhir_path.clone().unwrap_or_default(),
        openehr_path: entry.openehr_path.clone(),
        expected: "number",
    })
}

/// Extract the target EHR subject from the resource per the mapping's
/// `subject` rule.
pub(super) fn extract_subject(
    resource: &Value,
    def: &FhirMappingDefinition,
) -> Result<SubjectRef, FhirMapError> {
    let raw = resolve(resource, &def.subject.reference_path)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| FhirMapError::MissingSubject(def.subject.reference_path.clone()))?;
    let id = def
        .subject
        .strip_prefix
        .as_deref()
        .and_then(|p| raw.strip_prefix(p))
        .unwrap_or(raw);
    Ok(SubjectRef::person(
        id.to_owned(),
        def.subject.namespace.clone(),
    ))
}

/// The resource's logical id (`id`), or a non-empty fallback (`DV_IDENTIFIER`'s
/// `Id_valid` invariant forbids an empty id).
pub(super) fn resource_id(resource: &Value, resource_type: &str) -> String {
    resolve(resource, "id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("{resource_type}/unknown"), str::to_owned)
}

/// The resource version (`meta.versionId`), if present.
pub(super) fn resource_version(resource: &Value) -> Option<String> {
    resolve(resource, "meta.versionId")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Build the `FEEDER_AUDIT` (canonical JSON) recording the FHIR import trail
///: originating system `fhir-connector`, the resource
/// type/id as an originating-system item id, and the resource version + import
/// time on the originating-system audit (RM common `FEEDER_AUDIT_DETAILS`).
pub(super) fn feeder_audit(
    resource_type: &str,
    resource_id: &str,
    version_id: Option<&str>,
    time_iso: &str,
) -> Value {
    let mut details = json!({
        "_type": "FEEDER_AUDIT_DETAILS",
        "system_id": ORIGINATING_SYSTEM,
        "time": { "_type": "DV_DATE_TIME", "value": time_iso },
    });
    if let Some(v) = version_id {
        details["version_id"] = json!(v);
    }
    json!({
        "_type": "FEEDER_AUDIT",
        "originating_system_item_ids": [
            { "_type": "DV_IDENTIFIER", "id": resource_id, "type": resource_type, "issuer": "FHIR" }
        ],
        "originating_system_audit": details,
    })
}

/// Attach a `FEEDER_AUDIT` to a canonical-JSON COMPOSITION object.
pub(super) fn inject_feeder_audit(comp: &mut Value, feeder_audit: Value) {
    if let Value::Object(m) = comp {
        m.insert("feeder_audit".to_owned(), feeder_audit);
    }
}

// ── reverse mapping: canonical COMPOSITION → FHIR resource ──
//
// The exact inverse of [`build_flat`]: the COMPOSITION is flattened to the same
// simSDT FLAT map [`from_flat`] consumes (via [`openehr_flat::to_flat`], so the
// leaf keys — `path|magnitude`, `path|unit`, `path|code`, `path|terminology`,
// `path|value` — are byte-identical to what an entry wrote inbound), then each
// mapping entry reads its leaf(s) back out and writes them to its FHIRPath-lite
// target, building the FHIR JSON. `code_map` is applied in reverse
// (`terminology_id` → FHIR system URL). The read façade and the outbound emitter
// share this transform.
//
// PORT NOTE: a `constant` entry is NOT reversed — it injected a fixed
// openEHR leaf inbound with no FHIR source, so it contributes nothing to the
// reconstructed resource (round-trip fidelity is defined over the FHIR-sourced
// mapped fields). The `subject` is reconstructed from the owning EHR's subject
// id (the façade/emitter supply it — the COMPOSITION does not carry it), with
// `strip_prefix` re-applied so the reference matches the inbound form.

/// Reverse [`build_flat`]: build the FHIR resource for a COMPOSITION under a
/// mapping definition. `subject_id` is the owning EHR's external subject id
/// (placed back at the mapping's `subject.reference_path`, `strip_prefix`
/// re-applied); `None` omits the subject.
pub(super) fn to_fhir(
    resource_type: &str,
    composition: &Value,
    wt: &openehr_flat::WebTemplate,
    def: &FhirMappingDefinition,
    subject_id: Option<&str>,
) -> Result<Value, FhirMapError> {
    let flat =
        openehr_flat::to_flat(composition, wt).map_err(|e| FhirMapError::Reverse(e.to_string()))?;
    let mut resource = json!({ "resourceType": resource_type });
    if let Some(sid) = subject_id {
        let reference = match &def.subject.strip_prefix {
            Some(prefix) => format!("{prefix}{sid}"),
            None => sid.to_owned(),
        };
        set_at(
            &mut resource,
            &def.subject.reference_path,
            Value::String(reference),
        );
    }
    for entry in &def.entries {
        // A constant has no FHIR source (see the module PORT NOTE).
        if entry.constant.is_some() {
            continue;
        }
        reverse_entry(&flat, entry, &mut resource);
    }
    Ok(resource)
}

/// Reverse one entry: read its FLAT leaf(s) and write them to the FHIRPath-lite
/// target(s). Absent leaves are simply not written (a partially-populated
/// COMPOSITION yields a partially-populated resource — the inverse of inbound's
/// skip-when-absent).
fn reverse_entry(flat: &impl FlatLookup, entry: &MappingEntry, resource: &mut Value) {
    let Some(fhir_path) = entry.fhir_path.as_deref() else {
        return;
    };
    match &entry.transform {
        Transform::Text | Transform::Date => {
            if let Some(v) = flat.lookup(&entry.openehr_path) {
                set_at(resource, fhir_path, v.clone());
            }
        }
        Transform::Quantity { unit_path, .. } => {
            if let Some(v) = flat.lookup(&format!("{}|magnitude", entry.openehr_path)) {
                set_at(resource, fhir_path, v.clone());
            }
            // A literal `unit` had no FHIR source, so only a `unit_path` is
            // reversed (the inverse of inbound: literal wins over path there).
            if let Some(up) = unit_path
                && let Some(u) = flat.lookup(&format!("{}|unit", entry.openehr_path))
            {
                set_at(resource, up, u.clone());
            }
        }
        Transform::Coded {
            system_path,
            display_path,
        } => {
            if let Some(code) = flat.lookup(&format!("{}|code", entry.openehr_path)) {
                set_at(resource, fhir_path, code.clone());
            }
            if let Some(sp) = system_path
                && let Some(term) = flat
                    .lookup(&format!("{}|terminology", entry.openehr_path))
                    .and_then(Value::as_str)
                && let Some(system) = reverse_code_map(&entry.code_map, term)
            {
                set_at(resource, sp, Value::String(system));
            }
            if let Some(dp) = display_path
                && let Some(display) = flat.lookup(&format!("{}|value", entry.openehr_path))
            {
                set_at(resource, dp, display.clone());
            }
        }
    }
}

/// Reverse `code_map` (FHIR system URL → openEHR `terminology_id`): given a
/// `terminology_id`, recover the FHIR system that mapped to it. The `*`
/// wildcard is skipped (it is lossy — many systems fold to one `terminology_id`
/// through it, so the original cannot be recovered); `BTreeMap` iteration order
/// makes the choice deterministic when several systems share a `terminology_id`.
fn reverse_code_map(code_map: &BTreeMap<String, String>, terminology: &str) -> Option<String> {
    code_map
        .iter()
        .find(|(system, term)| system.as_str() != "*" && term.as_str() == terminology)
        .map(|(system, _)| system.clone())
}

/// A minimal FLAT-map lookup seam so [`reverse_entry`] can be exercised against
/// both [`openehr_flat::to_flat`]'s map and a plain map in unit tests without
/// naming the crate-private `FlatMap` alias.
trait FlatLookup {
    /// The value stored at the exact FLAT key, if any.
    fn lookup(&self, key: &str) -> Option<&Value>;
}

impl FlatLookup for indexmap::IndexMap<String, Value> {
    fn lookup(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
}

impl FlatLookup for Map<String, Value> {
    fn lookup(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
}

/// Write `value` into `root` at a **`FHIRPath`-lite** path (the same grammar
/// [`resolve`] reads), materialising intermediate objects/arrays as it descends
/// — the write-side inverse of [`resolve`]. A malformed segment aborts silently
/// (the entry simply does not contribute), matching inbound's tolerant skip.
fn set_at(root: &mut Value, path: &str, value: Value) {
    let segments: Option<Vec<(&str, Option<usize>)>> = path.split('.').map(parse_segment).collect();
    if let Some(segments) = segments {
        place(root, &segments, value);
    }
}

/// Descend `segments` from `cur`, creating objects/arrays as needed, and write
/// `value` at the leaf.
fn place(cur: &mut Value, segments: &[(&str, Option<usize>)], value: Value) {
    let Some(((name, index), rest)) = segments.split_first() else {
        *cur = value;
        return;
    };
    // A named segment selects (creating if absent) an object field.
    let slot: &mut Value = if name.is_empty() {
        cur
    } else {
        if !cur.is_object() {
            *cur = Value::Object(Map::new());
        }
        match cur.as_object_mut() {
            Some(obj) => obj.entry((*name).to_owned()).or_insert(Value::Null),
            None => return,
        }
    };
    match index {
        // An indexed segment selects (creating + null-padding) an array element.
        Some(i) => {
            if !slot.is_array() {
                *slot = Value::Array(Vec::new());
            }
            let Some(arr) = slot.as_array_mut() else {
                return;
            };
            while arr.len() <= *i {
                arr.push(Value::Null);
            }
            place(&mut arr[*i], rest, value);
        }
        None => place(slot, rest, value),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use openehr_flat::{WebTemplate, build_web_template, from_flat};
    use openehr_its::opt14;
    use serde_json::json;

    use super::*;

    fn def(v: Value) -> FhirMappingDefinition {
        serde_json::from_value(v).expect("valid mapping definition")
    }

    // ── `FHIRPath`-lite resolver ───────────────────────────────────────────────
    #[test]
    fn resolve_navigates_fields_and_indices() {
        let r = json!({
            "resourceType": "Observation",
            "code": { "coding": [{ "system": "http://loinc.org", "code": "8480-6" }] },
            "component": [
                { "valueQuantity": { "value": 120, "unit": "mm[Hg]" } },
                { "valueQuantity": { "value": 80, "unit": "mm[Hg]" } }
            ],
            "meta": { "profile": ["http://example.org/bp"] }
        });
        assert_eq!(
            resolve(&r, "code.coding[0].code").and_then(Value::as_str),
            Some("8480-6")
        );
        assert_eq!(
            resolve(&r, "component[1].valueQuantity.value").and_then(Value::as_i64),
            Some(80)
        );
        assert_eq!(
            resolve(&r, "meta.profile[0]").and_then(Value::as_str),
            Some("http://example.org/bp")
        );
        assert!(resolve(&r, "component[5].valueQuantity").is_none());
        assert!(resolve(&r, "missing.field").is_none());
    }

    // ── build_flat: quantity + constant + context ────────────────────────────
    #[test]
    fn build_flat_observation_quantities() {
        let d = def(json!({
            "resource_type": "Observation",
            "template_id": "ehrbase_blood_pressure_simple.de.v0",
            "subject": { "reference_path": "subject.reference", "namespace": "fhir", "strip_prefix": "Patient/" },
            "context": { "ctx/language": "en", "ctx/territory": "US", "ctx/composer_name": "fhir-connector" },
            "entries": [
                { "openehr_path": "encounter_training_sample/blood_pressure_training_sample:0/systolic",
                  "fhir_path": "component[0].valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "component[0].valueQuantity.unit" } },
                { "openehr_path": "encounter_training_sample/blood_pressure_training_sample:0/diastolic",
                  "fhir_path": "component[1].valueQuantity.value",
                  "transform": { "kind": "quantity", "unit": "mm[Hg]" } },
                { "openehr_path": "ctx/time", "fhir_path": "effectiveDateTime", "transform": { "kind": "date" } }
            ]
        }));
        let resource = json!({
            "resourceType": "Observation",
            "subject": { "reference": "Patient/p-1" },
            "effectiveDateTime": "2026-02-03T04:05:06Z",
            "component": [
                { "valueQuantity": { "value": 120, "unit": "mm[Hg]" } },
                { "valueQuantity": { "value": 80 } }
            ]
        });
        let flat = build_flat(&resource, &d).expect("build_flat");
        assert_eq!(
            flat["encounter_training_sample/blood_pressure_training_sample:0/systolic|magnitude"],
            json!(120)
        );
        assert_eq!(
            flat["encounter_training_sample/blood_pressure_training_sample:0/systolic|unit"],
            json!("mm[Hg]")
        );
        assert_eq!(
            flat["encounter_training_sample/blood_pressure_training_sample:0/diastolic|magnitude"],
            json!(80)
        );
        assert_eq!(
            flat["encounter_training_sample/blood_pressure_training_sample:0/diastolic|unit"],
            json!("mm[Hg]")
        );
        assert_eq!(flat["ctx/time"], json!("2026-02-03T04:05:06Z"));
        assert_eq!(flat["ctx/language"], json!("en"));
    }

    // ── build_flat: coded value via code_map (terminology seam) ──────────────
    #[test]
    fn build_flat_coded_uses_code_map() {
        let d = def(json!({
            "resource_type": "Condition",
            "template_id": "t",
            "subject": { "reference_path": "subject.reference", "namespace": "fhir" },
            "entries": [
                { "openehr_path": "x/problem",
                  "fhir_path": "code.coding[0].code",
                  "transform": { "kind": "coded", "system_path": "code.coding[0].system", "display_path": "code.coding[0].display" },
                  "code_map": { "http://snomed.info/sct": "SNOMED-CT" } }
            ]
        }));
        let resource = json!({
            "resourceType": "Condition",
            "code": { "coding": [{ "system": "http://snomed.info/sct", "code": "73211009", "display": "Diabetes mellitus" }] }
        });
        let flat = build_flat(&resource, &d).expect("build_flat");
        assert_eq!(flat["x/problem|code"], json!("73211009"));
        assert_eq!(flat["x/problem|terminology"], json!("SNOMED-CT"));
        assert_eq!(flat["x/problem|value"], json!("Diabetes mellitus"));
    }

    #[test]
    fn build_flat_coded_unmapped_system_defaults_local() {
        let d = def(json!({
            "resource_type": "Condition", "template_id": "t",
            "subject": { "reference_path": "id", "namespace": "fhir" },
            "entries": [
                { "openehr_path": "x/problem", "fhir_path": "code.coding[0].code",
                  "transform": { "kind": "coded", "system_path": "code.coding[0].system" } }
            ]
        }));
        let resource =
            json!({ "code": { "coding": [{ "system": "http://unknown", "code": "z" }] } });
        let flat = build_flat(&resource, &d).expect("build_flat");
        assert_eq!(flat["x/problem|terminology"], json!("local"));
    }

    #[test]
    fn required_missing_field_errors() {
        let d = def(json!({
            "resource_type": "Observation", "template_id": "t",
            "subject": { "reference_path": "id", "namespace": "fhir" },
            "entries": [ { "openehr_path": "x/y", "fhir_path": "absent.here", "required": true } ]
        }));
        let err = build_flat(&json!({}), &d).unwrap_err();
        assert!(matches!(err, FhirMapError::MissingField { .. }));
    }

    #[test]
    fn extract_subject_strips_prefix() {
        let d = def(json!({
            "resource_type": "Observation", "template_id": "t",
            "subject": { "reference_path": "subject.reference", "namespace": "fhir", "strip_prefix": "Patient/" },
            "entries": []
        }));
        let s = extract_subject(&json!({ "subject": { "reference": "Patient/abc" } }), &d)
            .expect("subject");
        assert_eq!(s.id, "abc");
        assert_eq!(s.namespace, "fhir");
        assert_eq!(s.r#type, "PERSON");
    }

    #[test]
    fn extract_subject_absent_errors() {
        let d = def(json!({
            "resource_type": "Observation", "template_id": "t",
            "subject": { "reference_path": "subject.reference", "namespace": "fhir" },
            "entries": []
        }));
        assert!(matches!(
            extract_subject(&json!({}), &d),
            Err(FhirMapError::MissingSubject(_))
        ));
    }

    #[test]
    fn feeder_audit_shape() {
        let fa = feeder_audit("Observation", "obs-1", Some("2"), "2026-07-11T00:00:00Z");
        assert_eq!(fa["_type"], json!("FEEDER_AUDIT"));
        assert_eq!(
            fa["originating_system_audit"]["system_id"],
            json!(ORIGINATING_SYSTEM)
        );
        assert_eq!(fa["originating_system_audit"]["version_id"], json!("2"));
        assert_eq!(fa["originating_system_item_ids"][0]["id"], json!("obs-1"));
        assert_eq!(
            fa["originating_system_item_ids"][0]["type"],
            json!("Observation")
        );
    }

    // ── End-to-end: FHIR → FLAT → canonical COMPOSITION ──────────────────────
    // Justification for the corpus template: `ehrbase_blood_pressure_simple.de.v0`
    // (vendored under app/ehrbase/tests/resources) is a small, single-OBSERVATION
    // blood-pressure template whose flat json-ids (systolic/diastolic
    // magnitude+unit) map cleanly to a FHIR BP Observation's components — the
    // canonical starter mapping. The FLAT keys used here were taken from
    // `to_flat(example_composition(wt))` for this OPT, so they are exactly the
    // template's committable json-id leaves.
    fn bp_web_template() -> WebTemplate {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/resources/service/knowledge/opt/ehrbase_blood_pressure_simple.de.v0.opt");
        let xml = std::fs::read_to_string(path).expect("read opt");
        let opt = opt14::from_xml(&xml).expect("parse opt");
        build_web_template(&opt).expect("build wt")
    }

    #[test]
    fn observation_maps_to_canonical_composition_with_feeder_audit() {
        let wt = bp_web_template();
        let d = def(json!({
            "resource_type": "Observation",
            "template_id": "ehrbase_blood_pressure_simple.de.v0",
            "subject": { "reference_path": "subject.reference", "namespace": "fhir", "strip_prefix": "Patient/" },
            "context": {
                "ctx/language": "en", "ctx/territory": "US",
                "ctx/composer_name": "fhir-connector", "ctx/time": "2026-02-03T04:05:06Z"
            },
            "entries": [
                { "openehr_path": "encounter_training_sample/blood_pressure_training_sample:0/systolic",
                  "fhir_path": "component[0].valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "component[0].valueQuantity.unit" } },
                { "openehr_path": "encounter_training_sample/blood_pressure_training_sample:0/diastolic",
                  "fhir_path": "component[1].valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "component[1].valueQuantity.unit" } }
            ]
        }));
        let resource = json!({
            "resourceType": "Observation",
            "id": "bp-1",
            "meta": { "versionId": "3" },
            "subject": { "reference": "Patient/p-1" },
            "component": [
                { "valueQuantity": { "value": 118, "unit": "mm[Hg]" } },
                { "valueQuantity": { "value": 76, "unit": "mm[Hg]" } }
            ]
        });

        let flat = build_flat(&resource, &d).expect("build_flat");
        let mut comp = from_flat(&flat, &wt).expect("from_flat builds a composition");
        inject_feeder_audit(
            &mut comp,
            feeder_audit(
                "Observation",
                &resource_id(&resource, "Observation"),
                resource_version(&resource).as_deref(),
                "2026-02-03T04:05:06Z",
            ),
        );

        // The result is a canonical COMPOSITION that deserialises as openehr-rm.
        assert_eq!(comp["_type"], json!("COMPOSITION"));
        let parsed: openehr_rm::composition::composition::Composition =
            serde_json::from_value(comp.clone()).expect("deserialises as RM Composition");
        assert!(
            parsed.feeder_audit.is_some(),
            "FEEDER_AUDIT present on the composition"
        );
        assert_eq!(
            comp["feeder_audit"]["originating_system_audit"]["system_id"],
            json!(ORIGINATING_SYSTEM)
        );
        // The mapped systolic magnitude survives the FLAT→RM build.
        let s = comp.to_string();
        assert!(s.contains("118"), "systolic magnitude present");
        assert!(s.contains("76"), "diastolic magnitude present");
    }

    // ── reverse mapping: set_at + reverse_entry + reverse_code_map ───────────
    #[test]
    fn set_at_builds_nested_objects_and_arrays() {
        let mut r = json!({ "resourceType": "Observation" });
        set_at(&mut r, "component[1].valueQuantity.value", json!(80));
        set_at(&mut r, "component[0].valueQuantity.value", json!(120));
        set_at(&mut r, "subject.reference", json!("Patient/p-1"));
        // Array is null-padded to the highest index then filled.
        assert_eq!(r["component"][0]["valueQuantity"]["value"], json!(120));
        assert_eq!(r["component"][1]["valueQuantity"]["value"], json!(80));
        assert_eq!(r["subject"]["reference"], json!("Patient/p-1"));
    }

    #[test]
    fn reverse_code_map_recovers_system_and_skips_wildcard() {
        let mut m = BTreeMap::new();
        m.insert("http://snomed.info/sct".to_owned(), "SNOMED-CT".to_owned());
        m.insert("*".to_owned(), "SNOMED-CT".to_owned());
        assert_eq!(
            reverse_code_map(&m, "SNOMED-CT").as_deref(),
            Some("http://snomed.info/sct")
        );
        // A terminology only reachable via the lossy `*` fallback is not
        // recovered (the original system is unknowable).
        let mut only_wild = BTreeMap::new();
        only_wild.insert("*".to_owned(), "local".to_owned());
        assert_eq!(reverse_code_map(&only_wild, "local"), None);
    }

    #[test]
    fn reverse_entry_coded_places_code_system_and_display() {
        let entry: MappingEntry = serde_json::from_value(json!({
            "openehr_path": "x/problem",
            "fhir_path": "code.coding[0].code",
            "transform": { "kind": "coded", "system_path": "code.coding[0].system", "display_path": "code.coding[0].display" },
            "code_map": { "http://snomed.info/sct": "SNOMED-CT" }
        }))
        .expect("entry");
        let mut flat = Map::new();
        flat.insert("x/problem|code".to_owned(), json!("73211009"));
        flat.insert("x/problem|terminology".to_owned(), json!("SNOMED-CT"));
        flat.insert("x/problem|value".to_owned(), json!("Diabetes mellitus"));
        let mut r = json!({ "resourceType": "Condition" });
        reverse_entry(&flat, &entry, &mut r);
        assert_eq!(r["code"]["coding"][0]["code"], json!("73211009"));
        assert_eq!(
            r["code"]["coding"][0]["system"],
            json!("http://snomed.info/sct")
        );
        assert_eq!(
            r["code"]["coding"][0]["display"],
            json!("Diabetes mellitus")
        );
    }

    // ── Full round trip: FHIR → build → reverse → equals the original mapped
    // fields. Uses the same BP template + mapping as the
    // inbound end-to-end test above.
    #[test]
    fn reverse_round_trip_equals_original_mapped_fields() {
        let wt = bp_web_template();
        let d = def(json!({
            "resource_type": "Observation",
            "template_id": "ehrbase_blood_pressure_simple.de.v0",
            "subject": { "reference_path": "subject.reference", "namespace": "fhir", "strip_prefix": "Patient/" },
            "context": {
                "ctx/language": "en", "ctx/territory": "US",
                "ctx/composer_name": "fhir-connector", "ctx/time": "2026-02-03T04:05:06Z"
            },
            "entries": [
                { "openehr_path": "encounter_training_sample/blood_pressure_training_sample:0/systolic",
                  "fhir_path": "component[0].valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "component[0].valueQuantity.unit" } },
                { "openehr_path": "encounter_training_sample/blood_pressure_training_sample:0/diastolic",
                  "fhir_path": "component[1].valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "component[1].valueQuantity.unit" } }
            ]
        }));
        let original = json!({
            "resourceType": "Observation",
            "subject": { "reference": "Patient/p-1" },
            "component": [
                { "valueQuantity": { "value": 118, "unit": "mm[Hg]" } },
                { "valueQuantity": { "value": 76, "unit": "mm[Hg]" } }
            ]
        });

        // FHIR → inbound build → COMPOSITION.
        let flat = build_flat(&original, &d).expect("build_flat");
        let comp = from_flat(&flat, &wt).expect("from_flat");

        // COMPOSITION → reverse → FHIR (subject id is the post-strip subject).
        let reversed = to_fhir("Observation", &comp, &wt, &d, Some("p-1")).expect("reverse maps");

        // Equal to the original on every mapped field (magnitude compared
        // numerically — the FLAT round trip may re-render 118 as 118.0).
        let num = |v: &Value, ptr: &str| v.pointer(ptr).and_then(Value::as_f64);
        assert_eq!(
            num(&reversed, "/component/0/valueQuantity/value"),
            Some(118.0)
        );
        assert_eq!(
            num(&reversed, "/component/1/valueQuantity/value"),
            Some(76.0)
        );
        assert_eq!(
            reversed.pointer("/component/0/valueQuantity/unit"),
            Some(&json!("mm[Hg]"))
        );
        assert_eq!(
            reversed.pointer("/component/1/valueQuantity/unit"),
            Some(&json!("mm[Hg]"))
        );
        assert_eq!(
            reversed.pointer("/subject/reference"),
            Some(&json!("Patient/p-1")),
            "subject reconstructed with strip_prefix re-applied"
        );
    }
}
