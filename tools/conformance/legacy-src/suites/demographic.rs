//! DEMOGRAPHIC (PARTY) cases — our own ECC cases (reference:
//! `master10-func_tc_demographic.adoc`, design-time reading).
//!
//! The upstream CNF `master10-func_tc_demographic.adoc` chapter ships only
//! placeholder `aaaa`/`bbbb` headings (no concrete cases — "Test Environment:
//! TBD"), so these `DEM` cases are our own spec-grounded functional cases,
//! specified against the **ITS-REST demographic API**
//! (`/demographic/{person,agent,group,organisation,role}` plus `versioned_party`
//! and tags) realizing SM `I_DEMOGRAPHIC_SERVICE`
//! (`docs/specs/openehr/SM/...#_i_demographic_service_interface`) over the RM
//! Demographic IM.
//!
//! Party versioned-object contract (mirrors the EHR/composition group): `201`
//! create (+`ETag`/`Location`), `200` get, `200`/`204` update, `204` delete,
//! `404` absent, `4xx` on a wrong `If-Match`. A party has **no EHR scope** — the
//! endpoints are EHR-independent. The bodies mirror the proven
//! `service_demographic` fixtures (a PARTY has no template, so no OPT upload).

use serde_json::{Value, json};
use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::harness::{
    CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, Method, RunContext,
};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The demographic API citation shared by every case in this suite.
const CITATION: &str =
    "ITS-REST 1.0.3 DEMOGRAPHIC API; SM §I_DEMOGRAPHIC_SERVICE; RM 1.2.0 demographic";

/// The implemented DEMOGRAPHIC case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // PERSON — full versioned-object lifecycle + negatives.
        entry(
            "dem/person-create",
            "Demographic person create",
            run_person_create,
        ),
        entry("dem/person-get", "Demographic person get", run_person_get),
        entry(
            "dem/person-get-by-version",
            "Demographic person get by version",
            run_person_get_by_version,
        ),
        entry(
            "dem/person-update",
            "Demographic person update",
            run_person_update,
        ),
        entry(
            "dem/person-delete",
            "Demographic person delete",
            run_person_delete,
        ),
        entry(
            "dem/person-get-deleted",
            "Demographic person get deleted",
            run_person_get_deleted,
        ),
        entry(
            "dem/person-get-absent",
            "Demographic person get absent",
            run_person_get_absent,
        ),
        entry(
            "dem/person-update-bad-if-match",
            "Demographic person update bad if match",
            run_person_bad_if_match,
        ),
        // The other four PARTY kinds — create/get/delete.
        entry(
            "dem/agent-create",
            "Demographic agent create",
            run_agent_create,
        ),
        entry("dem/agent-get", "Demographic agent get", run_agent_get),
        entry(
            "dem/agent-delete",
            "Demographic agent delete",
            run_agent_delete,
        ),
        entry(
            "dem/group-create",
            "Demographic group create",
            run_group_create,
        ),
        entry("dem/group-get", "Demographic group get", run_group_get),
        entry(
            "dem/group-delete",
            "Demographic group delete",
            run_group_delete,
        ),
        entry(
            "dem/organisation-create",
            "Demographic organisation create",
            run_org_create,
        ),
        entry(
            "dem/organisation-get",
            "Demographic organisation get",
            run_org_get,
        ),
        entry(
            "dem/organisation-delete",
            "Demographic organisation delete",
            run_org_delete,
        ),
        entry(
            "dem/role-create",
            "Demographic role create",
            run_role_create,
        ),
        entry("dem/role-get", "Demographic role get", run_role_get),
        entry(
            "dem/role-delete",
            "Demographic role delete",
            run_role_delete,
        ),
        // Cross-cutting.
        entry(
            "dem/create-bad-body",
            "Demographic create bad body",
            run_create_bad_body,
        ),
        entry(
            "dem/versioned-party-get",
            "Demographic versioned party get",
            run_versioned_get,
        ),
        entry(
            "dem/versioned-party-revision-history",
            "Demographic versioned party revision history",
            run_versioned_history,
        ),
        entry(
            "dem/person-tags",
            "Demographic person tags",
            run_person_tags,
        ),
    ]
}

fn entry(id: &'static str, title: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Dem,
            capability: Capability::DemographicApi,
            profiles: &[Profile::Options],
            formats: &[Format::Json],
            citation: CITATION,
            compare: Compare::Superset,
            schedule_ref: None,
        },
        run,
    }
}

// ── PARTY bodies (mirror the proven service_demographic fixtures) ─────────────

/// An `ACTOR` subtype body (PERSON/AGENT/GROUP/ORGANISATION) with one mandatory
/// `PARTY_IDENTITY`.
fn actor(ty: &str, archetype: &str, name: &str) -> Value {
    json!({
        "_type": ty,
        "archetype_node_id": archetype,
        "name": { "_type": "DV_TEXT", "value": name },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": "legal name" },
            "details": {
                "_type": "ITEM_TREE",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" },
                "items": [{
                    "_type": "ELEMENT",
                    "archetype_node_id": "at0003",
                    "name": { "_type": "DV_TEXT", "value": "family" },
                    "value": { "_type": "DV_TEXT", "value": name }
                }]
            }
        }]
    })
}

/// A `ROLE` body (extends PARTY: identities + performer + capabilities).
fn role_body(name: &str) -> Value {
    json!({
        "_type": "ROLE",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-ROLE.role.v1",
        "name": { "_type": "DV_TEXT", "value": name },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": name },
            "details": {
                "_type": "ITEM_TREE",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" },
                "items": []
            }
        }],
        "performer": {
            "_type": "PARTY_REF",
            "namespace": "demographic",
            "type": "PERSON",
            "id": { "_type": "HIER_OBJECT_ID", "value": "cccccccc-cccc-4ccc-8ccc-cccccccccccc" }
        }
        // No `capabilities`: a PRESENT list must be non-empty
        // (ROLE.Capabilities_valid, role.adoc) — absence is the valid way to
        // carry "no capabilities".
    })
}

/// The body for a party kind path segment.
fn body_for(seg: &str) -> Value {
    match seg {
        "agent" => actor("AGENT", "openEHR-DEMOGRAPHIC-AGENT.agent.v1", "Demo Agent"),
        "group" => actor("GROUP", "openEHR-DEMOGRAPHIC-GROUP.group.v1", "Demo Group"),
        "organisation" => actor(
            "ORGANISATION",
            "openEHR-DEMOGRAPHIC-ORGANISATION.organisation.v1",
            "Demo Org",
        ),
        "role" => role_body("Demo Role"),
        _ => actor("PERSON", "openEHR-DEMOGRAPHIC-PERSON.person.v1", "Jane"),
    }
}

/// Create a party of `seg`, asserting `201` + `ETag`, and return
/// `(versioned_object_uid, object_version_id)`. The versioned-object uid is the
/// `HIER_OBJECT_ID` (the part of the OVID before `::`), used on the get/update/
/// delete path; the OVID is the `ETag`, used as `If-Match`.
async fn create(ctx: &RunContext<'_>, seg: &str) -> Result<(String, String), CaseError> {
    let resp = ctx
        .send(
            HttpRequest::post(format!("/demographic/{seg}"))
                .json_body(&body_for(seg))?
                .header("accept", "application/json")
                .header("prefer", "return=representation"),
        )
        .await?;
    assert::status(&resp, 201)?;
    let ovid = support::uid_of(&resp.json()?)?;
    let vo = ovid.split("::").next().unwrap_or(ovid.as_str()).to_owned();
    Ok((vo, ovid))
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── PERSON lifecycle ──────────────────────────────────────────────────────────

fn run_person_create<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::post("/demographic/person")
                    .json_body(&body_for("person"))?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation"),
            )
            .await?;
        assert::status(&resp, 201)?;
        assert::header_present(&resp, "etag")?;
        assert::header_present(&resp, "location")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, _) = create(ctx, "person").await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/demographic/person/{vo}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get_by_version<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (_, ovid) = create(ctx, "person").await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/demographic/person/{ovid}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_update<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, ovid) = create(ctx, "person").await?;
        let updated = actor("PERSON", "openEHR-DEMOGRAPHIC-PERSON.person.v1", "Jane Roe");
        let resp = ctx
            .send(
                HttpRequest::put(format!("/demographic/person/{vo}"))
                    .json_body(&updated)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation")
                    .header("if-match", ovid),
            )
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, ovid) = create(ctx, "person").await?;
        let resp = ctx
            .send(
                HttpRequest::new(Method::Delete, format!("/demographic/person/{vo}"))
                    .header("if-match", ovid),
            )
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get_deleted<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, ovid) = create(ctx, "person").await?;
        let _ = ctx
            .send(
                HttpRequest::new(Method::Delete, format!("/demographic/person/{vo}"))
                    .header("if-match", ovid),
            )
            .await?;
        // A deleted party's current version reads back as 204 (no content) or 404.
        let resp = ctx
            .send(
                HttpRequest::get(format!("/demographic/person/{vo}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&resp, &[204, 404])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get_absent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!("/demographic/person/{}", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_bad_if_match<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, _) = create(ctx, "person").await?;
        let updated = actor("PERSON", "openEHR-DEMOGRAPHIC-PERSON.person.v1", "Wrong");
        let resp = ctx
            .send(
                HttpRequest::put(format!("/demographic/person/{vo}"))
                    .json_body(&updated)?
                    .header("accept", "application/json")
                    .header("if-match", format!("{vo}::conformance::99")),
            )
            .await?;
        assert::status_in(&resp, &[400, 409, 412])?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── the other four kinds: create / get / delete (macro-generated) ─────────────

macro_rules! kind_crud {
    ($seg:literal, $create:ident, $get:ident, $del:ident) => {
        fn $create<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            case!({
                let resp = ctx
                    .send(
                        HttpRequest::post(concat!("/demographic/", $seg))
                            .json_body(&body_for($seg))?
                            .header("accept", "application/json")
                            .header("prefer", "return=representation"),
                    )
                    .await?;
                assert::status(&resp, 201)?;
                assert::header_present(&resp, "etag")?;
                Ok(DataSetReport::SINGLE)
            })
        }
        fn $get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            case!({
                let (vo, _) = create(ctx, $seg).await?;
                let resp = ctx
                    .send(
                        HttpRequest::get(format!(concat!("/demographic/", $seg, "/{}"), vo))
                            .header("accept", "application/json"),
                    )
                    .await?;
                assert::status(&resp, 200)?;
                Ok(DataSetReport::SINGLE)
            })
        }
        fn $del<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            case!({
                let (vo, ovid) = create(ctx, $seg).await?;
                let resp = ctx
                    .send(
                        HttpRequest::new(
                            Method::Delete,
                            format!(concat!("/demographic/", $seg, "/{}"), vo),
                        )
                        .header("if-match", ovid),
                    )
                    .await?;
                assert::status_in(&resp, &[200, 204])?;
                Ok(DataSetReport::SINGLE)
            })
        }
    };
}

kind_crud!("agent", run_agent_create, run_agent_get, run_agent_delete);
kind_crud!("group", run_group_create, run_group_get, run_group_delete);
kind_crud!("organisation", run_org_create, run_org_get, run_org_delete);
kind_crud!("role", run_role_create, run_role_get, run_role_delete);

// ── cross-cutting ─────────────────────────────────────────────────────────────

fn run_create_bad_body<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // A PERSON missing its mandatory `identities` [1..*] must be rejected.
        let bad = json!({
            "_type": "PERSON",
            "archetype_node_id": "openEHR-DEMOGRAPHIC-PERSON.person.v1",
            "name": { "_type": "DV_TEXT", "value": "no identities" }
        });
        let resp = ctx
            .send(
                HttpRequest::post("/demographic/person")
                    .json_body(&bad)?
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&resp, &[400, 422])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_versioned_get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, _) = create(ctx, "person").await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/demographic/versioned_party/{vo}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_versioned_history<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, _) = create(ctx, "person").await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/demographic/versioned_party/{vo}/revision_history"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_tags<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (vo, _) = create(ctx, "person").await?;
        // Reading the (empty) item-tag set for a fresh party is 200 (or 204).
        let resp = ctx
            .send(
                HttpRequest::get(format!("/demographic/person/{vo}/tags"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}
