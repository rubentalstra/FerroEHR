//! RM structural-node builders + mandatory-field completion shared by the
//! FLAT/STRUCTURED reverse converter.
//!
//! The web-template compacts the RM structure it does not need for data entry
//! (`HISTORY`, a single `EVENT`, `ITEM_TREE`, the `ELEMENT` wrapper, and the
//! never-surfaced identity/occurrence fields of `ACTIVITY`/`ACTION`/
//! `INTERVAL_EVENT`). Rebuilding a canonical [`openehr_rm`] value therefore
//! requires re-materialising those nodes and filling the RM-mandatory fields
//! that FLAT never carries. This module is the single source of truth for those
//! defaults so [`super::from_flat`] and the structured path share it (a value
//! filled here never appears in FLAT, so it does not affect the round-trip).

use serde_json::{Map, Value, json};

/// A canonical `DV_DATE_TIME` leaf.
pub(crate) fn dv_date_time(time: &str) -> Value {
    json!({"_type": "DV_DATE_TIME", "value": time})
}

/// A canonical `CODE_PHRASE`.
pub(crate) fn code_phrase(terminology: &str, code: &str) -> Value {
    json!({
        "_type": "CODE_PHRASE",
        "terminology_id": {"_type": "TERMINOLOGY_ID", "value": terminology},
        "code_string": code,
    })
}

/// A canonical `DV_CODED_TEXT`.
pub(crate) fn dv_coded_text(value: &str, terminology: &str, code: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT",
        "value": value,
        "defining_code": code_phrase(terminology, code),
    })
}

/// An empty `ITEM_TREE` named `Tree` (the compacted-away data structure).
pub(crate) fn empty_item_tree() -> Value {
    json!({"_type": "ITEM_TREE", "name": {"_type": "DV_TEXT", "value": "Tree"}, "items": []})
}

/// Fill the RM-mandatory fields of `obj` (an RM node of `rm_type`) that are not
/// surfaced in FLAT and are therefore absent after the datum-driven build.
///
/// Only *missing* fields are added (`or_insert_with`), so a value the build
/// already produced from a populated leaf is never overwritten — the round-trip
/// stays stable.
pub(crate) fn fill_structural_mandatory(obj: &mut Map<String, Value>, rm_type: &str, time: &str) {
    match rm_type {
        "HISTORY" => {
            obj.entry("origin".to_owned())
                .or_insert_with(|| dv_date_time(time));
            obj.entry("events".to_owned()).or_insert_with(|| json!([]));
        }
        "POINT_EVENT" | "EVENT" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| dv_date_time(time));
            obj.entry("data".to_owned()).or_insert_with(empty_item_tree);
        }
        "INTERVAL_EVENT" => {
            obj.entry("time".to_owned())
                .or_insert_with(|| dv_date_time(time));
            obj.entry("data".to_owned()).or_insert_with(empty_item_tree);
            // `width` + `math_function` are RM-mandatory on INTERVAL_EVENT but
            // are never data-entry leaves (openEHR `146` = "mean").
            obj.entry("width".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DURATION", "value": "P0D"}));
            obj.entry("math_function".to_owned())
                .or_insert_with(|| dv_coded_text("mean", "openehr", "146"));
        }
        "ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE" | "CLUSTER" => {
            obj.entry("items".to_owned()).or_insert_with(|| json!([]));
        }
        "ACTIVITY" => {
            // `action_archetype_id` (regex over allowed Action archetypes,
            // default `/.*/`) and `description` are RM-mandatory but never leaves.
            obj.entry("action_archetype_id".to_owned())
                .or_insert_with(|| json!("/.*/"));
            obj.entry("description".to_owned())
                .or_insert_with(empty_item_tree);
        }
        "ISM_TRANSITION" => {
            obj.entry("current_state".to_owned())
                .or_insert_with(|| dv_coded_text("initial", "openehr", "524"));
        }
        _ => {}
    }
}
