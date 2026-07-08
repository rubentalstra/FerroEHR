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
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, RunContext};
use crate::registry::CaseEntry;

use super::author::{self, Card};
use super::drive::{self, Constraint, Expected, meta};
use super::mutate;

/// The persistent base composition (a persistent COMPOSITION carrying
/// OBSERVATION → HISTORY → `POINT_EVENT`) — corpus-root-relative.
const PERSIST_COMP: &str = "compositions/CANONICAL_JSON/persistent_minimal.en.v1__full.json";
/// The persistent base OPT (relative to `valid_templates/`).
const PERSIST_OPT: &str = "minimal_persistent/persistent_minimal.opt";

/// Whether an events count `n` satisfies a cardinality interval (master16 HISTORY).
fn events_ok(card: Card, count: usize) -> bool {
    match card {
        Card::Any => true,
        Card::OnePlus => count >= 1,
        Card::ThreePlus => count >= 3,
        Card::Opt => count <= 1,
        Card::Mand => count == 1,
        Card::ThreeToFive => (3..=5).contains(&count),
    }
}

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

    // ── HISTORY (12) — driven against authored HISTORY.events cardinality +
    //    HISTORY.summary existence OPTs (the summary-absent half of each truth
    //    table: events cardinality {0..*,1..*,3..*,0..1,1..1,3..5} × summary
    //    existence {0..1,1..1}). The base persistent composition carries no
    //    HISTORY.summary, so the summary-absent rows exercise both the events
    //    cardinality (via the committed events count) and the summary existence
    //    (a mandatory summary is violated by its absence) constraints without
    //    fabricating extra RM data. ──
    const HIST: &[(&str, crate::harness::CaseRun)] = &[
        ("CONT-HIST-events_card_any-summary_ex_opt", h_any_opt),
        ("CONT-HIST-events_card_1plus-summary_ex_opt", h_1plus_opt),
        ("CONT-HIST-events_card_3plus-summary_ex_opt", h_3plus_opt),
        ("CONT-HIST-events_card_opt-summary_ex_opt", h_opt_opt),
        ("CONT-HIST-events_card_mand-summary_ex_opt", h_mand_opt),
        ("CONT-HIST-events_card_3to5-summary_ex_opt", h_3to5_opt),
        ("CONT-HIST-events_card_any-summary_ex_mand", h_any_mand),
        ("CONT-HIST-events_card_1plus-summary_ex_mand", h_1plus_mand),
        ("CONT-HIST-events_card_3plus-summary_ex_mand", h_3plus_mand),
        ("CONT-HIST-events_card_opt-summary_ex_mand", h_opt_mand),
        ("CONT-HIST-events_card_mand-summary_ex_mand", h_mand_mand),
        ("CONT-HIST-events_card_3to5-summary_ex_mand", h_3to5_mand),
    ];
    for &(id, run) in HIST {
        all.push(entry(id, run));
    }

    // ── EVENT (5) — data existence driven; type narrowing driven via authored
    //    OPTs. `type_any`: the persistent base narrows HISTORY.events only to the
    //    abstract EVENT, so committing the base POINT_EVENT is the "any subtype
    //    accepted" positive. `type_point_event`: author a POINT_EVENT narrowing —
    //    base accepted, a sibling (_type→INTERVAL_EVENT) rejected. `type_interval_
    //    event` needs a valid INTERVAL_EVENT the corpus does not carry (mandatory
    //    `width`/`math_function`) → honest skip. ──
    all.push(entry("CONT-EVENT-state_ex_opt", run_event_data));
    all.push(entry("CONT-EVENT-state_ex_mand", run_event_data));
    all.push(entry("CONT-EVENT-type_any", run_event_any));
    all.push(entry("CONT-EVENT-type_point_event", run_event_point));
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

/// Drive one CONT-HIST case: author a `persistent_minimal` OPT tightening
/// `HISTORY.events` cardinality (+ a mandatory `HISTORY.summary` for the
/// `summary_ex_mand` cases), then commit {0,1,3}-event persistent compositions
/// (summary absent) and assert per the master16 HISTORY truth tables.
async fn drive_hist_case(
    ctx: &RunContext<'_>,
    tid: &'static str,
    card: Card,
    summary_mand: bool,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(PERSIST_OPT)?;
    author::set_template_id(&mut opt, tid);
    if !author::constrain_nested_multiple(&mut opt, "HISTORY", "events", card.interval()) {
        return Err(CaseError::Assertion(
            "base OPT has no HISTORY.events multiple attribute to constrain".to_owned(),
        ));
    }
    if summary_mand {
        author::constrain_nested_single_mandatory(&mut opt, "HISTORY", "summary");
    }
    let xml = author::to_xml(&opt)?;
    let base = fixtures::read_json(PERSIST_COMP).map_err(|e| CaseError::Codec(e.to_string()))?;

    let mut rows: Vec<(String, Value, Expected)> = Vec::new();
    for &count in &[0usize, 1, 3] {
        let mut c = base.clone();
        mutate::retarget_template(&mut c, tid);
        if let Some(h) = mutate::first_node_mut(&mut c, "HISTORY") {
            mutate::set_array_count(h, "events", count);
            mutate::remove_field(h, "summary");
        }
        // RM `HISTORY.Events_valid` (history_impl): at least one event OR a summary
        // must be present. Every row here has summary absent, so 0 events is
        // rejected by the RM invariant regardless of the archetype cardinality —
        // this is spec-authoritative over the master16 table's "no events, absent
        // summary → accepted" row (ADR-008: the RM invariant governs).
        let accepted = count >= 1 && events_ok(card, count) && !summary_mand;
        let label = format!(
            "{count} event(s), summary absent → {}",
            if accepted { "accepted" } else { "rejected" }
        );
        rows.push((
            label,
            c,
            if accepted {
                Expected::Accepted
            } else {
                Expected::Rejected
            },
        ));
    }
    drive::drive_authored(ctx, &xml, rows).await
}

/// Generate a per-case HISTORY run function (the `CaseRun` fn-pointer carries no
/// case id, so each case's cardinality/summary is bound in a distinct function).
macro_rules! hist {
    ($fn:ident, $tid:literal, $card:expr, $summary_mand:expr) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move { drive_hist_case(ctx, $tid, $card, $summary_mand).await })
        }
    };
}

hist!(
    h_any_opt,
    "cnf_cont_hist_events_any_summary_opt",
    Card::Any,
    false
);
hist!(
    h_1plus_opt,
    "cnf_cont_hist_events_1plus_summary_opt",
    Card::OnePlus,
    false
);
hist!(
    h_3plus_opt,
    "cnf_cont_hist_events_3plus_summary_opt",
    Card::ThreePlus,
    false
);
hist!(
    h_opt_opt,
    "cnf_cont_hist_events_opt_summary_opt",
    Card::Opt,
    false
);
hist!(
    h_mand_opt,
    "cnf_cont_hist_events_mand_summary_opt",
    Card::Mand,
    false
);
hist!(
    h_3to5_opt,
    "cnf_cont_hist_events_3to5_summary_opt",
    Card::ThreeToFive,
    false
);
hist!(
    h_any_mand,
    "cnf_cont_hist_events_any_summary_mand",
    Card::Any,
    true
);
hist!(
    h_1plus_mand,
    "cnf_cont_hist_events_1plus_summary_mand",
    Card::OnePlus,
    true
);
hist!(
    h_3plus_mand,
    "cnf_cont_hist_events_3plus_summary_mand",
    Card::ThreePlus,
    true
);
hist!(
    h_opt_mand,
    "cnf_cont_hist_events_opt_summary_mand",
    Card::Opt,
    true
);
hist!(
    h_mand_mand,
    "cnf_cont_hist_events_mand_summary_mand",
    Card::Mand,
    true
);
hist!(
    h_3to5_mand,
    "cnf_cont_hist_events_3to5_summary_mand",
    Card::ThreeToFive,
    true
);

/// EVENT `type_any`: the persistent base's `HISTORY.events` slot is constrained
/// only to the abstract `EVENT`, so committing the base `POINT_EVENT` exercises
/// "any concrete subtype accepted" (master16 §EVENT). Provisioned via the base OPT
/// (no authoring needed — the slot is already open).
fn run_event_any<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let base =
            fixtures::read_json(PERSIST_COMP).map_err(|e| CaseError::Codec(e.to_string()))?;
        drive::drive_constraint_base(
            ctx,
            PERSIST_OPT,
            base,
            "POINT_EVENT accepted in an open EVENT slot",
            vec![],
        )
        .await
    })
}

/// EVENT `type_point_event`: author a `persistent_minimal` OPT narrowing
/// `HISTORY.events` to `POINT_EVENT`; the base `POINT_EVENT` is accepted and a copy
/// whose event `_type` is swapped to the sibling `INTERVAL_EVENT` is rejected
/// ("Class not allowed" — master16 §EVENT).
fn run_event_point<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let tid = "cnf_cont_event_point_event";
        let mut opt = author::parse_base(PERSIST_OPT)?;
        author::set_template_id(&mut opt, tid);
        if !author::narrow_nested_child_type(&mut opt, "HISTORY", "events", "POINT_EVENT") {
            return Err(CaseError::Assertion(
                "base OPT has no HISTORY.events child object to narrow".to_owned(),
            ));
        }
        let xml = author::to_xml(&opt)?;
        let base =
            fixtures::read_json(PERSIST_COMP).map_err(|e| CaseError::Codec(e.to_string()))?;

        let mut accepted = base.clone();
        mutate::retarget_template(&mut accepted, tid);

        let mut rejected = base.clone();
        mutate::retarget_template(&mut rejected, tid);
        if let Some(e) = mutate::first_node_mut(&mut rejected, "POINT_EVENT") {
            mutate::set_field(e, "_type", json!("INTERVAL_EVENT"));
        }

        drive::drive_authored(
            ctx,
            &xml,
            vec![
                (
                    "POINT_EVENT accepted (events narrowed to POINT_EVENT)".to_owned(),
                    accepted,
                    Expected::Accepted,
                ),
                (
                    "INTERVAL_EVENT rejected (Class not allowed)".to_owned(),
                    rejected,
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_skip_event_type<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::skip_archetype(
            "EVENT narrowing to INTERVAL_EVENT (needs a valid INTERVAL_EVENT instance — mandatory \
             width/math_function — absent from the vendored corpus)",
        )
    })
}

fn run_skip_item_str<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::skip_archetype("ITEM_STRUCTURE type narrowing (ITEM_TREE/LIST/TABLE/SINGLE)")
    })
}
