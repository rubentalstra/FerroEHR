//! COMPOSITION cases — the master07 spine
//! (`docs/design/conformance/04-composition.md`).
//!
//! Every case concretizes a `master07-func_tc_ehr_composition.adoc` test case
//! (its [`ScheduleTrace`] carries the `<I_EHR_COMPOSITION.op-case>` form) over
//! the ITS-REST `/ehr/{ehr_id}/composition` + `/versioned_composition`
//! surface. The suite is authored from the schedule postconditions + the
//! vendored ITS-REST contract, NOT from observed server behaviour:
//!
//! - **Content check (G-1).** master07 attaches "the retrieved format should
//!   contain all the exact same data as the format used when committing" to
//!   every `get_*` case. Realized as retrieved ⊇ committed
//!   ([`Compare::Superset`]) — the server additionally assigns `uid` +
//!   committal metadata, so "contain all the same data" is a superset test,
//!   not exact equality. Applied over a JSON read (works for both JSON and XML
//!   runs).
//! - **Versioning postconditions (G-2).** `update` asserts the audit
//!   `change_type` CREATE→MODIFY (TERM SupportTerminology audit_change_type:
//!   249 creation, 251 modification); `delete` asserts the logical-delete
//!   `VERSION.lifecycle_state = openehr::523|deleted|` (master07 §delete NOTE)
//!   plus the ITS-REST 204/404 observable.
//!
//! Wire ids come ONLY from [`crate::wire`]; negative ids are built from an
//! OBSERVED id via [`support::nonexistent_version_like`] — never a
//! `::system::` literal (register 04 G-3). The SM `has_composition` boolean is
//! realized via `GET /composition/{uid}` (200 = TRUE) per the CNF guide's
//! abstract-call → REST mapping.
//
// PORT NOTE: register 04 G-6 (RM wire version ladder) is only partially met —
// positive bodies are the vendored RM-1.2.0-canonical fixtures; a per-edition
// COMPOSITION payload provider (RM 1.0.2 minimum, master03-overview §API
// Conformance) belongs to the register-90 wire adapter and is not yet exposed.
// Our pinned CI runs the development edition, so this is exercised faithfully.

use std::time::Duration;

use jiff::Timestamp;
use serde_json::Value;
use uuid::Uuid;

use crate::engine::assert;
use crate::engine::harness::{
    CaseError, CaseFuture, DataSetReport, HttpRequest, HttpResponse, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::testdata::fixtures;
use crate::wire::{ids, negotiate};

/// JSON-only formats.
const JSON: &[Format] = &[Format::Json];
/// Both canonical formats (the round-trip cases run under each).
const BOTH: &[Format] = &[Format::Json, Format::Xml];

/// The manifest dir key for the canonical-JSON compositions.
const JSON_DIR: &str = "composition.canonical-json";
/// The manifest dir key for the canonical-XML compositions.
const XML_DIR: &str = "composition.canonical-xml";
/// The manifest dir key for the valid OPTs.
const OPT_DIR: &str = "template.valid";

/// A shared citation stem for the COMPOSITION API + RM COMPOSITION type.
const CIT: &str = "ITS-REST 1.0.3 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control";

/// Every registered COMPOSITION case (31 carried + 1 new positive `has_composition`).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── create ─────────────────────────────────────────────────────────
        case(
            "com/create-composition-event",
            "Create composition — event",
            Capability::CompositionOps,
            BOTH,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.create_composition-event (master07 §create_composition)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition"),
            run_create_event,
        ),
        case(
            "com/create-composition-persistent",
            "Create composition — persistent",
            Capability::CompositionOps,
            BOTH,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.create_composition-persistent (master07 §create_composition)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition"),
            run_create_persistent,
        ),
        case(
            "com/create-composition-same-opt-twice",
            "Create composition — same OPT twice",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.create_composition-same_opt_twice (master07 §create_composition)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition"),
            run_create_same_opt_twice,
        ),
        case(
            "com/create-composition-invalid-event",
            "Create composition — invalid event",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.create_composition-invalid_event (master07 §create_composition)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition"),
            run_create_invalid_event,
        ),
        case(
            "com/create-composition-invalid-persistent",
            "Create composition — invalid persistent",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.create_composition-invalid_persistent (master07 §create_composition)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition"),
            run_create_invalid_persistent,
        ),
        case(
            "com/create-composition-event-bad-opt",
            "Create composition — event bad OPT",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.create_composition-event_bad_opt (master07 §create_composition)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition"),
            run_create_event_bad_opt,
        ),
        case(
            "com/create-composition-event-bad-ehr",
            "Create composition — event bad EHR",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.create_composition-event_bad_ehr (master07 §create_composition)",
            ),
            Binding::Rest("POST /ehr/{ehr_id}/composition"),
            run_create_event_bad_ehr,
        ),
        // ── has_composition ──────────────────────────────────────────────────
        case(
            "com/has-composition",
            "Composition existence check — existing composition",
            Capability::CompositionOps,
            JSON,
            Compare::Superset,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.has_composition (master07 §has_composition)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_has_composition,
        ),
        case(
            "com/has-composition-bad-composition",
            "Composition existence check — bad composition",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.has_composition-bad_composition (master07 §has_composition)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_has_composition_bad_composition,
        ),
        case(
            "com/has-composition-bad-ehr",
            "Composition existence check — bad EHR",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.has_composition-bad_ehr (master07 §has_composition)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_has_composition_bad_ehr,
        ),
        // ── get latest ─────────────────────────────────────────────────────
        case(
            "com/get-composition-latest",
            "Get latest composition",
            Capability::CompositionOps,
            BOTH,
            Compare::Superset,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_latest (master07 §get_composition_latest)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_get_latest,
        ),
        case(
            "com/get-composition-latest-bad-composition",
            "Get latest composition — bad composition",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_latest-bad_composition (master07 §get_composition_latest)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_get_latest_bad_composition,
        ),
        case(
            "com/get-composition-latest-bad-ehr",
            "Get latest composition — bad EHR",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_latest-bad_ehr (master07 §get_composition_latest)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_get_latest_bad_ehr,
        ),
        // ── get at time ──────────────────────────────────────────────────────
        case(
            "com/get-composition-at-time",
            "Get composition at time",
            Capability::CompositionOps,
            BOTH,
            Compare::Superset,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_at_time (master07 §get_composition_at_time)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}?version_at_time"),
            run_get_at_time,
        ),
        case(
            "com/get-composition-at-time-no-time-arg",
            "Get composition at time — no time arg",
            Capability::CompositionOps,
            BOTH,
            Compare::Superset,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_at_time-no_time_arg (master07 §get_composition_at_time)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_get_at_time_no_arg,
        ),
        case(
            "com/get-composition-at-time-bad-composition",
            "Get composition at time — bad composition",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_at_time-bad_composition (master07 §get_composition_at_time)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}?version_at_time"),
            run_get_at_time_bad_composition,
        ),
        case(
            "com/get-composition-at-time-bad-ehr",
            "Get composition at time — bad EHR",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_at_time-bad_ehr (master07 §get_composition_at_time)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}?version_at_time"),
            run_get_at_time_bad_ehr,
        ),
        case(
            "com/get-composition-at-times",
            "Get composition at multiple times",
            Capability::CompositionOps,
            JSON,
            Compare::Superset,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_at_times (master07 §get_composition_at_time)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{uid_based_id}?version_at_time"),
            run_get_at_times,
        ),
        // ── get version ──────────────────────────────────────────────────────
        case(
            "com/get-composition-version",
            "Get composition version",
            Capability::CompositionOps,
            BOTH,
            Compare::Superset,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_version (master07 §get_composition_version)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{version_uid}"),
            run_get_version,
        ),
        case(
            "com/get-composition-version-bad-version",
            "Get composition version — bad version",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_version-bad_version (master07 §get_composition_version)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{version_uid}"),
            run_get_version_bad_version,
        ),
        case(
            "com/get-composition-version-bad-ehr",
            "Get composition version — bad EHR",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_version-bad_ehr (master07 §get_composition_version)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{version_uid}"),
            run_get_version_bad_ehr,
        ),
        case(
            "com/get-composition-versions",
            "Get composition versions",
            Capability::CompositionOps,
            JSON,
            Compare::Superset,
            CIT,
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_composition_versions (master07 §get_composition_version)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/composition/{version_uid}"),
            run_get_versions,
        ),
        // ── versioned composition (capability Versioning) ────────────────────
        case(
            "com/get-versioned-composition",
            "Get versioned composition",
            Capability::Versioning,
            BOTH,
            Compare::Superset,
            "ITS-REST 1.0.3 COMPOSITION API versioned_composition_get.yaml 200; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT (F-05-06 version-family XML)",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_versioned_composition (master07 §get_versioned_composition)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}"),
            run_get_versioned,
        ),
        case(
            "com/get-versioned-composition-non-existent",
            "Get versioned composition — non existent",
            Capability::Versioning,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API versioned_composition_get.yaml 404; RM 1.2.0 common §VERSIONED_OBJECT",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_versioned_composition-non_existent (master07 §get_versioned_composition)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}"),
            run_get_versioned_non_existent,
        ),
        case(
            "com/get-versioned-composition-bad-ehr",
            "Get versioned composition — bad EHR",
            Capability::Versioning,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API versioned_composition_get.yaml 404; RM 1.2.0 common §VERSIONED_OBJECT",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.get_versioned_composition-bad_ehr (master07 §get_versioned_composition)",
            ),
            Binding::Rest("GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}"),
            run_get_versioned_bad_ehr,
        ),
        // ── update ─────────────────────────────────────────────────────────
        case(
            "com/update-composition-event",
            "Update composition — event",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API composition_update.yaml 200; RM common §Version tree; TERM SupportTerminology audit_change_type 249 creation / 251 modification",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.update_composition-event (master07 §update_composition)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_update_event,
        ),
        case(
            "com/update-composition-persistent",
            "Update composition — persistent",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API composition_update.yaml 200; RM common §Version tree; TERM SupportTerminology audit_change_type 249 creation / 251 modification",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.update_composition-persistent (master07 §update_composition)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_update_persistent,
        ),
        case(
            "com/update-composition-non-existent",
            "Update composition — non existent",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API composition_update.yaml 400/404/412/422 (non-existent preceding version)",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.update_composition-non_existent (master07 §update_composition)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_update_non_existent,
        ),
        case(
            "com/update-composition-wrong-template",
            "Update composition — wrong template",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API composition_update.yaml 422 (template_id mismatch); RM 1.2.0 ehr §COMPOSITION",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.update_composition-wrong_template (master07 §update_composition)",
            ),
            Binding::Rest("PUT /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_update_wrong_template,
        ),
        // ── delete ─────────────────────────────────────────────────────────
        case(
            "com/delete-composition-event",
            "Delete composition — event",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API composition_delete.yaml 204 + composition_get.yaml 204_because_deleted/404; master07 §delete_composition (logical delete: VERSION.lifecycle_state = openehr::523|deleted|)",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.delete_composition-event (master07 §delete_composition)",
            ),
            Binding::Rest("DELETE /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_delete_event,
        ),
        case(
            "com/delete-composition-persistent",
            "Delete composition — persistent",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API composition_delete.yaml 204 + composition_get.yaml 204_because_deleted/404; master07 §delete_composition (logical delete: VERSION.lifecycle_state = openehr::523|deleted|)",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.delete_composition-persistent (master07 §delete_composition)",
            ),
            Binding::Rest("DELETE /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_delete_persistent,
        ),
        case(
            "com/delete-composition-non-existent",
            "Delete composition — non existent",
            Capability::CompositionOps,
            JSON,
            Compare::None,
            "ITS-REST 1.0.3 COMPOSITION API composition_delete.yaml 400/404/409/412 (non-existent COMPOSITION)",
            ScheduleTrace::Schedule(
                "I_EHR_COMPOSITION.delete_composition-non_existent (master07 §delete_composition)",
            ),
            Binding::Rest("DELETE /ehr/{ehr_id}/composition/{uid_based_id}"),
            run_delete_non_existent,
        ),
    ]
}

/// Assemble a COMPOSITION-area case entry.
fn case(
    id: &'static str,
    title: &'static str,
    capability: Capability,
    formats: &'static [Format],
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
            area: Area::Com,
            capability,
            formats,
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

// ── fixtures + helpers ───────────────────────────────────────────────────────

/// A COMPOSITION category, selecting the vendored fixture set + its OPT
/// (master07 preconditions: the event OPT `nested.en.v1`, the persistent OPT
/// `persistent_minimal.en.v1`).
#[derive(Clone, Copy)]
enum Kind {
    /// An event `COMPOSITION` (`nested.en.v1`).
    Event,
    /// A persistent `COMPOSITION` (`persistent_minimal.en.v1`).
    Persistent,
}

impl Kind {
    /// The OPT file (relative to the `template.valid` dir key).
    fn opt_file(self) -> &'static str {
        match self {
            Kind::Event => "nested/nested.opt",
            Kind::Persistent => "minimal_persistent/persistent_minimal.opt",
        }
    }

    /// The canonical-JSON `__full` composition fixture (relative to `JSON_DIR`).
    fn json_file(self) -> &'static str {
        match self {
            Kind::Event => "nested.en.v1__full.json",
            Kind::Persistent => "persistent_minimal.en.v1__full.json",
        }
    }

    /// The canonical-XML `__full` composition fixture (relative to `XML_DIR`).
    fn xml_file(self) -> &'static str {
        match self {
            Kind::Event => "nested.en.v1__full.xml",
            Kind::Persistent => "persistent_minimal.en.v1__full.xml",
        }
    }

    /// The `__invalid_wrong_structure` (malformed) fixture (relative to `JSON_DIR`).
    fn invalid_structure_file(self) -> &'static str {
        match self {
            Kind::Event => "nested.en.v1__invalid_wrong_structure.json",
            Kind::Persistent => "persistent_minimal.en.v1__invalid_wrong_structure.json",
        }
    }
}

/// The `__invalid_opt_doesnt_exist` fixture (references a template never
/// uploaded) — event kind only (relative to `JSON_DIR`).
const BAD_OPT_FILE: &str = "nested.en.v1__invalid_opt_doesnt_exist.json";

fn codec(e: fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Read a canonical-JSON composition fixture as a [`Value`].
fn read_json_file(file: &str) -> Result<Value, CaseError> {
    let text = fixtures::read_from(JSON_DIR, file).map_err(codec)?;
    serde_json::from_str(&text).map_err(|e| CaseError::Codec(e.to_string()))
}

/// Provision the OPT for `kind` (tolerant of a re-upload on the shared SUT).
async fn ensure_opt(ctx: &RunContext<'_>, kind: Kind) -> Result<(), CaseError> {
    support::ensure_opt(ctx, OPT_DIR, kind.opt_file()).await
}

/// Commit a canonical composition for `kind` in the run's format (return the
/// raw response for the case to assert).
async fn commit(ctx: &RunContext<'_>, ehr_id: &str, kind: Kind) -> Result<HttpResponse, CaseError> {
    let path = format!("/ehr/{ehr_id}/composition");
    let req = match ctx.format {
        Format::Json => negotiate::representation(
            HttpRequest::post(path).json_body(&read_json_file(kind.json_file())?)?,
            Format::Json,
        ),
        Format::Xml => {
            let xml = fixtures::read_from(XML_DIR, kind.xml_file()).map_err(codec)?;
            negotiate::representation(
                HttpRequest::post(path).text_body(xml, "application/xml"),
                Format::Xml,
            )
        }
    };
    ctx.send(req).await
}

/// Create an EHR, provision the OPT, commit a `kind` composition (201), and
/// return `(ehr_id, version_uid)`.
async fn setup(ctx: &RunContext<'_>, kind: Kind) -> Result<(String, String), CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    ensure_opt(ctx, kind).await?;
    let resp = commit(ctx, &ehr_id, kind).await?;
    assert::status(&resp, 201)?;
    let uid = ids::version_uid(ctx, &resp)?;
    Ok((ehr_id, uid))
}

/// PUT an updated `kind` composition (JSON body) against `object_uid` under
/// `If-Match: precede`, returning the raw response.
async fn update(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    object_uid: &str,
    precede: &str,
    kind: Kind,
) -> Result<HttpResponse, CaseError> {
    let body = read_json_file(kind.json_file())?;
    let req = negotiate::if_match(
        negotiate::representation(
            HttpRequest::put(format!("/ehr/{ehr_id}/composition/{object_uid}")).json_body(&body)?,
            Format::Json,
        ),
        precede,
    );
    ctx.send(req).await
}

/// Create + update `kind` → two versions; return `(ehr_id, object_uid, uid1, uid2)`.
async fn setup_two(
    ctx: &RunContext<'_>,
    kind: Kind,
) -> Result<(String, String, String, String), CaseError> {
    let (ehr_id, uid1) = setup(ctx, kind).await?;
    let object = ids::object_uid(&uid1).to_owned();
    let resp = update(ctx, &ehr_id, &object, &uid1, kind).await?;
    assert::status_in(&resp, &[200, 204])?;
    let uid2 = ids::version_uid(ctx, &resp)?;
    Ok((ehr_id, object, uid1, uid2))
}

/// Assert an `OBJECT_VERSION_ID`'s version-tree id is `n` (RM common
/// §Version tree: version numbers increment from 1). Parses via
/// [`crate::wire::ids`] — no ad-hoc `::`-suffix scraping.
fn assert_version_number(uid: &str, n: u32) -> Result<(), CaseError> {
    let got = ids::parse_object_version_id(uid)?.version_tree_id;
    if got == n.to_string() {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "expected version-tree id {n} in {uid:?}, got {got:?}"
        )))
    }
}

/// The master07 content check (G-1): the retrieved composition must CONTAIN
/// all the same data as the committed fixture. Realized as retrieved ⊇
/// committed ([`Compare::Superset`]) — the server assigns `uid` + committal
/// metadata, so "contain all the same data" is a superset test.
fn content_check(kind: Kind, retrieved: &Value) -> Result<(), CaseError> {
    let committed = read_json_file(kind.json_file())?;
    support::assert_round_trip(Compare::Superset, &committed, retrieved)
}

/// GET a composition-read `url` in the run's format: assert 200 + non-empty
/// body, the version-tree number where `expected_version` is given, and the
/// content check (over a JSON read, so an XML run is content-checked via a
/// JSON re-read of the same resource — register 04 G-1).
async fn get_and_check(
    ctx: &RunContext<'_>,
    url: String,
    kind: Kind,
    expected_version: Option<u32>,
) -> Result<(), CaseError> {
    let resp = ctx
        .send(negotiate::accept(HttpRequest::get(url.clone()), ctx.format))
        .await?;
    assert::status(&resp, 200)?;
    if resp.body.is_empty() {
        return Err(CaseError::Assertion(
            "retrieved composition body is empty".to_owned(),
        ));
    }
    if let Some(n) = expected_version {
        assert_version_number(&ids::version_uid(ctx, &resp)?, n)?;
    }
    let retrieved = match ctx.format {
        Format::Json => resp.json()?,
        Format::Xml => {
            // JSON comparison is impossible against an XML body; re-read the
            // same resource as JSON for the content check.
            let j = ctx
                .send(negotiate::accept(HttpRequest::get(url), Format::Json))
                .await?;
            assert::status(&j, 200)?;
            j.json()?
        }
    };
    content_check(kind, &retrieved)
}

/// GET `path` (JSON) and assert `404` — the absent-resource negative
/// (composition_get.yaml / versioned_composition_get.yaml 404).
async fn get_expect_404(ctx: &RunContext<'_>, path: String) -> Result<DataSetReport, CaseError> {
    let resp = ctx
        .send(negotiate::accept(HttpRequest::get(path), Format::Json))
        .await?;
    assert::status(&resp, 404)?;
    Ok(DataSetReport::SINGLE)
}

/// Commit a throwaway event composition to OBSERVE a real `OBJECT_VERSION_ID`
/// (the SUT's own creating-system id + version-tree id), returning
/// `(ehr_id, observed)` — the seed for [`support::nonexistent_version_like`].
async fn observe_ovid(ctx: &RunContext<'_>) -> Result<(String, ids::ObjectVersionId), CaseError> {
    let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
    Ok((ehr_id, ids::parse_object_version_id(&uid)?))
}

/// GET the ORIGINAL_VERSION for `version_uid` (JSON), asserting 200.
async fn original_version(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    object: &str,
    version_uid: &str,
) -> Result<Value, CaseError> {
    let resp = ctx
        .send(negotiate::accept(
            HttpRequest::get(format!(
                "/ehr/{ehr_id}/versioned_composition/{object}/version/{version_uid}"
            )),
            Format::Json,
        ))
        .await?;
    assert::status(&resp, 200)?;
    resp.json()
}

/// The `defining_code.code_string` of a DV_CODED_TEXT node, if present.
fn coded_code(node: &Value) -> Option<&str> {
    node.get("defining_code")
        .and_then(|c| c.get("code_string"))
        .and_then(Value::as_str)
}

/// Assert an ORIGINAL_VERSION's `commit_audit.change_type` is the given
/// openEHR audit_change_type (matched by code OR rubric — the same coded value
/// in two representations; TERM SupportTerminology §audit_change_type).
fn assert_change_type(ov: &Value, code: &str, rubric: &str) -> Result<(), CaseError> {
    let ct = &ov["commit_audit"]["change_type"];
    let by_code = coded_code(ct) == Some(code);
    let by_rubric = ct.get("value").and_then(Value::as_str) == Some(rubric);
    if by_code || by_rubric {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "expected commit_audit.change_type openehr::{code}|{rubric}|, got {ct}"
        )))
    }
}

// ── create ───────────────────────────────────────────────────────────────────

fn run_create_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_create(ctx, Kind::Event).await })
}
fn run_create_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_create(ctx, Kind::Persistent).await })
}

/// Create a new `kind` composition: 201 + ETag/Location; version-tree number 1
/// (composition_create.yaml 201; RM common §Version tree).
async fn run_create(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    ensure_opt(ctx, kind).await?;
    let resp = commit(ctx, &ehr_id, kind).await?;
    assert::status(&resp, 201)?;
    assert::header_present(&resp, "etag")?;
    assert::header_present(&resp, "location")?;
    assert_version_number(&ids::version_uid(ctx, &resp)?, 1)?;
    Ok(DataSetReport::SINGLE)
}

fn run_create_same_opt_twice<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Only one 'create' is allowed for a persistent COMPOSITION; the second
        // create for the same persistent OPT is a negative response
        // (master07 §create_composition-same_opt_twice; the schedule §Notes flags
        // this as under debate in the openEHR SEC).
        let ehr_id = support::create_ehr(ctx).await?;
        ensure_opt(ctx, Kind::Persistent).await?;
        let first = commit(ctx, &ehr_id, Kind::Persistent).await?;
        assert::status(&first, 201)?;
        let second = commit(ctx, &ehr_id, Kind::Persistent).await?;
        support::assert_negative(&second)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_invalid_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_create_invalid(ctx, Kind::Event).await })
}
fn run_create_invalid_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_create_invalid(ctx, Kind::Persistent).await })
}

/// Commit the vendored `__invalid_wrong_structure` fixture (malformed content);
/// the server must reject it (composition_create.yaml 400/422).
async fn run_create_invalid(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    ensure_opt(ctx, kind).await?;
    let malformed = fixtures::read_from(JSON_DIR, kind.invalid_structure_file()).map_err(codec)?;
    let resp = ctx
        .send(
            HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                .text_body(malformed, "application/json")
                .header("accept", "application/json"),
        )
        .await?;
    support::assert_negative(&resp)?;
    Ok(DataSetReport::SINGLE)
}

fn run_create_event_bad_opt<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // The composition references a template never uploaded; the server must
        // reject it (composition_create.yaml 404/422 — a negative with
        // non-existent-OPT info).
        let ehr_id = support::create_ehr(ctx).await?;
        let body = read_json_file(BAD_OPT_FILE)?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition")).json_body(&body)?,
                Format::Json,
            ))
            .await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_event_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // A valid composition against a non-existent EHR: the {ehr_id} path
        // resource is absent → 404 (composition_create.yaml 404 EHR-not-found).
        ensure_opt(ctx, Kind::Event).await?;
        let body = read_json_file(Kind::Event.json_file())?;
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::post(format!("/ehr/{}/composition", Uuid::new_v4()))
                    .json_body(&body)?,
                Format::Json,
            ))
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── has_composition ────────────────────────────────────────────────────────────

fn run_has_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // SM has_composition TRUE = GET /composition/{uid} 200 + content check.
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let object = ids::object_uid(&uid).to_owned();
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{object}"),
            Kind::Event,
            Some(1),
        )
        .await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_composition_bad_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // SM has_composition FALSE = GET /composition/{uid} 404 (composition_get.yaml 404).
        let ehr_id = support::create_ehr(ctx).await?;
        get_expect_404(ctx, format!("/ehr/{ehr_id}/composition/{}", Uuid::new_v4())).await
    })
}

fn run_has_composition_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        get_expect_404(
            ctx,
            format!("/ehr/{}/composition/{}", Uuid::new_v4(), Uuid::new_v4()),
        )
        .await
    })
}

// ── get latest ─────────────────────────────────────────────────────────────────

fn run_get_latest<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Two versions committed; GET latest must return version 2 (proving
        // "is latest") + the content check (master07 §get_composition_latest).
        let (ehr_id, object, _uid1, _uid2) = setup_two(ctx, Kind::Event).await?;
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{object}"),
            Kind::Event,
            Some(2),
        )
        .await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_latest_bad_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = support::create_ehr(ctx).await?;
        get_expect_404(ctx, format!("/ehr/{ehr_id}/composition/{}", Uuid::new_v4())).await
    })
}

fn run_get_latest_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        get_expect_404(
            ctx,
            format!("/ehr/{}/composition/{}", Uuid::new_v4(), Uuid::new_v4()),
        )
        .await
    })
}

// ── get at time ──────────────────────────────────────────────────────────────

fn run_get_at_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // At the current time → the latest version of the matching COMPOSITION
        // + content check (master07 §get_composition_at_time).
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let object = ids::object_uid(&uid).to_owned();
        let now = Timestamp::now();
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{object}?version_at_time={now}"),
            Kind::Event,
            Some(1),
        )
        .await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_at_time_no_arg<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // No time argument → the latest version + content check.
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let object = ids::object_uid(&uid).to_owned();
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{object}"),
            Kind::Event,
            Some(1),
        )
        .await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_at_time_bad_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = support::create_ehr(ctx).await?;
        get_expect_404(
            ctx,
            format!(
                "/ehr/{ehr_id}/composition/{}?version_at_time=2030-01-01T00:00:00Z",
                Uuid::new_v4()
            ),
        )
        .await
    })
}

fn run_get_at_time_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        get_expect_404(
            ctx,
            format!(
                "/ehr/{}/composition/{}?version_at_time=2030-01-01T00:00:00Z",
                Uuid::new_v4(),
                Uuid::new_v4()
            ),
        )
        .await
    })
}

fn run_get_at_times<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Two versions committed at t0 < t1; probe three time points
        // (master07 §get_composition_at_times): before t0 → negative;
        // t0<t<t1 → v1; t>t1 → v2 (each with the content check).
        let ehr_id = support::create_ehr(ctx).await?;
        ensure_opt(ctx, Kind::Event).await?;
        let first = commit(ctx, &ehr_id, Kind::Event).await?;
        assert::status(&first, 201)?;
        let uid1 = ids::version_uid(ctx, &first)?;
        let object = ids::object_uid(&uid1).to_owned();

        // A window strictly between the two commits.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let between = Timestamp::now();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let second = update(ctx, &ehr_id, &object, &uid1, Kind::Event).await?;
        assert::status_in(&second, &[200, 204])?;

        // Before any version exists: no matching VERSION at that time. The
        // schedule frames this as a negative; ITS-REST composition_get.yaml
        // returns 404 (not found) or 204 (no version content) — underdetermined,
        // so both are accepted.
        let before = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/composition/{object}?version_at_time=1900-01-01T00:00:00Z"
                )),
                Format::Json,
            ))
            .await?;
        assert::status_in(&before, &[204, 404])?;

        // Between t0 and t1 → v1; after t1 → v2 (latest). Each is version-checked
        // and content-checked.
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{object}?version_at_time={between}"),
            Kind::Event,
            Some(1),
        )
        .await?;
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{object}?version_at_time=2030-01-01T00:00:00Z"),
            Kind::Event,
            Some(2),
        )
        .await?;
        Ok(DataSetReport::all(3).of_schedule_rows(3))
    })
}

// ── get version ──────────────────────────────────────────────────────────────

fn run_get_version<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Two versions committed; retrieve v1 by its OBJECT_VERSION_ID — the
        // returned version must be v1 (not the latest) + content check
        // (master07 §get_composition_version).
        let (ehr_id, _object, uid1, _uid2) = setup_two(ctx, Kind::Event).await?;
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{uid1}"),
            Kind::Event,
            Some(1),
        )
        .await?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_version_bad_version<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // A syntactically valid OBJECT_VERSION_ID naming a version the SUT does
        // not hold (built from an OBSERVED id) → 404 (composition_get.yaml 404).
        let (ehr_id, observed) = observe_ovid(ctx).await?;
        let bogus = support::nonexistent_version_like(&observed);
        get_expect_404(ctx, format!("/ehr/{ehr_id}/composition/{bogus}")).await
    })
}

fn run_get_version_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let (_ehr_id, observed) = observe_ovid(ctx).await?;
        let bogus = support::nonexistent_version_like(&observed);
        get_expect_404(ctx, format!("/ehr/{}/composition/{bogus}", Uuid::new_v4())).await
    })
}

fn run_get_versions<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Two versions v1, v2; each id retrieves its own version + content check.
        let (ehr_id, _object, uid1, uid2) = setup_two(ctx, Kind::Event).await?;
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{uid1}"),
            Kind::Event,
            Some(1),
        )
        .await?;
        get_and_check(
            ctx,
            format!("/ehr/{ehr_id}/composition/{uid2}"),
            Kind::Event,
            Some(2),
        )
        .await?;
        Ok(DataSetReport::all(2).of_schedule_rows(2))
    })
}

// ── versioned composition ──────────────────────────────────────────────────────

fn run_get_versioned<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // A valid VERSIONED_COMPOSITION referencing its VERSION(s): assert the
        // container _type + that its uid is the versioned-object uid (master07
        // §get_versioned_composition; RM common §VERSIONED_OBJECT).
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let object = ids::object_uid(&uid).to_owned();
        let resp = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/ehr/{ehr_id}/versioned_composition/{object}")),
                ctx.format,
            ))
            .await?;
        assert::status(&resp, 200)?;
        if resp.body.is_empty() {
            return Err(CaseError::Assertion(
                "VERSIONED_COMPOSITION body is empty".to_owned(),
            ));
        }
        // Validate over a JSON read (works for both formats — F-05-06 version-family XML).
        let body = match ctx.format {
            Format::Json => resp.json()?,
            Format::Xml => {
                let j = ctx
                    .send(negotiate::accept(
                        HttpRequest::get(format!("/ehr/{ehr_id}/versioned_composition/{object}")),
                        Format::Json,
                    ))
                    .await?;
                assert::status(&j, 200)?;
                j.json()?
            }
        };
        if body["_type"] != "VERSIONED_COMPOSITION" {
            return Err(CaseError::Assertion(format!(
                "expected VERSIONED_COMPOSITION, got {}",
                body["_type"]
            )));
        }
        if body.pointer("/uid/value").and_then(Value::as_str) != Some(object.as_str()) {
            return Err(CaseError::Assertion(format!(
                "VERSIONED_COMPOSITION.uid should be the versioned-object uid {object:?}, got {}",
                body["uid"]["value"]
            )));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_versioned_non_existent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        let ehr_id = support::create_ehr(ctx).await?;
        get_expect_404(
            ctx,
            format!("/ehr/{ehr_id}/versioned_composition/{}", Uuid::new_v4()),
        )
        .await
    })
}

fn run_get_versioned_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        get_expect_404(
            ctx,
            format!(
                "/ehr/{}/versioned_composition/{}",
                Uuid::new_v4(),
                Uuid::new_v4()
            ),
        )
        .await
    })
}

// ── update ─────────────────────────────────────────────────────────────────────

fn run_update_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_update(ctx, Kind::Event).await })
}
fn run_update_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_update(ctx, Kind::Persistent).await })
}

/// Create then update a `kind` composition → 2 VERSIONs; assert version-tree
/// number 2 and the audit `change_type` postcondition CREATE(249)→MODIFY(251)
/// read back from the ORIGINAL_VERSIONs (master07 §update_composition; G-2).
async fn run_update(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let (ehr_id, object, uid1, uid2) = setup_two(ctx, kind).await?;
    assert_version_number(&uid2, 2)?;
    let v1 = original_version(ctx, &ehr_id, &object, &uid1).await?;
    assert_change_type(&v1, "249", "creation")?;
    let v2 = original_version(ctx, &ehr_id, &object, &uid2).await?;
    assert_change_type(&v2, "251", "modification")?;
    Ok(DataSetReport::SINGLE)
}

fn run_update_non_existent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // A random (absent) preceding_version_uid → negative (composition_update.yaml
        // 400/404/412/422). The id is built from an OBSERVED id.
        let ehr_id = support::create_ehr(ctx).await?;
        ensure_opt(ctx, Kind::Event).await?;
        let (_obs, observed) = observe_ovid(ctx).await?;
        let bogus = support::nonexistent_version_like(&observed);
        let object = ids::object_uid(&bogus).to_owned();
        let resp = update(ctx, &ehr_id, &object, &bogus, Kind::Event).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_update_wrong_template<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // Update an event composition with a body referencing a DIFFERENT
        // template (persistent_minimal) → negative (composition_update.yaml 422
        // template_id mismatch).
        //
        // Boundary: the schedule wants a template_id-mismatch error; the exact
        // error-body shape is underdetermined here, so only the negative status
        // is asserted (composition_update.yaml 422; register 04 §2).
        let (ehr_id, uid1) = setup(ctx, Kind::Event).await?;
        let object = ids::object_uid(&uid1).to_owned();
        ensure_opt(ctx, Kind::Persistent).await?;
        let resp = update(ctx, &ehr_id, &object, &uid1, Kind::Persistent).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── delete ─────────────────────────────────────────────────────────────────────

fn run_delete_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_delete(ctx, Kind::Event).await })
}
fn run_delete_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({ run_delete(ctx, Kind::Persistent).await })
}

/// Create then delete a `kind` composition → a new (deleted) VERSION. Assert
/// the logical-delete postcondition: the deleted `VERSION.lifecycle_state =
/// openehr::523|deleted|` (master07 §delete_composition NOTE) AND a subsequent
/// GET of the latest composition is 204/404 (composition_get.yaml
/// 204_because_deleted). The delete path segment is the version uid to delete
/// (composition_delete.yaml: the uid_based_id MUST be the OBJECT_VERSION_ID of
/// the most recent version).
async fn run_delete(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let (ehr_id, uid1) = setup(ctx, kind).await?;
    let object = ids::object_uid(&uid1).to_owned();
    let resp = ctx
        .send(HttpRequest::delete(format!(
            "/ehr/{ehr_id}/composition/{uid1}"
        )))
        .await?;
    assert::status_in(&resp, &[200, 204])?;
    // composition_delete.yaml 204: ETag + Location of the deleted version.
    let deleted_uid = ids::version_uid(ctx, &resp)?;
    let ov = original_version(ctx, &ehr_id, &object, &deleted_uid).await?;
    let ls = coded_code(&ov["lifecycle_state"]);
    if ls != Some("523") {
        return Err(CaseError::Assertion(format!(
            "deleted VERSION.lifecycle_state should be openehr::523|deleted| \
             (master07 §delete_composition), got {ls:?}"
        )));
    }
    let after = ctx
        .send(negotiate::accept(
            HttpRequest::get(format!("/ehr/{ehr_id}/composition/{object}")),
            Format::Json,
        ))
        .await?;
    assert::status_in(&after, &[204, 404])?;
    Ok(DataSetReport::SINGLE)
}

fn run_delete_non_existent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    boxed!({
        // A well-formed but non-existent OBJECT_VERSION_ID (built from an
        // OBSERVED id) → negative (composition_delete.yaml 400/404/409/412).
        let ehr_id = support::create_ehr(ctx).await?;
        let (_obs, observed) = observe_ovid(ctx).await?;
        let bogus = support::nonexistent_version_like(&observed);
        let resp = ctx
            .send(HttpRequest::delete(format!(
                "/ehr/{ehr_id}/composition/{bogus}"
            )))
            .await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}
