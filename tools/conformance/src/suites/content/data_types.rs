//! master17.1–17.7 — `DATA_VALUE` data-validation truth tables
//! (`master17.N-content_tc_data_types-*.adoc`), the register-13 spine.
//!
//! Every case commits a data instance and asserts the SUT accepts or rejects
//! it per the constraint the schedule case expresses (capability
//! [`Capability::ArchetypeValidation`] — CORE + STANDARD, `profiles master03
//! §Functional`). All 81 ECC ids `ECC-VAL-039`…`ECC-VAL-119` keep their
//! pre-W-10 slugs (register 13, ID stability); 80 map 1:1 onto the 80 distinct
//! schedule case ids, one (`ECC-VAL-119`) is an ECC-original negative guard
//! (`ScheduleTrace::EccOriginal`, §3).
//!
//! **Spine-first (owner ruling 2026-07-13):** accept/reject expectations trace
//! to the master17 data-set tables + the RM `data_types`/`Interval` invariants,
//! **never** to observed ehrbase-rs behaviour. A case our server fails is a
//! correct outcome — the driver returns the finding, never a masked pass
//! (`drive::check`).
//!
//! **G-2 — coverage bounds are logged, never silent.** The schedule expresses
//! each case as a truth table of many data-set rows (~1,130 total); the suite
//! drives ~2 variants/case (one in-constraint accept + one out-of-constraint
//! reject), plus the marked boundary rows where the authoring machinery makes
//! it cheap (the interval `_open` invariant triple). Every case declares its
//! schedule table's row count via
//! [`DataSetReport::of_schedule_rows`], so the ~7:1 collapse is a visible,
//! auditable number in the report.
//!
//! **G-1 — `DV_INTERVAL`<T> (`ECC-VAL-068`…`095`, 28 ids).** The schedule's
//! per-variant *bound* constraints (`C_INTEGER.range`/`.list` on the interval
//! ends, `C_DV_QUANTITY.list`, temporal bounds, proportion-kind lists) are
//! **inexpressible** with the carried opt14 authoring machinery
//! ([`super::author`] constrains a DV_* leaf's primitive value attribute, not a
//! `DV_INTERVAL`'s `lower`/`upper` bound objects — and the validator has no
//! `DV_INTERVAL` constraint support). So each case is **bound-declared**
//! (`of_schedule_rows` + a per-variant boundary comment citing this register),
//! never a silent generic probe presented as full coverage. What *is* asserted
//! is the RM-invariant subset that holds for every bound type: the nine
//! `validate_open` cases drive the three universal `Interval` invariants
//! (`lower ≤ upper`, `lower_included_valid`, `upper_included_valid` — BASE
//! `foundation_types` §Interval), the nineteen non-open cases drive the
//! `lower ≤ upper` invariant. Split: **0 per-variant-authored / 28
//! bound-declared** (the machinery cannot express any `DV_INTERVAL` bound
//! constraint).
//!
//! **G-3 — `validate_open`/temporal substitutes.** `data_type_mandatory`
//! removes a mandatory RM field only, so the semantic constraint a case targets
//! (`DV_URI` RFC3986 validity `113`, `DV_EHR_URI` `ehr:` scheme `116`, `DV_PROPORTION`
//! kind invariants `060`, ISO8601 field-range opens) stands in on the
//! RM-mandatory dimension; the boundary comment records the untested headline
//! dimension. The temporal constraint/range cases (`097`…`108` non-open) assert
//! what master17.4 states via an authored `C_*` pattern/range; where the
//! validator still defers temporal enforcement the reject is a **reported
//! finding**, not a masked pass (register 13 open findings 097–108, 107
//! partial-value).
//!
//! **G-4 — edition/RM sensitivity.** `DV_SCALE` (`051`/`052`, interval-scale
//! `087`/`088`) needs **RM ≥ 1.1.0** (master17.3 §`DV_SCALE`, SPECRM-19); on an
//! RM < 1.1.0 SUT these are an edition finding, carried by the runner's
//! version ladder, not a fail. `C_DV_SCALE` does not exist in AM 1.4
//! (SPECPR-381), so `052` substitutes a `C_REAL` on `value`. Reject status
//! codes are pinned via [`super::drive`]'s ITS-REST ladder
//! (`422` semantic / `400` malformed) — never asserted here.
//!
//! **G-5 — zero-case types (recorded verbatim, no fabricated cases).** Four
//! `DATA_VALUE` types carry **no schedule test case** by explicit chapter NOTE
//! and therefore no ECC id; they are conformant-by-absence and stay case-free
//! until a CNF re-vendor adds cases:
//!
//! - master17.1 `=== DV_STATE` — NOTE: "not used and not supported by modeling
//!   tools".
//! - master17.2 `=== DV_PARAGRAPH` — NOTE: "not used or supported".
//! - master17.5 `=== DV_GENERAL_TIME_SPECIFICATION` — "TBD: this data type
//!   might not be used or supported by modeling tools".
//! - master17.5 `=== DV_PERIODIC_TIME_SPECIFICATION` — "TBD: this data type
//!   might not be used or supported by modeling tools".
//!
//! **G-6 — schedule defect: `CONT-DV_TEXT-validate_open` duplicated.**
//! master17.2 carries the heading `==== Test Case CONT-DV_TEXT-validate_open`
//! **twice** (the second is the `C_STRING.pattern XYZ` table): 81 headings, 80
//! distinct ids. Per standing rule 2 the defect is recorded, not silently
//! guessed — `ECC-VAL-044` folds both onto the RM-mandatory dimension, so the
//! second table's `C_STRING.pattern` rows are uncovered (noted on the case's
//! coverage bound). No 82nd ECC id is minted here.
//!
//! **G-7 — `DV_CODED_TEXT-validate_ext_term` binding substitution.**
//! `ECC-VAL-048` constrains with a direct external `C_CODE_PHRASE` (SNOMED-CT)
//! rather than the schedule's `CONSTRAINT_REF` → `ac`-code → template
//! `constraint_binding` path (master17.2 §`validate_ext_term` NOTE); functionally
//! close, but the binding-resolution surface is untested — a coverage note for
//! the terminology-binding work (register 11).

use openehr_its::opt14::{CPrimitive, OperationalTemplate};
use serde_json::{Value, json};

use crate::engine::assert;
use crate::engine::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::testdata::fixtures;

use super::author;
use super::drive::{self, Expected};
use super::mutate;

/// JSON-only: canonical-JSON validation is the wire path these cases exercise
/// (`composition_create`).
const JSON: &[Format] = &[Format::Json];

/// Every master17 case binds the same ITS-REST resource (register 13 meta).
const BINDING: &str = "POST /ehr/{ehr_id}/composition";

/// The `all_types` base OPT (relative to `valid_templates/`) — the authored
/// constraint base for master17 leaf cases.
const ALL_TYPES_OPT: &str = "all_types/Test_all_types.opt";

/// `Test_all_types.opt` + its bare canonical composition. OBSERVATION
/// `data/events[0]/data/items` carry one leaf per data type at fixed indices.
// The OWNED corrected copy (B2 register): the vendored corpus original's
// INSTRUCTION-activity `at0003` DV_DATE carries a day, which the OPT's
// C_DATE pattern `yyyy-??-XX` forbids (AOM 1.4 c_date: day disallowed) —
// committing the raw corpus file would fail its own accept row; the raw
// original stays under test as the byte-pinned `invalid/` negative.
const ALL_TYPES: drive::Constraint = drive::Constraint {
    opt_file: "all_types/Test_all_types.opt",
    comp: drive::CompBase::Key("owned.composition.all-types.valid"),
};

/// `Test_all_types_v2.opt` — `items[at0005]` (index 1) adds a `local`
/// `code_list` `{at0023, at0024}` on its `DV_CODED_TEXT`.
const ALL_TYPES_V2: drive::Constraint = drive::Constraint {
    opt_file: "all_types/Test_all_types_v2.opt",
    comp: drive::CompBase::Key("owned.composition.all-types-v2.valid"),
};

/// `minimal_action_2.opt` — ACTION `description/items[0]` `DV_PROPORTION`
/// constrained by a `C_INTEGER` list `{3,4}` on `type`.
const MINIMAL_ACTION_2_PROPORTION: drive::Constraint = drive::Constraint {
    opt_file: "minimal/minimal_action_2.opt",
    comp: drive::CompBase::InDir {
        dir_key: "template.valid",
        file: "minimal/minimal_action_2.instance.composition.json",
    },
};

/// A registered master17 case: fixed area/capability/binding/format, per-case
/// schedule trace + citation + run fn.
struct Def {
    /// The registration slug (bound to the ECC number in `ecc-catalog.tsv`).
    id: &'static str,
    /// The human title.
    title: &'static str,
    /// Spec citation (openEHR specs only).
    citation: &'static str,
    /// The abstract schedule trace.
    schedule: ScheduleTrace,
    /// The run fn.
    run: crate::engine::harness::CaseRun,
}

/// Map a [`fixtures::FixtureError`] onto a codec [`CaseError`].
fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// The canonical-JSON pointer to `items[idx]/value/<suffix>` in the `all_types`
/// OBSERVATION.
fn leaf_ptr(idx: usize, suffix: &str) -> String {
    format!("/content/0/data/events/0/data/items/{idx}/value/{suffix}")
}

/// The pointer to the whole `items[idx]/value` object.
fn value_ptr(idx: usize) -> String {
    format!("/content/0/data/events/0/data/items/{idx}/value")
}

/// The implemented master17.x cases — the register-13 spine, in schedule order.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    CASES
        .iter()
        .map(|d| CaseEntry {
            meta: CaseMeta {
                id: d.id,
                title: d.title,
                area: Area::Val,
                capability: Capability::ArchetypeValidation,
                formats: JSON,
                citation: d.citation,
                schedule: d.schedule,
                binding: Binding::Rest(BINDING),
                compare: Compare::None,
            },
            run: d.run,
        })
        .collect()
}

/// A schedule trace for a master17 case.
const fn sched(s: &'static str) -> ScheduleTrace {
    ScheduleTrace::Schedule(s)
}

/// The 81 registered cases (`ECC-VAL-039`…`119`), schedule order. `schedule_rows`
/// (the register §2 truth-table row count, G-2) is baked into each run fn.
const CASES: &[Def] = &[
    // ── 17.1 basic (039–043) ─────────────────────────────────────────────────
    Def {
        id: "val/dv-boolean-anything-allowed",
        title: "Validate DV_BOOLEAN — anything allowed",
        citation: "RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_BOOLEAN; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_BOOLEAN.anything_allowed (master17.1 §DV_BOOLEAN)"),
        run: open_dv_boolean,
    },
    Def {
        id: "val/dv-boolean-only-true-allowed",
        title: "Validate DV_BOOLEAN — only true allowed",
        citation: "RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_BOOLEAN {true}; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_BOOLEAN.only_true_allowed (master17.1 §DV_BOOLEAN)"),
        run: run_dv_boolean_true,
    },
    Def {
        id: "val/dv-boolean-only-false-allowed",
        title: "Validate DV_BOOLEAN — only false allowed",
        citation: "RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_BOOLEAN {false}; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_BOOLEAN.only_false_allowed (master17.1 §DV_BOOLEAN)"),
        run: run_dv_boolean_false,
    },
    Def {
        id: "val/dv-identifier-all-pattern",
        title: "Validate DV_IDENTIFIER — all pattern",
        citation: "RM 1.2.0 data_types §DV_IDENTIFIER; AM 1.4 C_STRING.pattern on id; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_IDENTIFIER.validate_all_pattern (master17.1 §DV_IDENTIFIER)"),
        run: run_dv_identifier_pattern,
    },
    Def {
        id: "val/dv-identifier-all-list",
        title: "Validate DV_IDENTIFIER — all list",
        citation: "RM 1.2.0 data_types §DV_IDENTIFIER; AM 1.4 C_STRING.list on id; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_IDENTIFIER.validate_all_list (master17.1 §DV_IDENTIFIER)"),
        run: run_dv_identifier_list,
    },
    // ── 17.2 text (044–048) ──────────────────────────────────────────────────
    Def {
        id: "val/dv-text-open",
        title: "Validate DV_TEXT — open",
        citation: "RM 1.2.0 data_types §DV_TEXT; AM 1.4 C_STRING open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_TEXT.validate_open (master17.2 §DV_TEXT; G-6: heading duplicated, 2nd C_STRING.pattern table folded)",
        ),
        run: open_dv_text,
    },
    Def {
        id: "val/dv-text-list",
        title: "Validate DV_TEXT — list",
        citation: "RM 1.2.0 data_types §DV_TEXT; AM 1.4 C_STRING.list; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_TEXT.validate_list (master17.2 §DV_TEXT)"),
        run: run_dv_text_list,
    },
    Def {
        id: "val/dv-coded-text-open",
        title: "Validate DV_CODED_TEXT — open",
        citation: "RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_CODE_PHRASE open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_CODED_TEXT.validate_open (master17.2 §DV_CODED_TEXT)"),
        run: open_dv_coded_text,
    },
    Def {
        id: "val/dv-coded-text-local-codes",
        title: "Validate DV_CODED_TEXT — local codes",
        citation: "RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_CODE_PHRASE local code_list; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_CODED_TEXT.validate_local_codes (master17.2 §DV_CODED_TEXT)"),
        run: run_dv_coded_local,
    },
    Def {
        id: "val/dv-coded-text-ext-term",
        title: "Validate DV_CODED_TEXT — ext term",
        citation: "RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_CODE_PHRASE external terminology; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_CODED_TEXT.validate_ext_term (master17.2 §DV_CODED_TEXT; G-7: direct C_CODE_PHRASE substitutes the CONSTRAINT_REF binding path)",
        ),
        run: run_dv_coded_ext_term,
    },
    // ── 17.3 quantity — scalars (049–067) ────────────────────────────────────
    Def {
        id: "val/dv-ordinal-open",
        title: "Validate DV_ORDINAL — open",
        citation: "RM 1.2.0 data_types §DV_ORDINAL; AM 1.4 C_DV_ORDINAL open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_ORDINAL.validate_open (master17.3 §DV_ORDINAL)"),
        run: open_dv_ordinal,
    },
    Def {
        id: "val/dv-ordinal-constraint",
        title: "Validate DV_ORDINAL — constraint",
        citation: "RM 1.2.0 data_types §DV_ORDINAL; AM 1.4 C_DV_ORDINAL.list; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_ORDINAL.validate_constraint (master17.3 §DV_ORDINAL)"),
        run: run_dv_ordinal_constraint,
    },
    Def {
        id: "val/dv-scale-open",
        title: "Validate DV_SCALE — open",
        citation: "RM 1.2.0 data_types §DV_SCALE (RM ≥ 1.1.0, SPECRM-19); AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_SCALE.validate_open (master17.3 §DV_SCALE; G-4: RM ≥ 1.1.0)"),
        run: run_dv_scale_open,
    },
    Def {
        id: "val/dv-scale-constraint",
        title: "Validate DV_SCALE — constraint",
        citation: "RM 1.2.0 data_types §DV_SCALE (RM ≥ 1.1.0); AM 1.4 C_REAL.list on value (no C_DV_SCALE in AM 1.4, SPECPR-381); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_SCALE.validate_constraint (master17.3 §DV_SCALE; G-4: RM ≥ 1.1.0, C_REAL substitute)",
        ),
        run: run_dv_scale_constraint,
    },
    Def {
        id: "val/dv-count-open",
        title: "Validate DV_COUNT — open",
        citation: "RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_INTEGER open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_COUNT.validate_open (master17.3 §DV_COUNT)"),
        run: open_dv_count,
    },
    Def {
        id: "val/dv-count-range",
        title: "Validate DV_COUNT — range",
        citation: "RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_INTEGER.range; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_COUNT.validate_range (master17.3 §DV_COUNT)"),
        run: run_dv_count_range,
    },
    Def {
        id: "val/dv-count-list",
        title: "Validate DV_COUNT — list",
        citation: "RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_INTEGER.list; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_COUNT.validate_list (master17.3 §DV_COUNT)"),
        run: run_dv_count_list,
    },
    Def {
        id: "val/dv-quantity-open",
        title: "Validate DV_QUANTITY — open",
        citation: "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_QUANTITY.validate_open (master17.3 §DV_QUANTITY)"),
        run: open_dv_quantity,
    },
    Def {
        id: "val/dv-quantity-property",
        title: "Validate DV_QUANTITY — property",
        citation: "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY.property; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_QUANTITY.validate_property (master17.3 §DV_QUANTITY)"),
        run: run_dv_quantity_property,
    },
    Def {
        id: "val/dv-quantity-property-units",
        title: "Validate DV_QUANTITY — property units",
        citation: "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY units list; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_QUANTITY.validate_property_units (master17.3 §DV_QUANTITY)"),
        run: run_dv_quantity_units,
    },
    Def {
        id: "val/dv-quantity-property-units-mag",
        title: "Validate DV_QUANTITY — property units mag",
        citation: "RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY units list + magnitude range; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_QUANTITY.validate_property_units_mag (master17.3 §DV_QUANTITY)"),
        run: run_dv_quantity_units_mag,
    },
    Def {
        id: "val/dv-proportion-open",
        title: "Validate DV_PROPORTION — open",
        citation: "RM 1.2.0 data_types §DV_PROPORTION (kind invariants); AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_PROPORTION.validate_open (master17.3 §DV_PROPORTION; G-3: 14 kind-invariant rejects untested, RM-mandatory numerator only)",
        ),
        run: open_dv_proportion,
    },
    Def {
        id: "val/dv-proportion-ratio",
        title: "Validate DV_PROPORTION — ratio",
        citation: "RM 1.2.0 data_types §DV_PROPORTION (ratio kind 0); AM 1.4 C_INTEGER.list on type; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_PROPORTION.validate_ratio (master17.3 §DV_PROPORTION)"),
        run: run_dv_proportion_ratio,
    },
    Def {
        id: "val/dv-proportion-unitary",
        title: "Validate DV_PROPORTION — unitary",
        citation: "RM 1.2.0 data_types §DV_PROPORTION (unitary kind 1); AM 1.4 C_INTEGER.list on type; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_PROPORTION.validate_unitary (master17.3 §DV_PROPORTION)"),
        run: run_dv_proportion_unitary,
    },
    Def {
        id: "val/dv-proportion-percent",
        title: "Validate DV_PROPORTION — percent",
        citation: "RM 1.2.0 data_types §DV_PROPORTION (percent kind 2); AM 1.4 C_INTEGER.list on type; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_PROPORTION.validate_percent (master17.3 §DV_PROPORTION)"),
        run: run_dv_proportion_percent,
    },
    Def {
        id: "val/dv-proportion-fraction",
        title: "Validate DV_PROPORTION — fraction",
        citation: "RM 1.2.0 data_types §DV_PROPORTION (fraction kind 3); AM 1.4 C_INTEGER.list on type; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_PROPORTION.validate_fraction (master17.3 §DV_PROPORTION)"),
        run: run_dv_proportion_fraction,
    },
    Def {
        id: "val/dv-proportion-integer-fraction",
        title: "Validate DV_PROPORTION — integer fraction",
        citation: "RM 1.2.0 data_types §DV_PROPORTION (integer_fraction kind 4); AM 1.4 C_INTEGER.list on type; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_PROPORTION.validate_integer_fraction (master17.3 §DV_PROPORTION)"),
        run: run_dv_proportion_integer_fraction,
    },
    Def {
        id: "val/dv-proportion-any-fraction",
        title: "Validate DV_PROPORTION — any fraction",
        citation: "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_INTEGER.list {3,4} on type; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_PROPORTION.validate_any_fraction (master17.3 §DV_PROPORTION)"),
        run: run_dv_proportion_any_fraction,
    },
    Def {
        id: "val/dv-proportion-ratio-range",
        title: "Validate DV_PROPORTION — ratio range",
        citation: "RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_REAL.range on numerator; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_PROPORTION.validate_ratio_range (master17.3 §DV_PROPORTION; denominator C_REAL.range table not driven)",
        ),
        run: run_dv_proportion_ratio_range,
    },
    // ── 17.3 quantity — DV_INTERVAL<T> (068–095, 28 ids; G-1 bound-declared) ──
    Def {
        id: "val/dv-interval-dv-count-open",
        title: "Validate DV_INTERVAL<DV_COUNT> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_COUNT.validate_open (master17.3 §DV_INTERVAL<DV_COUNT>; G-1: bound C_INTEGER constraint inexpressible → RM Interval invariant triple)",
        ),
        run: ivc_open,
    },
    Def {
        id: "val/dv-interval-dv-count-lower-upper",
        title: "Validate DV_INTERVAL<DV_COUNT> — lower upper",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_COUNT.validate_lower_upper (master17.3; G-1: bound constraint inexpressible → RM lower ≤ upper)",
        ),
        run: ivc_lu,
    },
    Def {
        id: "val/dv-interval-dv-count-lower-upper-list",
        title: "Validate DV_INTERVAL<DV_COUNT> — lower upper list",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_COUNT.validate_lower_upper_list (master17.3; G-1: C_INTEGER.list on bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivc_lul,
    },
    Def {
        id: "val/dv-interval-dv-quantity-open",
        title: "Validate DV_INTERVAL<DV_QUANTITY> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_QUANTITY>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_QUANTITY.validate_open (master17.3; G-1: bound C_DV_QUANTITY.list inexpressible → RM Interval invariant triple)",
        ),
        run: ivq_open,
    },
    Def {
        id: "val/dv-interval-dv-quantity-upper-lower",
        title: "Validate DV_INTERVAL<DV_QUANTITY> — upper lower",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_QUANTITY>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_QUANTITY.validate_upper_lower (master17.3; G-1: bound constraint inexpressible → RM lower ≤ upper)",
        ),
        run: ivq_ul,
    },
    Def {
        id: "val/dv-interval-dv-date-time-open",
        title: "Validate DV_INTERVAL<DV_DATE_TIME> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DATE_TIME.validate_open (master17.3; G-1: temporal bound inexpressible → RM Interval invariant triple)",
        ),
        run: ivdt_open,
    },
    Def {
        id: "val/dv-interval-dv-date-time-lower-upper-constraint",
        title: "Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DATE_TIME.validate_lower_upper_constraint (master17.3, 68-row table; G-1: C_DATE_TIME bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivdt_luc,
    },
    Def {
        id: "val/dv-interval-dv-date-time-lower-upper-range",
        title: "Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DATE_TIME.validate_lower_upper_range (master17.3; G-1: C_DATE_TIME.range bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivdt_lur,
    },
    Def {
        id: "val/dv-interval-dv-date-open",
        title: "Validate DV_INTERVAL<DV_DATE> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DATE.validate_open (master17.3; G-1: temporal bound inexpressible → RM Interval invariant triple)",
        ),
        run: ivd_open,
    },
    Def {
        id: "val/dv-interval-dv-date-lower-upper-constraint",
        title: "Validate DV_INTERVAL<DV_DATE> — lower upper constraint",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DATE.validate_lower_upper_constraint (master17.3; G-1: C_DATE bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivd_luc,
    },
    Def {
        id: "val/dv-interval-dv-date-lower-upper-range",
        title: "Validate DV_INTERVAL<DV_DATE> — lower upper range",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DATE.validate_lower_upper_range (master17.3; G-1: C_DATE.range bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivd_lur,
    },
    Def {
        id: "val/dv-interval-dv-time-open",
        title: "Validate DV_INTERVAL<DV_TIME> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_TIME.validate_open (master17.3; G-1: temporal bound inexpressible → RM Interval invariant triple)",
        ),
        run: ivt_open,
    },
    Def {
        id: "val/dv-interval-dv-time-lower-upper-constraint",
        title: "Validate DV_INTERVAL<DV_TIME> — lower upper constraint",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_TIME.validate_lower_upper_constraint (master17.3; G-1: C_TIME bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivt_luc,
    },
    Def {
        id: "val/dv-interval-dv-time-lower-upper-range",
        title: "Validate DV_INTERVAL<DV_TIME> — lower upper range",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_TIME.validate_lower_upper_range (master17.3; G-1: C_TIME.range bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivt_lur,
    },
    Def {
        id: "val/dv-interval-dv-duration-open",
        title: "Validate DV_INTERVAL<DV_DURATION> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DURATION.validate_open (master17.3; G-1: temporal bound inexpressible → RM Interval invariant triple)",
        ),
        run: ivdu_open,
    },
    Def {
        id: "val/dv-interval-dv-duration-constraint",
        title: "Validate DV_INTERVAL<DV_DURATION> — constraint",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DURATION.validate_constraint (master17.3, 35-row table; G-1: C_DURATION bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivdu_c,
    },
    Def {
        id: "val/dv-interval-dv-duration-range",
        title: "Validate DV_INTERVAL<DV_DURATION> — range",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_DURATION.validate_range (master17.3; G-1: C_DURATION.range bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivdu_r,
    },
    Def {
        id: "val/dv-interval-dv-ordinal-open",
        title: "Validate DV_INTERVAL<DV_ORDINAL> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_ORDINAL>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_ORDINAL.validate_open (master17.3; G-1: bound constraint inexpressible → RM Interval invariant triple)",
        ),
        run: ivo_open,
    },
    Def {
        id: "val/dv-interval-dv-ordinal-constraint",
        title: "Validate DV_INTERVAL<DV_ORDINAL> — constraint",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_ORDINAL>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_ORDINAL.validate_constraint (master17.3; G-1: C_DV_ORDINAL bounds inexpressible → RM lower ≤ upper)",
        ),
        run: ivo_c,
    },
    Def {
        id: "val/dv-interval-dv-scale-open",
        title: "Validate DV_INTERVAL<DV_SCALE> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_SCALE> (RM ≥ 1.1.0); BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_SCALE.validate_open (master17.3; G-4: RM ≥ 1.1.0; G-1: bound inexpressible → RM Interval invariant triple)",
        ),
        run: ivs_open,
    },
    Def {
        id: "val/dv-interval-dv-scale-constraint",
        title: "Validate DV_INTERVAL<DV_SCALE> — constraint",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_SCALE> (RM ≥ 1.1.0); BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_SCALE.validate_constraint (master17.3; G-4: RM ≥ 1.1.0; G-1: bound inexpressible → RM lower ≤ upper)",
        ),
        run: ivs_c,
    },
    Def {
        id: "val/dv-interval-dv-proportion-open",
        title: "Validate DV_INTERVAL<DV_PROPORTION> — open",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval invariants; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_PROPORTION.validate_open (master17.3, 18-row table; G-1: bound inexpressible → RM Interval invariant triple)",
        ),
        run: ivp_open,
    },
    Def {
        id: "val/dv-interval-dv-proportion-ratio",
        title: "Validate DV_INTERVAL<DV_PROPORTION> — ratio",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_PROPORTION.validate_ratio (master17.3, 12-row table; G-1: proportion-kind bound inexpressible → RM lower ≤ upper)",
        ),
        run: ivp_ratio,
    },
    Def {
        id: "val/dv-interval-dv-proportion-unitary",
        title: "Validate DV_INTERVAL<DV_PROPORTION> — unitary",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_PROPORTION.validate_unitary (master17.3, 12-row table; G-1: proportion-kind bound inexpressible → RM lower ≤ upper)",
        ),
        run: ivp_unitary,
    },
    Def {
        id: "val/dv-interval-dv-proportion-percentage",
        title: "Validate DV_INTERVAL<DV_PROPORTION> — percentage",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_PROPORTION.validate_percentage (master17.3, 12-row table; G-1: proportion-kind bound inexpressible → RM lower ≤ upper)",
        ),
        run: ivp_percent,
    },
    Def {
        id: "val/dv-interval-dv-proportion-fraction",
        title: "Validate DV_INTERVAL<DV_PROPORTION> — fraction",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_PROPORTION.validate_fraction (master17.3, 12-row table; G-1: proportion-kind bound inexpressible → RM lower ≤ upper)",
        ),
        run: ivp_fraction,
    },
    Def {
        id: "val/dv-interval-dv-proportion-integer-fraction",
        title: "Validate DV_INTERVAL<DV_PROPORTION> — integer fraction",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_PROPORTION.validate_integer_fraction (master17.3, 12-row table; G-1: proportion-kind bound inexpressible → RM lower ≤ upper)",
        ),
        run: ivp_intfrac,
    },
    Def {
        id: "val/dv-interval-dv-proportion-ratio-range",
        title: "Validate DV_INTERVAL<DV_PROPORTION> — ratio range",
        citation: "RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_INTERVAL_DV_PROPORTION.validate_ratio_range (master17.3, 18-row table; G-1: proportion-kind bound inexpressible → RM lower ≤ upper)",
        ),
        run: ivp_ratiorange,
    },
    // ── 17.4 date_time (096–108, + 119) ──────────────────────────────────────
    Def {
        id: "val/dv-duration-open",
        title: "Validate DV_DURATION — open",
        citation: "RM 1.2.0 data_types §DV_DURATION; AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_DURATION.validate_open (master17.4 §DV_DURATION)"),
        run: open_dv_duration,
    },
    Def {
        id: "val/dv-duration-fields",
        title: "Validate DV_DURATION — fields",
        citation: "RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_DURATION.pattern (allowed fields); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DURATION.validate_fields (master17.4 §DV_DURATION; G-3 open finding: temporal enforcement — SUT reject reported, never masked)",
        ),
        run: run_dv_duration_fields,
    },
    Def {
        id: "val/dv-duration-range",
        title: "Validate DV_DURATION — range",
        citation: "RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_DURATION.range; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DURATION.validate_range (master17.4 §DV_DURATION; G-3 open finding: temporal enforcement)",
        ),
        run: run_dv_duration_range,
    },
    Def {
        id: "val/dv-duration-fields-range",
        title: "Validate DV_DURATION — fields range",
        citation: "RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_DURATION.pattern + range; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DURATION.validate_fields_range (master17.4 §DV_DURATION; G-3 open finding: temporal enforcement)",
        ),
        run: run_dv_duration_fields_range,
    },
    Def {
        id: "val/dv-time-open",
        title: "Validate DV_TIME — open",
        citation: "RM 1.2.0 data_types §DV_TIME; AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_TIME.validate_open (master17.4 §DV_TIME; G-3: ISO8601-validity rows not driven, RM-mandatory value only)",
        ),
        run: open_dv_time,
    },
    Def {
        id: "val/dv-time-constraint",
        title: "Validate DV_TIME — constraint",
        citation: "RM 1.2.0 data_types §DV_TIME; AM 1.4 C_TIME.pattern; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_TIME.validate_constraint (master17.4 §DV_TIME, 70-row table; G-3 open finding: temporal enforcement)",
        ),
        run: run_dv_time_constraint,
    },
    Def {
        id: "val/dv-time-range",
        title: "Validate DV_TIME — range",
        citation: "RM 1.2.0 data_types §DV_TIME; AM 1.4 C_TIME.range; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_TIME.validate_range (master17.4 §DV_TIME, 200-row table — largest; G-2/G-3 open finding: temporal enforcement)",
        ),
        run: run_dv_time_range,
    },
    Def {
        id: "val/dv-date-open",
        title: "Validate DV_DATE — open",
        citation: "RM 1.2.0 data_types §DV_DATE; AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DATE.validate_open (master17.4 §DV_DATE; G-3: ISO8601-validity rows not driven, RM-mandatory value only)",
        ),
        run: open_dv_date,
    },
    Def {
        id: "val/dv-date-constraint",
        title: "Validate DV_DATE — constraint",
        citation: "RM 1.2.0 data_types §DV_DATE; AM 1.4 C_DATE.pattern; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DATE.validate_constraint (master17.4 §DV_DATE; G-3 open finding: temporal enforcement)",
        ),
        run: run_dv_date_constraint,
    },
    Def {
        id: "val/dv-date-range",
        title: "Validate DV_DATE — range",
        citation: "RM 1.2.0 data_types §DV_DATE; AM 1.4 C_DATE.range; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DATE.validate_range (master17.4 §DV_DATE; G-3 open finding: temporal enforcement)",
        ),
        run: run_dv_date_range,
    },
    Def {
        id: "val/dv-date-day-disallowed-pattern",
        title: "Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected)",
        citation: "AM 1.4 C_DATE (yyyy-??-XX: month optional, day disallowed; org.openehr.am.aom14.c_date.adoc); ITS-REST 1.0.3 composition_create (422 rejected)",
        schedule: ScheduleTrace::EccOriginal(
            "no schedule case — ECC-authored negative guard for the corrected all_types fixture (§3, testdata/fixtures/REGISTER.md); the vendored all_types.composition.json carries a day-bearing DV_DATE at a leaf whose OPT C_DATE pattern disallows the day; a spec-correct validator must 422 it (archie is lenient)",
        ),
        run: run_dv_date_day_disallowed,
    },
    Def {
        id: "val/dv-date-time-open",
        title: "Validate DV_DATE_TIME — open",
        citation: "RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DATE_TIME.validate_open (master17.4 §DV_DATE_TIME; RM-mandatory value only)",
        ),
        run: open_dv_date_time,
    },
    Def {
        id: "val/dv-date-time-constraint",
        title: "Validate DV_DATE_TIME — constraint",
        citation: "RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 C_DATE_TIME.pattern (yyyy-mm-ddTHH:MM:SS); ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DATE_TIME.validate_constraint (master17.4 §DV_DATE_TIME, 176-row table; G-3 explicit open finding: SUT accepts the partial value the table rejects)",
        ),
        run: run_dv_date_time_constraint,
    },
    Def {
        id: "val/dv-date-time-range",
        title: "Validate DV_DATE_TIME — range",
        citation: "RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 C_DATE_TIME.range; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_DATE_TIME.validate_range (master17.4 §DV_DATE_TIME; G-3 open finding: temporal enforcement)",
        ),
        run: run_dv_date_time_range,
    },
    // ── 17.6 encapsulated (109–112) ──────────────────────────────────────────
    Def {
        id: "val/dv-parsable-open",
        title: "Validate DV_PARSABLE — open",
        citation: "RM 1.2.0 data_types §DV_PARSABLE; AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_PARSABLE.validate_open (master17.6 §DV_PARSABLE; formalism-mandatory row not driven)",
        ),
        run: open_dv_parsable,
    },
    Def {
        id: "val/dv-parsable-value-formalism",
        title: "Validate DV_PARSABLE — value formalism",
        citation: "RM 1.2.0 data_types §DV_PARSABLE; AM 1.4 C_STRING.list on formalism; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_PARSABLE.validate_value_formalism (master17.6 §DV_PARSABLE; value C_STRING rows not driven)",
        ),
        run: run_dv_parsable_formalism,
    },
    Def {
        id: "val/dv-multimedia-open",
        title: "Validate DV_MULTIMEDIA — open",
        citation: "RM 1.2.0 data_types §DV_MULTIMEDIA; AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_MULTIMEDIA.validate_open (master17.6 §DV_MULTIMEDIA; size-mandatory + media-type-codeset rows not driven)",
        ),
        run: open_dv_multimedia,
    },
    Def {
        id: "val/dv-multimedia-media-type",
        title: "Validate DV_MULTIMEDIA — media type",
        citation: "RM 1.2.0 data_types §DV_MULTIMEDIA; AM 1.4 C_CODE_PHRASE on media_type; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_MULTIMEDIA.validate_media_type (master17.6 §DV_MULTIMEDIA; size C_INTEGER half of the table not driven)",
        ),
        run: run_dv_multimedia_media_type,
    },
    // ── 17.7 uri (113–118) ───────────────────────────────────────────────────
    Def {
        id: "val/dv-uri-open",
        title: "Validate DV_URI — open",
        citation: "RM 1.2.0 data_types §DV_URI (RFC3986 validity); AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_URI.validate_open (master17.7 §DV_URI; G-3: headline is RFC3986 validity, ECC drives RM-mandatory value only)",
        ),
        run: open_dv_uri,
    },
    Def {
        id: "val/dv-uri-pattern",
        title: "Validate DV_URI — pattern",
        citation: "RM 1.2.0 data_types §DV_URI; AM 1.4 C_STRING.pattern; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_URI.validate_pattern (master17.7 §DV_URI)"),
        run: run_dv_uri_pattern,
    },
    Def {
        id: "val/dv-uri-list",
        title: "Validate DV_URI — list",
        citation: "RM 1.2.0 data_types §DV_URI; AM 1.4 C_STRING.list; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_URI.validate_list (master17.7 §DV_URI)"),
        run: run_dv_uri_list,
    },
    Def {
        id: "val/dv-ehr-uri-open",
        title: "Validate DV_EHR_URI — open",
        citation: "RM 1.2.0 data_types §DV_EHR_URI (ehr: scheme); AM 1.4 open; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched(
            "DV_EHR_URI.validate_open (master17.7 §DV_EHR_URI; G-3: headline is the ehr: scheme rule, ECC drives RM-mandatory value only)",
        ),
        run: run_dv_ehr_uri_open,
    },
    Def {
        id: "val/dv-ehr-uri-pattern",
        title: "Validate DV_EHR_URI — pattern",
        citation: "RM 1.2.0 data_types §DV_EHR_URI; AM 1.4 C_STRING.pattern; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_EHR_URI.validate_pattern (master17.7 §DV_EHR_URI)"),
        run: run_dv_ehr_uri_pattern,
    },
    Def {
        id: "val/dv-ehr-uri-list",
        title: "Validate DV_EHR_URI — list",
        citation: "RM 1.2.0 data_types §DV_EHR_URI; AM 1.4 C_STRING.list; ITS-REST 1.0.3 composition_create (201/422)",
        schedule: sched("DV_EHR_URI.validate_list (master17.7 §DV_EHR_URI)"),
        run: run_dv_ehr_uri_list,
    },
];

// ── local authored-leaf drivers ───────────────────────────────────────────────

/// One authored-leaf table row: `(label, [(pointer, value)] mutations, expected)`.
type LeafRow = (String, Vec<(String, Value)>, Expected);

/// Drive a master17 leaf case by authoring `constrain` into the `all_types` OPT,
/// then committing the base composition (its vendored leaf satisfies the
/// constraint → accepted) and a copy with the leaf pushed out of the constraint
/// (rejected). The accept/reject is the SUT's genuine validation of a real
/// authored template (design §4.5), never a fabricated pass.
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

    let base = drive::Base::AllTypes.load()?;
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

/// A flexible authored-leaf driver: author `constrain` into the `all_types` OPT,
/// then commit one composition per `rows` entry (a clone of the base with the
/// listed `(pointer, value)` mutations applied), asserting its `Expected`.
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
    let base = drive::Base::AllTypes.load()?;
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

// ── validate_open (RM/Schema-mandatory) run fns ───────────────────────────────
//
// Each drives the value type's mandatory field on `Base::AllTypes`, self-skipping
// when the corpus carries no such leaf (`data_type_mandatory`); the `of_schedule_rows`
// literal is the register §2 truth-table row count (G-2). DV_SCALE / DV_EHR_URI
// have no all_types leaf and are handled by dedicated retyping run fns below.
macro_rules! open_case {
    ($fn:ident, $ty:literal, $field:literal, $rows:expr) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move {
                Ok(
                    drive::data_type_mandatory(ctx, drive::Base::AllTypes, $ty, $field)
                        .await?
                        .of_schedule_rows($rows),
                )
            })
        }
    };
}

open_case!(open_dv_boolean, "DV_BOOLEAN", "value", 2);
open_case!(open_dv_text, "DV_TEXT", "value", 3);
open_case!(open_dv_coded_text, "DV_CODED_TEXT", "defining_code", 5);
open_case!(open_dv_ordinal, "DV_ORDINAL", "value", 5);
open_case!(open_dv_count, "DV_COUNT", "magnitude", 5);
open_case!(open_dv_quantity, "DV_QUANTITY", "magnitude", 7);
open_case!(open_dv_proportion, "DV_PROPORTION", "numerator", 19);
open_case!(open_dv_duration, "DV_DURATION", "value", 14);
open_case!(open_dv_time, "DV_TIME", "value", 23);
open_case!(open_dv_date, "DV_DATE", "value", 10);
open_case!(open_dv_date_time, "DV_DATE_TIME", "value", 29);
open_case!(open_dv_parsable, "DV_PARSABLE", "value", 4);
open_case!(open_dv_multimedia, "DV_MULTIMEDIA", "media_type", 4);
open_case!(open_dv_uri, "DV_URI", "value", 13);

// ── 17.1 DV_BOOLEAN true-only / false-only ────────────────────────────────────

fn run_dv_boolean_true<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(10, "value");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(2))
    })
}

fn run_dv_boolean_false<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(10, "value");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(2))
    })
}

// ── 17.1 DV_IDENTIFIER — C_STRING pattern / list on id ────────────────────────

fn run_dv_identifier_pattern<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(14, "id");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(12))
    })
}

fn run_dv_identifier_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(14, "id");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(12))
    })
}

// ── 17.2 DV_TEXT / DV_CODED_TEXT ──────────────────────────────────────────────

fn run_dv_text_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
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
        Ok(drive_leaf(
            ctx,
            "cnf_cont_dv_text_list",
            move |opt| author::constrain_leaf_string(opt, "DV_TEXT", "value", None, list),
            "DV_TEXT value in the C_STRING list (accepted)",
            "DV_TEXT value not in the C_STRING list (C_STRING.list)".to_owned(),
            &ptr,
            json!("cnf-not-in-list-value"),
        )
        .await?
        .of_schedule_rows(3))
    })
}

fn run_dv_coded_local<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive::drive_constraint(
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
        .await?
        .of_schedule_rows(5))
    })
}

fn run_dv_coded_ext_term<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let code = leaf_ptr(1, "defining_code/code_string");
        let term = leaf_ptr(1, "defining_code/terminology_id/value");
        Ok(drive_leaf_rows(
            ctx,
            "cnf_dv_coded_ext_term",
            |opt| {
                // Pinned to ELEMENT at0005 (the OBSERVATION's DV_CODED_TEXT leaf this
                // case mutates); the blanket first-match hits the COMPOSITION category.
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
        .await?
        .of_schedule_rows(5))
    })
}

// ── 17.3 DV_ORDINAL / DV_COUNT ────────────────────────────────────────────────

fn run_dv_ordinal_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive::drive_constraint(
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
        .await?
        .of_schedule_rows(3))
    })
}

fn run_dv_count_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_leaf(
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
        .await?
        .of_schedule_rows(5))
    })
}

fn run_dv_count_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_leaf(
            ctx,
            "cnf_cont_dv_count_list",
            |opt| author::constrain_leaf_integer(opt, "DV_COUNT", "magnitude", None, vec![3]),
            "DV_COUNT magnitude 3 in the C_INTEGER list {3} (accepted)",
            "DV_COUNT magnitude 7 not in the C_INTEGER list {3} (C_INTEGER.list)".to_owned(),
            &leaf_ptr(4, "magnitude"),
            json!(7),
        )
        .await?
        .of_schedule_rows(5))
    })
}

// ── 17.3 DV_QUANTITY ──────────────────────────────────────────────────────────

fn run_dv_quantity_property<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let u = leaf_ptr(3, "units");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(8))
    })
}

fn run_dv_quantity_units<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive::drive_constraint(
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
        .await?
        .of_schedule_rows(9))
    })
}

fn run_dv_quantity_units_mag<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        // The time_series OPT constrains a DV_QUANTITY magnitude range [0,inf) (units
        // {mm3}); its only committable instance is FLAT, converted in-harness.
        let base =
            fixtures::flat_to_canonical("template.time-series.opt", "composition.flat.time-series")
                .map_err(|e| codec(&e))?;
        Ok(drive::drive_constraint_base(
            ctx,
            "template.valid",
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
        .await?
        .of_schedule_rows(9))
    })
}

// ── 17.3 DV_PROPORTION ────────────────────────────────────────────────────────

/// Drive one `DV_PROPORTION` `type`-kind case: constrain `type` to the single kind
/// code, commit an accepted instance (that kind with RM-valid num/den) and a
/// rejected instance (an off-list kind).
async fn drive_proportion_kind(
    ctx: &RunContext<'_>,
    tid: &'static str,
    kind: i32,
    num: Value,
    den: Value,
) -> Result<DataSetReport, CaseError> {
    let ty = leaf_ptr(15, "type");
    let n = leaf_ptr(15, "numerator");
    let d = leaf_ptr(15, "denominator");
    drive_leaf_rows(
        ctx,
        tid,
        move |opt| author::constrain_leaf_integer(opt, "DV_PROPORTION", "type", None, vec![kind]),
        vec![
            (
                format!("type {kind} in list {{{kind}}} with RM-valid num/den (accepted)"),
                vec![(ty.clone(), json!(kind)), (n, num), (d, den)],
                Expected::Accepted,
            ),
            (
                // Off-list kind: 0 (ratio) for the non-ratio cases; for the ratio
                // case itself 0 IS permitted, so use 2 (percent).
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
}

fn run_dv_proportion_ratio<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(
            drive_proportion_kind(ctx, "cnf_cont_dv_prop_ratio", 0, json!(398.5), json!(209.2))
                .await?
                .of_schedule_rows(5),
        )
    })
}
fn run_dv_proportion_unitary<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(
            drive_proportion_kind(ctx, "cnf_cont_dv_prop_unitary", 1, json!(5.0), json!(1.0))
                .await?
                .of_schedule_rows(5),
        )
    })
}
fn run_dv_proportion_percent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_proportion_kind(
            ctx,
            "cnf_cont_dv_prop_percent",
            2,
            json!(42.0),
            json!(100.0),
        )
        .await?
        .of_schedule_rows(5))
    })
}
fn run_dv_proportion_fraction<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(
            drive_proportion_kind(ctx, "cnf_cont_dv_prop_fraction", 3, json!(3.0), json!(4.0))
                .await?
                .of_schedule_rows(5),
        )
    })
}
fn run_dv_proportion_integer_fraction<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_proportion_kind(
            ctx,
            "cnf_cont_dv_prop_int_fraction",
            4,
            json!(3.0),
            json!(4.0),
        )
        .await?
        .of_schedule_rows(5))
    })
}

fn run_dv_proportion_any_fraction<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive::drive_constraint(
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
        .await?
        .of_schedule_rows(5))
    })
}

fn run_dv_proportion_ratio_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let n = leaf_ptr(15, "numerator");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(4))
    })
}

// ── 17.3 DV_SCALE (retyped scratch leaf; G-4 RM ≥ 1.1.0) ──────────────────────

/// A `DV_SCALE` value with a coded symbol.
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
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(5))
    })
}

fn run_dv_scale_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(3))
    })
}

// ── 17.3 DV_INTERVAL<T> — G-1 bound-declared RM-invariant probes ──────────────
//
// No base composition carries a DV_INTERVAL<T> leaf, so each case slot-retypes the
// DV_COUNT scratch slot (items[4]) to an open DV_INTERVAL. The per-variant bound
// constraint is inexpressible with the carried authoring machinery (see the module
// G-1 note), so the assertion is the RM Interval invariant subset that holds for
// every bound type. `_open` cases drive the three universal Interval invariants;
// the other cases drive lower ≤ upper. Every case declares its schedule row count.

/// A canonical bounded, included `DV_INTERVAL`.
fn iv(lower: &Value, upper: &Value) -> Value {
    json!({ "_type": "DV_INTERVAL", "lower": lower, "upper": upper,
        "lower_included": true, "upper_included": true,
        "lower_unbounded": false, "upper_unbounded": false })
}

/// An interval violating RM Interval `lower_included_valid`
/// (`lower_unbounded implies not lower_included`; BASE `foundation_types` §Interval).
fn iv_lower_unbounded_included(upper: &Value) -> Value {
    json!({ "_type": "DV_INTERVAL", "upper": upper,
        "lower_included": true, "upper_included": true,
        "lower_unbounded": true, "upper_unbounded": false })
}

/// An interval violating RM Interval `upper_included_valid`
/// (`upper_unbounded implies not upper_included`; BASE `foundation_types` §Interval).
fn iv_upper_unbounded_included(lower: &Value) -> Value {
    json!({ "_type": "DV_INTERVAL", "lower": lower,
        "lower_included": true, "upper_included": true,
        "lower_unbounded": false, "upper_unbounded": true })
}

/// Retype items[4] to an open `DV_INTERVAL`; drive the three universal RM Interval
/// invariants (`lower ≤ upper`, `lower_included_valid`, `upper_included_valid`).
async fn drive_interval_open(
    ctx: &RunContext<'_>,
    tid: &'static str,
    lower: Value,
    upper: Value,
) -> Result<DataSetReport, CaseError> {
    let p = value_ptr(4);
    drive_leaf_rows(
        ctx,
        tid,
        |opt| author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_INTERVAL")),
        vec![
            (
                "valid DV_INTERVAL, bounded + included, lower<=upper (accepted)".to_owned(),
                vec![(p.clone(), iv(&lower, &upper))],
                Expected::Accepted,
            ),
            (
                "reversed DV_INTERVAL lower>upper (RM Interval lower<=upper)".to_owned(),
                vec![(p.clone(), iv(&upper, &lower))],
                Expected::Rejected,
            ),
            (
                "lower_unbounded with lower_included=true (RM Interval lower_included_valid)"
                    .to_owned(),
                vec![(p.clone(), iv_lower_unbounded_included(&upper))],
                Expected::Rejected,
            ),
            (
                "upper_unbounded with upper_included=true (RM Interval upper_included_valid)"
                    .to_owned(),
                vec![(p, iv_upper_unbounded_included(&lower))],
                Expected::Rejected,
            ),
        ],
    )
    .await
}

/// Retype items[4] to an open `DV_INTERVAL`; drive the RM Interval `lower ≤ upper`
/// invariant (accept `[l,u]`, reject the reversed `[u,l]`).
async fn drive_interval_bound(
    ctx: &RunContext<'_>,
    tid: &'static str,
    lower: Value,
    upper: Value,
) -> Result<DataSetReport, CaseError> {
    let p = value_ptr(4);
    drive_leaf_rows(
        ctx,
        tid,
        |opt| author::retype_leaf(opt, "DV_COUNT", author::open_complex("DV_INTERVAL")),
        vec![
            (
                "valid DV_INTERVAL lower<=upper (accepted)".to_owned(),
                vec![(p.clone(), iv(&lower, &upper))],
                Expected::Accepted,
            ),
            (
                "reversed DV_INTERVAL lower>upper (RM Interval lower<=upper)".to_owned(),
                vec![(p, iv(&upper, &lower))],
                Expected::Rejected,
            ),
        ],
    )
    .await
}

// Bound-type value constructors.
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
    dv_scale_value(v, code)
}
fn dv_proportion(n: f64, d: f64) -> Value {
    json!({ "_type": "DV_PROPORTION", "numerator": n, "denominator": d, "type": 0 })
}

/// Generate an interval `_open` run fn (three RM Interval invariants).
macro_rules! iv_open {
    ($fn:ident, $tid:literal, $rows:expr, $lo:expr, $hi:expr) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move {
                Ok(drive_interval_open(ctx, $tid, $lo, $hi)
                    .await?
                    .of_schedule_rows($rows))
            })
        }
    };
}

/// Generate an interval non-open run fn (RM Interval `lower ≤ upper`; bound
/// constraint declared, not asserted — G-1).
macro_rules! iv_bound {
    ($fn:ident, $tid:literal, $rows:expr, $lo:expr, $hi:expr) => {
        fn $fn<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move {
                Ok(drive_interval_bound(ctx, $tid, $lo, $hi)
                    .await?
                    .of_schedule_rows($rows))
            })
        }
    };
}

// `validate_open` interval cases — the invariant triple.
iv_open!(ivc_open, "cnf_iv_count_open", 12, dv_count(1), dv_count(10));
iv_open!(
    ivq_open,
    "cnf_iv_quantity_open",
    10,
    dv_quantity(1.0),
    dv_quantity(10.0)
);
iv_open!(
    ivdt_open,
    "cnf_iv_datetime_open",
    27,
    dv_date_time("2021-01-01T00:00:00"),
    dv_date_time("2021-12-31T00:00:00")
);
iv_open!(
    ivd_open,
    "cnf_iv_date_open",
    8,
    dv_date("2021-01-01"),
    dv_date("2021-12-31")
);
iv_open!(
    ivt_open,
    "cnf_iv_time_open",
    8,
    dv_time("01:00:00"),
    dv_time("10:00:00")
);
iv_open!(
    ivdu_open,
    "cnf_iv_duration_open",
    9,
    dv_duration("PT1H"),
    dv_duration("PT10H")
);
iv_open!(
    ivo_open,
    "cnf_iv_ordinal_open",
    6,
    dv_ordinal(0, "at0014"),
    dv_ordinal(1, "at0015")
);
iv_open!(
    ivs_open,
    "cnf_iv_scale_open",
    6,
    dv_scale(1.0, "at0014"),
    dv_scale(2.0, "at0015")
);
iv_open!(
    ivp_open,
    "cnf_iv_proportion_open",
    18,
    dv_proportion(1.0, 2.0),
    dv_proportion(3.0, 2.0)
);

// Non-open interval cases — lower ≤ upper only.
iv_bound!(ivc_lu, "cnf_iv_count_lu", 7, dv_count(1), dv_count(10));
iv_bound!(ivc_lul, "cnf_iv_count_lul", 7, dv_count(1), dv_count(10));
iv_bound!(
    ivq_ul,
    "cnf_iv_quantity_ul",
    7,
    dv_quantity(1.0),
    dv_quantity(10.0)
);
iv_bound!(
    ivdt_luc,
    "cnf_iv_datetime_luc",
    68,
    dv_date_time("2021-01-01T00:00:00"),
    dv_date_time("2021-12-31T00:00:00")
);
iv_bound!(
    ivdt_lur,
    "cnf_iv_datetime_lur",
    24,
    dv_date_time("2021-01-01T00:00:00"),
    dv_date_time("2021-12-31T00:00:00")
);
iv_bound!(
    ivd_luc,
    "cnf_iv_date_luc",
    29,
    dv_date("2021-01-01"),
    dv_date("2021-12-31")
);
iv_bound!(
    ivd_lur,
    "cnf_iv_date_lur",
    4,
    dv_date("2021-01-01"),
    dv_date("2021-12-31")
);
iv_bound!(
    ivt_luc,
    "cnf_iv_time_luc",
    5,
    dv_time("01:00:00"),
    dv_time("10:00:00")
);
iv_bound!(
    ivt_lur,
    "cnf_iv_time_lur",
    9,
    dv_time("01:00:00"),
    dv_time("10:00:00")
);
iv_bound!(
    ivdu_c,
    "cnf_iv_duration_c",
    35,
    dv_duration("PT1H"),
    dv_duration("PT10H")
);
iv_bound!(
    ivdu_r,
    "cnf_iv_duration_r",
    10,
    dv_duration("PT1H"),
    dv_duration("PT10H")
);
iv_bound!(
    ivo_c,
    "cnf_iv_ordinal_c",
    7,
    dv_ordinal(0, "at0014"),
    dv_ordinal(1, "at0015")
);
iv_bound!(
    ivs_c,
    "cnf_iv_scale_c",
    7,
    dv_scale(1.0, "at0014"),
    dv_scale(2.0, "at0015")
);
iv_bound!(
    ivp_ratio,
    "cnf_iv_proportion_ratio",
    12,
    dv_proportion(1.0, 2.0),
    dv_proportion(3.0, 2.0)
);
iv_bound!(
    ivp_unitary,
    "cnf_iv_proportion_unitary",
    12,
    dv_proportion(1.0, 2.0),
    dv_proportion(3.0, 2.0)
);
iv_bound!(
    ivp_percent,
    "cnf_iv_proportion_percent",
    12,
    dv_proportion(1.0, 2.0),
    dv_proportion(3.0, 2.0)
);
iv_bound!(
    ivp_fraction,
    "cnf_iv_proportion_fraction",
    12,
    dv_proportion(1.0, 2.0),
    dv_proportion(3.0, 2.0)
);
iv_bound!(
    ivp_intfrac,
    "cnf_iv_proportion_intfrac",
    12,
    dv_proportion(1.0, 2.0),
    dv_proportion(3.0, 2.0)
);
iv_bound!(
    ivp_ratiorange,
    "cnf_iv_proportion_ratiorange",
    18,
    dv_proportion(1.0, 2.0),
    dv_proportion(3.0, 2.0)
);

// ── 17.4 temporal value constraints (G-3 open findings until temporal enforced) ─
//
// Base leaves: DV_DATE items[5], DV_TIME items[8], DV_DATE_TIME items[6],
// DV_DURATION items[11]. Each authors a temporal C_* pattern/range and commits the
// in-constraint base (accepted) + an out-of-constraint value (rejected). Where the
// validator still defers temporal enforcement the reject is a reported finding.

/// Author a temporal `C_*` constraint on `host`'s value; commit the base value
/// (accepted) and an out-of-constraint value (rejected).
#[allow(clippy::too_many_arguments)]
async fn drive_temporal(
    ctx: &RunContext<'_>,
    tid: &'static str,
    host: &'static str,
    rm_prim: &'static str,
    prim: CPrimitive,
    idx: usize,
    bad: Value,
    label: &'static str,
) -> Result<DataSetReport, CaseError> {
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
}

fn run_dv_date_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_date_pat",
            "DV_DATE",
            "Date",
            author::c_date(Some("yyyy-mm-dd"), None),
            5,
            json!("2021"),
            "partial date '2021' violates yyyy-mm-dd (C_DATE.pattern)",
        )
        .await?
        .of_schedule_rows(15))
    })
}

fn run_dv_date_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_date_rng",
            "DV_DATE",
            "Date",
            author::c_date(None, Some(("2021-01-01", "2021-12-31"))),
            5,
            json!("2025-06-01"),
            "'2025-06-01' outside [2021-01-01,2021-12-31] (C_DATE.range)",
        )
        .await?
        .of_schedule_rows(9))
    })
}

fn run_dv_time_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_time_pat",
            "DV_TIME",
            "Time",
            author::c_time(Some("HH:MM:SS"), None),
            8,
            json!("22"),
            "partial time '22' violates HH:MM:SS (C_TIME.pattern)",
        )
        .await?
        .of_schedule_rows(70))
    })
}

fn run_dv_time_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_time_rng",
            "DV_TIME",
            "Time",
            author::c_time(None, Some(("00:00:00", "23:00:00"))),
            8,
            json!("23:59:59"),
            "'23:59:59' outside [00:00:00,23:00:00] (C_TIME.range)",
        )
        .await?
        .of_schedule_rows(200))
    })
}

fn run_dv_duration_fields<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        // AOM 1.4 §C_DURATION: the pattern's letters are the *allowed* fields —
        // PTHMS = time-only, so the base PT30M conforms and any date field is
        // forbidden.
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_dur_fields",
            "DV_DURATION",
            "Duration",
            author::c_duration(Some("PTHMS"), None),
            11,
            json!("P1Y"),
            "'P1Y' uses a date field the PTHMS pattern forbids (C_DURATION.pattern)",
        )
        .await?
        .of_schedule_rows(18))
    })
}

fn run_dv_duration_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_dur_rng",
            "DV_DURATION",
            "Duration",
            author::c_duration(None, Some(("PT0S", "PT1H"))),
            11,
            json!("PT5H"),
            "'PT5H' outside [PT0S,PT1H] (C_DURATION.range)",
        )
        .await?
        .of_schedule_rows(21))
    })
}

fn run_dv_duration_fields_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_dur_fr",
            "DV_DURATION",
            "Duration",
            author::c_duration(Some("PTHM"), Some(("PT0S", "PT1H"))),
            11,
            json!("PT5H"),
            "'PT5H' outside [PT0S,PT1H] (C_DURATION.pattern+range)",
        )
        .await?
        .of_schedule_rows(9))
    })
}

fn run_dv_date_time_range<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive_temporal(
            ctx,
            "cnf_cont_dv_datetime_rng",
            "DV_DATE_TIME",
            "Date_Time",
            author::c_date_time(None, Some(("2021-01-01T00:00:00", "2021-12-31T23:59:59"))),
            6,
            json!("2025-06-01T12:00:00"),
            "'2025-06-01T12:00:00' outside the range (C_DATE_TIME.range)",
        )
        .await?
        .of_schedule_rows(37))
    })
}

/// master17.4 CONT-DV_DATE_TIME-validate_constraint — the vendored `all_types`
/// `items[6]` `DV_DATE_TIME` is constrained by a `C_DATE_TIME` full-field pattern;
/// a year-only value violates it. G-3 explicit open finding (the SUT accepts the
/// partial value the 176-row table rejects) — reported, never masked.
fn run_dv_date_time_constraint<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        Ok(drive::drive_constraint(
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
        .await?
        .of_schedule_rows(176))
    })
}

/// ECC-VAL-119 — the ECC-original negative guard for the corrected `all_types`
/// fixture (§3). Commits the byte-identical vendored defective composition (a
/// day-bearing `DV_DATE` at a leaf whose `C_DATE` pattern disallows the day) and
/// asserts the SUT rejects it (`422`); archie is lenient and accepts it.
fn run_dv_date_day_disallowed<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        support::ensure_opt(ctx, "template.valid", "all_types/Test_all_types.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let comp =
            fixtures::owned_json("owned.composition.all-types.invalid").map_err(|e| codec(&e))?;
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                    .json_body(&comp)?
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 422)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── 17.6 DV_PARSABLE / DV_MULTIMEDIA ──────────────────────────────────────────

fn run_dv_parsable_formalism<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(13, "formalism");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(7))
    })
}

fn run_dv_multimedia_media_type<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = leaf_ptr(12, "media_type/code_string");
        let t = leaf_ptr(12, "media_type/terminology_id/value");
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(8))
    })
}

// ── 17.7 DV_URI / DV_EHR_URI (retyped scratch leaf) ───────────────────────────

fn run_dv_uri_pattern<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(2))
    })
}

fn run_dv_uri_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(2))
    })
}

fn run_dv_ehr_uri_open<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(17))
    })
}

fn run_dv_ehr_uri_pattern<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(3))
    })
}

fn run_dv_ehr_uri_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        let p = value_ptr(4);
        Ok(drive_leaf_rows(
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
        .await?
        .of_schedule_rows(3))
    })
}
