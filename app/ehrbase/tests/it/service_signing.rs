//! End-to-end tests for version signing (`VERSION.signature`, RM common
//! §"Digital Signature")
//! against a real `PostgreSQL` 18 (shared testkit harness).
//!
//! The strongest assertion (§6.3): the digest recomputes from the **served**
//! `ORIGINAL_VERSION`'s `canonical_form` — proving commit-time and read-time
//! object identity. Also: `EHR_STATUS` / FOLDER / contribution versions are all
//! signed; client-supplied signatures are stored verbatim; `verify_on_read =
//! strict` turns a tampered row into a 5xx; canonical XML carries the signature.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use std::sync::Arc;

use ehrbase::service::EhrbaseService;
use ehrbase::service::status::{CallStatusType, SmError};
use ehrbase::versioning::signature::config::{Mode, SigningConfig, VerifyOnRead};
use ehrbase::versioning::signature::signer::Signer;
use ehrbase::versioning::signature::verify::Verdict;

use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};
use openehr_base::prelude::{ObjectVersionId, TerminologyCode};
use openehr_rm::common::change_control::version_impl::canonical_form_of_json;
use openehr_rm::prelude::PartyProxy;
use serde_json::{Value, json};
use sqlx::{PgPool, Row};

fn uid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
}

/// An `openehr` terminology code (audit change type / lifecycle state).
fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

fn committer(name: &str) -> PartyProxy {
    openehr_its::json::from_canonical_value(&json!({ "_type": "PARTY_IDENTIFIED", "name": name }))
        .expect("committer")
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
            committer: committer("conformance tester"),
            system_id: None,
        },
        signature: None,
    }
}

/// The contribution-level `UPDATE_AUDIT`.
fn contribution_audit(change_code: &str, committer_name: &str) -> UpdateAudit {
    UpdateAudit {
        change_type: term(change_code),
        description: None,
        committer: committer(committer_name),
        system_id: None,
    }
}

/// Split an `OBJECT_VERSION_ID` into `(object_id uuid, trunk version)`.
fn version_components(ovid: &str) -> (ehrbase::ids::VoId, String) {
    let parts: Vec<&str> = ovid.split("::").collect();
    (parts[0].parse().expect("vo uuid"), parts[2].to_owned())
}

/// A minimal *valid* RM COMPOSITION: `language`, `territory`, `category`, and
/// `composer` are all `1..1` (RM ehr, COMPOSITION class), so the typed RM
/// validation rejects a fixture without them.
fn composition(name: &str) -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": {
                "_type": "ARCHETYPE_ID",
                "value": "openEHR-EHR-COMPOSITION.encounter.v1"
            },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": name },
        "language": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" },
            "code_string": "en"
        },
        "territory": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" },
            "code_string": "NL"
        },
        "category": {
            "_type": "DV_CODED_TEXT",
            "value": "event",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "433"
            }
        },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }
    })
}

fn change_type(code: &str, value: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT", "value": value,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
            "code_string": code
        }
    })
}

/// Assert a served `ORIGINAL_VERSION` carries a server digest signature that
/// recomputes from its own `canonical_form` — the strongest test (§6.3).
fn assert_digest_recomputes(ov: &Value) {
    assert_eq!(
        ov["_type"], "ORIGINAL_VERSION",
        "the served version must be an ORIGINAL_VERSION"
    );
    let signature = ov["signature"]
        .as_str()
        .expect("ORIGINAL_VERSION.signature");
    assert!(
        signature.starts_with("sha256:"),
        "expected a digest signature, got {signature}"
    );
    // canonical_form_of_json strips the signature key + JCS-canonicalises — the
    // exact bytes that were signed at commit. digest_default().verify recomputes.
    let canonical = canonical_form_of_json(ov).expect("canonical form");
    assert_eq!(
        Signer::digest_default().verify(&canonical, signature),
        Verdict::DigestMatch,
        "digest must recompute from the served version's canonical form"
    );
}

async fn create_ehr(svc: &EhrbaseService) -> String {
    svc.create_ehr(None).await.expect("create_ehr").to_string()
}

#[tokio::test]
async fn composition_version_is_signed_and_digest_recomputes_from_served_version() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    // Create a composition, then commit a second version.
    let ovid_v1 = svc
        .create_composition(ehr_uuid, uv(composition("v1"), "249", None))
        .await
        .expect("create_composition")
        .version_uid();
    let vo_id = ovid_v1.split("::").next().unwrap().to_owned();
    let vo_uuid = vo_id.parse::<ehrbase::ids::VoId>().expect("vo uuid");
    let ovid_v2 = svc
        .update_composition(
            ehr_uuid,
            vo_uuid,
            uv(composition("v2"), "251", Some(&ovid_v1)),
        )
        .await
        .expect("update_composition")
        .version_uid();

    for ovid in [&ovid_v1, &ovid_v2] {
        let ov = svc
            .composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
            .await
            .expect("versioned composition version");
        assert_digest_recomputes(&ov);
    }
}

#[tokio::test]
async fn ehr_status_versions_are_signed_and_every_vo_version_carries_a_digest() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    // Update EHR_STATUS → v2, then read the ORIGINAL_VERSION of v1.
    let mut body = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status get");
    let status_ovid_v1 = uid(&body).to_owned();
    body.as_object_mut().expect("status obj").remove("uid");
    // Flip is_queryable (not is_modifiable): the test only needs a second
    // EHR_STATUS version, and with the B2 write guard a deactivated EHR
    // (is_modifiable = false, RM ehr master04 §"EHR Active Status") would
    // refuse the directory commit below.
    body["is_queryable"] = json!(false);
    svc.replace_ehr_status(ehr_uuid, uv(body, "251", Some(&status_ovid_v1)))
        .await
        .expect("status update");

    let (status_vo, status_ver) = version_components(&status_ovid_v1);
    let ov = svc
        .ehr_status_original_version(ehr_uuid, status_vo, &status_ver)
        .await
        .expect("versioned ehr_status version");
    assert_eq!(ov["data"]["_type"], "EHR_STATUS");
    assert_digest_recomputes(&ov);

    // Also commit a directory (FOLDER) — its version is signed even though the
    // FOLDER endpoints serve the bare folder (no ORIGINAL_VERSION wrapper).
    let folder = json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "root" }
    });
    svc.create_directory(ehr_uuid, uv(folder, "249", None))
        .await
        .expect("create_directory");

    // Sweep: EHR_STATUS (x2), EHR_ACCESS, FOLDER — every stored version is signed
    // with a digest (design §3.4: signing on by default).
    let rows = sqlx::query("SELECT kind, signature FROM vo_version ORDER BY kind, sys_version")
        .fetch_all(&pool)
        .await
        .expect("select vo_version");
    assert!(!rows.is_empty());
    for row in &rows {
        let kind: String = row.try_get("kind").unwrap();
        let sig: Option<String> = row.try_get("signature").unwrap();
        let sig = sig.unwrap_or_else(|| panic!("{kind} version is unsigned"));
        assert!(
            sig.starts_with("sha256:"),
            "{kind} version signature is not a digest: {sig}"
        );
    }
}

#[tokio::test]
async fn contribution_versions_are_signed() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    let contribution_uid = svc
        .commit_contribution(
            ehr_uuid,
            vec![uv(composition("Via contribution"), "249", None)],
            contribution_audit("249", "Dr. Contribution"),
        )
        .await
        .expect("commit_contribution");
    // NOTE: `commit_contribution` returns the contribution_uid;
    // the created version's OBJECT_VERSION_ID is read back from the CONTRIBUTION.
    let contribution = svc
        .get_contribution(ehr_uuid, contribution_uid.parse().expect("contrib uuid"))
        .await
        .expect("get_contribution");
    let ovid = contribution["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();

    let ov = svc
        .composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await
        .expect("versioned composition version");
    assert_digest_recomputes(&ov);
}

#[tokio::test]
async fn client_supplied_signature_is_stored_verbatim() {
    // A CONTRIBUTION creation version carrying an author-generated signature —
    // stored verbatim, never re-signed (RM common master06 §Digital Signature).
    // The read runs under the STRICT default (EhrbaseService::new): a foreign
    // client signature is marked client-supplied at commit, so read-time
    // verification skips it (never recomputes a foreign canonical form) and the
    // strict read still succeeds — proving strict-by-default (#273) does not
    // reject legitimately-stored client signatures.
    const CLIENT_SIG: &str =
        "-----BEGIN PGP SIGNATURE-----\nauthored-elsewhere\n-----END PGP SIGNATURE-----";
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    let mut version = uv(composition("Client signed"), "249", None);
    version.signature = Some(CLIENT_SIG.to_owned());
    let contribution_uid = svc
        .commit_contribution(
            ehr_uuid,
            vec![version],
            contribution_audit("249", "Dr. Author"),
        )
        .await
        .expect("commit_contribution");
    let contribution = svc
        .get_contribution(ehr_uuid, contribution_uid.parse().expect("contrib uuid"))
        .await
        .expect("get_contribution");
    let ovid = contribution["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();

    let ov = svc
        .composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await
        .expect("versioned composition version");
    assert_eq!(
        ov["signature"].as_str(),
        Some(CLIENT_SIG),
        "client-supplied signature must be stored verbatim"
    );
}

#[tokio::test]
async fn strict_verify_on_read_rejects_a_tampered_row() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    // A strict-verify service: a signature that does not match the served
    // canonical form is a 5xx integrity failure (design §3.5).
    let config = SigningConfig {
        enabled: true,
        mode: Mode::Digest,
        key_path: None,
        key_passphrase: None,
        key_passphrase_file: None,
        verify_on_read: Some(VerifyOnRead::Strict),
    };
    let signer = Signer::from_config(&config).expect("strict signer");
    let svc = EhrbaseService::new(pool.clone()).with_signer(Arc::new(signer));
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    let ovid = svc
        .create_composition(ehr_uuid, uv(composition("tamper"), "249", None))
        .await
        .expect("create_composition")
        .version_uid();

    // A clean read verifies fine.
    svc.composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await
        .expect("clean read verifies");

    // Tamper the stored signature via SQL.
    sqlx::query(
        "UPDATE vo_version SET signature = 'sha256:dGFtcGVyZWQ=' WHERE kind = 'COMPOSITION'",
    )
    .execute(&pool)
    .await
    .expect("tamper");

    let tampered = svc
        .composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await;
    // NOTE: a signing/integrity failure surfaces at the SM boundary
    // as `SmError { status: Exception }` (the adapter maps it to the same wire 5xx
    // the old `ApiError::Internal` produced).
    assert!(
        matches!(
            tampered,
            Err(SmError {
                status: CallStatusType::Exception,
                ..
            })
        ),
        "strict verify_on_read must 5xx a tampered signature, got {tampered:?}"
    );
}

#[tokio::test]
async fn default_verify_on_read_is_strict_and_rejects_a_tampered_row() {
    // #273: with signing enabled and verify_on_read UNSET, the effective default
    // is strict — the "sign but silently never check" default is gone. This uses
    // the plain service default (EhrbaseService::new → Signer::digest_default),
    // no explicit signing config, to prove the DEFAULT posture.
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    let ovid = svc
        .create_composition(ehr_uuid, uv(composition("tamper-default"), "249", None))
        .await
        .expect("create_composition")
        .version_uid();

    // A clean read verifies fine under the strict default.
    svc.composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await
        .expect("clean read verifies under the strict default");

    // Tamper the stored (server-generated) signature.
    sqlx::query(
        "UPDATE vo_version SET signature = 'sha256:dGFtcGVyZWQ=' WHERE kind = 'COMPOSITION'",
    )
    .execute(&pool)
    .await
    .expect("tamper");

    let tampered = svc
        .composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await;
    assert!(
        matches!(
            tampered,
            Err(SmError {
                status: CallStatusType::Exception,
                ..
            })
        ),
        "the strict default must 5xx a tampered signature, got {tampered:?}"
    );
}

/// Build a digest-mode service with an explicit `verify_on_read` policy.
fn service_with_verify(pool: PgPool, policy: VerifyOnRead) -> EhrbaseService {
    let config = SigningConfig {
        enabled: true,
        mode: Mode::Digest,
        key_path: None,
        key_passphrase: None,
        key_passphrase_file: None,
        verify_on_read: Some(policy),
    };
    let signer = Signer::from_config(&config).expect("signer");
    EhrbaseService::new(pool).with_signer(Arc::new(signer))
}

#[tokio::test]
async fn warn_and_off_verify_on_read_serve_a_tampered_row() {
    // The other two poles of the policy (#273): `warn` logs + meters a mismatch
    // but still serves; explicit `off` never even checks. Neither is a 5xx —
    // together with the strict tests this proves the default is no longer the
    // silent-pass state, and the opt-outs remain reachable.
    for policy in [VerifyOnRead::Warn, VerifyOnRead::Off] {
        let db = testkit::db().await.expect("testkit database");
        let pool = db.pool();
        let svc = service_with_verify(pool.clone(), policy);
        let ehr_id = create_ehr(&svc).await;
        let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

        let ovid = svc
            .create_composition(ehr_uuid, uv(composition("tamper-nonstrict"), "249", None))
            .await
            .expect("create_composition")
            .version_uid();

        sqlx::query(
            "UPDATE vo_version SET signature = 'sha256:dGFtcGVyZWQ=' WHERE kind = 'COMPOSITION'",
        )
        .execute(&pool)
        .await
        .expect("tamper");

        svc.composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
            .await
            .unwrap_or_else(|e| {
                panic!("verify_on_read = {policy:?} must serve, not reject: {e:?}")
            });
    }
}

#[tokio::test]
async fn canonical_xml_carries_the_signature() {
    // The generated canonical-XML serialization (the same `to_canonical_xml` the
    // REST negotiate path uses) emits the `signature` element (design §4.4/§6.4).
    use openehr_rm::common::change_control::original_version::OriginalVersion;
    use openehr_rm::composition::composition::Composition;

    // A full corpus COMPOSITION (all mandatory fields) as the version data.
    const CORPUS: &str = include_str!(
        "../../../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/minimal_persistent.json"
    );
    let data: Composition =
        openehr_its::json::from_canonical_json(CORPUS).expect("typed composition");
    let ov: OriginalVersion<Composition> = openehr_its::json::from_canonical_value(&json!({
        "_type": "ORIGINAL_VERSION",
        "contribution": {
            "_type": "OBJECT_REF", "namespace": "local", "type": "CONTRIBUTION",
            "id": { "_type": "HIER_OBJECT_ID", "value": "0198f4a5-9df1-7d1e-8b6f-2b8c00000abc" }
        },
        "commit_audit": {
            "_type": "AUDIT_DETAILS",
            "system_id": "ehrbase-rs.local",
            "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-07T10:11:12Z" },
            "change_type": change_type("249", "creation"),
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "EHRbase" }
        },
        "uid": {
            "_type": "OBJECT_VERSION_ID",
            "value": "0198f4a5-9df1-7d1e-8b6f-2b8c00000001::ehrbase-rs.local::1"
        },
        "lifecycle_state": change_type("532", "complete"),
        "signature": "sha256:jtWX/CULavvzX0ehjowv2XZPICTQhN1t0+AXHfbEaNc=",
        "data": openehr_its::json::to_canonical_value(&data)
    }))
    .expect("typed ORIGINAL_VERSION");

    let xml =
        openehr_its::xml::to_canonical_xml(&ov, "original_version").expect("to_canonical_xml");
    assert!(
        xml.contains("sha256:jtWX/CULavvzX0ehjowv2XZPICTQhN1t0+AXHfbEaNc="),
        "canonical XML must carry the signature; got:\n{xml}"
    );
    assert!(xml.contains("signature"), "expected a <signature> element");
}

/// A service with server-side signing DISABLED — the common high-throughput
/// config. With signing off the `audit → sign → version`
/// dependency vanishes, so the commit path folds `audit`, `contribution` and
/// `vo_version` into one statement; this test proves the folded path preserves
/// the RM common master06 versioning semantics byte-for-byte and stores no
/// signature.
fn signing_disabled(pool: PgPool) -> EhrbaseService {
    let config = SigningConfig {
        enabled: false,
        mode: Mode::Digest,
        key_path: None,
        key_passphrase: None,
        key_passphrase_file: None,
        verify_on_read: Some(VerifyOnRead::Off),
    };
    let signer = Signer::from_config(&config).expect("disabled signer");
    EhrbaseService::new(pool).with_signer(Arc::new(signer))
}

#[tokio::test]
async fn signing_disabled_folds_commit_and_preserves_master06_semantics() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = signing_disabled(pool.clone());
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    // CREATE → the folded path: audit + contribution + vo_version in one CTE.
    let ovid_v1 = svc
        .create_composition(ehr_uuid, uv(composition("v1"), "249", None))
        .await
        .expect("create_composition")
        .version_uid();
    let (vo_uuid, v1) = version_components(&ovid_v1);
    assert_eq!(v1, "1", "first version is trunk 1 (master06 §Version tree)");

    // The served ORIGINAL_VERSION round-trips: uid stable, no signature, and the
    // server-computed commit instant present (master06 §Committal m3).
    let ov1 = svc
        .composition_original_version(ehr_uuid, ovid_v1.parse().expect("ovid"))
        .await
        .expect("v1 original version");
    assert_eq!(ov1["uid"]["value"], ovid_v1);
    assert!(
        ov1.get("signature").and_then(Value::as_str).is_none(),
        "signing off → no VERSION.signature on the served version"
    );
    assert!(
        ov1["commit_audit"]["time_committed"]["value"]
            .as_str()
            .is_some(),
        "commit_audit.time_committed is the server-computed instant"
    );

    // UPDATE → the folded path with a prior lineage-tip close (v1 → v2).
    let ovid_v2 = svc
        .update_composition(
            ehr_uuid,
            vo_uuid,
            uv(composition("v2"), "251", Some(&ovid_v1)),
        )
        .await
        .expect("update_composition")
        .version_uid();
    let (_, v2) = version_components(&ovid_v2);
    assert_eq!(v2, "2", "second trunk version");
    let latest = svc
        .get_composition_latest(ehr_uuid, vo_uuid)
        .await
        .expect("latest");
    assert_eq!(uid(&latest), ovid_v2, "current version is v2");

    // Exactly one open trunk row (v1 closed, v2 open) and neither is signed —
    // the folded write honours the one-open-row-per-lineage invariant.
    let rows = sqlx::query(
        "SELECT sys_version, signature, upper_inf(sys_period) AS open \
         FROM vo_version WHERE vo_id = $1 AND kind = 'COMPOSITION' ORDER BY sys_version",
    )
    .bind(vo_uuid)
    .fetch_all(&pool)
    .await
    .expect("select vo_version");
    assert_eq!(rows.len(), 2, "two composition versions stored");
    let open: Vec<bool> = rows.iter().map(|r| r.try_get("open").unwrap()).collect();
    assert_eq!(
        open,
        vec![false, true],
        "v1 superseded, v2 open (master06 §Version tree)"
    );
    for row in &rows {
        let sig: Option<String> = row.try_get("signature").unwrap();
        assert!(sig.is_none(), "signing off → vo_version.signature is NULL");
    }

    // DELETE → folded path (523|deleted|, no node rows); the current version
    // then resolves to an empty body (204), never 404.
    let ovid_v2_id: ObjectVersionId = ovid_v2.parse().expect("ovid");
    svc.delete_composition(ehr_uuid, &ovid_v2_id, None)
        .await
        .expect("delete_composition");
    let deleted = svc
        .get_composition_latest(ehr_uuid, vo_uuid)
        .await
        .expect("deleted get");
    assert_eq!(
        deleted,
        Value::Null,
        "a deleted current version reads empty"
    );

    // A multi-change CONTRIBUTION also folds each change (commit_version_into).
    let contribution_uid = svc
        .commit_contribution(
            ehr_uuid,
            vec![uv(composition("Via contribution"), "249", None)],
            contribution_audit("249", "Dr. Contribution"),
        )
        .await
        .expect("commit_contribution");
    let contribution = svc
        .get_contribution(ehr_uuid, contribution_uid.parse().expect("contrib uuid"))
        .await
        .expect("get_contribution");
    let c_ovid = contribution["versions"][0]["id"]["value"]
        .as_str()
        .expect("contribution version uid")
        .to_owned();
    let c_ov = svc
        .composition_original_version(ehr_uuid, c_ovid.parse().expect("ovid"))
        .await
        .expect("contribution version");
    assert!(
        c_ov.get("signature").and_then(Value::as_str).is_none(),
        "signing off → contribution version is unsigned"
    );
}

#[tokio::test]
async fn creating_system_id_and_signature_survive_a_system_id_change() {
    // M2 (RM common master06 §"Distributed Versioning"): the OBJECT_VERSION_ID's
    // creating_system_id is per-version immutable and reconstructed from
    // storage, never from the live service config. A later `with_system_id`
    // change must not mutate a historical version's uid nor invalidate the
    // signature that was computed over it.
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();

    // Commit a composition under system id "sys-origin".
    let svc_a = EhrbaseService::new(pool.clone()).with_system_id("sys-origin");
    let ehr_id = create_ehr(&svc_a).await;
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));
    let ovid = svc_a
        .create_composition(ehr_uuid, uv(composition("v1"), "249", None))
        .await
        .expect("create_composition")
        .version_uid();
    let vo_id = ovid.split("::").next().unwrap().to_owned();
    let vo_uuid = vo_id.parse::<ehrbase::ids::VoId>().expect("vo uuid");
    // The uid's middle part is the creating system id.
    assert_eq!(
        ovid.split("::").nth(1),
        Some("sys-origin"),
        "uid carries the committing system id"
    );

    // A second service over the SAME pool with a DIFFERENT system id.
    let svc_b = EhrbaseService::new(pool.clone()).with_system_id("sys-changed");

    // Reading the composition back: the injected uid still carries "sys-origin"
    // (the stored creating_system_id), not the new config value.
    let read = svc_b
        .get_composition_latest(ehr_uuid, vo_uuid)
        .await
        .expect("composition_get");
    assert_eq!(
        uid(&read),
        ovid,
        "uid must be stable across a system-id change"
    );

    // The served ORIGINAL_VERSION still verifies — the signature was computed
    // over the stored creating_system_id, which the read path reconstructs.
    let ov = svc_b
        .composition_original_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await
        .expect("versioned composition version");
    assert_eq!(ov["uid"]["value"], ovid);
    assert_digest_recomputes(&ov);
}
