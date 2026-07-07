//! master17.1–17.7 — `DATA_VALUE` data-validation truth tables
//! (`master17.N-content_tc_data_types-*.adoc`).
//!
//! Each `validate_open` case tests a value type's **RM/Schema mandatory** fields
//! (the "RM/Schema mandatory" rows): a mandatory RM attribute (`value`,
//! `magnitude`, `defining_code`, …) removed → the server must reject; the
//! unmutated base → accepted. These run through [`drive::data_type_mandatory`],
//! which **drives** the case when a committable base composition carries a leaf of
//! the type and otherwise returns `Skipped` (no fixture in the corpus for that
//! type) — never a fabricated pass.
//!
//! **Archetype-constraint cases driven via the full constraint-carrying corpus**
//! ([`drive::drive_constraint`], design §4.5): where a vendored OPT constrains a
//! leaf and ships a committable canonical-JSON composition, the case uploads the
//! OPT, commits the valid composition (accepted), then commits a copy with the
//! constrained leaf violated (rejected). Driven this way:
//! `CONT-DV_QUANTITY-validate_property_units` (units off the `{mg,kg}` list),
//! `CONT-DV_ORDINAL-validate_constraint` (symbol off the ordinal list) and
//! `CONT-DV_CODED_TEXT-validate_local_codes` (code off the `local` `code_list`) via
//! the `all_types` templates — all three our validator enforces (they PASS);
//! `CONT-DV_DATE_TIME-validate_constraint` (partial value vs a
//! `yyyy-mm-ddTHH:MM:SS` `C_DATE_TIME`) also drives but is an **open finding**
//! (the SUT accepts the partial value the truth table rejects).
//!
//! The remaining non-`validate_open` cases stay `Skipped`
//! ([`drive::skip_archetype`]): no vendored OPT both constrains the relevant leaf
//! *and* ships a committable canonical-JSON composition — `DV_COUNT` range/list &
//! `DV_QUANTITY` magnitude-range live only in `ehrn_vital_signs` (FLAT-only
//! instance); `DV_PROPORTION` in `proportion.opt`/`ehrn` (no canonical instance);
//! `DV_SCALE`, `DV_DATE/DV_TIME` constraints, `DV_BOOLEAN` & `DV_IDENTIFIER` patterns, and
//! `DV_MULTIMEDIA` media-type lists have no constrained committable canonical leaf.

use serde_json::{Value, json};

use crate::case::Chapter;
use crate::harness::{CaseFuture, CaseRun, RunContext};
use crate::registry::CaseEntry;

use super::drive::{self, Base, Constraint, Expected, meta};
use super::mutate;

/// The implemented master17.x case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let c = Chapter::Master17_1;
    let mut all = vec![
        // ── 17.1 basic ────────────────────────────────────────────────────────
        open("CONT-DV_BOOLEAN-anything_allowed", c, open_dv_boolean),
        skip("CONT-DV_BOOLEAN-only_true_allowed", c),
        skip("CONT-DV_BOOLEAN-only_false_allowed", c),
        skip("CONT-DV_IDENTIFIER-validate_all_pattern", c),
        skip("CONT-DV_IDENTIFIER-validate_all_list", c),
    ];

    // ── 17.2 text ──────────────────────────────────────────────────────────────
    let c = Chapter::Master17_2;
    all.push(open("CONT-DV_TEXT-validate_open", c, open_dv_text));
    all.push(skip("CONT-DV_TEXT-validate_list", c));
    all.push(open(
        "CONT-DV_CODED_TEXT-validate_open",
        c,
        open_dv_coded_text,
    ));
    all.push(open(
        "CONT-DV_CODED_TEXT-validate_local_codes",
        c,
        run_dv_coded_local,
    ));
    all.push(skip("CONT-DV_CODED_TEXT-validate_ext_term", c));

    // ── 17.3 quantity ──────────────────────────────────────────────────────────
    let c = Chapter::Master17_3;
    all.push(open("CONT-DV_ORDINAL-validate_open", c, open_dv_ordinal));
    all.push(open(
        "CONT-DV_ORDINAL-validate_constraint",
        c,
        run_dv_ordinal_constraint,
    ));
    all.push(open("CONT-DV_SCALE-validate_open", c, open_dv_scale));
    all.push(skip("CONT-DV_SCALE-validate_constraint", c));
    all.push(open("CONT-DV_COUNT-validate_open", c, open_dv_count));
    all.push(skip("CONT-DV_COUNT-validate_range", c));
    all.push(skip("CONT-DV_COUNT-validate_list", c));
    all.push(open("CONT-DV_QUANTITY-validate_open", c, open_dv_quantity));
    all.push(skip("CONT-DV_QUANTITY-validate_property", c));
    all.push(open(
        "CONT-DV_QUANTITY-validate_property_units",
        c,
        run_dv_quantity_units,
    ));
    all.push(skip("CONT-DV_QUANTITY-validate_property_units_mag", c));
    all.push(open(
        "CONT-DV_PROPORTION-validate_open",
        c,
        open_dv_proportion,
    ));
    all.push(skip("CONT-DV_PROPORTION-validate_ratio", c));
    all.push(skip("CONT-DV_PROPORTION-validate_unitary", c));
    all.push(skip("CONT-DV_PROPORTION-validate_percent", c));
    all.push(skip("CONT-DV_PROPORTION-validate_fraction", c));
    all.push(skip("CONT-DV_PROPORTION-validate_integer_fraction", c));
    all.push(skip("CONT-DV_PROPORTION-validate_any_fraction", c));
    all.push(skip("CONT-DV_PROPORTION-validate_ratio_range", c));
    // DV_INTERVAL<T> cases — interval bound constraints, no committable leaf.
    for id in [
        "CONT-DV_INTERVAL_DV_COUNT-validate_open",
        "CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper",
        "CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper_list",
        "CONT-DV_INTERVAL_DV_QUANTITY-validate_open",
        "CONT-DV_INTERVAL_DV_QUANTITY-validate_upper_lower",
        "CONT-DV_INTERVAL_DV_DATE_TIME-validate_open",
        "CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint",
        "CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_range",
        "CONT-DV_INTERVAL_DV_DATE-validate_open",
        "CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_constraint",
        "CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_range",
        "CONT-DV_INTERVAL_DV_TIME-validate_open",
        "CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_constraint",
        "CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_range",
        "CONT-DV_INTERVAL_DV_DURATION-validate_open",
        "CONT-DV_INTERVAL_DV_DURATION-validate_constraint",
        "CONT-DV_INTERVAL_DV_DURATION-validate_range",
        "CONT-DV_INTERVAL_DV_ORDINAL-validate_open",
        "CONT-DV_INTERVAL_DV_ORDINAL-validate_constraint",
        "CONT-DV_INTERVAL_DV_SCALE-validate_open",
        "CONT-DV_INTERVAL_DV_SCALE-validate_constraint",
        "CONT-DV_INTERVAL_DV_PROPORTION-validate_open",
        "CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio",
        "CONT-DV_INTERVAL_DV_PROPORTION-validate_unitary",
        "CONT-DV_INTERVAL_DV_PROPORTION-validate_percentage",
        "CONT-DV_INTERVAL_DV_PROPORTION-validate_fraction",
        "CONT-DV_INTERVAL_DV_PROPORTION-validate_integer_fraction",
        "CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio_range",
    ] {
        all.push(skip(id, c));
    }

    // ── 17.4 date_time ─────────────────────────────────────────────────────────
    let c = Chapter::Master17_4;
    all.push(open("CONT-DV_DURATION-validate_open", c, open_dv_duration));
    // No committable canonical-JSON composition carries a `C_DURATION`-constrained
    // DV_DURATION: all_types `items[at0018]` DV_DURATION is unconstrained; the
    // `obs_inst`/`ehrn_vital_signs` OPTs do constrain it (pattern `PDTH` / a
    // `PT24H` range) but ship no committable canonical composition — `obs_inst`'s
    // only instance is a CONTRIBUTION whose COMPOSITION omits `archetype_details`
    // on its content ENTRYs (fails the `Is_archetypeRoot` RM invariant as a bare
    // commit), and `ehrn_vital_signs` ships only a FLAT instance. Not drivable.
    all.push(skip("CONT-DV_DURATION-validate_fields", c));
    all.push(skip("CONT-DV_DURATION-validate_range", c));
    all.push(skip("CONT-DV_DURATION-validate_fields_range", c));
    all.push(open("CONT-DV_TIME-validate_open", c, open_dv_time));
    all.push(skip("CONT-DV_TIME-validate_constraint", c));
    all.push(skip("CONT-DV_TIME-validate_range", c));
    all.push(open("CONT-DV_DATE-validate_open", c, open_dv_date));
    all.push(skip("CONT-DV_DATE-validate_constraint", c));
    all.push(skip("CONT-DV_DATE-validate_range", c));
    all.push(open(
        "CONT-DV_DATE_TIME-validate_open",
        c,
        open_dv_date_time,
    ));
    all.push(open(
        "CONT-DV_DATE_TIME-validate_constraint",
        c,
        run_dv_date_time_constraint,
    ));
    all.push(skip("CONT-DV_DATE_TIME-validate_range", c));

    // ── 17.6 encapsulated ──────────────────────────────────────────────────────
    let c = Chapter::Master17_6;
    all.push(open("CONT-DV_PARSABLE-validate_open", c, open_dv_parsable));
    all.push(skip("CONT-DV_PARSABLE-validate_value_formalism", c));
    all.push(open(
        "CONT-DV_MULTIMEDIA-validate_open",
        c,
        open_dv_multimedia,
    ));
    all.push(skip("CONT-DV_MULTIMEDIA-validate_media_type", c));

    // ── 17.7 uri ───────────────────────────────────────────────────────────────
    let c = Chapter::Master17_7;
    all.push(open("CONT-DV_URI-validate_open", c, open_dv_uri));
    all.push(skip("CONT-DV_URI-validate_pattern", c));
    all.push(skip("CONT-DV_URI-validate_list", c));
    all.push(open("CONT-DV_EHR_URI-validate_open", c, open_dv_ehr_uri));
    all.push(skip("CONT-DV_EHR_URI-validate_pattern", c));
    all.push(skip("CONT-DV_EHR_URI-validate_list", c));

    all
}

/// A `validate_open` case: driven via [`drive::data_type_mandatory`] (auto-skips
/// when the corpus has no committable leaf of the type).
fn open(id: &'static str, chapter: Chapter, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: meta(id, chapter, id),
        run,
    }
}

/// An archetype-constraint case: transcribed + cited, skipped at run time.
fn skip(id: &'static str, chapter: Chapter) -> CaseEntry {
    CaseEntry {
        meta: meta(id, chapter, id),
        run: run_skip,
    }
}

/// The shared archetype-constraint skip (the specific constraint — `C_STRING` /
/// `C_INTEGER` / `C_CODE_PHRASE` / range / list / pattern / interval bounds — is
/// identified by the case id / `schedule_ref`).
fn run_skip<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::skip_archetype(
            "DATA_VALUE archetype constraint (C_STRING / C_INTEGER / C_CODE_PHRASE / \
             range / list / pattern / DV_INTERVAL bounds)",
        )
    })
}

/// Generate a `validate_open` run fn: drive the value type's mandatory field on a
/// base composition, self-skipping when the corpus carries no such leaf.
macro_rules! dt_open {
    ($fn:ident, $base:expr, $ty:literal, $field:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move { drive::data_type_mandatory(ctx, $base, $ty, $field).await })
        }
    };
}

// Drivable with a vendored committable base leaf:
dt_open!(open_dv_text, Base::PersistentMinimal, "DV_TEXT", "value");
dt_open!(open_dv_count, Base::EventNested, "DV_COUNT", "magnitude");
dt_open!(
    open_dv_date_time,
    Base::PersistentMinimal,
    "DV_DATE_TIME",
    "value"
);
// Self-skipping (no committable leaf of the type in the corpus) but expressed as
// the same mandatory-field driver so a future fixture makes them drive without a
// code change:
dt_open!(open_dv_boolean, Base::EventNested, "DV_BOOLEAN", "value");
dt_open!(
    open_dv_coded_text,
    Base::EventNested,
    "DV_CODED_TEXT",
    "defining_code"
);
dt_open!(open_dv_ordinal, Base::EventNested, "DV_ORDINAL", "value");
dt_open!(open_dv_scale, Base::EventNested, "DV_SCALE", "value");
dt_open!(
    open_dv_quantity,
    Base::EventNested,
    "DV_QUANTITY",
    "magnitude"
);
dt_open!(
    open_dv_proportion,
    Base::EventNested,
    "DV_PROPORTION",
    "numerator"
);
dt_open!(open_dv_date, Base::EventNested, "DV_DATE", "value");
dt_open!(open_dv_time, Base::EventNested, "DV_TIME", "value");
dt_open!(open_dv_duration, Base::EventNested, "DV_DURATION", "value");
dt_open!(open_dv_parsable, Base::EventNested, "DV_PARSABLE", "value");
dt_open!(
    open_dv_multimedia,
    Base::EventNested,
    "DV_MULTIMEDIA",
    "media_type"
);
dt_open!(open_dv_uri, Base::EventNested, "DV_URI", "value");
dt_open!(open_dv_ehr_uri, Base::EventNested, "DV_EHR_URI", "value");

// ── constraint-OPT driven data-type cases (the full constraint corpus, §4.5) ──
//
// Each uploads a constraint-carrying vendored OPT, commits its vendored valid
// composition (accepted), then a copy with the constrained leaf violated
// (rejected) — the master17.x truth-table oracle, cited per case. A server that
// accepts the violation is a finding (design §4.5), never a masked skip.

/// `Test_all_types.opt` + its bare canonical composition
/// (`query/data_load/compositions/all_types.composition.json`, template
/// `test_all_types.en.v1`). OBSERVATION `data/events[0]/data/items` carry one
/// leaf per data type at fixed indices.
const ALL_TYPES: Constraint = Constraint {
    opt: "all_types/Test_all_types.opt",
    comp: "query/data_load/compositions/all_types.composition.json",
};

/// `Test_all_types_v2.opt` — identical structure, but OBSERVATION `items[at0005]`
/// (index 1) adds a `local` `code_list` `{at0023, at0024}` on its `DV_CODED_TEXT`.
const ALL_TYPES_V2: Constraint = Constraint {
    opt: "all_types/Test_all_types_v2.opt",
    comp: "query/data_load/compositions/all_types_v2.composition.json",
};

/// master17.3 CONT-DV_QUANTITY-validate_property_units. `items[3]` (`at0007`) is a
/// `C_DV_QUANTITY`, property `openehr::124`, units list `{mg, kg}` (no magnitude
/// range). The vendored value is `mg` (accepted); `L` is off the list → the
/// truth-table `C_DV_QUANTITY.list` "units not allowed" row (rejected).
fn run_dv_quantity_units<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::drive_constraint(
            ctx,
            &ALL_TYPES,
            "DV_QUANTITY units 'mg' in [mg,kg] (accepted)",
            vec![(
                "DV_QUANTITY units 'L' not in [mg,kg] (C_DV_QUANTITY.list)".to_owned(),
                Box::new(|c: &mut Value| {
                    mutate::set_pointer(
                        c,
                        "/content/0/data/events/0/data/items/3/value/units",
                        json!("L"),
                    );
                }),
                Expected::Rejected,
            )],
        )
        .await
    })
}

/// master17.3 CONT-DV_ORDINAL-validate_constraint. `items[9]` (`at0013`) is a
/// `C_DV_ORDINAL` with list `{0→local::at0014, 1→local::at0015, 2→local::at0016}`.
/// The vendored symbol is `local::at0014` (accepted); `local::at0666` is off the
/// list → the truth-table "no matching symbol" row (rejected).
fn run_dv_ordinal_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::drive_constraint(
            ctx,
            &ALL_TYPES,
            "DV_ORDINAL symbol local::at0014 in list (accepted)",
            vec![(
                "DV_ORDINAL symbol local::at0666 not in list (C_DV_ORDINAL.list: no matching symbol)"
                    .to_owned(),
                Box::new(|c: &mut Value| {
                    mutate::set_pointer(
                        c,
                        "/content/0/data/events/0/data/items/9/value/symbol/defining_code/code_string",
                        json!("at0666"),
                    );
                }),
                Expected::Rejected,
            )],
        )
        .await
    })
}

/// master17.2 CONT-DV_CODED_TEXT-validate_local_codes. In `Test_all_types_v2`,
/// OBSERVATION `items[1]` (`at0005`) `DV_CODED_TEXT` is constrained by a
/// `C_CODE_PHRASE`, terminology `local`, `code_list` `{at0023, at0024}`. The
/// vendored code is `local::at0023` (accepted); `local::at0025` is off the list
/// → the truth-table "code not in list" rejection.
fn run_dv_coded_local<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::drive_constraint(
            ctx,
            &ALL_TYPES_V2,
            "DV_CODED_TEXT local::at0023 in code_list (accepted)",
            vec![(
                "DV_CODED_TEXT local::at0025 not in code_list {at0023,at0024} (C_CODE_PHRASE)"
                    .to_owned(),
                Box::new(|c: &mut Value| {
                    mutate::set_pointer(
                        c,
                        "/content/0/data/events/0/data/items/1/value/defining_code/code_string",
                        json!("at0025"),
                    );
                }),
                Expected::Rejected,
            )],
        )
        .await
    })
}

/// master17.4 CONT-DV_DATE_TIME-validate_constraint. `items[6]` (`at0010`) is a
/// `DV_DATE_TIME` whose `value` is a `C_DATE_TIME` with pattern
/// `yyyy-mm-ddTHH:MM:SS` (all fields mandatory). The vendored value is a full
/// timestamp (accepted); a year-only `2021` violates the mandatory
/// month/day/time fields → the truth-table first row (rejected).
fn run_dv_date_time_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::drive_constraint(
            ctx,
            &ALL_TYPES,
            "DV_DATE_TIME full timestamp matches yyyy-mm-ddTHH:MM:SS (accepted)",
            vec![(
                "DV_DATE_TIME '2021' missing mandatory month/day/time (C_DATE_TIME validity)"
                    .to_owned(),
                Box::new(|c: &mut Value| {
                    mutate::set_pointer(
                        c,
                        "/content/0/data/events/0/data/items/6/value/value",
                        json!("2021"),
                    );
                }),
                Expected::Rejected,
            )],
        )
        .await
    })
}
