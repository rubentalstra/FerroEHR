//! RM-mandated openEHR-terminology validation — the terminology-bound invariants
//! `openehr-rm` defers (it has no `openehr-term` dependency).
//!
//! Two families of terminology binding are enforced, both properties of the RM
//! instance (independent of the archetype / `WebTemplate`):
//!
//! 1. **openEHR terminology *group* codes** (`has_code_for_group_id`): the code
//!    must be a member of a specific openEHR group fixed by the owning RM type —
//!    e.g. `COMPOSITION.category` (`composition_category`), `EVENT_CONTEXT.setting`
//!    (`setting`), `ISM_TRANSITION.current_state`/`transition`,
//!    `PARTICIPATION.function`/`mode`, `EVENT.math_function`,
//!    `TERM_MAPPING.purpose`, `AUDIT_DETAILS.change_type`, `ATTESTATION.reason`,
//!    `PARTY_RELATED.relationship`, `null_flavour`. These carry
//!    `terminology_id = "openehr"`; a non-`openehr` terminology is out of scope
//!    for the group check and is skipped.
//! 2. **openEHR / ISO / IANA code-set codes** (`code_set(id).has_code`): the code
//!    must be a member of an external or internal code set fixed by the RM
//!    invariant — `COMPOSITION.language` / `ENTRY.language` / `DV_TEXT.language`
//!    (ISO 639-1 `languages`), `COMPOSITION.territory` (ISO 3166-1 `countries`),
//!    `ENTRY.encoding` / `DV_TEXT.encoding` (IANA `character_sets`),
//!    `DV_MULTIMEDIA.media_type` (IANA `media_types`), `DV_ORDERED.normal_status`
//!    (`normal_statuses`). The RM invariant here is `code_set(...).has_code(code)`
//!    with **no** `terminology_id` guard, so the code value is validated against
//!    the code set directly.
//!
//! Spec: the RM invariant tables under
//! `docs/specs/openehr/RM/docs/UML/classes/` (`composition`, `entry`, `dv_text`,
//! `ism_transition`, `term_mapping`, `dv_ordered`, `dv_multimedia`,
//! `party_related`, `audit_details`, `attestation`), resolved against the
//! terminology bundle in [`openehr_term::bundle`] (TERM 3.1.0). Findings F-07-03,
//! F-11-02, F-11-03, F-11-04, F-11-05.

use openehr_term::bundle::{OpenehrTerminology, openehr};
use serde_json::Value;

use super::{ValidationKind, Validator, norm_path};

/// An openEHR terminology *group* a coded slot must draw from (checked with a
/// `terminology_id == "openehr"` guard, per the `has_code_for_group_id`
/// invariants).
#[derive(Clone, Copy)]
enum Group {
    CompositionCategory,
    Setting,
    NullFlavour,
    InstructionState,
    InstructionTransition,
    ParticipationFunction,
    ParticipationMode,
    EventMathFunction,
    TermMappingPurpose,
    AuditChangeType,
    AttestationReason,
    SubjectRelationship,
}

impl Group {
    fn is_valid(self, t: &OpenehrTerminology, code: &str) -> bool {
        match self {
            Group::CompositionCategory => t.is_valid_composition_category(code),
            Group::Setting => t.is_valid_setting(code),
            Group::NullFlavour => t.is_valid_null_flavour(code),
            Group::InstructionState => t.is_valid_instruction_state(code),
            Group::InstructionTransition => t.is_valid_instruction_transition(code),
            Group::ParticipationFunction => t.is_valid_participation_function(code),
            Group::ParticipationMode => t.is_valid_participation_mode(code),
            Group::EventMathFunction => t.is_valid_event_math_function(code),
            Group::TermMappingPurpose => t.is_valid_term_mapping_purpose(code),
            Group::AuditChangeType => t.is_valid_audit_change_type(code),
            Group::AttestationReason => t.is_valid_attestation_reason(code),
            Group::SubjectRelationship => t.is_valid_subject_relationship(code),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Group::CompositionCategory => "composition category",
            Group::Setting => "setting",
            Group::NullFlavour => "null flavour",
            Group::InstructionState => "instruction state",
            Group::InstructionTransition => "instruction transition",
            Group::ParticipationFunction => "participation function",
            Group::ParticipationMode => "participation mode",
            Group::EventMathFunction => "event math function",
            Group::TermMappingPurpose => "term mapping purpose",
            Group::AuditChangeType => "audit change type",
            Group::AttestationReason => "attestation reason",
            Group::SubjectRelationship => "subject relationship",
        }
    }
}

/// A code set (openEHR-internal or ISO/IANA-external) a coded slot must draw
/// from. Checked **without** a `terminology_id` guard — the RM invariant is
/// `code_set(id).has_code(code)`.
#[derive(Clone, Copy)]
enum CodeSet {
    Languages,
    Countries,
    CharacterSets,
    MediaTypes,
    NormalStatuses,
}

impl CodeSet {
    fn is_valid(self, t: &OpenehrTerminology, code: &str) -> bool {
        match self {
            CodeSet::Languages => t.is_valid_language(code),
            CodeSet::Countries => t.is_valid_country(code),
            CodeSet::CharacterSets => t.is_valid_character_set(code),
            CodeSet::MediaTypes => t.is_valid_media_type(code),
            CodeSet::NormalStatuses => t.is_valid_normal_status(code),
        }
    }

    fn label(self) -> &'static str {
        match self {
            CodeSet::Languages => "language (ISO 639-1)",
            CodeSet::Countries => "country (ISO 3166-1)",
            CodeSet::CharacterSets => "character set (IANA)",
            CodeSet::MediaTypes => "media type (IANA)",
            CodeSet::NormalStatuses => "normal status",
        }
    }
}

/// The binding of a coded slot: an openEHR group (guarded) or a code set
/// (unguarded).
#[derive(Clone, Copy)]
enum Binding {
    Group(Group),
    CodeSet(CodeSet),
}

impl Validator {
    pub(super) fn terminology_pass(&mut self, v: &Value, path: &str, _parent_type: Option<&str>) {
        let Some(obj) = v.as_object() else { return };
        let this_type = obj.get("_type").and_then(Value::as_str);

        // Slots fixed by the owning RM type.
        for (attr, binding) in slots_for(this_type) {
            if let Some(node) = obj.get(*attr) {
                self.check_code(node, &format!("{path}/{attr}"), *binding);
            }
        }
        // Slots that may appear on any node, independent of its `_type`:
        //   `null_flavour` (any LOCATABLE), `normal_status` (any DV_ORDERED).
        if let Some(nf) = obj.get("null_flavour") {
            self.check_code(
                nf,
                &format!("{path}/null_flavour"),
                Binding::Group(Group::NullFlavour),
            );
        }
        if let Some(ns) = obj.get("normal_status") {
            self.check_code(
                ns,
                &format!("{path}/normal_status"),
                Binding::CodeSet(CodeSet::NormalStatuses),
            );
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

    /// Validate a coded node against its binding.
    fn check_code(&mut self, node: &Value, path: &str, binding: Binding) {
        let Some((code, terminology)) = openehr_code(node) else {
            return;
        };
        let (valid, label) = match binding {
            Binding::Group(group) => {
                if terminology != "openehr" {
                    return; // out of scope for the openEHR-group check
                }
                (group.is_valid(openehr(), code), group.label())
            }
            // Code-set invariants (`code_set(id).has_code`) are unconditional —
            // the code value is validated against the set regardless of the
            // stated `terminology_id`.
            Binding::CodeSet(cs) => (cs.is_valid(openehr(), code), cs.label()),
        };
        if !valid {
            self.push(
                norm_path(path),
                format!("code '{code}' is not a valid {label} (openEHR terminology)"),
                ValidationKind::Terminology,
            );
        }
    }
}

/// The coded slots fixed by the owning RM type.
fn slots_for(rm_type: Option<&str>) -> &'static [(&'static str, Binding)] {
    use Binding::{CodeSet as Cs, Group as G};
    match rm_type {
        Some("COMPOSITION") => &[
            ("category", G(Group::CompositionCategory)),
            ("language", Cs(CodeSet::Languages)),
            ("territory", Cs(CodeSet::Countries)),
        ],
        Some("EVENT_CONTEXT") => &[("setting", G(Group::Setting))],
        Some("ISM_TRANSITION") => &[
            ("current_state", G(Group::InstructionState)),
            ("transition", G(Group::InstructionTransition)),
        ],
        Some("PARTICIPATION") => &[
            ("function", G(Group::ParticipationFunction)),
            ("mode", G(Group::ParticipationMode)),
        ],
        Some("EVENT" | "POINT_EVENT" | "INTERVAL_EVENT") => {
            &[("math_function", G(Group::EventMathFunction))]
        }
        // Every ENTRY subtype (`ENTRY.language`/`.encoding`) and the DV_TEXT
        // family (`DV_TEXT.language`/`.encoding`): `language` (ISO 639-1) +
        // `encoding` (IANA character sets). Identical slot set, so one arm.
        Some(
            "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY"
            | "GENERIC_ENTRY" | "DV_TEXT" | "DV_CODED_TEXT" | "DV_PARAGRAPH",
        ) => &[
            ("language", Cs(CodeSet::Languages)),
            ("encoding", Cs(CodeSet::CharacterSets)),
        ],
        Some("TERM_MAPPING") => &[("purpose", G(Group::TermMappingPurpose))],
        Some("DV_MULTIMEDIA") => &[("media_type", Cs(CodeSet::MediaTypes))],
        Some("AUDIT_DETAILS") => &[("change_type", G(Group::AuditChangeType))],
        Some("ATTESTATION") => &[("reason", G(Group::AttestationReason))],
        Some("PARTY_RELATED") => &[("relationship", G(Group::SubjectRelationship))],
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
