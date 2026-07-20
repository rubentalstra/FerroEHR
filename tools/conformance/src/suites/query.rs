//! QUERY / AQL execution — the master11 spine + ECC-original AQL cases
//! (area `Qry`).
//!
//! **master11 is a schedule stub**: §Test Environment and
//! §Test Data Sets are `TBD`, one heading is the literal placeholder
//! "`Test Case bbbb`", and all four real flows are `xx`. So the four
//! `I_QUERY_SERVICE.*` headings are concretized as ECC-original cases (the
//! `schedule` trace records the stub heading; the assertions derive from
//! ITS-REST 1.0.3 QUERY + AQL 1.1), and the corpus evidence below the spine is
//! openly ECC-original.
//!
//! This module owns the **execution** cases (the four master11 headings, the
//! invalid-corpus negative, and one AQL-advanced case); the **golden-diff**
//! corpus cases + the golden normalizer live in [`crate::suites::query_golden`],
//! and the `TERMINOLOGY()` / `matches {uri}` family lives in the terminology
//! suite (kept out of `Qry`).
//!
//! Rulings realized here:
//!
//! - **`AqlAdvanced` claimable.** Every master11-spine case is
//!   [`Capability::AqlBasic`] (STANDARD), but [`run_advanced_order_limit`] is
//!   [`Capability::AqlAdvanced`] (OPTIONS) — an `ORDER BY … LIMIT`/`OFFSET`
//!   query per AQL 1.1 (`AqlParser.g4` `orderByClause? limitClause?`), so the
//!   AQL-advanced OPTIONS capability is earned from a real passing case, not
//!   left unclaimable.
//! - **No `_schema_version` pinning.** `RESULT_SET` fields are read through
//!   the local [`result_set`] helpers with explicit ITS-REST `RESULT_SET`
//!   citations; nothing asserts `meta._schema_version` (a dev-OAS/RM-1.2.0
//!   artefact). A fully centralized `RESULT_SET` wire adapter with an edition
//!   ladder is not yet exposed — recorded as a boundary.

use uuid::Uuid;

use crate::engine::assert;
use crate::engine::harness::{
    CaseError, CaseFuture, DataSetReport, HttpRequest, HttpResponse, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::query_golden::{self, Mode};
use crate::suites::support;
use crate::testdata::fixtures;

const JSON: &[Format] = &[Format::Json];

/// The A–D corpus groups (used by the invalid-negative sweep).
const GROUPS: [&str; 4] = ["A", "B", "C", "D"];

const AD_HOC_CITATION: &str = "CNF master11 §I_QUERY_SERVICE (stub, xx flow); ITS-REST 1.0.3 QUERY API §execute_ad_hoc_query \
     (200_QUERY.yaml RESULT_SET); AQL 1.1";
const STORED_CITATION: &str = "CNF master11 §I_QUERY_SERVICE (stub, xx flow); ITS-REST 1.0.3 QUERY API \
     §execute_stored_query + DEFINITION QUERY §store; AQL 1.1";

/// Every registered QUERY-execution case (4 master11 spine + 1 invalid-negative
/// + 1 `AqlAdvanced`).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        case(
            "qry/smoke-test",
            "Query service smoke test",
            Capability::AqlBasic,
            AD_HOC_CITATION,
            ScheduleTrace::EccOriginal("I_QUERY_SERVICE.smoke_test (master11:48, stub xx flow)"),
            Binding::Rest("POST /query/aql"),
            run_smoke_test,
        ),
        case(
            "qry/execute-ad-hoc-query-empty-db",
            "Execute ad-hoc AQL query — empty db",
            Capability::AqlBasic,
            AD_HOC_CITATION,
            ScheduleTrace::EccOriginal(
                "I_QUERY_SERVICE.execute_ad_hoc_query-empty_db (master11:83, A.1.z, stub xx flow)",
            ),
            Binding::Rest("POST /query/aql"),
            run_adhoc_empty_db,
        ),
        case(
            "qry/execute-stored-query-empty-db",
            "Execute stored AQL query — empty db",
            Capability::AqlBasic,
            STORED_CITATION,
            ScheduleTrace::EccOriginal(
                "I_QUERY_SERVICE.execute_stored_query-empty_db (master11:61, stub xx flow)",
            ),
            Binding::Rest("PUT /definition/query/{name}/{version}; GET /query/{name}"),
            run_stored_empty_db,
        ),
        case(
            "qry/execute-ad-hoc-query-loaded-db",
            "Execute ad-hoc AQL query — loaded db",
            Capability::AqlBasic,
            "CNF master11 §I_QUERY_SERVICE (stub, xx flow); ITS-REST 1.0.3 QUERY API \
             §execute_ad_hoc_query; AQL 1.1 CONTAINS (master03-syntax §containsExpr)",
            ScheduleTrace::EccOriginal(
                "I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db (master11:96, A.1.a, stub xx flow)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition; POST /query/aql"),
            run_adhoc_loaded_db,
        ),
        // ── ECC-original: the projected uid VALUE (not just the column shape) ──
        case(
            "qry/uid-projection-value",
            "AQL uid projection — c/uid/value returns the version id",
            Capability::AqlBasic,
            "AQL 1.1 master03-syntax §Identified paths (COMPOSITION.uid.value → /uid/value); \
             RM common master06 §Version identification (OBJECT_VERSION_ID); \
             ITS-REST 1.0.3 QUERY API §execute_ad_hoc_query 200_QUERY.yaml RESULT_SET",
            ScheduleTrace::EccOriginal(
                "schedule stub (master11 is TBD); the loaded-db case asserts only the projected \
                 column path — this case asserts the projected CELL equals the committed \
                 OBJECT_VERSION_ID (a null cell was a real, otherwise-invisible engine defect)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition; POST /query/aql"),
            run_uid_projection_value,
        ),
        // ── ECC-original: invalid queries must be rejected ─────────────────────
        case(
            "qry/corpus-invalid",
            "AQL corpus — invalid queries rejected",
            Capability::AqlBasic,
            "AQL 1.1 (invalid syntax); ITS-REST 1.0.3 QUERY API §execute_ad_hoc_query 400_QUERY.yaml; \
             reference: CNF query corpus aql_queries_invalid",
            ScheduleTrace::EccOriginal(
                "schedule stub (master11 is TBD — no invalid-query case); AQL 1.1 negative-rejection evidence",
            ),
            Binding::Rest("POST /query/aql"),
            run_invalid_queries,
        ),
        // ── AqlAdvanced (OPTIONS) — earned from a real passing case ────────────
        case(
            "qry/advanced-order-by-limit",
            "AQL advanced — ORDER BY + LIMIT/OFFSET",
            Capability::AqlAdvanced,
            "AQL 1.1 master03-syntax §orderByClause/§limitClause (AqlParser.g4 `orderByClause? limitClause?`); \
             ITS-REST 1.0.3 QUERY API §execute_ad_hoc_query 200_QUERY.yaml RESULT_SET",
            ScheduleTrace::EccOriginal(
                "schedule stub (master11 is TBD); AQL-advanced ORDER BY + LIMIT/OFFSET, profiles §AQL advanced OPTIONS",
            ),
            Binding::Rest("POST /query/aql"),
            run_advanced_order_limit,
        ),
    ]
}

// ── entry builder ────────────────────────────────────────────────────────────

fn case(
    id: &'static str,
    title: &'static str,
    capability: Capability,
    citation: &'static str,
    schedule: ScheduleTrace,
    binding: Binding,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Qry,
            capability,
            formats: JSON,
            citation,
            schedule,
            binding,
            compare: Compare::IgnoreSet,
        },
        run,
    }
}

/// Box a plain async result as a [`CaseFuture`].
macro_rules! boxed {
    ($body:block) => {
        Box::pin(async move $body)
    };
}

// ── shared helpers ──────────────────────────────────────────────────────────

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Execute an ad-hoc AQL query (`POST /query/aql`, body `{"q": …}`).
async fn adhoc(ctx: &RunContext<'_>, aql: &str) -> Result<HttpResponse, CaseError> {
    let body = serde_json::json!({ "q": aql });
    ctx.send(
        HttpRequest::post("/query/aql")
            .json_body(&body)?
            .header("accept", "application/json"),
    )
    .await
}

/// The AQL text of a group-A query by golden/fixture name (from the corpus,
/// resolved through the manifest — no free-path access).
fn query_text(group: &str, name: &str) -> Result<String, CaseError> {
    let fixtures = fixtures::aql_valid(group).map_err(|e| codec(&e))?;
    let fixture = fixtures
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| CaseError::Assertion(format!("missing {group}/{name} query fixture")))?;
    fixtures::aql_text(fixture).map_err(|e| codec(&e))
}

// ── RESULT_SET wire reads (explicit, cited, no `_schema_version`) ──────────────

/// `RESULT_SET` field reads. ITS-REST 1.0.3 QUERY API `200_QUERY.yaml` `RESULT_SET`:
/// `{ meta: { _type }, columns: [{ name, path? }], rows: [...] }`. `_schema_version`
/// is deliberately NOT read/asserted (a dev-OAS/RM-1.2.0 artefact).
mod result_set {
    use serde_json::Value;

    /// `meta._type` (`"RESULTSET"` on a well-formed result set).
    pub(super) fn meta_type(body: &Value) -> Option<&str> {
        body.pointer("/meta/_type").and_then(Value::as_str)
    }

    /// The `columns` array.
    pub(super) fn columns(body: &Value) -> Option<&Vec<Value>> {
        body.get("columns").and_then(Value::as_array)
    }

    /// The `rows` array length (0 when absent).
    pub(super) fn row_count(body: &Value) -> usize {
        body.get("rows")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    }

    /// The first column's `path` (ITS-REST allows omitting `path` for EHR
    /// pseudo-attribute projections; `None` is a legitimate shape, not a fault).
    pub(super) fn first_column_path(body: &Value) -> Option<&str> {
        columns(body)
            .and_then(|c| c.first())
            .and_then(|c| c.get("path"))
            .and_then(Value::as_str)
    }

    /// One projected cell (`rows[row][col]`), `None` when absent.
    pub(super) fn cell(body: &Value, row: usize, col: usize) -> Option<&Value> {
        body.get("rows")?.as_array()?.get(row)?.as_array()?.get(col)
    }
}

// ── master11 real cases (concretizing xx flows) ───────────────────────────────

/// `smoke_test`: a minimal ad-hoc query returns a well-formed `RESULT_SET`.
fn run_smoke_test<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let resp = adhoc(ctx, "SELECT e/ehr_id/value FROM EHR e").await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        if result_set::meta_type(&body) != Some("RESULTSET") {
            return Err(CaseError::Assertion(format!(
                "expected meta._type RESULTSET, got {:?}",
                result_set::meta_type(&body)
            )));
        }
        if result_set::columns(&body).is_none() {
            return Err(CaseError::Assertion(
                "RESULT_SET has no columns array".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// `execute_ad_hoc_query-empty_db`: a fixed-non-existent-id query returns the
/// empty golden `RESULT_SET` (full diff — DB-state-independent).
fn run_adhoc_empty_db<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let name = "200_get_ehr_by_id_empty_db.json";
        let aql = query_text("A", name)?;
        let golden = fixtures::aql_golden("empty_db", "A", name)
            .map_err(|e| codec(&e))?
            .ok_or_else(|| CaseError::Assertion("missing A/200 empty_db golden".to_owned()))?;
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        diff_golden(&golden, &resp.json()?, Mode::Full, "adhoc empty_db")
    })
}

/// `execute_stored_query-empty_db`: store a query, execute it by name, diff the
/// empty golden `RESULT_SET`.
fn run_stored_empty_db<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let name = "200_get_ehr_by_id_empty_db.json";
        let aql = query_text("A", name)?;
        let golden = fixtures::aql_golden("empty_db", "A", name)
            .map_err(|e| codec(&e))?
            .ok_or_else(|| CaseError::Assertion("missing A/200 empty_db golden".to_owned()))?;
        let qname = format!("org.conformance::stored_{}", Uuid::new_v4().simple());
        let store = ctx
            .send(
                HttpRequest::put(format!("/definition/query/{qname}/1.0.0"))
                    .text_body(aql, "text/plain")
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&store, &[200, 201])?;
        let resp = ctx
            .send(HttpRequest::get(format!("/query/{qname}")).header("accept", "application/json"))
            .await?;
        assert::status(&resp, 200)?;
        diff_golden(&golden, &resp.json()?, Mode::Full, "stored empty_db")
    })
}

/// `execute_ad_hoc_query-loaded_db`: commit a real composition to a fresh EHR,
/// then an EHR-scoped `CONTAINS COMPOSITION` returns it (self-contained through
/// the API, shared-SUT-safe).
fn run_adhoc_loaded_db<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = support::create_ehr(ctx).await?;
        support::ensure_opt(ctx, "template.valid", "nested/nested.opt").await?;
        let comp = compositions_by_name("nested.en.v1__full.json")?;
        let commit = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                    .json_body(&comp)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation"),
            )
            .await?;
        assert::status(&commit, 201)?;

        let aql = format!(
            "SELECT c/uid/value FROM EHR e[ehr_id/value='{ehr_id}'] CONTAINS COMPOSITION c"
        );
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        if result_set::row_count(&body) == 0 {
            return Err(CaseError::Assertion(
                "EHR-scoped CONTAINS COMPOSITION returned no rows after a commit".to_owned(),
            ));
        }
        // The single projected column is the composition uid path.
        match result_set::first_column_path(&body) {
            Some("/uid/value") => Ok(DataSetReport::SINGLE),
            other => Err(CaseError::Assertion(format!(
                "expected column path /uid/value, got {other:?}"
            ))),
        }
    })
}

// ── ECC-original cases ────────────────────────────────────────────────────────

/// The projected `c/uid/value` CELL equals the committed version's
/// `OBJECT_VERSION_ID` (AQL 1.1 master03 lists `COMPOSITION.uid.value` →
/// `/uid/value` as a normative identified path). The loaded-db spine case
/// asserts only the column path — a server that projects `null` for every uid
/// passes it, which was a real engine defect this case pins.
fn run_uid_projection_value<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = support::create_ehr(ctx).await?;
        support::ensure_opt(ctx, "template.valid", "nested/nested.opt").await?;
        let comp = compositions_by_name("nested.en.v1__full.json")?;
        let commit = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                    .json_body(&comp)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation"),
            )
            .await?;
        assert::status(&commit, 201)?;
        let committed_uid = commit
            .json()?
            .pointer("/uid/value")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                CaseError::Assertion("committed composition body carries no /uid/value".to_owned())
            })?;

        let aql = format!(
            "SELECT c/uid/value FROM EHR e[ehr_id/value='{ehr_id}'] CONTAINS COMPOSITION c"
        );
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        match result_set::cell(&body, 0, 0).and_then(serde_json::Value::as_str) {
            Some(projected) if projected == committed_uid => Ok(DataSetReport::SINGLE),
            other => Err(CaseError::Assertion(format!(
                "projected c/uid/value cell is {other:?}, expected the committed \
                 OBJECT_VERSION_ID '{committed_uid}'"
            ))),
        }
    })
}

/// Every vendored invalid query must be rejected (`4xx`, ITS-REST
/// `400_QUERY.yaml`). A `2xx` for a malformed query is the finding.
fn run_invalid_queries<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let mut invalid = Vec::new();
        for group in GROUPS {
            if let Ok(mut g) = fixtures::aql_invalid(group) {
                invalid.append(&mut g);
            }
        }
        if invalid.is_empty() {
            return Err(CaseError::Skipped(
                "no invalid query fixtures vendored".to_owned(),
            ));
        }
        let mut passed = 0u32;
        let mut total = 0u32;
        let mut first_fail: Option<String> = None;
        for fixture in invalid {
            let aql = fixtures::aql_text(&fixture).map_err(|e| codec(&e))?;
            total += 1;
            let resp = adhoc(ctx, &aql).await?;
            if (400..500).contains(&resp.status) {
                passed += 1;
            } else {
                first_fail.get_or_insert(format!(
                    "{}: invalid query accepted with {}",
                    fixture.name, resp.status
                ));
            }
        }
        if passed == total {
            Ok(DataSetReport::all(passed))
        } else {
            Err(CaseError::Assertion(format!(
                "{passed}/{total} invalid queries rejected; first: {}",
                first_fail.unwrap_or_default()
            )))
        }
    })
}

/// AQL-advanced: an `ORDER BY … LIMIT` query executes and returns a
/// well-formed, bounded `RESULT_SET`. Self-contained (queries whatever EHRs the
/// shared SUT holds; the LIMIT bound is the deterministic assertion).
fn run_advanced_order_limit<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // AQL 1.1 clause order: orderByClause? limitClause? (AqlParser.g4).
        let aql = "SELECT e/ehr_id/value FROM EHR e ORDER BY e/ehr_id/value ASC LIMIT 5";
        let resp = adhoc(ctx, aql).await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        if result_set::meta_type(&body) != Some("RESULTSET") {
            return Err(CaseError::Assertion(format!(
                "advanced ORDER BY+LIMIT: expected RESULTSET, got {:?}",
                result_set::meta_type(&body)
            )));
        }
        let rows = result_set::row_count(&body);
        if rows > 5 {
            return Err(CaseError::Assertion(format!(
                "LIMIT 5 was not honoured: RESULT_SET has {rows} rows"
            )));
        }
        Ok(DataSetReport::SINGLE)
    })
}

// ── golden + fixture helpers ────────────────────────────────────────────────

/// Diff a served `RESULT_SET` against a golden through the shared normalizer,
/// mapping a mismatch to a finding that names the suppression rules applied.
fn diff_golden(
    golden: &serde_json::Value,
    actual: &serde_json::Value,
    mode: Mode,
    what: &str,
) -> Result<DataSetReport, CaseError> {
    let cmp = query_golden::compare(golden, actual, mode);
    if cmp.matched {
        Ok(DataSetReport::SINGLE)
    } else {
        Err(CaseError::Assertion(format!(
            "{what} golden mismatch (suppressed via [{}]): {}",
            cmp.applied_labels(),
            cmp.detail.unwrap_or_default()
        )))
    }
}

/// A canonical-JSON composition fixture by name (from the corpus).
fn compositions_by_name(name: &str) -> Result<serde_json::Value, CaseError> {
    let comps = fixtures::compositions_canonical_json().map_err(|e| codec(&e))?;
    let fixture = comps
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| CaseError::Assertion(format!("missing composition fixture {name}")))?;
    fixture.json().map_err(|e| codec(&e))
}
