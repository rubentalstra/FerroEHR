// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The FHIR-connector **mapping definition** schema + the pure FHIR→FLAT
//! transform.
//!
//! **No openEHR spec governs this — our own design/extension.** FHIR↔openEHR
//! mapping is spec-silent, so this schema is a design decision, not a
//! transcription; master14's integration model maps *integration archetypes* →
//! *designed archetypes* via `GENERIC_ENTRY`, whereas this connector maps
//! directly to *designed* templates (mapping-as-data) — a different,
//! spec-silent mechanism. The `FEEDER_AUDIT` builder (the one RM-typed part)
//! lives in [`super::feeder_audit`]. Gate: the connector's inbound routes are
//! config-gated in `ferroehr-rest`.
//!
//! A mapping definition binds one FHIR R4B resource profile to one openEHR
//! template. Its `entries` each read a value out of the incoming FHIR resource
//! (a **`FHIRPath`-lite** dot-path — see [`resolve`]) and write it to a
//! template-relative **openEHR FLAT path** (the `id[:i]/…|suffix` key
//! `openehr_its::flat` consumes, ITS-REST `simplified_formats` master04 §Field
//! Identifiers). The resulting flat map is handed to
//! [`composition_from_flat`](openehr_its::flat::convert::composition_from_flat) with
//! the template's `WebTemplate` to build a canonical COMPOSITION, which then
//! commits through the platform's NORMAL validated path. This module is
//! protocol-free and DB-free: it is the deterministic transform, unit tested
//! here; the orchestration (mapping-store lookup, EHR resolution, commit)
//! lives in the parent [`super`] module on `FerroEhrService`.
//!
//! NOTE: the FHIR side is a deliberate **subset** of `FHIRPath` —
//! object-field navigation and array indexing only
//! (`code.coding[0].code`, `component[1].valueQuantity.value`) — NOT the full
//! `FHIRPath` language (no functions, filters, `where()`, `resolve()`, unions,
//! `$this`, arithmetic). That subset is sufficient for the flat, single-value
//! leaf extraction openEHR FLAT paths need; anything richer is out of the
//! starter scope. Cross-terminology *code-value* translation is likewise
//! deferred: `code_map` binds a FHIR system URL to an openEHR `terminology_id`
//! and passes the code through (the `TerminologyService` seam is where value
//! translation would plug in) — the built COMPOSITION's own terminology
//! validation is the authority on the result.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use std::collections::BTreeMap;

use crate::fhir::MappedSubject;
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// A validated FHIR→openEHR mapping definition (the `definition` JSON stored in
/// `fhir_mapping.definition`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FhirMappingDefinition {
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
pub struct SubjectMapping {
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
pub struct MappingEntry {
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
pub enum Transform {
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
pub enum FhirMapError {
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
    /// The same failure with its cause intact (RFC 0201): the flattener's own
    /// error says WHICH node defeated the walk, which a string cannot be
    /// matched on.
    #[error("could not flatten COMPOSITION for reverse mapping: {0}")]
    ReverseFailed(String, #[source] openehr_its::flat::error::FlatError),
}

/// Resolve a **`FHIRPath`-lite** dot-path against a JSON value.
///
/// Grammar (a deliberate subset of `FHIRPath` — see the module NOTE):
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
#[must_use]
pub fn resolve<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
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

/// Splits one path segment into its `(name, index)` parts.
///
/// `component[0]` → `("component", Some(0))`, `reference` → `("reference",
/// None)`; `None` for a malformed segment (unbalanced/broken index). Shared
/// with the reverse transform's writer ([`super::reverse`]).
#[must_use]
pub fn parse_segment(seg: &str) -> Option<(&str, Option<usize>)> {
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
/// # Errors
/// Returns [`FhirMapError`] when a mapping entry cannot be applied to the
/// resource (missing/mistyped member, unresolvable path).
pub fn build_flat(
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
/// # Errors
/// Returns [`FhirMapError::MissingSubject`] when the mapping's subject
/// reference path resolves to nothing usable.
pub fn extract_subject(
    resource: &Value,
    def: &FhirMappingDefinition,
) -> Result<MappedSubject, FhirMapError> {
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
    Ok(MappedSubject {
        id: id.to_owned(),
        namespace: def.subject.namespace.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use openehr_its::flat::convert::composition_from_flat;
    use openehr_its::flat::webtemplate::builder::build_web_template;
    use openehr_its::flat::webtemplate::model::WebTemplate;
    use openehr_its::opt14;
    use serde_json::json;

    use super::super::feeder_audit::{
        ORIGINATING_SYSTEM, feeder_audit, inject_feeder_audit, resource_id, resource_version,
    };
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

    // ── End-to-end: FHIR → FLAT → canonical COMPOSITION ──────────────────────
    // Justification for the corpus template: `ehrbase_blood_pressure_simple.de.v0`
    // (vendored under app/ferroehr/tests/resources) is a small, single-OBSERVATION
    // blood-pressure template whose flat json-ids (systolic/diastolic
    // magnitude+unit) map cleanly to a FHIR BP Observation's components — the
    // canonical starter mapping. The FLAT keys used here were taken from
    // `to_flat(example_composition(wt))` for this OPT, so they are exactly the
    // template's committable json-id leaves.
    fn bp_web_template() -> WebTemplate {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../ferroehr/tests/resources/service/knowledge/opt/ehrbase_blood_pressure_simple.de.v0.opt");
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
        // Fixed ctx/time (ITS-REST simplified_formats master04 §Context) so the
        // built composition is deterministic under test.
        let mut comp = composition_from_flat(&flat, &wt, "2024-01-01T00:00:00Z")
            .expect("from_flat builds a composition");
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
        let parsed: openehr_rm::v1_2::composition::composition::Composition =
            openehr_its::json::from_canonical_value(&comp).expect("deserialises as RM Composition");
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
}
