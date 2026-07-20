#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
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

use openehr_flat::convert::{composition_from_flat, composition_to_flat};
use openehr_flat::webtemplate::{WebTemplate, build_web_template};
use openehr_its::opt14;
use serde_json::Value;

/// Fixed `ctx/time` default for the FLAT build direction (ITS-REST
/// simplified_formats master04 §Context) so round-trips are deterministic.
const NOW: &str = "2024-01-01T00:00:00Z";

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
        manifest_dir().join("../../app/ehrbase/tests/resources/service"),
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

fn sorted(m: &serde_json::Map<String, Value>) -> BTreeMap<String, Value> {
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

        let flat0 = match composition_to_flat(comp, wt) {
            Ok(f) => f,
            Err(e) => {
                failures.push(format!("{name}: to_flat: {e}"));
                continue;
            }
        };
        let rm1 = match composition_from_flat(&flat0, wt, NOW) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{name}: from_flat: {e}"));
                continue;
            }
        };
        let flat1 = match composition_to_flat(&rm1, wt) {
            Ok(f) => f,
            Err(e) => {
                failures.push(format!("{name}: composition_to_flat(rm1): {e}"));
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
    // canonical `openehr-rm` Composition (the INSTRUCTION / ACTIVITY /
    // INTERVAL_EVENT / DV_MULTIMEDIA / DV_PARSABLE structural gaps are closed).
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
    let flat =
        composition_to_flat(&comp, wt).unwrap_or_else(|e| panic!("to_flat {comp_file}: {e}"));
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
    let flat = composition_to_flat(&comp, wt).expect("to_flat");

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

// ── closed-gap assertions ─────────────────────────────────────────────────────

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
    let rm = composition_from_flat(&composition_to_flat(&comp, wt).unwrap(), wt, NOW).unwrap();
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
    // Output uses the lossless master05 §EVENT_CONTEXT path rows
    // (`context/_participation:i|…`); the master06 ctx/ shortcuts remain
    // accepted on input (they are lossy — no scheme key — so they are not
    // the emission form).
    let wts = web_templates();
    let wt = wts
        .get("minimal_observation.en.v1")
        .expect("minimal_observation web template");
    let comp = load("minimal_observation.json");
    let flat0 = composition_to_flat(&comp, wt).unwrap();
    assert!(
        flat0
            .keys()
            .any(|k| k.contains("context/_participation:0|name")),
        "participations emit as the lossless path form: {:?}",
        flat0.keys().collect::<Vec<_>>()
    );
    let rm = composition_from_flat(&flat0, wt, NOW).unwrap();
    assert!(
        rm.pointer("/context/participations/0/performer").is_some(),
        "participations rebuilt in context: {rm}"
    );
    assert!(is_valid_rm(&rm), "participation round-trip valid RM: {rm}");
    let flat1 = composition_to_flat(&rm, wt).unwrap();
    assert_eq!(
        sorted(&flat0),
        sorted(&flat1),
        "participation round-trip not stable"
    );

    // The master06 §Participation ctx shortcuts still build the same RM on
    // input.
    let root = &wt.tree.id;
    let mut shortcut = serde_json::Map::new();
    shortcut.insert("ctx/language".to_owned(), Value::String("en".into()));
    shortcut.insert("ctx/territory".to_owned(), Value::String("US".into()));
    shortcut.insert("ctx/time".to_owned(), Value::String(NOW.into()));
    shortcut.insert(
        "ctx/participation_name:0".to_owned(),
        Value::String("Dr. Marcus Johnson".into()),
    );
    shortcut.insert(
        "ctx/participation_function:0".to_owned(),
        Value::String("requester".into()),
    );
    let _ = root;
    let rm = composition_from_flat(&shortcut, wt, NOW).expect("ctx shortcuts build");
    assert_eq!(
        rm.pointer("/context/participations/0/performer/name")
            .and_then(Value::as_str),
        Some("Dr. Marcus Johnson"),
        "ctx participation shortcut lands in EVENT_CONTEXT.participations: {rm}"
    );
}

/// `ctx/health_care_facility` (name + id) round-trips.
#[test]
fn ctx_health_care_facility_round_trips() {
    // Output uses the lossless master05 §EVENT_CONTEXT path row
    // (`context/_health_care_facility|…` — it carries |id_scheme, which the
    // master06 ctx shortcut cannot); the ctx/ shortcut stays input-only.
    let wts = web_templates();
    let Some(wt) = wts.get("cardinality_of_section") else {
        return; // template not present in this checkout
    };
    let comp = load("cardinality_of_section__full.json");
    let flat0 = composition_to_flat(&comp, wt).unwrap();
    assert!(
        flat0
            .keys()
            .any(|k| k.contains("context/_health_care_facility|name")),
        "health_care_facility emits as the lossless path form: {:?}",
        flat0.keys().collect::<Vec<_>>()
    );
    let rm = composition_from_flat(&flat0, wt, NOW).unwrap();
    assert_eq!(
        rm.pointer("/context/health_care_facility/name"),
        comp.pointer("/context/health_care_facility/name"),
        "health_care_facility name rebuilt"
    );
    assert_eq!(
        sorted(&flat0),
        sorted(&composition_to_flat(&rm, wt).unwrap())
    );

    // The master06 §health_care_facility ctx shortcut still builds on input.
    let root = &wt.tree.id;
    let mut shortcut = serde_json::Map::new();
    shortcut.insert("ctx/language".to_owned(), Value::String("en".into()));
    shortcut.insert("ctx/territory".to_owned(), Value::String("US".into()));
    shortcut.insert("ctx/time".to_owned(), Value::String(NOW.into()));
    shortcut.insert(
        "ctx/health_care_facility|name".to_owned(),
        Value::String("Hospital".into()),
    );
    shortcut.insert(
        "ctx/health_care_facility|id".to_owned(),
        Value::String("9091".into()),
    );
    shortcut.insert(
        "ctx/id_namespace".to_owned(),
        Value::String("HOSPITAL-NS".into()),
    );
    let _ = root;
    let rm = composition_from_flat(&shortcut, wt, NOW).expect("ctx facility shortcut builds");
    assert_eq!(
        rm.pointer("/context/health_care_facility/name")
            .and_then(Value::as_str),
        Some("Hospital"),
        "ctx facility shortcut lands in the EVENT_CONTEXT: {rm}"
    );
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
    let flat = composition_to_flat(&comp, wt).unwrap();
    assert!(
        flat.keys().any(|k| k.ends_with("|size")),
        "DV_MULTIMEDIA emits |size: {:?}",
        flat.keys().collect::<Vec<_>>()
    );
    let rm = composition_from_flat(&flat, wt, NOW).unwrap();
    assert!(is_valid_rm(&rm), "multimedia from_flat valid RM: {rm}");
}

/// The SDT path+terse coded form `"terminology::code|value|"` parses into the
/// full DV_CODED_TEXT (SIM-B master04 §S_DV_CODED_TEXT; the regular
/// `|code`/`|terminology`/`|value` form stays the emitted shape).
#[test]
fn terse_coded_text_string_is_rejected() {
    // The terse coded-string form ("terminology::code|value|") is the
    // DEVELOPMENT-state SM serial-data-formats encoding, not the STABLE
    // ITS-REST wire: master05 §DV_CODED_TEXT defines only the
    // |code/|value/|terminology suffixes (+ |other, master04 §Open
    // Value-Sets). A bare string on a closed coded leaf is rejected, never
    // silently coerced.
    let wts = web_templates();
    let wt = wts
        .get("Corona_Anamnese")
        .expect("Corona_Anamnese web template");
    let comp = load("compo_corona.json");
    let flat0 = composition_to_flat(&comp, wt).unwrap();
    let code_key = flat0
        .keys()
        .find(|k| {
            k.ends_with("|code")
                && !k.starts_with("ctx/")
                && !k.ends_with("/category|code")
                && !k.ends_with("/language|code")
                && !k.ends_with("/territory|code")
        })
        .expect("a coded tree leaf")
        .clone();
    let base = code_key.trim_end_matches("|code").to_owned();
    let code = flat0[&code_key].as_str().unwrap().to_owned();
    let terminology = flat0
        .get(&format!("{base}|terminology"))
        .and_then(|v| v.as_str())
        .unwrap_or("local")
        .to_owned();
    let value = flat0
        .get(&format!("{base}|value"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let mut terse = flat0.clone();
    terse.remove(&code_key);
    terse.remove(&format!("{base}|terminology"));
    terse.remove(&format!("{base}|value"));
    terse.insert(
        base.clone(),
        serde_json::Value::String(format!("{terminology}::{code}|{value}|")),
    );
    let err = composition_from_flat(&terse, wt, NOW)
        .expect_err("the terse coded-string form is not part of the STABLE wire");
    let msg = err.to_string();
    assert!(
        msg.contains(base.rsplit('/').next().unwrap_or(&base)) || msg.contains("invalid value"),
        "the rejection names the offending leaf: {msg}"
    );
}

// ── persistent-Composition context (finding 3) ──────────────────────────────────
//
// `COMPOSITION.context` is optional (0..1). A `431|persistent|` Composition
// idiomatically carries NO Event context (RM ehr
// `master05-composition_package.adoc` §"Persistent Compositions may optionally
// have an Event context" — the pre-1.0.4 invariant forbidding it was removed by
// SPECRM-52). `from_flat` must therefore NOT fabricate a default Event context
// for a persistent Composition that carried none, while still preserving one the
// caller explicitly supplied (participations / location / facility / end_time),
// and while still building the mandatory context for an event Composition.

const CNF_CORPUS: &str =
    "../../docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets";

fn persistent_wt() -> WebTemplate {
    let opt_xml = std::fs::read_to_string(format!(
        "{CNF_CORPUS}/valid_templates/minimal_persistent/persistent_minimal.opt"
    ))
    .unwrap_or_else(|e| panic!("read persistent OPT: {e}"));
    let opt = opt14::from_xml(&opt_xml).unwrap_or_else(|e| panic!("parse OPT: {e}"));
    build_web_template(&opt).unwrap_or_else(|e| panic!("build WT: {e}"))
}

#[test]
fn persistent_composition_round_trip_synthesises_no_context() {
    // The vendored canonical persistent Composition has category 431 and no
    // context — exactly the varier's decompose→reassemble scenario.
    let comp: Value = serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{CNF_CORPUS}/compositions/CANONICAL_JSON/persistent_minimal.en.v1__full.json"
        ))
        .expect("read persistent composition"),
    )
    .expect("parse composition");
    assert_eq!(
        comp.pointer("/category/defining_code/code_string"),
        Some(&Value::String("431".to_owned())),
        "fixture is persistent"
    );
    assert!(comp.get("context").is_none(), "fixture carries no context");

    let wt = persistent_wt();
    let flat = composition_to_flat(&comp, &wt).expect("to_flat");
    let rebuilt = composition_from_flat(&flat, &wt, NOW).expect("from_flat");
    assert_eq!(
        rebuilt.pointer("/category/defining_code/code_string"),
        Some(&Value::String("431".to_owned())),
        "category preserved as persistent"
    );
    assert!(
        rebuilt.get("context").is_none(),
        "no Event context is fabricated for a persistent Composition; got: {:?}",
        rebuilt.get("context")
    );
    // The rebuilt Composition is still valid RM (context being optional).
    openehr_its::json::from_canonical_value::<openehr_rm::prelude::Composition>(&rebuilt)
        .expect("rebuilt deserialises as an RM Composition without a context");
}

#[test]
fn persistent_composition_keeps_explicitly_supplied_context() {
    // A persistent Composition WITH explicit context content (a participation) is
    // valid (SPECRM-52) and must be preserved, not dropped.
    let wt = persistent_wt();
    let root = &wt.tree.id;
    let mut flat = serde_json::Map::new();
    flat.insert(format!("{root}/category|code"), Value::String("431".into()));
    flat.insert(
        format!("{root}/category|value"),
        Value::String("persistent".into()),
    );
    flat.insert(
        "ctx/participation_name:0".to_owned(),
        Value::String("Dr Explicit".into()),
    );
    let rebuilt = composition_from_flat(&flat, &wt, NOW).expect("from_flat");
    assert_eq!(
        rebuilt.pointer("/category/defining_code/code_string"),
        Some(&Value::String("431".to_owned()))
    );
    assert_eq!(
        rebuilt
            .pointer("/context/participations/0/performer/name")
            .and_then(Value::as_str),
        Some("Dr Explicit"),
        "an explicitly supplied context is preserved on a persistent Composition: {rebuilt:?}"
    );
}

#[test]
fn event_composition_context_follows_explicit_ctx_input() {
    // RM `COMPOSITION.context` is optional with no category-coupled
    // invariant (RM UML composition class: Category_validity constrains only
    // the category code), so no Event context is fabricated from the
    // category alone — it is built exactly when the client expressed
    // event-context content (master06: time/setting/... ctx keys).
    let wt = persistent_wt();
    let root = &wt.tree.id;
    let mut flat = serde_json::Map::new();
    flat.insert(format!("{root}/category|code"), Value::String("433".into()));
    flat.insert(
        format!("{root}/category|value"),
        Value::String("event".into()),
    );
    let rebuilt = composition_from_flat(&flat, &wt, NOW).expect("from_flat");
    assert_eq!(
        rebuilt.pointer("/category/defining_code/code_string"),
        Some(&Value::String("433".to_owned()))
    );
    assert!(
        rebuilt.pointer("/context").is_none(),
        "no Event context is fabricated without event-context input: {rebuilt:?}"
    );

    // With an explicit ctx/time the context IS built, defaults filled
    // (master06 §§time, setting).
    flat.insert("ctx/time".to_owned(), Value::String(NOW.into()));
    let rebuilt = composition_from_flat(&flat, &wt, NOW).expect("from_flat with ctx/time");
    assert_eq!(
        rebuilt
            .pointer("/context/start_time/value")
            .and_then(Value::as_str),
        Some(NOW),
        "explicit ctx/time builds the Event context: {rebuilt:?}"
    );
    assert_eq!(
        rebuilt
            .pointer("/context/setting/defining_code/code_string")
            .and_then(Value::as_str),
        Some("238"),
        "setting defaults to openehr 238 other care (master06 §setting)"
    );
}
