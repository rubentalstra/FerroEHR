//! master15 — COMPOSITION data-validation truth tables
//! (`master15-content_tc_composition.adoc`).
//!
//! Every CONT-COMP case constrains `COMPOSITION.content` **cardinality** (0..*,
//! 1..*, 3..*, 0..1, 1..1, 3..5) and/or `COMPOSITION.context` **occurrences**
//! (0..* vs 1..1). In the RM `COMPOSITION.content` is `0..*` and
//! `COMPOSITION.context` is `0..1`; a specific interval is expressible **only** in
//! an archetype/OPT, and the vendored corpus ships no OPT per cardinality variant
//! (master15 §Implementation notes: the archetypes "should be generated"). Rather
//! than skip, the suite **authors** the constraint OPT ([`super::author`]): a
//! `minimal_evaluation` base (a single `EVALUATION` content archetype whose own
//! occurrences are already `0..*`, so varying the number of committed content items
//! exercises *only* the attribute cardinality — master15 §Isolation) is tightened
//! to the case's content cardinality (+ a mandatory `context` `C_SINGLE_ATTRIBUTE`
//! for the `context_mand` cases), re-serialised, and uploaded. Each case then
//! commits the truth-table data sets — content counts {0,1,3} × context
//! {present, absent} — and asserts accepted/rejected (design §4.5).
//!
//! Content-cardinality expectation: a count `n` is accepted iff it lies in the
//! constrained interval. Context expectation: `context_mand` rejects a composition
//! with no `context`; `context_any` accepts either. (Our server does not enforce
//! the RM `Category_validity` invariant — persistent/event ⇒ context rules — so a
//! missing `context` is RM-accepted and only the authored OPT's `context`
//! existence governs it, isolating the occurrences constraint.)

use serde_json::{Value, json};

use crate::case::Chapter;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, DataSetReport, RunContext};
use crate::registry::CaseEntry;

use super::author::{self, Card};
use super::drive::{self, Expected, meta};
use super::mutate;

/// The `minimal_evaluation` base composition (a category-`event` COMPOSITION with
/// one `EVALUATION` content item and a valid `context`) — corpus-root-relative.
const BASE_COMP: &str = "compositions/valid/minimal_evaluation_1.composition.json";
/// The `minimal_evaluation` base OPT (relative to `valid_templates/`).
const BASE_OPT: &str = "minimal/minimal_evaluation.opt";

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

/// Retarget every `archetype_details.template_id` in a composition to `tid` so the
/// instance references the authored template (the `WebTemplate` the SUT builds is
/// keyed by the root `template_id`; the nested ones are retargeted for internal
/// consistency).
fn retarget(v: &mut Value, tid: &str) {
    match v {
        Value::Object(map) => {
            if let Some(Value::Object(ad)) = map.get_mut("archetype_details")
                && let Some(Value::Object(t)) = ad.get_mut("template_id")
            {
                t.insert("value".to_owned(), json!(tid));
            }
            for child in map.values_mut() {
                retarget(child, tid);
            }
        }
        Value::Array(items) => {
            for it in items {
                retarget(it, tid);
            }
        }
        _ => {}
    }
}

/// Build one data-set instance: a clone of `base` retargeted to `tid`, with the
/// content array resized to `count` copies of the base content item and `context`
/// removed when `context_present` is false.
fn instance(base: &Value, tid: &str, count: usize, context_present: bool) -> Value {
    let mut c = base.clone();
    retarget(&mut c, tid);
    mutate::set_array_count(&mut c, "content", count);
    if !context_present {
        mutate::remove_context(&mut c);
    }
    c
}

/// Drive one CONT-COMP case: author the constraint OPT for `card` (+ mandatory
/// `context` when `context_mand`), then commit the {0,1,3}×{present,absent} data
/// sets against it.
async fn drive_case(
    ctx: &RunContext<'_>,
    tid: &'static str,
    card: Card,
    context_mand: bool,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(BASE_OPT)?;
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

    let base = fixtures::read_json(BASE_COMP).map_err(|e| CaseError::Codec(e.to_string()))?;

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
    drive::drive_authored(ctx, &xml, rows).await
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

/// The implemented master15 case entries (all 12 CONT-COMP cases, driven against
/// authored constraint OPTs).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    const CASES: &[(&str, crate::harness::CaseRun)] = &[
        ("CONT-COMP-content_card_any-context_any", r_any_any),
        ("CONT-COMP-content_card_1plus-context_any", r_1plus_any),
        ("CONT-COMP-content_card_3plus-context_any", r_3plus_any),
        ("CONT-COMP-content_card_opt-context_any", r_opt_any),
        ("CONT-COMP-content_card_mand-context_any", r_mand_any),
        ("CONT-COMP-content_card_3to5-context_any", r_3to5_any),
        ("CONT-COMP-content_card_any-context_mand", r_any_mand),
        ("CONT-COMP-content_card_1plus-context_mand", r_1plus_mand),
        ("CONT-COMP-content_card_3plus-context_mand", r_3plus_mand),
        ("CONT-COMP-content_card_opt-context_mand", r_opt_mand),
        ("CONT-COMP-content_card_mand-context_mand", r_mand_mand),
        ("CONT-COMP-content_card_3to5-context_mand", r_3to5_mand),
    ];
    CASES
        .iter()
        .map(|&(id, run)| CaseEntry {
            meta: meta(id, Chapter::Master15, id),
            run,
        })
        .collect()
}
