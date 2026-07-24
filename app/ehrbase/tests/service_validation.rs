#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end composition-validation tests against a real PostgreSQL 18
//! (shared testkit harness): a COMPOSITION committed via the ITS-REST create/update
//! path is validated against its operational template *before* persistence.
//!
//! Oracle + fixtures: the vendored Apache-2.0 openEHR SDK corpus — the IPS
//! operational template (`openehr-its/tests/fixtures/sdk/ips.v0.opt`,
//! `template_id` "International Patient Summary") paired with its canonical-JSON
//! compositions (`openehr-its/tests/vendor/openehr_sdk/composition/…`):
//! `ips_canonical.json` (valid) and `ips_invalid.json` (out-of-range magnitudes
//! and coded values outside the value set). Same pairing the `openehr_its::flat`
//! validator's own corpus tests use (`openehr-its/tests/validation.rs`).
//!
//! Spec: openEHR ITS-REST 1.1.0 —
//! `docs/specs/openehr/ITS-REST/specifications/responses/422_COMPOSITION.yaml`
//! ("content could be converted to a COMPOSITION, but there are semantic
//! validation errors, such as the underlying template is not known or is not
//! validating the supplied COMPOSITION" → `422`). CNF cross-check:
//! `docs/specs/openehr/CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`
//! (`create_composition-event_bad_opt` → 422).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::doc_markdown)]

use serde_json::{Value, json};
use sqlx::PgPool;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::service::EhrbaseService;
use ehrbase::service::error::ServiceError;
use ehrbase::service::status::{CallStatusType, SmError};

use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};

/// An `openehr` terminology code (audit change type / lifecycle state).
fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// The SM `UPDATE_VERSION` commit envelope for a bare-RM write.
fn uv(data: Value, change_code: &str, preceding: Option<&str>) -> UpdateVersion {
    UpdateVersion {
        preceding_version_uid: preceding.map(|p| p.parse().expect("OBJECT_VERSION_ID")),
        lifecycle_state: term("532"),
        attestations: None,
        data,
        audit: UpdateAudit {
            change_type: term(change_code),
            description: None,
            committer: openehr_its::json::from_canonical_value::<PartyProxy>(
                &json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
            )
            .expect("committer"),
            system_id: None,
        },
        signature: None,
    }
}

/// Read a workspace fixture relative to this crate's manifest dir.
fn fixture(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn composition(name: &str) -> Value {
    let rel = format!(
        "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/{name}"
    );
    serde_json::from_str(&fixture(&rel)).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

const IPS_OPT: &str = "../../crates/openehr-its/tests/fixtures/sdk/ips.v0.opt";

/// Count the persisted COMPOSITION versions (kind discriminator on `vo_version`).
async fn composition_versions(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM vo_version WHERE kind = 'COMPOSITION'")
        .fetch_one(pool)
        .await
        .expect("count compositions")
}

#[tokio::test]
async fn composition_validation_gates_persistence() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());

    // Ingest the IPS operational template (the validation target).
    svc.template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");

    let ehr_id = svc.create_ehr(None).await.expect("create_ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    // ── valid composition → committed and retrievable ────────────────────────
    // NOTE: `create_composition` returns the new version_uid.
    let ovid = svc
        .create_composition(ehr_uuid, uv(composition("ips_canonical.json"), "249", None))
        .await
        .expect("valid composition accepted (201)")
        .version_uid();
    let vo_id = ovid.split("::").next().unwrap().to_owned();

    let fetched = svc
        .get_composition_latest(ehr_uuid, vo_id.parse().expect("vo uuid"))
        .await
        .expect("valid composition persisted");
    assert_eq!(
        fetched["uid"]["value"], ovid,
        "persisted composition round-trips"
    );
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "exactly the one valid composition is stored"
    );

    // ── invalid composition → 422 with per-path violations, NOT persisted ─────
    let err = svc
        .create_composition(ehr_uuid, uv(composition("ips_invalid.json"), "249", None))
        .await
        .expect_err("invalid composition rejected");
    // A well-formed body that fails RM/template validation is a 422 carrying the
    // per-path violations (`ServiceError::ValidationFailed`); the protocol adapter
    // (`ehrbase-rest`) renders the structured list into the 422 body. Here we
    // assert the variant + that the violations carry the RM paths.
    let ServiceError::ValidationFailed(violations) = &err else {
        panic!("expected content_invalid (422) with per-path violations, got {err:?}");
    };
    assert!(
        violations.iter().any(|v| v.path.contains('/')),
        "422 violations carry the RM-path-keyed entries: {violations:?}"
    );
    // Validation runs before the write transaction, so nothing was persisted.
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the rejected composition was not persisted"
    );

    // ── unknown template → 422 "template not known" ──────────────────────────
    let mut unknown = composition("ips_canonical.json");
    unknown["archetype_details"]["template_id"]["value"] =
        Value::String("no.such.template.v0".to_owned());
    let err = svc
        .create_composition(ehr_uuid, uv(unknown, "249", None))
        .await
        .expect_err("unknown template rejected");
    // An unknown template is a 422 `ServiceError::Unprocessable`; the cause
    // still rides in the message.
    let ServiceError::Unprocessable(message) = &err else {
        panic!("expected content_invalid (422) for unknown template, got {err:?}");
    };
    assert!(
        message.contains("not known"),
        "422 message names the cause: {message}"
    );
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the unknown-template composition was not persisted"
    );
}

#[tokio::test]
async fn composition_update_is_validated() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());

    svc.template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");
    let ehr_id = svc.create_ehr(None).await.expect("create_ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    // Seed a valid v1.
    let ovid_v1 = svc
        .create_composition(ehr_uuid, uv(composition("ips_canonical.json"), "249", None))
        .await
        .expect("valid v1")
        .version_uid();
    let vo_id = ovid_v1.split("::").next().unwrap().to_owned();
    let vo_uuid = vo_id.parse::<ehrbase::ids::VoId>().expect("vo uuid");

    // An update whose body fails template validation is rejected (422) and the
    // stored current version stays at v1.
    let err = svc
        .update_composition(
            ehr_uuid,
            vo_uuid,
            uv(composition("ips_invalid.json"), "251", Some(&ovid_v1)),
        )
        .await
        .expect_err("invalid update rejected");
    // A failed-validation update is a 422 carrying the per-path violations.
    let ServiceError::ValidationFailed(violations) = &err else {
        panic!("expected content_invalid (422), got {err:?}");
    };
    assert!(
        !violations.is_empty(),
        "the 422 carries at least one per-path violation, got {err:?}"
    );

    let current = svc
        .get_composition_latest(ehr_uuid, vo_uuid)
        .await
        .expect("current still readable");
    assert_eq!(
        current["uid"]["value"], ovid_v1,
        "the rejected update did not advance the version"
    );
}

/// A version-item `commit_audit.change_type` as a coded openEHR audit change type.
fn change_type_coded(code: &str, value: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT", "value": value,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
            "code_string": code
        }
    })
}

/// A `553|incomplete|` CONTRIBUTION carrying a single `creation` version.
fn incomplete_creation_contribution(data: &Value) -> Value {
    json!({
        "versions": [{
            "data": data,
            "commit_audit": { "change_type": change_type_coded("249", "creation") },
            "lifecycle_state": {
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "553"
            }
        }]
    })
}

#[tokio::test]
async fn incomplete_lifecycle_relaxes_lower_bounds_but_not_wrongness() {
    // RM common master06 §"Incomplete Content": a `553|incomplete|` commit
    // treats existence/cardinality lower limits as zero ("data may be missing"),
    // while every wrongness check still applies ("but it may not be wrong").
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());

    svc.template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");
    let ehr_uuid = svc.create_ehr(None).await.expect("create_ehr");

    // A composition missing its mandatory sections (content emptied): under the
    // IPS template each SECTION has occurrences min >= 1, so this is a pure
    // lower-bound (Required/Occurrences) violation with no wrongness. Missing
    // content is represented as ABSENT (Void) — a present-empty `content: []`
    // violates COMPOSITION.Content_valid (`content /= Void implies not
    // content.is_empty`, composition.adoc), an RM invariant that stays at full
    // strictness even under 553 ("data may be missing, but it may not be
    // wrong", RM common master06 §Incomplete Content).
    let mut missing = composition("ips_canonical.json");
    missing.as_object_mut().unwrap().remove("content");

    // Committed as `532|complete|` (the direct create path is always complete),
    // the missing-section lower bound is enforced → 422.
    let strict = svc
        .create_composition(ehr_uuid, uv(missing.clone(), "249", None))
        .await;
    assert!(
        matches!(strict, Err(ServiceError::ValidationFailed(_))),
        "a complete commit must reject the missing mandatory sections, got {strict:?}"
    );
    assert_eq!(
        composition_versions(&pool).await,
        0,
        "the rejected complete commit persisted nothing"
    );

    // The identical body committed as `553|incomplete|` is accepted: the lower
    // limits are treated as zero.
    svc.create_ehr_contribution(ehr_uuid, incomplete_creation_contribution(&missing))
        .await
        .expect("an incomplete commit tolerates the missing mandatory sections");
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the incomplete commit persisted the composition"
    );

    // But an incomplete commit does NOT tolerate *wrong* data: ips_invalid has
    // out-of-range magnitudes / coded values outside the value set, which are
    // still rejected under 553 ("may be missing, but may not be wrong").
    let wrong = svc
        .create_ehr_contribution(
            ehr_uuid,
            incomplete_creation_contribution(&composition("ips_invalid.json")),
        )
        .await;
    assert!(
        matches!(
            wrong,
            Err(SmError {
                status: CallStatusType::ContentInvalid,
                ..
            })
        ),
        "an incomplete commit must still reject wrong (out-of-range/coded) data, got {wrong:?}"
    );
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the wrong incomplete commit persisted nothing"
    );
}

// ── WebTemplate cache: per-commit store-read elimination + invalidation ──────
// The commit path validates each COMPOSITION against its operational template's
// derived-runtime `WebTemplate`, resolved through `web_template_for`. That seam
// is cache-first: once a template is warm, subsequent commits are served from
// the in-memory `WebTemplate` cache and never re-read `template_store` (the
// profiled per-commit OPT read). No openEHR spec governs the cache; the spec
// blesses a compiled near-runtime form, the caching is our own design.

/// A second commit against an already-warm template is served from the cached
/// `WebTemplate` and does **not** re-read `template_store`. Proof by probe: after
/// warming the cache with one commit, the stored OPT content is corrupted in
/// place; a second commit still succeeds, which is only possible if the resolver
/// never re-read the (now-broken) stored content. (The `vo_version.template_id`
/// FK forbids deleting the referenced row, so the probe corrupts content rather
/// than deleting — `UPDATE` is not a supported template mutation, only a probe.)
#[tokio::test]
async fn warm_template_is_served_from_cache_without_a_store_read() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());

    let desc = svc
        .template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");
    let template_id = desc["template_id"]
        .as_str()
        .expect("template_id")
        .to_owned();

    let ehr_uuid = svc.create_ehr(None).await.expect("create_ehr");

    // First commit warms the cache (web_template_for: miss → build → cache).
    svc.create_composition(ehr_uuid, uv(composition("ips_canonical.json"), "249", None))
        .await
        .expect("first valid composition warms the cache");

    // Poison the stored OPT content: any later *store read* on the commit path
    // would build a broken WebTemplate and fail.
    let poisoned =
        sqlx::query("UPDATE template_store SET content = $1 WHERE lower(template_id) = lower($2)")
            .bind("<not-an-operational-template/>")
            .bind(&template_id)
            .execute(&pool)
            .await
            .expect("poison the stored OPT content")
            .rows_affected();
    assert_eq!(poisoned, 1, "exactly the one template row was poisoned");

    // Second commit against the same warm template still succeeds — served from
    // the cached WebTemplate, never re-reading the poisoned store content.
    svc.create_composition(ehr_uuid, uv(composition("ips_canonical.json"), "249", None))
        .await
        .expect("second commit is served from the warm cache, not the poisoned store");

    assert_eq!(
        composition_versions(&pool).await,
        2,
        "both compositions persisted"
    );
}

/// Deleting a template through the SM `delete_opt` path invalidates its cached
/// `WebTemplate`. Proof: warm the cache via the example endpoint (no committed
/// composition, so no `vo_version` FK reference blocks the delete), delete the
/// OPT, then commit against it. With invalidation the resolver misses, re-reads
/// the store, finds nothing, and returns the clean "template not known" 422.
/// Were the entry NOT invalidated, the stale WebTemplate would validate the
/// commit and the `vo_version.template_id` FK would then fail with a database
/// error — a different, wrong outcome. Asserting the 422 proves eviction.
#[tokio::test]
async fn deleting_a_template_invalidates_its_web_template_cache() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());

    let desc = svc
        .template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");
    let template_id = desc["template_id"]
        .as_str()
        .expect("template_id")
        .to_owned();

    // Warm the WebTemplate cache without committing a composition (the example
    // endpoint builds and caches the same WebTemplate the commit path uses).
    let example = svc
        .template_adl14_example(template_id.clone(), Some("required".to_owned()), None)
        .await
        .expect("example warms the cache");
    assert_eq!(
        example.get("_type").and_then(Value::as_str),
        Some("COMPOSITION"),
        "example is a COMPOSITION"
    );

    // Delete the OPT through the real SM path (opt_delete), which invalidates
    // the cache entry.
    let opt_uuid = svc
        .list_opts_adl14(ehrbase::service::list::Page::all())
        .await
        .expect("list opts")
        .into_iter()
        .next()
        .expect("one stored OPT uuid");
    svc.delete_opt(opt_uuid).await.expect("delete opt");

    // A commit against the now-deleted template resolves to the clean
    // "template not known" 422 — only reachable if the stale WebTemplate was
    // evicted (otherwise the FK would fail with a database error instead).
    let ehr_uuid = svc.create_ehr(None).await.expect("create_ehr");
    let err = svc
        .create_composition(ehr_uuid, uv(composition("ips_canonical.json"), "249", None))
        .await
        .expect_err("commit against a deleted template is rejected");
    let ServiceError::Unprocessable(message) = &err else {
        panic!("expected content_invalid (422) for the deleted template, got {err:?}");
    };
    assert!(
        message.contains("not known"),
        "422 names the cause: {message}"
    );
    assert_eq!(
        composition_versions(&pool).await,
        0,
        "nothing persisted against the deleted template"
    );
}
