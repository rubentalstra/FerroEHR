//! master15 — COMPOSITION data-validation truth tables
//! (`master15-content_tc_composition.adoc`).
//!
//! Every `CONT-COMP-*` case constrains `COMPOSITION.content` **cardinality**
//! (one of `0..*`, `1..*`, `3..*`, `0..1`, `1..1`, `3..5` — master15 §"For
//! testing a 'multiple attribute' cardinality") crossed with
//! `COMPOSITION.context` **occurrences** (`_any` = unconstrained `0..1` vs
//! `_mand` = `1..1`). In the RM `COMPOSITION.content` is `0..1` (a `List`) and
//! `COMPOSITION.context` is `0..1`; a specific interval is expressible **only**
//! in an archetype/OPT, and the vendored corpus ships no OPT per variant
//! (master15 §Implementation notes: the archetypes "should be generated"). The
//! suite therefore **authors** the constraint OPT ([`super::author`]): a
//! `minimal_evaluation` base (category-event COMPOSITION with one `EVALUATION`
//! content archetype whose occurrences are already `0..*`, so varying the
//! committed content count exercises *only* the attribute cardinality —
//! master15 §Isolation) is tightened to the case's content cardinality (+ a
//! mandatory `context` for the `_mand` cases), re-serialised, uploaded, and the
//! truth-table data sets committed.
//!
//! **Schedule table (9 rows):** content ∈ {no entries, one entry, three
//! entries} × context ∈ {no context, context w/o `other_context`, context w/
//! `other_context`}. The accept/reject oracle depends only on the content count
//! vs the interval and on context present-vs-absent — `other_context` never
//! flips the outcome across any master15 table, so the runner
//! drives the 6 rows {0,1,3}×{present,absent} and declares the 9-row bound via
//! [`DataSetReport::of_schedule_rows`].
//!
//! **Context isolation:** RM 1.2.0 composition §COMPOSITION
//! carries no category↔context invariant — `context` is `0..1` and the only
//! COMPOSITION invariants are `Category_validity` (category code valid) and
//! `Territory_valid`. A missing `context` is therefore RM-legal on the
//! category-event base regardless of category, so the authored OPT's `context`
//! existence is the sole governing constraint — the isolation is genuine, not a
//! dependency on the server skipping an RM invariant.

use serde_json::Value;

use crate::engine::harness::{CaseError, CaseFuture, DataSetReport, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::ScheduleTrace;

use super::author::{self, Card};
use super::drive::{self, Expected};
use super::mutate;

/// The `minimal_evaluation` base composition (a category-event COMPOSITION with
/// one `EVALUATION` content item + a valid `context`) — the `content.base.*`
/// single-file manifest key.
const BASE_COMP_KEY: &str = "content.base.minimal-evaluation.composition";
/// The `minimal_evaluation` base OPT file (under the `template.valid` dir key).
const BASE_OPT_FILE: &str = "minimal/minimal_evaluation.opt";

/// The number of rows the master15 COMPOSITION truth tables tabulate (3 content
/// × 3 context) — the coverage bound (`other_context` never
/// flips the outcome, so 6 of the 9 are driven).
const SCHEDULE_ROWS: u32 = 9;

const CIT: &str = "RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.0.3 composition_create (201 / 422 validation)";

/// Whether a content count `n` satisfies a cardinality interval.
fn content_ok(card: Card, count: usize) -> bool {
    match card {
        Card::Any => true,
        Card::OnePlus => count >= 1,
        Card::ThreePlus => count >= 3,
        Card::Opt => count <= 1,
        Card::Mand => count == 1,
        Card::ThreeToFive => (3..=5).contains(&count),
    }
}

/// Build one data-set instance: a clone of `base` retargeted to `tid`, with the
/// content array resized to `count` and `context` removed when `!context_present`.
fn instance(base: &Value, tid: &str, count: usize, context_present: bool) -> Value {
    let mut c = base.clone();
    mutate::retarget_template(&mut c, tid);
    mutate::set_array_count(&mut c, "content", count);
    if !context_present {
        mutate::remove_context(&mut c);
    }
    c
}

/// Drive one CONT-COMP case: author the constraint OPT for `card` (+ mandatory
/// `context` when `context_mand`), then commit the {0,1,3}×{present,absent} data
/// sets. Content count `n` is accepted iff it lies in the interval; a
/// `context_mand` composition with no context is rejected on
/// `COMPOSITION.context occurrences.lower`.
async fn drive_case(
    ctx: &RunContext<'_>,
    tid: &'static str,
    card: Card,
    context_mand: bool,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(BASE_OPT_FILE)?;
    author::set_template_id(&mut opt, tid);
    if !author::set_root_multiple_cardinality(&mut opt, "content", card.interval()) {
        return Err(CaseError::Assertion(
            "base OPT has no COMPOSITION.content multiple attribute to constrain".to_owned(),
        ));
    }
    if context_mand {
        author::set_root_single_mandatory(&mut opt, "context");
    }
    let xml = author::to_xml(&opt)?;

    let text = crate::testdata::fixtures::read(BASE_COMP_KEY)
        .map_err(|e| CaseError::Codec(e.to_string()))?;
    let base: Value = serde_json::from_str(&text).map_err(|e| CaseError::Codec(e.to_string()))?;

    let mut rows: Vec<(String, Value, Expected)> = Vec::new();
    for &count in &[0usize, 1, 3] {
        for &context_present in &[true, false] {
            let accepted = content_ok(card, count) && (context_present || !context_mand);
            let ctx_label = if context_present {
                "context present"
            } else {
                "no context"
            };
            let label = format!(
                "{count} content item(s), {ctx_label} → {}",
                if accepted { "accepted" } else { "rejected" }
            );
            let expected = if accepted {
                Expected::Accepted
            } else {
                Expected::Rejected
            };
            rows.push((
                label,
                instance(&base, tid, count, context_present),
                expected,
            ));
        }
    }
    let report = drive::drive_authored(ctx, &xml, rows).await?;
    Ok(report.of_schedule_rows(SCHEDULE_ROWS))
}

/// Generate a per-case run function (the `CaseRun` fn-pointer carries no case id,
/// so each case's cardinality/context is bound in a distinct function).
macro_rules! cont_comp {
    ($fn:ident, $tid:literal, $card:expr, $ctx_mand:expr) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move { drive_case(ctx, $tid, $card, $ctx_mand).await })
        }
    };
}

cont_comp!(
    r_any_any,
    "cnf_cont_comp_card_any_ctx_any",
    Card::Any,
    false
);
cont_comp!(
    r_1plus_any,
    "cnf_cont_comp_card_1plus_ctx_any",
    Card::OnePlus,
    false
);
cont_comp!(
    r_3plus_any,
    "cnf_cont_comp_card_3plus_ctx_any",
    Card::ThreePlus,
    false
);
cont_comp!(
    r_opt_any,
    "cnf_cont_comp_card_opt_ctx_any",
    Card::Opt,
    false
);
cont_comp!(
    r_mand_any,
    "cnf_cont_comp_card_mand_ctx_any",
    Card::Mand,
    false
);
cont_comp!(
    r_3to5_any,
    "cnf_cont_comp_card_3to5_ctx_any",
    Card::ThreeToFive,
    false
);
cont_comp!(
    r_any_mand,
    "cnf_cont_comp_card_any_ctx_mand",
    Card::Any,
    true
);
cont_comp!(
    r_1plus_mand,
    "cnf_cont_comp_card_1plus_ctx_mand",
    Card::OnePlus,
    true
);
cont_comp!(
    r_3plus_mand,
    "cnf_cont_comp_card_3plus_ctx_mand",
    Card::ThreePlus,
    true
);
cont_comp!(
    r_opt_mand,
    "cnf_cont_comp_card_opt_ctx_mand",
    Card::Opt,
    true
);
cont_comp!(
    r_mand_mand,
    "cnf_cont_comp_card_mand_ctx_mand",
    Card::Mand,
    true
);
cont_comp!(
    r_3to5_mand,
    "cnf_cont_comp_card_3to5_ctx_mand",
    Card::ThreeToFive,
    true
);

/// The 12 master15 CONT-COMP cases (ECC-VAL-001..012), driven against authored
/// constraint OPTs.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    // (slug, title, schedule-case-id, run) — slugs + titles are the carried
    // ECC-VAL-001..012 ids (kept stable across renumbering).
    const CASES: &[(&str, &str, &str, crate::engine::harness::CaseRun)] = &[
        (
            "val/comp-content-card-any-context-any",
            "Validate COMPOSITION — content card any context any",
            "CONT-COMP.content_card_any-context_any (master15 §COMPOSITION Test Cases)",
            r_any_any,
        ),
        (
            "val/comp-content-card-1plus-context-any",
            "Validate COMPOSITION — content card 1plus context any",
            "CONT-COMP.content_card_1plus-context_any (master15 §COMPOSITION Test Cases)",
            r_1plus_any,
        ),
        (
            "val/comp-content-card-3plus-context-any",
            "Validate COMPOSITION — content card 3plus context any",
            "CONT-COMP.content_card_3plus-context_any (master15 §COMPOSITION Test Cases)",
            r_3plus_any,
        ),
        (
            "val/comp-content-card-opt-context-any",
            "Validate COMPOSITION — content card OPT context any",
            "CONT-COMP.content_card_opt-context_any (master15 §COMPOSITION Test Cases)",
            r_opt_any,
        ),
        (
            "val/comp-content-card-mand-context-any",
            "Validate COMPOSITION — content card mand context any",
            "CONT-COMP.content_card_mand-context_any (master15 §COMPOSITION Test Cases)",
            r_mand_any,
        ),
        (
            "val/comp-content-card-3to5-context-any",
            "Validate COMPOSITION — content card 3to5 context any",
            "CONT-COMP.content_card_3to5-context_any (master15 §COMPOSITION Test Cases)",
            r_3to5_any,
        ),
        (
            "val/comp-content-card-any-context-mand",
            "Validate COMPOSITION — content card any context mand",
            "CONT-COMP.content_card_any-context_mand (master15 §COMPOSITION Test Cases)",
            r_any_mand,
        ),
        (
            "val/comp-content-card-1plus-context-mand",
            "Validate COMPOSITION — content card 1plus context mand",
            "CONT-COMP.content_card_1plus-context_mand (master15 §COMPOSITION Test Cases)",
            r_1plus_mand,
        ),
        (
            "val/comp-content-card-3plus-context-mand",
            "Validate COMPOSITION — content card 3plus context mand",
            "CONT-COMP.content_card_3plus-context_mand (master15 §COMPOSITION Test Cases)",
            r_3plus_mand,
        ),
        (
            "val/comp-content-card-opt-context-mand",
            "Validate COMPOSITION — content card OPT context mand",
            "CONT-COMP.content_card_opt-context_mand (master15 §COMPOSITION Test Cases)",
            r_opt_mand,
        ),
        (
            "val/comp-content-card-mand-context-mand",
            "Validate COMPOSITION — content card mand context mand",
            "CONT-COMP.content_card_mand-context_mand (master15 §COMPOSITION Test Cases)",
            r_mand_mand,
        ),
        (
            "val/comp-content-card-3to5-context-mand",
            "Validate COMPOSITION — content card 3to5 context mand",
            "CONT-COMP.content_card_3to5-context_mand (master15 §COMPOSITION Test Cases)",
            r_3to5_mand,
        ),
    ];
    CASES
        .iter()
        .map(|&(id, title, sched, run)| CaseEntry {
            meta: drive::content_meta(id, title, CIT, ScheduleTrace::Schedule(sched)),
            run,
        })
        .collect()
}
