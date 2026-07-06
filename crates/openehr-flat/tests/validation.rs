//! End-to-end composition-validation tests (P15 PR-C).
//!
//! Oracle: the already-vendored, **Apache-2.0** `openEHR_SDK` corpus — canonical
//! JSON compositions in `openehr-its/tests/vendor/openehr_sdk/…` paired with the
//! OPT 1.4 operational templates in `openehr-flat/tests/fixtures/…` by
//! `template_id`. Each valid composition is validated against the `WebTemplate`
//! built from its template and must be clean; invalid variants (one real vendored
//! `*_invalid` pair plus mutations of valid corpus compositions) must be rejected
//! with a sensible path + kind.
//!
//! PORT NOTE — why the CNF `test_data_sets` are not vendored here: the openEHR
//! `specifications-CNF` repository is licensed **CC BY-SA 3.0** (a copyleft /
//! share-alike license — checked at vendoring time), which is incompatible with
//! vendoring verbatim into this workspace's fixture tree the way the existing
//! **Apache-2.0** `openEHR_SDK` corpus already is (see the `PROVENANCE.md` /
//! `NOTICE.md` under `openehr-its/tests/vendor/` and
//! `openehr-flat/tests/fixtures/sdk/`). The SDK corpus is the same class of
//! oracle (real openEHR canonical instances + their OPTs, including
//! `nested.en.v1`, `cardinality_of_section`, `test_all_types`, IPS, …), it needs
//! no network at test time, and it carries a real valid/invalid pair
//! (`ips_canonical` / `ips_invalid`). This is the fallback the task sanctions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use openehr_flat::{ValidationKind, ValidationMessage, WebTemplate, build_web_template};
use openehr_its::opt14;
use serde_json::Value;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn opt_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            opt_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "opt") {
            out.push(p);
        }
    }
}

/// `template_id` → `WebTemplate` for every parseable vendored OPT.
fn web_templates() -> HashMap<String, WebTemplate> {
    let mut opts = Vec::new();
    opt_files(&manifest_dir().join("tests/fixtures"), &mut opts);
    let mut wts = HashMap::new();
    for p in &opts {
        let Ok(xml) = std::fs::read_to_string(p) else {
            continue;
        };
        let Ok(opt) = opt14::from_xml(&xml) else {
            continue;
        };
        if let Ok(wt) = build_web_template(&opt) {
            wts.entry(opt.template_id.value.clone()).or_insert(wt);
        }
    }
    wts
}

fn composition(name: &str) -> Value {
    let p = manifest_dir()
        .join("../openehr-its/tests/vendor/openehr_sdk/composition/canonical_json")
        .join(name);
    let txt = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {name}: {e}"));
    serde_json::from_str(&txt).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

/// A composition's declared template id.
fn template_id(c: &Value) -> String {
    c.get("archetype_details")
        .and_then(|a| a.get("template_id"))
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn validate(name: &str, wts: &HashMap<String, WebTemplate>) -> Vec<ValidationMessage> {
    let c = composition(name);
    let tid = template_id(&c);
    let wt = wts
        .get(&tid)
        .unwrap_or_else(|| panic!("no WebTemplate for template id {tid:?} (composition {name})"));
    openehr_flat::validate_composition(&c, wt)
}

fn kinds(msgs: &[ValidationMessage]) -> Vec<ValidationKind> {
    msgs.iter().map(|m| m.kind).collect()
}

/// The curated set of vendored valid compositions whose WebTemplate-guided
/// validation is expected to be completely clean. (Other corpus compositions
/// deliberately omit `archetype_details` on nested archetype-root entries, which
/// the RM `Is_archetype_root` invariant — surfaced from `openehr-rm` — correctly
/// flags; those are excluded here, not silently tolerated.)
const CLEAN_COMPOSITIONS: &[&str] = &[
    "all_types_no_multimedia.json",
    "choice_validation_test.json",
    "compo_corona.json",
    "demo_vitals_352.json",
    "dvquantity_choice.json",
    "ehrb_adbm_op_consult_record.json",
    "interval_partial_date.json",
    "ips_canonical.json",
    "minimal_action2_1.json",
    "minimal_with_optional_attribute.json",
    "minimal_without_optional_attribute.json",
    "my_spanish_template_v0_COMPOSITION_EXAMPLE.json",
    "nested.en.v1.json",
    "other_participations.json",
    "participation_no_content.json",
    "rawdb_composition.json",
    "virology_finding_with_specimen_no_update.json",
];

#[test]
fn valid_corpus_compositions_validate_clean() {
    let wts = web_templates();
    let mut failures = Vec::new();
    for name in CLEAN_COMPOSITIONS {
        let msgs = validate(name, &wts);
        if !msgs.is_empty() {
            failures.push(format!(
                "{name}: {} unexpected violation(s): {:?}",
                msgs.len(),
                msgs.iter()
                    .map(|m| format!("[{:?}] {}", m.kind, m.path))
                    .collect::<Vec<_>>()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "expected these vendored valid compositions to validate clean:\n{}",
        failures.join("\n")
    );
}

// ── a real vendored valid / invalid pair (same template) ─────────────────────

#[test]
fn ips_valid_is_clean_invalid_is_rejected() {
    let wts = web_templates();

    // The valid IPS composition validates clean.
    assert!(
        validate("ips_canonical.json", &wts).is_empty(),
        "ips_canonical should be clean"
    );

    // Its intentionally-broken sibling is rejected with real domain violations:
    // out-of-range magnitudes/counts and coded values outside the value set.
    let bad = validate("ips_invalid.json", &wts);
    assert!(!bad.is_empty(), "ips_invalid should be rejected");
    let ks = kinds(&bad);
    assert!(
        ks.contains(&ValidationKind::RangeError),
        "expected a RangeError in ips_invalid, got {bad:?}"
    );
    assert!(
        ks.contains(&ValidationKind::CodedValue),
        "expected a CodedValue in ips_invalid, got {bad:?}"
    );
    // Every violation is keyed by a non-empty RM path.
    assert!(bad.iter().all(|m| !m.path.is_empty()));
}

// ── synthesized invalids (mutations of a valid corpus composition) ───────────

#[test]
fn dropping_mandatory_content_reports_required() {
    // Emptying the IPS content removes its mandatory sections (min >= 1).
    let wts = web_templates();
    let wt = wts.get("International Patient Summary").expect("IPS wt");
    let mut c = composition("ips_canonical.json");
    c["content"] = serde_json::json!([]);
    let msgs = openehr_flat::validate_composition(&c, wt);
    assert!(
        msgs.iter().any(|m| m.kind == ValidationKind::Required),
        "expected Required for the dropped mandatory sections, got {msgs:?}"
    );
}

#[test]
fn wrong_root_type_is_rejected() {
    // Corrupt the composition's root RM type; it no longer conforms to COMPOSITION.
    let wts = web_templates();
    let wt = wts.get("Demo Vitals").expect("Demo Vitals wt");
    let mut c = composition("demo_vitals_352.json");
    c["_type"] = Value::String("OBSERVATION".to_owned());
    let msgs = openehr_flat::validate_composition(&c, wt);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::WrongType && m.path.is_empty()),
        "expected a WrongType at the root, got {msgs:?}"
    );
}

#[test]
fn bad_composition_category_reports_terminology() {
    // Replace the composition category code with one outside the openEHR group.
    let wts = web_templates();
    let wt = wts.get("Demo Vitals").expect("Demo Vitals wt");
    let mut c = composition("demo_vitals_352.json");
    c["category"]["defining_code"]["code_string"] = Value::String("99999".to_owned());
    let msgs = openehr_flat::validate_composition(&c, wt);
    assert!(
        msgs.iter()
            .any(|m| m.kind == ValidationKind::Terminology
                && m.message.contains("composition category")),
        "expected a Terminology violation for the bad category, got {msgs:?}"
    );
}

#[test]
fn out_of_range_magnitude_reports_range_error() {
    // Inflate a range-constrained quantity magnitude in a clean composition.
    let wts = web_templates();
    let wt = wts.get("International Patient Summary").expect("IPS wt");
    let mut c = composition("ips_canonical.json");
    let mutated = inflate_first_quantity(&mut c);
    assert!(mutated, "expected at least one DV_QUANTITY to mutate");
    let msgs = openehr_flat::validate_composition(&c, wt);
    assert!(
        msgs.iter().any(|m| m.kind == ValidationKind::RangeError),
        "expected a RangeError after inflating a magnitude, got {msgs:?}"
    );
}

/// Set every `DV_QUANTITY.magnitude` in the tree to an out-of-range value.
fn inflate_first_quantity(v: &mut Value) -> bool {
    let mut mutated = false;
    inflate_rec(v, &mut mutated);
    mutated
}

fn inflate_rec(v: &mut Value, mutated: &mut bool) {
    match v {
        Value::Object(o) => {
            if o.get("_type").and_then(Value::as_str) == Some("DV_QUANTITY")
                && o.contains_key("magnitude")
            {
                o.insert("magnitude".to_owned(), Value::from(999_999.0));
                *mutated = true;
            }
            for val in o.values_mut() {
                inflate_rec(val, mutated);
            }
        }
        Value::Array(a) => {
            for val in a {
                inflate_rec(val, mutated);
            }
        }
        _ => {}
    }
}

// ── a WebTemplate node builder sanity check (paths line up) ───────────────────

#[test]
fn matched_violation_paths_are_archetype_paths() {
    // A rejected composition's archetype-conformance violations carry the wt
    // node's aqlPath (the archetype path), demonstrating instance↔template match.
    let wts = web_templates();
    let bad = validate("ips_invalid.json", &wts);
    let arche: Vec<&ValidationMessage> = bad
        .iter()
        .filter(|m| {
            matches!(
                m.kind,
                ValidationKind::RangeError | ValidationKind::CodedValue | ValidationKind::WrongType
            )
        })
        .collect();
    assert!(!arche.is_empty());
    for m in arche {
        assert!(
            m.path.starts_with("/content["),
            "archetype-conformance path should be an aqlPath, got {:?}",
            m.path
        );
    }
}
