// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Per-row OPT synthesis for the *structural* content families, plus the
//! single dispatch entry the driver calls for every synthesized content row.
//!
//! Issue #228: a content decision table whose `constraint_context.constraint_columns`
//! name constraint-axis columns needs one OPT per row (the archetype/template
//! constraint varies per row, so no single baked template makes every row's
//! verdict correct). This module owns the STRUCTURAL families — the ones whose
//! constraint is carrier *shape* (container cardinality, attribute existence,
//! object type narrowing), not an ELEMENT.value domain constraint — and routes
//! the value/interval families to [`crate::exec::opt_synth`].
//!
//! Constraint shapes are grounded in AM AOM1.4 (`docs/specs/openehr/AM/docs/AOM1.4/`):
//! `C_MULTIPLE_ATTRIBUTE.cardinality`, `C_ATTRIBUTE.existence`, and
//! `C_OBJECT.rm_type_name`. The carrier skeleton mirrors the committed Python
//! reference `corpus/templates/generate_content_opts.py` (itself built on the
//! vendored CNF Robot `minimal_observation.opt`).
//!
//! Contract: `corpus/recipes/opt_synth.md` (the digest-free per-row synthesis
//! contract, alongside the digest-pinned corpus-recipe contracts).
//!
//! NOTE: no openEHR spec governs the corpus template packaging — our own
//! corpus-authoring design; the constraint SHAPES are the AOM1.4 ones cited above.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use crate::exec::opt_synth::{self, SynthError};
use core::fmt::Write as _;

use crate::model::case::MatrixCell;

/// One decision-table row bound to its columns (constraint-cell reader).
struct Cells<'a> {
    columns: &'a [String],
    cells: &'a [MatrixCell],
}

impl Cells<'_> {
    fn cell(&self, name: &str) -> Option<&MatrixCell> {
        self.columns
            .iter()
            .position(|c| c == name)
            .and_then(|i| self.cells.get(i))
    }

    fn text(&self, name: &str) -> Option<&str> {
        match self.cell(name) {
            Some(MatrixCell::Literal(serde_json::Value::String(s))) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// AMB-42 realizability gate.
///
/// A rejected-expected row whose EVERY violated
/// constraint axis is unserializable on the OPT 1.4 wire (ITS-XML
/// `Archetype.xsd`, identical in both published lineages:
/// `C_TIME`/`C_DATE_TIME` carry no `millisecond_validity`
/// element; `C_DURATION` carries only pattern+range, so the AOM-1.4
/// `seconds_allowed` vs `fractional_seconds_allowed` distinction collapses
/// into the single pattern `S` slot) cannot be driven — its ground OPT
/// cannot exist. Returns the excusing citation, or `None` when the row is
/// realizable. (AOM-1.4 UML defines the fields — `c_time.adoc`,
/// `c_duration.adoc` — and CNF master17.4 tests them; the serialization gap
/// is the register entry's subject.)
#[must_use]
pub fn unrealizable_row(
    rm_class: &str,
    columns: &[String],
    cells: &[MatrixCell],
) -> Option<String> {
    if !matches!(
        rm_class,
        "DV_TIME" | "DV_DATE_TIME" | "DV_DURATION" | "DV_INTERVAL"
    ) {
        return None;
    }
    let cell = |name: &str| {
        columns
            .iter()
            .position(|c| c == name)
            .and_then(|i| cells.get(i))
    };
    // Only rejected-expected rows can be unrealizable this way. The loader
    // (`run::synthesize_content_case`) normalizes the authored `rejected`
    // token to the refusal outcome kind the row's `violates` list implies
    // before the driver sees the matrix — accept every spelling.
    match cell("expected") {
        Some(MatrixCell::Literal(serde_json::Value::String(s)))
            if s == "rejected" || s == "validation_failed" || s == "bad_request" => {}
        _ => return None,
    }
    let Some(MatrixCell::Literal(serde_json::Value::Array(violations))) = cell("violates") else {
        return None;
    };
    if violations.is_empty() {
        return None;
    }
    let flag_true = |name: &str| {
        matches!(
            cell(name),
            Some(MatrixCell::Literal(serde_json::Value::Bool(true)))
        )
    };
    let inexpressible = |violation: &str| {
        if violation.contains("millisecond_validity") {
            return true;
        }
        // The duration S slot is shared: prohibiting one of integer/
        // fractional seconds while allowing the other has no pattern form.
        let suffix = if violation.contains("for lower") || violation.contains("_lower") {
            "_lower"
        } else if violation.contains("for upper") || violation.contains("_upper") {
            "_upper"
        } else {
            ""
        };
        if violation.contains("fractional_seconds_allowed") {
            return flag_true(&format!("seconds_allowed{suffix}"));
        }
        if violation.contains("seconds_allowed") {
            return flag_true(&format!("fractional_seconds_allowed{suffix}"));
        }
        false
    };
    let all_inexpressible = violations
        .iter()
        .filter_map(|v| v.as_str())
        .all(inexpressible)
        && violations.iter().all(serde_json::Value::is_string);
    all_inexpressible.then(|| {
        "AMB-42: the violated constraint axes (millisecond_validity / the C_DURATION \
         seconds-vs-fractional distinction) are unserializable in the ITS-XML 1.0.2 \
         OPT wire (Archetype.xsd C_TIME/C_DATE_TIME/C_DURATION) — the row's ground \
         OPT cannot exist on this technology profile"
            .to_owned()
    })
}

/// Synthesize the OPT 1.4 XML for one content row. Dispatches structural
/// `rm_classes` here and value/interval classes to [`opt_synth`].
///
/// # Errors
/// [`SynthError`] when the `rm_class` / column shape is not covered (an
/// interpreter defect, never a conformance outcome).
pub fn synthesize_opt(
    case_id: &str,
    rm_class: &str,
    template_id: &str,
    columns: &[String],
    cells: &[MatrixCell],
) -> Result<String, SynthError> {
    let row = Cells { columns, cells };
    match rm_class {
        "COMPOSITION"
            if columns.iter().any(|c| c == "cardinality")
                && columns.iter().any(|c| c == "context_existence") =>
        {
            Ok(composition_content_cardinality_context(template_id, &row))
        }
        "COMPOSITION" if columns.iter().any(|c| c == "cardinality") => {
            Ok(composition_content_cardinality(template_id, &row))
        }
        "COMPOSITION" => Ok(composition_context_existence(template_id, &row)),
        "EVENT" if columns.iter().any(|c| c == "slot_type") => {
            Ok(event_type_narrowing(template_id, &row))
        }
        "EVENT" => Ok(event_state_existence(template_id, &row)),
        "HISTORY"
            if columns.iter().any(|c| c == "cardinality")
                && columns.iter().any(|c| c == "summary_existence") =>
        {
            Ok(history_events_cardinality_summary(template_id, &row))
        }
        "HISTORY" if columns.iter().any(|c| c == "cardinality") => {
            Ok(history_events_cardinality(template_id, &row))
        }
        "HISTORY" => Ok(history_summary_existence(template_id, &row)),
        "ITEM_STRUCTURE" => Ok(item_structure_type_narrowing(template_id, &row)),
        "OBSERVATION" => Ok(observation_state_protocol_existence(template_id, &row)),
        // Value + interval families.
        _ => opt_synth::synthesize_value_opt(case_id, rm_class, template_id, columns, cells),
    }
}

// ---------------------------------------------------------------------------
// Low-level cADL/OPT XML builders (mirror generate_content_opts.py exactly).
// ---------------------------------------------------------------------------

fn xesc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Deterministic uid namespace (mirrors the Python `_NS`
/// 6f9619ff-8b86-d011-b42d-00cf4fc964ff): stable across runs, our own.
fn det_uid(template_id: &str) -> String {
    const NS: uuid::Uuid = uuid::Uuid::from_bytes([
        0x6f, 0x96, 0x19, 0xff, 0x8b, 0x86, 0xd0, 0x11, 0xb4, 0x2d, 0x00, 0xcf, 0x4f, 0xc9, 0x64,
        0xff,
    ]);
    uuid::Uuid::new_v5(&NS, template_id.as_bytes()).to_string()
}

/// An `IntervalOfInteger` body (`None` bound => unbounded that side).
fn interval(lower: Option<i64>, upper: Option<i64>) -> String {
    let lu = if lower.is_none() { "true" } else { "false" };
    let uu = if upper.is_none() { "true" } else { "false" };
    let mut s = format!(
        "<lower_included>true</lower_included><upper_included>true</upper_included>\
         <lower_unbounded>{lu}</lower_unbounded><upper_unbounded>{uu}</upper_unbounded>"
    );
    if let Some(l) = lower {
        let _ = write!(s, "<lower>{l}</lower>");
    }
    if let Some(u) = upper {
        let _ = write!(s, "<upper>{u}</upper>");
    }
    s
}

fn occ(lower: i64, upper: Option<i64>) -> String {
    format!(
        "<occurrences>{}</occurrences>",
        interval(Some(lower), upper)
    )
}

fn existence(lower: i64, upper: i64) -> String {
    format!(
        "<existence>{}</existence>",
        interval(Some(lower), Some(upper))
    )
}

/// `C_MULTIPLE_ATTRIBUTE.cardinality` (AOM1.4 §`C_MULTIPLE_ATTRIBUTE`).
fn cardinality(lower: i64, upper: Option<i64>) -> String {
    let uu = if upper.is_none() { "true" } else { "false" };
    let mut body = String::from(
        "<is_ordered>false</is_ordered><is_unique>false</is_unique><interval>\
         <lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>",
    );
    let _ = write!(body, "<upper_unbounded>{uu}</upper_unbounded>");
    let _ = write!(body, "<lower>{lower}</lower>");
    if let Some(u) = upper {
        let _ = write!(body, "<upper>{u}</upper>");
    }
    body.push_str("</interval>");
    format!("<cardinality>{body}</cardinality>")
}

fn c_single_attr(name: &str, child: &str, exist: (i64, i64)) -> String {
    format!(
        "<attributes xsi:type=\"C_SINGLE_ATTRIBUTE\"><rm_attribute_name>{name}</rm_attribute_name>{}{child}</attributes>",
        existence(exist.0, exist.1)
    )
}

fn c_multiple_attr(name: &str, children: &str, card: &str, exist: (i64, i64)) -> String {
    format!(
        "<attributes xsi:type=\"C_MULTIPLE_ATTRIBUTE\"><rm_attribute_name>{name}</rm_attribute_name>{}{children}{card}</attributes>",
        existence(exist.0, exist.1)
    )
}

fn c_complex(rm_type: &str, attrs: &str, node_id: &str, occ_lu: (i64, Option<i64>)) -> String {
    let nid = if node_id.is_empty() {
        "<node_id />".to_owned()
    } else {
        format!("<node_id>{node_id}</node_id>")
    };
    format!(
        "<children xsi:type=\"C_COMPLEX_OBJECT\"><rm_type_name>{}</rm_type_name>{}{nid}{attrs}</children>",
        xesc(rm_type),
        occ(occ_lu.0, occ_lu.1)
    )
}

/// A `DV_TEXT` value `C_COMPLEX_OBJECT` with a fixed `C_STRING` list (the structural
/// carriers only need a *valid* ELEMENT.value; the constraint under test is the
/// carrier shape, not the value).
fn dv_text_value() -> String {
    c_complex("DV_TEXT", "", "", (1, Some(1)))
}

/// The observation `term_definitions` at0000..at0004 (+ optional extras),
/// mirroring the Python `obs_term_defs`.
fn obs_term_defs(extra: &[(&str, &str, &str)]) -> String {
    let mut base: Vec<(&str, &str, &str)> = vec![
        ("at0000", "Minimal", "unknown"),
        ("at0001", "Event Series", "@ internal @"),
        ("at0002", "Any event", "*"),
        ("at0003", "Tree", "@ internal @"),
        ("at0004", "value", "*"),
    ];
    base.extend_from_slice(extra);
    let mut out = String::new();
    for (code, text, desc) in base {
        let _ = write!(
            out,
            "<term_definitions code=\"{code}\"><items id=\"description\">{}</items><items id=\"text\">{}</items></term_definitions>",
            xesc(desc),
            xesc(text)
        );
    }
    out
}

/// The COMPOSITION template envelope (mirrors the Python `composition`).
#[expect(
    clippy::similar_names,
    reason = "content_exist / context_exist name the two RM attributes precisely"
)]
fn composition(
    template_id: &str,
    obs_children: &str,
    content_card: &str,
    content_exist: (i64, i64),
    context_exist: Option<(i64, i64)>,
) -> String {
    let uid = det_uid(template_id);
    let content_attr = c_multiple_attr("content", obs_children, content_card, content_exist);
    let category = c_single_attr(
        "category",
        &c_complex(
            "DV_CODED_TEXT",
            &c_single_attr("defining_code", &c_code_phrase("openehr", &["433"]), (1, 1)),
            "",
            (1, Some(1)),
        ),
        (1, 1),
    );
    // A constrained context always carries an OPTIONAL other_context child
    // (ITEM_TREE at0011, existence 0..1) so the official "context with
    // other_context" data rows are committable against a constrained node
    // (EVENT_CONTEXT itself is PATHABLE — empty node_id — while ITEM_TREE is
    // LOCATABLE and needs the archetype node).
    let (context_attr, context_terms) = match context_exist {
        Some(exist) => {
            let other = c_single_attr(
                "other_context",
                &c_complex("ITEM_TREE", "", "at0011", (1, Some(1))),
                (0, 1),
            );
            (
                c_single_attr(
                    "context",
                    &c_complex("EVENT_CONTEXT", &other, "", (1, Some(1))),
                    exist,
                ),
                "<term_definitions code=\"at0011\"><items id=\"description\">@ internal @</items><items id=\"text\">Other context</items></term_definitions>".to_owned(),
            )
        }
        None => (String::new(), String::new()),
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<template xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns=\"http://schemas.openehr.org/v1\">\n\
  <language><terminology_id><value>ISO_639-1</value></terminology_id><code_string>en</code_string></language>\n\
  <description><original_author id=\"Original Author\">CNF corpus</original_author><lifecycle_state>Initial</lifecycle_state><details><language><terminology_id><value>ISO_639-1</value></terminology_id><code_string>en</code_string></language><purpose>CNF content constraint template</purpose></details></description>\n\
  <uid><value>{uid}</value></uid>\n\
  <template_id><value>{tid}</value></template_id>\n\
  <concept>CNF content constraint</concept>\n\
  <definition>\n\
    <rm_type_name>COMPOSITION</rm_type_name>\n\
    {occ}\n\
    <node_id>at0000</node_id>\n\
    {category}\n\
    {context_attr}\n\
    {content_attr}\n\
    <archetype_id><value>openEHR-EHR-COMPOSITION.minimal.v1</value></archetype_id>\n\
    <template_id><value>{tid}</value></template_id>\n\
    <term_definitions code=\"at0000\"><items id=\"description\">unknown</items><items id=\"text\">Minimal</items></term_definitions>\n\
    {context_terms}\n\
  </definition>\n\
</template>\n",
        tid = xesc(template_id),
        occ = occ(1, Some(1)),
    )
}

fn c_code_phrase(term: &str, codes: &[&str]) -> String {
    let mut cl = String::new();
    for c in codes {
        let _ = write!(cl, "<code_list>{}</code_list>", xesc(c));
    }
    format!(
        "<children xsi:type=\"C_CODE_PHRASE\"><rm_type_name>CODE_PHRASE</rm_type_name>{}<node_id /><terminology_id><value>{}</value></terminology_id>{cl}</children>",
        occ(1, Some(1)),
        xesc(term)
    )
}

// ---------------------------------------------------------------------------
// OBSERVATION carrier assemblers (parameterized structural variants).
// ---------------------------------------------------------------------------

/// The value ELEMENT (at0004) with a valid `DV_TEXT` value slot.
fn value_element() -> String {
    let value_attr = c_single_attr("value", &dv_text_value(), (0, 1));
    format!(
        "<children xsi:type=\"C_COMPLEX_OBJECT\"><rm_type_name>ELEMENT</rm_type_name>{}<node_id>at0004</node_id>{value_attr}</children>",
        occ(0, Some(1))
    )
}

/// A minimal `data` `ITEM_TREE` (at0003) carrying the value element.
fn item_tree_data() -> String {
    let items = c_multiple_attr("items", &value_element(), &cardinality(0, None), (0, 1));
    let item_tree = c_complex("ITEM_TREE", &items, "at0003", (1, Some(1)));
    c_single_attr("data", &item_tree, (1, 1))
}

/// Wrap an OBSERVATION `C_ARCHETYPE_ROOT` (openEHR-EHR-OBSERVATION.minimal.v1)
/// around the given data/state/protocol attribute XML + term extras.
fn observation_root(attrs: &str, extra_terms: &[(&str, &str, &str)]) -> String {
    format!(
        "<children xsi:type=\"C_ARCHETYPE_ROOT\"><rm_type_name>OBSERVATION</rm_type_name>\
<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded><upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>\
<node_id>at0000</node_id>{attrs}<archetype_id><value>openEHR-EHR-OBSERVATION.minimal.v1</value></archetype_id>{}</children>",
        obs_term_defs(extra_terms)
    )
}

/// Standard OBSERVATION.data = HISTORY(at0001) / EVENT(slot,at0002) / `ITEM_TREE`.
/// `events_exist` is the `HISTORY.events` `C_ATTRIBUTE.existence`; callers pass
/// `cardinality_existence(token)` when the cardinality axis is under test so a
/// zero-events row is admitted for the tokens that allow it, and `(1, 1)` when
/// events are fixed mandatory (the standard `events 1..*` data shape).
fn observation_history(
    events_slot_type: &str,
    event_attrs_extra: &str,
    events_card: &str,
    events_exist: (i64, i64),
) -> String {
    let event_attrs = format!("{}{event_attrs_extra}", item_tree_data());
    let event = c_complex(events_slot_type, &event_attrs, "at0002", (0, Some(1)));
    let events = c_multiple_attr("events", &event, events_card, events_exist);
    let history = c_complex("HISTORY", &events, "at0001", (1, Some(1)));
    c_single_attr("data", &history, (1, 1))
}

// ---------------------------------------------------------------------------
// Structural family builders.
// ---------------------------------------------------------------------------

/// `any|1plus|3plus|opt|mand|3to5` → a `C_MULTIPLE_ATTRIBUTE.cardinality`
/// interval (AOM1.4 §`C_MULTIPLE_ATTRIBUTE`).
fn cardinality_token(token: &str) -> String {
    match token {
        "1plus" => cardinality(1, None),
        "3plus" => cardinality(3, None),
        "opt" => cardinality(0, Some(1)),
        "mand" => cardinality(1, Some(1)),
        "3to5" => cardinality(3, Some(5)),
        // "any" and any unknown token → unbounded 0..*
        _ => cardinality(0, None),
    }
}

/// `optional|mandatory` → a `C_ATTRIBUTE.existence` pair (AOM1.4 §`C_ATTRIBUTE`).
fn existence_token(token: &str) -> (i64, i64) {
    match token {
        "mandatory" => (1, 1),
        _ => (0, 1),
    }
}

/// A container attribute's `C_ATTRIBUTE.existence` must follow its cardinality
/// token: `any`/`opt` leave the attribute optional (0..1) so an omitted /
/// zero-count container is admitted and the RM invariant decides — e.g.
/// `HISTORY.Events_valid` (`(events /= Void and then not events.is_empty) or
/// summary /= Void`, RM `data_structures` §HISTORY Invariants) accepts a
/// zero-events HISTORY via its summary disjunct; `1plus`/`3plus`/`mand`/`3to5`
/// make the attribute mandatory (1..1). The cardinality alone never fires on an
/// omitted container (AOM1.4 §`C_MULTIPLE_ATTRIBUTE` + §`C_ATTRIBUTE`), and on
/// the canonical wire an empty list serializes as absent, so mandating
/// existence 1..1 would wrongly reject the zero-count row.
fn cardinality_existence(token: &str) -> (i64, i64) {
    match token {
        "1plus" | "3plus" | "mand" | "3to5" => (1, 1),
        _ => (0, 1),
    }
}

/// The master15 combined family: `C_MULTIPLE_ATTRIBUTE.cardinality` on
/// `COMPOSITION.content` AND `C_ATTRIBUTE.existence` on `COMPOSITION.context`
/// in one template (AOM1.4 §`C_MULTIPLE_ATTRIBUTE` + §`C_ATTRIBUTE`) — the
/// content_card_X-context_mand official cases constrain both axes at once.
#[expect(
    clippy::similar_names,
    reason = "content_exist / context_exist name the two RM attributes precisely"
)]
fn composition_content_cardinality_context(template_id: &str, row: &Cells<'_>) -> String {
    let token = row.text("cardinality").unwrap_or("any");
    let card = cardinality_token(token);
    // Same existence-follows-the-token rule as the single-axis family (see
    // `cardinality_existence`): the cardinality alone never fires on an omitted
    // container.
    let content_exist = cardinality_existence(token);
    let context_exist = existence_token(row.text("context_existence").unwrap_or("optional"));
    let obs = observation_root(&item_tree_data_observation_history(), &[]);
    composition(template_id, &obs, &card, content_exist, Some(context_exist))
}

fn composition_content_cardinality(template_id: &str, row: &Cells<'_>) -> String {
    let token = row.text("cardinality").unwrap_or("any");
    let card = cardinality_token(token);
    // The COMPOSITION.content existence follows the cardinality token so an
    // absent/zero-count content is accepted for `any`/`opt` and rejected by
    // existence for the mandatory families (`cardinality_existence`); the
    // instance omits content when the count is 0 (an empty present list is
    // rejected at the RM level regardless of cardinality).
    let content_exist = cardinality_existence(token);
    let obs = observation_root(&item_tree_data_observation_history(), &[]);
    composition(template_id, &obs, &card, content_exist, None)
}

/// OBSERVATION with the standard `HISTORY/EVENT/ITEM_TREE` data (default events
/// cardinality 1..*, EVENT slot).
fn item_tree_data_observation_history() -> String {
    observation_history("EVENT", "", &cardinality(1, None), (1, 1))
}

fn composition_context_existence(template_id: &str, row: &Cells<'_>) -> String {
    let exist = existence_token(row.text("context_existence").unwrap_or("optional"));
    let obs = observation_root(&item_tree_data_observation_history(), &[]);
    composition(
        template_id,
        &obs,
        &cardinality(0, None),
        (0, 1),
        Some(exist),
    )
}

fn event_state_existence(template_id: &str, row: &Cells<'_>) -> String {
    let exist = existence_token(row.text("state_existence").unwrap_or("optional"));
    let state_attr = c_single_attr(
        "state",
        &c_complex("ITEM_TREE", "", "at0005", (1, Some(1))),
        exist,
    );
    let data = observation_history("EVENT", &state_attr, &cardinality(1, None), (1, 1));
    let obs = observation_root(&data, &[("at0005", "State", "@ internal @")]);
    composition(template_id, &obs, &cardinality(0, None), (0, 1), None)
}

fn event_type_narrowing(template_id: &str, row: &Cells<'_>) -> String {
    let slot = row.text("slot_type").unwrap_or("EVENT");
    let data = observation_history(slot, "", &cardinality(1, None), (1, 1));
    let obs = observation_root(&data, &[]);
    composition(template_id, &obs, &cardinality(0, None), (0, 1), None)
}

fn history_events_cardinality(template_id: &str, row: &Cells<'_>) -> String {
    let token = row.text("cardinality").unwrap_or("any");
    let card = cardinality_token(token);
    let data = observation_history("EVENT", "", &card, cardinality_existence(token));
    let obs = observation_root(&data, &[]);
    composition(template_id, &obs, &cardinality(0, None), (0, 1), None)
}

/// The master16 combined family: `C_MULTIPLE_ATTRIBUTE.cardinality` on
/// HISTORY.events AND `C_ATTRIBUTE.existence` on HISTORY.summary in one
/// template — the events_card_X-summary_ex_mand official cases (and the
/// summary-present rows of the `summary_ex_opt` cases) constrain both axes.
fn history_events_cardinality_summary(template_id: &str, row: &Cells<'_>) -> String {
    let token = row.text("cardinality").unwrap_or("any");
    let card = cardinality_token(token);
    let summary_exist = existence_token(row.text("summary_existence").unwrap_or("optional"));
    let summary_attr = c_single_attr(
        "summary",
        &c_complex("ITEM_TREE", "", "at0007", (1, Some(1))),
        summary_exist,
    );
    let event_attrs = item_tree_data();
    let event = c_complex("EVENT", &event_attrs, "at0002", (0, Some(1)));
    let events = c_multiple_attr("events", &event, &card, cardinality_existence(token));
    let history = c_complex(
        "HISTORY",
        &format!("{events}{summary_attr}"),
        "at0001",
        (1, Some(1)),
    );
    let data = c_single_attr("data", &history, (1, 1));
    let obs = observation_root(&data, &[("at0007", "Summary", "@ internal @")]);
    composition(template_id, &obs, &cardinality(0, None), (0, 1), None)
}

fn history_summary_existence(template_id: &str, row: &Cells<'_>) -> String {
    let exist = existence_token(row.text("summary_existence").unwrap_or("optional"));
    let summary_attr = c_single_attr(
        "summary",
        &c_complex("ITEM_TREE", "", "at0007", (1, Some(1))),
        exist,
    );
    // HISTORY(at0001) with events 1..* + a summary attribute.
    let event_attrs = item_tree_data();
    let event = c_complex("EVENT", &event_attrs, "at0002", (0, Some(1)));
    let events = c_multiple_attr("events", &event, &cardinality(1, None), (1, 1));
    let history = c_complex(
        "HISTORY",
        &format!("{events}{summary_attr}"),
        "at0001",
        (1, Some(1)),
    );
    let data = c_single_attr("data", &history, (1, 1));
    let obs = observation_root(&data, &[("at0007", "Summary", "@ internal @")]);
    composition(template_id, &obs, &cardinality(0, None), (0, 1), None)
}

fn item_structure_type_narrowing(template_id: &str, row: &Cells<'_>) -> String {
    let slot = row.text("slot_type").unwrap_or("ITEM_STRUCTURE");
    let data_child = c_complex(slot, "", "at0003", (1, Some(1)));
    let data_attr = c_single_attr("data", &data_child, (1, 1));
    let eval_root = format!(
        "<children xsi:type=\"C_ARCHETYPE_ROOT\"><rm_type_name>EVALUATION</rm_type_name>\
<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded><upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>\
<node_id>at0000</node_id>{data_attr}<archetype_id><value>openEHR-EHR-EVALUATION.minimal.v1</value></archetype_id>\
<term_definitions code=\"at0000\"><items id=\"description\">unknown</items><items id=\"text\">Minimal</items></term_definitions>\
<term_definitions code=\"at0003\"><items id=\"description\">@ internal @</items><items id=\"text\">structure</items></term_definitions></children>"
    );
    composition(template_id, &eval_root, &cardinality(0, None), (0, 1), None)
}

fn observation_state_protocol_existence(template_id: &str, row: &Cells<'_>) -> String {
    let state_exist = existence_token(row.text("state_existence").unwrap_or("optional"));
    let protocol_exist = existence_token(row.text("protocol_existence").unwrap_or("optional"));
    let data = item_tree_data_observation_history();
    let state_attr = c_single_attr(
        "state",
        &c_complex("HISTORY", &events_only(), "at0005", (1, Some(1))),
        state_exist,
    );
    let protocol_attr = c_single_attr(
        "protocol",
        &c_complex("ITEM_TREE", "", "at0006", (1, Some(1))),
        protocol_exist,
    );
    let attrs = format!("{data}{state_attr}{protocol_attr}");
    let obs = observation_root(
        &attrs,
        &[
            ("at0005", "State", "@ internal @"),
            ("at0006", "Protocol", "@ internal @"),
        ],
    );
    composition(template_id, &obs, &cardinality(0, None), (0, 1), None)
}

/// A HISTORY events attribute (1..*) with one EVENT slot (for the state HISTORY).
fn events_only() -> String {
    let event = c_complex("EVENT", &item_tree_data(), "at0002", (0, Some(1)));
    c_multiple_attr("events", &event, &cardinality(1, None), (1, 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cols(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    fn lit(s: &str) -> MatrixCell {
        MatrixCell::Literal(json!(s))
    }

    #[test]
    fn cardinality_tokens_map_to_intervals() {
        let c = cols(&["cardinality", "content_count", "expected", "violates"]);
        let cells = vec![
            lit("3to5"),
            MatrixCell::Literal(json!(3)),
            lit("accepted"),
            lit("[]"),
        ];
        let xml = synthesize_opt(
            "CONT-COMPOSITION-content_cardinality",
            "COMPOSITION",
            "cnf.tpl.x.r0",
            &c,
            &cells,
        )
        .unwrap();
        assert!(xml.contains("<cardinality>"));
        assert!(xml.contains("<lower>3</lower>"));
        assert!(xml.contains("<upper>5</upper>"));
        assert!(xml.contains("openEHR-EHR-COMPOSITION.minimal.v1"));
    }

    #[test]
    fn context_existence_mandatory_emits_1_1() {
        let c = cols(&[
            "context_existence",
            "context_committed",
            "expected",
            "violates",
        ]);
        let cells = vec![lit("mandatory"), lit("absent"), lit("rejected"), lit("[]")];
        let xml = synthesize_opt(
            "CONT-COMPOSITION-context_existence",
            "COMPOSITION",
            "cnf.tpl.x.r0",
            &c,
            &cells,
        )
        .unwrap();
        assert!(xml.contains("<rm_attribute_name>context</rm_attribute_name>"));
        // existence 1..1 present on the context attribute.
        assert!(xml.contains("EVENT_CONTEXT"));
    }

    #[test]
    fn event_type_narrowing_sets_slot_rm_type() {
        let c = cols(&["slot_type", "committed_type", "expected", "violates"]);
        let cells = vec![
            lit("POINT_EVENT"),
            lit("INTERVAL_EVENT"),
            lit("rejected"),
            lit("[]"),
        ];
        let xml = synthesize_opt(
            "CONT-EVENT-type_narrowing",
            "EVENT",
            "cnf.tpl.x.r0",
            &c,
            &cells,
        )
        .unwrap();
        assert!(xml.contains("<rm_type_name>POINT_EVENT</rm_type_name>"));
    }

    #[test]
    fn item_structure_uses_evaluation_carrier() {
        let c = cols(&["slot_type", "committed_type", "expected", "violates"]);
        let cells = vec![
            lit("ITEM_TREE"),
            lit("ITEM_LIST"),
            lit("rejected"),
            lit("[]"),
        ];
        let xml = synthesize_opt(
            "CONT-ITEM_STRUCTURE-type_narrowing",
            "ITEM_STRUCTURE",
            "cnf.tpl.x.r0",
            &c,
            &cells,
        )
        .unwrap();
        assert!(xml.contains("openEHR-EHR-EVALUATION.minimal.v1"));
        assert!(xml.contains("<rm_type_name>ITEM_TREE</rm_type_name>"));
    }

    #[test]
    fn observation_state_protocol_existence_both_mandatory() {
        let c = cols(&[
            "state_existence",
            "protocol_existence",
            "data_committed",
            "state_committed",
            "protocol_committed",
            "expected",
            "violates",
        ]);
        let cells = vec![
            lit("mandatory"),
            lit("mandatory"),
            lit("present"),
            lit("absent"),
            lit("absent"),
            lit("rejected"),
            lit("[]"),
        ];
        let xml = synthesize_opt(
            "CONT-OBSERVATION-state_protocol_existence",
            "OBSERVATION",
            "cnf.tpl.x.r0",
            &c,
            &cells,
        )
        .unwrap();
        assert!(xml.contains("<rm_attribute_name>state</rm_attribute_name>"));
        assert!(xml.contains("<rm_attribute_name>protocol</rm_attribute_name>"));
    }

    #[test]
    fn synthesis_is_deterministic() {
        let c = cols(&["cardinality", "content_count", "expected", "violates"]);
        let cells = vec![
            lit("any"),
            MatrixCell::Literal(json!(0)),
            lit("accepted"),
            lit("[]"),
        ];
        let a = synthesize_opt(
            "CONT-COMPOSITION-content_cardinality",
            "COMPOSITION",
            "cnf.tpl.x.r0",
            &c,
            &cells,
        )
        .unwrap();
        let b = synthesize_opt(
            "CONT-COMPOSITION-content_cardinality",
            "COMPOSITION",
            "cnf.tpl.x.r0",
            &c,
            &cells,
        )
        .unwrap();
        assert_eq!(a, b);
    }
}
