//! Coded `LOCATABLE.name` + ISM `current_state` rubric fidelity.
//!
//! Two spec-facing generator defects, verified against the vendored OPT corpus:
//!
//! * **(b) coded names.** A template may constrain a `LOCATABLE.name` as a
//!   `DV_CODED_TEXT` (RM common `master03-archetyped_package.adoc` §"The
//!   `LOCATABLE` class" — a name is `DV_TEXT` *or* `DV_CODED_TEXT`; AOM 1.4
//!   `master04-constraint_model_package.adoc` — a `C_ATTRIBUTE` on `name`
//!   constrains the whole coded name). The WebTemplate builder must record the
//!   constrained `defining_code`, and the composition builder must stamp a
//!   coded name (with `defining_code`), not a plain `DV_TEXT` — otherwise a
//!   name-differentiated sibling cannot be routed by a strict RM validator.
//!
//! * **(c) ISM `current_state` rubric.** `ISM_TRANSITION.current_state` draws
//!   its openEHR code from the `instruction_states` group (RM `ehr`
//!   `ism_transition`). openEHR concept codes are not globally unique across
//!   groups (TERM 3.1.0 SPECPR-51: code `532` is `complete` in
//!   `version_lifecycle_state` but `completed` in `instruction_states`), so the
//!   display value must be resolved from the *owning* group.

use std::path::PathBuf;

use openehr_flat::webtemplate::WebTemplateNode;
use openehr_flat::{DetailLevel, WebTemplate, build_web_template, example_composition};
use openehr_its::opt14;
use serde_json::Value;

fn web_template(rel: &str) -> WebTemplate {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel);
    let name = path.display();
    let xml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}: {e}"));
    let opt = opt14::from_xml(&xml).unwrap_or_else(|e| panic!("parse {name}: {e}"));
    build_web_template(&opt).unwrap_or_else(|e| panic!("build {name}: {e}"))
}

/// Every WebTemplate node carrying a `name_coded` constraint, as `(display, code)`.
fn coded_name_nodes(node: &WebTemplateNode, out: &mut Vec<(String, String)>) {
    if let Some(coded) = &node.name_coded {
        out.push((node.name.clone().unwrap_or_default(), coded.code.clone()));
    }
    for c in &node.children {
        coded_name_nodes(c, out);
    }
}

/// Every `DV_CODED_TEXT` `name` in a composition, as `(value, defining_code.code_string)`.
fn coded_names_in(comp: &Value, out: &mut Vec<(String, String)>) {
    match comp {
        Value::Object(m) => {
            if let Some(name) = m.get("name")
                && name.get("_type").and_then(Value::as_str) == Some("DV_CODED_TEXT")
            {
                let value = name
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let code = name
                    .pointer("/defining_code/code_string")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                out.push((value.to_owned(), code.to_owned()));
            }
            for v in m.values() {
                coded_names_in(v, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|e| coded_names_in(e, out)),
        _ => {}
    }
}

/// The label of the coded value carrying `code` anywhere in the WebTemplate.
fn coded_value_label(node: &WebTemplateNode, code: &str) -> Option<String> {
    for input in &node.inputs {
        for cv in &input.list {
            if cv.value == code {
                return cv.label.clone();
            }
        }
    }
    node.children
        .iter()
        .find_map(|c| coded_value_label(c, code))
}

/// (b) A `DV_CODED_TEXT` name constrained by a fixed local `C_CODE_PHRASE` is
/// recorded on the WebTemplate node and stamped as a coded name in the example
/// composition — never a plain `DV_TEXT`. `Demo Vitals.opt` carries such a
/// constraint (an element whose `name` is `DV_CODED_TEXT` with a `local`
/// `defining_code`).
#[test]
fn coded_name_recorded_and_stamped() {
    let wt = web_template("better/Demo Vitals.opt");

    let mut nodes = Vec::new();
    coded_name_nodes(&wt.tree, &mut nodes);
    assert!(
        !nodes.is_empty(),
        "the WebTemplate builder must record at least one DV_CODED_TEXT name constraint"
    );
    for (display, code) in &nodes {
        assert!(
            !display.is_empty(),
            "a coded name must carry a display value"
        );
        assert!(!code.is_empty(), "a coded name must carry a defining code");
    }

    // The example composition stamps those names as DV_CODED_TEXT (with a
    // defining_code), and at least one matches a recorded (value, code) pair.
    let comp = example_composition(&wt, DetailLevel::Complete);
    let mut stamped = Vec::new();
    coded_names_in(&comp, &mut stamped);
    assert!(
        !stamped.is_empty(),
        "the composition must stamp at least one DV_CODED_TEXT name (not plain DV_TEXT)"
    );
    for (value, code) in &stamped {
        assert!(
            !code.is_empty(),
            "a stamped coded name must carry a non-empty defining_code.code_string (got value={value:?})"
        );
    }
    assert!(
        stamped.iter().any(|s| nodes.contains(s)),
        "a stamped coded name must match a template name constraint (value, code); \
         template={nodes:?} stamped={stamped:?}"
    );
}

/// (c) The `instruction_states` code `532` resolves to its group rubric
/// `completed`, not the `version_lifecycle_state` rubric `complete` — the
/// SPECPR-51 cross-group collision. `sdk/minimal_action3.opt` constrains an
/// `ISM_TRANSITION.current_state` to `openehr::532`.
#[test]
fn ism_current_state_532_uses_instruction_states_rubric() {
    let wt = web_template("sdk/minimal_action3.opt");
    let label = coded_value_label(&wt.tree, "532");
    assert_eq!(
        label.as_deref(),
        Some("completed"),
        "ISM current_state code 532 must render its instruction_states rubric ('completed'), \
         not the version_lifecycle_state rubric ('complete')"
    );
}
