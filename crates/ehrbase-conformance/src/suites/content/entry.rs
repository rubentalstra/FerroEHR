//! master16 — ENTRY data-validation truth tables (`master16-content_tc_entry.adoc`):
//! OBSERVATION, HISTORY, EVENT, and `ITEM_STRUCTURE` constraint sets.
//!
//! Two kinds of case:
//!
//! - **RM/schema-determinable** (driven): the OBSERVATION and EVENT tables each
//!   carry rows marked "(RM/schema constraint)" — `data` is mandatory `[1]` on
//!   ENTRY, so an instance without `data` must be rejected by any conformant
//!   server, and one with `data` accepted, independent of the archetype. These
//!   run via [`drive::entry_data_existence`] against the persistent base (which
//!   carries an OBSERVATION → HISTORY → `POINT_EVENT`). The `state`/`protocol`
//!   existence rows in the same tables are `0..1` in the RM and only an archetype
//!   narrows them — those rows are archetype constraints (not asserted here).
//! - **`ITEM_STRUCTURE` type narrowing** (driven via `clinical_content_validation`):
//!   that OPT narrows four EVALUATION `data` slots to `ITEM_SINGLE/TREE/LIST/TABLE`
//!   and ships a committable composition. Each case commits the valid composition
//!   (accepted) then a copy whose slot `_type` is swapped to a sibling subtype the
//!   slot forbids (rejected, "Class not allowed"). Our validator does **not** yet
//!   enforce this narrowing (an **open finding** — the SUT accepts the wrong
//!   subtype); driven and failing, never masked as a skip (design §4.5).
//! - **archetype constraints still skipped**: HISTORY `events` cardinality +
//!   `summary` existence and EVENT type narrowing (`POINT_EVENT/INTERVAL_EVENT`) —
//!   no vendored OPT narrows HISTORY.events cardinality beyond `0..*` nor an EVENT
//!   slot to a subtype (searched `all_types`, `minimal_entry_combination/*`,
//!   `clinical_content_validation`, `composition_evaluation_test`,
//!   `cardinality_of_section`); `CONT-ITEM_STR-type_any` likewise (no OPT leaves an
//!   `ITEM_STRUCTURE` slot open). Transcribed + cited, returning `Skipped`.

use serde_json::{Value, json};

use crate::case::Chapter;
use crate::harness::{CaseFuture, CaseRun, RunContext};
use crate::registry::CaseEntry;

use super::drive::{self, Constraint, Expected, meta};
use super::mutate;

/// The implemented master16 case entries (26 CONT cases).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut all = Vec::new();

    // ── OBSERVATION (4) — driven: OBSERVATION.data existence (RM/schema) ──────
    for id in [
        "CONT-OBS-state_ex_opt-protocol_ex_opt",
        "CONT-OBS-state_ex_opt-protocol_ex_mand",
        "CONT-OBS-state_ex_mand-protocol_ex_opt",
        "CONT-OBS-state_ex_mand-protocol_ex_mand",
    ] {
        all.push(entry(id, run_obs_data));
    }

    // ── HISTORY (12) — archetype: events cardinality + summary existence ──────
    for id in [
        "CONT-HIST-events_card_any-summary_ex_opt",
        "CONT-HIST-events_card_1plus-summary_ex_opt",
        "CONT-HIST-events_card_3plus-summary_ex_opt",
        "CONT-HIST-events_card_opt-summary_ex_opt",
        "CONT-HIST-events_card_mand-summary_ex_opt",
        "CONT-HIST-events_card_3to5-summary_ex_opt",
        "CONT-HIST-events_card_any-summary_ex_mand",
        "CONT-HIST-events_card_1plus-summary_ex_mand",
        "CONT-HIST-events_card_3plus-summary_ex_mand",
        "CONT-HIST-events_card_opt-summary_ex_mand",
        "CONT-HIST-events_card_mand-summary_ex_mand",
        "CONT-HIST-events_card_3to5-summary_ex_mand",
    ] {
        all.push(entry(id, run_skip_hist));
    }

    // ── EVENT (5) — data existence driven; type narrowing archetype ──────────
    all.push(entry("CONT-EVENT-state_ex_opt", run_event_data));
    all.push(entry("CONT-EVENT-state_ex_mand", run_event_data));
    all.push(entry("CONT-EVENT-type_any", run_skip_event_type));
    all.push(entry("CONT-EVENT-type_point_event", run_skip_event_type));
    all.push(entry("CONT-EVENT-type_interval_event", run_skip_event_type));

    // ── ITEM_STRUCTURE (5) — type narrowing, driven via clinical_content_validation ─
    // The `clinical_content_validation` OPT narrows four EVALUATION `data` slots to
    // a specific ITEM_STRUCTURE subtype; its vendored composition fills each with
    // the matching type (accepted). Swapping a slot's `_type` to a sibling subtype
    // is the truth-table "Class not allowed" rejection (master16 §ITEM_STRUCTURE).
    // `type_any` stays skipped: no vendored OPT leaves an ITEM_STRUCTURE slot open,
    // so the "any subtype accepted" positive cannot be isolated.
    all.push(entry("CONT-ITEM_STR-type_any", run_skip_item_str));
    all.push(entry("CONT-ITEM_STR-type_item_tree", run_item_str_tree));
    all.push(entry("CONT-ITEM_STR-type_item_list", run_item_str_list));
    all.push(entry("CONT-ITEM_STR-type_item_table", run_item_str_table));
    all.push(entry("CONT-ITEM_STR-type_item_single", run_item_str_single));

    all
}

/// `clinical_content_validation.opt` + its vendored composition. Content slots:
/// `content[1].data`=`ITEM_SINGLE`, `[2]`=`ITEM_TREE`, `[3]`=`ITEM_LIST`,
/// `[4]`=`ITEM_TABLE` — each narrowed by the OPT to that exact subtype.
const CLINICAL: Constraint = Constraint {
    opt: "validation/clinical_content_validation.opt",
    comp: "compositions/CANONICAL_JSON/clinical_content_validation__full.json",
};

/// Drive an `ITEM_STRUCTURE` type-narrowing case: commit the vendored composition
/// (accepted), then a copy whose `pointer` node's `_type` is swapped to
/// `wrong_type`, a sibling subtype the OPT slot forbids (rejected, "Class not
/// allowed").
fn drive_item_str<'a>(
    ctx: &'a RunContext<'a>,
    narrowed: &'static str,
    pointer: &'static str,
    wrong_type: &'static str,
) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::drive_constraint(
            ctx,
            &CLINICAL,
            "ITEM_STRUCTURE slot filled with the narrowed subtype (accepted)",
            vec![(
                format!(
                    "ITEM_STRUCTURE {narrowed} slot filled with {wrong_type} (Class not allowed)"
                ),
                Box::new(move |c: &mut Value| {
                    mutate::set_pointer(c, &format!("{pointer}/_type"), json!(wrong_type));
                }),
                Expected::Rejected,
            )],
        )
        .await
    })
}

fn run_item_str_tree<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_TREE", "/content/2/data", "ITEM_LIST")
}

fn run_item_str_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_LIST", "/content/3/data", "ITEM_TREE")
}

fn run_item_str_table<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_TABLE", "/content/4/data", "ITEM_TREE")
}

fn run_item_str_single<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_SINGLE", "/content/1/data", "ITEM_TREE")
}

fn entry(id: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: meta(id, Chapter::Master16, id),
        run,
    }
}

/// OBSERVATION.data existence (RM/schema constraint): data present → accepted,
/// data absent → rejected.
fn run_obs_data<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move { drive::entry_data_existence(ctx, "OBSERVATION").await })
}

/// EVENT.data existence (RM/schema constraint), driven on the base `POINT_EVENT`.
fn run_event_data<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move { drive::entry_data_existence(ctx, "POINT_EVENT").await })
}

fn run_skip_hist<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::skip_archetype("HISTORY.events cardinality / HISTORY.summary existence")
    })
}

fn run_skip_event_type<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(
        async move { drive::skip_archetype("EVENT type narrowing (POINT_EVENT / INTERVAL_EVENT)") },
    )
}

fn run_skip_item_str<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::skip_archetype("ITEM_STRUCTURE type narrowing (ITEM_TREE/LIST/TABLE/SINGLE)")
    })
}
