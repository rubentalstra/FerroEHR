//! FLAT (simSDT) `RM ⇄ FLAT` converter tests.
//!
//! Oracle strategy (see the report / NOTICE files): Better `web-template-tests`
//! ships **no** static flat `.json` fixtures — its compositions are Better RAW
//! (`@class`) and its `.xml` are templates, not canonical compositions. So the
//! corpus here pairs the **canonical-JSON compositions** vendored in
//! `openehr-its` with the **operational templates** (from the EHRbase SDK +
//! Better `web-template-tests` + the service corpus) whose `templateId` matches
//! — real `(composition, OPT)` pairs.
//!
//! * **FLAT → RM → FLAT round-trip stable** — for each paired composition:
//!   `to_flat` (flat0) → `from_flat` (rm1) → `to_flat` (flat1); assert
//!   `flat0 == flat1` (modulo key order). Also counts how many `rm1`
//!   deserialise as an `openehr-rm` `Composition` (the reverse output is valid
//!   canonical RM).
//! * **insta goldens** — deterministic flat snapshots for representative pairs.
//! * **targeted key assertions** — Better flat key shape (`|magnitude`,
//!   `|unit` singular, `|code`).
#![allow(clippy::doc_markdown)] // prose with many proper nouns (EHRbase, simSDT, …)

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use openehr_flat::{WebTemplate, build_web_template, from_flat, to_flat};
use openehr_its::opt14;
use serde_json::Value;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The canonical-JSON composition corpus vendored in `openehr-its`.
fn composition_dir() -> PathBuf {
    manifest_dir().join("../openehr-its/tests/vendor/openehr_sdk/composition/canonical_json")
}

/// All directories that hold `.opt` operational templates for pairing.
fn opt_dirs() -> Vec<PathBuf> {
    vec![
        manifest_dir().join("tests/fixtures/sdk"),
        manifest_dir().join("tests/fixtures/better"),
        manifest_dir().join("../ehrbase/tests/resources/service"),
    ]
}

fn opt_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(opt_files(&path));
        } else if path.extension().is_some_and(|e| e == "opt") {
            out.push(path);
        }
    }
    out
}

/// Build `templateId → WebTemplate` for every OPT the `opt14` parser can read.
fn web_templates() -> BTreeMap<String, WebTemplate> {
    let mut out = BTreeMap::new();
    for dir in opt_dirs() {
        for path in opt_files(&dir) {
            let Ok(xml) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(opt) = opt14::from_xml(&xml) else {
                continue;
            };
            if let Ok(wt) = build_web_template(&opt) {
                out.entry(wt.template_id.clone()).or_insert(wt);
            }
        }
    }
    out
}

/// Load every canonical COMPOSITION (with its template id) from the corpus.
fn compositions() -> Vec<(String, String, Value)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(composition_dir()) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if value.get("_type").and_then(Value::as_str) != Some("COMPOSITION") {
            continue;
        }
        let Some(tid) = value
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        out.push((name, tid.to_owned(), value));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn to_map(m: &indexmap::IndexMap<String, Value>) -> serde_json::Map<String, Value> {
    m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn sorted(m: &indexmap::IndexMap<String, Value>) -> BTreeMap<String, Value> {
    m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

// ── round-trip gate ────────────────────────────────────────────────────────────

#[test]
fn flat_roundtrip_stable() {
    let wts = web_templates();
    let comps = compositions();
    assert!(!wts.is_empty(), "no web templates built");
    assert!(!comps.is_empty(), "no canonical compositions found");

    let mut paired = 0usize;
    let mut stable = 0usize;
    let mut valid_rm = 0usize;
    let mut failures: Vec<String> = Vec::new();
    let mut invalid_rm: Vec<String> = Vec::new();

    for (name, tid, comp) in &comps {
        let Some(wt) = wts.get(tid) else { continue };
        paired += 1;

        let flat0 = match to_flat(comp, wt) {
            Ok(f) => f,
            Err(e) => {
                failures.push(format!("{name}: to_flat: {e}"));
                continue;
            }
        };
        let rm1 = match from_flat(&to_map(&flat0), wt) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{name}: from_flat: {e}"));
                continue;
            }
        };
        let flat1 = match to_flat(&rm1, wt) {
            Ok(f) => f,
            Err(e) => {
                failures.push(format!("{name}: to_flat(rm1): {e}"));
                continue;
            }
        };

        if sorted(&flat0) == sorted(&flat1) {
            stable += 1;
        } else {
            let m0 = sorted(&flat0);
            let m1 = sorted(&flat1);
            let only0: Vec<_> = m0.keys().filter(|k| !m1.contains_key(*k)).take(4).collect();
            let only1: Vec<_> = m1.keys().filter(|k| !m0.contains_key(*k)).take(4).collect();
            let changed: Vec<_> = m0
                .iter()
                .filter(|(k, v)| m1.get(*k).is_some_and(|v1| v1 != *v))
                .map(|(k, _)| k)
                .take(4)
                .collect();
            failures.push(format!(
                "{name} ({tid}): {} keys → {} keys | only-in-flat0={only0:?} only-in-flat1={only1:?} changed={changed:?}",
                m0.len(),
                m1.len()
            ));
        }

        // The reverse output should be valid canonical RM.
        if let Ok(s) = serde_json::to_string(&rm1) {
            match openehr_its::json::from_canonical_json::<openehr_rm::prelude::Composition>(&s) {
                Ok(_) => valid_rm += 1,
                Err(e) => invalid_rm.push(format!("{name}: {e}")),
            }
        }
    }

    eprintln!(
        "FLAT round-trip: {paired} paired (composition, OPT) | stable = {stable} | rm1 valid-RM = {valid_rm}"
    );
    if !invalid_rm.is_empty() {
        eprintln!(
            "rm1 not deserialising as openehr-rm Composition ({}):",
            invalid_rm.len()
        );
        for f in &invalid_rm {
            eprintln!("  {f}");
        }
    }
    if !failures.is_empty() {
        eprintln!("non-stable / errors ({}):", failures.len());
        for f in &failures {
            eprintln!("  {f}");
        }
    }

    assert!(paired >= 15, "expected ≥15 paired fixtures, got {paired}");
    assert!(
        stable == paired,
        "{}/{paired} pairs were not FLAT round-trip stable",
        paired - stable
    );
    // Every paired composition's `from_flat` output must deserialise as a
    // canonical `openehr-rm` Composition (P14 PR-C closed the INSTRUCTION /
    // ACTIVITY / INTERVAL_EVENT / DV_MULTIMEDIA / DV_PARSABLE structural gaps).
    assert!(
        valid_rm == paired,
        "{}/{paired} `from_flat` outputs did not deserialise as openehr-rm Composition: {invalid_rm:?}",
        paired - valid_rm
    );
}

// ── insta goldens ───────────────────────────────────────────────────────────────

fn golden_flat(comp_file: &str, template_id: &str, snap: &str) {
    let wts = web_templates();
    let wt = wts
        .get(template_id)
        .unwrap_or_else(|| panic!("no web template for {template_id:?}"));
    let text = std::fs::read_to_string(composition_dir().join(comp_file))
        .unwrap_or_else(|e| panic!("read {comp_file}: {e}"));
    let comp: Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {comp_file}: {e}"));
    let flat = to_flat(&comp, wt).unwrap_or_else(|e| panic!("to_flat {comp_file}: {e}"));
    let map: BTreeMap<String, Value> = flat.into_iter().collect();
    insta::assert_json_snapshot!(snap, map);
}

#[test]
fn golden_demo_vitals_flat() {
    golden_flat("demo_vitals_352.json", "Demo Vitals", "demo_vitals_flat");
}

#[test]
fn golden_minimal_observation_flat() {
    golden_flat(
        "minimal_observation.json",
        "minimal_observation.en.v1",
        "minimal_observation_flat",
    );
}

// ── targeted key assertions ──────────────────────────────────────────────────────

#[test]
fn demo_vitals_flat_key_shape() {
    let wts = web_templates();
    let wt = wts.get("Demo Vitals").expect("Demo Vitals web template");
    let text = std::fs::read_to_string(composition_dir().join("demo_vitals_352.json")).unwrap();
    let comp: Value = serde_json::from_str(&text).unwrap();
    let flat = to_flat(&comp, wt).expect("to_flat");

    // ctx keys present.
    assert!(flat.contains_key("ctx/language"), "ctx/language present");
    assert!(flat.contains_key("ctx/territory"), "ctx/territory present");

    // A DV_QUANTITY leaf uses |magnitude + |unit (singular).
    let has_magnitude = flat.keys().any(|k| k.ends_with("|magnitude"));
    let has_unit = flat.keys().any(|k| k.ends_with("|unit"));
    assert!(
        has_magnitude,
        "expected a |magnitude key: {:?}",
        flat.keys().collect::<Vec<_>>()
    );
    assert!(has_unit, "expected a |unit key (singular)");
    assert!(
        !flat.keys().any(|k| k.ends_with("|units")),
        "|unit must be singular, not |units"
    );

    // A coded leaf uses |code.
    assert!(
        flat.keys().any(|k| k.ends_with("|code")),
        "expected a |code key"
    );
}

// ── closed-gap assertions (P14 PR-C) ──────────────────────────────────────────

fn load(comp_file: &str) -> Value {
    let text = std::fs::read_to_string(composition_dir().join(comp_file))
        .unwrap_or_else(|e| panic!("read {comp_file}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {comp_file}: {e}"))
}

fn is_valid_rm(rm: &Value) -> bool {
    serde_json::to_string(rm).is_ok_and(|s| {
        openehr_its::json::from_canonical_json::<openehr_rm::prelude::Composition>(&s).is_ok()
    })
}

/// The previously-failing INSTRUCTION/ACTIVITY fixture now rebuilds valid RM
/// with the mandatory `ACTIVITY.action_archetype_id`.
#[test]
fn instruction_activity_from_flat_is_valid_rm() {
    let wts = web_templates();
    let wt = wts
        .get("minimal_instruction.en.v1")
        .expect("minimal_instruction web template");
    let comp = load("minimal_instruction.json");
    let rm = from_flat(&to_map(&to_flat(&comp, wt).unwrap()), wt).unwrap();
    assert!(
        is_valid_rm(&rm),
        "minimal_instruction from_flat must deserialise as openehr-rm: {rm}"
    );
    assert!(
        rm.pointer("/content/0/activities/0/action_archetype_id")
            .is_some(),
        "rebuilt ACTIVITY carries action_archetype_id: {rm}"
    );
}

/// The `ctx/participation_*` shortcuts emit on output and rebuild
/// `context.participations` on input, round-trip stable.
#[test]
fn ctx_participation_shortcuts_round_trip() {
    let wts = web_templates();
    let wt = wts
        .get("minimal_observation.en.v1")
        .expect("minimal_observation web template");
    let comp = load("minimal_observation.json");
    let flat0 = to_flat(&comp, wt).unwrap();
    assert!(
        flat0
            .keys()
            .any(|k| k.starts_with("ctx/participation_name")),
        "participation ctx emitted: {:?}",
        flat0.keys().collect::<Vec<_>>()
    );
    let rm = from_flat(&to_map(&flat0), wt).unwrap();
    assert!(
        rm.pointer("/context/participations/0/performer").is_some(),
        "participations rebuilt in context: {rm}"
    );
    assert!(is_valid_rm(&rm), "participation round-trip valid RM: {rm}");
    let flat1 = to_flat(&rm, wt).unwrap();
    assert_eq!(
        sorted(&flat0),
        sorted(&flat1),
        "ctx participation round-trip not stable"
    );
}

/// `ctx/health_care_facility` (name + id) round-trips.
#[test]
fn ctx_health_care_facility_round_trips() {
    let wts = web_templates();
    let Some(wt) = wts.get("cardinality_of_section") else {
        return; // template not present in this checkout
    };
    let comp = load("cardinality_of_section__full.json");
    let flat0 = to_flat(&comp, wt).unwrap();
    assert!(
        flat0.contains_key("ctx/health_care_facility|name"),
        "health_care_facility ctx emitted"
    );
    let rm = from_flat(&to_map(&flat0), wt).unwrap();
    assert_eq!(
        rm.pointer("/context/health_care_facility/name"),
        comp.pointer("/context/health_care_facility/name"),
        "health_care_facility name rebuilt"
    );
    assert_eq!(sorted(&flat0), sorted(&to_flat(&rm, wt).unwrap()));
}

/// DV_MULTIMEDIA (`|size` + uri) and DV_PARSABLE (`|formalism`) leaves rebuild
/// valid RM (the previously-broken `all_types` fixtures).
#[test]
fn multimedia_and_parsable_leaves_are_valid_rm() {
    let wts = web_templates();
    let wt = wts
        .get("minimal_action_3.en.v1")
        .expect("a web template with a DV_MULTIMEDIA leaf");
    let comp = load("minimal_with_optional_attribute.json");
    let flat = to_flat(&comp, wt).unwrap();
    assert!(
        flat.keys().any(|k| k.ends_with("|size")),
        "DV_MULTIMEDIA emits |size: {:?}",
        flat.keys().collect::<Vec<_>>()
    );
    let rm = from_flat(&to_map(&flat), wt).unwrap();
    assert!(is_valid_rm(&rm), "multimedia from_flat valid RM: {rm}");
}
