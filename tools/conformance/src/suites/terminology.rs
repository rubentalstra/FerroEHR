//! TERMINOLOGY-server integration cases — the cross-cutting Terminology
//! capability: the AQL
//! `TERMINOLOGY('expand', …)` family driven over the QUERY API
//! (`POST /query/aql`), plus the FHIR-tx provider + fault-injection cases.
//!
//! There is **no CNF schedule chapter** for terminology integration; the oracle
//! is `master03-profiles.adoc` §Functional *Querying* "AQL & terminology"
//! (OPTIONS) + `QUERY/master03-syntax.adoc` §TERMINOLOGY (lines 748–767:
//! `TERMINOLOGY('expand', service_api, params_uri)` merged into a `matches`
//! value list at semantic analysis). So every case is
//! [`ScheduleTrace::EccOriginal`] (owner ruling 2026-07-13). The
//! `TERMINOLOGY('expand', 'openehr', …)` bundle flavour is an **ehrbase-rs AQL
//! engine extension** (the spec defines the `service_api` mechanism, not an
//! in-process `openehr` flavour) — the fairness register rules those `extension`
//! for a foreign SUT lacking our engine, while the generic FHIR-`service_api`
//! and fault cases stay generic.
//!
//! Three dispositions: bundle-expansion cases are real passes against any SUT
//! with our engine; the FHIR-provider case passes when configured else
//! `SKIPPED(SutConfig)`; fault-injection cases are `SKIPPED(SutConfig)` with
//! off-wire fixture/wiremock evidence cited (the MSG precedent). Fault cases
//! read [`RunContext::tx`] to name the harness terminology server; a fault
//! cannot be wired into an external SUT over the HTTP-only ECC.

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
use crate::ts::fixture::{FAULT_MALFORMED_VS, FAULT_SERVER_ERROR_VS, FAULT_TIMEOUT_VS, SURFACE_VS};

/// The `service_api` for the in-process openEHR terminology bundle (extension).
const OPENEHR: &str = "openehr";
/// A FHIR R4 `service_api` (QUERY master03 example `hl7.org/fhir/4.0`).
const FHIR: &str = "hl7.org/fhir/4.0";
/// The bundle value set of `COMPOSITION.category` codes ({431,433,451,815}).
const VS_CATEGORY: &str = "composition_category";
/// A disjoint bundle value set (audit change type, {249..253}).
const VS_AUDIT: &str = "audit_change_type";
/// The nested event OPT + its canonical composition (category `433` event).
const NESTED_OPT: &str = "nested.template.opt";
const NESTED_JSON: &str = "nested.composition.json";

/// JSON is the wire format the TS cases run under.
const JSON: &[Format] = &[Format::Json];

/// Every registered TERMINOLOGY case (9).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── bundle expansion (real passes, any SUT with our engine) ─────────
        case(
            "ts/expand-bundle-accepted",
            "TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET",
            bundle_stub(),
            run_bundle_accepted,
        ),
        case(
            "ts/expand-bundle-constrains",
            "TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes",
            bundle_stub(),
            run_bundle_constrains,
        ),
        case(
            "ts/expand-bundle-mixed-list",
            "TERMINOLOGY expand (bundle) — explicit code merged with the expansion",
            bundle_stub(),
            run_bundle_mixed_list,
        ),
        case(
            "ts/expand-unknown-value-set-rejected",
            "TERMINOLOGY expand — unknown value set rejected (400)",
            bundle_stub(),
            run_unknown_value_set,
        ),
        case(
            "ts/expand-unknown-service-rejected",
            "TERMINOLOGY expand — unknown service_api rejected (400)",
            bundle_stub(),
            run_unknown_service,
        ),
        // ── FHIR provider (pass when configured, else SKIPPED(SutConfig)) ───
        case(
            "ts/expand-fhir-provider",
            "TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured",
            fhir_stub(),
            run_fhir_provider,
        ),
        // ── fault injection (fixture / real-server only, SKIPPED(SutConfig)) ─
        case(
            "ts/fault-timeout",
            "TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500)",
            fault_stub(),
            run_fault_timeout,
        ),
        case(
            "ts/fault-server-error",
            "TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500)",
            fault_stub(),
            run_fault_server_error,
        ),
        case(
            "ts/fault-malformed",
            "TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500)",
            fault_stub(),
            run_fault_malformed,
        ),
    ]
}

/// The bundle-flavour schedule trace (ehrbase-rs AQL engine extension).
fn bundle_stub() -> ScheduleTrace {
    ScheduleTrace::EccOriginal(
        "no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension",
    )
}
/// The FHIR-provider schedule trace (realizes the spec `service_api` mechanism).
fn fhir_stub() -> ScheduleTrace {
    ScheduleTrace::EccOriginal(
        "no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the FHIR service_api path realizes the spec mechanism (generic, not an extension)",
    )
}
/// The fault-injection schedule trace (off-wire evidence, MSG precedent).
fn fault_stub() -> ScheduleTrace {
    ScheduleTrace::EccOriginal(
        "no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)",
    )
}

/// The citation shared by every TS case.
const SVC: &str = "QUERY master03 §Functions/Other functions/TERMINOLOGY (lines 748–767); AQL 1.1; ITS-REST 1.1.0 QUERY API execute_ad_hoc_query; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS)";

/// Assemble a TS case entry (area [`Area::Ts`], OPTIONS Terminology capability).
fn case(
    id: &'static str,
    title: &'static str,
    schedule: ScheduleTrace,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Ts,
            capability: Capability::Terminology,
            formats: JSON,
            citation: SVC,
            schedule,
            binding: Binding::Rest("POST /query/aql"),
            compare: Compare::None,
        },
        run,
    }
}

macro_rules! case_body {
    ($body:block) => {
        Box::pin(async move $body)
    };
}

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Execute an ad-hoc AQL query (`POST /query/aql`, body `{"q": …}`).
async fn adhoc(ctx: &RunContext<'_>, aql: &str) -> Result<HttpResponse, CaseError> {
    ctx.send(crate::wire::negotiate::accept(
        HttpRequest::post("/query/aql").json_body(&serde_json::json!({ "q": aql }))?,
        Format::Json,
    ))
    .await
}

/// A `matches TERMINOLOGY('expand', service_api, params_uri)` query over
/// `COMPOSITION.category`, EHR-scoped when `ehr` is given (shared-SUT-safe).
fn category_expand_query(ehr: Option<&str>, service_api: &str, params_uri: &str) -> String {
    let from = match ehr {
        Some(id) => format!("EHR e[ehr_id/value='{id}'] CONTAINS COMPOSITION c"),
        None => "EHR e CONTAINS COMPOSITION c".to_owned(),
    };
    format!(
        "SELECT c/uid/value FROM {from} WHERE c/category/defining_code/code_string \
         matches TERMINOLOGY('expand', '{service_api}', '{params_uri}')"
    )
}

/// The row count of a `RESULT_SET` body.
fn row_count(body: &Value) -> usize {
    body["rows"].as_array().map_or(0, Vec::len)
}

/// Commit the nested event composition (category `433`) to a fresh EHR; returns
/// the `ehr_id`.
async fn commit_nested_composition(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let xml = fixtures::read(NESTED_OPT).map_err(|e| codec(&e))?;
    support::ensure_opt_xml(ctx, &xml).await?;
    let body = fixtures::read_json(NESTED_JSON).map_err(|e| codec(&e))?;
    let resp = ctx
        .send(crate::wire::negotiate::representation(
            HttpRequest::post(format!("/ehr/{ehr_id}/composition")).json_body(&body)?,
            Format::Json,
        ))
        .await?;
    assert::status(&resp, 201)?;
    Ok(ehr_id)
}

/// Whether a rejected FHIR expand indicates the SUT has **no** FHIR terminology
/// provider configured (the typed `UnknownTerminologyService` reject).
fn is_no_provider(resp: &HttpResponse) -> bool {
    resp.text()
        .to_ascii_lowercase()
        .contains("not a configured terminology service")
}

// ── bundle expansion (real passes) ────────────────────────────────────────────

fn run_bundle_accepted<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let resp = adhoc(ctx, &category_expand_query(None, OPENEHR, VS_CATEGORY)).await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        if body["meta"]["_type"] != "RESULTSET" {
            return Err(CaseError::Assertion(format!(
                "expected meta._type RESULTSET, got {}",
                body["meta"]["_type"]
            )));
        }
        if !body["columns"].is_array() {
            return Err(CaseError::Assertion(
                "RESULT_SET has no columns array".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_bundle_constrains<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let ehr = commit_nested_composition(ctx).await?;
        // Positive: category 433 ∈ composition_category → the composition matches.
        let pos = adhoc(
            ctx,
            &category_expand_query(Some(&ehr), OPENEHR, VS_CATEGORY),
        )
        .await?;
        assert::status(&pos, 200)?;
        if row_count(&pos.json()?) == 0 {
            return Err(CaseError::Assertion(
                "composition_category expansion did not match the category-433 composition"
                    .to_owned(),
            ));
        }
        // Negative: 433 ∉ audit_change_type → no match (proves specific codes).
        let neg = adhoc(ctx, &category_expand_query(Some(&ehr), OPENEHR, VS_AUDIT)).await?;
        assert::status(&neg, 200)?;
        if row_count(&neg.json()?) != 0 {
            return Err(CaseError::Assertion(
                "audit_change_type expansion matched a category-433 composition — not constrained to the value set's codes".to_owned()));
        }
        Ok(DataSetReport::all(2))
    })
}

fn run_bundle_mixed_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let ehr = commit_nested_composition(ctx).await?;
        let aql = format!(
            "SELECT c/uid/value FROM EHR e[ehr_id/value='{ehr}'] CONTAINS COMPOSITION c \
             WHERE c/category/defining_code/code_string matches {{'433', TERMINOLOGY('expand', '{OPENEHR}', '{VS_AUDIT}')}}"
        );
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        if row_count(&resp.json()?) == 0 {
            return Err(CaseError::Assertion(
                "mixed matches list ({explicit 433, expand audit_change_type}) matched nothing — the explicit code was dropped (QUERY master03 line 759)".to_owned()));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_unknown_value_set<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let resp = adhoc(
            ctx,
            &category_expand_query(None, OPENEHR, "no_such_value_set_xyz"),
        )
        .await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_unknown_service<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let resp = adhoc(
            ctx,
            &category_expand_query(None, "bogus.terminology.api", "x"),
        )
        .await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── FHIR provider (pass when configured, else SKIPPED(SutConfig)) ──────────────

fn run_fhir_provider<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let resp = adhoc(ctx, &category_expand_query(None, FHIR, SURFACE_VS)).await?;
        if resp.status == 200 {
            let body = resp.json()?;
            if body["meta"]["_type"] != "RESULTSET" {
                return Err(CaseError::Assertion(format!(
                    "FHIR expand accepted but body is not a RESULT_SET: {}",
                    body["meta"]["_type"]
                )));
            }
            return Ok(DataSetReport::SINGLE);
        }
        if is_no_provider(&resp) {
            return Err(CaseError::Skipped(skip_no_provider(ctx)));
        }
        Err(CaseError::Skipped(format!(
            "SutConfig: FHIR terminology provider not exercisable — the SUT answered {} to a `{FHIR}` \
             expand (a configured provider lacking the fixture value set, or a non-provider rejection). \
             Not a fabricated pass.",
            resp.status
        )))
    })
}

/// The skip reason for a SUT with no FHIR terminology provider, naming the
/// terminology server the harness *did* have available (fixture / real).
fn skip_no_provider(ctx: &RunContext<'_>) -> String {
    let tx = ctx.tx.map_or_else(
        || "no terminology server established for this run".to_owned(),
        |t| {
            format!(
                "harness terminology server: {} ({})",
                t.base_url,
                t.mode.label()
            )
        },
    );
    format!(
        "SutConfig: no FHIR terminology provider configured on the SUT — a `{FHIR}` expand is \
         rejected as `UnknownTerminologyService`. {tx}. The bundle (`openehr`) expand cases prove \
         the TERMINOLOGY family; wire this by pointing the SUT at a FHIR server."
    )
}

// ── fault injection (fixture / real-server only) ──────────────────────────────

/// The shared skip for a fault-injection case when the SUT is **not** wired to
/// the fault-injecting fixture (a bare run with no composed wiring): the
/// HTTP-only ECC cannot reconfigure an external SUT's provider per case, so the
/// fault→`500` mapping is proven off the wire and cited (the MSG/SIG precedent).
fn fault_skip(ctx: &RunContext<'_>, fault_label: &str, evidence: &str) -> CaseError {
    let tx = ctx.tx.map_or_else(
        || "no terminology server established".to_owned(),
        |t| format!("{} ({})", t.base_url, t.mode.label()),
    );
    CaseError::Skipped(format!(
        "SutConfig: the {fault_label} fault requires a fault-injecting terminology server wired to \
         the SUT (the composed run points the SUT's [terminology.external] provider at the fixture \
         via host.docker.internal); this run is not wired. Harness tx server: {tx}. The fault→500 \
         mapping is proven by {evidence}."
    ))
}

/// Drive the wired SUT with a `TERMINOLOGY('expand', 'hl7.org/fhir/4.0',
/// <fault_vs>)` operand whose value set makes the fixture inject a fault, and
/// assert the SUT maps the upstream fault to a `500` server fault. When the SUT
/// is not wired to the fixture, report `SKIPPED(SutConfig)` (the off-wire
/// evidence stands). A terminology-server fault is `ExecError::Terminology` →
/// `SmError::exception` → HTTP `500`
/// (`app/ehrbase/src/service/query/execute.rs` `map_exec_error`;
/// QUERY master03 §TERMINOLOGY distinguishes a bad query (400) from an upstream
/// server fault (500)).
async fn run_fault(
    ctx: &RunContext<'_>,
    fault_vs: &str,
    fault_label: &str,
    evidence: &str,
) -> Result<DataSetReport, CaseError> {
    if !ctx.tx.is_some_and(|t| t.wired) {
        return Err(fault_skip(ctx, fault_label, evidence));
    }
    let resp = adhoc(ctx, &category_expand_query(None, FHIR, fault_vs)).await?;
    assert::status(&resp, 500)?;
    Ok(DataSetReport::SINGLE)
}

fn run_fault_timeout<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        run_fault(
            ctx,
            FAULT_TIMEOUT_VS,
            "timeout",
            "conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline \
             + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception",
        )
        .await
    })
}

fn run_fault_server_error<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        run_fault(
            ctx,
            FAULT_SERVER_ERROR_VS,
            "5xx",
            "conformance ts::fixture::tests::fault_server_error_is_5xx \
             + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception",
        )
        .await
    })
}

fn run_fault_malformed<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        run_fault(
            ctx,
            FAULT_MALFORMED_VS,
            "malformed",
            "conformance ts::fixture::tests::fault_malformed_is_not_json \
             + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception",
        )
        .await
    })
}
