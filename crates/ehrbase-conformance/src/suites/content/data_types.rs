//! master17.1–17.7 — `DATA_VALUE` data-validation truth tables
//! (`master17.N-content_tc_data_types-*.adoc`).
//!
//! Each `validate_open` case tests a value type's **RM/Schema mandatory** fields
//! (the "RM/Schema mandatory" rows): a mandatory RM attribute (`value`,
//! `magnitude`, `defining_code`, …) removed → the server must reject; the
//! unmutated base → accepted. These run through [`drive::data_type_mandatory`],
//! which **drives** the case when a committable base composition carries a leaf of
//! the type and otherwise returns `Skipped` (no fixture in the corpus for that
//! type) — never a fabricated pass. With the two vendored committable bases
//! (`nested` / `persistent_minimal`) the drivable leaves are `DV_TEXT`, `DV_COUNT` and
//! `DV_DATE_TIME`; the other `validate_open` cases self-skip.
//!
//! Every non-`validate_open` case (`validate_range` / `validate_list` /
//! `validate_pattern` / `validate_constraint` / `validate_property*` /
//! `validate_ratio*` / the `DV_INTERVAL_*` bound constraints / `C_BOOLEAN` /
//! `C_STRING` / `C_CODE_PHRASE`) is an **archetype constraint** needing a
//! constraint-expressing OPT the corpus does not contain (design §2.2a):
//! transcribed + cited via `schedule_ref`, returning `Skipped`
//! ([`drive::skip_archetype`]).

use crate::case::Chapter;
use crate::harness::{CaseFuture, CaseRun, RunContext};
use crate::registry::CaseEntry;

use super::drive::{self, Base, meta};

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
    all.push(skip("CONT-DV_CODED_TEXT-validate_local_codes", c));
    all.push(skip("CONT-DV_CODED_TEXT-validate_ext_term", c));

    // ── 17.3 quantity ──────────────────────────────────────────────────────────
    let c = Chapter::Master17_3;
    all.push(open("CONT-DV_ORDINAL-validate_open", c, open_dv_ordinal));
    all.push(skip("CONT-DV_ORDINAL-validate_constraint", c));
    all.push(open("CONT-DV_SCALE-validate_open", c, open_dv_scale));
    all.push(skip("CONT-DV_SCALE-validate_constraint", c));
    all.push(open("CONT-DV_COUNT-validate_open", c, open_dv_count));
    all.push(skip("CONT-DV_COUNT-validate_range", c));
    all.push(skip("CONT-DV_COUNT-validate_list", c));
    all.push(open("CONT-DV_QUANTITY-validate_open", c, open_dv_quantity));
    all.push(skip("CONT-DV_QUANTITY-validate_property", c));
    all.push(skip("CONT-DV_QUANTITY-validate_property_units", c));
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
    all.push(skip("CONT-DV_DATE_TIME-validate_constraint", c));
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
