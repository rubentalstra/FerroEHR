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
//! (`Requests_and_responses.md` §Deprecated headers) — implemented new-form-
//! first with the deprecated forms still accepted/emitted where the spec says
//! MAY. Status below is *descriptive* of what this layer does today (the gap
//! register in `docs/design/its-rest/overview.md` §2 tracks the source rows):
//!
//! - **G-1 ETag weakness indicator — DONE.** Every resource-identifier `ETag`
//!   is emitted as the weak `W/"{uid}"` form ([`negotiate::resource_etag`],
//!   used by [`negotiate::set_versioning_headers`] and the template path).
//!   Inbound `If-Match` accepts both the weak and the deprecated bare quoted
//!   forms ([`version_id::strip_etag`]).
//! - **G-2/G-3 committal headers — DONE.** The development-edition value forms
//!   `openehr-version: lifecycle_state.code_string="…"` and
//!   `openehr-audit-details: change_type.code_string="…"` / `description.value`
//!   / `committer.*` / `system_id` are parsed and merged
//!   ([`committal::merge_committal_headers`]), repeated headers included; the
//!   deprecated dotted-*name* forms still work, and the new form wins on
//!   conflict. A client-supplied `system_id` is carried into
//!   `UpdateAudit.system_id`; when absent "the server MUST set it to its own
//!   configured system identifier" — asserted at the versioning seam.
//! - **G-4 Location — DONE.** `Location` is set only on create/update writes
//!   ([`negotiate::set_resource_headers`]); reads, deletes, and the `409`/`412`
//!   error path emit versioning headers without `Location`
//!   ([`negotiate::set_versioning_headers`]).
//! - **G-5 `return=identifier` — PARTIAL.** [`negotiate::write_rm`] /
//!   [`negotiate::write_json`] honour `return=identifier` with a
//!   `{ "uid": … }` body at a `200`/`201` status (never `204`). Per-API bodies
//!   that the OAS shapes differently are wired at the dispatch call sites
//!   (TODO(w3e-integrate) there).
//! - **G-6 `Preference-Applied` — DONE.** Emitted on write responses echoing
//!   the honoured `return=` preference ([`negotiate`], a MAY).
//! - **G-7 item-tag headers — PARTIAL.** Parse/emit helpers exist
//!   ([`params::parse_item_tag_header`] / [`params::emit_item_tag_header`]);
//!   the dispatch wiring to the ITEM_TAG service is a
//!   TODO(w3e-integrate) in [`params`].
//! - **G-10 method status — PARTIAL.** [`error::method_not_allowed_handler`]
//!   (`405`) and [`error::not_implemented_handler`] (`501`) render the openEHR
//!   body; mounting them on the router is a TODO(w3e-integrate) in [`error`].
//! - **G-8 version identity / G-9 `openehr-uri`** are out of this change's
//!   scope (tracked in the register); `/status` still reports the
//!   Release-1.0.3 label and `openehr-uri` is not emitted.
//!
//! Deferred cross-folder wiring (left as `// TODO(w3e-integrate)` notes in this
//! module's files): item-tag dispatch (`params`), `405`/`501` router mount
//! (`error`), per-API identifier bodies, and — a compile follow-up of the new
//! `UpdateAudit.system_id` field — the `UpdateAudit { … }` literal in
//! `api/ehr/dispatch.rs` gains `system_id: None`.
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
