//! AQL terminology functions — the `TERMINOLOGY()` function family as an AQL
//! **language** feature (area `Aqt`; OPTIONS capability
//! [`Capability::AqlTerminology`]).
//!
//! This area evidences the profiles master03 §Functional *Querying* **"AQL &
//! terminology"** OPTIONS capability, which the shipped instrument otherwise
//! leaves unevidenced: the existing terminology suite
//! ([`crate::suites::terminology`], area `Ts`) exercises the terminology-server
//! *integration* surface (FHIR-tx provider + fault injection + the `openehr`
//! bundle flavour) under the reported-only [`Capability::Terminology`], whereas
//! the profile-gating [`Capability::AqlTerminology`] had no cases. These cases
//! assert the AQL language contract of `QUERY master03-syntax.adoc §TERMINOLOGY`
//! (lines 699–767) directly and carry that capability, so a green run makes
//! OPTIONS-OBTAINED genuinely cover `AqlTerminology`.
//!
//! Oracle: `docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc §TERMINOLOGY`
//! together with `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc`
//! §Functional Querying "AQL & terminology" (OPTIONS). The CNF Platform
//! Conformance Test Schedule defines **no** terminology-function test case
//! (grep-confirmed empty in master05/master11), so every case is
//! [`ScheduleTrace::EccOriginal`].
//!
//! ## Envelope under test (verified against the engine)
//!
//! Only `TERMINOLOGY('expand', …)` as a `matches` operand (spec usage (a)/(b))
//! is implemented — merged into an explicit code list at semantic analysis;
//! against the composed SUT it resolves the in-process `openehr` bundle (no
//! external terminology server is configured for the conformance stack). Every
//! other form is a **typed rejection** — a `400 Bad Request` with the
//! `{ error, message }` body, never a `500` and never a silently-empty
//! `RESULT_SET`:
//!
//! - a non-`expand` operation as a `matches` operand (`lookup`/`map`),
//! - `TERMINOLOGY()` in an unsupported position (a `SELECT` column),
//! - a Boolean-assertion form with an unsupported operation
//!   (`TERMINOLOGY('lookup', …) = true`).
//!
//! (The accepted Boolean forms `validate`/`subsumes` route to a terminology
//! service; against the provider-less conformance stack they are not a stable
//! green, so this area asserts the rejects of the unsupported operations and the
//! one bundle-backed `expand` accept — the FHIR-service forms stay in the `Ts`
//! area's SUT-config-gated cases.)

use serde_json::Value;

use crate::engine::assert;
use crate::engine::harness::{
    CaseError, CaseFuture, DataSetReport, HttpRequest, HttpResponse, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::testdata::fixtures;
use crate::wire::negotiate;

/// JSON is the wire format the AQL terminology cases run under.
const JSON: &[Format] = &[Format::Json];

/// The in-process openEHR terminology bundle `service_api` (an ehrbase-rs
/// engine extension — the spec defines the `service_api` mechanism, not an
/// in-process `openehr` flavour; the composed conformance stack has no external
/// terminology server, so the bundle is the only resolvable expand source).
const OPENEHR: &str = "openehr";
/// The bundle value set of `COMPOSITION.category` codes (includes `433` event).
const VS_CATEGORY: &str = "composition_category";
/// The nested event OPT + its canonical composition (category `433`).
const NESTED_OPT: &str = "nested.template.opt";
const NESTED_JSON: &str = "nested.composition.json";

/// The citation shared by every AQL-terminology case.
const CITE: &str = "QUERY master03-syntax §Functions/Other functions/TERMINOLOGY (lines 699–767); AQL 1.1; \
     ITS-REST 1.0.3 QUERY API execute_ad_hoc_query (200_QUERY / 400_QUERY); profiles master03 \
     §Functional Querying 'AQL & terminology' (OPTIONS)";

/// The single ECC-original reason: no CNF chapter defines terminology-function
/// cases; ECC-derived from the AQL spec + the OPTIONS profile capability.
const ECC_REASON: &str = "the CNF Platform Conformance Test Schedule defines no terminology-function test \
     case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles \
     master03 §Functional Querying 'AQL & terminology' (OPTIONS).";

/// Every registered AQL-terminology case.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── accept: expand over committed data (the OPTIONS evidence) ──────────
        case(
            "aqt/expand-matches-over-committed-data",
            "TERMINOLOGY('expand') as a matches operand filters committed compositions by the value set's codes",
            Binding::Rest(
                "POST /ehr/{ehr_id}/composition; POST /query/aql (matches TERMINOLOGY('expand', …))",
            ),
            run_expand_over_committed_data,
        ),
        // ── reject: unsupported forms are typed 400s (never 500, never empty) ──
        case(
            "aqt/reject-unsupported-operation-in-matches",
            "A non-expand TERMINOLOGY operation as a matches operand (lookup/map) → 400",
            Binding::Rest("POST /query/aql (matches TERMINOLOGY('lookup'|'map', …))"),
            run_reject_unsupported_operation_matches,
        ),
        case(
            "aqt/reject-terminology-in-select",
            "TERMINOLOGY() in an unsupported position (a SELECT column) → 400",
            Binding::Rest("POST /query/aql (SELECT TERMINOLOGY('expand', …))"),
            run_reject_terminology_in_select,
        ),
        case(
            "aqt/reject-boolean-unsupported-operation",
            "A Boolean TERMINOLOGY assertion with an unsupported operation (lookup) → 400",
            Binding::Rest("POST /query/aql (WHERE TERMINOLOGY('lookup', …) = true)"),
            run_reject_boolean_unsupported_operation,
        ),
    ]
}

/// Assemble an AQL-terminology case entry: area `Aqt`, capability
/// [`Capability::AqlTerminology`], JSON axis, ECC-original (OPTIONS).
fn case(
    id: &'static str,
    title: &'static str,
    binding: Binding,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Aqt,
            capability: Capability::AqlTerminology,
            formats: JSON,
            citation: CITE,
            schedule: ScheduleTrace::EccOriginal(ECC_REASON),
            binding,
            compare: Compare::None,
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

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Execute an ad-hoc AQL query (`POST /query/aql`, body `{"q": …}`).
async fn adhoc(ctx: &RunContext<'_>, aql: &str) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::accept(
        HttpRequest::post("/query/aql").json_body(&serde_json::json!({ "q": aql }))?,
        Format::Json,
    ))
    .await
}

/// The row count of a `RESULT_SET` body.
fn row_count(body: &Value) -> usize {
    body["rows"].as_array().map_or(0, Vec::len)
}

/// Commit the nested event composition (category `433`) to a fresh EHR, tolerant
/// of an OPT re-upload on the shared SUT; returns the `ehr_id`.
async fn commit_nested_composition(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let xml = fixtures::read(NESTED_OPT).map_err(|e| codec(&e))?;
    support::ensure_opt_xml(ctx, &xml).await?;
    let body = fixtures::read_json(NESTED_JSON).map_err(|e| codec(&e))?;
    let resp = ctx
        .send(negotiate::representation(
            HttpRequest::post(format!("/ehr/{ehr_id}/composition")).json_body(&body)?,
            Format::Json,
        ))
        .await?;
    assert::status(&resp, 201)?;
    Ok(ehr_id)
}

/// Assert a query was a **typed** rejection: `400 Bad Request` carrying a
/// non-empty `{ error | message }` body — proving the unsupported form is
/// neither a `500` server fault nor a silently-empty `200 RESULT_SET`.
fn assert_typed_reject(resp: &HttpResponse) -> Result<(), CaseError> {
    assert::status(resp, 400)?;
    let body = resp.json()?;
    let non_empty = |k: &str| {
        body.get(k)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
    };
    if non_empty("message") || non_empty("error") {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "400 reject carries no {{error|message}} body: {body}"
        )))
    }
}

// ── accept ─────────────────────────────────────────────────────────────────

/// Commit a category-`433` composition, then a `matches TERMINOLOGY('expand',
/// 'openehr', 'composition_category')` predicate over it returns the row (the
/// value set contains `433`) — the master03 usage-(a) positive contract, and the
/// `AqlTerminology` OPTIONS evidence.
fn run_expand_over_committed_data<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr = commit_nested_composition(ctx).await?;
        let aql = format!(
            "SELECT c/uid/value FROM EHR e[ehr_id/value='{ehr}'] CONTAINS COMPOSITION c \
             WHERE c/category/defining_code/code_string \
             matches TERMINOLOGY('expand', '{OPENEHR}', '{VS_CATEGORY}')"
        );
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        if body["meta"]["_type"] != "RESULTSET" {
            return Err(CaseError::Assertion(format!(
                "expected meta._type RESULTSET, got {}",
                body["meta"]["_type"]
            )));
        }
        if row_count(&body) == 0 {
            return Err(CaseError::Assertion(
                "composition_category expansion did not match the committed category-433 composition"
                    .to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

// ── rejects (typed 400) ──────────────────────────────────────────────────────

/// A non-`expand` operation as a `matches` operand — `lookup` and `map` have no
/// code-list semantics — is a typed `400` (never a silently-empty result set).
fn run_reject_unsupported_operation_matches<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let mut checks = 0u32;
        for op in ["lookup", "map"] {
            let aql = format!(
                "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c \
                 WHERE c/category/defining_code/code_string \
                 matches TERMINOLOGY('{op}', '{OPENEHR}', '{VS_CATEGORY}')"
            );
            let resp = adhoc(ctx, &aql).await?;
            assert_typed_reject(&resp).map_err(|e| label(op, e))?;
            checks += 1;
        }
        Ok(DataSetReport::all(checks))
    })
}

/// `TERMINOLOGY()` in a `SELECT` column is an unsupported position → typed `400`.
fn run_reject_terminology_in_select<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let aql = format!("SELECT TERMINOLOGY('expand', '{OPENEHR}', '{VS_CATEGORY}') FROM EHR e");
        let resp = adhoc(ctx, &aql).await?;
        assert_typed_reject(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// A Boolean-assertion `TERMINOLOGY(op, …) = true` with an unsupported operation
/// (`lookup`; only `validate`/`subsumes` have Boolean semantics) → typed `400`.
fn run_reject_boolean_unsupported_operation<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let aql = format!(
            "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c \
             WHERE TERMINOLOGY('lookup', '{OPENEHR}', 'system=openehr&code=433') = true"
        );
        let resp = adhoc(ctx, &aql).await?;
        assert_typed_reject(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// Prefix a failure with the operation under test.
fn label(op: &str, e: CaseError) -> CaseError {
    match e {
        CaseError::Assertion(m) => CaseError::Assertion(format!("TERMINOLOGY('{op}'): {m}")),
        other => other,
    }
}
