//! The allocation-free fast path of [`super::validate_rm_value`] (hand-written).
//!
//! No openEHR spec governs this module — it is our own performance design; the
//! *semantics* it realizes are exactly those of the typed dispatch in
//! [`super`] (the RM class invariants of the `*_impl.rs` siblings plus the
//! structural type-conformance rejection of a typed deserialize).
//!
//! # Design: vouch-or-fall-back
//!
//! The typed dispatcher deserializes every `_type` node into its concrete RM
//! type just to run a handful of scalar invariant checks — on a populated
//! composition that is ~1.5k full `serde_json::from_value` runs per commit
//! (each enum-typed field additionally buffering an owned `Value` clone).
//! This module removes that cost for the common case:
//!
//! 1. **Structural conformance** is checked directly against the live
//!    `&serde_json::Value` node, driven by the **generated static RM model**
//!    ([`crate::model`] — the same BMM the structs are generated from), so the
//!    field tables can never drift from the generated types by hand-editing.
//! 2. When the node **conforms**, the class invariants are evaluated straight
//!    off the JSON map via the same `pub(crate)` invariant cores the typed
//!    `Validate` impls call — one source of truth for every violation message,
//!    byte-identical output by construction.
//! 3. When the node does **not** verifiably conform — a shape this checker
//!    does not model (`DV_INTERVAL` limits, `FEEDER_AUDIT`, …), a mandatory
//!    field missing, a wrong JSON kind, an unknown `_type` in a slot — the
//!    caller **falls back to the typed path**, which is authoritative: it
//!    either produces the exact `does not conform to RM type …` serde error or
//!    runs the typed invariants. The fast path never emits a rejection of its
//!    own, so a vouching bug can only degrade to the slow-correct path, never
//!    to a different wire result. Equivalence with the typed path over the
//!    canonical corpus (valid nodes + per-key mutations) is pinned by the
//!    tests below.
//!
//! # The conformance rules mirrored from the deserialize layer
//!
//! The vouch check replicates, conservatively, what `#[derive(OpenEhrType)]`
//! (and the generated `_type`-dispatched slot enums) accept:
//!
//! - mandatory single-valued attribute: present and non-`null` (the derive's
//!   shadow treats a `null` as absent → `missing field`);
//! - optional attribute: absent or `null` is fine;
//! - `Vec` attribute: absent → empty; present must be an array (`null` is a
//!   deserialize error);
//! - `_type` on a slot payload: must name a concrete descendant of the
//!   declared class; absent is only allowed when the declared class is itself
//!   concrete (the generated enums' untagged-default arm) — abstract slots
//!   require it;
//! - primitives: `String`→JSON string, `Boolean`→bool, `Real`→any number,
//!   `Integer`/`Integer64`→integral number in `i32`/`i64` range (floats
//!   rejected, as serde does), `Character`→one-char string;
//! - anything not modelled here (`Hash` attributes, classes outside
//!   [`fast_spec`], the `Interval` default-able bound flags) → fall back.
//!
//! **Shallow mode** mirrors [`super::prune_child_nodes`] for the structural
//! container classes the typed path checks via `run_shallow`: a child
//! *collection* is vouched without descending iff it is empty or contains at
//! least one object (exactly the arrays the prune empties before the typed
//! deserialize), while single-valued nested nodes are still checked (their
//! presence/type is a constraint the shallow deserialize enforces).
//!
//! Generic-parameter erasure: the dispatcher checks `HISTORY` as
//! `History<Value>` and the events as `POINT_EVENT`/`INTERVAL_EVENT` with
//! `data: Value` — so `data` on those two classes accepts anything non-absent
//! ([`generic_any_slot`]), mirroring the monomorphized `Value` payload.

use serde_json::{Map, Value};

use crate::model::{Container, RmClass};
use crate::validate::{
    InvariantViolation, is_valid_iso_date, is_valid_iso_date_time, is_valid_iso_time,
    push_archetype_node_id_valid, push_dv_amount_invariants, push_entry_root_invariants,
    push_magnitude_status_valid, push_temporal_value_valid,
};

/// Validate `value` (whose `_type` is `ty`) on the fast path. Returns `true`
/// when the node was fully handled (invariants appended to `out`); `false`
/// means the caller must run the typed dispatch instead. Nothing is appended
/// to `out` unless `true` is returned.
pub(super) fn try_validate(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) -> bool {
    let Value::Object(obj) = value else {
        return false;
    };
    // Dispatch mode: which classes have a fast invariant evaluator, and
    // whether the typed path would deserialize them shallowly (`run_shallow`)
    // or in full (`run`). Must stay in lockstep with the typed dispatch table
    // in `super::validate_rm_value_typed`.
    let shallow = match ty {
        // `run_shallow` classes (structural containers, scalar-only invariants).
        "CLUSTER" | "POINT_EVENT" | "INTERVAL_EVENT" | "COMPOSITION" | "EVENT_CONTEXT"
        | "ACTIVITY" | "OBSERVATION" | "INSTRUCTION" | "ACTION" | "EVALUATION" | "ADMIN_ENTRY"
        | "SECTION" | "FOLDER" => true,
        // `run` classes with a fast evaluator below.
        "CODE_PHRASE" | "DV_TEXT" | "DV_CODED_TEXT" | "DV_URI" | "DV_EHR_URI" | "DV_IDENTIFIER"
        | "TERM_MAPPING" | "DV_QUANTITY" | "DV_COUNT" | "DV_ORDINAL" | "DV_SCALE"
        | "DV_PROPORTION" | "DV_DURATION" | "DV_DATE" | "DV_TIME" | "DV_DATE_TIME"
        | "DV_PARSABLE" | "ELEMENT" | "HISTORY" | "ARCHETYPED" | "PARTY_IDENTIFIED"
        | "PARTY_RELATED" | "TERMINOLOGY_ID" | "ARCHETYPE_ID" => false,
        // Everything else (rare / complex classes) keeps the typed path.
        _ => return false,
    };
    let Some(spec) = fast_spec(ty) else {
        return false;
    };
    if !node_conforms(obj, spec, shallow) {
        return false;
    }
    run_invariants(ty, obj, out)
}

/// The structural specs this checker models, resolved from the generated
/// static RM model. A class outside this set (or absent from the model) makes
/// the containing node fall back to the typed path. The set covers the
/// composition-content hot path; notable *exclusions* (deliberate — their
/// typed acceptance has semantics this checker does not replicate):
/// `DV_INTERVAL` (default-able bound flags + ordered-limit invariants),
/// `REFERENCE_RANGE`, `DV_MULTIMEDIA` (octet payload), `FEEDER_AUDIT`,
/// `OBJECT_REF`/`LOCATABLE_REF`, `INSTRUCTION_DETAILS`, `PARTICIPATION`,
/// the time-specification family, `DV_STATE`, `DV_PARAGRAPH`,
/// `GENERIC_ENTRY`.
fn fast_spec(name: &str) -> Option<&'static RmClass> {
    let listed = matches!(
        name,
        // identification (BASE)
        "TERMINOLOGY_ID"
            | "ARCHETYPE_ID"
            | "TEMPLATE_ID"
            | "HIER_OBJECT_ID"
            | "OBJECT_VERSION_ID"
            | "GENERIC_ID"
            | "PARTY_REF"
            // text / coded
            | "CODE_PHRASE"
            | "DV_TEXT"
            | "DV_CODED_TEXT"
            | "TERM_MAPPING"
            // uri / basic / encapsulated
            | "DV_URI"
            | "DV_EHR_URI"
            | "DV_IDENTIFIER"
            | "DV_BOOLEAN"
            | "DV_PARSABLE"
            // ordered / quantified
            | "DV_QUANTITY"
            | "DV_COUNT"
            | "DV_ORDINAL"
            | "DV_SCALE"
            | "DV_PROPORTION"
            | "DV_DURATION"
            | "DV_DATE"
            | "DV_TIME"
            | "DV_DATE_TIME"
            // common
            | "ARCHETYPED"
            | "LINK"
            | "PARTY_IDENTIFIED"
            | "PARTY_RELATED"
            | "PARTY_SELF"
            // data structures
            | "ELEMENT"
            | "CLUSTER"
            | "ITEM_TREE"
            | "ITEM_LIST"
            | "ITEM_SINGLE"
            | "ITEM_TABLE"
            | "HISTORY"
            | "POINT_EVENT"
            | "INTERVAL_EVENT"
            // composition
            | "COMPOSITION"
            | "EVENT_CONTEXT"
            | "SECTION"
            | "OBSERVATION"
            | "EVALUATION"
            | "INSTRUCTION"
            | "ACTION"
            | "ADMIN_ENTRY"
            | "ACTIVITY"
            | "ISM_TRANSITION"
            | "FOLDER"
    );
    if listed {
        crate::model::class(name)
    } else {
        None
    }
}

/// `HISTORY` is dispatched as `History<Value>` and its events as
/// `POINT_EVENT`/`INTERVAL_EVENT` with `data: Value` (generic parameter
/// monomorphized away), so `data` on the event classes accepts any non-absent,
/// non-`null` payload.
fn generic_any_slot(class: &str, attr: &str) -> bool {
    attr == "data" && matches!(class, "POINT_EVENT" | "INTERVAL_EVENT")
}

/// Whether the typed deserialize of `class` verifiably accepts `obj`. `false`
/// means "cannot vouch" (not "invalid") — the caller falls back to the typed
/// path, which decides authoritatively.
///
/// PERF: iterates the node's entries once and matches attribute names by
/// static-string compare instead of one hashed `obj.get` per attribute — this
/// runs for every `_type` node of every commit, and the map hashing was the
/// measured residual cost. Mandatory-presence is closed out by counting the
/// mandatory single-valued attributes seen against the class's total.
fn node_conforms(obj: &Map<String, Value>, class: &'static RmClass, shallow: bool) -> bool {
    let mut mandatory_seen = 0usize;
    for (key, v) in obj {
        if key == "_type" {
            continue; // validated by the caller's slot/tag dispatch
        }
        // Unknown wire keys are ignored by the derive's deserialize.
        let Some(attr) = class.attributes.iter().find(|a| a.name == key) else {
            continue;
        };
        match attr.container {
            Container::None => {
                if v.is_null() {
                    // The derive's shadow reads every field as `Option`, so a
                    // JSON `null` is "absent": fine for an optional attribute,
                    // `missing field` for a mandatory one.
                    if attr.is_mandatory {
                        return false;
                    }
                    continue;
                }
                if attr.is_mandatory {
                    mandatory_seen += 1;
                }
                if generic_any_slot(class.name, attr.name) {
                    continue;
                }
                if !value_conforms(v, attr.declared_type, shallow) {
                    return false;
                }
            }
            Container::List | Container::Set => match v {
                Value::Array(items) => {
                    if shallow {
                        // Mirror `prune_child_nodes`: an array containing at
                        // least one object is emptied before the shallow typed
                        // deserialize (so its contents never matter), and an
                        // empty array trivially deserializes. A non-empty
                        // all-scalar array is kept by the prune and typed-
                        // checked — don't vouch for it.
                        if !(items.is_empty() || items.iter().any(Value::is_object)) {
                            return false;
                        }
                    } else {
                        for item in items {
                            if !value_conforms(item, attr.declared_type, false) {
                                return false;
                            }
                        }
                    }
                }
                // `Vec` never deserializes from a non-array (incl. `null`).
                _ => return false,
            },
            // No `Hash` attribute is modelled here.
            Container::Hash => return false,
        }
    }
    // Absent attributes: `Vec` defaults to empty and `Option` to `None`, but a
    // missing plain mandatory attribute fails the typed deserialize — every
    // mandatory single-valued attribute must have been seen (non-null).
    mandatory_seen
        == class
            .attributes
            .iter()
            .filter(|a| a.is_mandatory && a.container == Container::None)
            .count()
}

/// Whether a single value verifiably deserializes as the declared spec type.
fn value_conforms(v: &Value, declared: &str, shallow: bool) -> bool {
    match declared {
        "String" => v.is_string(),
        "Boolean" => v.is_boolean(),
        // Any JSON number deserializes as `f64`.
        "Real" | "Double" => v.is_number(),
        // serde rejects floats for integer targets; range-check the rest.
        "Integer" => v.as_i64().is_some_and(|n| i32::try_from(n).is_ok()),
        "Integer64" => v.as_i64().is_some(),
        // serde `char`: a one-character string.
        "Character" => v.as_str().is_some_and(|s| s.chars().count() == 1),
        _ => class_slot_conforms(v, declared, shallow),
    }
}

/// Whether an object in a slot declared as spec class `declared` verifiably
/// deserializes: the payload `_type` (validated by every generated
/// `Deserialize`) selects a concrete descendant; a `_type`-less payload is
/// only accepted by a slot whose declared class is itself concrete (the
/// untagged-default arm of the generated enums — an abstract slot errors).
fn class_slot_conforms(v: &Value, declared: &str, shallow: bool) -> bool {
    let Value::Object(obj) = v else {
        return false;
    };
    let Some(slot) = crate::model::class(declared) else {
        return false;
    };
    // Canonical JSON emits `_type` as the first key; peek there before paying
    // a hashed lookup (PERF — this runs per nested slot of every node).
    let tag = match obj.iter().next() {
        Some((k, v)) if k == "_type" => Some(v),
        _ => obj.get("_type"),
    };
    let target = match tag {
        Some(Value::String(t)) => {
            if !slot.descendants.contains(&t.as_str()) {
                return false;
            }
            t.as_str()
        }
        // A non-string `_type` fails the shadow's `Option<String>` read.
        Some(_) => return false,
        None => {
            if slot.is_abstract {
                return false;
            }
            declared
        }
    };
    let Some(spec) = fast_spec(target) else {
        return false;
    };
    node_conforms(obj, spec, shallow)
}

// ── invariant evaluation on the conformed node ───────────────────────────────

/// Linear-scan field access: an RM node has ≤ ~10 keys, so a sequential
/// static-string compare beats the map's hashed lookup on this hot path
/// (PERF — same reasoning as the entry iteration in [`node_conforms`]).
fn field<'a>(obj: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    obj.iter().find_map(|(k, v)| (k == key).then_some(v))
}

/// The attribute value as `&str` (`None` for absent or `null` — the typed
/// `Option<String>` reading).
fn str_of<'a>(obj: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    field(obj, key).and_then(Value::as_str)
}

/// The attribute value as `f64` (`None` for absent/`null`).
fn f64_of(obj: &Map<String, Value>, key: &str) -> Option<f64> {
    field(obj, key).and_then(Value::as_f64)
}

/// The attribute value as `bool` (`None` for absent/`null`).
fn bool_of(obj: &Map<String, Value>, key: &str) -> Option<bool> {
    field(obj, key).and_then(Value::as_bool)
}

/// Whether the attribute is present and non-`null` (the typed
/// `Option<T>::is_some` reading).
fn present(obj: &Map<String, Value>, key: &str) -> bool {
    field(obj, key).is_some_and(|v| !v.is_null())
}

/// Run the class invariants for the (structurally conformed) node, appending
/// via the same `pub(crate)` cores the typed `Validate` impls call — the
/// violation text has a single source. Returns `false` (nothing appended)
/// for the rare sub-cases the fast path declines (e.g. a periodic `HISTORY`,
/// whose `Period_consistency` needs the typed event/offset arithmetic).
#[allow(clippy::too_many_lines)] // a flat per-class dispatch, mirror of the typed table
fn run_invariants(ty: &str, obj: &Map<String, Value>, out: &mut Vec<InvariantViolation>) -> bool {
    match ty {
        "CODE_PHRASE" => {
            let Some(code_string) = str_of(obj, "code_string") else {
                return false;
            };
            crate::data_types::text::code_phrase_impl::push_code_phrase_invariants(
                code_string,
                out,
            );
        }
        "DV_TEXT" | "DV_CODED_TEXT" => {
            let Some(value) = str_of(obj, "value") else {
                return false;
            };
            crate::data_types::text::dv_text_impl::push_dv_text_invariants(
                ty,
                value,
                str_of(obj, "formatting"),
                out,
            );
        }
        "DV_URI" => {
            let Some(value) = str_of(obj, "value") else {
                return false;
            };
            crate::data_types::uri::dv_uri_impl::push_dv_uri_invariants(value, out);
        }
        "DV_EHR_URI" => {
            let Some(value) = str_of(obj, "value") else {
                return false;
            };
            crate::data_types::uri::dv_ehr_uri_impl::push_dv_ehr_uri_invariants(value, out);
        }
        "DV_IDENTIFIER" => {
            let Some(id) = str_of(obj, "id") else {
                return false;
            };
            crate::data_types::basic::dv_identifier_impl::push_dv_identifier_invariants(id, out);
        }
        "TERM_MAPPING" => {
            let Some(code) = str_of(obj, "match").and_then(|s| s.chars().next()) else {
                return false;
            };
            crate::data_types::text::term_mapping_impl::push_term_mapping_invariants(code, out);
        }
        "DV_PARSABLE" => {
            let Some(formalism) = str_of(obj, "formalism") else {
                return false;
            };
            crate::data_types::encapsulated::dv_parsable_impl::push_dv_parsable_invariants(
                formalism, out,
            );
        }
        // DV_AMOUNT descendants. `normal_range` (the only input of the
        // DV_ORDERED consistency invariant that could fire) is a DV_INTERVAL,
        // which `node_conforms` never vouches for — so on this path the
        // consistency check is vacuously satisfied, exactly as the typed
        // `push_normal_range_consistency` no-ops without a range.
        "DV_QUANTITY" | "DV_COUNT" | "DV_DURATION" => {
            if ty == "DV_DURATION" {
                let Some(value) = str_of(obj, "value") else {
                    return false;
                };
                push_temporal_value_valid(out, ty, crate::validate::is_valid_iso_duration(value));
            }
            push_dv_amount_invariants(
                out,
                ty,
                f64_of(obj, "accuracy"),
                bool_of(obj, "accuracy_is_percent"),
                str_of(obj, "magnitude_status"),
            );
        }
        "DV_DATE" | "DV_TIME" | "DV_DATE_TIME" => {
            let Some(value) = str_of(obj, "value") else {
                return false;
            };
            let valid = match ty {
                "DV_DATE" => is_valid_iso_date(value),
                "DV_TIME" => is_valid_iso_time(value),
                _ => is_valid_iso_date_time(value),
            };
            push_temporal_value_valid(out, ty, valid);
            push_magnitude_status_valid(out, ty, str_of(obj, "magnitude_status"));
        }
        // Only the DV_ORDERED consistency invariant, which cannot fire
        // without a (never-vouched) normal_range.
        "DV_ORDINAL" | "DV_SCALE" => {}
        "DV_PROPORTION" => {
            let (Some(numerator), Some(denominator)) =
                (f64_of(obj, "numerator"), f64_of(obj, "denominator"))
            else {
                return false;
            };
            let Some(kind) = obj
                .get("type")
                .and_then(Value::as_i64)
                .and_then(|n| i32::try_from(n).ok())
            else {
                return false;
            };
            let precision = obj
                .get("precision")
                .and_then(Value::as_i64)
                .and_then(|n| i32::try_from(n).ok());
            crate::data_types::quantity::dv_proportion_impl::push_dv_proportion_invariants(
                numerator,
                denominator,
                kind,
                precision,
                out,
            );
            push_dv_amount_invariants(
                out,
                ty,
                f64_of(obj, "accuracy"),
                bool_of(obj, "accuracy_is_percent"),
                str_of(obj, "magnitude_status"),
            );
        }
        "ELEMENT" => {
            let Some(node_id) = str_of(obj, "archetype_node_id") else {
                return false;
            };
            crate::data_structures::representation::element_impl::push_element_invariants(
                present(obj, "value"),
                present(obj, "null_flavour"),
                present(obj, "null_reason"),
                node_id,
                out,
            );
        }
        // LOCATABLE containers whose only class invariant is the inherited
        // Archetype_node_id_valid.
        "CLUSTER" | "SECTION" | "FOLDER" | "POINT_EVENT" | "INTERVAL_EVENT" => {
            let Some(node_id) = str_of(obj, "archetype_node_id") else {
                return false;
            };
            push_archetype_node_id_valid(out, ty, node_id);
        }
        "HISTORY" => {
            // Period_consistency needs the typed event-offset arithmetic —
            // decline periodic histories (rare) to the typed path.
            if present(obj, "period") {
                return false;
            }
            let Some(node_id) = str_of(obj, "archetype_node_id") else {
                return false;
            };
            let events_empty = obj
                .get("events")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty);
            crate::data_structures::history::history_impl::push_history_basic_invariants(
                events_empty,
                present(obj, "summary"),
                node_id,
                out,
            );
        }
        "COMPOSITION" => {
            let Some(node_id) = str_of(obj, "archetype_node_id") else {
                return false;
            };
            crate::composition::composition_impl::push_composition_invariants(
                present(obj, "archetype_details"),
                node_id,
                out,
            );
        }
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" => {
            let Some(node_id) = str_of(obj, "archetype_node_id") else {
                return false;
            };
            push_entry_root_invariants(out, ty, present(obj, "archetype_details"), node_id);
        }
        "ACTIVITY" => {
            let (Some(action_archetype_id), Some(node_id)) = (
                str_of(obj, "action_archetype_id"),
                str_of(obj, "archetype_node_id"),
            ) else {
                return false;
            };
            crate::composition::content::entry::activity_impl::push_activity_invariants(
                action_archetype_id,
                node_id,
                out,
            );
        }
        "EVENT_CONTEXT" => {
            crate::composition::event_context_impl::push_event_context_invariants(
                str_of(obj, "location"),
                out,
            );
        }
        "ARCHETYPED" => {
            let Some(rm_version) = str_of(obj, "rm_version") else {
                return false;
            };
            crate::common::archetyped::archetyped_impl::push_archetyped_invariants(rm_version, out);
        }
        "PARTY_IDENTIFIED" | "PARTY_RELATED" => {
            let has_identifiers = obj
                .get("identifiers")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
            crate::common::generic::party_identified_impl::push_party_identified_invariants(
                ty,
                str_of(obj, "name"),
                has_identifiers,
                present(obj, "external_ref"),
                out,
            );
        }
        // Single-`value` identifier classes: the invariant cores live in
        // `openehr-base`; constructing the (one-string) typed value keeps the
        // violation text single-sourced there at the cost of one allocation.
        "TERMINOLOGY_ID" => {
            let Some(value) = str_of(obj, "value") else {
                return false;
            };
            use crate::validate::Validate as _;
            openehr_base::prelude::TerminologyId {
                value: value.to_owned(),
            }
            .validate_invariants(out);
        }
        "ARCHETYPE_ID" => {
            let Some(value) = str_of(obj, "value") else {
                return false;
            };
            use crate::validate::Validate as _;
            openehr_base::prelude::ArchetypeId {
                value: value.to_owned(),
            }
            .validate_invariants(out);
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Run only the typed (fallback) dispatch — the oracle.
    fn typed(value: &Value) -> Vec<InvariantViolation> {
        let mut out = Vec::new();
        if let Some(ty) = value.get("_type").and_then(Value::as_str) {
            crate::validate::validate_rm_value_typed(ty, value, &mut out);
        }
        out
    }

    /// Run the public two-tier entry point.
    fn two_tier(value: &Value) -> Vec<InvariantViolation> {
        let mut out = Vec::new();
        crate::validate::validate_rm_value(value, &mut out);
        out
    }

    /// Whether the fast path handled the node (nothing appended on `false`).
    fn fast_handled(value: &Value) -> bool {
        let Some(ty) = value.get("_type").and_then(Value::as_str) else {
            return false;
        };
        let mut out = Vec::new();
        try_validate(ty, value, &mut out)
    }

    #[test]
    fn fast_path_matches_typed_on_simple_nodes() {
        let cases = [
            json!({"_type": "CODE_PHRASE",
                   "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                   "code_string": "433"}),
            json!({"_type": "CODE_PHRASE",
                   "terminology_id": {"value": "openehr"}, "code_string": ""}),
            json!({"_type": "DV_TEXT", "value": "hello"}),
            json!({"_type": "DV_TEXT", "value": "", "formatting": ""}),
            json!({"_type": "DV_CODED_TEXT", "value": "event",
                   "defining_code": {"terminology_id": {"value": "openehr"},
                                     "code_string": "433"}}),
            json!({"_type": "DV_QUANTITY", "magnitude": 120.0, "units": "mm[Hg]",
                   "accuracy": 0.0, "accuracy_is_percent": true}),
            json!({"_type": "DV_COUNT", "magnitude": 3}),
            json!({"_type": "DV_PROPORTION", "numerator": 1.0, "denominator": 3.0,
                   "type": 2}),
            json!({"_type": "DV_DURATION", "value": "P1DT2H"}),
            json!({"_type": "DV_DURATION", "value": "nonsense"}),
            json!({"_type": "DV_DATE", "value": "2021-02-31"}),
            json!({"_type": "DV_DATE_TIME", "value": "2021-05-17T10:30:00Z",
                   "magnitude_status": "??"}),
            json!({"_type": "DV_TIME", "value": "10:30:00"}),
            json!({"_type": "DV_IDENTIFIER", "id": ""}),
            json!({"_type": "DV_PARSABLE", "value": "x", "formalism": ""}),
            json!({"_type": "TERM_MAPPING", "match": "=",
                   "target": {"terminology_id": {"value": "x"}, "code_string": "1"}}),
            json!({"_type": "TERM_MAPPING", "match": "q",
                   "target": {"terminology_id": {"value": "x"}, "code_string": "1"}}),
            json!({"_type": "DV_URI", "value": ""}),
            json!({"_type": "DV_EHR_URI", "value": "http://not-ehr"}),
            json!({"_type": "TERMINOLOGY_ID", "value": "SNOMED CT "}),
            json!({"_type": "ARCHETYPE_ID",
                   "value": "openEHR-EHR-OBSERVATION.blood_pressure.v1"}),
            json!({"_type": "ARCHETYPE_ID", "value": "not-an-archetype-id"}),
            json!({"_type": "ARCHETYPED",
                   "archetype_id": {"value": "openEHR-EHR-COMPOSITION.x.v1"},
                   "rm_version": ""}),
            json!({"_type": "PARTY_IDENTIFIED"}),
            json!({"_type": "PARTY_IDENTIFIED", "name": ""}),
            json!({"_type": "EVENT_CONTEXT",
                   "start_time": {"value": "2021-05-17T10:00:00Z"},
                   "setting": {"value": "home",
                               "defining_code": {"terminology_id": {"value": "openehr"},
                                                 "code_string": "225"}},
                   "location": ""}),
        ];
        for node in &cases {
            assert!(
                fast_handled(node),
                "expected the fast path to handle {node}"
            );
            assert_eq!(two_tier(node), typed(node), "divergence on {node}");
        }
    }

    #[test]
    fn element_xor_matches_typed() {
        let name = json!({"value": "systolic"});
        let value = json!({"_type": "DV_QUANTITY", "magnitude": 120.0, "units": "mm[Hg]"});
        let nf = json!({"_type": "DV_CODED_TEXT", "value": "unknown",
                        "defining_code": {"terminology_id": {"value": "openehr"},
                                          "code_string": "253"}});
        for element in [
            json!({"_type": "ELEMENT", "name": name, "archetype_node_id": "at0001",
                   "value": value}),
            json!({"_type": "ELEMENT", "name": name, "archetype_node_id": "at0001",
                   "null_flavour": nf}),
            json!({"_type": "ELEMENT", "name": name, "archetype_node_id": "at0001",
                   "value": value, "null_flavour": nf}),
            json!({"_type": "ELEMENT", "name": name, "archetype_node_id": "",
                   "value": value, "null_reason": {"value": "why"}}),
        ] {
            assert!(fast_handled(&element), "not handled: {element}");
            assert_eq!(two_tier(&element), typed(&element), "on {element}");
        }
    }

    #[test]
    fn nonconforming_nodes_fall_back_with_identical_output() {
        // Each of these fails the typed deserialize → the fast path must
        // decline and the two-tier output must equal the typed output
        // (`does not conform to RM type …`).
        let cases = [
            // mandatory field missing
            json!({"_type": "DV_QUANTITY", "units": "kg"}),
            // mandatory field null
            json!({"_type": "DV_TEXT", "value": null}),
            // wrong scalar kind
            json!({"_type": "DV_TEXT", "value": 42}),
            // float where integer expected
            json!({"_type": "DV_COUNT", "magnitude": 3.5}),
            // i32 overflow
            json!({"_type": "DV_PROPORTION", "numerator": 1.0, "denominator": 1.0,
                   "type": 4_000_000_000_i64}),
            // wrong nested _type in a slot
            json!({"_type": "CODE_PHRASE",
                   "terminology_id": {"_type": "DV_TEXT", "value": "x"},
                   "code_string": "1"}),
            // abstract slot without _type (ELEMENT.value is DATA_VALUE)
            json!({"_type": "ELEMENT", "name": {"value": "n"},
                   "archetype_node_id": "at0001", "value": {"value": "x"}}),
            // Vec from null
            json!({"_type": "DV_TEXT", "value": "ok", "mappings": null}),
            // char from multi-char string
            json!({"_type": "TERM_MAPPING", "match": "==",
                   "target": {"terminology_id": {"value": "x"}, "code_string": "1"}}),
        ];
        for node in &cases {
            assert!(!fast_handled(node), "fast path must not vouch for {node}");
            let t = typed(node);
            assert_eq!(two_tier(node), t, "fallback divergence on {node}");
            assert!(
                !t.is_empty(),
                "the typed oracle should reject {node}, got clean"
            );
        }
    }

    #[test]
    fn unmodelled_shapes_fall_back() {
        // normal_range is a DV_INTERVAL — never vouched.
        let with_range = json!({"_type": "DV_COUNT", "magnitude": 1,
            "normal_status": {"terminology_id": {"value": "openehr"},
                              "code_string": "N"},
            "normal_range": {"lower": {"_type": "DV_COUNT", "magnitude": 0},
                             "upper": {"_type": "DV_COUNT", "magnitude": 5},
                             "lower_unbounded": false, "upper_unbounded": false,
                             "lower_included": true, "upper_included": true}});
        assert!(!fast_handled(&with_range));
        assert_eq!(two_tier(&with_range), typed(&with_range));

        // periodic HISTORY declines to the typed Period_consistency check.
        let periodic = json!({"_type": "HISTORY", "name": {"value": "h"},
            "archetype_node_id": "at0001",
            "origin": {"value": "2021-05-17T10:00:00Z"},
            "period": {"value": "PT1H"},
            "events": [{"_type": "POINT_EVENT", "name": {"value": "e"},
                        "archetype_node_id": "at0002",
                        "time": {"value": "2021-05-17T10:30:00Z"},
                        "data": {"_type": "ITEM_TREE", "name": {"value": "d"},
                                 "archetype_node_id": "at0003", "items": []}}]});
        assert!(!fast_handled(&periodic));
        assert_eq!(two_tier(&periodic), typed(&periodic));

        // A class outside the fast set keeps the typed path untouched.
        let multimedia = json!({"_type": "DV_MULTIMEDIA",
            "media_type": {"terminology_id": {"value": "IANA_media-types"},
                           "code_string": "image/png"},
            "size": 100});
        assert!(!fast_handled(&multimedia));
        assert_eq!(two_tier(&multimedia), typed(&multimedia));
    }

    #[test]
    fn shallow_collections_mirror_the_prune() {
        // Mixed object/scalar array: the prune empties it → both paths accept.
        let mixed = json!({"_type": "CLUSTER", "name": {"value": "c"},
            "archetype_node_id": "at0001",
            "items": [{"_type": "ELEMENT", "name": {"value": "e"},
                       "archetype_node_id": "", "value": {"_type": "DV_COUNT",
                                                          "magnitude": 1}}, "stray"]});
        assert!(fast_handled(&mixed));
        assert_eq!(two_tier(&mixed), typed(&mixed));

        // Non-empty all-scalar array is kept by the prune (typed rejects) —
        // the fast path must decline, and outputs must match.
        let scalars = json!({"_type": "CLUSTER", "name": {"value": "c"},
            "archetype_node_id": "at0001", "items": ["not-an-item"]});
        assert!(!fast_handled(&scalars));
        assert_eq!(two_tier(&scalars), typed(&scalars));
    }

    // ── corpus equivalence: the load-bearing oracle ──────────────────────────

    /// Every `_type`-bearing object node in `v`, depth-first.
    fn collect_nodes<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
        match v {
            Value::Object(map) => {
                if map.get("_type").is_some_and(Value::is_string) {
                    out.push(v);
                }
                for child in map.values() {
                    collect_nodes(child, out);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect_nodes(item, out);
                }
            }
            _ => {}
        }
    }

    fn corpus_files() -> Vec<std::path::PathBuf> {
        let mut roots = vec![std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../openehr-its/tests/vendor/openehr_sdk"
        ))];
        // The benchmark CKM examples exercise the exact hot commit shapes.
        roots.push(std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/benchmark/templates/ckm"
        )));
        let mut files = Vec::new();
        while let Some(dir) = roots.pop() {
            let entries = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("read corpus dir {}: {e}", dir.display()));
            for entry in entries {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    roots.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    files.push(path);
                }
            }
        }
        files.sort();
        assert!(
            files.len() >= 50,
            "corpus went missing? found only {} json files",
            files.len()
        );
        files
    }

    /// Every corpus node must produce byte-identical violations through the
    /// two-tier entry point and the typed oracle.
    #[test]
    fn corpus_equivalence_valid_nodes() {
        let mut total = 0usize;
        let mut fast = 0usize;
        for path in corpus_files() {
            let text = std::fs::read_to_string(&path).expect("read corpus file");
            let Ok(doc) = serde_json::from_str::<Value>(&text) else {
                continue; // non-RM json (e.g. web templates) — skip unparseable
            };
            let mut nodes = Vec::new();
            collect_nodes(&doc, &mut nodes);
            for node in nodes {
                total += 1;
                if fast_handled(node) {
                    fast += 1;
                }
                assert_eq!(
                    two_tier(node),
                    typed(node),
                    "divergence in {} on {node}",
                    path.display()
                );
            }
        }
        eprintln!("corpus equivalence: {total} nodes, {fast} fast-handled");
        assert!(total > 3_000, "expected a real corpus, saw {total} nodes");
    }

    /// Mutation equivalence: for the first-seen (`_type`, key) pair in the
    /// corpus, mutate that key through a battery of shape changes and assert
    /// the two-tier output still equals the typed oracle. This is the drift
    /// net for the model-vs-generated-struct agreement the fast path rests on.
    #[test]
    fn corpus_equivalence_mutated_nodes() {
        let mutations: &[Value] = &[
            Value::Null,
            json!(42),
            json!(3.5),
            json!("mutated"),
            json!(""),
            json!(true),
            json!([]),
            json!({}),
            json!([42]),
            json!([{}]),
            json!({"_type": "DV_QUANTITY"}),
        ];
        let mut seen = std::collections::HashSet::new();
        let mut checked = 0usize;
        for path in corpus_files() {
            let text = std::fs::read_to_string(&path).expect("read corpus file");
            let Ok(doc) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let mut nodes = Vec::new();
            collect_nodes(&doc, &mut nodes);
            for node in nodes {
                let Value::Object(map) = node else { continue };
                let ty = map
                    .get("_type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if key == "_type" || !seen.insert((ty.clone(), key.clone())) {
                        continue;
                    }
                    // Removal.
                    let mut removed = map.clone();
                    removed.shift_remove(&key);
                    let removed = Value::Object(removed);
                    assert_eq!(
                        two_tier(&removed),
                        typed(&removed),
                        "divergence removing {ty}.{key}"
                    );
                    checked += 1;
                    // Shape battery.
                    for m in mutations {
                        let mut mutated = map.clone();
                        mutated.insert(key.clone(), m.clone());
                        let mutated = Value::Object(mutated);
                        assert_eq!(
                            two_tier(&mutated),
                            typed(&mutated),
                            "divergence mutating {ty}.{key} to {m}"
                        );
                        checked += 1;
                    }
                }
                // An unknown key must stay ignored on both paths.
                if seen.insert((ty.clone(), "__unknown__".into())) {
                    let mut extra = map.clone();
                    extra.insert("__unknown_key__".into(), json!(42));
                    let extra = Value::Object(extra);
                    assert_eq!(
                        two_tier(&extra),
                        typed(&extra),
                        "divergence adding unknown key on {ty}"
                    );
                    checked += 1;
                }
            }
        }
        eprintln!("mutation equivalence: {checked} mutated nodes checked");
        assert!(checked > 500, "mutation battery too small: {checked}");
    }

    /// TEMP diagnostic: per-_type fast/fallback counts on the IPS example.
    #[test]
    fn temp_ips_fallback_profile() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/benchmark/templates/ckm/international-patient-summary.example.json"
        );
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("parse");
        let mut nodes = Vec::new();
        collect_nodes(&doc, &mut nodes);
        let mut counts: std::collections::BTreeMap<(String, bool), usize> = Default::default();
        for node in nodes {
            let ty = node
                .get("_type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let mut out = Vec::new();
            let handled = try_validate(&ty, node, &mut out);
            *counts.entry((ty, handled)).or_default() += 1;
        }
        for ((ty, handled), n) in &counts {
            eprintln!("{:30} fast={} count={}", ty, handled, n);
        }
    }

    /// TEMP diagnostic 2: time validate_rm_value over all IPS nodes.
    #[test]
    fn temp_ips_validate_timing() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/benchmark/templates/ckm/international-patient-summary.example.json"
        );
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("parse");
        let mut nodes = Vec::new();
        collect_nodes(&doc, &mut nodes);
        for _ in 0..5 {
            for n in &nodes {
                let mut out = Vec::new();
                crate::validate::validate_rm_value(n, &mut out);
                std::hint::black_box(out.len());
            }
        }
        let iters = 50u32;
        let start = std::time::Instant::now();
        for _ in 0..iters {
            for n in &nodes {
                let mut out = Vec::new();
                crate::validate::validate_rm_value(n, &mut out);
                std::hint::black_box(out.len());
            }
        }
        let fast_us = start.elapsed().as_secs_f64() * 1e6 / f64::from(iters);
        let start = std::time::Instant::now();
        for _ in 0..iters {
            for n in &nodes {
                let mut out = Vec::new();
                let ty = n.get("_type").and_then(Value::as_str).unwrap_or("");
                crate::validate::validate_rm_value_typed(ty, n, &mut out);
                std::hint::black_box(out.len());
            }
        }
        let typed_us = start.elapsed().as_secs_f64() * 1e6 / f64::from(iters);
        eprintln!("all-nodes validate_rm_value (two-tier): {fast_us:.1} us");
        eprintln!("all-nodes typed-only               : {typed_us:.1} us");
    }

    /// The hot commit shape must actually ride the fast path: on the populated
    /// IPS example the overwhelming majority of dispatched nodes are handled
    /// without a typed deserialize. Guards the perf property against silent
    /// coverage regressions (a model/struct drift would first show up here).
    #[test]
    fn ips_nodes_ride_the_fast_path() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/benchmark/templates/ckm/international-patient-summary.example.json"
        );
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read IPS example"))
                .expect("parse IPS example");
        let mut nodes = Vec::new();
        collect_nodes(&doc, &mut nodes);
        // Only count nodes the typed dispatch table handles at all.
        let mut dispatched = 0usize;
        let mut fast = 0usize;
        for node in nodes {
            let mut out = Vec::new();
            let ty = node.get("_type").and_then(Value::as_str).unwrap_or("");
            crate::validate::validate_rm_value_typed(ty, node, &mut out);
            let mut fast_out = Vec::new();
            let handled = try_validate(ty, node, &mut fast_out);
            if handled {
                fast += 1;
                assert_eq!(fast_out, out, "IPS divergence on {node}");
            }
            // Count classes with a fast evaluator as dispatch-relevant.
            if matches!(
                ty,
                "CODE_PHRASE"
                    | "DV_TEXT"
                    | "DV_CODED_TEXT"
                    | "DV_QUANTITY"
                    | "DV_COUNT"
                    | "DV_DATE_TIME"
                    | "ELEMENT"
                    | "CLUSTER"
                    | "OBSERVATION"
                    | "SECTION"
                    | "COMPOSITION"
                    | "TERMINOLOGY_ID"
            ) {
                dispatched += 1;
            }
        }
        assert!(
            fast * 10 >= dispatched * 9,
            "fast-path coverage regressed: {fast} fast of {dispatched} hot nodes"
        );
    }
}
