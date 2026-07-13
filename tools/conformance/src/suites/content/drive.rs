//! Content-chapter SUT driving (master15/16/17.x data-validation).
//!
//! The content chapters verify **data-validation** conformance: a data instance
//! is committed and the server must accept or reject it per the constraint the
//! case expresses. Most constraints are **archetype/OPT constraints**
//! (cardinality intervals, existence, `C_STRING` pattern/list, class-narrowing);
//! the schedule itself says these archetypes/templates "should be generated"
//! (master15 §Implementation notes) and ships **none** in the vendored corpus, so
//! the suite **authors** the constraint OPT per case ([`super::author`]) — never a
//! fabricated pass, never a masked failure (a wrong outcome is a genuine finding).
//!
//! The RM/schema-determinable subset — the truth-table rows marked
//! "(RM/schema constraint)" / "RM/Schema mandatory" — is driven against the
//! known-committable base compositions via [`super::mutate`], asserting the
//! ITS-REST validation contract.
//!
//! ## Reject-status assertion (register 12 G-3)
//!
//! The schedule's "rejected" is a negative response with **no** pinned code; the
//! ITS-REST edition matters. `composition_create` pins **422** for a semantic
//! validation failure (all content data-validation rejects are semantic). Older
//! implementations returned **400**. [`check`] therefore asserts the reject
//! through the edition ladder `[(Development, 422), (Release103, 400)]`
//! ([`crate::engine::assert::status_ladder`]): under our pinned-`development` CI
//! only 422 passes, so a 400 for a semantic failure is drift, not silently
//! tolerated. A `2xx` accept is always the finding.
//!
//! The schedule also names the *violated constraint* per reject row
//! (`COMPOSITION.content: cardinality.lower`, "Class not allowed", …). ITS-REST
//! 1.0.3 does not standardize a machine-readable validation-error body naming the
//! violated path, so [`check`] asserts only the (edition-laddered) accept/reject
//! verdict — the spec-determined part — and does not scrape the error body.
//! Boundary: were the SUT to emit a structured `openEHR-ERROR` body, the
//! offending-path assertion would move here.

use serde_json::Value;

use crate::edition::Edition;
use crate::engine::assert;
use crate::engine::harness::{CaseError, DataSetReport, HttpRequest, HttpResponse, RunContext};
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::testdata::fixtures;

use super::mutate;

/// The expected outcome of committing a data instance: the schedule truth-table
/// `expected` column, concretized to the ITS-REST validation contract.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Expected {
    /// `accepted` — the server stores it (`composition_create.yaml` 201).
    Accepted,
    /// `rejected` — the server refuses it as invalid (edition-laddered
    /// 422/400; see the module doc).
    Rejected,
}

/// A known-committable base composition from the vendored corpus. The
/// RM/schema-mandatory rows (master16/17.x) mutate a clone of one of these and
/// re-commit.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Base {
    /// `persistent_minimal.en.v1` — a persistent COMPOSITION (category 431),
    /// carrying OBSERVATION → HISTORY → `POINT_EVENT`.
    PersistentMinimal,
    /// `all_types.composition` (owned, corrected) — a leaf of nearly every
    /// `DV_*` type; the committable base for the master17.x RM/Schema-mandatory
    /// rows across every data type.
    AllTypes,
    /// `all_types_v2.composition` (owned, corrected) — the v2 leaf set.
    AllTypesV2,
}

/// The persistent base OPT file (under the `template.valid` corpus-dir key).
pub const PERSIST_OPT_FILE: &str = "minimal_persistent/persistent_minimal.opt";

impl Base {
    /// The OPT `(dir_key, file)` to provision.
    fn opt(self) -> (&'static str, &'static str) {
        match self {
            Base::PersistentMinimal => ("template.valid", PERSIST_OPT_FILE),
            Base::AllTypes | Base::AllTypesV2 => ("template.valid", "all_types/Test_all_types.opt"),
        }
    }

    /// A fresh clone of the base composition body. The `all_types` copies are the
    /// **owned** (corrected) fixtures — the vendored `at0003` `DV_DATE` leaf
    /// violates its own OPT's `yyyy-??-XX` `C_DATE` pattern (day disallowed;
    /// `testdata/fixtures/REGISTER.md`, companion negative case
    /// `val/dv-date-day-disallowed-pattern`).
    ///
    /// # Errors
    /// [`CaseError::Codec`] on lookup/read/parse failure.
    pub fn load(self) -> Result<Value, CaseError> {
        match self {
            Base::PersistentMinimal => fixtures::read_from(
                "composition.canonical-json",
                "persistent_minimal.en.v1__full.json",
            )
            .map_err(codec)
            .and_then(|t| serde_json::from_str(&t).map_err(|e| CaseError::Codec(e.to_string()))),
            Base::AllTypes => {
                fixtures::owned_json("owned.composition.all-types.valid").map_err(codec)
            }
            Base::AllTypesV2 => {
                fixtures::owned_json("owned.composition.all-types-v2.valid").map_err(codec)
            }
        }
    }
}

fn codec(e: fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// A committable COMPOSITION from the vendored corpus + the OPT that constrains
/// it. A constraint case uploads the OPT, commits the unmutated composition
/// (accepted), then commits copies with the constrained leaf violated (rejected).
/// `opt_file` is a file under the `template.valid` corpus-dir key; `comp_dir_key`
/// + `comp_file` name a canonical-JSON COMPOSITION in a corpus-dir key.
pub struct Constraint {
    /// OPT file relative to the `template.valid` dir key.
    pub opt_file: &'static str,
    /// The composition's corpus-dir manifest key (e.g. `composition.canonical-json`).
    pub comp_dir_key: &'static str,
    /// The composition file within that dir key.
    pub comp_file: &'static str,
}

/// One constraint-violating data set: a row label, a mutation applied to a clone
/// of the valid composition, and the expected (rejected) outcome.
pub type Violation = (String, Box<dyn FnOnce(&mut Value) + Send>, Expected);

/// Build a content [`CaseMeta`]: area [`Area::Val`], capability
/// [`Capability::ArchetypeValidation`] (profiles master03: required by CORE +
/// STANDARD), JSON format (canonical-JSON validation is the wire path these
/// cases exercise), binding `POST /ehr/{ehr_id}/composition`, no content
/// comparison ([`Compare::None`] — accept/reject cases).
#[must_use]
pub fn content_meta(
    id: &'static str,
    title: &'static str,
    citation: &'static str,
    schedule: ScheduleTrace,
) -> CaseMeta {
    CaseMeta {
        id,
        title,
        area: Area::Val,
        capability: Capability::ArchetypeValidation,
        formats: &[Format::Json],
        citation,
        schedule,
        binding: Binding::Rest("POST /ehr/{ehr_id}/composition"),
        compare: Compare::None,
    }
}

/// Assert a commit response matches `expected` (register 12 G-3): accept is
/// `composition_create` 201; reject is the edition ladder `[(Development, 422),
/// (Release103, 400)]`. The failure message cites the contract so a divergence is
/// a copy-pasteable finding.
fn check(
    ctx: &RunContext<'_>,
    resp: &HttpResponse,
    expected: Expected,
    row: &str,
) -> Result<(), CaseError> {
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
        Expected::Rejected => assert::status_ladder(
            ctx,
            resp,
            &[(Edition::Development, 422), (Edition::Release103, 400)],
            &format!("{row}: reject (ITS-REST composition_create validation)"),
        )
        .map(|_| ()),
    }
}

/// Provision an in-memory authored OPT (idempotent), create a fresh EHR, commit
/// `comp`.
async fn commit_authored(
    ctx: &RunContext<'_>,
    opt_xml: &str,
    comp: &Value,
) -> Result<HttpResponse, CaseError> {
    support::ensure_opt_xml(ctx, opt_xml).await?;
    let ehr_id = support::create_ehr(ctx).await?;
    post_composition(ctx, &ehr_id, comp).await
}

/// Provision a corpus OPT (`dir_key`/`file`, idempotent), create a fresh EHR,
/// commit `comp`.
async fn commit_opt(
    ctx: &RunContext<'_>,
    opt_dir_key: &str,
    opt_file: &str,
    comp: &Value,
) -> Result<HttpResponse, CaseError> {
    support::ensure_opt(ctx, opt_dir_key, opt_file).await?;
    let ehr_id = support::create_ehr(ctx).await?;
    post_composition(ctx, &ehr_id, comp).await
}

async fn post_composition(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    comp: &Value,
) -> Result<HttpResponse, CaseError> {
    ctx.send(
        HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
            .json_body(comp)?
            .header("accept", "application/json"),
    )
    .await
}

/// Drive `(label, composition, expected)` rows against an **authored** OPT
/// (in-memory ADL 1.4 XML), provisioned once per row (idempotently). Any wrong
/// outcome fails the whole case, naming the first diverging row (design §4.5).
///
/// # Errors
/// [`CaseError`] on transport failure or the first assertion that does not hold.
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
        match check(ctx, &resp, expected, &label) {
            Ok(()) => passed += 1,
            Err(e) if first_failure.is_none() => first_failure = Some(e),
            Err(_) => {}
        }
    }
    if let Some(e) = first_failure {
        return Err(e);
    }
    Ok(DataSetReport {
        passed,
        total,
        schedule_rows: None,
    })
}

/// Drive a constraint case whose valid base is a corpus-dir composition
/// ([`Constraint`]): row 0 is the unmutated composition (accepted), the rest are
/// [`Violation`]s.
///
/// # Errors
/// [`CaseError`] on transport failure or the first assertion that does not hold.
pub async fn drive_constraint(
    ctx: &RunContext<'_>,
    constraint: &Constraint,
    accepted_label: &str,
    violations: Vec<Violation>,
) -> Result<DataSetReport, CaseError> {
    let text = fixtures::read_from(constraint.comp_dir_key, constraint.comp_file).map_err(codec)?;
    let base = serde_json::from_str(&text).map_err(|e| CaseError::Codec(e.to_string()))?;
    drive_constraint_base(
        ctx,
        "template.valid",
        constraint.opt_file,
        base,
        accepted_label,
        violations,
    )
    .await
}

/// Drive a constraint case from an already-materialised base COMPOSITION
/// (owned/authored/converted): row 0 accepted, the rest [`Violation`]s. The OPT
/// (`opt_dir_key`/`opt_file`) is provisioned per row exactly as for
/// [`drive_constraint`].
///
/// # Errors
/// [`CaseError`] on transport failure or the first assertion that does not hold.
pub async fn drive_constraint_base(
    ctx: &RunContext<'_>,
    opt_dir_key: &str,
    opt_file: &str,
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
        let resp = commit_opt(ctx, opt_dir_key, opt_file, &comp).await?;
        match check(ctx, &resp, expected, &label) {
            Ok(()) => passed += 1,
            Err(e) if first_failure.is_none() => first_failure = Some(e),
            Err(_) => {}
        }
    }
    if let Some(e) = first_failure {
        return Err(e);
    }
    Ok(DataSetReport {
        passed,
        total,
        schedule_rows: None,
    })
}

/// Drive an ENTRY `data` existence case (master16 CONT-OBS-*/CONT-EVENT-* rows
/// marked "(RM/schema constraint)"): the ENTRY's `data` is mandatory `[1]` in the
/// RM, so a committed instance **without** `data` must be rejected and one
/// **with** `data` (the base) accepted, independent of any archetype. Provisioned
/// against the persistent base OPT (unauthored — the RM/schema rows need no
/// archetype constraint).
///
/// # Errors
/// [`CaseError::Skipped`] if the base carries no `entry_type` node;
/// [`CaseError`] on transport failure or an assertion that does not hold.
pub async fn entry_data_existence(
    ctx: &RunContext<'_>,
    entry_type: &str,
) -> Result<DataSetReport, CaseError> {
    let full = Base::PersistentMinimal.load()?;
    if !mutate::contains_node(&full, entry_type) {
        return Err(CaseError::Skipped(format!(
            "no committable base composition carries a {entry_type} node to mutate; \
             RM/schema data-existence row not drivable"
        )));
    }
    let present = full.clone();
    let mut absent = full;
    if let Some(node) = mutate::first_node_mut(&mut absent, entry_type) {
        mutate::remove_field(node, "data");
    }
    let (opt_dir_key, opt_file) = Base::PersistentMinimal.opt();
    drive_constraint_base(
        ctx,
        opt_dir_key,
        opt_file,
        present,
        &format!("{entry_type} with data (RM present)"),
        vec![(
            format!("{entry_type} without data (RM/schema {entry_type}.data existence.lower)"),
            Box::new(move |c: &mut Value| {
                if let Some(node) = mutate::first_node_mut(c, entry_type) {
                    mutate::remove_field(node, "data");
                }
            }),
            Expected::Rejected,
        )],
    )
    .await
}

/// Drive a `DATA_VALUE` `validate_open` case's **RM/Schema mandatory** rows
/// (master17.x): the value type's mandatory field (`value`, `magnitude`, …) is
/// `[1]` in the RM, so an instance with it removed must be rejected and the
/// unmutated base accepted. Skipped if the base carries no leaf of `rm_type`.
///
/// # Errors
/// [`CaseError::Skipped`] if the base carries no `rm_type` leaf; [`CaseError`] on
/// transport failure or an assertion that does not hold.
pub async fn data_type_mandatory(
    ctx: &RunContext<'_>,
    base: Base,
    rm_type: &str,
    mandatory_field: &'static str,
) -> Result<DataSetReport, CaseError> {
    let full = base.load()?;
    if !mutate::contains_node(&full, rm_type) {
        return Err(CaseError::Skipped(format!(
            "no committable base composition carries a {rm_type} leaf; \
             RM/schema mandatory-{mandatory_field} row not drivable"
        )));
    }
    let (opt_dir_key, opt_file) = base.opt();
    let rm_type_owned = rm_type.to_owned();
    drive_constraint_base(
        ctx,
        opt_dir_key,
        opt_file,
        full,
        &format!("{rm_type} with {mandatory_field} (RM present)"),
        vec![(
            format!("{rm_type} without {mandatory_field} (RM/Schema mandatory)"),
            Box::new(move |c: &mut Value| {
                if let Some(node) = mutate::first_node_mut(c, &rm_type_owned) {
                    mutate::remove_field(node, mandatory_field);
                }
            }),
            Expected::Rejected,
        )],
    )
    .await
}
