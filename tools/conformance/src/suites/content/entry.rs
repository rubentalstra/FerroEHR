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
    const OBS_CIT: &str = "RM 1.2.0 ehr §OBSERVATION; AM 1.4 archetype constraint; ITS-REST 1.0.3 composition_create (201/422)";
    const HIST_CIT: &str = "RM 1.2.0 ehr §HISTORY; AM 1.4 archetype constraint; ITS-REST 1.0.3 composition_create (201/422)";
    const EVENT_CIT: &str = "RM 1.2.0 ehr §EVENT; AM 1.4 archetype constraint; ITS-REST 1.0.3 composition_create (201/422)";
    const ITEM_CIT: &str = "RM 1.2.0 ehr §ITEM_STRUCTURE; AM 1.4 archetype constraint; ITS-REST 1.0.3 composition_create (201/422)";
    let mut all = Vec::new();

    // ── OBSERVATION (4) — driven: OBSERVATION.data existence (RM/schema) ──────
    for (id, title) in [
        (
            "val/obs-state-ex-opt-protocol-ex-opt",
            "Validate OBSERVATION — state ex OPT protocol ex OPT",
        ),
        (
            "val/obs-state-ex-opt-protocol-ex-mand",
            "Validate OBSERVATION — state ex OPT protocol ex mand",
        ),
        (
            "val/obs-state-ex-mand-protocol-ex-opt",
            "Validate OBSERVATION — state ex mand protocol ex OPT",
        ),
        (
            "val/obs-state-ex-mand-protocol-ex-mand",
            "Validate OBSERVATION — state ex mand protocol ex mand",
        ),
    ] {
        all.push(entry(id, title, OBS_CIT, run_obs_data));
    }

    // ── HISTORY (12) — driven against authored HISTORY.events cardinality +
    //    HISTORY.summary existence OPTs (the summary-absent half of each truth
    //    table: events cardinality {0..*,1..*,3..*,0..1,1..1,3..5} × summary
    //    existence {0..1,1..1}). The base persistent composition carries no
    //    HISTORY.summary, so the summary-absent rows exercise both the events
    //    cardinality (via the committed events count) and the summary existence
    //    (a mandatory summary is violated by its absence) constraints without
    //    fabricating extra RM data. ──
    #[allow(clippy::items_after_statements)]
    const HIST: &[(&str, &str, crate::harness::CaseRun)] = &[
        (
            "val/hist-events-card-any-summary-ex-opt",
            "Validate HISTORY — events card any summary ex OPT",
            h_any_opt,
        ),
        (
            "val/hist-events-card-1plus-summary-ex-opt",
            "Validate HISTORY — events card 1plus summary ex OPT",
            h_1plus_opt,
        ),
        (
            "val/hist-events-card-3plus-summary-ex-opt",
            "Validate HISTORY — events card 3plus summary ex OPT",
            h_3plus_opt,
        ),
        (
            "val/hist-events-card-opt-summary-ex-opt",
            "Validate HISTORY — events card OPT summary ex OPT",
            h_opt_opt,
        ),
        (
            "val/hist-events-card-mand-summary-ex-opt",
            "Validate HISTORY — events card mand summary ex OPT",
            h_mand_opt,
        ),
        (
            "val/hist-events-card-3to5-summary-ex-opt",
            "Validate HISTORY — events card 3to5 summary ex OPT",
            h_3to5_opt,
        ),
        (
            "val/hist-events-card-any-summary-ex-mand",
            "Validate HISTORY — events card any summary ex mand",
            h_any_mand,
        ),
        (
            "val/hist-events-card-1plus-summary-ex-mand",
            "Validate HISTORY — events card 1plus summary ex mand",
            h_1plus_mand,
        ),
        (
            "val/hist-events-card-3plus-summary-ex-mand",
            "Validate HISTORY — events card 3plus summary ex mand",
            h_3plus_mand,
        ),
        (
            "val/hist-events-card-opt-summary-ex-mand",
            "Validate HISTORY — events card OPT summary ex mand",
            h_opt_mand,
        ),
        (
            "val/hist-events-card-mand-summary-ex-mand",
            "Validate HISTORY — events card mand summary ex mand",
            h_mand_mand,
        ),
        (
            "val/hist-events-card-3to5-summary-ex-mand",
            "Validate HISTORY — events card 3to5 summary ex mand",
            h_3to5_mand,
        ),
    ];
    for &(id, title, run) in HIST {
        all.push(entry(id, title, HIST_CIT, run));
    }

    // ── EVENT (5) — data existence driven; type narrowing driven via authored
    //    OPTs. `type_any`: the persistent base narrows HISTORY.events only to the
    //    abstract EVENT, so committing the base POINT_EVENT is the "any subtype
    //    accepted" positive. `type_point_event`: author a POINT_EVENT narrowing —
    //    base accepted, a sibling (_type→INTERVAL_EVENT) rejected. `type_interval_
    //    event` needs a valid INTERVAL_EVENT the corpus does not carry (mandatory
    //    `width`/`math_function`) → honest skip. ──
    all.push(entry(
        "val/event-state-ex-opt",
        "Validate EVENT — state ex OPT",
        EVENT_CIT,
        run_event_data,
    ));
    all.push(entry(
        "val/event-state-ex-mand",
        "Validate EVENT — state ex mand",
        EVENT_CIT,
        run_event_data,
    ));
    all.push(entry(
        "val/event-type-any",
        "Validate EVENT — type any",
        EVENT_CIT,
        run_event_any,
    ));
    all.push(entry(
        "val/event-type-point-event",
        "Validate EVENT — type point event",
        EVENT_CIT,
        run_event_point,
    ));
    all.push(entry(
        "val/event-type-interval-event",
        "Validate EVENT — type interval event",
        EVENT_CIT,
        run_event_interval,
    ));

    // ── ITEM_STRUCTURE (5) — type narrowing, driven via clinical_content_validation ─
    // The `clinical_content_validation` OPT narrows four EVALUATION `data` slots to
    // a specific ITEM_STRUCTURE subtype; its vendored composition fills each with
    // the matching type (accepted). Swapping a slot's `_type` to a sibling subtype
    // is the truth-table "Class not allowed" rejection (master16 §ITEM_STRUCTURE).
    // `type_any` stays skipped: no vendored OPT leaves an ITEM_STRUCTURE slot open,
    // so the "any subtype accepted" positive cannot be isolated.
    all.push(entry(
        "val/item-str-type-any",
        "Validate ITEM_STRUCTURE — type any",
        ITEM_CIT,
        run_item_str_any,
    ));
    all.push(entry(
        "val/item-str-type-item-tree",
        "Validate ITEM_STRUCTURE — type item tree",
        ITEM_CIT,
        run_item_str_tree,
    ));
    all.push(entry(
        "val/item-str-type-item-list",
        "Validate ITEM_STRUCTURE — type item list",
        ITEM_CIT,
        run_item_str_list,
    ));
    all.push(entry(
        "val/item-str-type-item-table",
        "Validate ITEM_STRUCTURE — type item table",
        ITEM_CIT,
        run_item_str_table,
    ));
    all.push(entry(
        "val/item-str-type-item-single",
        "Validate ITEM_STRUCTURE — type item single",
        ITEM_CIT,
        run_item_str_single,
    ));

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

fn entry(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: meta(id, title, citation),
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

/// EVENT `type_interval_event`: author a `persistent_minimal` OPT narrowing
/// `HISTORY.events` to `INTERVAL_EVENT`; a valid `INTERVAL_EVENT` (the base event
/// augmented with the mandatory `width` + `math_function`) is accepted and the base
/// `POINT_EVENT` is rejected ("Class not allowed").
fn run_event_interval<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let tid = "cnf_cont_event_interval_event";
        let mut opt = author::parse_base(PERSIST_OPT)?;
        author::set_template_id(&mut opt, tid);
        if !author::narrow_nested_child_type(&mut opt, "HISTORY", "events", "INTERVAL_EVENT") {
            return Err(CaseError::Assertion(
                "base OPT has no HISTORY.events child object to narrow".to_owned(),
            ));
        }
        let xml = author::to_xml(&opt)?;
        let base =
            fixtures::read_json(PERSIST_COMP).map_err(|e| CaseError::Codec(e.to_string()))?;

        // Accepted: the base POINT_EVENT promoted to a valid INTERVAL_EVENT.
        let mut accepted = base.clone();
        mutate::retarget_template(&mut accepted, tid);
        if let Some(e) = mutate::first_node_mut(&mut accepted, "POINT_EVENT") {
            mutate::set_field(e, "_type", json!("INTERVAL_EVENT"));
            mutate::set_field(
                e,
                "width",
                json!({ "_type": "DV_DURATION", "value": "PT1H" }),
            );
            mutate::set_field(
                e,
                "math_function",
                json!({ "_type": "DV_CODED_TEXT", "value": "mean",
                    "defining_code": { "_type": "CODE_PHRASE",
                        "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                        "code_string": "146" } }),
            );
        }

        // Rejected: the base POINT_EVENT (a sibling the INTERVAL_EVENT slot forbids).
        let mut rejected = base;
        mutate::retarget_template(&mut rejected, tid);

        drive::drive_authored(
            ctx,
            &xml,
            vec![
                (
                    "INTERVAL_EVENT accepted (events narrowed to INTERVAL_EVENT)".to_owned(),
                    accepted,
                    Expected::Accepted,
                ),
                (
                    "POINT_EVENT rejected (Class not allowed)".to_owned(),
                    rejected,
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

/// `ITEM_STRUCTURE` `type_any`: author the `clinical_content_validation` OPT with the
/// `ITEM_TREE`-narrowed EVALUATION `data` slot **re-opened** to the abstract
/// `ITEM_STRUCTURE`, then commit the vendored composition (its slot filled with
/// `ITEM_TREE`) and a copy with that slot's `_type` swapped to `ITEM_LIST` — both
/// accepted, exercising "any `ITEM_STRUCTURE` subtype accepted" (master16
/// §`ITEM_STRUCTURE`).
fn run_item_str_any<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let tid = "cnf_cont_item_str_any";
        let mut opt = author::parse_base("validation/clinical_content_validation.opt")?;
        author::set_template_id(&mut opt, tid);
        // Re-open the ITEM_TREE-narrowed EVALUATION data slot to the abstract
        // ITEM_STRUCTURE so any subtype is accepted. Pinned to EVALUATION/data
        // (retype_attr_child): the template carries other ITEM_TREE leaves
        // (INSTRUCTION description first in document order) and a blanket
        // first-match retype re-opened the wrong one — leaving this slot
        // ITEM_TREE-narrowed, which a spec-correct validator must then reject.
        author::retype_attr_child(
            &mut opt,
            "EVALUATION",
            "data",
            "ITEM_TREE",
            author::open_complex("ITEM_STRUCTURE"),
        );
        let xml = author::to_xml(&opt)?;
        let base = fixtures::read_json(
            "compositions/CANONICAL_JSON/clinical_content_validation__full.json",
        )
        .map_err(|e| CaseError::Codec(e.to_string()))?;

        let mut tree = base.clone();
        mutate::retarget_template(&mut tree, tid);
        let mut list = base;
        mutate::retarget_template(&mut list, tid);
        // A bare `_type` swap would leave the ITEM_TREE's CLUSTER items in
        // place, and RM 1.2.0 `ITEM_LIST.items` is `List<ELEMENT>` (RM
        // data_structures §ITEM_LIST) — the instance must be RM-valid for the
        // "any subtype accepted" positive to isolate the archetype slot. Build
        // the ITEM_LIST from the tree's ELEMENT leaves instead.
        let elements: Vec<Value> = list
            .pointer("/content/2/data/items")
            .and_then(Value::as_array)
            .map(|clusters| {
                clusters
                    .iter()
                    .filter_map(|c| c.get("items").and_then(Value::as_array))
                    .flatten()
                    .filter(|i| i.get("_type").and_then(Value::as_str) == Some("ELEMENT"))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        mutate::set_pointer(&mut list, "/content/2/data/_type", json!("ITEM_LIST"));
        mutate::set_pointer(&mut list, "/content/2/data/items", Value::Array(elements));

        drive::drive_authored(
            ctx,
            &xml,
            vec![
                (
                    "ITEM_TREE accepted in an open ITEM_STRUCTURE slot".to_owned(),
                    tree,
                    Expected::Accepted,
                ),
                (
                    "ITEM_LIST accepted in an open ITEM_STRUCTURE slot (any subtype)".to_owned(),
                    list,
                    Expected::Accepted,
                ),
            ],
        )
        .await
    })
}
