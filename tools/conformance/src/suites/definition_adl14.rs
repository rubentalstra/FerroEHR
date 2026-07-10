//! DEFINITION: ADL 1.4 / OPT 1.4 provisioning (area TPL).
//!
//! Our own ECC cases (reference: `master04-func_tc_definition_adl.adoc`,
//! design-time reading), driving the ITS-REST `/definition/template/adl1.4`
//! surface with the vendored `valid_templates` and `invalid_templates` `.opt`
//! fixtures (valid **and** invalid — both load-bearing). Assertions concretize
//! the OPT provisioning contract (`2xx` accept a valid OPT; `4xx` reject an
//! invalid one; `200` list).
//!
//! `get_opt-*` round-trips a provisioned `template_id`
//! (`GET /definition/template/adl1.4/{id}`); `validate_opt-*` is realized via
//! the upload endpoint (which validates — 2xx valid / 4xx invalid);
//! `upload_opt-*_twice` asserts the conflict/idempotency semantics.
//!
//! **`delete_opt-*` — D2 skip-with-reason.** The SM
//! `I_DEFINITION_ADL14.delete_opt()` (CNF master04:319) has no ITS-REST ADL 1.4
//! binding: neither Release-1.0.3 nor the tested development@e8a093e OAS defines
//! a `DELETE` verb on `/definition/template/adl1.4/{id}` — ITS-REST puts template
//! deletion in the **ADMIN** API only. So a 405 is a schedule-vs-ITS-REST gap,
//! not a server defect; every `delete_opt` case reports `SKIPPED` rather than a
//! fabricated failure (`docs/blueprint/07-cnf.md` D2).
//!
//! **CORE `Adl14ArchetypeProvisioning` evidencing (D5).** openEHR ITS-REST
//! exposes no standalone ADL 1.4 *archetype* resource — archetypes are delivered
//! to the platform **inside** OPTs (the operational template is the flattened
//! archetype set). We therefore evidence the CORE "ADL 1.4 Archetype
//! provisioning" capability by the OPT upload that provisions archetype-bearing
//! content: `tpl/upload-opt-valid-opt` is tagged
//! [`Capability::Adl14ArchetypeProvisioning`] (the remaining OPT cases stay
//! [`Capability::Adl14OptProvisioning`]). This matches the schedule's
//! EHRbase-derived reality (`EHRbase` provisions OPTs, not raw archetypes) and
//! makes both CORE capabilities claimable from real, passing cases
//! (`docs/blueprint/07-cnf.md` D5; `model::profile`).

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

/// The implemented master04 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // D5: this OPT upload is the CORE `Adl14ArchetypeProvisioning` evidence —
        // ADL 1.4 archetypes are provisioned to the platform inside the OPT (no
        // standalone archetype resource in ITS-REST). See module docs.
        entry(
            "tpl/upload-opt-valid-opt",
            "Upload OPT — valid OPT (provisions ADL 1.4 archetypes)",
            "ITS-REST DEFINITION ADL 1.4 API §upload OPT; AM 1.4 §OPERATIONAL_TEMPLATE; \
             CORE Adl14ArchetypeProvisioning evidenced via OPT (archetypes embedded in the OPT)",
            Capability::Adl14ArchetypeProvisioning,
            run_upload_valid,
        ),
        entry(
            "tpl/upload-opt-invalid-opt",
            "Upload OPT — invalid OPT",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            Capability::Adl14OptProvisioning,
            run_upload_invalid,
        ),
        entry(
            "tpl/get-opts-retrieve-all-no-opts",
            "List OPTs — retrieve all no OPTs",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            Capability::Adl14OptProvisioning,
            run_list,
        ),
        // upload_opt idempotency.
        c(
            "tpl/upload-opt-valid-opt-twice-conflict",
            "Upload OPT — valid OPT twice conflict",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_upload_twice_conflict,
        ),
        c(
            "tpl/upload-opt-valid-opt-twice-no-conflict",
            "Upload OPT — valid OPT twice no conflict",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_upload_twice_no_conflict,
        ),
        // get_opt — GET /definition/template/adl1.4/{template_id}[/{version}].
        c(
            "tpl/get-opt-retrieve-single",
            "Get OPT — retrieve single",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_get_single,
        ),
        c(
            "tpl/get-opt-retrieve-latest-version",
            "Get OPT — retrieve latest version",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_get_latest,
        ),
        c(
            "tpl/get-opt-retrieve-specific-version",
            "Get OPT — retrieve specific version",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_get_specific,
        ),
        c(
            "tpl/get-opt-retrieve-fail",
            "Get OPT — retrieve fail",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_get_fail,
        ),
        c(
            "tpl/get-opts-retrieve-all",
            "List OPTs — retrieve all",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_get_all,
        ),
        // validate_opt — realized via the upload endpoint (which validates).
        c(
            "tpl/validate-opt-valid-opt",
            "Validate OPT — valid OPT",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_validate_valid,
        ),
        c(
            "tpl/validate-opt-invalid-opt",
            "Validate OPT — invalid OPT",
            "ITS-REST 1.0.3 DEFINITION ADL 1.4 API §upload/get/validate/delete OPT; AM 1.4 §OPERATIONAL_TEMPLATE",
            run_validate_invalid,
        ),
        // delete_opt — D2: no ITS-REST ADL 1.4 DELETE verb (deletion is in the
        // ADMIN API only) → skip-with-reason. See module docs.
        c(
            "tpl/delete-opt-delete-non-existing",
            "Delete OPT — delete non existing",
            DELETE_OPT_CITATION,
            run_delete_absent,
        )
        .with_schedule_ref(DELETE_OPT_SCHEDULE_REF),
        c(
            "tpl/delete-opt-delete-existing",
            "Delete OPT — delete existing",
            DELETE_OPT_CITATION,
            run_delete_existing,
        )
        .with_schedule_ref(DELETE_OPT_SCHEDULE_REF),
        c(
            "tpl/delete-opt-delete-latest-version",
            "Delete OPT — delete latest version",
            DELETE_OPT_CITATION,
            run_delete_latest,
        )
        .with_schedule_ref(DELETE_OPT_SCHEDULE_REF),
        c(
            "tpl/delete-opt-delete-specific-version",
            "Delete OPT — delete specific version",
            DELETE_OPT_CITATION,
            run_delete_specific,
        )
        .with_schedule_ref(DELETE_OPT_SCHEDULE_REF),
    ]
}

/// Shorthand for an OPT-provisioning case entry.
fn c(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    entry(id, title, citation, Capability::Adl14OptProvisioning, run)
}

/// The `template_id` of the vendored `minimal_evaluation` OPT.
const TID: &str = "minimal_evaluation.en.v1";

/// Ensure the `minimal_evaluation` OPT is provisioned (2xx new or 409 present),
/// returning its `template_id`.
async fn ensure_present(ctx: &RunContext<'_>) -> Result<&'static str, CaseError> {
    let xml = fixtures::read("valid_templates/minimal/minimal_evaluation.opt")
        .map_err(|e| CaseError::Codec(e.to_string()))?;
    let status = upload_opt(ctx, xml).await?;
    if (200..300).contains(&status) || status == 409 {
        Ok(TID)
    } else {
        Err(CaseError::Assertion(format!(
            "provisioning minimal_evaluation.opt returned {status}"
        )))
    }
}

fn run_upload_twice_conflict<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        ensure_present(ctx).await?;
        // A second upload of the same template_id conflicts.
        let xml = fixtures::read("valid_templates/minimal/minimal_evaluation.opt")
            .map_err(|e| CaseError::Codec(e.to_string()))?;
        let status = upload_opt(ctx, xml).await?;
        if status == 409 {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "re-upload of an existing template_id expected 409, got {status}"
            )))
        }
    })
}

fn run_upload_twice_no_conflict<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        ensure_present(ctx).await?;
        let xml = fixtures::read("valid_templates/minimal/minimal_evaluation.opt")
            .map_err(|e| CaseError::Codec(e.to_string()))?;
        // Idempotent re-upload of an identical OPT: 200 (idempotent) or 409 both
        // satisfy "no data corruption"; the spec is silent on which (§upload_opt).
        let status = upload_opt(ctx, xml).await?;
        if matches!(status, 200 | 204 | 409) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "idempotent re-upload expected 200/409, got {status}"
            )))
        }
    })
}

async fn get_template(ctx: &RunContext<'_>, path: String) -> Result<u16, CaseError> {
    let resp = ctx
        .send(HttpRequest::get(path).header("accept", "application/xml"))
        .await?;
    Ok(resp.status)
}

fn run_get_single<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let tid = ensure_present(ctx).await?;
        let status = get_template(ctx, format!("/definition/template/adl1.4/{tid}")).await?;
        assert_eq(status, 200)
    })
}

fn run_get_latest<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let tid = ensure_present(ctx).await?;
        let status = get_template(ctx, format!("/definition/template/adl1.4/{tid}")).await?;
        assert_eq(status, 200)
    })
}

fn run_get_specific<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let tid = ensure_present(ctx).await?;
        // ADL 1.4 OPTs are not version-addressed in ITS-REST; a specific-version
        // GET is 200 (if aliased to latest) or 404 (unsupported) — both conformant.
        let status = get_template(ctx, format!("/definition/template/adl1.4/{tid}/1.0.0")).await?;
        assert_in(status, &[200, 404])
    })
}

fn run_get_fail<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let status = get_template(
            ctx,
            "/definition/template/adl1.4/does.not.exist.v1".to_owned(),
        )
        .await?;
        assert_eq(status, 404)
    })
}

fn run_get_all<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        ensure_present(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get("/definition/template/adl1.4")
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_validate_valid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // The upload endpoint validates: a valid OPT is accepted (2xx) or already
        // present (409) — either proves it passed validation.
        let xml = fixtures::read("valid_templates/minimal/minimal_evaluation.opt")
            .map_err(|e| CaseError::Codec(e.to_string()))?;
        let status = upload_opt(ctx, xml).await?;
        if (200..300).contains(&status) || status == 409 {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "valid OPT not accepted: {status}"
            )))
        }
    })
}

fn run_validate_invalid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let opts = fixtures::opts_invalid().map_err(|e| CaseError::Codec(e.to_string()))?;
        let opt = opts
            .first()
            .ok_or_else(|| CaseError::Assertion("no invalid OPT fixtures".to_owned()))?;
        let xml = opt.read().map_err(|e| CaseError::Codec(e.to_string()))?;
        let status = upload_opt(ctx, xml).await?;
        if (400..500).contains(&status) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "invalid OPT {} not rejected (got {status})",
                opt.name
            )))
        }
    })
}

// delete_opt — D2 skip-with-reason (module docs): the SM
// `I_DEFINITION_ADL14.delete_opt()` (CNF master04:319) has no ITS-REST ADL 1.4
// binding — no DELETE verb on `/definition/template/adl1.4/{id}` in Release-1.0.3
// or the tested development@e8a093e OAS (deletion is in the ADMIN API only). A
// 405 there is a schedule gap, not a server defect, so every case is skipped.
const DELETE_OPT_CITATION: &str = "SM I_DEFINITION_ADL14.delete_opt() (CNF master04:319) — no ITS-REST ADL 1.4 binding \
     (no DELETE on /definition/template/adl1.4/{id}; deletion is ADMIN-API-only); skipped, \
     see module docs";

/// The CNF-schedule trace for the `delete_opt` cases (task 7): the SM operation +
/// its schedule locus.
const DELETE_OPT_SCHEDULE_REF: &str = "I_DEFINITION_ADL14.delete_opt (CNF master04:319)";

const DELETE_OPT_SKIP: &str = "SM I_DEFINITION_ADL14.delete_opt() (CNF master04:319) has no ITS-REST ADL 1.4 binding — \
     ITS-REST development@e8a093e (and Release-1.0.3) define no DELETE verb on \
     /definition/template/adl1.4/{id}; OPT deletion lives in the ADMIN API only";

fn skip_delete<'a>() -> CaseFuture<'a> {
    Box::pin(async move { Err::<DataSetReport, _>(CaseError::Skipped(DELETE_OPT_SKIP.to_owned())) })
}

fn run_delete_absent<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    skip_delete()
}

fn run_delete_existing<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    skip_delete()
}

fn run_delete_latest<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    skip_delete()
}

fn run_delete_specific<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    skip_delete()
}

/// Assert a status equals `want`, else a finding.
fn assert_eq(status: u16, want: u16) -> Result<DataSetReport, CaseError> {
    if status == want {
        Ok(DataSetReport::SINGLE)
    } else {
        Err(CaseError::Assertion(format!(
            "expected {want}, got {status}"
        )))
    }
}

/// Assert a status is in `allowed`, else a finding.
fn assert_in(status: u16, allowed: &[u16]) -> Result<DataSetReport, CaseError> {
    if allowed.contains(&status) {
        Ok(DataSetReport::SINGLE)
    } else {
        Err(CaseError::Assertion(format!(
            "expected one of {allowed:?}, got {status}"
        )))
    }
}

fn entry(
    id: &'static str,
    title: &'static str,
    citation: &'static str,
    capability: Capability,
    run: CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Tpl,
            capability,
            profiles: &[Profile::Core, Profile::Standard],
            formats: &[Format::Json],
            citation,
            compare: Compare::Superset,
            schedule_ref: None,
        },
        run,
    }
}

/// Upload an OPT XML body to the ADL 1.4 template endpoint.
async fn upload_opt(ctx: &RunContext<'_>, xml: String) -> Result<u16, CaseError> {
    let resp = ctx
        .send(
            HttpRequest::post("/definition/template/adl1.4")
                .text_body(xml, "application/xml")
                .header("accept", "application/json"),
        )
        .await?;
    Ok(resp.status)
}

fn run_upload_valid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // A fresh, valid OPT must be accepted with 201. The SUT is
        // shared across cases, so `minimal_evaluation.en.v1` may already be
        // provisioned (e.g. by `ensure_present`); uploading it verbatim would
        // then (correctly) 409 and this case would wrongly read as a rejected
        // upload. Retarget the OPT to a unique `template_id` via the typed opt14
        // model so it genuinely asserts a *fresh-upload* 201, order-independent.
        let base = fixtures::read("valid_templates/minimal/minimal_evaluation.opt")
            .map_err(|e| CaseError::Codec(e.to_string()))?;
        let mut opt = openehr_its::opt14::from_xml(&base)
            .map_err(|e| CaseError::Codec(format!("parse minimal_evaluation.opt: {e}")))?;
        // Replicated locally (super::content::author::set_template_id is not
        // edited): give the template a unique id per run.
        opt.template_id.value = format!("minimal_evaluation.fresh.{}.v1", uuid::Uuid::new_v4());
        let xml = openehr_its::opt14::to_xml(&opt)
            .map_err(|e| CaseError::Codec(format!("serialize retargeted OPT: {e}")))?;
        let status = upload_opt(ctx, xml).await?;
        if (200..300).contains(&status) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "fresh valid OPT rejected with {status}"
            )))
        }
    })
}

fn run_upload_invalid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ upload_invalid_set(ctx).await })
}

/// Upload every invalid `.opt` and require each be rejected (`4xx`).
async fn upload_invalid_set(ctx: &RunContext<'_>) -> Result<DataSetReport, CaseError> {
    let opts = fixtures::opts_invalid().map_err(|e| CaseError::Codec(e.to_string()))?;
    let opts: Vec<_> = opts
        .into_iter()
        .filter(|f| {
            std::path::Path::new(&f.name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("opt"))
        })
        .collect();
    let mut passed = 0u32;
    let mut total = 0u32;
    for opt in opts {
        total += 1;
        let xml = opt.read().map_err(|e| CaseError::Codec(e.to_string()))?;
        let status = upload_opt(ctx, xml).await?;
        if (400..500).contains(&status) {
            passed += 1;
        }
    }
    if passed == total {
        Ok(DataSetReport { passed, total })
    } else {
        Err(CaseError::Assertion(format!(
            "{passed}/{total} invalid OPTs were rejected (the rest were accepted)"
        )))
    }
}

fn run_list<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get("/definition/template/adl1.4")
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
