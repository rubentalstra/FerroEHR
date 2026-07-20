//! EHR + `EHR_STATUS` cases — the master06 spine.
//!
//! Every case concretizes a `master06-func_tc_ehr.adoc` test case (its
//! [`ScheduleTrace`] carries the `<I_*.op-case>` form), plus two
//! extensions (the invalid-`EHR_STATUS` data-set-class negative and the
//! non-functional Anonymous-EHRs capability case). The SM `has_ehr` /
//! `set/clear …` operations have no discrete ITS-REST verb, so they are
//! realized over the bound REST resources (`GET /ehr/{id}`,
//! `PUT /ehr/{id}/ehr_status`) per the CNF guide's abstract-call → REST
//! mapping — the binding string records exactly which resource each drives.
//!
//! Wire ids come ONLY from [`crate::wire`]; there are no `::system::`
//! literals — negative `If-Match` ids are built from an OBSERVED id via
//! [`support::nonexistent_version_like`]. Positive
//! `EHR_STATUS` payloads are authored at the RM 1.2.0 canonical shape (the
//! pinned Release-1.1.0).
//
// NOTE: the RM wire version ladder is only partially met:
// EHR_STATUS request payloads are authored at RM 1.2.0 (PARTY_SELF subject,
// RM ehr master04 §EHR Status). A per-edition request-payload provider
// (RM 1.0.2 minimum, master03-overview §API Conformance) belongs to the
// wire adapter, which does not yet expose one; our pinned CI runs
// Release-1.1.0, so this is exercised faithfully today.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::engine::assert;
use crate::engine::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::testdata::fixtures;
use crate::wire::{ids, negotiate};

/// JSON is the wire format the EHR/`EHR_STATUS` cases run under (the payloads
/// carry no XML-only surface the schedule tabulates).
const JSON: &[Format] = &[Format::Json];

/// The subject namespace for authored `EHR_STATUS` identities. Test-data
/// only — NOT a creating-system-id assumption about the SUT (only wire-id
/// literals are forbidden).
const SUBJECT_NS: &str = "conformance";

/// Every registered EHR/`EHR_STATUS` case (21 schedule + 2 extensions).
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the registered ECC case table is inherently enumerative"
)]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── I_EHR_SERVICE.has_ehr ──────────────────────────────────────────
        case(
            "ehr/has-ehr-existing-ehr-id",
            "EHR existence check — existing EHR id",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §has_ehr; ITS-REST 1.1.0 EHR API ehr_get.yaml 200; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule("I_EHR_SERVICE.has_ehr-existing_ehr_id (master06 §has_ehr)"),
            Binding::Rest("PUT /ehr/{ehr_id}; GET /ehr/{ehr_id}"),
            run_has_ehr_existing_ehr_id,
        ),
        case(
            "ehr/has-ehr-existing-subject-id",
            "EHR existence check — existing subject id",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §has_ehr; ITS-REST 1.1.0 EHR API ehr_get_by_subject.yaml 200; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.has_ehr-existing_subject_id (master06 §has_ehr)",
            ),
            Binding::Rest("GET /ehr?subject_id&subject_namespace"),
            run_has_ehr_existing_subject_id,
        ),
        case(
            "ehr/has-ehr-non-existing-ehr-id",
            "EHR existence check — non existing EHR id",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §has_ehr; ITS-REST 1.1.0 EHR API ehr_get.yaml 404; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.has_ehr-non_existing_ehr_id (master06 §has_ehr)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}"),
            run_has_ehr_non_existing_ehr_id,
        ),
        case(
            "ehr/has-ehr-non-existing-subject-id",
            "EHR existence check — non existing subject id",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §has_ehr; ITS-REST 1.1.0 EHR API ehr_get_by_subject.yaml 404; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.has_ehr-non_existing_subject_id (master06 §has_ehr)",
            ),
            Binding::Rest("GET /ehr?subject_id&subject_namespace"),
            run_has_ehr_non_existing_subject_id,
        ),
        // ── I_EHR_SERVICE.create_ehr ───────────────────────────────────────
        case(
            "ehr/create-ehr-main",
            "Create EHR — main (valid data-set matrix)",
            Capability::EhrOperations,
            Compare::Superset,
            "CNF master06-func_tc_ehr §create_ehr + §Test Data Sets class 1.a; ITS-REST 1.1.0 EHR API ehr_create.yaml/ehr_create_with_id.yaml 201; RM 1.2.0 ehr §EHR_STATUS",
            ScheduleTrace::Schedule("I_EHR_SERVICE.create_ehr-main (master06 §create_ehr)"),
            Binding::Rest("POST /ehr; PUT /ehr/{ehr_id}"),
            run_create_ehr_main,
        ),
        case(
            "ehr/create-ehr-same-ehr-twice",
            "Create EHR — same EHR twice",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §create_ehr; ITS-REST 1.1.0 EHR API ehr_create_with_id.yaml 409; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.create_ehr-same_ehr_twice (master06 §create_ehr)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}"),
            run_create_ehr_same_ehr_twice,
        ),
        case(
            "ehr/create-ehr-two-ehrs-same-patient",
            "Create EHR — two EHRs same patient",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §create_ehr; ITS-REST 1.1.0 EHR API ehr_create.yaml 409; RM 1.2.0 ehr §EHR (one EHR per subject)",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.create_ehr-two_ehrs_same_patient (master06 §create_ehr)",
            ),
            Binding::Rest("POST /ehr"),
            run_create_ehr_two_ehrs_same_patient,
        ),
        // ── I_EHR_SERVICE.get_ehr ──────────────────────────────────────────
        case(
            "ehr/get-ehr-existing-ehr-by-ehr-id",
            "Get EHR — existing EHR by EHR id",
            Capability::EhrOperations,
            Compare::Superset,
            "CNF master06-func_tc_ehr §get_ehr; ITS-REST 1.1.0 EHR API ehr_get.yaml 200; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.get_ehr-existing_ehr_by_ehr_id (master06 §get_ehr)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}"),
            run_get_ehr_existing_by_ehr_id,
        ),
        case(
            "ehr/get-ehr-existing-ehr-by-subject-id",
            "Get EHR — existing EHR by subject id",
            Capability::EhrOperations,
            Compare::Superset,
            "CNF master06-func_tc_ehr §get_ehr; ITS-REST 1.1.0 EHR API ehr_get_by_subject.yaml 200; RM 1.2.0 ehr §EHR_STATUS subject identity",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.get_ehr-existing_ehr_by_subject_id (master06 §get_ehr)",
            ),
            Binding::Rest("GET /ehr?subject_id&subject_namespace"),
            run_get_ehr_existing_by_subject_id,
        ),
        case(
            "ehr/get-ehr-get-ehr-by-invalid-ehr-id",
            "Get EHR — get EHR by invalid EHR id",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §get_ehr; ITS-REST 1.1.0 EHR API ehr_get.yaml 404; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_ehr_id (master06 §get_ehr)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}"),
            run_get_ehr_invalid_ehr_id,
        ),
        case(
            "ehr/get-ehr-get-ehr-by-invalid-subject-id",
            "Get EHR — get EHR by invalid subject id",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §get_ehr; ITS-REST 1.1.0 EHR API ehr_get_by_subject.yaml 404; RM 1.2.0 ehr §EHR",
            ScheduleTrace::Schedule(
                "I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_subject_id (master06 §get_ehr)",
            ),
            Binding::Rest("GET /ehr?subject_id&subject_namespace"),
            run_get_ehr_invalid_subject_id,
        ),
        // ── I_EHR_STATUS.get_ehr_status ────────────────────────────────────
        case(
            "sta/get-ehr-status-get-by-ehr-id",
            "Get EHR_STATUS — get by EHR id",
            Capability::EhrStatus,
            Compare::Superset,
            "CNF master06-func_tc_ehr §get_ehr_status; ITS-REST 1.1.0 EHR_STATUS API ehr_status_get.yaml 200; RM 1.2.0 ehr §EHR_STATUS (subject/is_queryable/is_modifiable)",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.get_ehr_status-get_by_ehr_id (master06 §get_ehr_status)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/ehr_status"),
            run_get_ehr_status_by_ehr_id,
        ),
        case(
            "sta/get-ehr-status-bad-ehr",
            "Get EHR_STATUS — bad EHR",
            Capability::EhrStatus,
            Compare::None,
            "CNF master06-func_tc_ehr §get_ehr_status; ITS-REST 1.1.0 EHR_STATUS API ehr_status_get.yaml 404; RM 1.2.0 ehr §EHR_STATUS",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.get_ehr_status-bad_ehr (master06 §get_ehr_status)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/ehr_status"),
            run_get_ehr_status_bad_ehr,
        ),
        // ── I_EHR_STATUS.set/clear queryable/modifiable ────────────────────
        case(
            "sta/set-ehr-queryable-existing-ehr",
            "Set EHR_STATUS is_queryable — existing EHR",
            Capability::EhrStatus,
            Compare::Superset,
            "CNF master06-func_tc_ehr §set_ehr_queryable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_queryable",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.set_ehr_queryable-existing_ehr (master06 §set_ehr_queryable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_set_queryable_existing,
        ),
        case(
            "sta/set-ehr-queryable-bad-ehr",
            "Set EHR_STATUS is_queryable — bad EHR",
            Capability::EhrStatus,
            Compare::None,
            "CNF master06-func_tc_ehr §set_ehr_queryable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 4xx; RM 1.2.0 ehr §EHR_STATUS",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.set_ehr_queryable-bad_ehr (master06 §set_ehr_queryable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_set_queryable_bad,
        ),
        case(
            "sta/set-ehr-modifiable-existing-ehr",
            "Set EHR_STATUS is_modifiable — existing EHR",
            Capability::EhrStatus,
            Compare::Superset,
            "CNF master06-func_tc_ehr §set_ehr_modifiable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_modifiable",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.set_ehr_modifiable-existing_ehr (master06 §set_ehr_modifiable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_set_modifiable_existing,
        ),
        case(
            "sta/set-ehr-modifiable-bad-ehr",
            "Set EHR_STATUS is_modifiable — bad EHR",
            Capability::EhrStatus,
            Compare::None,
            "CNF master06-func_tc_ehr §set_ehr_modifiable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 4xx; RM 1.2.0 ehr §EHR_STATUS",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.set_ehr_modifiable-bad_ehr (master06 §set_ehr_modifiable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_set_modifiable_bad,
        ),
        case(
            "sta/clear-ehr-queryable-existing-ehr",
            "Clear EHR_STATUS is_queryable — existing EHR",
            Capability::EhrStatus,
            Compare::Superset,
            "CNF master06-func_tc_ehr §clear_ehr_queryable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_queryable",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.clear_ehr_queryable-existing_ehr (master06 §clear_ehr_queryable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_clear_queryable_existing,
        ),
        case(
            "sta/clear-ehr-queryable-bad-ehr",
            "Clear EHR_STATUS is_queryable — bad EHR",
            Capability::EhrStatus,
            Compare::None,
            "CNF master06-func_tc_ehr §clear_ehr_queryable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 4xx; RM 1.2.0 ehr §EHR_STATUS",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.clear_ehr_queryable-bad_ehr (master06 §clear_ehr_queryable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_clear_queryable_bad,
        ),
        case(
            "sta/clear-ehr-modifiable-existing-ehr",
            "Clear EHR_STATUS is_modifiable — existing EHR",
            Capability::EhrStatus,
            Compare::Superset,
            "CNF master06-func_tc_ehr §clear_ehr_modifiable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_modifiable",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.clear_ehr_modifiable-existing_ehr (master06 §clear_ehr_modifiable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_clear_modifiable_existing,
        ),
        case(
            "sta/clear-ehr-modifiable-bad-ehr",
            "Clear EHR_STATUS is_modifiable — bad EHR",
            Capability::EhrStatus,
            Compare::None,
            "CNF master06-func_tc_ehr §clear_ehr_modifiable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 4xx; RM 1.2.0 ehr §EHR_STATUS",
            ScheduleTrace::Schedule(
                "I_EHR_STATUS.clear_ehr_modifiable-bad_ehr (master06 §clear_ehr_modifiable)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/ehr_status"),
            run_clear_modifiable_bad,
        ),
        // ── §3 extensions ──────────────────────────────────────────────────
        case(
            "ehr/create-ehr-invalid-status",
            "Create EHR — reject invalid EHR_STATUS data sets",
            Capability::EhrOperations,
            Compare::None,
            "CNF master06-func_tc_ehr §Test Data Sets class 2 (invalid EHR_STATUS shapes); ITS-REST 1.1.0 EHR API ehr_create.yaml 400/422; RM ehr master04 §EHR Status + common §PARTY_SELF",
            ScheduleTrace::EccOriginal(
                "data-set class 2 (master06 §Test Data Sets, invalid EHR_STATUS shapes); no single master06 test case enumerates class 2",
            ),
            Binding::Rest("POST /ehr"),
            run_create_ehr_invalid_status,
        ),
        case(
            "ehr/create-anonymous-ehr",
            "Create anonymous (subject-less) EHR",
            Capability::AnonymousEhrs,
            Compare::Superset,
            "CNF master03-profiles §Non-Functional (Anonymous EHRs — CORE+STANDARD) + master06 §Test Data Sets class 1.b (default EHR_STATUS); ITS-REST 1.1.0 EHR API ehr_create.yaml (no body); RM ehr master04 §EHR Status, common §PARTY_SELF",
            ScheduleTrace::EccOriginal(
                "extension: Anonymous EHRs non-functional capability (master03-profiles §Non-Functional); doubles as class 1.b default-EHR_STATUS coverage; no master06 functional test case",
            ),
            Binding::Rest("POST /ehr"),
            run_create_anonymous_ehr,
        ),
    ]
}

/// Assemble a case entry; `area` is derived from `capability` (`EHR_STATUS`
/// operations file under [`Area::Sta`], the rest under [`Area::Ehr`]).
#[expect(
    clippy::too_many_arguments,
    reason = "case-table constructor: each CaseEntry/CaseMeta field is a distinct required argument"
)]
fn case(
    id: &'static str,
    title: &'static str,
    capability: Capability,
    compare: Compare,
    citation: &'static str,
    schedule: ScheduleTrace,
    binding: Binding,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    let area = if matches!(capability, Capability::EhrStatus) {
        Area::Sta
    } else {
        Area::Ehr
    };
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area,
            capability,
            formats: JSON,
            citation,
            schedule,
            binding,
            compare,
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

// ── EHR_STATUS payload authoring (RM ehr master04 §EHR Status) ───────────────

/// A valid RM 1.2.0 `EHR_STATUS` with a `PARTY_SELF` subject identified through
/// `external_ref` (RM ehr master04 §EHR Status: the subject identity travels on
/// `PARTY_SELF.external_ref`, never as a foreign `PARTY_IDENTIFIED`).
fn ehr_status(
    is_queryable: bool,
    is_modifiable: bool,
    other_details: bool,
    subject_id: &str,
) -> Value {
    let mut status = json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": {
            "_type": "PARTY_SELF",
            "external_ref": {
                "_type": "PARTY_REF",
                "namespace": SUBJECT_NS,
                "type": "PERSON",
                "id": { "_type": "GENERIC_ID", "scheme": "id_scheme", "value": subject_id }
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

/// A fresh, unique subject identity.
fn fresh_subject() -> String {
    format!("conf-subj-{}", Uuid::new_v4())
}

/// POST `/ehr` with an `EHR_STATUS` body, asserting `201` + `ETag` + Location,
/// returning the created `ehr_id`.
async fn create_ehr_with_status(ctx: &RunContext<'_>, status: &Value) -> Result<String, CaseError> {
    let req = negotiate::representation(HttpRequest::post("/ehr").json_body(status)?, Format::Json);
    let resp = ctx.send(req).await?;
    assert::status(&resp, 201)?;
    assert::header_present(&resp, "etag")?;
    assert::header_present(&resp, "location")?;
    ids::ehr_id(&resp.json()?)
}

/// GET `path` (JSON) and assert `404` — the absent-resource negative
/// (ITS-REST `EHR/EHR_STATUS` API `ehr_get.yaml/ehr_status_get.yaml` 404).
async fn get_expect_404(ctx: &RunContext<'_>, path: String) -> Result<DataSetReport, CaseError> {
    let resp = ctx
        .send(negotiate::accept(HttpRequest::get(path), Format::Json))
        .await?;
    assert::status(&resp, 404)?;
    Ok(DataSetReport::SINGLE)
}

/// GET `/ehr/{ehr_id}/ehr_status`, asserting `200`, returning the status body.
async fn get_status(ctx: &RunContext<'_>, ehr_id: &str) -> Result<Value, CaseError> {
    let resp = ctx
        .send(negotiate::accept(
            HttpRequest::get(format!("/ehr/{ehr_id}/ehr_status")),
            Format::Json,
        ))
        .await?;
    assert::status(&resp, 200)?;
    resp.json()
}

/// The set/clear queryable/modifiable happy path: create an EHR, flip `field`
/// to `target` via `PUT /ehr_status` under an `If-Match` carrying the current
/// `EHR_STATUS` `OBJECT_VERSION_ID` (from [`crate::wire::ids`]), then re-read
/// and assert the served flag reflects the change.
async fn update_flag(
    ctx: &RunContext<'_>,
    field: &str,
    target: bool,
) -> Result<DataSetReport, CaseError> {
    let ehr_id =
        create_ehr_with_status(ctx, &ehr_status(true, true, false, &fresh_subject())).await?;
    let mut status = get_status(ctx, &ehr_id).await?;
    let version = ids::body_uid(&status)?;
    status[field] = json!(target);
    let put = negotiate::if_match(
        negotiate::representation(
            HttpRequest::put(format!("/ehr/{ehr_id}/ehr_status")).json_body(&status)?,
            Format::Json,
        ),
        &version,
    );
    let resp = ctx.send(put).await?;
    assert::status_in(&resp, &[200, 204])?;
    let after = get_status(ctx, &ehr_id).await?;
    if after[field] == json!(target) {
        Ok(DataSetReport::SINGLE)
    } else {
        Err(CaseError::Assertion(format!(
            "EHR_STATUS.{field} should be {target} after update, got {}",
            after[field]
        )))
    }
}

/// The set/clear `bad_ehr` negative: the same update against a random (absent)
/// `ehr_id` must be a negative response. The `If-Match` is a syntactically
/// valid `OBJECT_VERSION_ID` naming a version the SUT does not hold — built
/// from an OBSERVED id via [`support::nonexistent_version_like`] so no
/// `::system::` literal is baked into the instrument.
async fn update_flag_bad_ehr(
    ctx: &RunContext<'_>,
    field: &str,
    target: bool,
) -> Result<DataSetReport, CaseError> {
    // Observe a real EHR_STATUS OVID (for the SUT's own creating-system id).
    let real_ehr =
        create_ehr_with_status(ctx, &ehr_status(true, true, false, &fresh_subject())).await?;
    let observed =
        ids::parse_object_version_id(&ids::body_uid(&get_status(ctx, &real_ehr).await?)?)?;
    let bogus_version = support::nonexistent_version_like(&observed);

    let mut body = ehr_status(true, true, false, &fresh_subject());
    body[field] = json!(target);
    let put = negotiate::if_match(
        HttpRequest::put(format!("/ehr/{}/ehr_status", Uuid::new_v4())).json_body(&body)?,
        &bogus_version,
    );
    let resp = ctx.send(put).await?;
    // The schedule wants a negative for a non-existent EHR; 404 (absent) and
    // 412 (precondition evaluated first) are both spec-valid negatives
    // (ITS-REST ehr_status_update.yaml) — a ladder-safe any-4xx assertion.
    support::assert_negative(&resp)?;
    Ok(DataSetReport::SINGLE)
}

// ── I_EHR_SERVICE.has_ehr ────────────────────────────────────────────────────

fn run_has_ehr_existing_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = Uuid::new_v4();
        let put = negotiate::prefer(
            HttpRequest::put(format!("/ehr/{ehr_id}")),
            negotiate::PreferReturn::Minimal,
        );
        assert::status(&ctx.send(put).await?, 201)?;
        let got = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/ehr/{ehr_id}")),
                Format::Json,
            ))
            .await?;
        assert::status(&got, 200)?; // SM has_ehr TRUE
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_ehr_existing_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let subject = fresh_subject();
        create_ehr_with_status(ctx, &ehr_status(true, true, false, &subject)).await?;
        let got = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!(
                    "/ehr?subject_id={subject}&subject_namespace={SUBJECT_NS}"
                )),
                Format::Json,
            ))
            .await?;
        assert::status(&got, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_ehr_non_existing_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ get_expect_404(ctx, format!("/ehr/{}", Uuid::new_v4())).await })
}

fn run_has_ehr_non_existing_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        get_expect_404(
            ctx,
            format!(
                "/ehr?subject_id=nobody-{}&subject_namespace={SUBJECT_NS}",
                Uuid::new_v4()
            ),
        )
        .await
    })
}

// ── I_EHR_SERVICE.create_ehr ─────────────────────────────────────────────────

fn run_create_ehr_main<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // master06 §Test Data Sets class 1.a: the 16-row valid matrix, the
        // cartesian product of (is_queryable, is_modifiable, other_details,
        // ehr_id_provided). Authored programmatically (preferred data-set
        // source: generated > owned > corpus) — this replaces the legacy Rust
        // literal that masqueraded as corpus data. The 8
        // ehr_id-absent rows POST /ehr; the 8 ehr_id-provided rows PUT
        // /ehr/{id}. Each creation is verified against the data set by reading
        // back EHR_STATUS and asserting the served flags.
        for i in 0u8..16 {
            let is_queryable = i & 0b0001 != 0;
            let is_modifiable = i & 0b0010 != 0;
            let other_details = i & 0b0100 != 0;
            let with_ehr_id = i & 0b1000 != 0;
            let status = ehr_status(is_queryable, is_modifiable, other_details, &fresh_subject());

            let (provided_id, resp) = if with_ehr_id {
                let ehr_id = Uuid::new_v4().to_string();
                let req = negotiate::representation(
                    HttpRequest::put(format!("/ehr/{ehr_id}")).json_body(&status)?,
                    Format::Json,
                );
                (Some(ehr_id), ctx.send(req).await?)
            } else {
                let req = negotiate::representation(
                    HttpRequest::post("/ehr").json_body(&status)?,
                    Format::Json,
                );
                (None, ctx.send(req).await?)
            };
            // ITS-REST ehr_create/ehr_create_with_id 201 response: ETag(ehr_id) + Location.
            assert::status(&resp, 201)?;
            assert::header_present(&resp, "etag")?;
            assert::header_present(&resp, "location")?;
            let ehr_id = match provided_id {
                Some(id) => id,
                None => ids::ehr_id(&resp.json()?)?,
            };

            // Re-read the EHR + its EHR_STATUS; the served flags must match the row.
            assert::status(
                &ctx.send(negotiate::accept(
                    HttpRequest::get(format!("/ehr/{ehr_id}")),
                    Format::Json,
                ))
                .await?,
                200,
            )?;
            let served = get_status(ctx, &ehr_id).await?;
            if served["is_queryable"] != json!(is_queryable)
                || served["is_modifiable"] != json!(is_modifiable)
            {
                return Err(CaseError::Assertion(format!(
                    "data-set row {i}: served flags (q={}, m={}) do not match committed (q={is_queryable}, m={is_modifiable})",
                    served["is_queryable"], served["is_modifiable"]
                )));
            }
        }
        Ok(DataSetReport::all(16).of_schedule_rows(16))
    })
}

fn run_create_ehr_same_ehr_twice<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = Uuid::new_v4();
        let mk = || {
            negotiate::prefer(
                HttpRequest::put(format!("/ehr/{ehr_id}")),
                negotiate::PreferReturn::Minimal,
            )
        };
        assert::status(&ctx.send(mk()).await?, 201)?;
        // The same ehr_id again must conflict (an EHR already exists).
        assert::status(&ctx.send(mk()).await?, 409)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_ehr_two_ehrs_same_patient<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let status = ehr_status(true, true, false, &fresh_subject());
        create_ehr_with_status(ctx, &status).await?;
        // A second EHR for the same subject must conflict.
        let second = ctx
            .send(negotiate::prefer(
                HttpRequest::post("/ehr").json_body(&status)?,
                negotiate::PreferReturn::Minimal,
            ))
            .await?;
        assert::status(&second, 409)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── I_EHR_SERVICE.get_ehr ────────────────────────────────────────────────────

fn run_get_ehr_existing_by_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = support::create_ehr(ctx).await?;
        let got = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/ehr/{ehr_id}")),
                Format::Json,
            ))
            .await?;
        assert::status(&got, 200)?;
        // Content check: the retrieved EHR is the one created (identity by ehr_id).
        if ids::ehr_id(&got.json()?)? != ehr_id {
            return Err(CaseError::Assertion(
                "retrieved EHR ehr_id does not match the created one".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_ehr_existing_by_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Assert the EHR returned by the subject query is the
        // one whose EHR_STATUS subject matches (identity, not just 200).
        let subject = fresh_subject();
        create_ehr_with_status(ctx, &ehr_status(true, true, false, &subject)).await?;
        let got = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!(
                    "/ehr?subject_id={subject}&subject_namespace={SUBJECT_NS}"
                )),
                Format::Json,
            ))
            .await?;
        assert::status(&got, 200)?;
        let ehr_id = ids::ehr_id(&got.json()?)?;
        let served = get_status(ctx, &ehr_id).await?;
        let served_subject = served
            .pointer("/subject/external_ref/id/value")
            .and_then(Value::as_str);
        if served_subject != Some(subject.as_str()) {
            return Err(CaseError::Assertion(format!(
                "EHR returned by subject query has subject {served_subject:?}, expected {subject:?}"
            )));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_ehr_invalid_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ get_expect_404(ctx, format!("/ehr/{}", Uuid::new_v4())).await })
}

fn run_get_ehr_invalid_subject_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        get_expect_404(
            ctx,
            format!(
                "/ehr?subject_id=nobody-{}&subject_namespace={SUBJECT_NS}",
                Uuid::new_v4()
            ),
        )
        .await
    })
}

// ── I_EHR_STATUS.get_ehr_status ──────────────────────────────────────────────

fn run_get_ehr_status_by_ehr_id<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // The served EHR_STATUS must match the create-time
        // rules (subject presence + is_queryable + is_modifiable), not just be
        // _type EHR_STATUS.
        let subject = fresh_subject();
        let ehr_id = create_ehr_with_status(ctx, &ehr_status(true, false, false, &subject)).await?;
        let status = get_status(ctx, &ehr_id).await?;
        if status["_type"] != "EHR_STATUS" {
            return Err(CaseError::Assertion(format!(
                "expected EHR_STATUS, got {}",
                status["_type"]
            )));
        }
        if status["is_queryable"] != json!(true) || status["is_modifiable"] != json!(false) {
            return Err(CaseError::Assertion(format!(
                "served flags (q={}, m={}) do not match create-time (q=true, m=false)",
                status["is_queryable"], status["is_modifiable"]
            )));
        }
        if status
            .pointer("/subject/external_ref/id/value")
            .and_then(Value::as_str)
            != Some(subject.as_str())
        {
            return Err(CaseError::Assertion(
                "served EHR_STATUS subject does not match the create-time subject".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_ehr_status_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ get_expect_404(ctx, format!("/ehr/{}/ehr_status", Uuid::new_v4())).await })
}

// ── I_EHR_STATUS.set/clear queryable/modifiable ──────────────────────────────

fn run_set_queryable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag(ctx, "is_queryable", true).await })
}
fn run_set_queryable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag_bad_ehr(ctx, "is_queryable", true).await })
}
fn run_set_modifiable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag(ctx, "is_modifiable", true).await })
}
fn run_set_modifiable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag_bad_ehr(ctx, "is_modifiable", true).await })
}
fn run_clear_queryable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag(ctx, "is_queryable", false).await })
}
fn run_clear_queryable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag_bad_ehr(ctx, "is_queryable", false).await })
}
fn run_clear_modifiable_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag(ctx, "is_modifiable", false).await })
}
fn run_clear_modifiable_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ update_flag_bad_ehr(ctx, "is_modifiable", false).await })
}

// ── §3 extensions ─────────────────────────────────────────────────────────────

fn run_create_ehr_invalid_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // master06 §Test Data Sets class 2: the vendored invalid EHR_STATUS
        // data sets are posted verbatim; each must be rejected (4xx). The one
        // structural exception is a fixture whose `subject` is an empty
        // PARTY_SELF (`subject: {}` — no identity): RM ehr master04 §EHR Status
        // + common §PARTY_SELF make that a completely anonymous, spec-VALID
        // subject, so it must be ACCEPTED (2xx). The exception is detected by
        // the fixture's shape (spec fact), not a filename.
        //
        // NOTE: ideally per-fixture expected-outcome would live
        // in the runner's adjudication register; that seam is not yet exposed
        // to the suites, so the single spec-valid-anonymous exception is
        // encoded here with its spec citation (replacing the legacy hardcoded
        // filename + TRIAGE cross-ref).
        let invalid = fixtures::ehr_invalid().map_err(|e| CaseError::Codec(e.to_string()))?;
        let total = u32::try_from(invalid.len()).unwrap_or(u32::MAX);
        let mut passed = 0u32;
        for fixture in invalid {
            let body = fixture
                .json()
                .map_err(|e| CaseError::Codec(e.to_string()))?;
            let expect_accept = is_anonymous_empty_subject(&body);
            let resp = ctx
                .send(negotiate::prefer(
                    HttpRequest::post("/ehr").json_body(&body)?,
                    negotiate::PreferReturn::Minimal,
                ))
                .await?;
            let ok = if expect_accept {
                (200..300).contains(&resp.status)
            } else {
                (400..500).contains(&resp.status)
            };
            if ok {
                passed += 1;
            } else {
                return Err(CaseError::Assertion(format!(
                    "invalid EHR_STATUS fixture {:?}: expected {}, got {}",
                    fixture.name,
                    if expect_accept {
                        "2xx (spec-valid empty PARTY_SELF)"
                    } else {
                        "4xx (rejected)"
                    },
                    resp.status
                )));
            }
        }
        Ok(DataSetReport::all(passed).of_schedule_rows(total))
    })
}

/// Whether an `EHR_STATUS` body carries an empty `PARTY_SELF` subject (present,
/// but with no identity beyond an optional `_type`) — a completely anonymous
/// subject, which RM ehr master04 §EHR Status + common §`PARTY_SELF` make
/// spec-valid.
fn is_anonymous_empty_subject(body: &Value) -> bool {
    body.get("subject")
        .and_then(Value::as_object)
        .is_some_and(|s| s.keys().all(|k| k == "_type"))
}

fn run_create_anonymous_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // POST /ehr with no body → the server creates the default EHR_STATUS
        // (master06 §Test Data Sets class 1.b): anonymous subject (no
        // external_ref) + server-defaulted is_modifiable = is_queryable = true.
        let ehr_id = support::create_ehr(ctx).await?;
        let status = get_status(ctx, &ehr_id).await?;
        if status["subject"].get("external_ref").is_some() {
            return Err(CaseError::Assertion(
                "anonymous EHR_STATUS subject unexpectedly carries an external_ref".to_owned(),
            ));
        }
        if status["is_modifiable"] != json!(true) || status["is_queryable"] != json!(true) {
            return Err(CaseError::Assertion(format!(
                "default EHR_STATUS should have is_modifiable = is_queryable = true, got m={}, q={}",
                status["is_modifiable"], status["is_queryable"]
            )));
        }
        Ok(DataSetReport::SINGLE)
    })
}
