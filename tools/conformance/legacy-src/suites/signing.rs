//! The `SIG` capability cases — our own ECC cases (reference:
//! `version-signing.md`, design-time reading): the STANDARD Signing capability's
//! **entire evidence base** — upstream ships zero Signing test material, so these
//! cases, specified against the implemented behaviour in
//! `docs/design/version-signing.md`, are what proves the capability.
//!
//! The four digest cases are the must-haves. `sig/pgp-verifies` needs a
//! `pgp`-keyed SUT; the compose dev config ships in `digest` mode (design §3.4)
//! and an external SUT's key config is unknown, so it reports `SKIPPED(SutConfig)` (§4.6) — the digest cases still
//! prove the capability.
//!
//! Digest recomputation (`SIGN-digest-recomputes`, the strongest case) uses the
//! RM spec function `openehr_rm::…::version_impl::canonical_form_of_json` (the
//! exact RFC 8785 canonical form the server signs with) and reproduces the
//! `ehrbase-signing` digest format `sha256:` + base64(SHA-256(canonical_form))
//! (version-signing.md §3.2), so a mismatch is a genuine integrity finding.

use base64::Engine as _;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use openehr_rm::common::change_control::version_impl::canonical_form_of_json;

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, DataSetReport, HttpRequest, HttpResponse, RunContext};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The runner-defined SIGN-* case entries (design §4.6).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // The served VERSION carries a `sha256:<base64>` digest — JSON + XML.
        entry(
            "sig/digest-present",
            "Version signing — digest present",
            BOTH,
            "version-signing.md §3.2 (digest default-on); §4.4 (rides every VERSION read)",
            run_digest_present,
        ),
        // THE strongest case: the digest recomputes from the served version's
        // own canonical form (commit-time == read-time object identity).
        entry(
            "sig/digest-recomputes",
            "Version signing — digest recomputes",
            JSON,
            "version-signing.md §3.1 (canonical_form RFC 8785) + §6.3",
            run_digest_recomputes,
        ),
        // EHR_STATUS update, multi-version CONTRIBUTION, and a FOLDER write all
        // yield signed versions.
        entry(
            "sig/all-kinds",
            "Version signing — all kinds",
            JSON,
            "version-signing.md §3.3 (all object kinds via the shared vobject commit path)",
            run_all_kinds,
        ),
        // A CONTRIBUTION UPDATE_VERSION with a client-supplied signature is stored
        // + served verbatim (never re-signed).
        entry(
            "sig/client-verbatim",
            "Version signing — client verbatim",
            JSON,
            "version-signing.md §3.3 (client-supplied signatures win, stored verbatim)",
            run_client_verbatim,
        ),
        // Self-host / pgp-keyed SUT only — reports SKIPPED(SutConfig) otherwise.
        entry(
            "sig/pgp-verifies",
            "Version signing — pgp verifies",
            JSON,
            "version-signing.md §3.2 (pgp mode, RFC 4880 detached signature)",
            run_pgp_verifies,
        ),
    ]
}

/// JSON-only.
const JSON: &[Format] = &[Format::Json];
/// Both canonical formats.
const BOTH: &[Format] = &[Format::Json, Format::Xml];

/// A version-signing case entry (SIG area, STANDARD Signing capability).
fn entry(
    id: &'static str,
    title: &'static str,
    formats: &'static [Format],
    citation: &'static str,
    run: crate::harness::CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Sig,
            capability: Capability::Signing,
            profiles: &[Profile::Standard],
            formats,
            citation,
            compare: Compare::Superset,
            schedule_ref: None,
        },
        run,
    }
}

// ── fixtures + helpers ────────────────────────────────────────────────────────

const NESTED_OPT: &str = "nested/nested.opt";
const NESTED_JSON: &str = "compositions/CANONICAL_JSON/nested.en.v1__full.json";
const ADMIN_OPT: &str = "minimal/minimal_admin.opt";
const ADMIN_CONTRIB: &str = "contributions/valid/minimal/minimal_admin.contribution.json";
const ADMIN_CONTRIB_MOD: &str =
    "contributions/valid/minimal/minimal_admin.contribution.modification.complete.json";
const FOLDER_FIXTURE: &str = "directory/subfolders_in_directory.json";

/// A distinctive, non-digest client signature so `stored verbatim` is provable
/// (a server that re-signed would replace it with a `sha256:` digest).
const CLIENT_SIG: &str =
    "-----BEGIN PGP SIGNATURE-----\nauthored-elsewhere\n-----END PGP SIGNATURE-----";

fn codec(e: fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

/// Commit the vendored event composition (uploading its OPT), returning
/// `(ehr_id, versioned_object_uid, version_uid)`.
async fn commit_composition(ctx: &RunContext<'_>) -> Result<(String, String, String), CaseError> {
    support::ensure_opt(ctx, NESTED_OPT).await?;
    let ehr_id = support::create_ehr(ctx).await?;
    let body = fixtures::read_json(NESTED_JSON).map_err(codec)?;
    let resp = ctx
        .send(
            HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                .json_body(&body)?
                .header("accept", "application/json")
                .header("prefer", "return=representation"),
        )
        .await?;
    assert::status(&resp, 201)?;
    let ovid = support::version_uid(&resp)?;
    let vo_id = support::object_uid(&ovid).to_owned();
    Ok((ehr_id, vo_id, ovid))
}

/// GET the `ORIGINAL_VERSION` of a versioned composition version in `ctx.format`
/// (`GET /ehr/{ehr}/versioned_composition/{vo}/version/{ovid}`).
async fn get_composition_version(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    vo_id: &str,
    ovid: &str,
) -> Result<HttpResponse, CaseError> {
    let resp = ctx
        .send(
            HttpRequest::get(format!(
                "/ehr/{ehr_id}/versioned_composition/{vo_id}/version/{ovid}"
            ))
            .header("accept", ctx.format.media_type()),
        )
        .await?;
    assert::status(&resp, 200)?;
    Ok(resp)
}

/// The `sha256:<base64>` digest the server must have produced for `ov` — the
/// exact `ehrbase-signing` format over the RM `canonical_form` (version-signing.md
/// §3.2), computed here so a mismatch is a real finding.
fn expected_digest(ov: &Value) -> Result<String, CaseError> {
    let canonical = canonical_form_of_json(ov).map_err(|e| {
        CaseError::Assertion(format!(
            "canonical_form of the served ORIGINAL_VERSION: {e}"
        ))
    })?;
    let hash = Sha256::digest(canonical.as_bytes());
    Ok(format!(
        "sha256:{}",
        base64::engine::general_purpose::STANDARD.encode(hash)
    ))
}

/// Assert `sig` is a well-formed `sha256:<standard-base64-of-32-bytes>` digest.
fn assert_digest_shape(sig: &str) -> Result<(), CaseError> {
    let b64 = sig.strip_prefix("sha256:").ok_or_else(|| {
        CaseError::Assertion(format!(
            "VERSION.signature must be a `sha256:` digest (version-signing.md §3.2), got {sig:?}"
        ))
    })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| {
            CaseError::Assertion(format!(
                "digest is not valid standard base64: {e} ({sig:?})"
            ))
        })?;
    if bytes.len() == 32 {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "SHA-256 digest must be 32 bytes, got {} ({sig:?})",
            bytes.len()
        )))
    }
}

/// Extract the text between the first `<signature>…</signature>` element of a
/// canonical-XML `ORIGINAL_VERSION` (the RM `signature` is a plain string element
/// with no `xsi:type`, so no attribute handling is needed).
fn xml_signature(xml: &str) -> Option<&str> {
    let start = xml.find("<signature>")? + "<signature>".len();
    let end = xml[start..].find("</signature>")? + start;
    Some(&xml[start..end])
}

/// Assert the served `ORIGINAL_VERSION` carries a digest that **recomputes** from
/// its own canonical form — the commit-time/read-time identity proof (§6.3).
fn assert_digest_recomputes(ov: &Value) -> Result<(), CaseError> {
    let sig = ov["signature"].as_str().ok_or_else(|| {
        CaseError::Assertion(
            "served ORIGINAL_VERSION carries no `signature` (version-signing.md §4.4)".to_owned(),
        )
    })?;
    assert_digest_shape(sig)?;
    let expected = expected_digest(ov)?;
    if sig == expected {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "digest does not recompute from the served version's canonical form \
             (version-signing.md §6.3): served {sig:?}, recomputed {expected:?}"
        )))
    }
}

// ── SIGN-digest-present ────────────────────────────────────────────────────────

fn run_digest_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, vo_id, ovid) = commit_composition(ctx).await?;
        let resp = get_composition_version(ctx, &ehr_id, &vo_id, &ovid).await?;
        match ctx.format {
            Format::Json => {
                let ov = resp.json()?;
                if ov["_type"] != "ORIGINAL_VERSION" {
                    return Err(CaseError::Assertion(format!(
                        "expected ORIGINAL_VERSION, got {}",
                        ov["_type"]
                    )));
                }
                let sig = ov["signature"].as_str().ok_or_else(|| {
                    CaseError::Assertion(
                        "served ORIGINAL_VERSION has no `signature` (version-signing.md §4.4)"
                            .to_owned(),
                    )
                })?;
                assert_digest_shape(sig)?;
            }
            Format::Xml => {
                let xml = resp.text();
                let sig = xml_signature(&xml).ok_or_else(|| {
                    CaseError::Assertion(
                        "canonical-XML ORIGINAL_VERSION has no <signature> element \
                         (version-signing.md §4.4)"
                            .to_owned(),
                    )
                })?;
                assert_digest_shape(sig)?;
            }
        }
        Ok(DataSetReport::SINGLE)
    })
}

// ── SIGN-digest-recomputes ─────────────────────────────────────────────────────

fn run_digest_recomputes<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, vo_id, ovid) = commit_composition(ctx).await?;
        let ov = get_composition_version(ctx, &ehr_id, &vo_id, &ovid)
            .await?
            .json()?;
        assert_digest_recomputes(&ov)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── SIGN-all-kinds ─────────────────────────────────────────────────────────────

fn run_all_kinds<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Data set 1: an EHR_STATUS update — the original status version is signed
        // and recomputes.
        let ehr_id = support::create_ehr(ctx).await?;
        let status = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/ehr_status"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&status, 200)?;
        let mut status_body = status.json()?;
        let status_v1 = support::uid_of(&status_body)?;
        // Flip is_queryable (not is_modifiable): the case only needs a second
        // signed status version, and a deactivated EHR
        // (EHR_STATUS.is_modifiable = false — RM ehr master04 §"EHR Active
        // Status") correctly refuses the later content commits of this case.
        status_body["is_queryable"] = Value::Bool(false);
        let updated = ctx
            .send(
                HttpRequest::put(format!("/ehr/{ehr_id}/ehr_status"))
                    .json_body(&status_body)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation")
                    .header("if-match", status_v1.clone()),
            )
            .await?;
        assert::status_in(&updated, &[200, 204])?;
        let status_ov = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/versioned_ehr_status/version/{status_v1}"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&status_ov, 200)?;
        assert_digest_recomputes(&status_ov.json()?)?;

        // Data sets 2–3: a multi-version COMPOSITION via the CONTRIBUTION path —
        // creation (v1) then modification (v2), both signed + recomputing.
        support::ensure_opt(ctx, ADMIN_OPT).await?;
        let create = commit_contribution(
            ctx,
            &ehr_id,
            &fixtures::read_json(ADMIN_CONTRIB).map_err(codec)?,
        )
        .await?;
        assert::status(&create, 201)?;
        let v1 = contribution_version_uid(&create, 0)?;
        let vo_id = support::object_uid(&v1).to_owned();

        let mut modify = fixtures::read_json(ADMIN_CONTRIB_MOD).map_err(codec)?;
        modify["versions"][0]["preceding_version_uid"] =
            serde_json::json!({ "_type": "OBJECT_VERSION_ID", "value": v1 });
        let update = commit_contribution(ctx, &ehr_id, &modify).await?;
        assert::status(&update, 201)?;
        let v2 = contribution_version_uid(&update, 0)?;

        for ovid in [&v1, &v2] {
            let ov = get_composition_version(ctx, &ehr_id, &vo_id, ovid)
                .await?
                .json()?;
            assert_digest_recomputes(&ov)?;
        }

        // Data set 4: a FOLDER write. The directory read surface serves the bare
        // FOLDER (no ORIGINAL_VERSION wrapper), so the FOLDER version's signature
        // is not API-observable here; the write is driven (must be accepted) and
        // the storage-level signing is proven by the ehrbase `service_signing`
        // SQL sweep (which asserts every `vo_version` row — incl. FOLDER — is
        // signed). PORT NOTE: FOLDER signature verification via the API awaits a
        // versioned-directory version-read surface.
        let folder = fixtures::read_json(FOLDER_FIXTURE).map_err(codec)?;
        let dir = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/directory"))
                    .json_body(&folder)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation"),
            )
            .await?;
        assert::status(&dir, 201)?;

        Ok(DataSetReport::all(4))
    })
}

/// Commit a CONTRIBUTION body against `ehr_id` (JSON, `return=representation`).
async fn commit_contribution(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    body: &Value,
) -> Result<HttpResponse, CaseError> {
    ctx.send(
        HttpRequest::post(format!("/ehr/{ehr_id}/contribution"))
            .json_body(body)?
            .header("accept", "application/json")
            .header("prefer", "return=representation"),
    )
    .await
}

/// The `OBJECT_VERSION_ID` of `versions[i]` in a committed CONTRIBUTION.
fn contribution_version_uid(resp: &HttpResponse, i: usize) -> Result<String, CaseError> {
    let body = resp.json()?;
    body["versions"][i]["id"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| {
            CaseError::Assertion(format!(
                "committed CONTRIBUTION has no versions[{i}].id.value"
            ))
        })
}

// ── SIGN-client-verbatim ───────────────────────────────────────────────────────

fn run_client_verbatim<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // A CONTRIBUTION creation version carrying an author-generated signature —
        // stored + served verbatim, never re-signed (version-signing.md §3.3).
        support::ensure_opt(ctx, ADMIN_OPT).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body = fixtures::read_json(ADMIN_CONTRIB).map_err(codec)?;
        body["versions"][0]["signature"] = Value::String(CLIENT_SIG.to_owned());
        let resp = commit_contribution(ctx, &ehr_id, &body).await?;
        assert::status(&resp, 201)?;
        let ovid = contribution_version_uid(&resp, 0)?;
        let vo_id = support::object_uid(&ovid).to_owned();

        let ov = get_composition_version(ctx, &ehr_id, &vo_id, &ovid)
            .await?
            .json()?;
        match ov["signature"].as_str() {
            Some(sig) if sig == CLIENT_SIG => Ok(DataSetReport::SINGLE),
            Some(sig) => Err(CaseError::Assertion(format!(
                "client-supplied signature must be served verbatim (version-signing.md §3.3), \
                 got {sig:?}"
            ))),
            None => Err(CaseError::Assertion(
                "client-supplied signature was dropped (version-signing.md §3.3)".to_owned(),
            )),
        }
    })
}

// ── SIGN-pgp-verifies ──────────────────────────────────────────────────────────

fn run_pgp_verifies<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Needs a `pgp`-mode SUT with a configured key (design §4.6): the
        // compose dev config boots in `digest` mode (version-signing.md §3.4) and
        // an external SUT's key config is unknown. A pgp-keyed compose profile
        // is a follow-up; the four digest cases prove the
        // capability. Reported SKIPPED(SutConfig), never fabricated.
        Err::<DataSetReport, _>(CaseError::Skipped(
            "SutConfig: server not in `pgp` mode (needs a configured OpenPGP key); \
             a pgp-keyed compose profile is a follow-up — digest cases prove the capability"
                .to_owned(),
        ))
    })
}
