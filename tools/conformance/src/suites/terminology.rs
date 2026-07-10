//! The `TS` capability cases — terminology-server integration (B4): the AQL
//! `TERMINOLOGY('expand', …)` family driven over the QUERY API (`POST
//! /query/aql`), plus the FHIR-tx provider and fault-injection cases.
//!
//! Spec grounding: QUERY master03 §Functions/Other functions/TERMINOLOGY
//! (`docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc` lines 748–767) —
//! `TERMINOLOGY('expand', service_api, params_uri)` used as (or inside) a
//! `matches` operand, "merging explicit codes with the function results … the
//! AQL interpreter is responsible for generating a valid list of codes during
//! semantic analysis" (lines 756–759). Design:
//! `docs/design/terminology-server-integration.md` §5.
//!
//! ## Three case families, three dispositions
//!
//! 1. **Bundle expansion (`service_api = "openehr"`).** Wire-driveable against
//!    **any** SUT with our engine: the in-process `openehr-term` bundle resolves
//!    the value set at semantic analysis and merges its codes into the `matches`
//!    value list — no external server. These are **real passes**.
//! 2. **FHIR provider (`service_api = "hl7.org/fhir/4.0"`).** The SUT routes to
//!    its configured FHIR terminology provider. Passes when a provider is
//!    configured (the query is accepted, `200`); with **no** provider the SUT
//!    returns a typed `400` (`UnknownTerminologyService`, "not a configured
//!    terminology service") and the case reports `SKIPPED(SutConfig)` — never a
//!    fabricated pass. The standard `scripts/conformance.sh` SUT configures no
//!    provider, so this skips there; a compose SUT pointed at a FHIR server
//!    (`EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_*`, using `host.docker.internal`
//!    to reach a runner-host fixture) exercises it.
//! 3. **Fault injection (timeout / `5xx` / malformed).** Real-server / fixture
//!    mode only: they need a *fault-injecting* terminology server wired to the
//!    SUT, which the HTTP-only ECC cannot arrange for an external SUT (the
//!    runner cannot reconfigure the SUT's provider URL per case). They report
//!    `SKIPPED(SutConfig)` and cite the evidence that *does* prove the
//!    fault-mapping — the [`crate::ts::fixture`] tests (the FHIR-tx fixture's
//!    own fault behaviour) and the CDR provider's `wiremock` fault tests
//!    (`app/ehrbase/tests/terminology_fhir.rs`) — so the behaviour is traceable
//!    even though it is off the wire (the MSG/SIG precedent).

use serde_json::Value;

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, HttpResponse, RunContext};
use crate::registry::CaseEntry;
use crate::suites::support;
use crate::ts::fixture::SURFACE_VS;

/// The `service_api` for the in-process openEHR terminology bundle (no external
/// server) — `crate::service::terminology`'s `BUNDLE_SERVICE_API`.
const OPENEHR: &str = "openehr";
/// A FHIR R4 `service_api` (master03 example `hl7.org/fhir/4.0`).
const FHIR: &str = "hl7.org/fhir/4.0";
/// The bundle value set of `COMPOSITION.category` codes ({431,433,451,815}).
const VS_CATEGORY: &str = "composition_category";
/// A disjoint bundle value set (audit change type, {249..253}) — no category
/// code is a member, so it proves the expansion produced *specific* codes.
const VS_AUDIT: &str = "audit_change_type";

/// The nested event OPT + its canonical composition (category = `433` event),
/// reused across the constraining cases (the proven master07/master11 fixture).
const NESTED_OPT: &str = "nested/nested.opt";
const NESTED_JSON: &str = "compositions/CANONICAL_JSON/nested.en.v1__full.json";

/// The implemented `TS` case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    const SVC: &str = "QUERY master03 §Functions/Other functions/TERMINOLOGY (lines 748–767); \
         AQL 1.1; ITS-REST 1.0.3 QUERY API §execute_ad_hoc_query";
    vec![
        // ── bundle expansion (real passes, any SUT) ──────────────────────────
        entry(
            "ts/expand-bundle-accepted",
            "TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET",
            SVC,
            run_bundle_accepted,
        ),
        entry(
            "ts/expand-bundle-constrains",
            "TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes",
            SVC,
            run_bundle_constrains,
        ),
        entry(
            "ts/expand-bundle-mixed-list",
            "TERMINOLOGY expand (bundle) — explicit code merged with the expansion (matches list)",
            SVC,
            run_bundle_mixed_list,
        ),
        entry(
            "ts/expand-unknown-value-set-rejected",
            "TERMINOLOGY expand — unknown value set rejected (400)",
            SVC,
            run_unknown_value_set,
        ),
        entry(
            "ts/expand-unknown-service-rejected",
            "TERMINOLOGY expand — unknown service_api rejected (400)",
            SVC,
            run_unknown_service,
        ),
        // ── FHIR provider (pass when configured, else SKIPPED(SutConfig)) ─────
        entry(
            "ts/expand-fhir-provider",
            "TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured",
            SVC,
            run_fhir_provider,
        ),
        // ── fault injection (fixture / real-server only, SKIPPED(SutConfig)) ──
        entry(
            "ts/fault-timeout",
            "TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500)",
            SVC,
            run_fault_timeout,
        ),
        entry(
            "ts/fault-server-error",
            "TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500)",
            SVC,
            run_fault_server_error,
        ),
        entry(
            "ts/fault-malformed",
            "TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500)",
            SVC,
            run_fault_malformed,
        ),
    ]
}

/// A `TS`-area case entry (OPTIONS-profile `Terminology` capability, JSON).
fn entry(
    id: &'static str,
    title: &'static str,
    citation: &'static str,
    run: crate::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Ts,
            capability: Capability::Terminology,
            profiles: &[Profile::Options],
            formats: &[Format::Json],
            citation,
            compare: Compare::IgnoreSet,
        },
        run,
    }
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── shared helpers ──────────────────────────────────────────────────────────

fn fx(e: fixtures::FixtureError) -> CaseError {
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

/// A `matches TERMINOLOGY('expand', service_api, params_uri)` query over
/// `COMPOSITION.category`, EHR-scoped when `ehr` is given (shared-SUT-safe).
fn category_expand_query(ehr: Option<&str>, service_api: &str, params_uri: &str) -> String {
    let from = match ehr {
        Some(id) => format!("EHR e[ehr_id/value='{id}'] CONTAINS COMPOSITION c"),
        None => "EHR e CONTAINS COMPOSITION c".to_owned(),
    };
    format!(
        "SELECT c/uid/value FROM {from} \
         WHERE c/category/defining_code/code_string \
         matches TERMINOLOGY('expand', '{service_api}', '{params_uri}')"
    )
}

/// The row count of a `RESULT_SET` body.
fn row_count(body: &Value) -> usize {
    body["rows"].as_array().map_or(0, Vec::len)
}

/// Commit the nested event composition (category `433`) to a fresh EHR, so its
/// `category` is queryable; returns the `ehr_id`.
async fn commit_nested_composition(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    support::ensure_opt(ctx, NESTED_OPT).await?;
    let body = fixtures::read_json(NESTED_JSON).map_err(fx)?;
    let resp = ctx
        .send(
            HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                .json_body(&body)?
                .header("accept", "application/json")
                .header("prefer", "return=representation"),
        )
        .await?;
    assert::status(&resp, 201)?;
    Ok(ehr_id)
}

/// Whether a rejected FHIR expand indicates the SUT has **no** FHIR terminology
/// provider configured (the typed `UnknownTerminologyService` reject) — vs any
/// other rejection. The message text is `AqlFeatureError::UnknownTerminologyService`
/// ("… is not a configured terminology service", `app/ehrbase/src/aql/error.rs`).
fn is_no_provider(resp: &HttpResponse) -> bool {
    resp.text()
        .to_ascii_lowercase()
        .contains("not a configured terminology service")
}

// ── bundle expansion (real passes) ────────────────────────────────────────────

/// The bundle `expand` operand is resolved + merged at semantic analysis and
/// the query is accepted with a well-formed `RESULT_SET` (a broken/absent
/// expander would reject it). Data-independent — passes against any SUT.
fn run_bundle_accepted<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let aql = category_expand_query(None, OPENEHR, VS_CATEGORY);
        let resp = adhoc(ctx, &aql).await?;
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

/// The expansion produces the value set's *specific* codes (not a wildcard):
/// against a fresh EHR holding one composition (category `433`), the
/// `composition_category` expansion (includes `433`) returns it, while the
/// disjoint `audit_change_type` expansion (excludes `433`) returns nothing.
/// (2 data sets.)
fn run_bundle_constrains<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
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
                "composition_category expansion did not match the category-433 composition \
                 (expansion produced no matching code)"
                    .to_owned(),
            ));
        }

        // Negative: 433 ∉ audit_change_type → no match (proves the expansion is
        // the value set's codes, not everything).
        let neg = adhoc(ctx, &category_expand_query(Some(&ehr), OPENEHR, VS_AUDIT)).await?;
        assert::status(&neg, 200)?;
        if row_count(&neg.json()?) != 0 {
            return Err(CaseError::Assertion(
                "audit_change_type expansion matched a category-433 composition — the expansion \
                 is not constrained to the value set's codes"
                    .to_owned(),
            ));
        }
        Ok(DataSetReport::all(2))
    })
}

/// A mixed `matches { <explicit>, TERMINOLOGY('expand', …) }` list merges the
/// explicit code with the expansion (master03 line 759). The explicit `433`
/// matches the committed composition even though the paired `audit_change_type`
/// expansion does not — proving the merge keeps both.
fn run_bundle_mixed_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr = commit_nested_composition(ctx).await?;
        let aql = format!(
            "SELECT c/uid/value FROM EHR e[ehr_id/value='{ehr}'] CONTAINS COMPOSITION c \
             WHERE c/category/defining_code/code_string \
             matches {{'433', TERMINOLOGY('expand', '{OPENEHR}', '{VS_AUDIT}')}}"
        );
        let resp = adhoc(ctx, &aql).await?;
        assert::status(&resp, 200)?;
        if row_count(&resp.json()?) == 0 {
            return Err(CaseError::Assertion(
                "mixed matches list ({explicit 433, expand audit_change_type}) matched nothing — \
                 the explicit code was dropped by the expansion merge"
                    .to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// An `expand` naming a value set the bundle does not know is a typed bad
/// request (`400` `TerminologyValueSetNotFound`), rejected at semantic
/// analysis. Data-independent — passes against any SUT with our engine.
fn run_unknown_value_set<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let aql = category_expand_query(None, OPENEHR, "no_such_value_set_xyz");
        let resp = adhoc(ctx, &aql).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// An `expand` naming an unrecognised `service_api` (no such flavour, no FHIR
/// provider) is a typed bad request (`400` `UnknownTerminologyService`).
fn run_unknown_service<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let aql = category_expand_query(None, "bogus.terminology.api", "x");
        let resp = adhoc(ctx, &aql).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── FHIR provider (pass when configured, else SKIPPED(SutConfig)) ──────────────

/// Drive an `expand` through the FHIR `service_api`. A `200` proves the SUT has
/// a FHIR provider configured and the expansion was accepted; a `400`
/// `UnknownTerminologyService` proves it has none → `SKIPPED(SutConfig)`. Any
/// other outcome is skipped with the observed status, never a fabricated pass.
fn run_fhir_provider<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let aql = category_expand_query(None, FHIR, SURFACE_VS);
        let resp = adhoc(ctx, &aql).await?;
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
            "SutConfig: FHIR terminology provider not exercisable — the SUT answered \
             {} to a `{FHIR}` expand (a configured provider that lacks the fixture value set, \
             or a non-provider rejection). Not a fabricated pass.",
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
        "SutConfig: no FHIR terminology provider configured on the SUT \
         (EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_* unset) — a `{FHIR}` expand is rejected as \
         `UnknownTerminologyService`. {tx}. The bundle (`openehr`) expand cases prove the \
         TERMINOLOGY family; wire this by pointing the SUT at a FHIR server \
         (host.docker.internal for a runner-host fixture, docs/design/terminology-server-integration.md §5)."
    )
}

// ── fault injection (fixture / real-server only) ──────────────────────────────

/// The shared skip for a fault-injection case: the HTTP-only ECC cannot wire a
/// *fault-injecting* terminology server into an external SUT per case, so the
/// fault→`500` mapping is proven off the wire — by the FHIR-tx fixture's own
/// fault tests and the CDR provider's `wiremock` fault tests. Never a fabricated
/// pass (the MSG/SIG precedent).
fn fault_skip(ctx: &RunContext<'_>, fault_label: &str, evidence: &str) -> CaseError {
    let tx = ctx.tx.map_or_else(
        || "no terminology server established".to_owned(),
        |t| format!("{} ({})", t.base_url, t.mode.label()),
    );
    CaseError::Skipped(format!(
        "SutConfig: the {fault_label} fault requires a fault-injecting terminology server wired \
         to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC \
         cannot reconfigure an external SUT's provider per case. Harness tx server: {tx}. \
         The fault→500 mapping is proven by {evidence}."
    ))
}

fn run_fault_timeout<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        Err::<DataSetReport, _>(fault_skip(
            ctx,
            "timeout",
            "conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline \
             + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception",
        ))
    })
}

fn run_fault_server_error<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        Err::<DataSetReport, _>(fault_skip(
            ctx,
            "5xx",
            "conformance ts::fixture::tests::fault_server_error_is_5xx \
             + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception",
        ))
    })
}

fn run_fault_malformed<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        Err::<DataSetReport, _>(fault_skip(
            ctx,
            "malformed",
            "conformance ts::fixture::tests::fault_malformed_is_not_json \
             + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception",
        ))
    })
}
