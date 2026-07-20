//! Simplified Formats wire surface — FLAT / STRUCTURED / Web Template
//! (area `Sf`). The acceptance oracle is the STABLE ITS-REST Simplified
//! Formats specification:
//!
//! - `docs/specs/openehr/ITS-REST/docs/simplified_formats/master04-basic_concepts.adoc`
//!   (format variants, the `ctx/` context block, the `|other` open-value-set
//!   suffix, the §Validation "field identifiers match WT metadata" rule),
//! - `.../master05-rm_mapping.adoc` (§scope: the mapping covers COMPOSITION and
//!   every class reachable from it, and nothing else),
//! - `.../master06-context_information.adoc` (the `ctx/` vocabulary + defaults:
//!   `ctx/time` → `COMPOSITION.context.start_time`; `ctx/setting` defaults to
//!   `openehr::238|other care|`),
//! - `.../specifications/docs/overview/Resources.md` §Simplified Formats (the
//!   three live media types + the 406/415 MUST rules + the deprecation NOTE)
//!   and §Alternative data formats (the legacy `nc.flat`/`tds2` forms),
//! - `.../specifications/docs/overview/Requests_and_responses.md`
//!   §openehr-template-id (the header is THE template-resolution mechanism for
//!   a simplified COMPOSITION commit) and §HTTP status codes (row `422`),
//! - `.../specifications/operations/contribution_create.yaml` §Simplified
//!   Formats (the envelope stays canonical; only `versions[i].data` is
//!   simplified).
//!
//! Every case is [`ScheduleTrace::EccOriginal`]: the CNF Platform Conformance
//! Test Schedule (`docs/specs/openehr/CNF/docs/platform_test_schedule/`) has no
//! simplified-formats chapter, so these are ECC-derived from the ITS-REST
//! Simplified Formats spec above, spec-silence flagged. Simplified Formats is a
//! SHOULD in Resources.md §67-68, so the capability is OPTIONS-level
//! ([`Capability::SimplifiedFormats`]) — it never gates CORE/STANDARD.
//!
//! Wire ids come only from [`crate::wire::ids`]; the constraining OPT
//! (`time_series.en.v1`) and its vendored FLAT instance are provisioned through
//! the fixture manifest, and the STRUCTURED body is derived from that FLAT
//! instance through the same `openehr-flat` converters the SUT uses — no
//! hand-pasted payloads.

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::engine::assert;
use crate::engine::harness::{
    CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, HttpResponse, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::testdata::fixtures;
use crate::wire::ids;

/// JSON is the runner's negotiation axis; the case drives the simplified media
/// types explicitly via request headers (the FLAT/STRUCTURED wire is JSON, and
/// the schedule tabulates no format-sensitive axis for this surface).
const JSON: &[Format] = &[Format::Json];

/// The single ECC-original reason shared by every case in this area — the CNF
/// schedule defines no simplified-formats chapter, so the whole area is derived
/// from the ITS-REST Simplified Formats spec.
const ECC_REASON: &str = "the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; \
     ECC-derived from the STABLE ITS-REST Simplified Formats specification \
     (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + \
     specifications/docs/overview/Resources.md §Simplified Formats)";

// ── media types (Resources.md §Simplified Formats + §Alternative data formats) ──

/// FLAT (simSDT) media type (Resources.md §Simplified Formats).
const FLAT: &str = "application/openehr.wt.flat+json";
/// STRUCTURED (structSDT) media type (Resources.md §Simplified Formats).
const STRUCTURED: &str = "application/openehr.wt.structured+json";
/// Web Template document media type (Resources.md §Simplified Formats).
const WT: &str = "application/openehr.wt+json";
/// Canonical JSON.
const JSON_MT: &str = "application/json";
/// Canonical XML.
const XML_MT: &str = "application/xml";

/// The retired media types every one of which MUST fail — both the deprecated
/// `.schema+json` names (Resources.md §Simplified Formats NOTE: "now deprecated
/// and will be removed in a future version") and the legacy alternatives
/// (Resources.md §Alternative data formats: ECISFLAT + TDS2, not implemented).
/// Each is proven to fail as an `Accept` (`406`) and as a write `Content-Type`
/// (`415`), in both directions.
const RETIRED_MEDIA_TYPES: &[&str] = &[
    // Deprecated (Resources.md §Simplified Formats NOTE).
    "application/openehr.wt.flat.schema+json",
    "application/openehr.wt.structured.schema+json",
    // Legacy / experimental (Resources.md §Alternative data formats).
    "application/openehr.nc.flat+json",
    "application/openehr.tds2+xml",
];

// ── fixtures: the time_series template + its vendored FLAT instance ──────────

/// The manifest key for the `time_series` constraining OPT (ADL 1.4 XML).
const TS_OPT_KEY: &str = "template.time-series.opt";
/// The manifest key for the vendored `time_series` FLAT (simSDT) instance.
const TS_FLAT_KEY: &str = "composition.flat.time-series";
/// The `time_series` OPT dir + file under the `template.valid` corpus-dir key,
/// used for the FLAT/STRUCTURED provisioning through [`fixtures::flat_to_canonical`].
const TS_CANONICAL: (&str, &str) = (TS_OPT_KEY, TS_FLAT_KEY);

// ── catalogue ────────────────────────────────────────────────────────────────

/// Every registered Simplified-Formats case.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the registered ECC case table is inherently enumerative"
)]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── commit + read-back (both simplified wire forms round-trip) ────────
        case(
            "sf/flat-commit-read-back",
            "FLAT commit then read-back as FLAT, canonical JSON, and STRUCTURED",
            "ITS-REST simplified_formats master04 §Flat format + master05 §RM Mapping; \
             Resources.md §Simplified Formats (application/openehr.wt.flat+json); \
             Requests_and_responses.md §openehr-template-id",
            Binding::Rest(
                "POST /ehr/{ehr_id}/composition (Content-Type application/openehr.wt.flat+json) \
                 → GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept flat/json/structured)",
            ),
            run_flat_commit_read_back,
        ),
        case(
            "sf/structured-commit-read-back",
            "STRUCTURED commit then read-back as STRUCTURED, canonical JSON, and FLAT",
            "ITS-REST simplified_formats master04 §Structured format + master05 §RM Mapping; \
             Resources.md §Simplified Formats (application/openehr.wt.structured+json); \
             Requests_and_responses.md §openehr-template-id",
            Binding::Rest(
                "POST /ehr/{ehr_id}/composition (Content-Type application/openehr.wt.structured+json) \
                 → GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept structured/json/flat)",
            ),
            run_structured_commit_read_back,
        ),
        // ── negotiation strictness ────────────────────────────────────────────
        case(
            "sf/negotiation-qvalue",
            "Accept q-values select the highest-weight simplified format; every non-204 carries Content-Type",
            "Resources.md §Data representation (RFC 9110 §12.5.1 quality-value negotiation) + \
             §Simplified Formats (Content-Type MUST be present unless 204)",
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept with q-values)"),
            run_negotiation_qvalue,
        ),
        case(
            "sf/reject-retired-media-type-accept",
            "Deprecated + legacy simplified media types are rejected on Accept (406)",
            "Resources.md §Simplified Formats NOTE (deprecated .schema+json) + §Alternative data formats \
             (legacy nc.flat/tds2) + the 406 MUST rule (\"If the service cannot fulfill this aspect of the \
             request, it MUST respond with 406 Not Acceptable\")",
            Binding::Rest(
                "GET /ehr/{ehr_id}/composition/{uid_based_id} + \
                 GET /definition/template/adl1.4/{template_id}/example (Accept a retired type)",
            ),
            run_reject_retired_accept,
        ),
        case(
            "sf/reject-retired-media-type-content-type",
            "Deprecated + legacy simplified media types are rejected on write Content-Type (415)",
            "Resources.md §Simplified Formats NOTE (deprecated .schema+json) + §Alternative data formats \
             (legacy nc.flat/tds2) + the 415 MUST rule (\"If the service cannot process the request payload \
             as the simplified format is not supported, it MUST respond with 415 Unsupported Media Type\")",
            Binding::Rest(
                "POST /ehr/{ehr_id}/composition + POST /ehr/{ehr_id}/contribution \
                 (Content-Type a retired type)",
            ),
            run_reject_retired_content_type,
        ),
        // ── header rule (openehr-template-id) ─────────────────────────────────
        case(
            "sf/flat-missing-template-id",
            "FLAT commit without openehr-template-id (and no payload template id) → 422",
            "Requests_and_responses.md §openehr-template-id (\"MUST be used whenever committing COMPOSITION \
             using a Simplified Format which does not support TEMPLATE_ID under archetype_details.template_id\") \
             + §HTTP status codes row 422 (well-formed but unprocessable)",
            Binding::Rest(
                "POST /ehr/{ehr_id}/composition (Content-Type flat, no openehr-template-id header)",
            ),
            run_flat_missing_template_id,
        ),
        // ── reject rules (master04 §Validation + §Open Value-Sets) ────────────
        case(
            "sf/flat-reject-unknown-field",
            "FLAT commit with an unknown field identifier → 422",
            "ITS-REST simplified_formats master04 §Validation (\"Field identifiers match WT metadata \
             structure\"); Requests_and_responses.md §HTTP status codes row 422",
            Binding::Rest("POST /ehr/{ehr_id}/composition (Content-Type flat, unknown field id)"),
            run_flat_reject_unknown_field,
        ),
        case(
            "sf/flat-reject-other-with-code",
            "FLAT commit with |other combined with |code on one coded leaf → 422",
            "ITS-REST simplified_formats master04 §Open Value-Sets and the |other Suffix (\"|other is \
             mutually exclusive with |code, |value and |terminology on the same leaf path; servers MUST \
             reject combinations\"); Requests_and_responses.md §HTTP status codes row 422",
            Binding::Rest(
                "POST /ehr/{ehr_id}/composition (Content-Type flat, |other + |code on one leaf)",
            ),
            run_flat_reject_other_with_code,
        ),
        // ── template surfaces ─────────────────────────────────────────────────
        case(
            "sf/template-web-template-get",
            "GET a template as a Web Template document (Accept application/openehr.wt+json)",
            "Resources.md §Simplified Formats (application/openehr.wt+json = the OPT as a Web Template JSON \
             document); ITS-REST simplified_formats master04 §Node ID Generation Rules (WT tree shape)",
            Binding::Rest(
                "GET /definition/template/adl1.4/{template_id} (Accept application/openehr.wt+json)",
            ),
            run_template_web_template_get,
        ),
        case(
            "sf/template-example-accept-forms",
            "GET a template example in each of the four Accept forms (json, xml, flat, structured)",
            "Resources.md §Data representation + §Simplified Formats (the LOCATABLE example is negotiable \
             across canonical JSON/XML and the FLAT/STRUCTURED simplified forms); the Content-Type MUST match \
             the negotiated format",
            Binding::Rest(
                "GET /definition/template/adl1.4/{template_id}/example (Accept json/xml/flat/structured)",
            ),
            run_template_example_accept_forms,
        ),
        case(
            "sf/template-example-unsupported-accept",
            "GET a template example with an unsupported Accept → 406",
            "Resources.md §Simplified Formats 406 MUST rule (the example endpoint offers canonical JSON/XML + \
             FLAT/STRUCTURED only; the Web Template media type is not a LOCATABLE representation)",
            Binding::Rest(
                "GET /definition/template/adl1.4/{template_id}/example (Accept application/openehr.wt+json)",
            ),
            run_template_example_unsupported_accept,
        ),
        // ── CONTRIBUTION: envelope canonical, inner data simplified ───────────
        case(
            "sf/contribution-flat-commit-read-back",
            "CONTRIBUTION with a FLAT COMPOSITION inner payload: canonical envelope in, simplified read-back",
            "ITS-REST contribution_create.yaml + contribution_get.yaml §Simplified Formats (the CONTRIBUTION \
             envelope stays canonical JSON; each versions[i].data COMPOSITION is simplified); \
             simplified_formats master05 §scope (COMPOSITION + contained classes only)",
            Binding::Rest(
                "POST /ehr/{ehr_id}/contribution (Content-Type flat) → \
                 GET /ehr/{ehr_id}/contribution/{contribution_uid} (Accept flat)",
            ),
            run_contribution_flat_commit_read_back,
        ),
        // ── non-templated resources: uniform reject (master05 §scope) ─────────
        case(
            "sf/non-templated-ehr-status-reject",
            "EHR_STATUS has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415",
            "ITS-REST simplified_formats master05 §scope (mappings exist for COMPOSITION and its contained \
             classes only; EHR_STATUS is not templated) + Resources.md §Simplified Formats 406/415 MUST rules",
            Binding::Rest(
                "GET /ehr/{ehr_id}/ehr_status (Accept flat) + PUT /ehr/{ehr_id}/ehr_status (Content-Type flat)",
            ),
            run_non_templated_ehr_status,
        ),
        case(
            "sf/non-templated-directory-reject",
            "DIRECTORY (FOLDER) has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415",
            "ITS-REST simplified_formats master05 §scope (FOLDER is not templated) + Resources.md \
             §Simplified Formats 406/415 MUST rules",
            Binding::Rest(
                "GET /ehr/{ehr_id}/directory (Accept flat) + POST /ehr/{ehr_id}/directory (Content-Type flat)",
            ),
            run_non_templated_directory,
        ),
        case(
            "sf/non-templated-demographic-reject",
            "Demographic PARTY has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415",
            "ITS-REST simplified_formats master05 §scope (demographic PARTY types are not templated) + \
             Resources.md §Simplified Formats 406/415 MUST rules",
            Binding::Rest(
                "GET /demographic/person/{uid} (Accept flat) + POST /demographic/person (Content-Type flat)",
            ),
            run_non_templated_demographic,
        ),
        // ── ctx observability (master06 §time + §setting) ─────────────────────
        case(
            "sf/ctx-observability",
            "FLAT ctx/time sets EVENT_CONTEXT.start_time; ctx/setting defaults to openehr::238",
            "ITS-REST simplified_formats master06 §time (ctx/time sets COMPOSITION.context.start_time) + \
             §setting (ctx/setting defaults to openehr::238|other care| when not set)",
            Binding::Rest(
                "POST /ehr/{ehr_id}/composition (Content-Type flat, ctx/time set) → \
                 GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept application/json)",
            ),
            run_ctx_observability,
        ),
    ]
}

/// Assemble a Simplified-Formats case entry: area `Sf`, capability
/// [`Capability::SimplifiedFormats`], JSON axis, ECC-original.
fn case(
    id: &'static str,
    title: &'static str,
    citation: &'static str,
    binding: Binding,
    run: CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Sf,
            capability: Capability::SimplifiedFormats,
            formats: JSON,
            citation,
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

// ── fixture helpers ────────────────────────────────────────────────────────

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

fn flat_err(e: &openehr_flat::error::FlatError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// The `time_series` OPT's ADL 1.4 XML.
fn ts_opt_xml() -> Result<String, CaseError> {
    fixtures::read(TS_OPT_KEY).map_err(|e| codec(&e))
}

/// The `template_id` declared inside the `time_series` OPT (read from the
/// artefact, never hardcoded).
fn ts_template_id(xml: &str) -> Result<String, CaseError> {
    openehr_its::opt14::from_xml(xml)
        .map(|opt| opt.template_id.value)
        .map_err(|e| CaseError::Codec(format!("parse time_series OPT: {e}")))
}

/// The `time_series` `WebTemplate`, built from its OPT — the same builder the
/// SUT uses (`state.backend().web_template`).
fn ts_web_template(xml: &str) -> Result<openehr_flat::webtemplate::WebTemplate, CaseError> {
    let opt = openehr_its::opt14::from_xml(xml)
        .map_err(|e| CaseError::Codec(format!("parse time_series OPT: {e}")))?;
    openehr_flat::webtemplate::build_web_template(&opt).map_err(|e| flat_err(&e))
}

/// The vendored `time_series` FLAT instance as a JSON object.
fn ts_flat_map() -> Result<Map<String, Value>, CaseError> {
    let text = fixtures::read(TS_FLAT_KEY).map_err(|e| codec(&e))?;
    serde_json::from_str(&text).map_err(|e| CaseError::Codec(e.to_string()))
}

/// The `time_series` composition in STRUCTURED form, derived from the vendored
/// FLAT instance through the same `openehr-flat` converters the SUT uses
/// (FLAT → canonical → STRUCTURED). No hand-authored STRUCTURED blob.
fn ts_structured_value() -> Result<Value, CaseError> {
    let xml = ts_opt_xml()?;
    let wt = ts_web_template(&xml)?;
    let canonical =
        fixtures::flat_to_canonical(TS_CANONICAL.0, TS_CANONICAL.1).map_err(|e| codec(&e))?;
    openehr_flat::convert::composition_to_structured(&canonical, &wt).map_err(|e| flat_err(&e))
}

/// Provision the `time_series` OPT (tolerant of a re-upload on the shared SUT),
/// returning its `template_id`.
async fn provision_ts_opt(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let xml = ts_opt_xml()?;
    let template_id = ts_template_id(&xml)?;
    support::ensure_opt_xml(ctx, &xml).await?;
    Ok(template_id)
}

// ── request builders ────────────────────────────────────────────────────────

/// POST a simplified COMPOSITION body of `media_type`, resolving the template
/// through the `openehr-template-id` header. No `Accept` → canonical-JSON
/// `return=minimal`, so the `201` carries the `ETag` the version-uid reader needs.
async fn post_simplified(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    media_type: &str,
    template_id: &str,
    body: String,
) -> Result<HttpResponse, CaseError> {
    ctx.send(
        HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
            .text_body(body, media_type)
            .header("openehr-template-id", template_id.to_owned()),
    )
    .await
}

/// GET a stored COMPOSITION with the given `Accept`.
async fn get_composition(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    object: &str,
    accept: &str,
) -> Result<HttpResponse, CaseError> {
    ctx.send(
        HttpRequest::get(format!("/ehr/{ehr_id}/composition/{object}")).header("accept", accept),
    )
    .await
}

// ── assertion helpers ────────────────────────────────────────────────────────

/// Assert a response carries a `Content-Type` header whose value starts with
/// `expected` (tolerating an appended `; charset`). Resources.md §Simplified
/// Formats: "Proper header `Content-Type` MUST be present in the response".
fn assert_content_type(resp: &HttpResponse, expected: &str) -> Result<(), CaseError> {
    match resp.header("content-type") {
        Some(v) if v.split(';').next().unwrap_or(v).trim() == expected => Ok(()),
        other => Err(CaseError::Assertion(format!(
            "expected Content-Type {expected:?}, got {other:?}"
        ))),
    }
}

/// Assert a client-error response carries a `Content-Type` and the server's
/// documented `{ error | message }` JSON body (overview error shape). Used on
/// the `406`/`415` rejections so the reject is proven to be a structured
/// response, not a bare status.
fn assert_error_body(resp: &HttpResponse) -> Result<(), CaseError> {
    assert::header_present(resp, "content-type")?;
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
            "reject response carries no {{error|message}} body: {body}"
        )))
    }
}

/// The value of the first flat key ending in `suffix`.
fn flat_leaf<'a>(map: &'a Value, suffix: &str) -> Option<&'a Value> {
    map.as_object()?
        .iter()
        .find(|(k, _)| k.ends_with(suffix))
        .map(|(_, v)| v)
}

// ── commit + read-back ─────────────────────────────────────────────────────

/// Commit the vendored FLAT `time_series` instance, then read it back as FLAT
/// (the data leaf round-trips), canonical JSON, and STRUCTURED — each `200`
/// with the correct response `Content-Type`.
fn run_flat_commit_read_back<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let flat_text = fixtures::read(TS_FLAT_KEY).map_err(|e| codec(&e))?;

        let resp = post_simplified(ctx, &ehr_id, FLAT, &template_id, flat_text).await?;
        assert::status(&resp, 201)?;
        let object = ids::object_uid(&ids::version_uid(ctx, &resp)?).to_owned();

        // Read-back as FLAT: 200 + Content-Type + the DV_QUANTITY data leaf
        // round-trips (the server assigns the uid; the clinical data must survive).
        let flat = get_composition(ctx, &ehr_id, &object, FLAT).await?;
        assert::status(&flat, 200)?;
        assert_content_type(&flat, FLAT)?;
        let flat_body = flat.json()?;
        let mag = flat_leaf(&flat_body, "|magnitude").and_then(Value::as_f64);
        if mag.is_none_or(|m| (m - 702.9).abs() > 1e-6) {
            return Err(CaseError::Assertion(format!(
                "FLAT read-back lost the DV_QUANTITY |magnitude leaf (got {mag:?}, expected 702.9)"
            )));
        }
        if flat_leaf(&flat_body, "|unit").and_then(Value::as_str) != Some("mm3") {
            return Err(CaseError::Assertion(
                "FLAT read-back lost the DV_QUANTITY |unit leaf (expected mm3)".to_owned(),
            ));
        }

        // Read-back as canonical JSON: 200 + Content-Type + _type COMPOSITION.
        let json = get_composition(ctx, &ehr_id, &object, JSON_MT).await?;
        assert::status(&json, 200)?;
        assert_content_type(&json, JSON_MT)?;
        if json.json()?["_type"] != "COMPOSITION" {
            return Err(CaseError::Assertion(
                "canonical read-back is not a COMPOSITION".to_owned(),
            ));
        }

        // Read-back as STRUCTURED: 200 + Content-Type.
        let structured = get_composition(ctx, &ehr_id, &object, STRUCTURED).await?;
        assert::status(&structured, 200)?;
        assert_content_type(&structured, STRUCTURED)?;
        if !structured.json()?.is_object() {
            return Err(CaseError::Assertion(
                "STRUCTURED read-back is not a JSON object".to_owned(),
            ));
        }
        Ok(DataSetReport::all(3))
    })
}

/// Commit the `time_series` composition in STRUCTURED form (derived from the
/// vendored FLAT instance through the `openehr-flat` converters), then read it
/// back as STRUCTURED, canonical JSON, and FLAT.
fn run_structured_commit_read_back<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let structured = ts_structured_value()?;
        let body =
            serde_json::to_string(&structured).map_err(|e| CaseError::Codec(e.to_string()))?;

        let resp = post_simplified(ctx, &ehr_id, STRUCTURED, &template_id, body).await?;
        assert::status(&resp, 201)?;
        let object = ids::object_uid(&ids::version_uid(ctx, &resp)?).to_owned();

        let structured_back = get_composition(ctx, &ehr_id, &object, STRUCTURED).await?;
        assert::status(&structured_back, 200)?;
        assert_content_type(&structured_back, STRUCTURED)?;
        if !structured_back.json()?.is_object() {
            return Err(CaseError::Assertion(
                "STRUCTURED read-back is not a JSON object".to_owned(),
            ));
        }

        let json = get_composition(ctx, &ehr_id, &object, JSON_MT).await?;
        assert::status(&json, 200)?;
        assert_content_type(&json, JSON_MT)?;
        if json.json()?["_type"] != "COMPOSITION" {
            return Err(CaseError::Assertion(
                "canonical read-back is not a COMPOSITION".to_owned(),
            ));
        }

        let flat = get_composition(ctx, &ehr_id, &object, FLAT).await?;
        assert::status(&flat, 200)?;
        assert_content_type(&flat, FLAT)?;
        if flat_leaf(&flat.json()?, "|magnitude")
            .and_then(Value::as_f64)
            .is_none()
        {
            return Err(CaseError::Assertion(
                "FLAT read-back of the STRUCTURED commit lost the DV_QUANTITY |magnitude leaf"
                    .to_owned(),
            ));
        }
        Ok(DataSetReport::all(3))
    })
}

// ── negotiation strictness ────────────────────────────────────────────────

/// An `Accept` with q-values selects the highest-weight acceptable format
/// (RFC 9110 §12.5.1): `application/xml;q=0.5, application/openehr.wt.flat+json`
/// (flat at the implicit q=1) serves FLAT, with a `Content-Type` present.
fn run_negotiation_qvalue<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let flat_text = fixtures::read(TS_FLAT_KEY).map_err(|e| codec(&e))?;
        let resp = post_simplified(ctx, &ehr_id, FLAT, &template_id, flat_text).await?;
        assert::status(&resp, 201)?;
        let object = ids::object_uid(&ids::version_uid(ctx, &resp)?).to_owned();

        // XML at q=0.5 vs FLAT at the default q=1 → FLAT wins.
        let got =
            get_composition(ctx, &ehr_id, &object, &format!("{XML_MT};q=0.5, {FLAT}")).await?;
        assert::status(&got, 200)?;
        assert_content_type(&got, FLAT)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// Every retired media type (deprecated `.schema+json` + legacy `nc.flat`/`tds2`)
/// is unacceptable on `Accept` → `406`, on both a composition GET and the
/// template example GET; each reject carries a `Content-Type` + error body.
fn run_reject_retired_accept<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let flat_text = fixtures::read(TS_FLAT_KEY).map_err(|e| codec(&e))?;
        let resp = post_simplified(ctx, &ehr_id, FLAT, &template_id, flat_text).await?;
        assert::status(&resp, 201)?;
        let object = ids::object_uid(&ids::version_uid(ctx, &resp)?).to_owned();
        let example_path = format!(
            "/definition/template/adl1.4/{}/example?type=input&detail_level=required",
            urlencoding::encode(&template_id)
        );

        let mut checks = 0u32;
        for &media in RETIRED_MEDIA_TYPES {
            // composition GET
            let g = get_composition(ctx, &ehr_id, &object, media).await?;
            assert::status(&g, 406).map_err(|e| label(media, "composition GET", e))?;
            assert_error_body(&g).map_err(|e| label(media, "composition GET", e))?;
            checks += 1;
            // template example GET
            let ex = ctx
                .send(HttpRequest::get(example_path.clone()).header("accept", media))
                .await?;
            assert::status(&ex, 406).map_err(|e| label(media, "example GET", e))?;
            assert_error_body(&ex).map_err(|e| label(media, "example GET", e))?;
            checks += 1;
        }
        Ok(DataSetReport::all(checks))
    })
}

/// Every retired media type is unsupported as a write `Content-Type` → `415`,
/// on both a composition POST and a CONTRIBUTION POST; each reject carries a
/// `Content-Type` + error body. The media-type reject fires before resource
/// resolution, so a fresh EHR id suffices.
fn run_reject_retired_content_type<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = support::create_ehr(ctx).await?;
        let mut checks = 0u32;
        for &media in RETIRED_MEDIA_TYPES {
            // composition POST — the unrecognized Content-Type is a 415 before
            // body/template resolution, so no openehr-template-id header is needed.
            let c = ctx
                .send(
                    HttpRequest::post(format!("/ehr/{ehr_id}/composition")).text_body("{}", media),
                )
                .await?;
            assert::status(&c, 415).map_err(|e| label(media, "composition POST", e))?;
            assert_error_body(&c).map_err(|e| label(media, "composition POST", e))?;
            checks += 1;
            // CONTRIBUTION POST
            let ctb = ctx
                .send(
                    HttpRequest::post(format!("/ehr/{ehr_id}/contribution")).text_body("{}", media),
                )
                .await?;
            assert::status(&ctb, 415).map_err(|e| label(media, "contribution POST", e))?;
            assert_error_body(&ctb).map_err(|e| label(media, "contribution POST", e))?;
            checks += 1;
        }
        Ok(DataSetReport::all(checks))
    })
}

/// Prefix a failure with the media type + endpoint under test.
fn label(media: &str, endpoint: &str, e: CaseError) -> CaseError {
    match e {
        CaseError::Assertion(m) => CaseError::Assertion(format!("{media} on {endpoint}: {m}")),
        other => other,
    }
}

// ── header rule ──────────────────────────────────────────────────────────────

/// A FLAT COMPOSITION commit with no `openehr-template-id` header (and a FLAT
/// payload, which never carries `archetype_details.template_id`) cannot resolve
/// a template → `422` (`Requests_and_responses` §openehr-template-id).
fn run_flat_missing_template_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let flat_text = fixtures::read(TS_FLAT_KEY).map_err(|e| codec(&e))?;
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition")).text_body(flat_text, FLAT),
            )
            .await?;
        assert::status(&resp, 422)?;
        assert_error_body(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── reject rules ─────────────────────────────────────────────────────────────

/// A FLAT commit carrying a field identifier that resolves to no Web Template
/// node → `422` (master04 §Validation).
fn run_flat_reject_unknown_field<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let mut flat = ts_flat_map()?;
        flat.insert(
            "event_series/this_field_is_not_in_the_template|value".to_owned(),
            Value::String("x".to_owned()),
        );
        let body = serde_json::to_string(&flat).map_err(|e| CaseError::Codec(e.to_string()))?;
        let resp = post_simplified(ctx, &ehr_id, FLAT, &template_id, body).await?;
        assert::status(&resp, 422)?;
        assert_error_body(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// A FLAT commit combining `|other` with `|code` on one coded leaf → `422`
/// (master04 §Open Value-Sets: the combination is a MUST-reject). The
/// `event_series/category` leaf is a `DV_CODED_TEXT` carrying `|code`/`|value`
/// in the vendored instance; adding `|other` makes the combination illegal.
fn run_flat_reject_other_with_code<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let mut flat = ts_flat_map()?;
        // The instance already carries event_series/category|code + |value;
        // adding |other on the same leaf is the MUST-reject combination.
        flat.insert(
            "event_series/category|other".to_owned(),
            Value::String("free text".to_owned()),
        );
        let body = serde_json::to_string(&flat).map_err(|e| CaseError::Codec(e.to_string()))?;
        let resp = post_simplified(ctx, &ehr_id, FLAT, &template_id, body).await?;
        assert::status(&resp, 422)?;
        assert_error_body(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── template surfaces ────────────────────────────────────────────────────────

/// GET a provisioned template with `Accept: application/openehr.wt+json` → `200`
/// + the Web Template document (`templateId` + `tree` present) + Content-Type.
fn run_template_web_template_get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/definition/template/adl1.4/{}",
                    urlencoding::encode(&template_id)
                ))
                .header("accept", WT),
            )
            .await?;
        assert::status(&resp, 200)?;
        assert_content_type(&resp, WT)?;
        let body = resp.json()?;
        if body.get("templateId").and_then(Value::as_str).is_none() {
            return Err(CaseError::Assertion(
                "Web Template document has no templateId".to_owned(),
            ));
        }
        if body.get("tree").is_none() {
            return Err(CaseError::Assertion(
                "Web Template document has no tree".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// GET a template example under each of the four `Accept_LOCATABLE` forms —
/// canonical JSON/XML + FLAT/STRUCTURED — each `200` with the matching
/// response `Content-Type`.
fn run_template_example_accept_forms<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let base = format!(
            "/definition/template/adl1.4/{}/example?type=input&detail_level=required",
            urlencoding::encode(&template_id)
        );
        let mut checks = 0u32;
        for accept in [JSON_MT, XML_MT, FLAT, STRUCTURED] {
            let resp = ctx
                .send(HttpRequest::get(base.clone()).header("accept", accept))
                .await?;
            assert::status(&resp, 200).map_err(|e| label(accept, "example GET", e))?;
            assert_content_type(&resp, accept).map_err(|e| label(accept, "example GET", e))?;
            checks += 1;
        }
        Ok(DataSetReport::all(checks))
    })
}

/// GET a template example with an `Accept` outside the four LOCATABLE forms
/// (the Web Template media type is not a LOCATABLE representation) → `406`.
fn run_template_example_unsupported_accept<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/definition/template/adl1.4/{}/example?type=input&detail_level=required",
                    urlencoding::encode(&template_id)
                ))
                .header("accept", WT),
            )
            .await?;
        assert::status(&resp, 406)?;
        assert_error_body(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── CONTRIBUTION ─────────────────────────────────────────────────────────────

/// Commit a CONTRIBUTION whose single `versions[0].data` is a FLAT COMPOSITION
/// (Content-Type flat + `openehr-template-id`): the envelope stays canonical,
/// the commit succeeds (`201`), and reading the CONTRIBUTION back with a FLAT
/// `Accept` (resolving the version refs) yields a canonical CONTRIBUTION
/// envelope whose inner `versions[0].data` is a simplified (FLAT) object, not a
/// canonical COMPOSITION.
fn run_contribution_flat_commit_read_back<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;

        // A vendored valid single-version creation envelope, with its canonical
        // inner COMPOSITION replaced by the vendored time_series FLAT object —
        // the envelope + audit are template-agnostic; the template comes from the
        // openehr-template-id header, and the server rebuilds each versions[i].data.
        let mut envelope = load_contribution_envelope()?;
        let flat = ts_flat_map()?;
        envelope["versions"][0]["data"] = Value::Object(flat);

        let commit = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/contribution"))
                    .text_body(
                        serde_json::to_string(&envelope)
                            .map_err(|e| CaseError::Codec(e.to_string()))?,
                        FLAT,
                    )
                    .header("openehr-template-id", template_id),
            )
            .await?;
        assert::status(&commit, 201)?;
        let ctb_uid = ids::contribution_uid(ctx, &commit)?;

        // Read back as FLAT with resolved refs: canonical envelope, simplified
        // inner data.
        let got = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/contribution/{ctb_uid}"))
                    .header("accept", FLAT)
                    .header("prefer", "resolve_refs"),
            )
            .await?;
        assert::status(&got, 200)?;
        assert_content_type(&got, FLAT)?;
        let body = got.json()?;
        if body["_type"] != "CONTRIBUTION" {
            return Err(CaseError::Assertion(format!(
                "expected a canonical CONTRIBUTION envelope, got _type {}",
                body["_type"]
            )));
        }
        let data = &body["versions"][0]["data"];
        if data.get("_type").and_then(Value::as_str) == Some("COMPOSITION") {
            return Err(CaseError::Assertion(
                "versions[0].data was returned canonical (COMPOSITION), not simplified".to_owned(),
            ));
        }
        let is_flat = data
            .as_object()
            .is_some_and(|m| m.keys().any(|k| k.contains('/') && k.contains('|')));
        if !is_flat {
            return Err(CaseError::Assertion(
                "versions[0].data is not a FLAT object (no flat-path keys)".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// A vendored valid single-version CONTRIBUTION creation envelope (audit +
/// `commit_audit` are template-agnostic; only its inner `data` is swapped).
fn load_contribution_envelope() -> Result<Value, CaseError> {
    let text = fixtures::read_from(
        "contribution.valid",
        "minimal/minimal_evaluation.contribution.json",
    )
    .map_err(|e| codec(&e))?;
    serde_json::from_str(&text).map_err(|e| CaseError::Codec(e.to_string()))
}

// ── non-templated resource rejects ────────────────────────────────────────────

/// A non-templated resource has no Simplified-Formats mapping: a simplified
/// `Accept` on a read → `406`, a simplified `Content-Type` on a write → `415`.
async fn assert_non_templated(
    ctx: &RunContext<'_>,
    get_path: &str,
    write: HttpRequest,
) -> Result<DataSetReport, CaseError> {
    // read with a simplified Accept → 406.
    let read = ctx
        .send(HttpRequest::get(get_path.to_owned()).header("accept", FLAT))
        .await?;
    assert::status(&read, 406)?;
    assert_error_body(&read)?;
    // write with a simplified Content-Type → 415.
    let w = ctx.send(write.text_body("{}", FLAT)).await?;
    assert::status(&w, 415)?;
    assert_error_body(&w)?;
    Ok(DataSetReport::all(2))
}

fn run_non_templated_ehr_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr = Uuid::new_v4();
        assert_non_templated(
            ctx,
            &format!("/ehr/{ehr}/ehr_status"),
            HttpRequest::put(format!("/ehr/{ehr}/ehr_status")),
        )
        .await
    })
}

fn run_non_templated_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr = Uuid::new_v4();
        assert_non_templated(
            ctx,
            &format!("/ehr/{ehr}/directory"),
            HttpRequest::post(format!("/ehr/{ehr}/directory")),
        )
        .await
    })
}

fn run_non_templated_demographic<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        assert_non_templated(
            ctx,
            &format!("/demographic/person/{}", Uuid::new_v4()),
            HttpRequest::post("/demographic/person"),
        )
        .await
    })
}

// ── ctx observability ──────────────────────────────────────────────────────

/// A FLAT commit with an explicit `ctx/time` and no `ctx/setting`: the canonical
/// read-back shows `COMPOSITION.context.start_time` = the supplied `ctx/time`
/// (master06 §time) and `context.setting` defaulted to `openehr::238|other care|`
/// (master06 §setting). Built by augmenting the vendored `time_series` FLAT
/// instance (the mandatory data + inline language/territory/composer are already
/// present) with `ctx/time`, so the case exercises the context defaulting rules
/// without coupling to the template's mandatory-leaf set.
fn run_ctx_observability<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        const CTX_TIME: &str = "2024-01-15T10:30:00Z";
        let template_id = provision_ts_opt(ctx).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let mut flat = ts_flat_map()?;
        flat.insert("ctx/time".to_owned(), Value::String(CTX_TIME.to_owned()));
        let body = serde_json::to_string(&flat).map_err(|e| CaseError::Codec(e.to_string()))?;

        let resp = post_simplified(ctx, &ehr_id, FLAT, &template_id, body).await?;
        assert::status(&resp, 201)?;
        let object = ids::object_uid(&ids::version_uid(ctx, &resp)?).to_owned();

        let got = get_composition(ctx, &ehr_id, &object, JSON_MT).await?;
        assert::status(&got, 200)?;
        let comp = got.json()?;

        // start_time echoes ctx/time (compared as an instant, tolerant of an
        // equivalent offset/precision rendering — master06 §time).
        let start = comp
            .pointer("/context/start_time/value")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CaseError::Assertion("canonical read has no context/start_time/value".to_owned())
            })?;
        let want: jiff::Timestamp = CTX_TIME
            .parse()
            .map_err(|e| CaseError::Codec(format!("parse ctx/time: {e}")))?;
        let got_ts: jiff::Timestamp = start.parse().map_err(|_| {
            CaseError::Assertion(format!(
                "context/start_time {start:?} does not parse as a timestamp (expected {CTX_TIME})"
            ))
        })?;
        if got_ts != want {
            return Err(CaseError::Assertion(format!(
                "context/start_time {start:?} is not the supplied ctx/time {CTX_TIME}"
            )));
        }

        // setting defaults to openehr::238|other care| (master06 §setting).
        let code = comp
            .pointer("/context/setting/defining_code/code_string")
            .and_then(Value::as_str);
        if code != Some("238") {
            return Err(CaseError::Assertion(format!(
                "context/setting defaulted to openehr::{code:?}, expected openehr::238 (master06 §setting)"
            )));
        }
        Ok(DataSetReport::all(2))
    })
}
