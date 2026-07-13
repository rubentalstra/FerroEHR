//! Content-chapter SUT driving (design §2.2a, §4.5).
//!
//! The content chapters (master15/16/17.x) verify **data-validation** conformance:
//! a data instance is committed and the server must accept or reject it per the
//! constraint the case expresses. Most of those constraints are **archetype/OPT
//! constraints** (cardinality intervals, `C_STRING` pattern/list, `C_INTEGER`
//! range/list, `C_QUANTITY` property/units, class-narrowing) — the upstream
//! schedule itself says these archetypes/templates "should be generated"
//! (master15 §Implementation notes) and ships **none** in the vendored corpus.
//! Historically those cases were skipped for lack of a constraint-expressing
//! OPT; the suite now **authors** the constraint OPT per case ([`super::author`])
//! so every one is driven — never a fabricated pass and never a masked failure
//! (design §4.5: failures are findings).
//!
//! What **is** drivable with the vendored corpus is the subset the truth tables
//! mark **"(RM/schema constraint)"** / **"RM/Schema mandatory"** — a mandatory RM
//! attribute whose absence any conformant server must reject regardless of
//! archetype. Those rows are driven against the known-committable base
//! compositions ([`Base`]) via the typed mutators in [`super::mutate`], asserting
//! the ITS-REST validation contract (`composition_create.yaml`: `201` accepted /
//! `422` rejected).

use serde_json::Value;

use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::fixtures;
use crate::harness::{CaseError, DataSetReport, HttpRequest, HttpResponse, RunContext};
use crate::suites::support;

use super::mutate;

/// A known-committable base composition from the vendored corpus (the ones the
/// master07 suite already commits green). Content cases mutate a clone of the
/// base and re-commit.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Base {
    /// `persistent_minimal.en.v1` — a **persistent** COMPOSITION (category `431`),
    /// no context; contains an OBSERVATION → HISTORY → `POINT_EVENT`.
    PersistentMinimal,
    /// `test_all_types.en.v1` — the all-data-types COMPOSITION: carries a leaf of
    /// nearly every `DV_*` type (COUNT, DATE, `DATE_TIME`, TIME, DURATION,
    /// IDENTIFIER, MULTIMEDIA, PARSABLE, PROPORTION, URI, ORDINAL, QUANTITY,
    /// `CODED_TEXT`, BOOLEAN, TEXT). The committable base for the master17.x
    /// `validate_open` (RM/Schema-mandatory) rows across every data type.
    AllTypes,
}

impl Base {
    /// The OPT to provision (relative to `valid_templates/`).
    fn opt(self) -> &'static str {
        match self {
            Base::PersistentMinimal => "minimal_persistent/persistent_minimal.opt",
            Base::AllTypes => "all_types/Test_all_types.opt",
        }
    }

    /// The canonical-JSON `__full` fixture path.
    pub fn json_fixture(self) -> &'static str {
        match self {
            Base::PersistentMinimal => {
                "compositions/CANONICAL_JSON/persistent_minimal.en.v1__full.json"
            }
            Base::AllTypes => "query/data_load/compositions/all_types.composition.json",
        }
    }

    /// A fresh clone of the base composition body. For [`Base::AllTypes`] this
    /// is the **owned** (corrected) `all_types.composition.json`, not the
    /// vendored original — the vendored fixture's `at0003` `DV_DATE` leaf
    /// violates its own OPT's `yyyy-??-XX` `C_DATE` pattern (day disallowed);
    /// see `testdata/fixtures/REGISTER.md` and the companion negative case
    /// `val/dv-date-day-disallowed-pattern`.
    pub fn load(self) -> Result<Value, CaseError> {
        match self {
            Base::AllTypes => fixtures::owned_fixture(ALL_TYPES_COMP_FILE).map_err(codec),
            Base::PersistentMinimal => fixtures::read_json(self.json_fixture()).map_err(codec),
        }
    }
}

/// Path (owned-root-relative) of the corrected `all_types` v1 composition in
/// `testdata/fixtures/valid/compositions/`.
const ALL_TYPES_COMP_FILE: &str = "valid/compositions/all_types.composition.json";
/// Path of the corrected `all_types` v2 composition (load-bearing: the accepted
/// base of `val/dv-coded-text-local-codes`).
const ALL_TYPES_V2_COMP_FILE: &str = "valid/compositions/all_types_v2.composition.json";
/// Vendored (corpus-relative) path of the `all_types` v1 composition.
const ALL_TYPES_COMP_VENDORED: &str = "query/data_load/compositions/all_types.composition.json";
/// Vendored (corpus-relative) path of the `all_types` v2 composition.
const ALL_TYPES_V2_COMP_VENDORED: &str =
    "query/data_load/compositions/all_types_v2.composition.json";

/// Map a vendored `all_types` composition path to its owned (corrected)
/// fixture file name, or `None` for any other composition. The owned copies fix
/// the `at0003` `DV_DATE` leaf that violates the `yyyy-??-XX` `C_DATE` pattern
/// (`testdata/fixtures/REGISTER.md`).
fn owned_all_types(comp: &str) -> Option<&'static str> {
    match comp {
        ALL_TYPES_COMP_VENDORED => Some(ALL_TYPES_COMP_FILE),
        ALL_TYPES_V2_COMP_VENDORED => Some(ALL_TYPES_V2_COMP_FILE),
        _ => None,
    }
}

fn codec(e: fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

// ── constraint-OPT driving (the full constraint-carrying corpus) ──────────────

/// A committable COMPOSITION from the vendored corpus, plus the OPT that
/// constrains it. Content-chapter constraint cases upload the OPT, commit the
/// unmutated composition (accepted), then commit a copy with the constrained
/// leaf violated (rejected) — the `master15/16/17.x` truth-table oracle
/// (design §4.5). `opt` is `valid_templates/`-relative (matching
/// [`support::ensure_opt`]); `comp` is a corpus-root-relative bare
/// canonical-JSON COMPOSITION fixture (a `.contribution.json` wrapper is not
/// usable directly — its embedded COMPOSITION omits `archetype_details` on its
/// content ENTRYs and fails the `Is_archetypeRoot` RM invariant as a bare commit).
pub struct Constraint {
    /// OPT path relative to `valid_templates/` (e.g. `all_types/Test_all_types.opt`).
    pub opt: &'static str,
    /// Bare canonical-JSON COMPOSITION fixture path (corpus-root-relative).
    pub comp: &'static str,
}

/// Provision `opt_rel` (idempotent), create a fresh EHR, and commit `comp`.
async fn commit_opt(
    ctx: &RunContext<'_>,
    opt_rel: &str,
    comp: &Value,
) -> Result<HttpResponse, CaseError> {
    support::ensure_opt(ctx, opt_rel).await?;
    let ehr_id = support::create_ehr(ctx).await?;
    ctx.send(
        HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
            .json_body(comp)?
            .header("accept", "application/json"),
    )
    .await
}

/// One constraint-violating data set: a row label, a mutation to apply to a clone
/// of the valid composition, and the expected (rejected) outcome.
pub type Violation = (String, Box<dyn FnOnce(&mut Value) + Send>, Expected);

/// Drive a constraint case: row 0 is the unmutated composition (accepted), the
/// remaining rows are [`Violation`]s. Each row commits a fresh clone to a fresh
/// EHR against the constraint OPT. Any wrong outcome fails the whole case (a
/// finding, design §4.5), naming the first diverging row.
pub async fn drive_constraint(
    ctx: &RunContext<'_>,
    constraint: &Constraint,
    accepted_label: &str,
    violations: Vec<Violation>,
) -> Result<DataSetReport, CaseError> {
    // For the two `all_types` compositions, load the owned (corrected) copy
    // instead of the vendored original: the vendored `at0003` `DV_DATE` leaf
    // violates its own OPT's `yyyy-??-XX` `C_DATE` pattern
    // (`testdata/fixtures/REGISTER.md`; companion negative case
    // `val/dv-date-day-disallowed-pattern`).
    let base = match owned_all_types(constraint.comp) {
        Some(owned) => fixtures::owned_fixture(owned).map_err(codec)?,
        None => fixtures::read_json(constraint.comp).map_err(codec)?,
    };
    drive_constraint_base(ctx, constraint.opt, base, accepted_label, violations).await
}

/// Drive a constraint case from an already-materialised base COMPOSITION
/// (design §4.5). Identical row semantics to [`drive_constraint`], but the valid
/// base is supplied directly rather than read from a canonical-JSON fixture —
/// used where the constraint's only committable instance is a **FLAT** one
/// converted via [`fixtures::flat_to_canonical`] (path *b*), so no canonical
/// composition fixture exists to name. `opt_rel` is provisioned per row exactly
/// as for [`drive_constraint`].
pub async fn drive_constraint_base(
    ctx: &RunContext<'_>,
    opt_rel: &str,
    base: Value,
    accepted_label: &str,
    violations: Vec<Violation>,
) -> Result<DataSetReport, CaseError> {
    let mut rows: Vec<(String, Value, Expected)> =
        vec![(accepted_label.to_owned(), base.clone(), Expected::Accepted)];
    for (label, mutate_fn, expected) in violations {
        let mut comp = base.clone();
        mutate_fn(&mut comp);
        rows.push((label, comp, expected));
    }
    let total = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    let mut passed = 0u32;
    let mut first_failure: Option<CaseError> = None;
    for (label, comp, expected) in rows {
        let resp = commit_opt(ctx, opt_rel, &comp).await?;
        match check(&resp, expected, &label) {
            Ok(()) => passed += 1,
            Err(e) if first_failure.is_none() => first_failure = Some(e),
            Err(_) => {}
        }
    }
    if let Some(e) = first_failure {
        return Err(e);
    }
    Ok(DataSetReport { passed, total })
}

/// The expected outcome of committing a data instance (design §4.5): the schedule
/// truth-table `expected` column, concretized to the ITS-REST validation contract.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Expected {
    /// `accepted` — the server stores it (`composition_create.yaml` `201`).
    Accepted,
    /// `rejected` — the server refuses it as invalid
    /// (`composition_create.yaml` `422`, ITS-REST validation).
    Rejected,
}

/// Provision an **authored** OPT (in-memory ADL 1.4 XML, idempotent), create a
/// fresh EHR, and commit `comp`.
async fn commit_authored(
    ctx: &RunContext<'_>,
    opt_xml: &str,
    comp: &Value,
) -> Result<HttpResponse, CaseError> {
    support::ensure_opt_xml(ctx, opt_xml).await?;
    let ehr_id = support::create_ehr(ctx).await?;
    ctx.send(
        HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
            .json_body(comp)?
            .header("accept", "application/json"),
    )
    .await
}

/// Drive a set of `(row-label, composition, expected)` data sets against an
/// **authored** constraint OPT (provisioned once per row, idempotently). Identical
/// row semantics to [`drive_rows`]/[`drive_constraint_base`] but the OPT is an
/// in-memory string authored by [`super::author`] rather than a corpus fixture —
/// the provisioning path for the master15–17 archetype-constraint truth tables the
/// vendored corpus ships no template for. Any wrong outcome fails the whole case (a
/// finding, design §4.5), naming the first diverging row.
pub async fn drive_authored(
    ctx: &RunContext<'_>,
    opt_xml: &str,
    rows: Vec<(String, Value, Expected)>,
) -> Result<DataSetReport, CaseError> {
    let total = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    let mut passed = 0u32;
    let mut first_failure: Option<CaseError> = None;
    for (label, comp, expected) in rows {
        let resp = commit_authored(ctx, opt_xml, &comp).await?;
        match check(&resp, expected, &label) {
            Ok(()) => passed += 1,
            Err(e) if first_failure.is_none() => first_failure = Some(e),
            Err(_) => {}
        }
    }
    if let Some(e) = first_failure {
        return Err(e);
    }
    Ok(DataSetReport { passed, total })
}

/// Provision `base`'s OPT (idempotent), create a fresh EHR, and commit `comp`.
async fn commit(ctx: &RunContext<'_>, base: Base, comp: &Value) -> Result<HttpResponse, CaseError> {
    support::ensure_opt(ctx, base.opt()).await?;
    let ehr_id = support::create_ehr(ctx).await?;
    ctx.send(
        HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
            .json_body(comp)?
            .header("accept", "application/json"),
    )
    .await
}

/// Assert a commit response matches `expected`, citing the ITS-REST contract in
/// the failure message so a divergence is a copy-pasteable finding (design §4.5).
fn check(resp: &HttpResponse, expected: Expected, row: &str) -> Result<(), CaseError> {
    match expected {
        Expected::Accepted => {
            if resp.status == 201 {
                Ok(())
            } else {
                Err(CaseError::Assertion(format!(
                    "{row}: expected accepted (composition_create.yaml 201), got {} ({})",
                    resp.status,
                    resp.text().chars().take(200).collect::<String>()
                )))
            }
        }
        Expected::Rejected => {
            // The schedule's "rejected" is a negative response; ITS-REST
            // `composition_create.yaml` returns `422` for semantic/validation
            // failure and `400` for a malformed body — both are a valid refusal
            // of the invalid instance. A `2xx` accept is the finding.
            if resp.status == 422 || resp.status == 400 {
                Ok(())
            } else {
                Err(CaseError::Assertion(format!(
                    "{row}: expected rejected (ITS-REST validation composition_create.yaml 422), \
                     got {}",
                    resp.status
                )))
            }
        }
    }
}

/// Drive a set of `(row-label, mutated-composition, expected)` data sets against
/// `base`, accumulating the per-row result. Any wrong outcome fails the whole
/// case (a finding) but the message names the first diverging row.
async fn drive_rows(
    ctx: &RunContext<'_>,
    base: Base,
    rows: Vec<(String, Value, Expected)>,
) -> Result<DataSetReport, CaseError> {
    let total = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    let mut passed = 0u32;
    let mut first_failure: Option<CaseError> = None;
    for (label, comp, expected) in rows {
        let resp = commit(ctx, base, &comp).await?;
        match check(&resp, expected, &label) {
            Ok(()) => passed += 1,
            Err(e) if first_failure.is_none() => first_failure = Some(e),
            Err(_) => {}
        }
    }
    if let Some(e) = first_failure {
        return Err(e);
    }
    Ok(DataSetReport { passed, total })
}

// ── generic drivers (the RM/schema-determinable rows) ─────────────────────────

/// Drive an ENTRY `data` existence case (master16 CONT-OBS-* / CONT-EVENT-* — the
/// rows marked "(RM/schema constraint)"): the ENTRY's `data` is mandatory `[1]` in
/// the RM, so a committed instance **without** `data` must be rejected and one
/// **with** `data` (the base) accepted, independent of any archetype.
///
/// `entry_type` is the RM `_type` whose `data` is removed (`"OBSERVATION"`, or an
/// EVENT subtype for the EVENT.data rows). If the base carries no such node the
/// case is skipped (no committable fixture).
pub async fn entry_data_existence(
    ctx: &RunContext<'_>,
    entry_type: &str,
) -> Result<DataSetReport, CaseError> {
    let base = Base::PersistentMinimal;
    let full = base.load()?;
    if !mutate::contains_node(&full, entry_type) {
        return Err(CaseError::Skipped(format!(
            "no committable base composition in the vendored corpus carries a {entry_type} node \
             to mutate; RM/schema data-existence row not drivable"
        )));
    }
    // Row 1: data present (the unmutated base) → accepted.
    let present = full.clone();
    // Row 2: data absent → rejected (RM/schema: <entry>.data existence.lower).
    let mut absent = full;
    if let Some(node) = mutate::first_node_mut(&mut absent, entry_type) {
        mutate::remove_field(node, "data");
    }
    drive_rows(
        ctx,
        base,
        vec![
            (
                format!("{entry_type} with data (RM present)"),
                present,
                Expected::Accepted,
            ),
            (
                format!("{entry_type} without data (RM/schema {entry_type}.data existence.lower)"),
                absent,
                Expected::Rejected,
            ),
        ],
    )
    .await
}

/// Drive a `DATA_VALUE` `validate_open` case's **RM/Schema mandatory** rows
/// (master17.x): the value type's mandatory field (`value`, `magnitude`, …) is
/// `[1]` in the RM, so an instance with it removed must be rejected and the
/// unmutated base accepted. Skipped if the base carries no leaf of `rm_type`
/// (no committable fixture in the corpus for that type).
pub async fn data_type_mandatory(
    ctx: &RunContext<'_>,
    base: Base,
    rm_type: &str,
    mandatory_field: &str,
) -> Result<DataSetReport, CaseError> {
    let full = base.load()?;
    if !mutate::contains_node(&full, rm_type) {
        return Err(CaseError::Skipped(format!(
            "no committable base composition in the vendored corpus carries a {rm_type} leaf; \
             RM/schema mandatory-{mandatory_field} row not drivable"
        )));
    }
    let present = full.clone();
    let mut absent = full;
    if let Some(node) = mutate::first_node_mut(&mut absent, rm_type) {
        mutate::remove_field(node, mandatory_field);
    }
    drive_rows(
        ctx,
        base,
        vec![
            (
                format!("{rm_type} with {mandatory_field} (RM present)"),
                present,
                Expected::Accepted,
            ),
            (
                format!("{rm_type} without {mandatory_field} (RM/Schema mandatory)"),
                absent,
                Expected::Rejected,
            ),
        ],
    )
    .await
}

// ── the shared CaseMeta builder for content cases ─────────────────────────────

/// Build a content [`CaseMeta`] (design §4.2): area [`Area::Val`], capability
/// [`Capability::ArchetypeValidation`], required by CORE + STANDARD, JSON format
/// (canonical-JSON validation is the wire path these cases exercise), with the
/// registration slug `id`, human `title`, and spec `citation`.
#[must_use]
pub fn meta(id: &'static str, title: &'static str, citation: &'static str) -> CaseMeta {
    CaseMeta {
        id,
        title,
        area: Area::Val,
        capability: Capability::ArchetypeValidation,
        profiles: &[Profile::Core, Profile::Standard],
        formats: &[Format::Json],
        citation,
        compare: Compare::Superset,
        schedule_ref: None,
    }
}
