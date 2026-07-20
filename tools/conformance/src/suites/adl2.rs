//! DEFINITION / ADL 2 template provisioning — the `I_DEFINITION_ADL2` REST
//! surface (area `Adl2`; OPTIONS capability [`Capability::Adl2Provisioning`]).
//!
//! ADL 2 provisioning is **OPTIONAL** for openEHR conformance: the ITS-REST
//! DEFINITION ADL2 group is DEVELOPMENT-status (`docs/VERSIONS.md` ITS-REST row
//! — "per API: … Demographic/Admin/SMART DEVELOPMENT"; the ADL2 operations
//! `definition_template_adl2_*` sit in that development surface), and the CNF
//! Platform Conformance Test Schedule defines **no** ADL 2 test case — its
//! master04 `I_DEFINITION_ADL2` half carries no test cases upstream (recorded in
//! [`crate::suites::definition_adl14`]). So every case here is
//! [`ScheduleTrace::EccOriginal`], derived from the vendored ITS-REST ADL2
//! operation YAMLs (the per-case oracle), and the area lands in the OPTIONS
//! tier — it never gates CORE/STANDARD. A green run makes OPTIONS-OBTAINED
//! genuinely cover `Adl2Provisioning`.
//!
//! Oracle (per case): the operation YAMLs under
//! `docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml`
//! and their response schemas.
//!
//! ## Data-set discipline
//!
//! Every ADL 2 source is **generated in-harness** (register-80 preference order
//! `generated: > owned: > corpus:`): a minimal, spec-valid ADL 2
//! `operational_template` / `archetype` synthesized by [`opt_source`] /
//! [`archetype_parent`] / [`specialised_child`] — the same minimal shape the
//! application's own ADL2 wire test drives (a COMPOSITION-rooted OPT: header +
//! HRID, `language`, `description` (mandatory — AOM2 master03 §Validity Rules
//! VARD), `definition` (root `id1`), `terminology`). Every case mints a
//! **fresh, unique HRID** ([`unique_hrid`]), so a shared SUT never books a
//! spurious `409` and the cases are order-independent. The validating engine
//! (`openehr-adl` + the generated `openehr-rm` model) is self-contained — no RM
//! or terminology provisioning is required.

use serde_json::Value;
use uuid::Uuid;

use crate::engine::assert;
use crate::engine::harness::{
    CaseError, CaseFuture, DataSetReport, HttpRequest, HttpResponse, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;

/// JSON is the runner's negotiation axis; the ADL 2 source payload itself is
/// `text/plain`, driven explicitly per request. The schedule tabulates no
/// format-sensitive ADL 2 surface.
const JSON: &[Format] = &[Format::Json];

/// The ADL 2 template resource base.
const ADL2: &str = "/definition/template/adl2";

/// The single ECC-original reason shared by every ADL 2 case: no CNF schedule
/// chapter defines ADL 2 provisioning, so the whole area is derived from the
/// DEVELOPMENT-status ITS-REST ADL2 operation YAMLs, OPTIONS-tier.
const ECC_REASON: &str = "the CNF Platform Conformance Test Schedule defines no ADL 2 test case \
     (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status \
     ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/\
     definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md \
     ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.";

/// The three `Accept_LOCATABLE` non-canonical example media types (plus canonical
/// JSON/XML) the example endpoint negotiates.
const EXAMPLE_ACCEPTS: [&str; 4] = [
    "application/json",
    "application/xml",
    "application/openehr.wt.flat+json",
    "application/openehr.wt.structured+json",
];

/// Every registered ADL 2 provisioning case.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the registered ECC case table is inherently enumerative"
)]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── upload ────────────────────────────────────────────────────────────
        case(
            "adl2/upload-201-prefer-triad",
            "Upload a valid ADL2 template → 201 with Location; Prefer selects minimal/representation/identifier bodies",
            "ITS-REST definition_template_adl2_upload.yaml (text/plain source) + \
             201_Template_adl2_upload.yaml (Location + Prefer: minimal empty / representation source / \
             identifier TemplateIdentifier JSON)",
            Binding::Rest(
                "POST /definition/template/adl2 (Prefer return=minimal|representation|identifier)",
            ),
            run_upload_prefer_triad,
        ),
        case(
            "adl2/upload-duplicate-conflict",
            "Upload the same ADL2 HRID twice → the second is a 409 conflict",
            "ITS-REST definition_template_adl2_upload.yaml §409 (409_template_already_exists.yaml — \
             a template with the same template_id already exists)",
            Binding::Rest("POST /definition/template/adl2 (same HRID twice)"),
            run_upload_duplicate,
        ),
        case(
            "adl2/upload-unparseable-422",
            "Upload an unparseable ADL2 source → 422 carrying syntax rule codes in validationErrors",
            "ITS-REST definition_template_adl2_upload.yaml (an invalid source is rejected; our wire \
             renders the Error object with validationErrors — the OAS folds this under 400, the served \
             surface documents 422)",
            Binding::Rest("POST /definition/template/adl2 (unparseable source)"),
            run_upload_unparseable,
        ),
        case(
            "adl2/upload-invalid-422-vcode",
            "Upload a semantically invalid ADL2 template (missing description) → 422 with the AOM2 rule code VARD",
            "ITS-REST definition_template_adl2_upload.yaml; AOM2 master03-archetype_definitions §Validity \
             Rules VARD (a description section is mandatory) — reported as a rule code in validationErrors",
            Binding::Rest("POST /definition/template/adl2 (AOM2-invalid source)"),
            run_upload_invalid_vcode,
        ),
        case(
            "adl2/upload-specialised-child-resolves-parent",
            "Upload a parent archetype, then a specialised child that validates against the stored parent → 201",
            "ITS-REST definition_template_adl2_upload.yaml; AOM2 master05-specialisation §Specialisation \
             (a specialised archetype is validated against its flat parent, resolved from the repository)",
            Binding::Rest(
                "POST /definition/template/adl2 (parent) → POST /definition/template/adl2 (specialised child)",
            ),
            run_upload_specialised_child,
        ),
        // ── get ───────────────────────────────────────────────────────────────
        case(
            "adl2/get-representations",
            "Get an ADL2 template as text/plain source, application/json OperationalTemplateV2, and 406 on xml-only",
            "ITS-REST definition_template_adl2_get.yaml + 200_Template_adl2_retrieved.yaml \
             (text/plain source | application/json OperationalTemplateV2) + Accept_Template_adl2.yaml \
             (application/xml has no declared response body → 406)",
            Binding::Rest(
                "GET /definition/template/adl2/{template_id} (Accept text/plain | application/json | application/xml)",
            ),
            run_get_representations,
        ),
        case(
            "adl2/get-unknown-404",
            "Get an unknown ADL2 template_id → 404",
            "ITS-REST definition_template_adl2_get.yaml §404 (404_unknown_template_id.yaml)",
            Binding::Rest("GET /definition/template/adl2/{template_id}"),
            run_get_unknown,
        ),
        // ── version get (partial / SEMVER-prefix resolution) ────────────────────
        case(
            "adl2/version-get-exact-prefix-unknown",
            "Version get resolves an exact SEMVER and a major prefix (latest match) → 200; an unknown version → 404",
            "ITS-REST definition_template_adl2_version_get.yaml (deprecated but served) + \
             200_Template_adl2_retrieved.yaml + template_id_adl2.yaml (a partial template_id resolves to \
             the latest matching major version)",
            Binding::Rest("GET /definition/template/adl2/{template_id}/{version}"),
            run_version_get,
        ),
        // ── example ─────────────────────────────────────────────────────────────
        case(
            "adl2/example-four-accept-forms",
            "Get a template example in each of the four Accept_LOCATABLE forms → 200; the JSON form is a COMPOSITION rooted at the template's archetype",
            "ITS-REST definition_template_adl2_example_get.yaml + 200_Template_example_retrieved.yaml \
             (LOCATABLE oneOf) + Accept_LOCATABLE.yaml (json / xml / wt.flat+json / wt.structured+json)",
            Binding::Rest(
                "GET /definition/template/adl2/{template_id}/example (Accept json/xml/flat/structured)",
            ),
            run_example_accept_forms,
        ),
        case(
            "adl2/example-detail-levels-and-bad-enum",
            "Example honours the detail_level enum (required/medium/complete) and rejects a bad type/detail_level with 400",
            "ITS-REST definition_template_adl2_example_get.yaml + example_type.yaml (input|output) + \
             example_detail_level.yaml (required|medium|complete) + 400.yaml (out-of-enum → 400)",
            Binding::Rest(
                "GET /definition/template/adl2/{template_id}/example?type=&detail_level=",
            ),
            run_example_enums,
        ),
        case(
            "adl2/example-unknown-404-wrong-accept-406",
            "Example for an unknown template_id → 404; an Accept outside the four LOCATABLE forms → 406",
            "ITS-REST definition_template_adl2_example_get.yaml §404 (404_unknown_template_id.yaml) + §406 \
             (406.yaml) + Accept_LOCATABLE.yaml",
            Binding::Rest("GET /definition/template/adl2/{template_id}/example"),
            run_example_unknown_and_wrong_accept,
        ),
        // ── list ──────────────────────────────────────────────────────────────
        case(
            "adl2/list-template-metadata",
            "List ADL2 templates → TemplateMetadata carrying template_id, concept, archetype_id, created_timestamp",
            "ITS-REST definition_template_adl2_list.yaml + 200_TemplateList_adl2.yaml + \
             schemas/definition/TemplateMetadata.yaml (template_id / concept / archetype_id / created_timestamp)",
            Binding::Rest("GET /definition/template/adl2"),
            run_list_metadata,
        ),
    ]
}

/// Assemble an ADL 2 case entry: area `Adl2`, capability
/// [`Capability::Adl2Provisioning`], JSON axis, ECC-original (OPTIONS).
fn case(
    id: &'static str,
    title: &'static str,
    citation: &'static str,
    binding: Binding,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Adl2,
            capability: Capability::Adl2Provisioning,
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

// ── source generators (register-80 `generated:` discipline) ──────────────────

/// A fresh, unique COMPOSITION HRID: `openEHR-EHR-COMPOSITION.ecc_<hex>.v1.0.0`.
/// The `ecc_` prefix keeps the concept a valid identifier (a bare hex digit
/// could not lead one), and the UUID guarantees no `409` on a shared SUT.
fn unique_hrid() -> String {
    format!(
        "openEHR-EHR-COMPOSITION.ecc_{}.v1.0.0",
        Uuid::new_v4().simple()
    )
}

/// The concept family of an HRID (`…v1.0.0` version suffix stripped) — the
/// `template_id` a version-addressed / partial get resolves within.
fn family_of(hrid: &str) -> &str {
    hrid.strip_suffix(".v1.0.0").unwrap_or(hrid)
}

/// A minimal, spec-valid ADL 2 `operational_template` source rooted at
/// COMPOSITION (the shape the application's own ADL2 wire test drives): header +
/// HRID, `language`, `description` (mandatory — AOM2 VARD), an open `definition`
/// (root `id1`), and a `terminology` block for `id1`.
fn opt_source(hrid: &str) -> String {
    format!(
        "operational_template (adl_version=2.0.6; rm_release=1.1.0)\n    {hrid}\n\n\
         language\n    original_language = <[ISO_639-1::en]>\n\n\
         description\n    lifecycle_state = <\"published\">\n    details = <\n        [\"en\"] = <\n            language = <[ISO_639-1::en]>\n        >\n    >\n\n\
         definition\n    COMPOSITION[id1] matches {{ *}}\n\n\
         terminology\n    term_definitions = <\n        [\"en\"] = <\n            [\"id1\"] = <text = <\"Root\"> description = <\"Root.\">>\n        >\n    >\n"
    )
}

/// The same minimal COMPOSITION source but **missing the `description` section**
/// — an AOM2 §Validity Rules VARD violation the engine rejects with a V-code.
fn opt_source_no_description(hrid: &str) -> String {
    format!(
        "operational_template (adl_version=2.0.6; rm_release=1.1.0)\n    {hrid}\n\n\
         language\n    original_language = <[ISO_639-1::en]>\n\n\
         definition\n    COMPOSITION[id1] matches {{ *}}\n\n\
         terminology\n    term_definitions = <\n        [\"en\"] = <\n            [\"id1\"] = <text = <\"Root\"> description = <\"Root.\">>\n        >\n    >\n"
    )
}

/// A minimal, spec-valid ADL 2 **archetype** (specialisable parent) rooted at
/// COMPOSITION.
fn archetype_parent(hrid: &str) -> String {
    format!(
        "archetype (adl_version=2.0.6; rm_release=1.1.0)\n    {hrid}\n\n\
         language\n    original_language = <[ISO_639-1::en]>\n\n\
         description\n    lifecycle_state = <\"published\">\n    details = <\n        [\"en\"] = <\n            language = <[ISO_639-1::en]>\n        >\n    >\n\n\
         definition\n    COMPOSITION[id1] matches {{ *}}\n\n\
         terminology\n    term_definitions = <\n        [\"en\"] = <\n            [\"id1\"] = <text = <\"Root\"> description = <\"Root.\">>\n        >\n    >\n"
    )
}

/// A minimal no-change specialised child of `parent_family` (the proven no-change
/// specialisation shape: `specialize` clause + a specialised root node `id1.1`),
/// mirroring the vendored openEHR ADL2 reference `…-no_change` specialisation.
fn specialised_child(child_hrid: &str, parent_family: &str) -> String {
    format!(
        "archetype (adl_version=2.0.6; rm_release=1.1.0)\n    {child_hrid}\n\n\
         specialize\n    {parent_family}.v1\n\n\
         language\n    original_language = <[ISO_639-1::en]>\n\n\
         description\n    lifecycle_state = <\"published\">\n    details = <\n        [\"en\"] = <\n            language = <[ISO_639-1::en]>\n        >\n    >\n\n\
         definition\n    COMPOSITION[id1.1] matches {{ *}}\n\n\
         terminology\n    term_definitions = <\n        [\"en\"] = <\n            [\"id1.1\"] = <text = <\"Root\"> description = <\"Root.\">>\n        >\n    >\n"
    )
}

// ── request helpers ──────────────────────────────────────────────────────────

/// POST an ADL 2 `text/plain` source with an optional `Prefer`.
async fn upload(
    ctx: &RunContext<'_>,
    source: String,
    prefer: Option<&str>,
) -> Result<HttpResponse, CaseError> {
    let mut req = HttpRequest::post(ADL2).text_body(source, "text/plain");
    if let Some(p) = prefer {
        req = req.header("prefer", p.to_owned());
    }
    ctx.send(req).await
}

/// Provision a fresh valid OPT and return its HRID (asserts a fresh `201`).
async fn provision_opt(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let hrid = unique_hrid();
    let resp = upload(ctx, opt_source(&hrid), None).await?;
    assert::status(&resp, 201)?;
    Ok(hrid)
}

/// GET an ADL 2 resource with an optional `Accept`.
async fn get(
    ctx: &RunContext<'_>,
    path: String,
    accept: Option<&str>,
) -> Result<HttpResponse, CaseError> {
    let mut req = HttpRequest::get(path);
    if let Some(a) = accept {
        req = req.header("accept", a.to_owned());
    }
    ctx.send(req).await
}

/// Assert a `Content-Type` header starts with `expected` (tolerating a `; charset`).
fn assert_content_type(resp: &HttpResponse, expected: &str) -> Result<(), CaseError> {
    match resp.header("content-type") {
        Some(v)
            if v.split(';')
                .next()
                .unwrap_or(v)
                .trim()
                .starts_with(expected) =>
        {
            Ok(())
        }
        other => Err(CaseError::Assertion(format!(
            "expected Content-Type starting {expected:?}, got {other:?}"
        ))),
    }
}

/// The `validationErrors` array of a `422` ITS-REST `Error` body
/// (`{ message, validationErrors: [ "<CODE>: <message>" ] }`).
fn validation_errors(resp: &HttpResponse) -> Result<Vec<String>, CaseError> {
    let body = resp.json()?;
    let errors = body
        .get("validationErrors")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CaseError::Assertion(format!(
                "422 body carries no validationErrors array: {body}"
            ))
        })?;
    Ok(errors
        .iter()
        .filter_map(|e| e.as_str().map(str::to_owned))
        .collect())
}

// ── upload cases ─────────────────────────────────────────────────────────────

/// Upload with each `Prefer` return form on three fresh HRIDs: `minimal`
/// (empty body), `representation` (the source echoed, `text/plain`), and
/// `identifier` (a `{template_id}` JSON object) — each `201` carrying `Location`.
fn run_upload_prefer_triad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // return=minimal (or absent) → empty body.
        let hrid_min = unique_hrid();
        let min = upload(ctx, opt_source(&hrid_min), Some("return=minimal")).await?;
        assert::status(&min, 201)?;
        assert::header_present(&min, "location")?;
        if !min.body.is_empty() {
            return Err(CaseError::Assertion(format!(
                "return=minimal upload must have an empty body, got {} bytes",
                min.body.len()
            )));
        }

        // return=representation → the stored source echoed, text/plain.
        let hrid_repr = unique_hrid();
        let source = opt_source(&hrid_repr);
        let repr = upload(ctx, source.clone(), Some("return=representation")).await?;
        assert::status(&repr, 201)?;
        assert::header_present(&repr, "location")?;
        assert_content_type(&repr, "text/plain")?;
        if repr.text() != source {
            return Err(CaseError::Assertion(
                "return=representation body did not echo the uploaded ADL2 source verbatim"
                    .to_owned(),
            ));
        }

        // return=identifier → a { template_id } JSON object.
        let hrid_id = unique_hrid();
        let ident = upload(ctx, opt_source(&hrid_id), Some("return=identifier")).await?;
        assert::status(&ident, 201)?;
        assert::header_present(&ident, "location")?;
        if ident.json()?.get("template_id").and_then(Value::as_str) != Some(hrid_id.as_str()) {
            return Err(CaseError::Assertion(
                "return=identifier body is not { template_id: <hrid> }".to_owned(),
            ));
        }
        Ok(DataSetReport::all(3))
    })
}

/// The same HRID uploaded twice: the first is `201`, the second a `409`
/// conflict.
fn run_upload_duplicate<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let hrid = unique_hrid();
        let first = upload(ctx, opt_source(&hrid), None).await?;
        assert::status(&first, 201)?;
        let second = upload(ctx, opt_source(&hrid), None).await?;
        assert::status(&second, 409)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// An unparseable ADL 2 source is a `422` carrying syntax rule codes.
fn run_upload_unparseable<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let resp = upload(
            ctx,
            "this is not a valid ADL2 artefact — no header, no sections".to_owned(),
            None,
        )
        .await?;
        assert::status(&resp, 422)?;
        let errors = validation_errors(&resp)?;
        if errors.is_empty() {
            return Err(CaseError::Assertion(
                "unparseable source: 422 body carries an empty validationErrors array".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// A semantically invalid template (missing the mandatory `description`) is a
/// `422` whose `validationErrors` name the AOM2 rule code `VARD`.
fn run_upload_invalid_vcode<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let hrid = unique_hrid();
        let resp = upload(ctx, opt_source_no_description(&hrid), None).await?;
        assert::status(&resp, 422)?;
        let errors = validation_errors(&resp)?;
        if !errors.iter().any(|e| e.contains("VARD")) {
            return Err(CaseError::Assertion(format!(
                "missing-description upload: expected the AOM2 rule code VARD in validationErrors, got {errors:?}"
            )));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// A parent archetype is stored, then a specialised child that references it
/// (`specialize <parent>.v1`) validates against the stored flat parent → `201`
/// (the repository-resolution path). The parent and child carry fresh, unique,
/// related HRIDs.
fn run_upload_specialised_child<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let parent_hrid = unique_hrid();
        let parent_family = family_of(&parent_hrid).to_owned();
        let parent = upload(ctx, archetype_parent(&parent_hrid), None).await?;
        assert::status(&parent, 201)?;

        // Child concept = <parent-concept>-child; a level-1 specialisation whose
        // root node is `id1.1` — resolves the parent from the repository.
        let child_hrid = format!("{parent_family}-child.v1.0.0");
        let child = upload(ctx, specialised_child(&child_hrid, &parent_family), None).await?;
        assert::status(&child, 201)?;
        Ok(DataSetReport::all(2))
    })
}

// ── get cases ────────────────────────────────────────────────────────────────

/// A stored template served as `text/plain` source (verbatim), as
/// `application/json` `OperationalTemplateV2` (`_type: OPERATIONAL_TEMPLATE`),
/// and a `406` when only `application/xml` is acceptable.
fn run_get_representations<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let hrid = unique_hrid();
        let source = opt_source(&hrid);
        assert::status(&upload(ctx, source.clone(), None).await?, 201)?;
        let path = format!("{ADL2}/{hrid}");

        // text/plain source, verbatim.
        let text = get(ctx, path.clone(), Some("text/plain")).await?;
        assert::status(&text, 200)?;
        assert_content_type(&text, "text/plain")?;
        if text.text() != source {
            return Err(CaseError::Assertion(
                "text/plain get did not echo the stored ADL2 source verbatim".to_owned(),
            ));
        }

        // application/json OperationalTemplateV2.
        let json = get(ctx, path.clone(), Some("application/json")).await?;
        assert::status(&json, 200)?;
        assert_content_type(&json, "application/json")?;
        if json.json()?.get("_type").and_then(Value::as_str) != Some("OPERATIONAL_TEMPLATE") {
            return Err(CaseError::Assertion(
                "application/json get is not an OperationalTemplateV2 (_type OPERATIONAL_TEMPLATE)"
                    .to_owned(),
            ));
        }

        // application/xml only → 406 (the response declares no XML body).
        let xml = get(ctx, path, Some("application/xml")).await?;
        assert::status(&xml, 406)?;
        Ok(DataSetReport::all(3))
    })
}

/// An unknown `template_id` → `404`.
fn run_get_unknown<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let unknown = format!(
            "openEHR-EHR-COMPOSITION.ecc_absent_{}.v1.0.0",
            Uuid::new_v4().simple()
        );
        let resp = get(ctx, format!("{ADL2}/{unknown}"), None).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── version get ──────────────────────────────────────────────────────────────

/// Version get resolves an exact SEMVER (`1.0.0`) and a `{major}` prefix (`1`,
/// the latest matching version — the partial-`template_id`-resolves-latest path)
/// → `200`; an absent version (`9`) → `404`.
fn run_version_get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let hrid = provision_opt(ctx).await?;
        let family = family_of(&hrid).to_owned();

        // Exact SEMVER.
        let exact = get(ctx, format!("{ADL2}/{family}/1.0.0"), None).await?;
        assert::status(&exact, 200)?;

        // Major prefix → latest matching version.
        let prefix = get(ctx, format!("{ADL2}/{family}/1"), None).await?;
        assert::status(&prefix, 200)?;

        // Absent version → 404.
        let absent = get(ctx, format!("{ADL2}/{family}/9"), None).await?;
        assert::status(&absent, 404)?;
        Ok(DataSetReport::all(3))
    })
}

// ── example cases ────────────────────────────────────────────────────────────

/// A generated example in each of the four `Accept_LOCATABLE` forms → `200`; the
/// canonical-JSON form is a COMPOSITION rooted at the template's archetype
/// (`archetype_details.template_id.value` = the HRID).
fn run_example_accept_forms<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let hrid = provision_opt(ctx).await?;
        let base = format!("{ADL2}/{hrid}/example");

        let mut checks = 0u32;
        for accept in EXAMPLE_ACCEPTS {
            let resp = get(ctx, base.clone(), Some(accept)).await?;
            assert::status(&resp, 200).map_err(|e| label(accept, e))?;
            assert::header_present(&resp, "content-type").map_err(|e| label(accept, e))?;
            checks += 1;
        }

        // The JSON form parses as a COMPOSITION rooted at the template's
        // archetype: `_type` COMPOSITION and a mandatory `archetype_details`
        // (RM `ARCHETYPED`) carrying the root `archetype_id`.
        let json = get(ctx, base, Some("application/json")).await?;
        let comp = json.json()?;
        if comp.get("_type").and_then(Value::as_str) != Some("COMPOSITION") {
            return Err(CaseError::Assertion(format!(
                "example JSON is not a COMPOSITION (_type={:?})",
                comp.get("_type")
            )));
        }
        if comp
            .pointer("/archetype_details/archetype_id/value")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(CaseError::Assertion(
                "example COMPOSITION carries no archetype at root (archetype_details/archetype_id/value)"
                    .to_owned(),
            ));
        }
        Ok(DataSetReport::all(checks))
    })
}

/// Each `detail_level` in the enum serves `200`; an out-of-enum `detail_level`
/// and an out-of-enum `type` each → `400`.
fn run_example_enums<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let hrid = provision_opt(ctx).await?;
        let base = format!("{ADL2}/{hrid}/example");

        let mut checks = 0u32;
        for level in ["required", "medium", "complete"] {
            let resp = get(ctx, format!("{base}?detail_level={level}"), None).await?;
            assert::status(&resp, 200).map_err(|e| label(level, e))?;
            checks += 1;
        }

        // Out-of-enum detail_level → 400.
        let bad_level = get(ctx, format!("{base}?detail_level=full"), None).await?;
        assert::status(&bad_level, 400)?;
        checks += 1;

        // Out-of-enum type → 400.
        let bad_type = get(ctx, format!("{base}?type=bogus"), None).await?;
        assert::status(&bad_type, 400)?;
        checks += 1;

        Ok(DataSetReport::all(checks))
    })
}

/// An example for an unknown `template_id` → `404`; an `Accept` outside the four
/// LOCATABLE forms → `406`.
fn run_example_unknown_and_wrong_accept<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Unknown template → 404.
        let unknown = format!(
            "openEHR-EHR-COMPOSITION.ecc_absent_{}.v1.0.0",
            Uuid::new_v4().simple()
        );
        let missing = get(ctx, format!("{ADL2}/{unknown}/example"), None).await?;
        assert::status(&missing, 404)?;

        // Wrong Accept on an existing template → 406.
        let hrid = provision_opt(ctx).await?;
        let wrong = get(ctx, format!("{ADL2}/{hrid}/example"), Some("text/csv")).await?;
        assert::status(&wrong, 406)?;
        Ok(DataSetReport::all(2))
    })
}

// ── list ─────────────────────────────────────────────────────────────────────

/// The list carries a `TemplateMetadata` row for the uploaded template with
/// `template_id`, `concept`, `archetype_id`, and `created_timestamp`.
fn run_list_metadata<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let hrid = provision_opt(ctx).await?;
        let resp = get(ctx, ADL2.to_owned(), Some("application/json")).await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        let list = body.as_array().ok_or_else(|| {
            CaseError::Assertion("ADL2 template list body is not a JSON array".to_owned())
        })?;
        let row = list
            .iter()
            .find(|r| r.get("template_id").and_then(Value::as_str) == Some(hrid.as_str()))
            .ok_or_else(|| {
                CaseError::Assertion(format!(
                    "template list does not contain the provisioned template_id {hrid:?}"
                ))
            })?;
        // TemplateMetadata: template_id / concept / archetype_id / created_timestamp.
        for field in ["concept", "archetype_id", "created_timestamp"] {
            if row.get(field).is_none() {
                return Err(CaseError::Assertion(format!(
                    "TemplateMetadata row is missing {field:?}: {row}"
                )));
            }
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// Prefix a failure with the sub-variant (media type / detail level) under test.
fn label(variant: &str, e: CaseError) -> CaseError {
    match e {
        CaseError::Assertion(m) => CaseError::Assertion(format!("{variant}: {m}")),
        other => other,
    }
}
