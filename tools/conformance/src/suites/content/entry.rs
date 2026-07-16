//! master16 — ENTRY data-validation truth tables (`master16-content_tc_entry.adoc`;
//! register 12 §2.2–2.5): OBSERVATION, HISTORY, EVENT, and `ITEM_STRUCTURE`.
//!
//! All 26 cases author a constraining OPT (or narrow the vendored one) and
//! commit truth-table data-set instances against it — never a fabricated pass,
//! never a masked skip. By family:
//!
//! - **OBSERVATION (4, ECC-VAL-013..016)** — `state`/`protocol` **existence**
//! (`0..1` vs `1..1`). The persistent base carries data but neither
//! `state` nor `protocol`, so each case authors the existence constraint
//! ([`author::constrain_nested_single_mandatory`] on `OBSERVATION`) and drives
//! three genuine rows of the 8-row table (register 12 G-1): the RM/schema
//! `data`-absent reject, the archetype existence-boundary row
//! (`data present, state/protocol absent`) that **distinguishes** this case
//! from its siblings, and a `state`+`protocol`-present accept row (injected —
//! see [`inject_obs_state_protocol`]). The five `data`-absent-only and the
//! remaining `present` permutations are the declared coverage bound
//! ([`DataSetReport::of_schedule_rows`]).
//! - **HISTORY (12, ECC-VAL-017..028)** — `events` cardinality × `summary`
//! existence, authored OPT, {0,1,3} events with `summary` absent (the
//! summary-absent half of each 6-row table). RM `HISTORY.Events_valid` (≥1
//! event OR a summary) overrides the schedule's "no events, absent summary →
//! accepted" row (register 12 G-7): the RM invariant is spec-authoritative
//! over the printed schedule table.
//! - **EVENT (5, ECC-VAL-029..033)** — `state` existence (2, like OBSERVATION)
//! + type narrowing (`POINT_EVENT`/`INTERVAL_EVENT`; abstract `EVENT` accepts
//! either). The valid `INTERVAL_EVENT` is fabricated from the base
//! `POINT_EVENT` + the mandatory `width`/`math_function` (register 12 G-8,
//! [`build_interval_event`]).
//! - **`ITEM_STRUCTURE` (5, ECC-VAL-034..038)** — type narrowing driven against
//! `clinical_content_validation` (four EVALUATION `data` slots narrowed to
//! `ITEM_SINGLE`/`TREE`/`LIST`/`TABLE`): the vendored composition accepted, a
//! sibling `_type` rejected ("Class not allowed"). `type_any` re-opens the
//! slot to abstract `ITEM_STRUCTURE`.

use serde_json::{Value, json};

use crate::engine::harness::{CaseError, CaseFuture, DataSetReport, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::ScheduleTrace;

use super::author::{self, Card};
use super::drive::{self, Base, Constraint, Expected, PERSIST_OPT_FILE};
use super::mutate;

const OBS_CIT: &str = "RM 1.2.0 ehr §OBSERVATION (data 1..1; state/protocol 0..1); AM aom14 §C_ATTRIBUTE existence; ITS-REST 1.0.3 composition_create (201 / 422 validation)";
const HIST_CIT: &str = "RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.0.3 composition_create (201 / 422)";
const EVENT_CIT: &str = "RM 1.2.0 data_structures §EVENT/POINT_EVENT/INTERVAL_EVENT (state 0..1; INTERVAL_EVENT.width/math_function mandatory); AM aom14 §type narrowing; ITS-REST 1.0.3 composition_create (201 / 422)";
const ITEM_CIT: &str = "RM 1.2.0 data_structures §ITEM_STRUCTURE (ITEM_TREE/LIST/TABLE/SINGLE); AM aom14 §type narrowing (Class not allowed); ITS-REST 1.0.3 composition_create (201 / 422)";

/// `clinical_content_validation.opt` + its vendored composition (content slots:
/// `content[1].data`=`ITEM_SINGLE`, `[2]`=`ITEM_TREE`, `[3]`=`ITEM_LIST`,
/// `[4]`=`ITEM_TABLE`, each narrowed by the OPT to that exact subtype).
const CLINICAL: Constraint = Constraint {
    opt_file: "validation/clinical_content_validation.opt",
    comp: drive::CompBase::InDir {
        dir_key: "composition.canonical-json",
        file: "clinical_content_validation__full.json",
    },
};

/// The persistent base composition (a persistent COMPOSITION carrying
/// OBSERVATION → HISTORY → `POINT_EVENT`, no `state`/`protocol`/`summary`).
fn persist_base() -> Result<Value, CaseError> {
    Base::PersistentMinimal.load()
}

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

/// The 26 master16 ENTRY cases (ECC-VAL-013..038).
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the registered ECC case table is inherently enumerative"
)]
pub fn entries() -> Vec<CaseEntry> {
    let mut all = Vec::new();

    // ── OBSERVATION (4) — state/protocol existence (register 12 G-1) ──────────
    let obs: [(&str, &str, &str, crate::engine::harness::CaseRun); 4] = [
        (
            "val/obs-state-ex-opt-protocol-ex-opt",
            "Validate OBSERVATION — state ex OPT protocol ex OPT",
            "ENTRY.CONT-OBS-state_ex_opt-protocol_ex_opt (master16 §OBSERVATION Test Cases)",
            obs_opt_opt,
        ),
        (
            "val/obs-state-ex-opt-protocol-ex-mand",
            "Validate OBSERVATION — state ex OPT protocol ex mand",
            "ENTRY.CONT-OBS-state_ex_opt-protocol_ex_mand (master16 §OBSERVATION Test Cases)",
            obs_opt_mand,
        ),
        (
            "val/obs-state-ex-mand-protocol-ex-opt",
            "Validate OBSERVATION — state ex mand protocol ex OPT",
            "ENTRY.CONT-OBS-state_ex_mand-protocol_ex_opt (master16 §OBSERVATION Test Cases)",
            obs_mand_opt,
        ),
        (
            "val/obs-state-ex-mand-protocol-ex-mand",
            "Validate OBSERVATION — state ex mand protocol ex mand",
            "ENTRY.CONT-OBS-state_ex_mand-protocol_ex_mand (master16 §OBSERVATION Test Cases)",
            obs_mand_mand,
        ),
    ];
    for (id, title, sched, run) in obs {
        all.push(CaseEntry {
            meta: drive::content_meta(id, title, OBS_CIT, ScheduleTrace::Schedule(sched)),
            run,
        });
    }

    // ── HISTORY (12) — events cardinality × summary existence ─────────────────
    let hist: [(&str, &str, &str, crate::engine::harness::CaseRun); 12] = [
        (
            "val/hist-events-card-any-summary-ex-opt",
            "Validate HISTORY — events card any summary ex OPT",
            "ENTRY.CONT-HIST-events_card_any-summary_ex_opt (master16 §HISTORY Test Cases)",
            h_any_opt,
        ),
        (
            "val/hist-events-card-1plus-summary-ex-opt",
            "Validate HISTORY — events card 1plus summary ex OPT",
            "ENTRY.CONT-HIST-events_card_1plus-summary_ex_opt (master16 §HISTORY Test Cases)",
            h_1plus_opt,
        ),
        (
            "val/hist-events-card-3plus-summary-ex-opt",
            "Validate HISTORY — events card 3plus summary ex OPT",
            "ENTRY.CONT-HIST-events_card_3plus-summary_ex_opt (master16 §HISTORY Test Cases)",
            h_3plus_opt,
        ),
        (
            "val/hist-events-card-opt-summary-ex-opt",
            "Validate HISTORY — events card OPT summary ex OPT",
            "ENTRY.CONT-HIST-events_card_opt-summary_ex_opt (master16 §HISTORY Test Cases)",
            h_opt_opt,
        ),
        (
            "val/hist-events-card-mand-summary-ex-opt",
            "Validate HISTORY — events card mand summary ex OPT",
            "ENTRY.CONT-HIST-events_card_mand-summary_ex_opt (master16 §HISTORY Test Cases)",
            h_mand_opt,
        ),
        (
            "val/hist-events-card-3to5-summary-ex-opt",
            "Validate HISTORY — events card 3to5 summary ex OPT",
            "ENTRY.CONT-HIST-events_card_3to5-summary_ex_opt (master16 §HISTORY Test Cases)",
            h_3to5_opt,
        ),
        (
            "val/hist-events-card-any-summary-ex-mand",
            "Validate HISTORY — events card any summary ex mand",
            "ENTRY.CONT-HIST-events_card_any-summary_ex_mand (master16 §HISTORY Test Cases)",
            h_any_mand,
        ),
        (
            "val/hist-events-card-1plus-summary-ex-mand",
            "Validate HISTORY — events card 1plus summary ex mand",
            "ENTRY.CONT-HIST-events_card_1plus-summary_ex_mand (master16 §HISTORY Test Cases)",
            h_1plus_mand,
        ),
        (
            "val/hist-events-card-3plus-summary-ex-mand",
            "Validate HISTORY — events card 3plus summary ex mand",
            "ENTRY.CONT-HIST-events_card_3plus-summary_ex_mand (master16 §HISTORY Test Cases)",
            h_3plus_mand,
        ),
        (
            "val/hist-events-card-opt-summary-ex-mand",
            "Validate HISTORY — events card OPT summary ex mand",
            "ENTRY.CONT-HIST-events_card_opt-summary_ex_mand (master16 §HISTORY Test Cases)",
            h_opt_mand,
        ),
        (
            "val/hist-events-card-mand-summary-ex-mand",
            "Validate HISTORY — events card mand summary ex mand",
            "ENTRY.CONT-HIST-events_card_mand-summary_ex_mand (master16 §HISTORY Test Cases)",
            h_mand_mand,
        ),
        (
            "val/hist-events-card-3to5-summary-ex-mand",
            "Validate HISTORY — events card 3to5 summary ex mand",
            "ENTRY.CONT-HIST-events_card_3to5-summary_ex_mand (master16 §HISTORY Test Cases)",
            h_3to5_mand,
        ),
    ];
    for (id, title, sched, run) in hist {
        all.push(CaseEntry {
            meta: drive::content_meta(id, title, HIST_CIT, ScheduleTrace::Schedule(sched)),
            run,
        });
    }

    // ── EVENT (5) — state existence + type narrowing ──────────────────────────
    let event: [(&str, &str, &str, crate::engine::harness::CaseRun); 5] = [
        (
            "val/event-state-ex-opt",
            "Validate EVENT — state ex OPT",
            "ENTRY.CONT-EVENT-state_ex_opt (master16 §EVENT Test Cases)",
            event_state_opt,
        ),
        (
            "val/event-state-ex-mand",
            "Validate EVENT — state ex mand",
            "ENTRY.CONT-EVENT-state_ex_mand (master16 §EVENT Test Cases)",
            event_state_mand,
        ),
        (
            "val/event-type-any",
            "Validate EVENT — type any",
            "ENTRY.CONT-EVENT-type_any (master16 §EVENT Test Cases)",
            event_type_any,
        ),
        (
            "val/event-type-point-event",
            "Validate EVENT — type point event",
            "ENTRY.CONT-EVENT-type_point_event (master16 §EVENT Test Cases)",
            event_type_point,
        ),
        (
            "val/event-type-interval-event",
            "Validate EVENT — type interval event",
            "ENTRY.CONT-EVENT-type_interval_event (master16 §EVENT Test Cases)",
            event_type_interval,
        ),
    ];
    for (id, title, sched, run) in event {
        all.push(CaseEntry {
            meta: drive::content_meta(id, title, EVENT_CIT, ScheduleTrace::Schedule(sched)),
            run,
        });
    }

    // ── ITEM_STRUCTURE (5) — type narrowing ───────────────────────────────────
    let item_str: [(&str, &str, &str, crate::engine::harness::CaseRun); 5] = [
        (
            "val/item-str-type-any",
            "Validate ITEM_STRUCTURE — type any",
            "ENTRY.CONT-ITEM_STR-type_any (master16 §ITEM_STRUCTURE Test Cases)",
            item_str_any,
        ),
        (
            "val/item-str-type-item-tree",
            "Validate ITEM_STRUCTURE — type item tree",
            "ENTRY.CONT-ITEM_STR-type_item_tree (master16 §ITEM_STRUCTURE Test Cases)",
            item_str_tree,
        ),
        (
            "val/item-str-type-item-list",
            "Validate ITEM_STRUCTURE — type item list",
            "ENTRY.CONT-ITEM_STR-type_item_list (master16 §ITEM_STRUCTURE Test Cases)",
            item_str_list,
        ),
        (
            "val/item-str-type-item-table",
            "Validate ITEM_STRUCTURE — type item table",
            "ENTRY.CONT-ITEM_STR-type_item_table (master16 §ITEM_STRUCTURE Test Cases)",
            item_str_table,
        ),
        (
            "val/item-str-type-item-single",
            "Validate ITEM_STRUCTURE — type item single",
            "ENTRY.CONT-ITEM_STR-type_item_single (master16 §ITEM_STRUCTURE Test Cases)",
            item_str_single,
        ),
    ];
    for (id, title, sched, run) in item_str {
        all.push(CaseEntry {
            meta: drive::content_meta(id, title, ITEM_CIT, ScheduleTrace::Schedule(sched)),
            run,
        });
    }

    all
}

// ── OBSERVATION state/protocol existence (register 12 G-1) ────────────────────

/// Inject an `OBSERVATION.state` (a HISTORY, cloned from the mandatory
/// `data`) and `OBSERVATION.protocol` (an `ITEM_STRUCTURE`, cloned from the
/// event's `ITEM_TREE` `data`) — a fabricated `state present, protocol present`
/// instance the persistent base lacks (register 12 G-1/G-8). RM 1.2.0 ehr
/// §OBSERVATION: `state` is `HISTORY<ITEM_STRUCTURE>`, `protocol` is
/// `ITEM_STRUCTURE` — both cloned subtrees are RM-valid.
fn inject_obs_state_protocol(comp: &mut Value) {
    if let Some(obs) = mutate::first_node_mut(comp, "OBSERVATION") {
        let state = obs.get("data").cloned();
        let protocol = obs.pointer("/data/events/0/data").cloned();
        if let Some(s) = state {
            mutate::set_field(obs, "state", s);
        }
        if let Some(p) = protocol {
            mutate::set_field(obs, "protocol", p);
        }
    }
}

/// Inject an `EVENT.state` (an `ITEM_STRUCTURE`, cloned from the event's `ITEM_TREE`
/// `data`) — the fabricated `state present` instance (RM 1.2.0 `data_structures`
/// §EVENT: `state` is `ITEM_STRUCTURE`).
fn inject_event_state(comp: &mut Value) {
    if let Some(ev) = mutate::first_node_mut(comp, "POINT_EVENT") {
        let state = ev.get("data").cloned();
        if let Some(s) = state {
            mutate::set_field(ev, "state", s);
        }
    }
}

/// Drive one OBSERVATION existence case (register 12 G-1): author the
/// `state`/`protocol` existence constraint on the persistent OPT, then commit
/// three genuine rows of the 8-row master16 table — the RM/schema `data`-absent
/// reject, the archetype existence-boundary row (`data present, state/protocol
/// absent`, which distinguishes this case), and the `state`+`protocol`-present
/// accept row. The remaining rows (further `data`-absent permutations; other
/// present combinations) are the declared coverage bound.
///
/// Boundary: the `present` accept row commits `state`/`protocol` subtrees the
/// base lacks against a slot the OPT constrains only by existence — it probes
/// the master16 §Isolation "anything valid in the RM allowed" acceptance; a
/// rejection there is a genuine finding (open-slot handling), not masked.
async fn drive_obs(
    ctx: &RunContext<'_>,
    tid: &'static str,
    state_mand: bool,
    protocol_mand: bool,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(PERSIST_OPT_FILE)?;
    author::set_template_id(&mut opt, tid);
    if state_mand {
        author::constrain_nested_single_mandatory(&mut opt, "OBSERVATION", "state");
    }
    if protocol_mand {
        author::constrain_nested_single_mandatory(&mut opt, "OBSERVATION", "protocol");
    }
    let xml = author::to_xml(&opt)?;
    let base = persist_base()?;

    // Row A: data absent → reject (RM/schema OBSERVATION.data existence.lower).
    let mut data_absent = base.clone();
    mutate::retarget_template(&mut data_absent, tid);
    if let Some(o) = mutate::first_node_mut(&mut data_absent, "OBSERVATION") {
        mutate::remove_field(o, "data");
    }

    // Row B: data present, state/protocol absent → accept iff neither mandatory
    // (else reject on the mandated attribute's existence.lower). This is
    // the row that distinguishes the four OBS cases.
    let mut boundary = base.clone();
    mutate::retarget_template(&mut boundary, tid);
    let boundary_accept = !state_mand && !protocol_mand;

    // Row C: data present, state present, protocol present → accepted (all four
    // cases: schedule `present|present|present` row).
    let mut present = base;
    mutate::retarget_template(&mut present, tid);
    inject_obs_state_protocol(&mut present);

    let rows = vec![
        (
            "data absent → reject (RM/schema OBSERVATION.data existence.lower)".to_owned(),
            data_absent,
            Expected::Rejected,
        ),
        (
            format!(
                "data present, state/protocol absent → {}",
                if boundary_accept {
                    "accepted"
                } else {
                    "rejected (existence.lower)"
                }
            ),
            boundary,
            if boundary_accept {
                Expected::Accepted
            } else {
                Expected::Rejected
            },
        ),
        (
            "data present, state present, protocol present → accepted".to_owned(),
            present,
            Expected::Accepted,
        ),
    ];
    let report = drive::drive_authored(ctx, &xml, rows).await?;
    Ok(report.of_schedule_rows(8))
}

macro_rules! obs {
    ($fn:ident, $tid:literal, $state:expr, $protocol:expr) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move { drive_obs(ctx, $tid, $state, $protocol).await })
        }
    };
}
obs!(
    obs_opt_opt,
    "cnf_cont_obs_state_opt_protocol_opt",
    false,
    false
);
obs!(
    obs_opt_mand,
    "cnf_cont_obs_state_opt_protocol_mand",
    false,
    true
);
obs!(
    obs_mand_opt,
    "cnf_cont_obs_state_mand_protocol_opt",
    true,
    false
);
obs!(
    obs_mand_mand,
    "cnf_cont_obs_state_mand_protocol_mand",
    true,
    true
);

// ── EVENT state existence (register 12 G-1) ───────────────────────────────────

/// Drive one EVENT state-existence case: author `EVENT.state` existence on the
/// persistent OPT, then commit the `data`-absent reject (RM/schema), the
/// `state`-absent boundary row (the distinguishing row), and a `state`-present
/// accept row (injected `ITEM_STRUCTURE`). Declared bound: the 4-row table.
async fn drive_event_state(
    ctx: &RunContext<'_>,
    tid: &'static str,
    state_mand: bool,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(PERSIST_OPT_FILE)?;
    author::set_template_id(&mut opt, tid);
    if state_mand {
        author::constrain_nested_single_mandatory(&mut opt, "EVENT", "state");
    }
    let xml = author::to_xml(&opt)?;
    let base = persist_base()?;

    let mut data_absent = base.clone();
    mutate::retarget_template(&mut data_absent, tid);
    if let Some(e) = mutate::first_node_mut(&mut data_absent, "POINT_EVENT") {
        mutate::remove_field(e, "data");
    }

    let mut boundary = base.clone();
    mutate::retarget_template(&mut boundary, tid);

    let mut present = base;
    mutate::retarget_template(&mut present, tid);
    inject_event_state(&mut present);

    let rows = vec![
        (
            "data absent → reject (RM/schema EVENT.data existence.lower)".to_owned(),
            data_absent,
            Expected::Rejected,
        ),
        (
            format!(
                "data present, state absent → {}",
                if state_mand {
                    "rejected (EVENT.state existence.lower)"
                } else {
                    "accepted"
                }
            ),
            boundary,
            if state_mand {
                Expected::Rejected
            } else {
                Expected::Accepted
            },
        ),
        (
            "data present, state present → accepted".to_owned(),
            present,
            Expected::Accepted,
        ),
    ];
    let report = drive::drive_authored(ctx, &xml, rows).await?;
    Ok(report.of_schedule_rows(4))
}

fn event_state_opt<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move { drive_event_state(ctx, "cnf_cont_event_state_opt", false).await })
}
fn event_state_mand<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move { drive_event_state(ctx, "cnf_cont_event_state_mand", true).await })
}

// ── EVENT type narrowing ──────────────────────────────────────────────────────

/// Fabricate a valid `INTERVAL_EVENT` in place of the base `POINT_EVENT`
/// (register 12 G-8): promote its `_type` and inject the mandatory `width`
/// (`DV_DURATION` PT1H) + `math_function` (`DV_CODED_TEXT` `openehr::146|mean`|;
/// RM 1.2.0 `data_structures` §`INTERVAL_EVENT`).
fn build_interval_event(comp: &mut Value) {
    if let Some(e) = mutate::first_node_mut(comp, "POINT_EVENT") {
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
}

/// EVENT `type_any`: the persistent base's `HISTORY.events` slot constrains only
/// the abstract `EVENT`, so both a `POINT_EVENT` (the base) and an
/// `INTERVAL_EVENT` (fabricated) are accepted (master16 §EVENT `type_any`: both
/// subtypes accepted).
fn event_type_any<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let base = persist_base()?;
        let report = drive::drive_constraint_base(
            ctx,
            "template.valid",
            PERSIST_OPT_FILE,
            base,
            "POINT_EVENT accepted in an open EVENT slot",
            vec![(
                "INTERVAL_EVENT accepted in an open EVENT slot".to_owned(),
                Box::new(|c: &mut Value| build_interval_event(c)),
                Expected::Accepted,
            )],
        )
        .await?;
        Ok(report.of_schedule_rows(2))
    })
}

/// EVENT `type_point_event`: author `HISTORY.events` narrowed to `POINT_EVENT`;
/// the base `POINT_EVENT` accepted, a copy whose event `_type` is swapped to the
/// sibling `INTERVAL_EVENT` rejected ("Class not allowed").
///
/// Boundary: the reject copy swaps only `_type` (no `width`/`math_function`), so
/// it is also RM-incomplete — a spec-correct SUT rejects it, but the schedule's
/// exact "Class not allowed" reason string is not machine-assertable here
/// (register 12 G-3); only the (edition-laddered) reject verdict is asserted.
fn event_type_point<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let tid = "cnf_cont_event_point_event";
        let mut opt = author::parse_base(PERSIST_OPT_FILE)?;
        author::set_template_id(&mut opt, tid);
        if !author::narrow_nested_child_type(&mut opt, "HISTORY", "events", "POINT_EVENT") {
            return Err(CaseError::Assertion(
                "base OPT has no HISTORY.events child object to narrow".to_owned(),
            ));
        }
        let xml = author::to_xml(&opt)?;
        let base = persist_base()?;

        let mut accepted = base.clone();
        mutate::retarget_template(&mut accepted, tid);
        let mut rejected = base;
        mutate::retarget_template(&mut rejected, tid);
        if let Some(e) = mutate::first_node_mut(&mut rejected, "POINT_EVENT") {
            mutate::set_field(e, "_type", json!("INTERVAL_EVENT"));
        }

        let report = drive::drive_authored(
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
        .await?;
        Ok(report.of_schedule_rows(2))
    })
}

/// EVENT `type_interval_event`: author `HISTORY.events` narrowed to
/// `INTERVAL_EVENT`; a valid `INTERVAL_EVENT` (the base `POINT_EVENT` + the
/// mandatory `width`/`math_function`, [`build_interval_event`]) accepted, the
/// base `POINT_EVENT` rejected ("Class not allowed").
fn event_type_interval<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let tid = "cnf_cont_event_interval_event";
        let mut opt = author::parse_base(PERSIST_OPT_FILE)?;
        author::set_template_id(&mut opt, tid);
        if !author::narrow_nested_child_type(&mut opt, "HISTORY", "events", "INTERVAL_EVENT") {
            return Err(CaseError::Assertion(
                "base OPT has no HISTORY.events child object to narrow".to_owned(),
            ));
        }
        let xml = author::to_xml(&opt)?;
        let base = persist_base()?;

        let mut accepted = base.clone();
        mutate::retarget_template(&mut accepted, tid);
        build_interval_event(&mut accepted);
        let mut rejected = base;
        mutate::retarget_template(&mut rejected, tid);

        let report = drive::drive_authored(
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
        .await?;
        Ok(report.of_schedule_rows(2))
    })
}

// ── HISTORY events cardinality × summary existence ────────────────────────────

/// Drive one CONT-HIST case: author a persistent OPT tightening `HISTORY.events`
/// cardinality (+ a mandatory `HISTORY.summary` for the `summary_ex_mand`
/// cases), then commit {0,1,3}-event compositions (summary absent) per the
/// master16 HISTORY tables.
///
/// RM `HISTORY.Events_valid` (≥1 event OR a summary) makes the 0-events +
/// absent-summary row **reject** regardless of the archetype cardinality — this
/// overrides the schedule's `CONT-HIST-events_card_any-summary_ex_opt` "no
/// events, absent summary → accepted" row (register 12 G-7: the RM invariant is
/// spec-authoritative over the printed table).
async fn drive_hist(
    ctx: &RunContext<'_>,
    tid: &'static str,
    card: Card,
    summary_mand: bool,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(PERSIST_OPT_FILE)?;
    author::set_template_id(&mut opt, tid);
    if !author::constrain_nested_multiple(&mut opt, "HISTORY", "events", &card.interval()) {
        return Err(CaseError::Assertion(
            "base OPT has no HISTORY.events multiple attribute to constrain".to_owned(),
        ));
    }
    if summary_mand {
        author::constrain_nested_single_mandatory(&mut opt, "HISTORY", "summary");
    }
    let xml = author::to_xml(&opt)?;
    let base = persist_base()?;

    let mut rows: Vec<(String, Value, Expected)> = Vec::new();
    for &count in &[0usize, 1, 3] {
        let mut c = base.clone();
        mutate::retarget_template(&mut c, tid);
        if let Some(h) = mutate::first_node_mut(&mut c, "HISTORY") {
            mutate::set_array_count(h, "events", count);
            mutate::remove_field(h, "summary");
        }
        // Events_valid override: summary is absent on every row, so 0
        // events is rejected by the RM invariant even where the archetype
        // cardinality would permit it.
        let accepted = count >= 1 && events_ok(card, count) && !summary_mand;
        rows.push((
            format!(
                "{count} event(s), summary absent → {}",
                if accepted { "accepted" } else { "rejected" }
            ),
            c,
            if accepted {
                Expected::Accepted
            } else {
                Expected::Rejected
            },
        ));
    }
    let report = drive::drive_authored(ctx, &xml, rows).await?;
    // Schedule table = 6 rows (3 events × 2 summary); the summary-present half is
    // the declared coverage bound (register 12 G-5).
    Ok(report.of_schedule_rows(6))
}

macro_rules! hist {
    ($fn:ident, $tid:literal, $card:expr, $summary_mand:expr) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move { drive_hist(ctx, $tid, $card, $summary_mand).await })
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

// ── ITEM_STRUCTURE type narrowing ─────────────────────────────────────────────

/// Drive an `ITEM_STRUCTURE` type-narrowing case: commit the vendored
/// `clinical_content_validation` composition (accepted), then a copy whose
/// `pointer` node's `_type` is swapped to `wrong_type` — a sibling subtype the
/// OPT slot forbids (rejected, "Class not allowed"). Declared bound: the 4-row
/// table (one sibling driven per case; the swap leaves the narrowed subtype's
/// items in place, so a bare `_type` change to a sibling is both class-forbidden
/// and RM-shape-invalid — the reject verdict, not the exact reason, is asserted,
/// register 12 G-3).
fn drive_item_str<'a>(
    ctx: &'a RunContext<'a>,
    narrowed: &'static str,
    pointer: &'static str,
    wrong_type: &'static str,
) -> CaseFuture<'a> {
    Box::pin(async move {
        let report = drive::drive_constraint(
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
        .await?;
        Ok(report.of_schedule_rows(4))
    })
}

fn item_str_tree<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_TREE", "/content/2/data", "ITEM_LIST")
}
fn item_str_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_LIST", "/content/3/data", "ITEM_TREE")
}
fn item_str_table<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_TABLE", "/content/4/data", "ITEM_TREE")
}
fn item_str_single<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_item_str(ctx, "ITEM_SINGLE", "/content/1/data", "ITEM_TREE")
}

/// `ITEM_STRUCTURE` `type_any`: author `clinical_content_validation` with the
/// `ITEM_TREE`-narrowed EVALUATION `data` slot **re-opened** to the abstract
/// `ITEM_STRUCTURE`, then commit the vendored composition (slot = `ITEM_TREE`)
/// and a copy with that slot rebuilt as `ITEM_LIST` — both accepted (any
/// subtype). Declared bound: the 4-row table (`ITEM_TABLE/ITEM_SINGLE` need
/// further fabricated instances).
fn item_str_any<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let tid = "cnf_cont_item_str_any";
        let mut opt = author::parse_base("validation/clinical_content_validation.opt")?;
        author::set_template_id(&mut opt, tid);
        // Re-open the ITEM_TREE-narrowed EVALUATION data slot to abstract
        // ITEM_STRUCTURE. Pinned to EVALUATION/data (retype_attr_child): the
        // template carries other ITEM_TREE leaves (an INSTRUCTION description
        // first in document order); a blanket first-match retype re-opened the
        // wrong one, leaving this slot ITEM_TREE-narrowed (which a spec-correct
        // validator must then reject).
        author::retype_attr_child(
            &mut opt,
            "EVALUATION",
            "data",
            "ITEM_TREE",
            author::open_complex("ITEM_STRUCTURE"),
        );
        let xml = author::to_xml(&opt)?;
        let base = crate::testdata::fixtures::read_from(
            "composition.canonical-json",
            "clinical_content_validation__full.json",
        )
        .map_err(|e| CaseError::Codec(e.to_string()))?;
        let base: Value =
            serde_json::from_str(&base).map_err(|e| CaseError::Codec(e.to_string()))?;

        let mut tree = base.clone();
        mutate::retarget_template(&mut tree, tid);
        let mut list = base;
        mutate::retarget_template(&mut list, tid);
        // RM 1.2.0 data_structures §ITEM_LIST.items is `List<ELEMENT>`, so the
        // "any subtype accepted" positive must be RM-valid: rebuild the
        // ITEM_LIST from the tree's ELEMENT leaves rather than a bare `_type`
        // swap (which would leave CLUSTER items an ITEM_LIST forbids).
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

        let report = drive::drive_authored(
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
        .await?;
        Ok(report.of_schedule_rows(4))
    })
}
