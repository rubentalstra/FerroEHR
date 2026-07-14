//! Example-COMPOSITION synthesis of RM-mandatory ENTRY structural attributes.
//!
//! Regression coverage for the defect where the FLAT/TDD composition builder
//! synthesised a blind `at0001` placeholder for a *constrained* structural
//! attribute the simplified form carried no content under (e.g. the CKM
//! "International Patient Summary" `ACTION.medication` whose `description` the
//! template constrains to `ITEM_TREE[at0017]`), producing a COMPOSITION the
//! validator then rejected with "unexpected node 'at0001' under 'description'".
//!
//! Oracle: the vendored `ips.v0.opt` (SDK corpus) reproduces the case exactly —
//! its single `ACTION` constrains `description` to `ITEM_TREE[at0017]` with no
//! leaf content, so the attribute is synthesised. The builder now records the
//! constrained node id/type/name as a structural stub and the composition builder
//! stamps it, per AOM 1.4
//! `AM/docs/AOM1.4/master04-constraint_model_package.adoc` §`Valid_value` (a
//! constrained attribute must be filled by a conforming value). Where an attribute
//! is *unconstrained*, the spec-legal `at0001` "Any" placeholder is kept (ADL 1.4
//! `AM/docs/ADL1.4/master05-cadl.adoc` §"Any" Constraints; CNF
//! `master15-content_tc_composition.adoc` L38).

use std::path::{Path, PathBuf};

use openehr_flat::webtemplate::WebTemplateNode;
use openehr_flat::{
    DetailLevel, ValidationKind, WebTemplate, build_web_template, example_composition,
    validate_composition,
};
use openehr_its::opt14;
use serde_json::Value;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn web_template(rel: &str) -> WebTemplate {
    let path = fixtures_dir().join(rel);
    let name = path.display();
    let xml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}: {e}"));
    let opt = opt14::from_xml(&xml).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    build_web_template(&opt).unwrap_or_else(|e| panic!("build {name}: {e}"))
}

/// Collect every ACTION object in a composition.
fn actions(v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            if m.get("_type").and_then(Value::as_str) == Some("ACTION") {
                out.push(v.clone());
            }
            m.values().for_each(|c| actions(c, out));
        }
        Value::Array(a) => a.iter().for_each(|e| actions(e, out)),
        _ => {}
    }
}

/// `Unexpected`-kind messages — the defect's signature ("unexpected node …").
fn unexpected(comp: &Value, wt: &WebTemplate) -> Vec<String> {
    validate_composition(comp, wt)
        .into_iter()
        .filter(|m| m.kind == ValidationKind::Unexpected)
        .map(|m| format!("{}: {}", m.path, m.message))
        .collect()
}

/// Recursively edit the `WebTemplate` tree in place.
fn edit_tree(node: &mut WebTemplateNode, f: &impl Fn(&mut WebTemplateNode)) {
    f(node);
    for child in &mut node.children {
        edit_tree(child, f);
    }
}

/// The vendored template constrains `ACTION.description` to `ITEM_TREE[at0017]`
/// with no content, so the attribute is synthesised: it must carry the
/// constrained node id (`at0017`) with the term-definition name — never the
/// blind `at0001` placeholder — and the composition must validate clean.
#[test]
fn constrained_action_description_synthesises_the_constrained_node_id() {
    let wt = web_template("sdk/ips.v0.opt");
    let comp = example_composition(&wt, DetailLevel::Required);

    let mut found = Vec::new();
    actions(&comp, &mut found);
    assert!(!found.is_empty(), "the IPS template has an ACTION");

    let description = found[0]
        .get("description")
        .expect("ACTION.description synthesised");
    assert_eq!(
        description.get("_type").and_then(Value::as_str),
        Some("ITEM_TREE"),
        "description is the constrained ITEM_TREE"
    );
    assert_eq!(
        description.get("archetype_node_id").and_then(Value::as_str),
        Some("at0017"),
        "the constrained node id is stamped, not the at0001 placeholder"
    );
    assert!(
        description
            .pointer("/name/value")
            .and_then(Value::as_str)
            .is_some_and(|n| !n.is_empty()),
        "the synthesised node carries a name"
    );
    assert!(
        unexpected(&comp, &wt).is_empty(),
        "the generated composition validates without 'unexpected node' errors"
    );
}

/// The defect's mechanism, pinned both ways on the one real fixture: with the
/// structural stub cleared but the closed-archetype constraint kept, the
/// synthesised `at0001` placeholder is (correctly) rejected — proving the
/// validator is spec-right and the stub is load-bearing.
#[test]
fn clearing_the_stub_reintroduces_the_rejected_placeholder() {
    let mut wt = web_template("sdk/ips.v0.opt");
    // Simulate the pre-fix builder: drop the structural stubs (the constrained
    // identity is lost) but keep the closed-archetype constraint that admits only
    // at0017.
    edit_tree(&mut wt.tree, &|n| n.structural_stubs.clear());

    let comp = example_composition(&wt, DetailLevel::Required);
    let mut found = Vec::new();
    actions(&comp, &mut found);
    assert_eq!(
        found[0]
            .get("description")
            .and_then(|d| d.get("archetype_node_id"))
            .and_then(Value::as_str),
        Some("at0001"),
        "without the stub the builder falls back to the placeholder"
    );
    let errs = unexpected(&comp, &wt);
    assert!(
        errs.iter()
            .any(|e| e.contains("at0001") && e.contains("description")),
        "the closed-archetype walk rejects the placeholder under 'description': {errs:?}"
    );
}

/// When the attribute is genuinely unconstrained (no stub and no closed-archetype
/// constraint), the spec-legal `at0001` "Any" placeholder is kept and the
/// composition still validates — the "no constraint ≡ anything allowed" rule.
#[test]
fn unconstrained_structural_attribute_keeps_the_any_placeholder() {
    let mut wt = web_template("sdk/ips.v0.opt");
    edit_tree(&mut wt.tree, &|n| {
        n.structural_stubs.clear();
        n.closed_attributes.clear();
    });

    let comp = example_composition(&wt, DetailLevel::Required);
    let mut found = Vec::new();
    actions(&comp, &mut found);
    assert_eq!(
        found[0]
            .get("description")
            .and_then(|d| d.get("archetype_node_id"))
            .and_then(Value::as_str),
        Some("at0001"),
        "unconstrained attribute keeps the at0001 'Any' placeholder"
    );
    assert!(
        unexpected(&comp, &wt).is_empty(),
        "an unconstrained attribute admits the placeholder (no closed-world rejection)"
    );
}

/// Broad guard: the committable levels (`Required` and `Medium`) of every
/// vendored template produce no "unexpected node" — the synthesised
/// RM-mandatory structural attributes conform across the whole corpus at the
/// mandatory skeleton *and* with every optional branch populated. (`Complete`
/// is documented as "not necessarily committable", so is not asserted here.)
#[test]
fn committable_example_of_every_fixture_has_no_unexpected_node() {
    let mut opts = Vec::new();
    for sub in ["sdk", "better"] {
        collect_opts(&fixtures_dir().join(sub), &mut opts);
    }
    assert!(
        opts.len() > 40,
        "the corpus is present ({} OPTs)",
        opts.len()
    );

    let mut checked = 0;
    for path in &opts {
        let Ok(xml) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(opt) = opt14::from_xml(&xml) else {
            continue;
        };
        let Ok(wt) = build_web_template(&opt) else {
            continue;
        };
        for level in [DetailLevel::Required, DetailLevel::Medium] {
            let comp = example_composition(&wt, level);
            let errs = unexpected(&comp, &wt);
            assert!(
                errs.is_empty(),
                "{path:?} @ {level:?} produced unexpected nodes: {errs:?}"
            );
        }
        checked += 1;
    }
    assert!(checked > 40, "checked {checked} templates");
}

fn collect_opts(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_opts(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("opt") {
            out.push(p);
        }
    }
}
