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
//! - **archetype constraints** (skipped): HISTORY `events` cardinality + `summary`
//!   existence, EVENT type narrowing (`POINT_EVENT/INTERVAL_EVENT`), and
//!   `ITEM_STRUCTURE` type narrowing (`ITEM_TREE/LIST/TABLE/SINGLE`) — all need a
//!   constraint-expressing OPT the corpus does not contain (design §2.2a);
//!   transcribed + cited, returning `Skipped`.

use crate::case::Chapter;
use crate::harness::{CaseFuture, CaseRun, RunContext};
use crate::registry::CaseEntry;

use super::drive::{self, meta};

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

    // ── ITEM_STRUCTURE (5) — type narrowing archetype ────────────────────────
    for id in [
        "CONT-ITEM_STR-type_any",
        "CONT-ITEM_STR-type_item_tree",
        "CONT-ITEM_STR-type_item_list",
        "CONT-ITEM_STR-type_item_table",
        "CONT-ITEM_STR-type_item_single",
    ] {
        all.push(entry(id, run_skip_item_str));
    }

    all
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
