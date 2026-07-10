//! EHR + `EHR_STATUS` cases (design §4.1: `suites/ehr.rs`).
//!
//! Our own ECC cases (reference: `master06-func_tc_ehr.adoc`, design-time
//! reading), plus a fixture-derived negative case over the vendored invalid
//! `EHR_STATUS` data sets. Positive payloads consume the vendored `ehr/valid`
//! fixtures (RM-1.2.0-adapted per §6); negatives post the vendored `ehr/invalid`
//! set verbatim. Assertions concretize the ITS-REST EHR API status contract
//! (`201_EHR`: `ETag(ehr_id)` + Location; `200_EHR`; `404` for absent resources;
//! `409` for duplicate id/subject).

use serde_json::{Value, json};
use uuid::Uuid;

use openehr_rm::prelude::Ehr;

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;

/// The implemented master06 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry(
            "ehr/has-ehr-existing-ehr-id",
            "EHR existence check — existing EHR id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_has_ehr_existing_ehr_id,
        ),
        entry(
            "ehr/has-ehr-existing-subject-id",
            "EHR existence check — existing subject id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_has_ehr_existing_subject_id,
        ),
        entry(
            "ehr/has-ehr-non-existing-ehr-id",
            "EHR existence check — non existing EHR id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_has_ehr_non_existing_ehr_id,
        ),
        entry(
            "ehr/has-ehr-non-existing-subject-id",
            "EHR existence check — non existing subject id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_has_ehr_non_existing_subject_id,
        ),
        entry(
            "ehr/create-ehr-main",
            "Create EHR — main",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_create_ehr_main,
        ),
        entry(
            "ehr/create-ehr-same-ehr-twice",
            "Create EHR — same EHR twice",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_create_ehr_same_ehr_twice,
        ),
        entry(
            "ehr/create-ehr-two-ehrs-same-patient",
            "Create EHR — two EHRs same patient",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_create_ehr_two_ehrs_same_patient,
        ),
        entry(
            "ehr/get-ehr-existing-ehr-by-ehr-id",
            "Get EHR — existing EHR by EHR id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_get_ehr_existing_by_ehr_id,
        ),
        entry(
            "ehr/get-ehr-existing-ehr-by-subject-id",
            "Get EHR — existing EHR by subject id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_get_ehr_existing_by_subject_id,
        ),
        entry(
            "ehr/get-ehr-get-ehr-by-invalid-ehr-id",
            "Get EHR — get EHR by invalid EHR id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_get_ehr_invalid_ehr_id,
        ),
        entry(
            "ehr/get-ehr-get-ehr-by-invalid-subject-id",
            "Get EHR — get EHR by invalid subject id",
            "ITS-REST 1.0.3 EHR API §create_ehr/get_ehr; RM 1.2.0 ehr §EHR",
            Capability::EhrOperations,
            run_get_ehr_invalid_subject_id,
        ),
        entry(
            "sta/get-ehr-status-get-by-ehr-id",
            "Get EHR_STATUS — get by EHR id",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_get_ehr_status_by_ehr_id,
        ),
        entry(
            "sta/get-ehr-status-bad-ehr",
            "Get EHR_STATUS — bad EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_get_ehr_status_bad_ehr,
        ),
        entry(
            "sta/set-ehr-queryable-existing-ehr",
            "Set EHR_STATUS is_queryable — existing EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_set_ehr_queryable_existing,
        ),
        entry(
            "sta/set-ehr-queryable-bad-ehr",
            "Set EHR_STATUS is_queryable — bad EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_set_ehr_queryable_bad,
        ),
        entry(
            "sta/set-ehr-modifiable-existing-ehr",
            "Set EHR_STATUS is_modifiable — existing EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_set_ehr_modifiable_existing,
        ),
        entry(
            "sta/set-ehr-modifiable-bad-ehr",
            "Set EHR_STATUS is_modifiable — bad EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_set_ehr_modifiable_bad,
        ),
        entry(
            "sta/clear-ehr-queryable-existing-ehr",
            "Clear EHR_STATUS is_queryable — existing EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_clear_ehr_queryable_existing,
        ),
        entry(
            "sta/clear-ehr-queryable-bad-ehr",
            "Clear EHR_STATUS is_queryable — bad EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_clear_ehr_queryable_bad,
        ),
        entry(
            "sta/clear-ehr-modifiable-existing-ehr",
            "Clear EHR_STATUS is_modifiable — existing EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_clear_ehr_modifiable_existing,
        ),
        entry(
            "sta/clear-ehr-modifiable-bad-ehr",
            "Clear EHR_STATUS is_modifiable — bad EHR",
            "ITS-REST 1.0.3 EHR_STATUS API §get_ehr_status/update_ehr_status; RM 1.2.0 ehr §EHR_STATUS",
            Capability::EhrStatus,
            run_clear_ehr_modifiable_bad,
        ),
        // Fixture-derived negative: the vendored invalid EHR_STATUS data sets
        // (master06 §Test Data Sets class 2) must be rejected.
        CaseEntry {
            meta: CaseMeta {
                id: "ehr/create-ehr-invalid-status",
                title: "Create EHR — reject invalid EHR_STATUS data sets",
                area: Area::Ehr,
                capability: Capability::EhrOperations,
                profiles: &[Profile::Core, Profile::Standard],
                formats: &[Format::Json],
                citation: "ITS-REST 1.0.3 EHR API §create_ehr (422); RM 1.2.0 ehr §EHR_STATUS validation",
                compare: Compare::Superset,
                schedule_ref: None,
            },
            run: run_create_ehr_invalid_status,
        },
        // D5: the CORE + STANDARD non-functional "Anonymous EHRs" capability
        // (`master03-profiles.adoc` §Non-Functional). Creating an EHR with no
        // body yields a subject-less (anonymous) EHR — the capability had zero
        // tagged cases before, making CORE unclaimable by construction.
        CaseEntry {
            meta: CaseMeta {
                id: "ehr/create-anonymous-ehr",
                title: "Create anonymous (subject-less) EHR",
                area: Area::Ehr,
                capability: Capability::AnonymousEhrs,
                profiles: &[Profile::Core, Profile::Standard],
                formats: &[Format::Json],
                citation: "CNF master03-profiles §Non-Functional (Anonymous EHRs — CORE+STANDARD); \
                           ITS-REST EHR API §create_ehr (no body); RM 1.2.0 ehr §EHR_STATUS",
                compare: Compare::Superset,
                schedule_ref: None,
            },
            run: run_create_anonymous_ehr,
        },
    ]
}

/// An ECC case with the shared metadata; `area` is derived from `capability`.
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
            area: if matches!(capability, Capability::EhrStatus) {
                Area::Sta
            } else {
                Area::Ehr
            },
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

// ── shared helpers ───────────────────────────────────────────────────────────

/// A valid `EHR_STATUS` from the vendored corpus, RM-1.2.0-adapted, with the
/// given unique subject identity injected (namespace `conformance`).
fn valid_status(subject_id: &str) -> Result<Value, CaseError> {
    let raw = fixtures::read_json("ehr/valid/000_ehr_status.json")
        .map_err(|e| CaseError::Codec(e.to_string()))?;
    Ok(fixtures::adapt_ehr_status(raw, "conformance", subject_id))
}

/// POST `/ehr` (optionally with an `EHR_STATUS` body), asserting `201` +
/// ETag/Location, and return the created `ehr_id`.
async fn create_ehr(ctx: &RunContext<'_>, status: Option<&Value>) -> Result<String, CaseError> {
    let mut req = HttpRequest::post("/ehr")
        .header("accept", "application/json")
        .header("prefer", "return=representation");
    if let Some(body) = status {
        req = req.json_body(body)?;
    }
    let resp = ctx.send(req).await?;
    assert::status(&resp, 201)?;
    assert::header_present(&resp, "etag")?;
    assert::header_present(&resp, "location")?;
    let body = resp.text();
    openehr_its::json::from_canonical_json::<Ehr>(&body)
        .map_err(|e| CaseError::Assertion(format!("returned EHR is not valid RM: {e}")))?;
    ehr_id_of(&resp.json()?)
}

/// Extract `ehr_id.value` from an EHR representation body.
fn ehr_id_of(ehr: &Value) -> Result<String, CaseError> {
    ehr["ehr_id"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("EHR body has no ehr_id".to_owned()))
}

/// GET `/ehr/{id}/ehr_status`, asserting `200`, returning the status body.
async fn get_status(ctx: &RunContext<'_>, ehr_id: &str) -> Result<Value, CaseError> {
    let resp = ctx
        .send(
            HttpRequest::get(format!("/ehr/{ehr_id}/ehr_status"))
                .header("accept", "application/json"),
        )
        .await?;
    assert::status(&resp, 200)?;
    resp.json()
}

/// Update `EHR_STATUS`, flipping `field` to `target`, then assert the served
/// status reflects it (the set/clear queryable/modifiable flow).
async fn update_status_field(
    ctx: &RunContext<'_>,
    field: &str,
    target: bool,
) -> Result<DataSetReport, CaseError> {
    let ehr_id = create_ehr(ctx, None).await?;
    let mut status = get_status(ctx, &ehr_id).await?;
    let version = status["uid"]["value"]
        .as_str()
        .ok_or_else(|| CaseError::Assertion("EHR_STATUS has no uid".to_owned()))?
        .to_owned();
    status[field] = json!(target);
    let put = HttpRequest::put(format!("/ehr/{ehr_id}/ehr_status"))
        .json_body(&status)?
        .header("accept", "application/json")
        .header("prefer", "return=representation")
        .header("if-match", version);
    let resp = ctx.send(put).await?;
    assert::status_in(&resp, &[200, 204])?;
    let after = get_status(ctx, &ehr_id).await?;
    if after[field] == json!(target) {
        Ok(DataSetReport::SINGLE)
    } else {
        Err(CaseError::Assertion(format!(
            "{field} should be {target} after update, got {}",
            after[field]
        )))
    }
}

/// The set/clear `bad_ehr` flow: the operation against a random `ehr_id` must be
/// a negative response (the EHR does not exist).
async fn update_status_bad_ehr(
    ctx: &RunContext<'_>,
    field: &str,
    target: bool,
) -> Result<DataSetReport, CaseError> {
    let ehr_id = Uuid::new_v4();
    let mut body = valid_status(&format!("subj-{}", Uuid::new_v4()))?;
    body[field] = json!(target);
    let put = HttpRequest::put(format!("/ehr/{ehr_id}/ehr_status"))
        .json_body(&body)?
        .header("accept", "application/json")
        .header("if-match", format!("{ehr_id}::conformance::1"));
    let resp = ctx.send(put).await?;
    // A negative response for a non-existent EHR (ITS-REST: 404; 412 if the
    // precondition is evaluated first). Never a 2xx.
    assert::status_in(&resp, &[400, 404, 412])?;
    Ok(DataSetReport::SINGLE)
}

/// Box a plain async result as a [`CaseFuture`].
macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── I_EHR_SERVICE.has_ehr ────────────────────────────────────────────────────

fn run_has_ehr_existing_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = Uuid::new_v4();
        let put = HttpRequest::put(format!("/ehr/{ehr_id}")).header("prefer", "return=minimal");
        assert::status(&ctx.send(put).await?, 201)?;
        let got = ctx
            .send(HttpRequest::get(format!("/ehr/{ehr_id}")).header("accept", "application/json"))
            .await?;
        assert::status(&got, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_ehr_existing_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let subject = format!("subj-{}", Uuid::new_v4());
        create_ehr(ctx, Some(&valid_status(&subject)?)).await?;
        let got = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr?subject_id={subject}&subject_namespace=conformance"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_ehr_non_existing_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let got = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_ehr_non_existing_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let got = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr?subject_id=nobody-{}&subject_namespace=conformance",
                    Uuid::new_v4()
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── I_EHR_SERVICE.create_ehr ─────────────────────────────────────────────────

/// One row of the normative "valid data sets" table (master06 §Test Data Sets):
/// `(is_queryable, is_modifiable, other_details_provided, ehr_id_provided)`.
const DATA_SETS: [(bool, bool, bool, bool); 16] = [
    (true, true, false, false),
    (true, false, false, false),
    (false, true, false, false),
    (false, false, false, false),
    (true, true, true, false),
    (true, false, true, false),
    (false, true, true, false),
    (false, false, true, false),
    (true, true, false, true),
    (true, false, false, true),
    (false, true, false, true),
    (false, false, false, true),
    (true, true, true, true),
    (true, false, true, true),
    (false, true, true, true),
    (false, false, true, true),
];

/// Build a valid `EHR_STATUS` for a data-set row with a unique subject.
fn ehr_status_row(row: (bool, bool, bool, bool), subject_id: &str) -> Value {
    let (is_queryable, is_modifiable, other_details, _) = row;
    let mut status = json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": {
            "_type": "PARTY_IDENTIFIED",
            "external_ref": {
                "_type": "PARTY_REF",
                "namespace": "conformance",
                "type": "PERSON",
                "id": { "_type": "HIER_OBJECT_ID", "value": subject_id }
            }
        },
        "is_queryable": is_queryable,
        "is_modifiable": is_modifiable
    });
    if other_details {
        status["other_details"] = json!({
            "_type": "ITEM_TREE",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": "other details" },
            "items": [{
                "_type": "ELEMENT",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "note" },
                "value": { "_type": "DV_TEXT", "value": "conformance" }
            }]
        });
    }
    status
}

fn run_create_ehr_main<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let mut passed = 0u32;
        let mut total = 0u32;
        for row in DATA_SETS {
            total += 1;
            let subject_id = format!("conf-subj-{}", Uuid::new_v4());
            let status = ehr_status_row(row, &subject_id);
            let request = if row.3 {
                let ehr_id = Uuid::new_v4();
                HttpRequest::put(format!("/ehr/{ehr_id}"))
            } else {
                HttpRequest::post("/ehr")
            }
            .json_body(&status)?
            .header("accept", "application/json")
            .header("prefer", "return=representation");
            let response = ctx.send(request).await?;
            assert::status(&response, 201)?;
            assert::header_present(&response, "etag")?;
            assert::header_present(&response, "location")?;
            openehr_its::json::from_canonical_json::<Ehr>(&response.text())
                .map_err(|e| CaseError::Assertion(format!("returned EHR is not valid RM: {e}")))?;
            let ehr_id = ehr_id_of(&response.json()?)?;
            let got = ctx
                .send(
                    HttpRequest::get(format!("/ehr/{ehr_id}")).header("accept", "application/json"),
                )
                .await?;
            assert::status(&got, 200)?;
            passed += 1;
        }
        Ok(DataSetReport { passed, total })
    })
}

/// D5: an anonymous EHR — `POST /ehr` with no body creates a subject-less EHR
/// (its default `EHR_STATUS` carries no `subject.external_ref`). Evidences the
/// CORE+STANDARD "Anonymous EHRs" non-functional capability
/// (`master03-profiles.adoc` §Non-Functional).
fn run_create_anonymous_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = create_ehr(ctx, None).await?;
        let status = get_status(ctx, &ehr_id).await?;
        // Anonymous = no identified subject: the default PARTY_SELF/PARTY_PROXY
        // carries no external_ref (RM 1.2.0 common §PARTY_SELF).
        if status["subject"].get("external_ref").is_some() {
            return Err(CaseError::Assertion(
                "anonymous EHR_STATUS subject unexpectedly carries an external_ref".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_ehr_same_ehr_twice<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = Uuid::new_v4();
        let first = ctx
            .send(HttpRequest::put(format!("/ehr/{ehr_id}")).header("prefer", "return=minimal"))
            .await?;
        assert::status(&first, 201)?;
        // The same ehr_id again must conflict (409_EHR: an EHR already exists).
        let second = ctx
            .send(HttpRequest::put(format!("/ehr/{ehr_id}")).header("prefer", "return=minimal"))
            .await?;
        assert::status(&second, 409)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_ehr_two_ehrs_same_patient<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let subject = format!("subj-{}", Uuid::new_v4());
        let status = valid_status(&subject)?;
        create_ehr(ctx, Some(&status)).await?;
        // A second EHR for the same subject must conflict (409_EHR).
        let second = ctx
            .send(
                HttpRequest::post("/ehr")
                    .json_body(&status)?
                    .header("accept", "application/json")
                    .header("prefer", "return=minimal"),
            )
            .await?;
        assert::status(&second, 409)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── I_EHR_SERVICE.get_ehr ────────────────────────────────────────────────────

fn run_get_ehr_existing_by_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = create_ehr(ctx, None).await?;
        let got = ctx
            .send(HttpRequest::get(format!("/ehr/{ehr_id}")).header("accept", "application/json"))
            .await?;
        assert::status(&got, 200)?;
        openehr_its::json::from_canonical_json::<Ehr>(&got.text())
            .map_err(|e| CaseError::Assertion(format!("retrieved EHR is not valid RM: {e}")))?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_ehr_existing_by_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let subject = format!("subj-{}", Uuid::new_v4());
        create_ehr(ctx, Some(&valid_status(&subject)?)).await?;
        let got = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr?subject_id={subject}&subject_namespace=conformance"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_ehr_invalid_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let got = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_ehr_invalid_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let got = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr?subject_id=nobody-{}&subject_namespace=conformance",
                    Uuid::new_v4()
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── I_EHR_STATUS.get_ehr_status ──────────────────────────────────────────────

fn run_get_ehr_status_by_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = create_ehr(ctx, None).await?;
        let status = get_status(ctx, &ehr_id).await?;
        if status["_type"] == "EHR_STATUS" {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected EHR_STATUS, got {}",
                status["_type"]
            )))
        }
    })
}

fn run_get_ehr_status_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let got = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}/ehr_status", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&got, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── I_EHR_STATUS.set/clear queryable/modifiable ──────────────────────────────

fn run_set_ehr_queryable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_field(ctx, "is_queryable", true).await })
}
fn run_set_ehr_queryable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_bad_ehr(ctx, "is_queryable", true).await })
}
fn run_set_ehr_modifiable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_field(ctx, "is_modifiable", true).await })
}
fn run_set_ehr_modifiable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_bad_ehr(ctx, "is_modifiable", true).await })
}
fn run_clear_ehr_queryable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_field(ctx, "is_queryable", false).await })
}
fn run_clear_ehr_queryable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_bad_ehr(ctx, "is_queryable", false).await })
}
fn run_clear_ehr_modifiable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_field(ctx, "is_modifiable", false).await })
}
fn run_clear_ehr_modifiable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ update_status_bad_ehr(ctx, "is_modifiable", false).await })
}

// ── Fixture-derived negative: invalid EHR_STATUS data sets ───────────────────

fn run_create_ehr_invalid_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let invalid = fixtures::ehr_invalid().map_err(|e| CaseError::Codec(e.to_string()))?;
        let mut passed = 0u32;
        let mut total = 0u32;
        for fixture in invalid {
            total += 1;
            let body = fixture
                .json()
                .map_err(|e| CaseError::Codec(e.to_string()))?;
            let resp = ctx
                .send(
                    HttpRequest::post("/ehr")
                        .json_body(&body)?
                        .header("accept", "application/json")
                        .header("prefer", "return=minimal"),
                )
                .await?;
            // An invalid EHR_STATUS must be rejected (4xx): 400 (malformed) or
            // 422 (RM/terminology validation).
            if (400..500).contains(&resp.status) {
                passed += 1;
            }
        }
        if passed == total {
            Ok(DataSetReport { passed, total })
        } else {
            Err(CaseError::Assertion(format!(
                "{passed}/{total} invalid EHR_STATUS data sets were rejected (the rest were accepted)"
            )))
        }
    })
}
