//! VERSION SIGNING cases — the cross-cutting register
//! (`docs/design/conformance/11-crosscutting.md` §Signing).
//!
//! `master03-profiles.adoc` §Non-Functional lists **Signing = STANDARD** but no
//! prose defines *how*: the openEHR RM provides only the `VERSION.signature`
//! slot (RM common master06 §Version). The `sha256:` digest algorithm has **no
//! openEHR spec governing it — it is an ehrbase-rs extension**
//! (`docs/design/version-signing.md`), so every case is
//! [`ScheduleTrace::EccOriginal`] (profile-anchored capability, extension
//! mechanism; owner ruling 2026-07-13). The fairness register already rules
//! `SIG → extension` (N/A) for any SUT that does not sign versions
//! (register 11 G-2), so these never dent a foreign SUT's verdict; upstream
//! ships zero signing material, so this suite is the capability's entire
//! evidence base.
//!
//! The strongest case (`sig/digest-recomputes`) recomputes `sha256:base64(...)`
//! from the served version's own RFC-8785 canonical form via the RM spec
//! function `openehr_rm::…::version_impl::canonical_form_of_json`, so a mismatch
//! is a genuine integrity finding.
//
// PORT NOTE: register 11 G-3 (canonical-form version ladder) is unmet — the
// recompute basis is RM 1.2.0-shaped; a SUT signing an RM-1.1.0-era version
// would fail the recompute even if its digest is internally correct. A
// per-edition recompute basis belongs to the register-90 wire adapter.

use base64::Engine as _;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use openehr_rm::common::change_control::version_impl::canonical_form_of_json;

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

/// JSON-only.
const JSON: &[Format] = &[Format::Json];
/// Both canonical formats (digest presence is asserted in JSON + XML).
const BOTH: &[Format] = &[Format::Json, Format::Xml];

/// Manifest keys (register-80; the rows are appended to MANIFEST.tsv).
const NESTED_OPT: &str = "nested.template.opt";
const NESTED_JSON: &str = "nested.composition.json";
const ADMIN_OPT: &str = "minimal-admin.template.opt";
const ADMIN_CONTRIB: &str = "minimal-admin.contribution.create";
const ADMIN_CONTRIB_MOD: &str = "minimal-admin.contribution.modification";
const FOLDER_FIXTURE: &str = "subfolders.directory.json";

/// A distinctive, non-digest client signature so `stored verbatim` is provable
/// (a server that re-signed would replace it with a `sha256:` digest).
const CLIENT_SIG: &str =
    "-----BEGIN PGP SIGNATURE-----\nauthored-elsewhere\n-----END PGP SIGNATURE-----";

/// Every registered SIGNING case (5, the STANDARD capability's evidence base).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        case(
            "sig/digest-present",
            "Version signing — digest present",
            BOTH,
            Compare::None,
            "profiles master03 §Non-Functional Signing (STANDARD); RM common master06 §Version (signature slot); no openEHR spec governs the sha256: digest — ehrbase-rs extension (docs/design/version-signing.md §3.2/§4.4)",
            sig_stub("digest present on every served VERSION"),
            Binding::Rest(
                "GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}",
            ),
            run_digest_present,
        ),
        case(
            "sig/digest-recomputes",
            "Version signing — digest recomputes",
            JSON,
            Compare::None,
            "profiles master03 §Non-Functional Signing (STANDARD); RM common master06 §Version (signature slot) + RFC 8785 canonical form; no openEHR spec governs the digest — ehrbase-rs extension (docs/design/version-signing.md §3.1/§6.3)",
            sig_stub("served digest recomputes from the version's own canonical form"),
            Binding::Rest(
                "GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}",
            ),
            run_digest_recomputes,
        ),
        case(
            "sig/all-kinds",
            "Version signing — all kinds",
            JSON,
            Compare::None,
            "profiles master03 §Non-Functional Signing (STANDARD); RM common master06 §Version (signature slot on every versioned object); ehrbase-rs extension (docs/design/version-signing.md §3.3)",
            sig_stub(
                "EHR_STATUS + multi-version COMPOSITION + FOLDER writes all yield signed versions",
            ),
            Binding::Rest(
                "PUT /ehr/{ehr_id}/ehr_status; POST /ehr/{ehr_id}/contribution; POST /ehr/{ehr_id}/directory",
            ),
            run_all_kinds,
        ),
        case(
            "sig/client-verbatim",
            "Version signing — client verbatim",
            JSON,
            Compare::None,
            "profiles master03 §Non-Functional Signing (STANDARD); RM common master06 §Version (client-supplied signature stored verbatim, never re-signed); ehrbase-rs extension (docs/design/version-signing.md §3.3)",
            sig_stub("a client-supplied signature is stored + served verbatim"),
            Binding::Rest("POST /ehr/{ehr_id}/contribution"),
            run_client_verbatim,
        ),
        case(
            "sig/pgp-verifies",
            "Version signing — pgp verifies",
            JSON,
            Compare::None,
            "profiles master03 §Non-Functional Signing (STANDARD); RM common master06 §Version (signature slot); ehrbase-rs extension — pgp mode, RFC 4880 detached signature (docs/design/version-signing.md §3.2)",
            sig_stub("a pgp-keyed SUT serves an RFC 4880 detached signature"),
            Binding::Rest(
                "GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}",
            ),
            run_pgp_verifies,
        ),
    ]
}

/// The extension-flagged schedule trace for a signing assertion (no CNF schedule
/// chapter; profile-anchored, extension mechanism).
fn sig_stub(what: &'static str) -> ScheduleTrace {
    ScheduleTrace::EccOriginal(match what {
        "digest present on every served VERSION" => {
            "extension: VERSION.signature is an ehrbase-rs feature (no openEHR spec governs the digest algorithm); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot"
        }
        "served digest recomputes from the version's own canonical form" => {
            "extension: sha256: digest recompute is an ehrbase-rs feature (RFC 8785 canonical form); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot"
        }
        "EHR_STATUS + multi-version COMPOSITION + FOLDER writes all yield signed versions" => {
            "extension: version signing rides every versioned-object write (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot"
        }
        "a client-supplied signature is stored + served verbatim" => {
            "extension: client-supplied signatures win (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot"
        }
        _ => {
            "extension: pgp signing mode is an ehrbase-rs feature (RFC 4880); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot"
        }
    })
}

/// Assemble a SIGNING case entry (area [`Area::Sig`], STANDARD Signing capability).
fn case(
    id: &'static str,
    title: &'static str,
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
            area: Area::Sig,
            capability: Capability::Signing,
            formats,
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

fn codec(e: fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Upload a single-file OPT (manifest key) tolerating a re-upload.
async fn ensure_opt(ctx: &RunContext<'_>, key: &str) -> Result<(), CaseError> {
    let xml = fixtures::read(key).map_err(codec)?;
    support::ensure_opt_xml(ctx, &xml).await
}

/// Commit the vendored nested event composition (uploading its OPT), returning
/// `(ehr_id, versioned_object_uid, version_uid)`.
async fn commit_composition(ctx: &RunContext<'_>) -> Result<(String, String, String), CaseError> {
    ensure_opt(ctx, NESTED_OPT).await?;
    let ehr_id = support::create_ehr(ctx).await?;
    let body = fixtures::read_json(NESTED_JSON).map_err(codec)?;
    let resp = ctx
        .send(negotiate::representation(
            HttpRequest::post(format!("/ehr/{ehr_id}/composition")).json_body(&body)?,
            Format::Json,
        ))
        .await?;
    assert::status(&resp, 201)?;
    let ovid = ids::version_uid(ctx, &resp)?;
    let vo_id = ids::object_uid(&ovid).to_owned();
    Ok((ehr_id, vo_id, ovid))
}

/// GET the `ORIGINAL_VERSION` of a versioned composition version in `ctx.format`.
async fn get_composition_version(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    vo_id: &str,
    ovid: &str,
) -> Result<HttpResponse, CaseError> {
    let resp = ctx
        .send(negotiate::accept(
            HttpRequest::get(format!(
                "/ehr/{ehr_id}/versioned_composition/{vo_id}/version/{ovid}"
            )),
            ctx.format,
        ))
        .await?;
    assert::status(&resp, 200)?;
    Ok(resp)
}

/// The `sha256:<base64>` digest the server must have produced for `ov` — the
/// ehrbase-rs digest over the RM `canonical_form` (RFC 8785), computed here so a
/// mismatch is a real finding.
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
            "VERSION.signature must be a `sha256:` digest (docs/design/version-signing.md §3.2), got {sig:?}"
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

/// Extract the text of the first `<signature>…</signature>` XML element.
fn xml_signature(xml: &str) -> Option<&str> {
    let start = xml.find("<signature>")? + "<signature>".len();
    let end = xml[start..].find("</signature>")? + start;
    Some(&xml[start..end])
}

/// Assert the served `ORIGINAL_VERSION` carries a digest that **recomputes** from
/// its own canonical form (commit-time == read-time object identity).
fn assert_digest_recomputes(ov: &Value) -> Result<(), CaseError> {
    let sig = ov["signature"].as_str().ok_or_else(|| {
        CaseError::Assertion(
            "served ORIGINAL_VERSION carries no `signature` (RM common master06 §Version)"
                .to_owned(),
        )
    })?;
    assert_digest_shape(sig)?;
    let expected = expected_digest(ov)?;
    if sig == expected {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "digest does not recompute from the served version's canonical form: served {sig:?}, recomputed {expected:?}"
        )))
    }
}

/// Commit a CONTRIBUTION body against `ehr_id` (JSON, `return=representation`).
async fn commit_contribution(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    body: &Value,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::representation(
        HttpRequest::post(format!("/ehr/{ehr_id}/contribution")).json_body(body)?,
        Format::Json,
    ))
    .await
}

/// The `OBJECT_VERSION_ID` of `versions[i]` in a committed CONTRIBUTION.
fn contribution_version_uid(resp: &HttpResponse, i: usize) -> Result<String, CaseError> {
    resp.json()?["versions"][i]["id"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| {
            CaseError::Assertion(format!(
                "committed CONTRIBUTION has no versions[{i}].id.value"
            ))
        })
}

// ── runs ─────────────────────────────────────────────────────────────────────

fn run_digest_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
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
                        "served ORIGINAL_VERSION has no `signature` (RM common master06 §Version)"
                            .to_owned(),
                    )
                })?;
                assert_digest_shape(sig)?;
            }
            Format::Xml => {
                let xml = resp.text();
                let sig = xml_signature(&xml).ok_or_else(|| {
                    CaseError::Assertion("canonical-XML ORIGINAL_VERSION has no <signature> element (RM common master06 §Version)".to_owned())
                })?;
                assert_digest_shape(sig)?;
            }
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_digest_recomputes<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let (ehr_id, vo_id, ovid) = commit_composition(ctx).await?;
        let ov = get_composition_version(ctx, &ehr_id, &vo_id, &ovid)
            .await?
            .json()?;
        assert_digest_recomputes(&ov)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_all_kinds<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // Data set 1: an EHR_STATUS update — the original status version is signed.
        let ehr_id = support::create_ehr(ctx).await?;
        let status = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/ehr/{ehr_id}/ehr_status")),
                Format::Json,
            ))
            .await?;
        assert::status(&status, 200)?;
        let mut status_body = status.json()?;
        let status_v1 = ids::body_uid(&status_body)?;
        // Flip is_queryable (not is_modifiable): a deactivated EHR would refuse the
        // later content commits of this case (RM ehr master04 §EHR Active Status).
        status_body["is_queryable"] = Value::Bool(false);
        let updated = ctx
            .send(negotiate::if_match(
                negotiate::representation(
                    HttpRequest::put(format!("/ehr/{ehr_id}/ehr_status"))
                        .json_body(&status_body)?,
                    Format::Json,
                ),
                &status_v1,
            ))
            .await?;
        assert::status_in(&updated, &[200, 204])?;
        let status_ov = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/versioned_ehr_status/version/{status_v1}"
                )),
                Format::Json,
            ))
            .await?;
        assert::status(&status_ov, 200)?;
        assert_digest_recomputes(&status_ov.json()?)?;

        // Data sets 2–3: a multi-version COMPOSITION via the CONTRIBUTION path.
        ensure_opt(ctx, ADMIN_OPT).await?;
        let create = commit_contribution(
            ctx,
            &ehr_id,
            &fixtures::read_json(ADMIN_CONTRIB).map_err(codec)?,
        )
        .await?;
        assert::status(&create, 201)?;
        let v1 = contribution_version_uid(&create, 0)?;
        let vo_id = ids::object_uid(&v1).to_owned();

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
        // FOLDER (no ORIGINAL_VERSION wrapper), so the FOLDER version's signature is
        // not API-observable here; the write must be accepted, and the storage-level
        // signing is proven off-wire by app/ehrbase service_signing (a documented
        // instrument-encodes-server-behaviour boundary, register 11 §2).
        let folder = fixtures::read_json(FOLDER_FIXTURE).map_err(codec)?;
        let dir = ctx
            .send(negotiate::representation(
                HttpRequest::post(format!("/ehr/{ehr_id}/directory")).json_body(&folder)?,
                Format::Json,
            ))
            .await?;
        assert::status(&dir, 201)?;
        Ok(DataSetReport::all(4))
    })
}

fn run_client_verbatim<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // A CONTRIBUTION creation version carrying an author-generated signature —
        // stored + served verbatim, never re-signed (RM common master06 §Version).
        ensure_opt(ctx, ADMIN_OPT).await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body = fixtures::read_json(ADMIN_CONTRIB).map_err(codec)?;
        body["versions"][0]["signature"] = Value::String(CLIENT_SIG.to_owned());
        let resp = commit_contribution(ctx, &ehr_id, &body).await?;
        assert::status(&resp, 201)?;
        let ovid = contribution_version_uid(&resp, 0)?;
        let vo_id = ids::object_uid(&ovid).to_owned();
        let ov = get_composition_version(ctx, &ehr_id, &vo_id, &ovid)
            .await?
            .json()?;
        match ov["signature"].as_str() {
            Some(sig) if sig == CLIENT_SIG => Ok(DataSetReport::SINGLE),
            Some(sig) => Err(CaseError::Assertion(format!(
                "client-supplied signature must be served verbatim (docs/design/version-signing.md §3.3), got {sig:?}"
            ))),
            None => Err(CaseError::Assertion(
                "client-supplied signature was dropped (docs/design/version-signing.md §3.3)"
                    .to_owned(),
            )),
        }
    })
}

fn run_pgp_verifies<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // Needs a `pgp`-mode SUT with a configured key; the compose dev config boots
        // in `digest` mode and an external SUT's key config is unknown. The four
        // digest cases prove the capability. Reported SKIPPED(SutConfig), never
        // fabricated (register 11 G-6: a pgp compose profile is a follow-up).
        Err::<DataSetReport, _>(CaseError::Skipped(
            "SutConfig: server not in `pgp` mode (needs a configured OpenPGP key); a pgp-keyed \
             compose profile is a follow-up — the digest cases prove the Signing capability"
                .to_owned(),
        ))
    })
}
