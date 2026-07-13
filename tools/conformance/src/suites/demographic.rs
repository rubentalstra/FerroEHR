//! DEMOGRAPHIC (PARTY + PARTY_RELATIONSHIP) cases — the master10 spine
//! (`docs/design/conformance/08-demographic.md`).
//!
//! master10-func_tc_demographic.adoc ships **no concrete test case** (every one
//! of its 12 SM-operation subsections is a `TBD` `aaaa`/`bbbb` stub — register
//! 08 §2). So every case here is [`ScheduleTrace::EccOriginal`], stub-derived:
//! the honest provenance is the SM operation heading + the RM Demographic IM,
//! never a schedule-conformant claim (owner ruling 2026-07-13). The wire is the
//! ITS-REST DEVELOPMENT demographic API
//! (`crates/openehr-its/vendor/rest-oas/demographic-codegen.openapi.yaml`), an
//! OPTIONS-profile surface that for a foreign SUT the fairness register rules
//! `extension` (register 08 G-7) — its absence never dents CORE/STANDARD.
//!
//! Party bodies are authored inline at the RM 1.2.0 canonical shape.
//! Negative `If-Match` ids come from [`support::nonexistent_version_like`] over
//! an OBSERVED id — the `::conformance::99` literal the legacy suite baked in is
//! gone (register 08 G-1). The `I_PARTY_RELATIONSHIP` family (register 08 G-3)
//! and `get_party_at_time` (G-2) are added here; the relationship wire is an
//! ehrbase-rs extension (the ITS-REST demographic OAS declares **no**
//! `party_relationship` path — only the schema), so those cases are
//! EccOriginal against the extension route, never presented as ITS-REST-bound.
//
// PORT NOTE: register 08 G-4 (RM wire version ladder) is only partially met —
// party/relationship request payloads are authored at RM 1.2.0; a per-edition
// request-payload provider (RM 1.0.2 minimum, master03-overview §API
// Conformance) belongs to the register-90 wire adapter, not yet exposed. The
// DEMOGRAPHIC API is a DEVELOPMENT-only surface (no Release-1.0.3 rung), so the
// status ladders below are single-rung `[(Development, code)]`.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::edition::Edition;
use crate::engine::assert;
use crate::engine::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::wire::{ids, negotiate};

/// JSON is the wire format the DEMOGRAPHIC cases run under.
const JSON: &[Format] = &[Format::Json];

/// Single-rung ladders: the DEMOGRAPHIC API exists only in the DEVELOPMENT
/// edition (a DEVELOPMENT-status ITS-REST API), so there is no lower rung.
const CREATED: &[(Edition, u16)] = &[(Edition::Development, 201)];
const OK: &[(Edition, u16)] = &[(Edition::Development, 200)];
const ABSENT: &[(Edition, u16)] = &[(Edition::Development, 404)];

/// Every registered DEMOGRAPHIC case (24 carried + 7 new: G-2 + G-3).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── I_DEMOGRAPHIC_SERVICE.create_party ──────────────────────────────
        case(
            "dem/person-create",
            "Demographic person create",
            Capability::PartyOperations,
            Compare::Superset,
            "CNF master10 §create_party (TBD stub); ITS-REST DEVELOPMENT demographic person_create 201; SM I_DEMOGRAPHIC_SERVICE.create_party; RM 1.2.0 demographic §PERSON",
            stub("I_DEMOGRAPHIC_SERVICE.create_party"),
            Binding::Rest("POST /demographic/person"),
            run_person_create,
        ),
        case(
            "dem/create-bad-body",
            "Demographic create bad body",
            Capability::PartyOperations,
            Compare::None,
            "CNF master10 §create_party (TBD stub); ITS-REST DEVELOPMENT demographic person_create 400/422; SM I_DEMOGRAPHIC_SERVICE.create_party; RM 1.2.0 demographic §PERSON.identities [1..*]",
            stub("I_DEMOGRAPHIC_SERVICE.create_party"),
            Binding::Rest("POST /demographic/person"),
            run_create_bad_body,
        ),
        // ── I_DEMOGRAPHIC_SERVICE.get_party ─────────────────────────────────
        case(
            "dem/person-get",
            "Demographic person get",
            Capability::PartyOperations,
            Compare::Superset,
            "CNF master10 §get_party (TBD stub); ITS-REST DEVELOPMENT demographic person_get 200; SM I_DEMOGRAPHIC_SERVICE.get_party; RM 1.2.0 demographic §PERSON",
            stub("I_DEMOGRAPHIC_SERVICE.get_party"),
            Binding::Rest("GET /demographic/person/{uid_based_id}"),
            run_person_get,
        ),
        case(
            "dem/person-get-absent",
            "Demographic person get absent",
            Capability::PartyOperations,
            Compare::None,
            "CNF master10 §get_party (TBD stub); ITS-REST DEVELOPMENT demographic person_get 404; SM I_DEMOGRAPHIC_SERVICE.get_party",
            stub("I_DEMOGRAPHIC_SERVICE.get_party"),
            Binding::Rest("GET /demographic/person/{uid_based_id}"),
            run_person_get_absent,
        ),
        case(
            "dem/person-get-deleted",
            "Demographic person get deleted",
            Capability::PartyOperations,
            Compare::None,
            "CNF master10 §get_party + §delete_party (TBD stubs); ITS-REST DEVELOPMENT demographic person_get; SM I_DEMOGRAPHIC_SERVICE.get_party of a deleted party",
            stub("I_DEMOGRAPHIC_SERVICE.get_party"),
            Binding::Rest("GET /demographic/person/{uid_based_id}"),
            run_person_get_deleted,
        ),
        // ── I_DEMOGRAPHIC_SERVICE.get_party_at_version ──────────────────────
        case(
            "dem/person-get-by-version",
            "Demographic person get by version",
            Capability::PartyOperations,
            Compare::Superset,
            "CNF master10 §get_party_at_version (TBD stub); ITS-REST DEVELOPMENT demographic person_get (OVID) 200; SM I_DEMOGRAPHIC_SERVICE.get_party_at_version; RM common §OBJECT_VERSION_ID",
            stub("I_DEMOGRAPHIC_SERVICE.get_party_at_version"),
            Binding::Rest("GET /demographic/person/{version_uid}"),
            run_person_get_by_version,
        ),
        // ── I_DEMOGRAPHIC_SERVICE.get_party_at_time (G-2, new) ──────────────
        case(
            "dem/person-get-at-time",
            "Demographic person get at time",
            Capability::PartyOperations,
            Compare::Superset,
            "CNF master10 §get_party_at_time (TBD stub); ITS-REST DEVELOPMENT demographic person_get ?version_at_time 200; SM I_DEMOGRAPHIC_SERVICE.get_party_at_time; RM common §version_at_time",
            stub("I_DEMOGRAPHIC_SERVICE.get_party_at_time"),
            Binding::Rest("GET /demographic/person/{uid_based_id}?version_at_time"),
            run_person_get_at_time,
        ),
        // ── I_DEMOGRAPHIC_SERVICE.update_party ──────────────────────────────
        case(
            "dem/person-update",
            "Demographic person update",
            Capability::PartyOperations,
            Compare::None,
            "CNF master10 §update_party (TBD stub); ITS-REST DEVELOPMENT demographic person_update 200/204; SM I_DEMOGRAPHIC_SERVICE.update_party; RM common §Concurrency (If-Match OVID)",
            stub("I_DEMOGRAPHIC_SERVICE.update_party"),
            Binding::Rest("PUT /demographic/person/{uid_based_id}"),
            run_person_update,
        ),
        case(
            "dem/person-update-bad-if-match",
            "Demographic person update bad if match",
            Capability::PartyOperations,
            Compare::None,
            "CNF master10 §update_party (TBD stub); ITS-REST DEVELOPMENT demographic person_update 400/409/412; SM I_DEMOGRAPHIC_SERVICE.update_party (stale OVID); ITS-REST overview §Concurrency control",
            stub("I_DEMOGRAPHIC_SERVICE.update_party"),
            Binding::Rest("PUT /demographic/person/{uid_based_id}"),
            run_person_bad_if_match,
        ),
        // ── I_DEMOGRAPHIC_SERVICE.delete_party ──────────────────────────────
        case(
            "dem/person-delete",
            "Demographic person delete",
            Capability::PartyOperations,
            Compare::None,
            "CNF master10 §delete_party (TBD stub); ITS-REST DEVELOPMENT demographic person_delete 200/204; SM I_DEMOGRAPHIC_SERVICE.delete_party",
            stub("I_DEMOGRAPHIC_SERVICE.delete_party"),
            Binding::Rest("DELETE /demographic/person/{uid_based_id}"),
            run_person_delete,
        ),
        // ── the other four ACTOR kinds: create / get / delete ───────────────
        kind_create("dem/agent-create", "agent", "AGENT", run_agent_create),
        kind_get("dem/agent-get", "agent", "AGENT", run_agent_get),
        kind_delete("dem/agent-delete", "agent", "AGENT", run_agent_delete),
        kind_create("dem/group-create", "group", "GROUP", run_group_create),
        kind_get("dem/group-get", "group", "GROUP", run_group_get),
        kind_delete("dem/group-delete", "group", "GROUP", run_group_delete),
        kind_create(
            "dem/organisation-create",
            "organisation",
            "ORGANISATION",
            run_org_create,
        ),
        kind_get(
            "dem/organisation-get",
            "organisation",
            "ORGANISATION",
            run_org_get,
        ),
        kind_delete(
            "dem/organisation-delete",
            "organisation",
            "ORGANISATION",
            run_org_delete,
        ),
        kind_create("dem/role-create", "role", "ROLE", run_role_create),
        kind_get("dem/role-get", "role", "ROLE", run_role_get),
        kind_delete("dem/role-delete", "role", "ROLE", run_role_delete),
        // ── §3 wire extensions (no SM operation names them) ─────────────────
        case(
            "dem/versioned-party-get",
            "Demographic versioned party get",
            Capability::PartyOperations,
            Compare::None,
            "ITS-REST DEVELOPMENT demographic versioned_party_get 200; RM common §VERSIONED_OBJECT — no master10 SM operation names this wire resource",
            ScheduleTrace::EccOriginal(
                "extension: VERSIONED_PARTY read (RM common Versioning); no master10 SM operation",
            ),
            Binding::Rest("GET /demographic/versioned_party/{versioned_object_uid}"),
            run_versioned_get,
        ),
        case(
            "dem/versioned-party-revision-history",
            "Demographic versioned party revision history",
            Capability::PartyOperations,
            Compare::None,
            "ITS-REST DEVELOPMENT demographic versioned_party_revision_history 200; RM common §REVISION_HISTORY — no master10 SM operation names this wire resource",
            ScheduleTrace::EccOriginal(
                "extension: REVISION_HISTORY read (RM common Versioning); no master10 SM operation",
            ),
            Binding::Rest(
                "GET /demographic/versioned_party/{versioned_object_uid}/revision_history",
            ),
            run_versioned_history,
        ),
        case(
            "dem/person-tags",
            "Demographic person tags",
            Capability::PartyOperations,
            Compare::None,
            "ITS-REST DEVELOPMENT demographic person_tags_get 200/204 — item tags are an ehrbase-rs extension (no openEHR spec governs item tags)",
            ScheduleTrace::EccOriginal("extension: item tags — no openEHR spec governs item tags"),
            Binding::Rest("GET /demographic/person/{uid_based_id}/tags"),
            run_person_tags,
        ),
        // ── I_PARTY_RELATIONSHIP family (G-3, new; ehrbase-rs extension wire) ─
        case(
            "dem/relationship-create",
            "Demographic relationship create",
            Capability::PartyRelationshipOperations,
            Compare::Superset,
            "CNF master10 §create_party_relationship (TBD stub); SM I_PARTY_RELATIONSHIP.create_party_relationship; RM 1.2.0 demographic §PARTY_RELATIONSHIP — ehrbase-rs extension wire (no ITS-REST party_relationship path)",
            rel_stub("create_party_relationship"),
            Binding::Rest("POST /demographic/party_relationship (ehrbase-rs extension)"),
            run_rel_create,
        ),
        case(
            "dem/relationship-get",
            "Demographic relationship get",
            Capability::PartyRelationshipOperations,
            Compare::Superset,
            "CNF master10 §get_party_relationship (TBD stub); SM I_PARTY_RELATIONSHIP.get_party_relationship; RM 1.2.0 demographic §PARTY_RELATIONSHIP — ehrbase-rs extension wire",
            rel_stub("get_party_relationship"),
            Binding::Rest(
                "GET /demographic/party_relationship/{uid_based_id} (ehrbase-rs extension)",
            ),
            run_rel_get,
        ),
        case(
            "dem/relationship-get-at-time",
            "Demographic relationship get at time",
            Capability::PartyRelationshipOperations,
            Compare::Superset,
            "CNF master10 §get_party_relationship_at_time (TBD stub); SM I_PARTY_RELATIONSHIP.get_party_relationship_at_time; RM common §version_at_time — ehrbase-rs extension wire",
            rel_stub("get_party_relationship_at_time"),
            Binding::Rest(
                "GET /demographic/party_relationship/{uid_based_id}?version_at_time (ehrbase-rs extension)",
            ),
            run_rel_get_at_time,
        ),
        case(
            "dem/relationship-update",
            "Demographic relationship update",
            Capability::PartyRelationshipOperations,
            Compare::None,
            "CNF master10 §update_party_relationship (TBD stub); SM I_PARTY_RELATIONSHIP.update_party_relationship; ITS-REST overview §Concurrency (If-Match OVID) — ehrbase-rs extension wire",
            rel_stub("update_party_relationship"),
            Binding::Rest(
                "PUT /demographic/party_relationship/{uid_based_id} (ehrbase-rs extension)",
            ),
            run_rel_update,
        ),
        case(
            "dem/relationship-delete",
            "Demographic relationship delete",
            Capability::PartyRelationshipOperations,
            Compare::None,
            "CNF master10 §delete_party_relationship (TBD stub); SM I_PARTY_RELATIONSHIP.delete_party_relationship — ehrbase-rs extension wire",
            rel_stub("delete_party_relationship"),
            Binding::Rest(
                "DELETE /demographic/party_relationship/{uid_based_id} (ehrbase-rs extension)",
            ),
            run_rel_delete,
        ),
        case(
            "dem/relationship-get-by-version",
            "Demographic relationship get by version",
            Capability::PartyRelationshipOperations,
            Compare::Superset,
            "CNF master10 §get_party_relationship_at_version (TBD stub); SM I_PARTY_RELATIONSHIP.get_party_relationship_at_version; RM common §OBJECT_VERSION_ID — ehrbase-rs extension wire",
            rel_stub("get_party_relationship_at_version"),
            Binding::Rest(
                "GET /demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid} (ehrbase-rs extension)",
            ),
            run_rel_get_by_version,
        ),
    ]
}

/// A stub-derived schedule trace for an `I_DEMOGRAPHIC_SERVICE` operation
/// (master10 body is TBD — never presented as schedule-conformant).
fn stub(op: &'static str) -> ScheduleTrace {
    ScheduleTrace::EccOriginal(match op {
        "I_DEMOGRAPHIC_SERVICE.create_party" => {
            "schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM"
        }
        "I_DEMOGRAPHIC_SERVICE.get_party" => {
            "schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM"
        }
        "I_DEMOGRAPHIC_SERVICE.get_party_at_version" => {
            "schedule stub (master10 §get_party_at_version TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_version + RM common Versioning"
        }
        "I_DEMOGRAPHIC_SERVICE.get_party_at_time" => {
            "schedule stub (master10 §get_party_at_time TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_time + RM common version_at_time"
        }
        "I_DEMOGRAPHIC_SERVICE.update_party" => {
            "schedule stub (master10 §update_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.update_party + RM Demographic IM"
        }
        "I_DEMOGRAPHIC_SERVICE.delete_party" => {
            "schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM"
        }
        _ => {
            "schedule stub (master10 TBD); derived from SM I_DEMOGRAPHIC_SERVICE + RM Demographic IM"
        }
    })
}

/// A stub-derived schedule trace for an `I_PARTY_RELATIONSHIP` operation.
fn rel_stub(op: &'static str) -> ScheduleTrace {
    ScheduleTrace::EccOriginal(match op {
        "create_party_relationship" => {
            "schedule stub (master10 §create_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire"
        }
        "get_party_relationship" => {
            "schedule stub (master10 §get_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire"
        }
        "get_party_relationship_at_time" => {
            "schedule stub (master10 §get_party_relationship_at_time TBD); derived from SM I_PARTY_RELATIONSHIP + RM common version_at_time — ehrbase-rs extension wire"
        }
        "update_party_relationship" => {
            "schedule stub (master10 §update_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire"
        }
        "delete_party_relationship" => {
            "schedule stub (master10 §delete_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire"
        }
        _ => {
            "schedule stub (master10 §get_party_relationship_at_version TBD); derived from SM I_PARTY_RELATIONSHIP + RM common OBJECT_VERSION_ID — ehrbase-rs extension wire"
        }
    })
}

/// Assemble a DEMOGRAPHIC case entry (area [`Area::Dem`], JSON).
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
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Dem,
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

macro_rules! case_body {
    ($body:block) => {
        Box::pin(async move $body)
    };
}

// ── ACTOR-kind case builders (create / get / delete) ─────────────────────────

fn kind_create(
    id: &'static str,
    seg: &'static str,
    ty: &'static str,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    case(
        id,
        ty_title(ty, "create"),
        Capability::PartyOperations,
        Compare::Superset,
        kind_citation(seg, "create_party"),
        stub("I_DEMOGRAPHIC_SERVICE.create_party"),
        kind_binding("POST", seg),
        run,
    )
}
fn kind_get(
    id: &'static str,
    seg: &'static str,
    ty: &'static str,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    case(
        id,
        ty_title(ty, "get"),
        Capability::PartyOperations,
        Compare::Superset,
        kind_citation(seg, "get_party"),
        stub("I_DEMOGRAPHIC_SERVICE.get_party"),
        kind_binding("GET", seg),
        run,
    )
}
fn kind_delete(
    id: &'static str,
    seg: &'static str,
    ty: &'static str,
    run: crate::engine::harness::CaseRun,
) -> CaseEntry {
    case(
        id,
        ty_title(ty, "delete"),
        Capability::PartyOperations,
        Compare::None,
        kind_citation(seg, "delete_party"),
        stub("I_DEMOGRAPHIC_SERVICE.delete_party"),
        kind_binding("DELETE", seg),
        run,
    )
}

fn ty_title(ty: &'static str, op: &'static str) -> &'static str {
    match (ty, op) {
        ("AGENT", "create") => "Demographic agent create",
        ("AGENT", "get") => "Demographic agent get",
        ("AGENT", _) => "Demographic agent delete",
        ("GROUP", "create") => "Demographic group create",
        ("GROUP", "get") => "Demographic group get",
        ("GROUP", _) => "Demographic group delete",
        ("ORGANISATION", "create") => "Demographic organisation create",
        ("ORGANISATION", "get") => "Demographic organisation get",
        ("ORGANISATION", _) => "Demographic organisation delete",
        (_, "create") => "Demographic role create",
        (_, "get") => "Demographic role get",
        _ => "Demographic role delete",
    }
}

fn kind_citation(seg: &'static str, op: &'static str) -> &'static str {
    // Static citation per (seg, op) — the OAS operation is `<seg>_<verb>`.
    match (seg, op) {
        ("agent", "create_party") => {
            "CNF master10 §create_party (TBD stub); ITS-REST DEVELOPMENT demographic agent_create 201; SM I_DEMOGRAPHIC_SERVICE.create_party; RM 1.2.0 demographic §AGENT"
        }
        ("agent", "get_party") => {
            "CNF master10 §get_party (TBD stub); ITS-REST DEVELOPMENT demographic agent_get 200; SM I_DEMOGRAPHIC_SERVICE.get_party; RM 1.2.0 demographic §AGENT"
        }
        ("agent", _) => {
            "CNF master10 §delete_party (TBD stub); ITS-REST DEVELOPMENT demographic agent_delete 200/204; SM I_DEMOGRAPHIC_SERVICE.delete_party"
        }
        ("group", "create_party") => {
            "CNF master10 §create_party (TBD stub); ITS-REST DEVELOPMENT demographic group_create 201; SM I_DEMOGRAPHIC_SERVICE.create_party; RM 1.2.0 demographic §GROUP"
        }
        ("group", "get_party") => {
            "CNF master10 §get_party (TBD stub); ITS-REST DEVELOPMENT demographic group_get 200; SM I_DEMOGRAPHIC_SERVICE.get_party; RM 1.2.0 demographic §GROUP"
        }
        ("group", _) => {
            "CNF master10 §delete_party (TBD stub); ITS-REST DEVELOPMENT demographic group_delete 200/204; SM I_DEMOGRAPHIC_SERVICE.delete_party"
        }
        ("organisation", "create_party") => {
            "CNF master10 §create_party (TBD stub); ITS-REST DEVELOPMENT demographic organisation_create 201; SM I_DEMOGRAPHIC_SERVICE.create_party; RM 1.2.0 demographic §ORGANISATION"
        }
        ("organisation", "get_party") => {
            "CNF master10 §get_party (TBD stub); ITS-REST DEVELOPMENT demographic organisation_get 200; SM I_DEMOGRAPHIC_SERVICE.get_party; RM 1.2.0 demographic §ORGANISATION"
        }
        ("organisation", _) => {
            "CNF master10 §delete_party (TBD stub); ITS-REST DEVELOPMENT demographic organisation_delete 200/204; SM I_DEMOGRAPHIC_SERVICE.delete_party"
        }
        (_, "create_party") => {
            "CNF master10 §create_party (TBD stub); ITS-REST DEVELOPMENT demographic role_create 201; SM I_DEMOGRAPHIC_SERVICE.create_party; RM 1.2.0 demographic §ROLE (Capabilities_valid)"
        }
        (_, "get_party") => {
            "CNF master10 §get_party (TBD stub); ITS-REST DEVELOPMENT demographic role_get 200; SM I_DEMOGRAPHIC_SERVICE.get_party; RM 1.2.0 demographic §ROLE"
        }
        _ => {
            "CNF master10 §delete_party (TBD stub); ITS-REST DEVELOPMENT demographic role_delete 200/204; SM I_DEMOGRAPHIC_SERVICE.delete_party"
        }
    }
}

fn kind_binding(verb: &'static str, seg: &'static str) -> Binding {
    match (verb, seg) {
        ("POST", "agent") => Binding::Rest("POST /demographic/agent"),
        ("GET", "agent") => Binding::Rest("GET /demographic/agent/{uid_based_id}"),
        ("DELETE", "agent") => Binding::Rest("DELETE /demographic/agent/{uid_based_id}"),
        ("POST", "group") => Binding::Rest("POST /demographic/group"),
        ("GET", "group") => Binding::Rest("GET /demographic/group/{uid_based_id}"),
        ("DELETE", "group") => Binding::Rest("DELETE /demographic/group/{uid_based_id}"),
        ("POST", "organisation") => Binding::Rest("POST /demographic/organisation"),
        ("GET", "organisation") => Binding::Rest("GET /demographic/organisation/{uid_based_id}"),
        ("DELETE", "organisation") => {
            Binding::Rest("DELETE /demographic/organisation/{uid_based_id}")
        }
        ("POST", _) => Binding::Rest("POST /demographic/role"),
        ("GET", _) => Binding::Rest("GET /demographic/role/{uid_based_id}"),
        _ => Binding::Rest("DELETE /demographic/role/{uid_based_id}"),
    }
}

/// Generate the three named run fns for an ACTOR kind (a `CaseRun` is a bare
/// `fn` pointer, so each op needs its own named item — a closure would not
/// coerce to the higher-ranked pointer).
macro_rules! actor_kind {
    ($seg:literal, $create:ident, $get:ident, $del:ident) => {
        fn $create<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            case_body!({
                create_party(ctx, $seg, &fresh_name()).await?;
                Ok(DataSetReport::SINGLE)
            })
        }
        fn $get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            case_body!({ get_and_check(ctx, $seg).await })
        }
        fn $del<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            case_body!({ delete_party(ctx, $seg).await })
        }
    };
}

actor_kind!("agent", run_agent_create, run_agent_get, run_agent_delete);
actor_kind!("group", run_group_create, run_group_get, run_group_delete);
actor_kind!("organisation", run_org_create, run_org_get, run_org_delete);
actor_kind!("role", run_role_create, run_role_get, run_role_delete);

// ── PARTY bodies (RM 1.2.0 demographic) ──────────────────────────────────────

/// A unique family-name marker so a create→read round-trip is provable.
fn fresh_name() -> String {
    format!("conf-party-{}", Uuid::new_v4())
}

/// An `ACTOR` subtype body (PERSON/AGENT/GROUP/ORGANISATION) with one mandatory
/// `PARTY_IDENTITY` carrying `name` as its family value.
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

/// A `ROLE` body. RM 1.2.0 demographic §ROLE.Capabilities_valid: a *present*
/// capabilities list must be non-empty, so absence carries "no capabilities".
fn role_body(name: &str) -> Value {
    json!({
        "_type": "ROLE",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-ROLE.role.v1",
        "name": { "_type": "DV_TEXT", "value": name },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": name },
            "details": { "_type": "ITEM_TREE", "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" }, "items": [] }
        }],
        "performer": {
            "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
            "id": { "_type": "HIER_OBJECT_ID", "value": "cccccccc-cccc-4ccc-8ccc-cccccccccccc" }
        }
    })
}

/// The body for a party kind path segment, carrying `name` as its marker.
fn body_for(seg: &str, name: &str) -> Value {
    match seg {
        "agent" => actor("AGENT", "openEHR-DEMOGRAPHIC-AGENT.agent.v1", name),
        "group" => actor("GROUP", "openEHR-DEMOGRAPHIC-GROUP.group.v1", name),
        "organisation" => actor(
            "ORGANISATION",
            "openEHR-DEMOGRAPHIC-ORGANISATION.organisation.v1",
            name,
        ),
        "role" => role_body(name),
        _ => actor("PERSON", "openEHR-DEMOGRAPHIC-PERSON.person.v1", name),
    }
}

/// A `PARTY_RELATIONSHIP` between two party HIER_OBJECT_IDs (RM 1.2.0
/// demographic §PARTY_RELATIONSHIP: mandatory `source`/`target` PARTY_REF).
fn rel_body(name: &str, source_vo: &str, target_vo: &str) -> Value {
    json!({
        "_type": "PARTY_RELATIONSHIP",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-PARTY_RELATIONSHIP.relationship.v1",
        "name": { "_type": "DV_TEXT", "value": name },
        "source": { "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
                    "id": { "_type": "HIER_OBJECT_ID", "value": source_vo } },
        "target": { "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
                    "id": { "_type": "HIER_OBJECT_ID", "value": target_vo } }
    })
}

// ── wire helpers ─────────────────────────────────────────────────────────────

/// Create a party of `seg` (named `name`), asserting `201` + ETag + Location;
/// returns `(versioned_object_uid, object_version_id)`.
async fn create_party(
    ctx: &RunContext<'_>,
    seg: &str,
    name: &str,
) -> Result<(String, String), CaseError> {
    let req = negotiate::representation(
        HttpRequest::post(format!("/demographic/{seg}")).json_body(&body_for(seg, name))?,
        Format::Json,
    );
    let resp = ctx.send(req).await?;
    assert::status_ladder(ctx, &resp, CREATED, "create_party 201")?;
    assert::header_present(&resp, "etag")?;
    assert::header_present(&resp, "location")?;
    let ovid = ids::version_uid(ctx, &resp)?;
    let vo = ids::object_uid(&ovid).to_owned();
    Ok((vo, ovid))
}

/// GET a party of `seg` and assert its served family `name` round-trips
/// (register 08 G-6: identity, not merely `200`).
async fn get_and_check(ctx: &RunContext<'_>, seg: &str) -> Result<DataSetReport, CaseError> {
    let name = fresh_name();
    let (vo, _) = create_party(ctx, seg, &name).await?;
    let resp = ctx
        .send(negotiate::accept(
            HttpRequest::get(format!("/demographic/{seg}/{vo}")),
            Format::Json,
        ))
        .await?;
    assert::status_ladder(ctx, &resp, OK, "get_party 200")?;
    let served = resp.json()?;
    if served["name"]["value"].as_str() != Some(name.as_str()) {
        return Err(CaseError::Assertion(format!(
            "served party name {:?} does not match the created {name:?}",
            served["name"]["value"]
        )));
    }
    Ok(DataSetReport::SINGLE)
}

/// Create then delete a party of `seg` (matching `If-Match`), asserting the
/// delete is a `200`/`204` success.
async fn delete_party(ctx: &RunContext<'_>, seg: &str) -> Result<DataSetReport, CaseError> {
    let (vo, ovid) = create_party(ctx, seg, &fresh_name()).await?;
    let resp = ctx
        .send(negotiate::if_match(
            HttpRequest::delete(format!("/demographic/{seg}/{vo}")),
            &ovid,
        ))
        .await?;
    assert::status_in(&resp, &[200, 204])?;
    Ok(DataSetReport::SINGLE)
}

// ── PERSON runs ──────────────────────────────────────────────────────────────

fn run_person_create<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        create_party(ctx, "person", &fresh_name()).await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({ get_and_check(ctx, "person").await })
}

fn run_person_get_by_version<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (_, ovid) = create_party(ctx, "person", &fresh_name()).await?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/person/{ovid}")),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, OK, "get_party_at_version 200")?;
        // Register 08 G-6: the served version's uid must equal the requested OVID.
        if ids::body_uid(&resp.json()?)? != ovid {
            return Err(CaseError::Assertion(
                "served party version uid does not match the requested OVID".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get_at_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // get_party_at_time: read the version current at "now" (RM version_at_time).
        let (vo, _) = create_party(ctx, "person", &fresh_name()).await?;
        let at = "2999-01-01T00:00:00Z"; // a time after creation: the current version is live.
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/person/{vo}?version_at_time={at}")),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, OK, "get_party_at_time 200")?;
        if resp.json()?["uid"]["value"].as_str().is_none() {
            return Err(CaseError::Assertion(
                "party-at-time read carries no uid.value".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get_absent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/person/{}", Uuid::new_v4())),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, ABSENT, "get_party absent 404")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_get_deleted<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, ovid) = create_party(ctx, "person", &fresh_name()).await?;
        ctx.send(negotiate::if_match(
            HttpRequest::delete(format!("/demographic/person/{vo}")),
            &ovid,
        ))
        .await?;
        // instrument-encodes-server-behaviour (register 08 §2): a deleted party's
        // current-version read is 204 (no content) or 404 — the widened set is
        // documented, not a hidden acceptance.
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/person/{vo}")),
                Format::Json,
            ))
            .await?;
        assert::status_in(&resp, &[204, 404])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_update<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, ovid) = create_party(ctx, "person", &fresh_name()).await?;
        let updated = actor(
            "PERSON",
            "openEHR-DEMOGRAPHIC-PERSON.person.v1",
            &fresh_name(),
        );
        let put = negotiate::if_match(
            negotiate::representation(
                HttpRequest::put(format!("/demographic/person/{vo}")).json_body(&updated)?,
                Format::Json,
            ),
            &ovid,
        );
        let resp = ctx.send(put).await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_bad_if_match<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // Register 08 G-1: the stale If-Match is built from an OBSERVED OVID via
        // support::nonexistent_version_like — the `::conformance::99` literal is gone.
        let (vo, ovid) = create_party(ctx, "person", &fresh_name()).await?;
        let bogus = support::nonexistent_version_like(&ids::parse_object_version_id(&ovid)?);
        let updated = actor(
            "PERSON",
            "openEHR-DEMOGRAPHIC-PERSON.person.v1",
            &fresh_name(),
        );
        let put = negotiate::if_match(
            negotiate::representation(
                HttpRequest::put(format!("/demographic/person/{vo}")).json_body(&updated)?,
                Format::Json,
            ),
            &bogus,
        );
        let resp = ctx.send(put).await?;
        // A stale/unknown OVID is a spec-valid negative at 400/409/412 (ITS-REST
        // overview §Concurrency control) — a ladder-safe any-4xx assertion.
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({ delete_party(ctx, "person").await })
}

fn run_create_bad_body<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // A PERSON missing its mandatory `identities [1..*]` must be rejected
        // (RM 1.2.0 demographic §PERSON.identities cardinality).
        let bad = json!({
            "_type": "PERSON",
            "archetype_node_id": "openEHR-DEMOGRAPHIC-PERSON.person.v1",
            "name": { "_type": "DV_TEXT", "value": "no identities" }
        });
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::post("/demographic/person").json_body(&bad)?,
                Format::Json,
            ))
            .await?;
        assert::status_in(&resp, &[400, 422])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_versioned_get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, _) = create_party(ctx, "person", &fresh_name()).await?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/versioned_party/{vo}")),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, OK, "versioned_party_get 200")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_versioned_history<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, _) = create_party(ctx, "person", &fresh_name()).await?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!(
                    "/demographic/versioned_party/{vo}/revision_history"
                )),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, OK, "versioned_party revision_history 200")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_person_tags<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, _) = create_party(ctx, "person", &fresh_name()).await?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/person/{vo}/tags")),
                Format::Json,
            ))
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── PARTY_RELATIONSHIP runs (ehrbase-rs extension wire) ──────────────────────

/// Create two persons and a relationship between them; returns
/// `(relationship_vo, relationship_ovid)`.
async fn create_relationship(
    ctx: &RunContext<'_>,
    name: &str,
) -> Result<(String, String), CaseError> {
    let (src, _) = create_party(ctx, "person", &fresh_name()).await?;
    let (tgt, _) = create_party(ctx, "person", &fresh_name()).await?;
    let req = negotiate::representation(
        HttpRequest::post("/demographic/party_relationship")
            .json_body(&rel_body(name, &src, &tgt))?,
        Format::Json,
    );
    let resp = ctx.send(req).await?;
    assert::status_ladder(ctx, &resp, CREATED, "create_party_relationship 201")?;
    assert::header_present(&resp, "etag")?;
    let ovid = ids::version_uid(ctx, &resp)?;
    let vo = ids::object_uid(&ovid).to_owned();
    Ok((vo, ovid))
}

fn run_rel_create<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        create_relationship(ctx, &fresh_name()).await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_rel_get<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let name = fresh_name();
        let (vo, _) = create_relationship(ctx, &name).await?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/party_relationship/{vo}")),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, OK, "get_party_relationship 200")?;
        if resp.json()?["name"]["value"].as_str() != Some(name.as_str()) {
            return Err(CaseError::Assertion(
                "served relationship name does not round-trip".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_rel_get_at_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, _) = create_relationship(ctx, &fresh_name()).await?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!(
                    "/demographic/party_relationship/{vo}?version_at_time=2999-01-01T00:00:00Z"
                )),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, OK, "get_party_relationship_at_time 200")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_rel_update<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, ovid) = create_relationship(ctx, &fresh_name()).await?;
        // Re-read to author a well-formed modification carrying its source/target.
        let current = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/demographic/party_relationship/{vo}")),
                Format::Json,
            ))
            .await?
            .json()?;
        let mut updated = current.clone();
        updated["name"] = json!({ "_type": "DV_TEXT", "value": fresh_name() });
        let put = negotiate::if_match(
            negotiate::representation(
                HttpRequest::put(format!("/demographic/party_relationship/{vo}"))
                    .json_body(&updated)?,
                Format::Json,
            ),
            &ovid,
        );
        let resp = ctx.send(put).await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_rel_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, ovid) = create_relationship(ctx, &fresh_name()).await?;
        let resp = ctx
            .send(negotiate::if_match(
                HttpRequest::delete(format!("/demographic/party_relationship/{vo}")),
                &ovid,
            ))
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_rel_get_by_version<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (vo, ovid) = create_relationship(ctx, &fresh_name()).await?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!(
                    "/demographic/versioned_party_relationship/{vo}/version/{ovid}"
                )),
                Format::Json,
            ))
            .await?;
        assert::status_ladder(ctx, &resp, OK, "get_party_relationship_at_version 200")?;
        Ok(DataSetReport::SINGLE)
    })
}
