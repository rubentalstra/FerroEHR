//! RM-mandated openEHR-terminology validation (archie's `Category_validity`,
//! `Setting_valid`, `Current_state_valid`, null-flavour, participation
//! function/mode, … — the terminology-bound invariants `openehr-rm` defers).
//!
//! These codes must come from a specific openEHR terminology *group* fixed by
//! the RM (not the archetype), so the check is independent of the `WebTemplate`.
//! We walk the instance and, for each RM node bearing one of these coded slots,
//! validate the code against the group via [`openehr_term::bundle`]. Only codes
//! whose `terminology_id` is `openehr` are checked (a non-`openehr` terminology
//! is out of scope for the group check and is skipped).

use openehr_term::bundle::{OpenehrTerminology, openehr};
use serde_json::Value;

use super::{ValidationKind, Validator, norm_path};

/// The openEHR group a coded slot must draw from.
#[derive(Clone, Copy)]
enum Group {
    CompositionCategory,
    Setting,
    NullFlavour,
    InstructionState,
    ParticipationFunction,
    ParticipationMode,
    EventMathFunction,
}

impl Group {
    fn is_valid(self, t: &OpenehrTerminology, code: &str) -> bool {
        match self {
            Group::CompositionCategory => t.is_valid_composition_category(code),
            Group::Setting => t.is_valid_setting(code),
            Group::NullFlavour => t.is_valid_null_flavour(code),
            Group::InstructionState => t.is_valid_instruction_state(code),
            Group::ParticipationFunction => t.is_valid_participation_function(code),
            Group::ParticipationMode => t.is_valid_participation_mode(code),
            Group::EventMathFunction => t.is_valid_event_math_function(code),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Group::CompositionCategory => "composition category",
            Group::Setting => "setting",
            Group::NullFlavour => "null flavour",
            Group::InstructionState => "instruction state",
            Group::ParticipationFunction => "participation function",
            Group::ParticipationMode => "participation mode",
            Group::EventMathFunction => "event math function",
        }
    }
}

impl Validator {
    pub(super) fn terminology_pass(&mut self, v: &Value, path: &str, _parent_type: Option<&str>) {
        let Some(obj) = v.as_object() else { return };
        let this_type = obj.get("_type").and_then(Value::as_str);

        // Slots fixed by the owning RM type.
        for (attr, group) in slots_for(this_type) {
            if let Some(node) = obj.get(*attr) {
                self.check_openehr_code(node, &format!("{path}/{attr}"), *group);
            }
        }
        // `null_flavour` may appear on any LOCATABLE, independent of its type.
        if let Some(nf) = obj.get("null_flavour") {
            self.check_openehr_code(nf, &format!("{path}/null_flavour"), Group::NullFlavour);
        }

        for (k, val) in obj {
            if k.starts_with('_') {
                continue;
            }
            match val {
                Value::Array(a) => {
                    for (i, item) in a.iter().enumerate() {
                        if item.is_object() {
                            self.terminology_pass(item, &format!("{path}/{k}[{i}]"), this_type);
                        }
                    }
                }
                Value::Object(_) => {
                    self.terminology_pass(val, &format!("{path}/{k}"), this_type);
                }
                _ => {}
            }
        }
    }

    /// Validate an openEHR-terminology code on a coded node against its group.
    fn check_openehr_code(&mut self, node: &Value, path: &str, group: Group) {
        let Some((code, terminology)) = openehr_code(node) else {
            return;
        };
        if terminology != "openehr" {
            return; // out of scope for the RM group check
        }
        if !group.is_valid(openehr(), code) {
            self.push(
                norm_path(path),
                format!(
                    "code '{code}' is not a valid {} (openEHR terminology)",
                    group.label()
                ),
                ValidationKind::Terminology,
            );
        }
    }
}

/// The coded slots fixed by the owning RM type.
fn slots_for(rm_type: Option<&str>) -> &'static [(&'static str, Group)] {
    match rm_type {
        Some("COMPOSITION") => &[("category", Group::CompositionCategory)],
        Some("EVENT_CONTEXT") => &[("setting", Group::Setting)],
        Some("ISM_TRANSITION") => &[("current_state", Group::InstructionState)],
        Some("PARTICIPATION") => &[
            ("function", Group::ParticipationFunction),
            ("mode", Group::ParticipationMode),
        ],
        Some("EVENT" | "POINT_EVENT" | "INTERVAL_EVENT") => {
            &[("math_function", Group::EventMathFunction)]
        }
        _ => &[],
    }
}

/// The `(code, terminology)` of a coded node — a `DV_CODED_TEXT`/`DV_STATE` (via
/// `defining_code`) or a bare `CODE_PHRASE`. `None` when the node is not coded
/// (e.g. a plain `DV_TEXT` participation function).
fn openehr_code(node: &Value) -> Option<(&str, &str)> {
    let code_phrase = node.get("defining_code").unwrap_or(node);
    let code = code_phrase.get("code_string").and_then(Value::as_str)?;
    let terminology = code_phrase
        .get("terminology_id")
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("");
    Some((code, terminology))
}
