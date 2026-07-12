//! The ITS-REST **Overview** specification — the cross-cutting protocol every
//! openEHR REST API inherits.
//!
//! Oracle: `docs/specs/openehr/ITS-REST/specifications/docs/overview/`
//! (`Intro.md`, `Glossary_and_conventions.md`, `Resources.md`,
//! `Requests_and_responses.md`) at the **development edition** (`e8a093e9`,
//! the same commit as the vendored OAS — one identity for prose + machine
//! contract), plus the shared OAS pieces
//! (`…/specifications/{headers,parameters,responses}/`).
//!
//! ## The development-edition protocol deltas vs Release-1.0.3
//!
//! (`Requests_and_responses.md` §Deprecated headers) — implemented as
//! new-form-first with the deprecated forms still accepted/emitted where the
//! spec says MAY:
//! - custom headers are lowercase: `openehr-version`, `openehr-audit-details`
//!   (now also carrying `system_id`; `time_committed` "is always set by the
//!   server"), `openehr-template-id`, `openehr-ehr-id`, `openehr-uri` — the
//!   `openEHR-*` spellings are deprecated but remain for backward
//!   compatibility;
//! - `ETag` "MUST include a weakness indicator `W/`" (the bare quoted form
//!   is deprecated; implementations "MAY still support it");
//! - `Location` "MUST ONLY be used for resource creation (e.g. `201
//!   Created`) or redirect responses" — its old use on `GET`/`DELETE`
//!   responses is deprecated;
//! - when a service expects `If-Match` and the client omits it, it "SHOULD
//!   respond with `400 Bad Request`";
//! - new `openehr-item-tag` / `openehr-version-item-tag` headers wrap the
//!   ITEM_TAG operations for VERSIONED_OBJECT / VERSION targets;
//! - clients are steered to always send `Prefer` explicitly (a future
//!   `Prefer=identifier` default is signalled).
//!
//! ## Module map (file ↦ governing sections)
//!
//! - [`negotiate`] — data representation + content negotiation
//!   (`Resources.md` §Data representation: canonical XML/JSON MUSTs, the
//!   `415`/`406` discipline, the Simplified-Format media types
//!   `application/openehr.wt.{flat,structured}+json`;
//!   `Requests_and_responses.md` §Representation details negotiation:
//!   `Prefer: return=minimal|return=representation` — minimal the current
//!   default — and `Prefer: resolve_refs`).
//! - [`committal`] — the committal metadata request headers
//!   (`Requests_and_responses.md` §openehr-version and openehr-audit-details:
//!   services MUST accept them; "whatever is provided it MUST be merged with
//!   the default VERSION and VERSION.audit_details attributes on commit";
//!   direct `PUT`/`POST`/`DELETE` on change-controlled resources "MUST
//!   internally be executed using the 'native' way" — a CONTRIBUTION; when
//!   `system_id` is absent "the server MUST set it to its own configured
//!   system identifier").
//! - [`error`] — the HTTP status-code table (`Requests_and_responses.md`
//!   §HTTP status codes: 200/201/204/400/401/403/404/405/406/408/409/412/
//!   415/422/500/501; unrecognized method → `501`, known-but-not-allowed →
//!   `405`) + the optional error body ("if `Prefer: return=representation`")
//!   + the single SM → HTTP mapping table (`CALL_STATUS_TYPE` meets the
//!   wire here and only here).
//! - [`version_id`] — resource identification (`Resources.md` §Resource
//!   identification: `versioned_object_uid` HIER_OBJECT_ID vs `version_uid`
//!   OBJECT_VERSION_ID `object_id::creating_system_id::version_tree_id`,
//!   `uid_based_id` dual addressing) and the `If-Match` discipline
//!   (§If-Match: on a false condition "MUST NOT perform the requested
//!   method … MUST respond with `412 Precondition Failed`, and SHOULD
//!   return also latest `version_uid` in the `ETag` response headers";
//!   expected-but-missing → `400`).
//! - [`params`] — common parameters (`Glossary_and_conventions.md`:
//!   `version_at_time` extended ISO 8601; `Resources.md` §Datetime format:
//!   temporal query/path values "MUST always use the extended ISO 8601
//!   format").
//! - [`status`] — response headers (`Requests_and_responses.md` §Location:
//!   201-only; §ETag and Last-Modified: weak `W/` quoted resource
//!   identifier + `Last-Modified` from
//!   `VERSION.commit_audit.time_committed.value`, both SHOULD be present on
//!   VERSION/VERSIONED_OBJECT responses; §openehr-uri MAY).
//!
//! Auth is out of band (`Requests_and_responses.md` §Authentication and
//! authorization: no scheme mandated; `401`/`403`/`407` +
//! `WWW-Authenticate` discipline when a framework is present) — the authn
//! stack lives with the extension surface (`crate::access`), not here.

pub mod committal;
pub mod error;
pub mod negotiate;
pub mod params;
pub mod status;
pub mod version_id;
