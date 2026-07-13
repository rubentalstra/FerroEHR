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
//! **FLAT-backed & instance-backed constraint cases (design §4.5, path *b*).**
//! Two further cases have a constraining OPT that parses but whose only
//! committable instance is not a plain canonical-JSON composition:
//! - `CONT-DV_QUANTITY-validate_property_units_mag` — the `time_series` OPT
//!   constrains a `DV_QUANTITY` magnitude range `[0,∞)` (units `{mm3}`) and ships
//!   **only a FLAT instance**. The harness converts that FLAT to canonical
//!   in-process via [`fixtures::flat_to_canonical`] (the same `from_flat` path
//!   the SUT's FLAT endpoint uses), commits `702.9 mm3` (accepted), then a
//!   below-range magnitude and an off-list unit (both rejected).
//! - `CONT-DV_PROPORTION-validate_any_fraction` — `minimal_action_2.opt`
//!   constrains `DV_PROPORTION` `type` to `C_INTEGER.list {3,4}` and ships a bare
//!   canonical composition (`type=3`, accepted); `type=0` is off the list
//!   (rejected).
//!
//! **All other value-constraint cases are driven via authored OPTs**
//! ([`super::author`]): where no vendored OPT constrains the leaf, the case tightens
//! the constraint into the `all_types` OPT (`C_STRING/C_INTEGER/C_REAL/C_BOOLEAN`/
//! `C_DATE/TIME/DATE_TIME/DURATION/C_DV_QUANTITY.property/C_CODE_PHRASE`) or, for a
//! type the base composition does not carry (`DV_URI`/`DV_EHR_URI`/`DV_SCALE`/
//! `DV_INTERVAL<T>`), **slot-retypes** a scratch leaf to that type
//! ([`author::retype_leaf`]) and commits a whole-value instance. Every master17
//! case is a real endpoint test — **none are skipped**. Constraints the validator
//! enforces PASS; the rest are recorded findings (temporal ranges, integer/real
//! lists, `DV_INTERVAL` bounds, external terminology), never masked as skips.

use openehr_its::opt14::{CPrimitive, OperationalTemplate};
use serde_json::{Value, json};

use crate::assert;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;
use crate::suites::support;

use super::author;
use super::drive::{self, Base, Constraint, Expected, meta};
use super::mutate;

/// The `all_types` base OPT + composition (a leaf of nearly every `DV_*` type at
/// fixed `items` indices) — the authored-constraint base for master17 leaf cases.
const ALL_TYPES_OPT: &str = "all_types/Test_all_types.opt";

/// Drive a master17 leaf value-constraint case by **authoring** the constraint into
/// the `all_types` OPT (the vendored corpus ships no OPT constraining these leaves),
/// then committing the base composition (its vendored leaf value satisfies the
/// constraint → accepted) and a copy with the leaf value pushed out of the
/// constraint (rejected). The accept/reject is the SUT's genuine validation of a
/// real authored template (design §4.5), not a fabricated pass.
async fn drive_leaf(
    ctx: &RunContext<'_>,
    tid: &'static str,
    constrain: impl FnOnce(&mut OperationalTemplate) -> bool,
    accepted_label: &str,
    rejected_label: String,
    invalid_pointer: &str,
    invalid_value: Value,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(ALL_TYPES_OPT)?;
    author::set_template_id(&mut opt, tid);
    if !constrain(&mut opt) {
        return Err(CaseError::Assertion(format!(
            "authoring the leaf constraint for {tid} found no matching leaf in the all_types OPT"
        )));
    }
    let xml = author::to_xml(&opt)?;

    let base = drive::Base::AllTypes.load()?; // owned corrected copy (see testdata/fixtures/REGISTER.md)
    let mut accepted = base.clone();
    mutate::retarget_template(&mut accepted, tid);
    let mut rejected = base;
    mutate::retarget_template(&mut rejected, tid);
    if !mutate::set_pointer(&mut rejected, invalid_pointer, invalid_value) {
        return Err(CaseError::Assertion(format!(
            "invalid-value pointer {invalid_pointer} did not resolve in the all_types composition"
        )));
    }
    drive::drive_authored(
        ctx,
        &xml,
        vec![
            (accepted_label.to_owned(), accepted, Expected::Accepted),
            (rejected_label, rejected, Expected::Rejected),
        ],
    )
    .await
}

/// The canonical-JSON pointer to the value of the `items[idx]` leaf in the
/// `all_types` OBSERVATION (`content[0]/data/events[0]/data/items[idx]/value`).
fn leaf_ptr(idx: usize, suffix: &str) -> String {
    format!("/content/0/data/events/0/data/items/{idx}/value/{suffix}")
}

/// The implemented master17.x case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut all = vec![
        // ── 17.1 basic ────────────────────────────────────────────────────────
        open(
            "val/dv-boolean-anything-allowed",
            "Validate DV_BOOLEAN — anything allowed",
            "RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            open_dv_boolean,
        ),
        open(
            "val/dv-boolean-only-true-allowed",
            "Validate DV_BOOLEAN — only true allowed",
            "RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            run_dv_boolean_true,
        ),
        open(
            "val/dv-boolean-only-false-allowed",
            "Validate DV_BOOLEAN — only false allowed",
            "RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            run_dv_boolean_false,
        ),
        open(
            "val/dv-identifier-all-pattern",
            "Validate DV_IDENTIFIER — all pattern",
            "RM 1.2.0 data_types §DV_IDENTIFIER; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            run_dv_identifier_pattern,
        ),
        open(
            "val/dv-identifier-all-list",
            "Validate DV_IDENTIFIER — all list",
            "RM 1.2.0 data_types §DV_IDENTIFIER; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            run_dv_identifier_list,
        ),
    ];

    // ── 17.2 text ──────────────────────────────────────────────────────────────
    all.push(open("val/dv-text-open", "Validate DV_TEXT — open", "RM 1.2.0 data_types §DV_TEXT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_text));
    all.push(open("val/dv-text-list", "Validate DV_TEXT — list", "RM 1.2.0 data_types §DV_TEXT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_text_list));
    all.push(open(
        "val/dv-coded-text-open", "Validate DV_CODED_TEXT — open", "RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        open_dv_coded_text,
    ));
    all.push(open(
        "val/dv-coded-text-local-codes", "Validate DV_CODED_TEXT — local codes", "RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_coded_local,
    ));
    all.push(open(
        "val/dv-coded-text-ext-term", "Validate DV_CODED_TEXT — ext term", "RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_coded_ext_term,
    ));

    // ── 17.3 quantity ──────────────────────────────────────────────────────────
    all.push(open("val/dv-ordinal-open", "Validate DV_ORDINAL — open", "RM 1.2.0 data_types §DV_ORDINAL; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_ordinal));
    all.push(open(
        "val/dv-ordinal-constraint", "Validate DV_ORDINAL — constraint", "RM 1.2.0 data_types §DV_ORDINAL; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_ordinal_constraint,
    ));
    all.push(open("val/dv-scale-open", "Validate DV_SCALE — open", "RM 1.2.0 data_types §DV_SCALE; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_scale_open));
    all.push(open(
        "val/dv-scale-constraint", "Validate DV_SCALE — constraint", "RM 1.2.0 data_types §DV_SCALE; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_scale_constraint,
    ));
    all.push(open("val/dv-count-open", "Validate DV_COUNT — open", "RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_count));
    all.push(open("val/dv-count-range", "Validate DV_COUNT — range", "RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_count_range));
    all.push(open("val/dv-count-list", "Validate DV_COUNT — list", "RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_count_list));
    all.push(open("val/dv-quantity-open", "Validate DV_QUANTITY — open", "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_quantity));
    all.push(open(
        "val/dv-quantity-property", "Validate DV_QUANTITY — property", "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_quantity_property,
    ));
    all.push(open(
        "val/dv-quantity-property-units", "Validate DV_QUANTITY — property units", "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_quantity_units,
    ));
    all.push(open(
        "val/dv-quantity-property-units-mag", "Validate DV_QUANTITY — property units mag", "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_quantity_units_mag,
    ));
    all.push(open(
        "val/dv-proportion-open", "Validate DV_PROPORTION — open", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        open_dv_proportion,
    ));
    all.push(open(
        "val/dv-proportion-ratio", "Validate DV_PROPORTION — ratio", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_proportion_ratio,
    ));
    all.push(open(
        "val/dv-proportion-unitary", "Validate DV_PROPORTION — unitary", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_proportion_unitary,
    ));
    all.push(open(
        "val/dv-proportion-percent", "Validate DV_PROPORTION — percent", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_proportion_percent,
    ));
    all.push(open(
        "val/dv-proportion-fraction", "Validate DV_PROPORTION — fraction", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_proportion_fraction,
    ));
    all.push(open(
        "val/dv-proportion-integer-fraction", "Validate DV_PROPORTION — integer fraction", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_proportion_integer_fraction,
    ));
    all.push(open(
        "val/dv-proportion-any-fraction", "Validate DV_PROPORTION — any fraction", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_proportion_any_fraction,
    ));
    all.push(open(
        "val/dv-proportion-ratio-range", "Validate DV_PROPORTION — ratio range", "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_proportion_ratio_range,
    ));
    // DV_INTERVAL<T> cases — driven by slot-retyping a scratch leaf to an open
    // DV_INTERVAL and asserting the RM Interval invariant (drive_interval); the
    // per-variant bound constraints need DV_INTERVAL constraint support the
    // validator lacks, so most record as findings — driven, never skipped.
    let interval: &[(&str, &str, &str, CaseRun)] = &[
        (
            "val/dv-interval-dv-count-open",
            "Validate DV_INTERVAL<DV_COUNT> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivc_open,
        ),
        (
            "val/dv-interval-dv-count-lower-upper",
            "Validate DV_INTERVAL<DV_COUNT> — lower upper",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivc_lu,
        ),
        (
            "val/dv-interval-dv-count-lower-upper-list",
            "Validate DV_INTERVAL<DV_COUNT> — lower upper list",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivc_lul,
        ),
        (
            "val/dv-interval-dv-quantity-open",
            "Validate DV_INTERVAL<DV_QUANTITY> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_QUANTITY>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivq_open,
        ),
        (
            "val/dv-interval-dv-quantity-upper-lower",
            "Validate DV_INTERVAL<DV_QUANTITY> — upper lower",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_QUANTITY>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivq_ul,
        ),
        (
            "val/dv-interval-dv-date-time-open",
            "Validate DV_INTERVAL<DV_DATE_TIME> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivdt_open,
        ),
        (
            "val/dv-interval-dv-date-time-lower-upper-constraint",
            "Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivdt_luc,
        ),
        (
            "val/dv-interval-dv-date-time-lower-upper-range",
            "Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivdt_lur,
        ),
        (
            "val/dv-interval-dv-date-open",
            "Validate DV_INTERVAL<DV_DATE> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivd_open,
        ),
        (
            "val/dv-interval-dv-date-lower-upper-constraint",
            "Validate DV_INTERVAL<DV_DATE> — lower upper constraint",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivd_luc,
        ),
        (
            "val/dv-interval-dv-date-lower-upper-range",
            "Validate DV_INTERVAL<DV_DATE> — lower upper range",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivd_lur,
        ),
        (
            "val/dv-interval-dv-time-open",
            "Validate DV_INTERVAL<DV_TIME> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivt_open,
        ),
        (
            "val/dv-interval-dv-time-lower-upper-constraint",
            "Validate DV_INTERVAL<DV_TIME> — lower upper constraint",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivt_luc,
        ),
        (
            "val/dv-interval-dv-time-lower-upper-range",
            "Validate DV_INTERVAL<DV_TIME> — lower upper range",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivt_lur,
        ),
        (
            "val/dv-interval-dv-duration-open",
            "Validate DV_INTERVAL<DV_DURATION> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivdu_open,
        ),
        (
            "val/dv-interval-dv-duration-constraint",
            "Validate DV_INTERVAL<DV_DURATION> — constraint",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivdu_c,
        ),
        (
            "val/dv-interval-dv-duration-range",
            "Validate DV_INTERVAL<DV_DURATION> — range",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivdu_r,
        ),
        (
            "val/dv-interval-dv-ordinal-open",
            "Validate DV_INTERVAL<DV_ORDINAL> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_ORDINAL>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivo_open,
        ),
        (
            "val/dv-interval-dv-ordinal-constraint",
            "Validate DV_INTERVAL<DV_ORDINAL> — constraint",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_ORDINAL>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivo_c,
        ),
        (
            "val/dv-interval-dv-scale-open",
            "Validate DV_INTERVAL<DV_SCALE> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_SCALE>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivs_open,
        ),
        (
            "val/dv-interval-dv-scale-constraint",
            "Validate DV_INTERVAL<DV_SCALE> — constraint",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_SCALE>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivs_c,
        ),
        (
            "val/dv-interval-dv-proportion-open",
            "Validate DV_INTERVAL<DV_PROPORTION> — open",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivp_open,
        ),
        (
            "val/dv-interval-dv-proportion-ratio",
            "Validate DV_INTERVAL<DV_PROPORTION> — ratio",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivp_ratio,
        ),
        (
            "val/dv-interval-dv-proportion-unitary",
            "Validate DV_INTERVAL<DV_PROPORTION> — unitary",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivp_unitary,
        ),
        (
            "val/dv-interval-dv-proportion-percentage",
            "Validate DV_INTERVAL<DV_PROPORTION> — percentage",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivp_percent,
        ),
        (
            "val/dv-interval-dv-proportion-fraction",
            "Validate DV_INTERVAL<DV_PROPORTION> — fraction",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivp_fraction,
        ),
        (
            "val/dv-interval-dv-proportion-integer-fraction",
            "Validate DV_INTERVAL<DV_PROPORTION> — integer fraction",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivp_intfrac,
        ),
        (
            "val/dv-interval-dv-proportion-ratio-range",
            "Validate DV_INTERVAL<DV_PROPORTION> — ratio range",
            "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
            ivp_ratiorange,
        ),
    ];
    for &(id, title, cit, run) in interval {
        all.push(open(id, title, cit, run));
    }

    // ── 17.4 date_time ─────────────────────────────────────────────────────────
    all.push(open("val/dv-duration-open", "Validate DV_DURATION — open", "RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_duration));
    // No committable canonical-JSON composition carries a `C_DURATION`-constrained
    // DV_DURATION: all_types `items[at0018]` DV_DURATION is unconstrained; the
    // `obs_inst`/`ehrn_vital_signs` OPTs do constrain it (pattern `PDTH` / a
    // `PT24H` range) but ship no committable canonical composition — `obs_inst`'s
    // only instance is a CONTRIBUTION whose COMPOSITION omits `archetype_details`
    // on its content ENTRYs (fails the `Is_archetypeRoot` RM invariant as a bare
    // commit), and `ehrn_vital_signs` ships only a FLAT instance. Not drivable.
    all.push(open(
        "val/dv-duration-fields", "Validate DV_DURATION — fields", "RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_duration_fields,
    ));
    all.push(open(
        "val/dv-duration-range", "Validate DV_DURATION — range", "RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_duration_range,
    ));
    all.push(open(
        "val/dv-duration-fields-range", "Validate DV_DURATION — fields range", "RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_duration_fields_range,
    ));
    all.push(open("val/dv-time-open", "Validate DV_TIME — open", "RM 1.2.0 data_types §DV_TIME; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_time));
    all.push(open(
        "val/dv-time-constraint", "Validate DV_TIME — constraint", "RM 1.2.0 data_types §DV_TIME; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_time_constraint,
    ));
    all.push(open("val/dv-time-range", "Validate DV_TIME — range", "RM 1.2.0 data_types §DV_TIME; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_time_range));
    all.push(open("val/dv-date-open", "Validate DV_DATE — open", "RM 1.2.0 data_types §DV_DATE; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_date));
    all.push(open(
        "val/dv-date-constraint", "Validate DV_DATE — constraint", "RM 1.2.0 data_types §DV_DATE; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_date_constraint,
    ));
    all.push(open("val/dv-date-range", "Validate DV_DATE — range", "RM 1.2.0 data_types §DV_DATE; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_date_range));
    all.push(open(
        "val/dv-date-day-disallowed-pattern",
        "Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected)",
        "AM 1.4 C_DATE (yyyy-??-XX: month optional, day disallowed; org.openehr.am.aom14.c_date.adoc); valid_templates/all_types/Test_all_types.opt; ITS-REST 1.0.3 composition_create (422 rejected)",
        run_dv_date_day_disallowed,
    ));
    all.push(open(
        "val/dv-date-time-open", "Validate DV_DATE_TIME — open", "RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        open_dv_date_time,
    ));
    all.push(open(
        "val/dv-date-time-constraint", "Validate DV_DATE_TIME — constraint", "RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_date_time_constraint,
    ));
    all.push(open(
        "val/dv-date-time-range", "Validate DV_DATE_TIME — range", "RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_date_time_range,
    ));

    // ── 17.6 encapsulated ──────────────────────────────────────────────────────
    all.push(open("val/dv-parsable-open", "Validate DV_PARSABLE — open", "RM 1.2.0 data_types §DV_PARSABLE; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_parsable));
    all.push(open(
        "val/dv-parsable-value-formalism", "Validate DV_PARSABLE — value formalism", "RM 1.2.0 data_types §DV_PARSABLE; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_parsable_formalism,
    ));
    all.push(open(
        "val/dv-multimedia-open", "Validate DV_MULTIMEDIA — open", "RM 1.2.0 data_types §DV_MULTIMEDIA; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        open_dv_multimedia,
    ));
    all.push(open(
        "val/dv-multimedia-media-type", "Validate DV_MULTIMEDIA — media type", "RM 1.2.0 data_types §DV_MULTIMEDIA; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_multimedia_media_type,
    ));

    // ── 17.7 uri ───────────────────────────────────────────────────────────────
    all.push(open("val/dv-uri-open", "Validate DV_URI — open", "RM 1.2.0 data_types §DV_URI; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", open_dv_uri));
    all.push(open("val/dv-uri-pattern", "Validate DV_URI — pattern", "RM 1.2.0 data_types §DV_URI; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_uri_pattern));
    all.push(open("val/dv-uri-list", "Validate DV_URI — list", "RM 1.2.0 data_types §DV_URI; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)", run_dv_uri_list));
    all.push(open(
        "val/dv-ehr-uri-open", "Validate DV_EHR_URI — open", "RM 1.2.0 data_types §DV_EHR_URI; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_ehr_uri_open,
    ));
    all.push(open(
        "val/dv-ehr-uri-pattern", "Validate DV_EHR_URI — pattern", "RM 1.2.0 data_types §DV_EHR_URI; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_ehr_uri_pattern,
    ));
    all.push(open(
        "val/dv-ehr-uri-list", "Validate DV_EHR_URI — list", "RM 1.2.0 data_types §DV_EHR_URI; AM 1.4 C_* constraint; ITS-REST 1.0.3 composition_create (201/422)",
        run_dv_ehr_uri_list,
    ));

    all
}

/// A `validate_open` case: driven via [`drive::data_type_mandatory`] (auto-skips
/// when the corpus has no committable leaf of the type).
fn open(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: meta(id, title, citation),
        run,
    }
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

// Driven against `Base::AllTypes` (the `test_all_types.en.v1` composition), which
// carries a committable leaf of nearly every `DV_*` type — so every `validate_open`
// (RM/Schema-mandatory) row drives through the `/composition` endpoint. The two
// types `all_types` does not carry (`DV_SCALE`, `DV_EHR_URI`) auto-skip via
// `data_type_mandatory`'s leaf-presence guard (never a fabricated pass).
dt_open!(open_dv_text, Base::AllTypes, "DV_TEXT", "value");
dt_open!(open_dv_count, Base::AllTypes, "DV_COUNT", "magnitude");
dt_open!(open_dv_date_time, Base::AllTypes, "DV_DATE_TIME", "value");
dt_open!(open_dv_boolean, Base::AllTypes, "DV_BOOLEAN", "value");
dt_open!(
    open_dv_coded_text,
    Base::AllTypes,
    "DV_CODED_TEXT",
    "defining_code"
);
dt_open!(open_dv_ordinal, Base::AllTypes, "DV_ORDINAL", "value");
dt_open!(open_dv_quantity, Base::AllTypes, "DV_QUANTITY", "magnitude");
dt_open!(
    open_dv_proportion,
    Base::AllTypes,
    "DV_PROPORTION",
    "numerator"
);
dt_open!(open_dv_date, Base::AllTypes, "DV_DATE", "value");
dt_open!(open_dv_time, Base::AllTypes, "DV_TIME", "value");
dt_open!(open_dv_duration, Base::AllTypes, "DV_DURATION", "value");
dt_open!(open_dv_parsable, Base::AllTypes, "DV_PARSABLE", "value");
dt_open!(
    open_dv_multimedia,
    Base::AllTypes,
    "DV_MULTIMEDIA",
    "media_type"
);
dt_open!(open_dv_uri, Base::AllTypes, "DV_URI", "value");

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

/// master17.3 CONT-DV_QUANTITY-validate_property_units_mag. The `time_series` OPT
/// constrains OBSERVATION `data/events[0]/data/items[0]` (`at0004`) `DV_QUANTITY` to
/// property `openehr::129` (volume), units list `{mm3}`, **magnitude range**
/// `[0,∞)`. Its only committable instance is a FLAT one (no canonical
/// composition), converted in-harness via [`fixtures::flat_to_canonical`] (path
/// *b*): the vendored value is `702.9 mm3` (accepted); `-1.0 mm3` is below the
/// magnitude range → the truth-table "magnitude not in range for unit" row, and
/// `702.9 L` uses a unit off the `{mm3}` list → the "`L` is not allowed" row (both
/// rejected).
fn run_dv_quantity_units_mag<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let base = fixtures::flat_to_canonical(
            "time_series/time_series.opt",
            "compositions/FLAT/time_series.en.v1_20211018103435_000001_1.xml.flat.json",
        )
        .map_err(|e| CaseError::Codec(e.to_string()))?;
        drive::drive_constraint_base(
            ctx,
            "time_series/time_series.opt",
            base,
            "DV_QUANTITY 702.9 mm3 in magnitude range [0,inf) (accepted)",
            vec![
                (
                    "DV_QUANTITY magnitude -1.0 below range [0,inf) \
                     (C_DV_QUANTITY.list: magnitude not in range for unit)"
                        .to_owned(),
                    Box::new(|c: &mut Value| {
                        mutate::set_pointer(
                            c,
                            "/content/0/data/events/0/data/items/0/value/magnitude",
                            json!(-1.0),
                        );
                    }),
                    Expected::Rejected,
                ),
                (
                    "DV_QUANTITY units 'L' not in [mm3] (C_DV_QUANTITY.list: `L` is not allowed)"
                        .to_owned(),
                    Box::new(|c: &mut Value| {
                        mutate::set_pointer(
                            c,
                            "/content/0/data/events/0/data/items/0/value/units",
                            json!("L"),
                        );
                    }),
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

/// `Test minimal_action_2.opt` + its bare canonical composition. ACTION
/// `description/items[0]` (`at0002`) `DV_PROPORTION` is constrained by a
/// `C_INTEGER` list `{3,4}` on `type` (numerator `C_REAL` `[0,∞)`, denominator
/// `C_REAL` `(0,∞)`). The vendored value is `type=3` num=889 den=149 (accepted);
/// `type=0` (ratio) is off the `{3,4}` list → the truth-table "`C_INTEGER.list`"
/// rejection.
const MINIMAL_ACTION_2_PROPORTION: Constraint = Constraint {
    opt: "minimal/minimal_action_2.opt",
    comp: "valid_templates/minimal/minimal_action_2.instance.composition.json",
};

/// master17.3 CONT-DV_PROPORTION-validate_any_fraction (`type` `C_INTEGER.list`
/// `[3,4]`). `type=3` (fraction) is in the list (accepted); `type=0` (ratio) is
/// off it → the truth-table `C_INTEGER.list` row (rejected). num=889/den=149 stay
/// RM-valid for `type=0` (ratio: any denominator != 0), so the only violated
/// constraint is the archetype's `C_INTEGER.list`.
fn run_dv_proportion_any_fraction<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::drive_constraint(
            ctx,
            &MINIMAL_ACTION_2_PROPORTION,
            "DV_PROPORTION type 3 (fraction) in list [3,4] (accepted)",
            vec![(
                "DV_PROPORTION type 0 (ratio) not in list [3,4] (C_INTEGER.list)".to_owned(),
                Box::new(|c: &mut Value| {
                    mutate::set_pointer(c, "/content/0/description/items/0/value/type", json!(0));
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

/// `val/dv-date-day-disallowed-pattern` — the negative companion to the owned
/// (corrected) `all_types` fixture (`testdata/fixtures/REGISTER.md`, owner ruling
/// 2026-07-09 B2). The **vendored** `all_types.composition.json` carries a full
/// `DV_DATE` (`2021-10-18`) at the INSTRUCTION activity `at0003` leaf, but
/// `Test_all_types.opt` constrains that leaf with the `C_DATE` pattern
/// `yyyy-??-XX` — month optional, **day disallowed** (`VALIDITY_KIND.disallowed`;
/// AOM 1.4 `org.openehr.am.aom14.c_date.adoc`: `XX` = disallowed). A spec-correct
/// validator must reject the fixture (EHRbase/archie is lenient on `day_validity`
/// and accepts it, which is how the defect survived upstream). The case uploads
/// the OPT, creates a fresh EHR, commits the **uncorrected defective** composition
/// (the owned `invalid/` copy, pinned byte-identical to the vendored original by
/// the [`fixtures`] guard test), and asserts the SUT rejects it (ITS-REST
/// `composition_create` `422`) — keeping the defect under test while the positive
/// cases commit the corrected copy via [`drive::Base::AllTypes`].
fn run_dv_date_day_disallowed<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        support::ensure_opt(ctx, "all_types/Test_all_types.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        // The uncorrected defective fixture (day-bearing DV_DATE at at0003),
        // pinned byte-identical to the vendored original by the fixtures guard.
        let comp = fixtures::owned_fixture("invalid/compositions/all_types.composition.json")
            .map_err(|e| CaseError::Codec(e.to_string()))?;
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                    .json_body(&comp)?
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 422)?;
        Ok(DataSetReport {
            passed: 1,
            total: 1,
        })
    })
}

// ── authored leaf value-constraint cases (master17, via `all_types`) ──────────
//
// The vendored corpus ships no OPT constraining these leaves, so the case authors
// the constraint into the `all_types` OPT and drives the base (which satisfies it)
// + an out-of-constraint copy. Limited to leaves the `all_types` composition
// carries (`DV_TEXT` items[0], `DV_COUNT` items[4]); the string/integer
// `pattern`/`list`/`range` constraints these use are surfaced into the leaf input
// by the WebTemplate builder and enforced by `validation::leaf`.

/// master17.2 CONT-DV_TEXT-validate_list: author a `C_STRING` `list` on
/// `DV_TEXT.value` that includes the base value (accepted), then commit a value off
/// the list (rejected).
fn run_dv_text_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        // owned corrected copy (see testdata/fixtures/REGISTER.md)
        let base = drive::Base::AllTypes.load()?;
        let ptr = leaf_ptr(0, "value");
        let Some(base_val) = base
            .pointer(&ptr)
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return Err(CaseError::Skipped(
                "all_types composition has no DV_TEXT value leaf to constrain".to_owned(),
            ));
        };
        let list = vec![base_val, "cnf-allowed-alternate".to_owned()];
        drive_leaf(
            ctx,
            "cnf_cont_dv_text_list",
            move |opt| author::constrain_leaf_string(opt, "DV_TEXT", "value", None, list),
            "DV_TEXT value in the C_STRING list (accepted)",
            "DV_TEXT value not in the C_STRING list (C_STRING.list)".to_owned(),
            &ptr,
            json!("cnf-not-in-list-value"),
        )
        .await
    })
}

/// master17.3 CONT-DV_COUNT-validate_range: author a `C_INTEGER` `range` `[0,10]` on
/// `DV_COUNT.magnitude` (base `3` is in range → accepted), then commit `999`
/// (rejected).
fn run_dv_count_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive_leaf(
            ctx,
            "cnf_cont_dv_count_range",
            |opt| {
                author::constrain_leaf_integer(opt, "DV_COUNT", "magnitude", Some((0, 10)), vec![])
            },
            "DV_COUNT magnitude 3 in range [0,10] (accepted)",
            "DV_COUNT magnitude 999 outside range [0,10] (C_INTEGER.range)".to_owned(),
            &leaf_ptr(4, "magnitude"),
            json!(999),
        )
        .await
    })
}

/// master17.3 CONT-DV_COUNT-validate_list: author a `C_INTEGER` `list` `{3}` on
/// `DV_COUNT.magnitude` (base `3` is in the list → accepted), then commit `7`
/// (rejected).
fn run_dv_count_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive_leaf(
            ctx,
            "cnf_cont_dv_count_list",
            |opt| author::constrain_leaf_integer(opt, "DV_COUNT", "magnitude", None, vec![3]),
            "DV_COUNT magnitude 3 in the C_INTEGER list {3} (accepted)",
            "DV_COUNT magnitude 7 not in the C_INTEGER list {3} (C_INTEGER.list)".to_owned(),
            &leaf_ptr(4, "magnitude"),
            json!(7),
        )
        .await
    })
}

/// A flexible authored-leaf driver: author `constrain` into the `all_types` OPT,
/// then commit one composition per `rows` entry — each a clone of the base with the
/// listed `(pointer, value)` mutations applied — asserting its `Expected`. Handles
/// cases whose accepted instance itself needs a mutation (e.g. `DV_BOOLEAN`
/// only-false, `DV_PROPORTION` kinds whose valid instance differs from the base).
/// One leaf-table row: `(label, [(path, value)], expected)`.
type LeafRow = (String, Vec<(String, Value)>, Expected);

async fn drive_leaf_rows(
    ctx: &RunContext<'_>,
    tid: &'static str,
    constrain: impl FnOnce(&mut OperationalTemplate) -> bool,
    rows: Vec<LeafRow>,
) -> Result<DataSetReport, CaseError> {
    let mut opt = author::parse_base(ALL_TYPES_OPT)?;
    author::set_template_id(&mut opt, tid);
    if !constrain(&mut opt) {
        return Err(CaseError::Assertion(format!(
            "authoring the constraint for {tid} matched no leaf in the all_types OPT"
        )));
    }
    let xml = author::to_xml(&opt)?;
    let base = drive::Base::AllTypes.load()?; // owned corrected copy (see testdata/fixtures/REGISTER.md)
    let mut drows: Vec<(String, Value, Expected)> = Vec::new();
    for (label, muts, expected) in rows {
        let mut c = base.clone();
        mutate::retarget_template(&mut c, tid);
        for (ptr, val) in muts {
            mutate::set_pointer(&mut c, &ptr, val);
        }
        drows.push((label, c, expected));
    }
    drive::drive_authored(ctx, &xml, drows).await
}

// ── DV_BOOLEAN (master17.1) — C_BOOLEAN true-only / false-only ────────────────

/// CONT-DV_BOOLEAN-only_true_allowed: `C_BOOLEAN {true}` — `value=true` accepted,
/// `value=false` rejected.
fn run_dv_boolean_true<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(10, "value");
        drive_leaf_rows(
            ctx,
            "cnf_cont_dv_boolean_true",
            |opt| author::constrain_leaf_boolean(opt, "DV_BOOLEAN", "value", true, false),
            vec![
                (
                    "value true allowed (C_BOOLEAN true-only)".to_owned(),
                    vec![(p.clone(), json!(true))],
                    Expected::Accepted,
                ),
                (
                    "value false not allowed (C_BOOLEAN true-only)".to_owned(),
                    vec![(p, json!(false))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

/// CONT-DV_BOOLEAN-only_false_allowed: `C_BOOLEAN {false}` — `value=false`
/// accepted, `value=true` rejected.
fn run_dv_boolean_false<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(10, "value");
        drive_leaf_rows(
            ctx,
            "cnf_cont_dv_boolean_false",
            |opt| author::constrain_leaf_boolean(opt, "DV_BOOLEAN", "value", false, true),
            vec![
                (
                    "value false allowed (C_BOOLEAN false-only)".to_owned(),
                    vec![(p.clone(), json!(false))],
                    Expected::Accepted,
                ),
                (
                    "value true not allowed (C_BOOLEAN false-only)".to_owned(),
                    vec![(p, json!(true))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

// ── DV_PARSABLE (master17.6) — C_STRING on formalism ──────────────────────────

/// CONT-DV_PARSABLE-validate_value_formalism: constrain `DV_PARSABLE.formalism` to
/// the `{ISO8601}` list — base `ISO8601` accepted, `text/xyz` rejected.
fn run_dv_parsable_formalism<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(13, "formalism");
        drive_leaf_rows(
            ctx,
            "cnf_cont_dv_parsable_formalism",
            |opt| {
                author::constrain_leaf_string(
                    opt,
                    "DV_PARSABLE",
                    "formalism",
                    None,
                    vec!["ISO8601".to_owned()],
                )
            },
            vec![
                (
                    "formalism ISO8601 in list (accepted)".to_owned(),
                    vec![],
                    Expected::Accepted,
                ),
                (
                    "formalism text/xyz not in list (C_STRING.list)".to_owned(),
                    vec![(p, json!("text/xyz"))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

// ── DV_IDENTIFIER (master17.1) — C_STRING pattern / list on id ────────────────

/// CONT-DV_IDENTIFIER-validate_all_pattern: constrain `DV_IDENTIFIER.id` to the
/// digit pattern `[0-9]+` — base `54480987` accepted, `ABC` rejected.
fn run_dv_identifier_pattern<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(14, "id");
        drive_leaf_rows(
            ctx,
            "cnf_cont_dv_identifier_pattern",
            |opt| author::constrain_leaf_string(opt, "DV_IDENTIFIER", "id", Some("[0-9]+"), vec![]),
            vec![
                (
                    "id 54480987 matches [0-9]+ (accepted)".to_owned(),
                    vec![],
                    Expected::Accepted,
                ),
                (
                    "id ABC does not match [0-9]+ (C_STRING.pattern)".to_owned(),
                    vec![(p, json!("ABC"))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

/// CONT-DV_IDENTIFIER-validate_all_list: constrain `DV_IDENTIFIER.id` to the list
/// `{54480987}` — base accepted, `99999999` rejected.
fn run_dv_identifier_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(14, "id");
        drive_leaf_rows(
            ctx,
            "cnf_cont_dv_identifier_list",
            |opt| {
                author::constrain_leaf_string(
                    opt,
                    "DV_IDENTIFIER",
                    "id",
                    None,
                    vec!["54480987".to_owned()],
                )
            },
            vec![
                (
                    "id 54480987 in list (accepted)".to_owned(),
                    vec![],
                    Expected::Accepted,
                ),
                (
                    "id 99999999 not in list (C_STRING.list)".to_owned(),
                    vec![(p, json!("99999999"))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

// ── DV_PROPORTION kinds (master17.3) — C_INTEGER on type + RM kind validity ────

/// Drive one `DV_PROPORTION` `type`-kind case: constrain `DV_PROPORTION.type` to the
/// single kind code `kind`, commit an accepted instance (that kind with RM-valid
/// numerator/denominator) and a rejected instance (`type=0` ratio, off the list).
/// `den` sets the accepted instance's denominator to satisfy the kind's RM
/// `Proportion` invariant (unitary ⇒ 1, percent ⇒ 100, `fraction/integer_fraction` ⇒
/// integer parts). `num` sets the numerator likewise.
fn drive_proportion_kind<'a>(
    ctx: &'a RunContext<'a>,
    tid: &'static str,
    kind: i32,
    num: Value,
    den: Value,
) -> CaseFuture<'a> {
    Box::pin(async move {
        let ty = leaf_ptr(15, "type");
        let n = leaf_ptr(15, "numerator");
        let d = leaf_ptr(15, "denominator");
        drive_leaf_rows(
            ctx,
            tid,
            move |opt| {
                author::constrain_leaf_integer(opt, "DV_PROPORTION", "type", None, vec![kind])
            },
            vec![
                (
                    format!("type {kind} in list {{{kind}}} with RM-valid num/den (accepted)"),
                    vec![(ty.clone(), json!(kind)), (n, num), (d, den)],
                    Expected::Accepted,
                ),
                (
                    // An off-list kind: 0 (ratio) for the non-ratio cases; for
                    // the ratio case itself 0 IS the permitted kind, so use 2
                    // (percent) — the previous unconditional 0 made the ratio
                    // case's reject row a no-op mutation that could never
                    // reject (master17.3 CONT-DV_PROPORTION truth table).
                    format!(
                        "type {bad} not in list {{{kind}}} (C_INTEGER.list)",
                        bad = if kind == 0 { 2 } else { 0 }
                    ),
                    vec![(ty, json!(if kind == 0 { 2 } else { 0 }))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_proportion_ratio<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    // ratio (0): any numerator/denominator (denominator != 0).
    drive_proportion_kind(ctx, "cnf_cont_dv_prop_ratio", 0, json!(398.5), json!(209.2))
}
fn run_dv_proportion_unitary<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    // unitary (1): denominator must be 1.
    drive_proportion_kind(ctx, "cnf_cont_dv_prop_unitary", 1, json!(5.0), json!(1.0))
}
fn run_dv_proportion_percent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    // percent (2): denominator must be 100.
    drive_proportion_kind(
        ctx,
        "cnf_cont_dv_prop_percent",
        2,
        json!(42.0),
        json!(100.0),
    )
}
fn run_dv_proportion_fraction<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    // fraction (3): integer numerator/denominator.
    drive_proportion_kind(ctx, "cnf_cont_dv_prop_fraction", 3, json!(3.0), json!(4.0))
}
fn run_dv_proportion_integer_fraction<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    // integer_fraction (4): integer numerator/denominator.
    drive_proportion_kind(
        ctx,
        "cnf_cont_dv_prop_int_fraction",
        4,
        json!(3.0),
        json!(4.0),
    )
}

// ── temporal value constraints (master17.4): C_DATE/TIME/DATE_TIME/DURATION ────
//
// Base leaves: DV_DATE items[5]=`2021-10-18`, DV_TIME items[8]=`22:18:16`,
// DV_DATE_TIME items[6], DV_DURATION items[11]=`PT30M`. (Our validator currently
// defers temporal range/pattern enforcement — `validation::leaf` — so several of
// these drive as open findings until that gap closes; driven, never skipped.)

/// A temporal case: author a temporal `C_*` constraint on `host`'s value, commit
/// the in-constraint base value (accepted) and an out-of-constraint value
/// (rejected).
#[allow(clippy::too_many_arguments)]
fn drive_temporal<'a>(
    ctx: &'a RunContext<'a>,
    tid: &'static str,
    host: &'static str,
    rm_prim: &'static str,
    prim: CPrimitive,
    idx: usize,
    bad: Value,
    label: &'static str,
) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(idx, "value");
        drive_leaf_rows(
            ctx,
            tid,
            move |opt| author::constrain_leaf_temporal(opt, host, rm_prim, prim),
            vec![
                (
                    format!("{host} base value satisfies the constraint (accepted)"),
                    vec![],
                    Expected::Accepted,
                ),
                (
                    format!("{host} {label} (rejected)"),
                    vec![(p, bad)],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_date_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_date_pat",
        "DV_DATE",
        "Date",
        author::c_date(Some("yyyy-mm-dd"), None),
        5,
        json!("2021"),
        "partial date '2021' violates yyyy-mm-dd (C_DATE.pattern)",
    )
}
fn run_dv_date_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_date_rng",
        "DV_DATE",
        "Date",
        author::c_date(None, Some(("2021-01-01", "2021-12-31"))),
        5,
        json!("2025-06-01"),
        "'2025-06-01' outside [2021-01-01,2021-12-31] (C_DATE.range)",
    )
}
fn run_dv_time_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_time_pat",
        "DV_TIME",
        "Time",
        author::c_time(Some("HH:MM:SS"), None),
        8,
        json!("22"),
        "partial time '22' violates HH:MM:SS (C_TIME.pattern)",
    )
}
fn run_dv_time_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_time_rng",
        "DV_TIME",
        "Time",
        author::c_time(None, Some(("00:00:00", "23:00:00"))),
        8,
        json!("23:59:59"),
        "'23:59:59' outside [00:00:00,23:00:00] (C_TIME.range)",
    )
}
fn run_dv_date_time_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_datetime_rng",
        "DV_DATE_TIME",
        "Date_Time",
        author::c_date_time(None, Some(("2021-01-01T00:00:00", "2021-12-31T23:59:59"))),
        6,
        json!("2025-06-01T12:00:00"),
        "'2025-06-01T12:00:00' outside the range (C_DATE_TIME.range)",
    )
}
fn run_dv_duration_fields<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_dur_fields",
        "DV_DURATION",
        "Duration",
        // AOM 1.4 §C_DURATION: the pattern's letters (before/after `T`) are the
        // *allowed* fields — `PTHMS` = time-only, so the base `PT30M` conforms
        // and any date field is forbidden. A bare `PT` allows nothing and would
        // reject the base row too.
        author::c_duration(Some("PTHMS"), None),
        11,
        json!("P1Y"),
        "'P1Y' uses a date field the PTHMS pattern forbids (C_DURATION.pattern)",
    )
}
fn run_dv_duration_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_dur_rng",
        "DV_DURATION",
        "Duration",
        author::c_duration(None, Some(("PT0S", "PT1H"))),
        11,
        json!("PT5H"),
        "'PT5H' outside [PT0S,PT1H] (C_DURATION.range)",
    )
}
fn run_dv_duration_fields_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    drive_temporal(
        ctx,
        "cnf_cont_dv_dur_fr",
        "DV_DURATION",
        "Duration",
        // `PTHM` allows hours+minutes (the base `PT30M` conforms); `PT5H` is an
        // allowed field but exceeds the range.
        author::c_duration(Some("PTHM"), Some(("PT0S", "PT1H"))),
        11,
        json!("PT5H"),
        "'PT5H' outside [PT0S,PT1H] (C_DURATION.pattern+range)",
    )
}

// ── DV_QUANTITY property (master17.3): C_DV_QUANTITY.property ──────────────────

/// CONT-DV_QUANTITY-validate_property: constrain the quantity `property` to mass
/// (`openehr::124`); base units `mg` (mass) accepted, units `cm` (length, a
/// different property) rejected.
fn run_dv_quantity_property<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let u = leaf_ptr(3, "units");
        drive_leaf_rows(
            ctx,
            "cnf_cont_dv_quantity_property",
            |opt| author::constrain_dv_quantity_property(opt, "openehr", "124"),
            vec![
                (
                    "units mg matches property mass openehr::124 (accepted)".to_owned(),
                    vec![],
                    Expected::Accepted,
                ),
                (
                    "units cm (length) violate property mass openehr::124 (C_DV_QUANTITY.property)"
                        .to_owned(),
                    vec![(u, json!("cm"))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

// ── DV_INTERVAL<T> (master17.3/17.5): interval leaves absent from all_types ────
//
// No base composition carries a `DV_INTERVAL<T>` leaf, so each case **slot-retypes**
// the `DV_COUNT` slot (items[4]) to an open `DV_INTERVAL` and commits a valid
// interval of the case's bound type (accepted) plus a malformed `lower > upper`
// interval (rejected per the RM `Interval` invariant `lower <= upper`, which
// applies to every bound type). The specific per-variant bound / range / list
// constraints require `DV_INTERVAL` constraint support the validator does not yet
// have, so most drive as recorded findings — driven, never skipped. The bound type
// is cited by the case id / citation.

/// A canonical `DV_INTERVAL` with included, bounded ends.
fn iv(lower: Value, upper: Value) -> Value {
    json!({
        "_type": "DV_INTERVAL",
        "lower": lower,
        "upper": upper,
        "lower_included": true,
        "upper_included": true,
        "lower_unbounded": false,
        "upper_unbounded": false,
    })
}

/// Drive a `DV_INTERVAL<T>` case: retype the `DV_COUNT` slot to an open
/// `DV_INTERVAL`, commit `[lower,upper]` (accepted) and the reversed `[upper,lower]`
/// (rejected, RM `Interval.lower <= upper`).
fn drive_interval<'a>(
    ctx: &'a RunContext<'a>,
    tid: &'static str,
    lower: Value,
    upper: Value,
) -> CaseFuture<'a> {
    Box::pin(async move {
        // The whole ELEMENT.value is replaced (`…/items/4/value`), not a datum
        // inside it — `leaf_ptr(4, "value")` would nest the interval inside the
        // retyped leaf and the slot would still hold the original type.
        let p = value_ptr(4);
        drive_leaf_rows(
            ctx,
            tid,
            |opt| author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_INTERVAL")),
            vec![
                (
                    "valid DV_INTERVAL lower<=upper (accepted)".to_owned(),
                    vec![(p.clone(), iv(lower.clone(), upper.clone()))],
                    Expected::Accepted,
                ),
                (
                    "malformed DV_INTERVAL lower>upper (RM Interval invariant)".to_owned(),
                    vec![(p, iv(upper, lower))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn dv_count(m: i64) -> Value {
    json!({ "_type": "DV_COUNT", "magnitude": m })
}
fn dv_quantity(m: f64) -> Value {
    json!({ "_type": "DV_QUANTITY", "magnitude": m, "units": "mg" })
}
fn dv_date(v: &str) -> Value {
    json!({ "_type": "DV_DATE", "value": v })
}
fn dv_date_time(v: &str) -> Value {
    json!({ "_type": "DV_DATE_TIME", "value": v })
}
fn dv_time(v: &str) -> Value {
    json!({ "_type": "DV_TIME", "value": v })
}
fn dv_duration(v: &str) -> Value {
    json!({ "_type": "DV_DURATION", "value": v })
}
fn dv_ordinal(v: i64, code: &str) -> Value {
    json!({ "_type": "DV_ORDINAL", "value": v,
        "symbol": { "_type": "DV_CODED_TEXT", "value": "ord",
            "defining_code": { "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "local" },
                "code_string": code } } })
}
fn dv_scale(v: f64, code: &str) -> Value {
    json!({ "_type": "DV_SCALE", "value": v,
        "symbol": { "_type": "DV_CODED_TEXT", "value": "sc",
            "defining_code": { "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "local" },
                "code_string": code } } })
}
fn dv_proportion(n: f64, d: f64) -> Value {
    json!({ "_type": "DV_PROPORTION", "numerator": n, "denominator": d, "type": 0 })
}

macro_rules! iv_count {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_count(1), dv_count(10))
        }
    };
}
macro_rules! iv_quantity {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_quantity(1.0), dv_quantity(10.0))
        }
    };
}
macro_rules! iv_date {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_date("2021-01-01"), dv_date("2021-12-31"))
        }
    };
}
macro_rules! iv_date_time {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(
                ctx,
                $tid,
                dv_date_time("2021-01-01T00:00:00"),
                dv_date_time("2021-12-31T00:00:00"),
            )
        }
    };
}
macro_rules! iv_time {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_time("01:00:00"), dv_time("10:00:00"))
        }
    };
}
macro_rules! iv_duration {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_duration("PT1H"), dv_duration("PT10H"))
        }
    };
}
macro_rules! iv_ordinal {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_ordinal(0, "at0014"), dv_ordinal(1, "at0015"))
        }
    };
}
macro_rules! iv_scale {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_scale(1.0, "at0014"), dv_scale(2.0, "at0015"))
        }
    };
}
macro_rules! iv_proportion {
    ($fn:ident, $tid:literal) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            drive_interval(ctx, $tid, dv_proportion(1.0, 2.0), dv_proportion(3.0, 2.0))
        }
    };
}

iv_count!(ivc_open, "cnf_iv_count_open");
iv_count!(ivc_lu, "cnf_iv_count_lu");
iv_count!(ivc_lul, "cnf_iv_count_lul");
iv_quantity!(ivq_open, "cnf_iv_quantity_open");
iv_quantity!(ivq_ul, "cnf_iv_quantity_ul");
iv_date!(ivd_open, "cnf_iv_date_open");
iv_date!(ivd_luc, "cnf_iv_date_luc");
iv_date!(ivd_lur, "cnf_iv_date_lur");
iv_date_time!(ivdt_open, "cnf_iv_datetime_open");
iv_date_time!(ivdt_luc, "cnf_iv_datetime_luc");
iv_date_time!(ivdt_lur, "cnf_iv_datetime_lur");
iv_time!(ivt_open, "cnf_iv_time_open");
iv_time!(ivt_luc, "cnf_iv_time_luc");
iv_time!(ivt_lur, "cnf_iv_time_lur");
iv_duration!(ivdu_open, "cnf_iv_duration_open");
iv_duration!(ivdu_c, "cnf_iv_duration_c");
iv_duration!(ivdu_r, "cnf_iv_duration_r");
iv_ordinal!(ivo_open, "cnf_iv_ordinal_open");
iv_ordinal!(ivo_c, "cnf_iv_ordinal_c");
iv_scale!(ivs_open, "cnf_iv_scale_open");
iv_scale!(ivs_c, "cnf_iv_scale_c");
iv_proportion!(ivp_open, "cnf_iv_proportion_open");
iv_proportion!(ivp_ratio, "cnf_iv_proportion_ratio");
iv_proportion!(ivp_unitary, "cnf_iv_proportion_unitary");
iv_proportion!(ivp_percent, "cnf_iv_proportion_percent");
iv_proportion!(ivp_fraction, "cnf_iv_proportion_fraction");
iv_proportion!(ivp_intfrac, "cnf_iv_proportion_intfrac");
iv_proportion!(ivp_ratiorange, "cnf_iv_proportion_ratiorange");

/// master17.3 CONT-DV_PROPORTION-validate_ratio_range: constrain
/// `DV_PROPORTION.numerator` (`C_REAL`) to the range `[0,1000]` — base `398.5`
/// accepted, `9999.0` rejected.
fn run_dv_proportion_ratio_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let n = leaf_ptr(15, "numerator");
        drive_leaf_rows(
            ctx,
            "cnf_cont_dv_prop_ratio_range",
            |opt| {
                author::constrain_leaf_real(
                    opt,
                    "DV_PROPORTION",
                    "numerator",
                    Some((0.0, 1000.0)),
                    vec![],
                )
            },
            vec![
                (
                    "numerator 398.5 in range [0,1000] (accepted)".to_owned(),
                    vec![],
                    Expected::Accepted,
                ),
                (
                    "numerator 9999 outside range [0,1000] (C_REAL.range)".to_owned(),
                    vec![(n, json!(9999.0))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

// ── slot-retyped leaves absent from all_types: DV_URI/DV_EHR_URI/DV_SCALE ──────
//
// These DV_* types have no leaf in the all_types composition, so each case
// retypes the DV_COUNT scratch slot (items[4]) to the target type and commits a
// whole-value instance of it.

/// The pointer to the whole value object of the `items[idx]` leaf.
fn value_ptr(idx: usize) -> String {
    format!("/content/0/data/events/0/data/items/{idx}/value")
}

fn run_dv_uri_pattern<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        drive_leaf_rows(
            ctx,
            "cnf_dv_uri_pattern",
            |opt| {
                author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_URI"))
                    && author::constrain_leaf_string(
                        opt,
                        "DV_URI",
                        "value",
                        Some("http://.*"),
                        vec![],
                    )
            },
            vec![
                (
                    "URI http://ok matches pattern (accepted)".to_owned(),
                    vec![(p.clone(), json!({"_type":"DV_URI","value":"http://ok"}))],
                    Expected::Accepted,
                ),
                (
                    "URI ftp://no not matching http://.* (C_STRING.pattern)".to_owned(),
                    vec![(p, json!({"_type":"DV_URI","value":"ftp://no"}))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_uri_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        drive_leaf_rows(
            ctx,
            "cnf_dv_uri_list",
            |opt| {
                author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_URI"))
                    && author::constrain_leaf_string(
                        opt,
                        "DV_URI",
                        "value",
                        None,
                        vec!["http://ok".to_owned()],
                    )
            },
            vec![
                (
                    "URI http://ok in list (accepted)".to_owned(),
                    vec![(p.clone(), json!({"_type":"DV_URI","value":"http://ok"}))],
                    Expected::Accepted,
                ),
                (
                    "URI http://other not in list (C_STRING.list)".to_owned(),
                    vec![(p, json!({"_type":"DV_URI","value":"http://other"}))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_ehr_uri_open<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        drive_leaf_rows(
            ctx,
            "cnf_dv_ehr_uri_open",
            |opt| author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_EHR_URI")),
            vec![
                (
                    "DV_EHR_URI with value (accepted)".to_owned(),
                    vec![(p.clone(), json!({"_type":"DV_EHR_URI","value":"ehr://x/y"}))],
                    Expected::Accepted,
                ),
                (
                    "DV_EHR_URI without value (RM/Schema mandatory)".to_owned(),
                    vec![(p, json!({"_type":"DV_EHR_URI"}))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_ehr_uri_pattern<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        drive_leaf_rows(
            ctx,
            "cnf_dv_ehr_uri_pattern",
            |opt| {
                author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_EHR_URI"))
                    && author::constrain_leaf_string(
                        opt,
                        "DV_EHR_URI",
                        "value",
                        Some("ehr://.*"),
                        vec![],
                    )
            },
            vec![
                (
                    "ehr://x matches pattern (accepted)".to_owned(),
                    vec![(p.clone(), json!({"_type":"DV_EHR_URI","value":"ehr://x"}))],
                    Expected::Accepted,
                ),
                (
                    "http://x not matching ehr://.* (C_STRING.pattern)".to_owned(),
                    vec![(p, json!({"_type":"DV_EHR_URI","value":"http://x"}))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_ehr_uri_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        drive_leaf_rows(
            ctx,
            "cnf_dv_ehr_uri_list",
            |opt| {
                author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_EHR_URI"))
                    && author::constrain_leaf_string(
                        opt,
                        "DV_EHR_URI",
                        "value",
                        None,
                        vec!["ehr://ok".to_owned()],
                    )
            },
            vec![
                (
                    "ehr://ok in list (accepted)".to_owned(),
                    vec![(p.clone(), json!({"_type":"DV_EHR_URI","value":"ehr://ok"}))],
                    Expected::Accepted,
                ),
                (
                    "ehr://other not in list (C_STRING.list)".to_owned(),
                    vec![(p, json!({"_type":"DV_EHR_URI","value":"ehr://other"}))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn dv_scale_value(v: f64, code: &str) -> Value {
    json!({ "_type": "DV_SCALE", "value": v,
        "symbol": { "_type": "DV_CODED_TEXT", "value": "sc",
            "defining_code": { "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "local" },
                "code_string": code } } })
}

fn run_dv_scale_open<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        drive_leaf_rows(
            ctx,
            "cnf_dv_scale_open",
            |opt| author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_SCALE")),
            vec![
                (
                    "DV_SCALE with value+symbol (accepted)".to_owned(),
                    vec![(p.clone(), dv_scale_value(1.0, "at0014"))],
                    Expected::Accepted,
                ),
                (
                    "DV_SCALE without value (RM/Schema mandatory)".to_owned(),
                    vec![(
                        p,
                        json!({"_type":"DV_SCALE","symbol":{"_type":"DV_CODED_TEXT","value":"sc",
                            "defining_code":{"_type":"CODE_PHRASE",
                                "terminology_id":{"_type":"TERMINOLOGY_ID","value":"local"},
                                "code_string":"at0014"}}}),
                    )],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_scale_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        // Constrain the retyped scale's value to the C_REAL list {1.0}; base 1.0
        // accepted, 9.0 off the list rejected.
        drive_leaf_rows(
            ctx,
            "cnf_dv_scale_constraint",
            |opt| {
                author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_SCALE"))
                    && author::constrain_leaf_real(opt, "DV_SCALE", "value", None, vec![1.0])
            },
            vec![
                (
                    "DV_SCALE value 1.0 in list {1.0} (accepted)".to_owned(),
                    vec![(p.clone(), dv_scale_value(1.0, "at0014"))],
                    Expected::Accepted,
                ),
                (
                    "DV_SCALE value 9.0 not in list {1.0} (C_REAL.list)".to_owned(),
                    vec![(p, dv_scale_value(9.0, "at0014"))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

// ── code-phrase leaves present in all_types: DV_MULTIMEDIA / DV_CODED_TEXT ─────

fn run_dv_multimedia_media_type<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(12, "media_type/code_string");
        let t = leaf_ptr(12, "media_type/terminology_id/value");
        drive_leaf_rows(
            ctx,
            "cnf_dv_multimedia_media_type",
            |opt| {
                author::constrain_codephrase(
                    opt,
                    "DV_MULTIMEDIA",
                    "media_type",
                    "IANA_media-types",
                    vec!["image/png".to_owned()],
                )
            },
            vec![
                (
                    "media_type image/png in list (accepted)".to_owned(),
                    vec![
                        (t.clone(), json!("IANA_media-types")),
                        (p.clone(), json!("image/png")),
                    ],
                    Expected::Accepted,
                ),
                (
                    "media_type image/gif not in list (C_CODE_PHRASE)".to_owned(),
                    vec![(t, json!("IANA_media-types")), (p, json!("image/gif"))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}

fn run_dv_coded_ext_term<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let code = leaf_ptr(1, "defining_code/code_string");
        let term = leaf_ptr(1, "defining_code/terminology_id/value");
        drive_leaf_rows(
            ctx,
            "cnf_dv_coded_ext_term",
            |opt| {
                // Pinned to ELEMENT at0005 (the OBSERVATION's DV_CODED_TEXT
                // leaf this case mutates): the first-match variant constrained
                // the COMPOSITION `category` coded text instead, so neither
                // row ever exercised the external code list.
                author::constrain_codephrase_under(
                    opt,
                    "openEHR-EHR-OBSERVATION.test_all_types",
                    "at0005",
                    "defining_code",
                    "SNOMED-CT",
                    vec!["73211009".to_owned()],
                )
            },
            vec![
                (
                    "SNOMED-CT 73211009 in the external code_list (accepted)".to_owned(),
                    vec![
                        (term.clone(), json!("SNOMED-CT")),
                        (code.clone(), json!("73211009")),
                    ],
                    Expected::Accepted,
                ),
                (
                    "SNOMED-CT 99999999 not in the external code_list (C_CODE_PHRASE)".to_owned(),
                    vec![(term, json!("SNOMED-CT")), (code, json!("99999999"))],
                    Expected::Rejected,
                ),
            ],
        )
        .await
    })
}
