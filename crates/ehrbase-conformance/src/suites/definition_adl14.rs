//! master04 — DEFINITION: ADL 1.4 / OPT 1.4 provisioning (design §4.1).
//!
//! Transcribed from `master04-func_tc_definition_adl.adoc`, driving the ITS-REST
//! `/definition/template/adl1.4` surface with the vendored `valid_templates` and
//! `invalid_templates` `.opt` fixtures (valid **and** invalid — both load-bearing).
//! Assertions concretize the OPT provisioning contract (`2xx` accept a valid OPT;
//! `4xx` reject an invalid one; `200` list).
//!
//! The `get_opt-*`, `delete_opt-*`, and version cases need a `template_id`
//! round-trip / a delete endpoint our surface does not expose, so they stay
//! `NotYetTranscribed`.

use crate::assert;
use crate::case::{Capability, CaseMeta, Chapter, Compare, Format, Profile, Provenance};
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;

/// The implemented master04 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry(
            "I_DEFINITION_ADL14.upload_opt-valid_opt",
            Capability::Adl14OptProvisioning,
            run_upload_valid,
        ),
        entry(
            "I_DEFINITION_ADL14.upload_opt-invalid_opt",
            Capability::Adl14OptProvisioning,
            run_upload_invalid,
        ),
        entry(
            "I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts",
            Capability::Adl14OptProvisioning,
            run_list,
        ),
    ]
}

fn entry(id: &'static str, capability: Capability, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master04,
            capability,
            profiles: &[Profile::Core, Profile::Standard],
            formats: &[Format::Json],
            provenance: Provenance::Schedule,
            schedule_ref: id,
            upstream_tags: &[],
            compare: Compare::Superset,
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

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

fn run_upload_valid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // One valid OPT must be accepted (a distinct file per case avoids
        // template_id collisions with other cases in the same server).
        let opts = fixtures::opts_valid().map_err(|e| CaseError::Codec(e.to_string()))?;
        let opt = opts
            .iter()
            .find(|f| f.name.contains("minimal_evaluation"))
            .or_else(|| opts.first())
            .ok_or_else(|| CaseError::Assertion("no valid OPT fixtures".to_owned()))?;
        let xml = opt.read().map_err(|e| CaseError::Codec(e.to_string()))?;
        let status = upload_opt(ctx, xml).await?;
        if (200..300).contains(&status) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "valid OPT {} rejected with {status}",
                opt.name
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
        .filter(|f| f.name.ends_with(".opt"))
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
