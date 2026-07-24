# Endpoint → function-chain map

The standing navigation + optimization instrument (owner-mandated,
2026-07-16): every HTTP operation and every background path of the server,
traced from the route mount through the dispatcher, the concrete
`EhrbaseService` method, the versioning layer, down to the actual SQL
statements — with happy-path round-trip counts and every per-item (N+1) loop
named. Derived from the code on `claude/w14-audit` after the 2026-07-16
platform rewrite; regenerate (agent fleet, one section per API family) after
any structural change. Spec citations reference `docs/specs/openehr/`.

Counting convention (all sections): `BEGIN`/`COMMIT` and the config-gated
event-outbox `INSERT` are excluded from round-trip counts; a "full version
read" = 2 statements (the `vo_version⋈audit` row with attestations folded in
via LATERAL, plus the node fetch).

---

# Endpoint → function-chain map — EHR API group (33 operations)

All paths are relative to the configured base path (default `/ehrbase/rest/openehr/v1`).
File paths: handlers under `app/ehrbase-rest/src/`, service + storage under `app/ehrbase/src/`.

## Shared spine (runs for every operation below)

Every EHR-group operation is a `#[utoipa::path]` handler in
`api/ehr/openapi_routes.rs` that snapshots the whole request via
`api/mod.rs::into_parts` (path params, raw query, headers, body bytes) and runs it
through `api/mod.rs::guarded_dispatch`: (1) `extensions/access/ehr_access.rs::enforce`
— the per-EHR `EHR_ACCESS` gate (RM ehr `ehr_access.adoc`), reading
`EhrbaseService::current_ehr_access_settings`, a moka single-flight cache (DB load
only on a cache miss; prewarmed on EHR create) plus the composition privacy ceiling
on composition reads; (2) `extensions/access/pep.rs::pre_check` — the ABAC PEP
pre-check (inert unless an authz policy is wired); (3) the group dispatcher
`api/ehr/dispatch.rs::dispatch` → the owning resource module's `run` (one module per
spec resource: `ehr_resource` / `ehr_status` / `versioned_ehr_status` / `composition`
/ `versioned_composition` / `directory` / `contribution`); (4) `pep::post_check` (may
replace a success with 403/500); (5) the response is tagged with `AuditOpId` for the
ATNA audit layer. Around the whole API subtree (`router.rs`, inner→outer): the auth
middleware (Basic/Bearer → principal task-local; tenant middleware only when tenancy
is on) → ATNA audit middleware (`system_log/middleware.rs`, early-return when
auditing is off) → HTTP metrics + root span → overload shedding (503 above
`max_in_flight`) → the shared tower-http stack (request-id, trace, catch-panic, CORS,
16 MiB body limit, 30 s timeout, compression), with a `405` fallback in the openEHR
error-body shape. Inside each arm: `params::build::<*Params>` rebuilds the generated
contract struct (`openehr_its::rest::generated::ehr`), wire ids decode via
`overview/version_id.rs`, RM bodies decode via `negotiate::rm_value` (canonical JSON
or XML), and responses render through the `negotiate::*` helpers (content
negotiation, `ETag`/`Location`/`Last-Modified`, `Prefer: return=minimal|representation`
per ITS-REST). Writes build the SM `UPDATE_VERSION` envelope via
`api/ehr/mod.rs::mk_update_version` (committer from the authenticated principal,
verb-derived change type, lifecycle `532|complete|`), merged with any
`openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal request headers
(`overview/committal.rs::merge_committal_headers` — the ITS-REST MUST), and may
carry the `openehr-item-tag` / `openehr-version-item-tag` write-wrapper headers
(`api/ehr/mod.rs::apply_item_tag_headers`, an extra tag-replace after the commit,
echoed on the response).

**Counting conventions.** Every content write is ONE `sqlx` transaction;
`BEGIN`/`COMMIT` are not counted as statements. When the event outbox is enabled,
each commit adds exactly one `INSERT event_outbox` inside the transaction (not
counted below). A "full version read" is 2 round trips:
`SELECT vo_version ⋈ audit` (attestations folded in via LATERAL `jsonb_agg`) +
`SELECT node` rows ordered by `num`, reassembled in-process
(`storage/version_repo/read.rs` + `storage/node_repo.rs::read_version_canonical`).
The `VERSION.signature` is computed in-process before any insert (signing enabled)
and verified in-process on `ORIGINAL_VERSION` reads — no extra SQL either way.
`audit` + `contribution` + `vo_version` always land as ONE data-modifying CTE
(`storage/version_repo/commit.rs::commit_new_version` / `commit_version_into`);
node rows are one bulk `INSERT` (`node_repo::write_nodes`, skipped when empty).

---

## EHR resource

### GET /ehr
chain: handler `api/ehr/openapi_routes.rs::ehr_get_by_subject` → `api/ehr/ehr_resource.rs::run` → service `EhrbaseService::ehr_object_for_subject` → `ehr_by_subject` (service/ehr/service.rs) → storage `ehr_repo::ehr_id_by_subject` → `ehr_summary` → `ehr_repo::ehr_summary_read`
sql: 2 round trips — SELECT ehr id by promoted `subject_id`/`subject_namespace` columns (unique, one EHR per subject); SELECT ehr ⋈ LATERAL(current EHR_STATUS identity, EHR_ACCESS vo, live folder ids by rank)
notes: 404 when no EHR names the subject. Body is the canonical RM `EHR` (status ref carries the current `OBJECT_VERSION_ID`; `directory` = `folders[1]`, RM ehr `Directory_in_folders`). `200_EHR` declares no ETag/Location.

### POST /ehr
chain: handler `openapi_routes.rs::ehr_create` → `ehr_resource.rs::run` → `negotiate::optional_rm_value::<EhrStatus>` → service `EhrbaseService::create_ehr` → `commit_new_ehr` (service/ehr/service.rs) → `versioning::change::commit_contribution` (EHR_STATUS + EHR_ACCESS creates under ONE CONTRIBUTION, RM ehr master04 §EHR Creation) → storage `ehr_repo::insert_ehr` + `version_repo::commit::{write_contribution, commit_version_into}` + `node_repo::write_nodes`
sql: 6 round trips in one tx — INSERT ehr (RETURNING time_created, ON CONFLICT (id) DO NOTHING; promoted subject/is_queryable/is_modifiable columns folded in); CTE INSERT audit+contribution; then per created object (EHR_STATUS, EHR_ACCESS): CTE INSERT audit+vo_version, bulk INSERT node (1 row each)
notes: server id is `uuid::Uuid::now_v7()`; supplied EHR_STATUS is structurally validated in-memory first (`validation::validate_ehr_status`). 409 on id conflict or `uq_ehr_subject` (one EHR per subject, CNF `create_ehr-two_ehrs_same_patient`). The `EHR` wire body is assembled from the `Committed` results (zero re-reads) and stashed in the `created_ehr_repr` moka cache, so `Prefer: return=representation` (`ehr_created_object`) is served without SQL; the EHR_ACCESS settings cache is prewarmed. `201` with `ETag(ehr_id)` + `Location`.

### GET /ehr/{ehr_id}
chain: handler `openapi_routes.rs::ehr_get_by_id` → `ehr_resource.rs::run` → service `EhrbaseService::ehr_object` → `ehr_summary` → storage `ehr_repo::ehr_summary_read`
sql: 1 round trip — SELECT ehr ⋈ LATERAL(status identity, access vo, live folders)
notes: same body/no-header shape as GET /ehr.

### PUT /ehr/{ehr_id}
chain: handler `openapi_routes.rs::ehr_create_with_id` → `ehr_resource.rs::run` → service `EhrbaseService::create_ehr_with_id` → `commit_new_ehr` → (identical to POST /ehr from here)
sql: 6 round trips in one tx (as POST /ehr)
notes: an existing id → `insert_ehr` returns None → 409. Same stash/prewarm/response behaviour as POST /ehr.

## EHR_STATUS

### GET /ehr/{ehr_id}/ehr_status/{version_uid}
chain: handler `openapi_routes.rs::ehr_status_get_by_version_id` → `api/ehr/ehr_status.rs::run` → service `EhrbaseService::get_ehr_status_at_version` → `status_by_version` → `versioning::read::read_version` → storage `version_repo::read::read_version` + `node_repo::read_version_canonical`
sql: 2 round trips — SELECT vo_version⋈audit (attestations LATERAL); SELECT node rows
notes: returns the **bare** EHR_STATUS at that version (not ORIGINAL_VERSION), `uid` injected; EHR-ownership filtered → 404 otherwise. `ETag(version_uid)` + `Location`.

### GET /ehr/{ehr_id}/ehr_status
chain: handler `openapi_routes.rs::ehr_status_get_at_time` → `ehr_status.rs::run` → service `EhrbaseService::get_ehr_status_at_time` → `status_at` → storage `version_repo::meta::current_vo` then `versioning::read::{read_current | version_at}` (with `version_at_time`) → full version read
sql: 3 round trips — SELECT current (ehr_id, kind='EHR_STATUS') vo; SELECT vo_version⋈audit (current row, or `sys_period @> $at` for time-travel); SELECT node rows
notes: `version_at_time` parse failure → 400. ETag/Location from the resolved version.

### PUT /ehr/{ehr_id}/ehr_status
chain: handler `openapi_routes.rs::ehr_status_update` → `ehr_status.rs::run` → `mk_update_version` (If-Match → `preceding_version_uid`, required) → service `EhrbaseService::replace_ehr_status` (service/ehr/status.rs) → `ehr_status_meta_with_vo` (merged If-Match pre-read) → `ensure_if_match` (full-OBJECT_VERSION_ID compare, ITS-REST §Concurrency control) → `commit_status` → `versioning::change::update` → storage `version_repo::commit::{advisory_lock, close_ordinal_at_now, commit_new_version}` + `version_repo::placement::next_placement` + `node_repo::write_nodes`, then `sync_ehr_subject` in the same tx
sql: 1 pre-read + 6 in one tx — SELECT vo_version⋈audit metadata by (ehr_id, kind) [pre-read]; then in tx: SELECT pg_advisory_xact_lock; SELECT next_placement (tip + next ordinal + now(), one statement); UPDATE vo_version close superseded tip; CTE INSERT audit+contribution+vo_version; bulk INSERT node; UPDATE ehr promoted subject/is_queryable/is_modifiable columns
notes: body validated in-memory (`validate_ehr_status`); EHR_STATUS is exempt from the `is_modifiable` content gate ("always modifiable", RM ehr master04 §EHR Active Status). 412 (VersionMismatch) is decorated with the latest version's ETag/Location via one extra metadata read. Item-tag wrapper headers, if present, run a tag replace after the commit (see PUT tags). `Prefer: return=representation` re-reads the current status (+3: current_vo + full read); default 204.

## VERSIONED_EHR_STATUS

### GET /ehr/{ehr_id}/versioned_ehr_status
chain: handler `openapi_routes.rs::versioned_ehr_status_get` → `api/ehr/versioned_ehr_status.rs::run` → service `EhrbaseService::get_versioned_ehr_status` → `versioned_status` → `versioning::wire::versioned_object` → storage `version_repo::meta::time_created`
sql: 2 round trips — SELECT current EHR_STATUS vo for the EHR; SELECT earliest version's audit.time_committed
notes: builds the `VERSIONED_EHR_STATUS` container (uid/owner_id/time_created) in-process; canonical JSON or XML.

### GET /ehr/{ehr_id}/versioned_ehr_status/revision_history
chain: handler `…::versioned_ehr_status_revision_history` → `versioned_ehr_status.rs::run` → service `EhrbaseService::ehr_status_revision_history` → `status_revision_history` → `versioning::wire::revision_history` → storage `version_repo::meta::all_version_meta` + `version_repo::attestation::read_attestations_all`
sql: 3 round trips — SELECT current vo; SELECT all vo_version⋈audit metadata rows ordered by ordinal; SELECT all vo_attestation rows
notes: one `REVISION_HISTORY_ITEM` per version, commit audit first then that revision's attestations (RM common master04 §Revision History).

### GET /ehr/{ehr_id}/versioned_ehr_status/version
chain: handler `…::versioned_ehr_status_version_get_at_time` → `versioned_ehr_status.rs::run` → service `EhrbaseService::ehr_status_version_at_time` → `status_version_at_time` → full version read (`version_at` | `read_current`) → `versioning::wire::original_version`
sql: 3 round trips — SELECT current vo; SELECT vo_version⋈audit (at-time or current); SELECT node rows
notes: `ORIGINAL_VERSION<EHR_STATUS>` with read-time signature verification (warn/strict modes); ETag/Location of the VERSION.

### GET /ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}
chain: handler `…::versioned_ehr_status_version_get_by_id` → `versioned_ehr_status.rs::run` → `version_components` (branch ids first-class) → service `EhrbaseService::ehr_status_original_version` → `status_version` → `versioning::read::read_version` → `wire::original_version`
sql: 2 round trips — SELECT vo_version⋈audit by tree id; SELECT node rows
notes: signature verified on read; attestations appended after verification.

## COMPOSITION

### POST /ehr/{ehr_id}/composition
chain: handler `openapi_routes.rs::composition_create` → `api/ehr/composition.rs::run` → body: `negotiate::rm_value::<Composition>` (or FLAT/STRUCTURED rebuild via `formats/dispatch.rs`, WebTemplate-cache-backed) → `mk_update_version` → service `EhrbaseService::create_composition` (service/ehr/composition.rs) → `ensure_ehr_content_writable` + `validate_composition_for_commit` + `reject_duplicate_persistent` → `versioning::change::create` → storage `version_repo::placement::tx_now` + `version_repo::commit::commit_new_version` + `node_repo::write_nodes`
sql: 1 pre-read + 3 in one tx (happy path) — SELECT ehr.is_modifiable (existence 404 + writability 409 in one trip); then in tx: SELECT now(); CTE INSERT audit+contribution+vo_version; bulk INSERT node (one row per RM structure node). Validation is in-process against the moka WebTemplate cache (a cold template adds 1 template_store read + build). A `431|persistent|` composition adds the duplicate-persistent scan: 1 SELECT of the EHR's live COMPOSITION vo_ids, then **N+1** — a full version read (2 statements) per live composition (PERF-noted; EHRs hold few persistent compositions). Client-supplied `UPDATE_VERSION.attestations` insert one row each (per-item loop).
notes: RM/terminology + template conformance failures → 422 (`422_COMPOSITION`); unknown declared template → 422. `553|incomplete|` lifecycle relaxes lower-cardinality limits. Signature computed pre-insert over the exact served ORIGINAL_VERSION bytes. Item-tag wrapper headers apply after the commit. Response: 201 with `ETag(version_uid)`+`Location`; `Prefer: return=representation` (or a FLAT/STRUCTURED Accept) re-reads the committed version (+2: full version read).

### GET /ehr/{ehr_id}/composition/{uid_based_id}
chain: handler `…::composition_get` → `composition.rs::run` → `parse_uid_based_id` → service `EhrbaseService::{get_composition_at_version | get_composition_at_time | get_composition_latest}` → `read_composition`/`composition_at_time` → full version read (`read_version` | `version_at` | `read_current`)
sql: 2 round trips — SELECT vo_version⋈audit (by tree id / `sys_period @> $at` / current); SELECT node rows
notes: a deleted version resolves to a null body → 204 (`204_because_deleted`). `?expand_multimedia=true` re-inlines externalized DV_MULTIMEDIA (object-store fetches, no SQL; no-op when off). FLAT/STRUCTURED Accept renders through the converter seam (WebTemplate cache). 200 with `ETag(version_uid)` + `Location`.

### PUT /ehr/{ehr_id}/composition/{uid_based_id}
chain: handler `…::composition_update` → `composition.rs::run` → body decode (canonical/FLAT/STRUCTURED) + body-uid vs path cross-check (400 on mismatch) → `mk_update_version` (If-Match required) → service `EhrbaseService::update_composition` → merged pre-read `version_repo::meta::current_composition_meta` (ownership 404, full-OVID If-Match 412, deleted 404, `is_modifiable` 409, stored-template 422 — one statement) → `validate_composition_for_commit` → tx: `validation::check_versioned_composition_invariants` → `versioning::change::update` → storage commit + `node_repo::write_nodes`
sql: 1 pre-read + 6 in one tx — SELECT vo_version⋈audit⋈ehr LEFT JOIN node(num=0) [the merged pre-read]; then in tx: SELECT first_version_root (VERSIONED_COMPOSITION invariants, RM ehr `versioned_composition.adoc`); SELECT pg_advisory_xact_lock; SELECT next_placement; UPDATE close superseded tip; CTE INSERT audit+contribution+vo_version; bulk INSERT node. (A cross-system preceding version FORKS a branch instead: +1 SELECT next_branch_number, no tip close.)
notes: template-id mismatch with the stored root → 422 (CNF `update_composition-wrong_template`). 412 decorated with the latest version's ETag/Location (+1 metadata read). Item-tag wrapper headers post-commit; `Prefer`/FLAT re-read +2. 200 (or 204-shaped minimal per `write_rm` with ok/ok here — both statuses 200).

### DELETE /ehr/{ehr_id}/composition/{uid_based_id}
chain: handler `…::composition_delete` → `composition.rs::run` → `parse_version_uid` (must be a full OBJECT_VERSION_ID — the preceding version; bare uid → 400) → service `EhrbaseService::delete_composition` → lean pre-read `current_composition_meta` (already-deleted 400, `is_modifiable` 409, stale preceding 409) → `versioning::change::delete` → storage commit (no node rows — data Void, RM common master06 §Logical Deletion)
sql: 1 pre-read + 4 in one tx — SELECT current_composition_meta; then in tx: SELECT pg_advisory_xact_lock; SELECT next_placement; UPDATE close tip; CTE INSERT audit+contribution+vo_version (lifecycle `523`, no node insert)
notes: 204 with the ETag/Location of the just-committed deleted version; a stale/conflicting preceding → 409 decorated with the latest version uid (+1 metadata read).

## VERSIONED_COMPOSITION

### GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}
chain: handler `…::versioned_composition_get` → `api/ehr/versioned_composition.rs::run` → service `EhrbaseService::get_versioned_composition` → `versioned_composition` (ownership check via `versioning::read::read_current`) → `wire::versioned_object` → `version_repo::meta::time_created`
sql: 3 round trips — SELECT vo_version⋈audit current row (ownership gate, full read incl. attestation LATERAL); SELECT node rows (paid by that full read); SELECT earliest audit.time_committed
notes: the ownership pre-check uses the full `read_current` (its reassembled body is discarded) — the one read on this surface that pays more than metadata.

### GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history
chain: handler `…::versioned_composition_revision_history` → `versioned_composition.rs::run` → service `EhrbaseService::composition_revision_history` → `wire::revision_history` → `version_repo::meta::all_version_meta` + `version_repo::attestation::read_attestations_all`
sql: 2 round trips — SELECT all version metadata rows; SELECT all attestations for the object
notes: EHR ownership checked from the first metadata row → 404 otherwise.

### GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version
chain: handler `…::versioned_composition_version_get_at_time` → `versioned_composition.rs::run` → service `EhrbaseService::composition_version_at_time` → `composition_version_at_time_read` → full version read (`version_at` | `read_current`) → `wire::original_version`
sql: 2 round trips — SELECT vo_version⋈audit (at-time or current); SELECT node rows
notes: a deleted version still returns 200 as a data-less deleted-lifecycle ORIGINAL_VERSION; ETag/Location point at `…/version/{version_uid}`.

### GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}
chain: handler `…::versioned_composition_version_get_by_id` → `versioned_composition.rs::run` → service `EhrbaseService::composition_original_version` → `composition_version` → `versioning::read::read_version` → `wire::original_version`
sql: 2 round trips — SELECT vo_version⋈audit by tree id; SELECT node rows
notes: carries the version `<signature>`; read-time signature verification (strict mode → 5xx on mismatch).

## DIRECTORY (FOLDER)

### GET /ehr/{ehr_id}/directory
chain: handler `…::directory_get_at_time` → `api/ehr/directory.rs::run` → service `EhrbaseService::get_directory_at_time` → `directory_at_time` → storage `ehr_repo::directory_vo` (slot = lowest-rank live hierarchy, fallback to deleted) → full version read → in-process `select_subfolder` for the `path` query param
sql: 3 round trips — SELECT ehr_folder⋈vo_version slot resolution; SELECT vo_version⋈audit (current or at-time); SELECT node rows
notes: deleted directory → 204; unresolved sub-folder `path` → 404. `200_FOLDER_retrieved` declares no ETag/Location.

### PUT /ehr/{ehr_id}/directory
chain: handler `…::directory_update` → `directory.rs::run` → `mk_update_version` (If-Match required) → service `EhrbaseService::update_directory` → merged pre-read `ehr_repo::directory_current_meta` (slot ⋈ version meta ⋈ ehr.is_modifiable, one statement) → `ensure_if_match` → `commit_directory_update` (in-memory `validate_folder`, 409 when not modifiable) → `versioning::change::update` → storage commit + `node_repo::write_nodes`
sql: 1 pre-read + 5 in one tx — SELECT directory_current_meta; then in tx: advisory lock; next_placement; close tip; CTE INSERT audit+contribution+vo_version; bulk INSERT node
notes: 412 decorated with the latest directory version (+1 read). Item-tag wrapper headers post-commit. `Prefer: return=representation` re-reads the directory (+3); default 204 with ETag/Location.

### POST /ehr/{ehr_id}/directory
chain: handler `…::directory_create` → `directory.rs::run` → `mk_update_version` → service `EhrbaseService::create_directory` → `commit_new_directory` → `ensure_ehr_exists` + `validate_folder` + `ensure_content_writable` + `directory_vo_opt` (409 when a directory already occupies the slot) → `versioning::change::create` → storage commit + `write_nodes` + `version_repo::commit::insert_ehr_folder_rank`
sql: 3 pre-reads + 4 in one tx — SELECT ehr exists; SELECT ehr.is_modifiable; SELECT directory slot; then in tx: SELECT now(); CTE INSERT audit+contribution+vo_version; bulk INSERT node; INSERT ehr_folder (rank = max+1 — the new hierarchy joins `EHR.folders`, RM ehr master04 §Folders)
notes: 201 with ETag/Location; representation re-read +3.

### DELETE /ehr/{ehr_id}/directory
chain: handler `…::directory_delete` → `directory.rs::run` → `require_if_match` → service `EhrbaseService::delete_directory` → merged pre-read `directory_current_meta` → `ensure_if_match` → `delete_directory_at` → `versioning::change::delete`
sql: 1 pre-read + 4 in one tx — SELECT directory_current_meta; then in tx: advisory lock; next_placement; close tip; CTE INSERT audit+contribution+vo_version (lifecycle 523, no nodes)
notes: plain 204, no headers (`204_because_deleted` declares none); 412 decorated (+1 read).

### GET /ehr/{ehr_id}/directory/{version_uid}
chain: handler `…::directory_get_by_version_id` → `directory.rs::run` → service `EhrbaseService::get_directory_at_version` → `directory_version` → `versioning::read::read_version`
sql: 2 round trips — SELECT vo_version⋈audit by tree id; SELECT node rows
notes: deleted version → 204; `?path=` selects the named sub-folder subtree in-process (slash-separated FOLDER names, unresolved → 404), same semantics as the at-time read.

## CONTRIBUTION

### POST /ehr/{ehr_id}/contribution
chain: handler `…::contribution_create` → `api/ehr/contribution.rs::run` → `negotiate::json_value` (JSON only — a CONTRIBUTION commit is a wrapper DTO with no canonical-XML shape) → service `EhrbaseService::ehr_contribution_commit` → `versioning::contribution::commit_version_set` (classify each version per RM common master06 §Contributions; the raw-wire seam covers attestation-only 666 and delete 523 members + committer/system_id inheritance) → per change `versioning::change::apply_change` under one `commit_contribution` tx → storage `version_repo::commit::{write_contribution, commit_version_into, close_ordinal_at_now}` + `node_repo::write_nodes` + `version_repo::attestation` inserts
sql: variable, per change set — pre-tx: SELECT ehr exists (1); ONE batched SELECT of all modify/delete/attest target kinds (`object_kinds` over `ANY($ids)` — deliberately not N+1); per-create duplicate-singleton probe (1 each: current_vo for EHR_STATUS/EHR_ACCESS, live_folder_root_exists for FOLDER); SELECT ehr.is_modifiable (1, skipped for an EHR_STATUS-only set). In the tx: per COMPOSITION-modify 1 SELECT first_version_root (cross-version invariants); CTE INSERT audit+contribution (1, the CONTRIBUTION's own audit — one now() for the whole set); then per version: create = CTE INSERT audit+vo_version + bulk INSERT node (2); modify = advisory lock + next_placement + close tip + CTE + nodes (5); delete = advisory lock + next_placement + close + CTE (4); each accompanying/666 attestation = 1 INSERT (per-item loop). Composition validation per member runs against the WebTemplate cache (cold template = +1 store read).
notes: change-control mismatches (249 with a preceding, 666 without) → 400 (`400_CONTRIBUTION` scope); content violations → 422; duplicate supplied CONTRIBUTION uid → conflict, never overwrite. An EHR_ACCESS member invalidates the access-settings cache after commit. 201 with `ETag(contribution_uid)` + `Location`; `Prefer: return=representation` re-assembles the stored CONTRIBUTION (+2 — the GET read below); `return=minimal` skips that re-read entirely.

### GET /ehr/{ehr_id}/contribution/{contribution_uid}
chain: handler `…::contribution_get` → `contribution.rs::run` → service `EhrbaseService::{get_contribution | get_contribution_resolved}` → `ehr_contribution` → `versioning::contribution::get_contribution` → storage `version_repo::contribution::{contribution_audit, contribution_version_refs}`
sql: 2 round trips — SELECT contribution⋈audit (EHR-scoped, 404 otherwise); SELECT the version identities the contribution committed (∪ versions its 666 attestations reference). With `Prefer: resolve_refs`: **N+1** — one full version read (2 statements) per referenced version to inline the ORIGINAL_VERSIONs.
notes: default `versions` are OBJECT_REFs; `resolve_refs` per ITS-REST `Requests_and_responses` §Representation details.

### GET /ehr/{ehr_id}/contribution  (no uid — OUR OWN EXTENSION)
chain: handler `…::contribution_list` → `contribution.rs::run` → service `EhrbaseService::ehr_contribution_list_page` → storage `version_repo::contribution::{count_contributions (via versioning), list_contribution_summaries}`
sql: 3 round trips — SELECT the EHR existence probe (`ensure_ehr_exists` → 404); SELECT count(*) FROM contribution c JOIN audit a ON a.id=c.audit_id WHERE c.ehr_id=$1 as `total` (via `versioning::count_contributions`); SELECT c.id, a.time_committed, a.change_type, a.committer#>>'{name}' FROM contribution c JOIN audit a ON a.id=c.audit_id WHERE c.ehr_id=$1 ORDER BY a.time_committed DESC, c.id DESC OFFSET $2 LIMIT $3
notes: OUR OWN EXTENSION — no openEHR spec governs it (the ITS-REST contract defines only the by-uid CONTRIBUTION GET). Session-authenticated (Clinical RBAC class; ABAC Pre-checked on the target EHR, subject-gated like the sibling EHR reads). Response `{ "rows": [ { uid, time_committed, committer, change_type } ], "total" }` newest-first; `offset` default 0, `fetch` default 20 capped at 100. `committer` = the audit committer PARTY_PROXY's `name` (a summary string; the by-uid GET returns the full party); `change_type` = the stored `audit.change_type` code. JSON only (a DTO with no canonical-XML shape → 406 on an XML-only Accept). Unknown EHR → 404.

## Item tags (ITS-REST experimental extension; `item_tag` table, spec-silent storage)

### GET /ehr/{ehr_id}/tags
chain: handler `…::ehr_tags_get` → `ehr_resource.rs::run` → service `EhrbaseService::ehr_tags_get` → `ehr_tags` → storage `tag_repo::list_tags`
sql: 1 round trip — SELECT item_tag filtered by ehr scope (+ optional key/value/target_path query params), ordered by key
notes: wire shape is exactly the OAS `ItemTag` (RM `ITEM_TAG` invariants enforced on write).

### GET /ehr/{ehr_id}/composition/{uid_based_id}/tags
chain: handler `…::composition_tags_get` → `composition.rs::run` → service `EhrbaseService::target_tags_get` → `target_tags` → `tag_repo::list_tags`
sql: 1 round trip — SELECT item_tag by (ehr, target_vo_id)

### PUT /ehr/{ehr_id}/composition/{uid_based_id}/tags
chain: handler `…::composition_tags_update` → `composition.rs::run` → `negotiate::json_vec` → service `EhrbaseService::target_tags_replace` → `replace_tags` (service/ehr/tags.rs: `ensure_ehr_exists` + `vo_owner` same-EHR gate + RM `Inv_key_valid`/`Inv_value_valid` in-memory) → storage `tag_repo::replace_tags` in a tx → re-list
sql: 3 + N round trips — SELECT ehr exists; SELECT vo_version owner (tag targets only within the same EHR, RM ehr `EHR.tags`); tx[DELETE all target tags; then **one INSERT … ON CONFLICT upsert per posted tag** — an explicit per-item loop]; SELECT the stored collection back
notes: PUT is full-collection replace — an empty list clears all tags; returns the stored list.

### DELETE /ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}
chain: handler `…::composition_tags_delete` → `composition.rs::run` → service `EhrbaseService::target_tag_delete` → `delete_tag` → `tag_repo::delete_tag`
sql: 1 round trip — DELETE item_tag by (ehr, target, key)
notes: zero rows deleted → 404; else 204.

### GET /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags
chain: handler `…::ehr_status_tags_get` → `ehr_status.rs::run` → service `EhrbaseService::target_tags_get` → `tag_repo::list_tags`
sql: 1 round trip — SELECT item_tag by (ehr, target_vo_id)
notes: identical seam to the COMPOSITION variant (target_type differs only on write).

### PUT /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags
chain: handler `…::ehr_status_tags_update` → `ehr_status.rs::run` → service `EhrbaseService::target_tags_replace` (target_type "EHR_STATUS") → `tag_repo::replace_tags` + re-list
sql: 3 + N round trips (per-tag INSERT loop, as the COMPOSITION variant)

### DELETE /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}
chain: handler `…::ehr_status_tags_delete` → `ehr_status.rs::run` → service `EhrbaseService::target_tag_delete` → `tag_repo::delete_tag`
sql: 1 round trip — DELETE item_tag by (ehr, target, key)
notes: 204 / 404 as the COMPOSITION variant.

---

# Endpoint → function-chain map — QUERY + DEFINITION groups

All paths are group-relative, mounted under the configured base path
(default `/ehrbase/rest/openehr/v1`) by `app/ehrbase-rest/src/api/query/openapi_routes.rs`
and `app/ehrbase-rest/src/api/definition/openapi_routes.rs`. Every handler is the same
thin shape: `into_parts` → `guarded_dispatch(state, op_id, parts, <group>::dispatch::dispatch)`;
that shared spine (auth, ATNA, tracing, panic guard, error rendering) is documented once
elsewhere — each entry below says only `spine: standard api/** dispatch`.

File shorthand: `rest:` = `app/ehrbase-rest/src`, `app:` = `app/ehrbase/src`.

---

## QUERY group (6 operations) — `rest:api/query/`

The six operations share one pipeline behind `rest:api/query/dispatch.rs::run`:

1. **ABAC pre** — `rest:extensions/access/pep.rs::query_pre` (no-op returning
   `(None, false)` unless RBAC/ABAC is configured; when on, resolves the patient
   subject-scope claim and turns on touched-attribute collection).
2. **Normalize** — `adhoc.rs` / `stored.rs` build an
   `ehrbase::service::query::request::AqlQueryRequest` (ehr_ids, offset/fetch,
   `$parameter` binds) from query string or JSON body; `ehr_id` accepted as query
   parameter **or** `openEHR-EHR-id` header (ITS-REST QUERY `Request.md` §About the
   `ehr_id` parameter), query parameter winning.
3. **Execute** — `EhrbaseService::execute_ad_hoc_query` / `::execute_stored_query`
   (`app:service/query/execute.rs`), the full AQL pipeline below.
4. **ABAC post** — `pep::query_post` (PDP fan-out over the touched template set; no-op
   when ABAC off or the result set is empty).
5. **Render** — `rest:api/query/response.rs::respond_result_set`: JSON-only negotiation
   (an XML `Accept` is `406` — the QUERY operations declare no canonical-XML form) plus
   the spec-mandated weak `ETag` on the `RESULT_SET` (`responses/200_Query.yaml` +
   `headers/ETag_RESULT_SET.yaml`), derived as a deterministic content digest of the
   assembled document.

**The AQL pipeline** (`execute_aql_inner`, `app:service/query/execute.rs`):

- **plan** (`plan_query`): **plan-cache** lookup (`app:service/query/plan_cache.rs`,
  moka, keyed on the exact query text, capacity `[query].plan_cache_capacity`,
  default 256). Miss: parse (`openehr_query::parser::parse_str`, logos+chumsky, pure
  CPU) → **terminology-expansion seam** (`app:aql/terminology.rs::expand_matches`:
  resolves `TERMINOLOGY('expand', …)` operands in `matches` through the terminology
  service and merges the codes into the value list before planning — QUERY master03
  lines 756–759; a no-op for terminology-free queries; may call the external FHIR TS
  when configured) → **lower to typed IR** (`app:aql/mod.rs::lower_query` →
  `aql/lower.rs` + `aql/analyze.rs`, path analysis against the BMM-generated RM model)
  → cache insert (**terminology-resolving plans are never cached** — the expansion may
  differ next time). Hit or miss, `aql::check_params` re-validates the caller's
  `$parameter` bindings against the plan.
- **paging** (`compose_paging`): REST `fetch`/`offset` vs AQL `LIMIT`/`OFFSET`/`TOP`
  collision → `400` (ITS-REST QUERY `Request.md`); otherwise the AQL clause wins.
- **scope** (`resolve_ehr_ids`): parses the ehr_id strings (malformed → `400`), then one
  existence probe; a well-formed-but-absent id → `ehr_id_does_not_exist` → `404`
  (`i_query_service.adoc`). Skipped entirely when no ehr_id is scoped.
- **sql build** (`app:aql/sql/mod.rs::build` + `from`/`select`/`predicate`/`value`/`expr`):
  one `SELECT` over `node`/`vo_version`/`ehr`/`audit` via sea-query — nested-set
  interval joins for CONTAINS, `jsonb_path_query_first` + jsonpath item methods +
  `openehr_magnitude` for typed leaf extraction/ordering; parameter values, paging,
  and scope all bind here (which is what makes the cached plan request-independent).
- **execute** (`app:aql/exec.rs::execute`): one `fetch_all`; whole-object SELECT
  columns are batch-reassembled through the P10 node codec
  (`app:storage/node_repo.rs::read_subtrees_canonical`) in **one** follow-up round
  trip (never per-row).
- **budget**: when `[query].timeout_ms` is set the DB execution is wrapped in
  `tokio::time::timeout`; overrun → tagged `SmError` rendered `408`
  (`responses/408_Query.yaml`). Default off.
- **assemble** (`app:service/query/result_set.rs`): `result_set_json` builds the
  ITS-REST 1.1.0 `RESULT_SET` (columns + rows + `meta`); `substitute_params` produces
  `meta._executed_aql` (the parameter-substituted text). CPU only.
- **metrics**: `aql_query_duration_seconds{phase=plan|execute}`,
  `aql_queries_total{outcome}` exactly once per call.

### GET /query/aql
chain: handler `rest:api/query/openapi_routes.rs::query_execute_adhoc_query` → `api/query/dispatch.rs::run` → `api/query/adhoc.rs::execute` (params from query string via `params::build::<QueryExecuteAdhocQueryParams>`) → `EhrbaseService::execute_ad_hoc_query` (`app:service/query/execute.rs`) → the AQL pipeline above → `response.rs::respond_result_set`
spine: standard api/** dispatch
sql: 1–3 round trips — 1 SELECT over node/vo_version/ehr/audit (the generated query, always); +1 `SELECT id FROM ehr WHERE id = ANY($1)` (only when an ehr_id is scoped); +1 batched node-subtree SELECT (only when a SELECT column is a whole RM object); (+1 scope-collection SELECT when ABAC is on)
notes: plan cache (moka, text-keyed) skips parse+lower on repeat text; terminology expansion seam pre-plan; weak content-digest `ETag` on the 200; JSON only (XML Accept → 406); optional `[query].timeout_ms` → 408.

### POST /query/aql
chain: handler `…::query_execute_adhoc_query_body` → same as GET, except the `AdhocQueryExecute` body (`q`/`offset`/`fetch`/`query_parameters`) is decoded by `response.rs::decode_body` (JSON only — non-JSON Content-Type → 415) and `ehr_id` still comes from the query string/header
spine: standard api/** dispatch
sql: identical to GET /query/aql
notes: same pipeline, body-borne parameters (`schemas/query/AdhocQueryExecute.yaml`).

### GET /query/{qualified_query_name}
chain: handler `…::query_execute_stored_query` → `dispatch.rs::run` → `api/query/stored.rs::execute` → `EhrbaseService::execute_stored_query(name, None, …)` (`app:service/query/execute.rs`) → `EhrbaseService::get_stored_query` (`app:service/definition/query.rs` — DEFINITION owns the store; latest version = `ORDER BY string_to_array(semver,'.')::int[] DESC LIMIT 1`, case-insensitive name) → extract `q` → the same AQL pipeline
spine: standard api/** dispatch
sql: 2–4 round trips — 1 SELECT stored_query (resolve the text) + the ad-hoc set above
notes: the stored text then executes byte-identically to an ad-hoc query, so the plan cache serves repeats of a stored query too (keyed on the resolved text); unknown name → 404.

### POST /query/{qualified_query_name}
chain: handler `…::query_execute_stored_query_body` → `stored.rs::execute` (`Query` body: `offset`/`fetch`/`query_parameters`, no `q` — `schemas/query/Query.yaml`; name read from the matched path) → `execute_stored_query(name, None, …)` → as GET
spine: standard api/** dispatch
sql: identical to GET /query/{name}
notes: —

### GET /query/{qualified_query_name}/{version}
chain: handler `…::query_execute_stored_query_version` → `stored.rs::execute` → `execute_stored_query(name, Some(version), …)` → `get_stored_query` with an exact SEMVER or a `{major}`/`{major}.{minor}` **prefix** (highest matching stored version, `parameters/path/version.yaml`) → the AQL pipeline
spine: standard api/** dispatch
sql: identical to GET /query/{name} (the stored_query SELECT carries the version predicate)
notes: unknown name/version → 404.

### POST /query/{qualified_query_name}/{version}
chain: handler `…::query_execute_stored_query_version_body` → `stored.rs::execute` (`Query` body; name+version from the path) → `execute_stored_query(name, Some(version), …)` → as the GET variant
spine: standard api/** dispatch
sql: identical to GET /query/{name}/{version}
notes: —

---

## DEFINITION group (13 operations) — `rest:api/definition/`

Group dispatcher: `rest:api/definition/dispatch.rs::run` — a pure operation-id match
fanning out to `template_adl14.rs` / `template_adl2.rs` / `stored_query.rs`. The two
list operations share `dispatch.rs::list_filter_and_page` (wire
`template_id`/`concept`/`version` globs + `offset`/`fetch` → `TemplateListFilter` +
SM `Page`).

### GET /definition/template/adl1.4
chain: handler `rest:api/definition/openapi_routes.rs::definition_template_adl1_4_list` → `template_adl14.rs::list` → `EhrbaseService::template_adl14_list` (`app:service/definition/wire.rs`) → `EhrbaseService::template_summaries` (`app:templates/store.rs`) → `wire.rs::filter_templates` (glob `*` → anchored regex, then paginate)
spine: standard api/** dispatch
sql: 1 round trip — SELECT template_id/concept/root_archetype/created_at FROM template_store (full set, ordered)
notes: filtering + pagination happen in Rust over the full descriptor set (`parameters/query/filter_template_id.yaml` glob semantics), not in SQL.

### POST /definition/template/adl1.4
chain: handler `…::definition_template_adl1_4_upload` → `template_adl14.rs::upload` (lenient text body) → `EhrbaseService::template_adl14_upload` (`app:service/definition/wire.rs`) → `EhrbaseService::store_template` (`app:templates/store.rs`): parse (`app:templates/ingest.rs::parse_opt` → `openehr_its::opt14::from_xml`, quick-xml CPU) → validate structure (`app:validation/structure.rs::validate_opt_structure`, closes the tolerant codec's leniency) → validate artefact (`app:validation/opt/mod.rs::validate_opt_artefact`, the AOM2 master08 standalone-artefact catalogue: VCOC/VACMCO, VATID/VTLC, VTTBK/VTCBK, VCORM/VCARM/VCAEX/VCACA/VCAM) → mandatory `template_id`/`concept` checks → single insert
spine: standard api/** dispatch
sql: 1 round trip — INSERT INTO template_store … ON CONFLICT (lower(template_id)) DO NOTHING RETURNING … (no-row = duplicate → 409, race-free, case-insensitive per BASE master05 §Composite Identifiers and Case)
notes: parse + two validation passes are the CPU stages, all pre-insert (only validated templates are stored — AM OPT2 master02 §Purpose of the OPT); no WebTemplate-cache invalidation needed (create-only insert); handler builds `Location`, weak `ETag`, and the `Prefer`-negotiated body (representation = the OPT XML, identifier = `{"template_id":…}` JSON).

### GET /definition/template/adl1.4/{template_id}
chain: handler `…::definition_template_adl1_4_get` → `template_adl14.rs::get`: `negotiate_accept` first (outside `Accept_Template` → 406, before touching storage) → `EhrbaseService::template_adl14_get` (`app:service/definition/wire.rs`) → `opt_get_by_template_id` (`app:service/definition/adl14.rs`, case-insensitive; the existence probe **and** the canonical body). XML branch: respond directly. `wt+json`/JSON branch: `template_adl14.rs::web_template_response` → `EhrbaseService::web_template` (`app:service/mod.rs`) → `web_template_for` (`app:templates/runtime.rs`): moka WebTemplate-cache hit, or miss → `get_template_xml` + `openehr_its::flat::build_web_template` (single-flighted `get_or_build`)
spine: standard api/** dispatch
sql: 1 round trip (XML, or wt+json with a warm cache) — SELECT content FROM template_store; +1 identical SELECT on a wt+json cold-cache build
notes: WebTemplate cache (`openehr_its::flat::cache::WebTemplateCache`, moka, keyed on the identity-canonical template_id); serving wt+json on this endpoint is a deliberate EHRbase-compatible extension (ITS-REST returns only the OPT); weak `ETag` keyed on template_id (`headers/ETag_Template_adl1_4.yaml`).

### GET /definition/template/adl1.4/{template_id}/example
chain: handler `…::definition_template_adl1_4_example_get` → `template_adl14.rs::example_get` → `EhrbaseService::template_adl14_example` (`app:service/definition/wire.rs`; `detail_level`/`type` enum parse → 400) → `EhrbaseService::template_example` (`app:templates/runtime.rs`): `get_template_xml` (unconditional — the existence probe doubles as the cold-build source; unknown id → 404, unlike the commit-path 422) → WebTemplate cache hit or `build_cached_web_template` → **example generator** `openehr_its::flat::example_composition(&wt, level)` (+ `openehr_its::flat::apply_output_uid` for `type=output`) → response negotiated across the four `Accept_LOCATABLE` forms: FLAT/STRUCTURED via the shared converter seam `rest:formats/dispatch.rs::composition_{flat,structured}_response`, else canonical JSON/XML via `negotiate::respond_rm::<Composition>`
spine: standard api/** dispatch
sql: 1 round trip — SELECT content FROM template_store
notes: example generation is not spec-mandated (a convenience surface); the WebTemplate build is the only heavy CPU stage and is cached; the FLAT/STRUCTURED renderings re-resolve the same cached WebTemplate.

### GET /definition/template/adl2
chain: handler `…::definition_template_adl2_list` → `template_adl2.rs::list` → `EhrbaseService::template_adl2_list` (`app:service/definition/wire.rs`) → `adl2_template_list(Page::all())` (`app:service/definition/adl2.rs`) → `filter_templates` in memory
spine: standard api/** dispatch
sql: 1 round trip — SELECT hrid, created_at FROM adl2_artefact WHERE kind IN ('template','operational_template') ORDER BY hrid
notes: the service fetches the full set (`Page::all`) and filters/pages in Rust; each row is a `TemplateMetadata` object (`template_id`, `concept` = the HRID concept segment, `archetype_id` = the HRID, `created_timestamp`) — derived from the stored HRID, no cADL parse needed.

### POST /definition/template/adl2
chain: handler `…::definition_template_adl2_upload` → `template_adl2.rs::upload` (text/plain body) → `EhrbaseService::template_adl2_upload` (`app:service/definition/wire.rs`) → `adl2_wire_upload` (`app:service/definition/adl2.rs`): `adl2_validate` (parse + AOM2 phases via the `openehr-adl` engine, against a repository built from the stored ADL2 set) → `adl2_exists` (case-insensitive → 409) → `adl2_persist` (INSERT)
spine: standard api/** dispatch
sql: 1 fetch + 1 probe + 1 insert — SELECT adl FROM adl2_artefact (build the validation repository); SELECT EXISTS (409 probe); INSERT INTO adl2_artefact. The SM-native `upload_artefact` uses a case-insensitive DELETE+INSERT replace instead of the 409.
notes: an invalid source is a 422 whose `Error.validationErrors` carry the AOM2/ADL2 rule codes (S-codes for a parse failure, V-codes for a validation-phase failure); a non-UTF-8 body is a 400. Duplicate handling diverges by surface — REST 409 (`definition-codegen.openapi.yaml`) vs SM `upload_artefact` replace (`i_definition_adl2.adoc`). Response is `Prefer`-negotiated (representation = text/plain source, identifier = JSON `{template_id}`) with `Location`.

### GET /definition/template/adl2/{template_id}
chain: handler `…::definition_template_adl2_get` → `template_adl2.rs::get` → `render` (Accept negotiation: text/plain source | application/json OPT | xml-only → 406) → `EhrbaseService::template_adl2_source` / `template_adl2_opt_json` (`app:service/definition/wire.rs`) → `adl2_resolve` + `adl2_get` (+ `adl2_opt_json` for JSON)
spine: standard api/** dispatch
sql: 1–2 round trips — SELECT hrid (exact) or SELECT hrid[] (partial-resolve family), then SELECT adl; the JSON projection also SELECTs the full set to build the OPT repository
notes: `text/plain` = the stored source verbatim (`200_Template_adl2_retrieved.yaml`); `application/json` = the `OperationalTemplateV2` canonical JSON (opaque `type: object` in the OAS → the AOM2 OPT JSON satisfies it, built via `openehr_adl::opt::create_opt` for non-OPT kinds); `application/xml` has no declared response body → 406. Unknown HRID → 404.

### GET /definition/template/adl2/{template_id}/example
chain: handler `…::definition_template_adl2_example_get` → `template_adl2.rs::example_get` → `EhrbaseService::template_adl2_example` → `adl2_resolve` (HRID) → `adl2_get` (source) → `openehr_adl::opt::create_opt` → `openehr_its::flat::webtemplate::build_web_template_am24` (am24 → Web Template) → `openehr_its::flat::example::example_composition` → `Accept_LOCATABLE` negotiation (canonical JSON/XML + FLAT/STRUCTURED); 400 (bad `type`/`detail_level`), 404 (unknown template), 406 (unsupported Accept)
spine: standard api/** dispatch
sql: 0 round trips
notes: needs an example generator over an `am24`-OPT WebTemplate (the same generator issue #94 builds); ADL2 is OPTIONAL for CNF.

### GET /definition/template/adl2/{template_id}/{version}
chain: handler `…::definition_template_adl2_version_get` → `template_adl2.rs::version_get` → `render` with the explicit `version` → `template_adl2_source` / `template_adl2_opt_json` → `adl2_resolve(template_id, Some(version))`
spine: standard api/** dispatch
sql: 1–2 round trips — SELECT hrid[] (resolve family + version prefix → highest match), then SELECT adl (+ full set for the JSON OPT repository)
notes: `deprecated: true` in the vendored OAS (reflected via `#[deprecated]` on the handler); serves the same `text/plain` / `application/json` representations as `_get`; a missing template/version → 404.

### GET /definition/query/{qualified_query_name}
chain: handler `…::definition_query_list` → `stored_query.rs::list` → `EhrbaseService::query_list` (`app:service/definition/wire.rs`) → `list_stored_queries` (`app:service/definition/query.rs`)
spine: standard api/** dispatch
sql: 1 round trip — SELECT … FROM stored_query WHERE left(lower(name), length($1)) = lower($1) ORDER BY name, semver-array
notes: the name is a case-insensitive **prefix** (empty ⇒ wildcard), all versions of every match returned (`definition_query_list.yaml`).

### PUT /definition/query/{qualified_query_name}
chain: handler `…::definition_query_store_yaml` (op id `definition_query_store.yaml`) → `stored_query.rs::store` (text body; `query_type` default `AQL`) → `EhrbaseService::query_store(name, None, …)` (`app:service/definition/wire.rs`; non-AQL formalism → typed 400) → `store_query_version` (`app:service/definition/query.rs`): store-time AQL parse (`openehr_query::parser::parse_str`, CPU — invalid → 400) → transactional upsert at the fixed default version `1.0.0` → back in the handler, `stored_version_of` recovers the assigned version through `query_list` for the `Location` header
spine: standard api/** dispatch
sql: 1 tx + 1 SELECT — BEGIN / DELETE (case-insensitive, semver 1.0.0) + INSERT INTO stored_query / COMMIT; then the prefix-list SELECT to rebuild `Location`
notes: success is `200 OK` with `Location`, not 201/204 (`200_StoredQuery_stored.yaml` + `headers/Location_Query.yaml`); the no-version store is an upsert at `1.0.0` (the "stores or updates" semantics), PORT-NOTEd; a failed version-recovery degrades to a Location-less 200 rather than failing the already-committed store.

### GET /definition/query/{qualified_query_name}/{version}
chain: handler `…::definition_query_version_get` → `stored_query.rs::version_get` → `EhrbaseService::query_version_get` (`app:service/definition/wire.rs`) → `get_stored_query(name, Some(version))` (`app:service/definition/query.rs`; exact SEMVER, or `{major}`/`{major}.{minor}` prefix → highest match)
spine: standard api/** dispatch
sql: 1 round trip — SELECT … FROM stored_query with the exact/prefix version predicate (`left(semver, …)` on a dot boundary, ordered by semver-array DESC LIMIT 1)
notes: unknown name/version → 404; identity case-insensitive (BASE master05 §Composite Identifiers and Case).

### PUT /definition/query/{qualified_query_name}/{version}
chain: handler `…::definition_query_version_store_yaml` (op id `definition_query_version_store.yaml`) → `stored_query.rs::version_store` (text body; `query_type` default `AQL`) → `EhrbaseService::query_store(name, Some(version), …)` → `store_query_version`: AQL parse (CPU) → case-insensitive existence probe → insert-only
spine: standard api/** dispatch
sql: 2 round trips — SELECT EXISTS (case-variant 409 probe) + INSERT … ON CONFLICT (rdn, semantic_id, semver) DO NOTHING (0 rows affected also → 409, race-safe)
notes: an explicit `(name, version)` pair is **immutable** — 409, never an overwrite (`409_StoredQuery_version.yaml`); `200 OK` + `Location` on success.

---

# Endpoint → function-chain map — DEMOGRAPHIC

Scope: every operation mounted from
`app/ehrbase-rest/src/api/demographic/openapi_routes.rs` (the standard
Demographic API group — `x-status: DEVELOPMENT` in the vendored
`docs/specs/openehr/ITS-REST/specifications/demographic.openapi.yaml`) and
`app/ehrbase-rest/src/api/demographic/relationship.rs` (the own-design
`PARTY_RELATIONSHIP` extension, realizing SM `I_PARTY_RELATIONSHIP` —
excluded from conformance-profile claims). **50 mounted operations**: 42 in
`openapi_routes.rs` + 8 in `relationship.rs`.

Every handler is the same thin shape: snapshot the request
(`api::into_parts`) → `guarded_dispatch(state, "<op_id>", parts,
demographic::dispatch::dispatch)`. The group dispatcher
(`api/demographic/dispatch.rs::run`) routes `party_relationship*` ops to
`relationship::run`, maps `{kind}_{action}` operation ids onto
`(PartyKind, action)` (`mod.rs::parse_party_op`) into `party::run` /
`tags::run`, and the kind-agnostic ids into `versioned_party::run`,
`contribution::run`, `tags::run_collection`. The five per-kind operation
families are one shared code path — the generated per-kind `*Params`
structs are field-identical, so one representative struct (`Agent*Params`)
is reused across kinds. Per row below: **spine: standard api/** dispatch**
(EHR_ACCESS gate → ABAC PEP pre-check → dispatcher → PEP post-check → ATNA
op-id tag; the EHR_ACCESS gate is inert here — parties are not EHR-scoped).

Shared building blocks (SQL shapes referenced below):

- **lean current resolve** — `version_repo::meta::current_demographic_meta`:
  ONE `SELECT vo_version ⋈ audit` (kind + lifecycle + tree + system id +
  commit instant; `ehr_id IS NULL`, `upper_inf(sys_period)`), no node
  reassembly. Serves both the full-OVID `If-Match` compare and the write
  gate in one resolve (ITS-REST overview §Concurrency control).
- **full version read** — `versioning::read` over
  `version_repo::read`: ONE `SELECT vo_version ⋈ audit` with the
  attestations folded in as a `LEFT JOIN LATERAL jsonb_agg` (no separate
  round trip) + ONE `SELECT node` rows → codec reassembly to canonical
  JSON. A logically deleted version has no node rows → `Null` body.
- **standalone write commit** — `versioning::change::{create,update,delete}`
  in one transaction: (update/delete only) `SELECT pg_advisory_xact_lock`
  + ONE placement `SELECT vo_version` (preceding tip + next ordinal +
  `now()`; create instead reads `SELECT now()` via `tx_now`) + (continue
  case) `UPDATE vo_version` closing the superseded tip + ONE folded
  data-modifying CTE `INSERT audit + INSERT contribution + INSERT
  vo_version` (`commit::commit_new_version`) + batched `INSERT node`
  (skipped for delete — a `523|deleted|` version stores no nodes, RM
  common master06 §Logical Deletion) + optional `INSERT` event-outbox row
  (only when eventing is configured). Version signature is computed
  in-process before insert.
- Write responses are built **from the commit result + the in-hand body**
  (`support::committed_response`) — no post-write re-read; the only
  exception is a fresh `read_party` when the DV_MULTIMEDIA externalization
  engine is enabled (stored form ≠ in-memory form).

---

### POST /demographic/{kind} — `{kind}_create` (serves: agent|group|organisation|person|role)
spine: standard api/** dispatch
chain: handler `api/demographic/openapi_routes.rs::{kind}_create` →
`dispatch::run` → `api/demographic/party.rs::run("create")`
(`negotiate::rm_value::<Agent|Group|Organisation|Person|Role>` decodes
canonical JSON or XML through the concrete RM type) →
`EhrbaseService::party_create` (`service/demographic/api.rs`) →
`commit_new_party` (`service/demographic/party.rs`:
`validate::validate_party_body` → `versioning::change::create`) →
`attach_party_item_tags`; then, iff the request carried an
`openehr-item-tag` header, the handler persists those tags via
`EhrbaseService::party_tags_update` and re-populates the metadata seam
(`party.rs::persist_request_tags` — the party must exist first,
`item_tag.target_vo_id`).
sql: 4 round trips (base) — tx: SELECT now(); INSERT audit+contribution+vo_version (one folded CTE); INSERT node (batched); [INSERT outbox if eventing on]; then SELECT item_tag (response-header seam). Header-tag persistence adds the PUT-tags chain (see tags row). Multimedia-externalization adds a fresh read (SELECT vo_version kind; SELECT vo_version⋈audit; SELECT node).
notes: 201 + weak `ETag W/"{ovid}"` + `Location {base}/demographic/{kind}/{uid}`; body per `Prefer` (`return=representation` → the created party re-typed per kind, else empty). Response echoes `openehr-item-tag`/`openehr-version-item-tag` headers (demographic tags anchor on the VERSIONED_OBJECT, so the two headers coincide). Validation seam: root `_type` must equal the routed kind (mismatch → 422), typed deserialization through `openehr-rm`, PARTY `Identities_valid`, present-but-empty list invariants (`Contacts_valid`/`Relationships_validity`/`Roles_valid`/`Capabilities_valid`), inline-relationship source check, `PARTY_REF.Type_validity` + `OBJECT_REF.namespace` (RM `demographic.party.adoc`, BASE `party_ref.adoc`/`object_ref.adoc`). Asymmetry: the wire seam passes `update_audit = None` — the `openEHR-VERSION.*`/`openEHR-AUDIT_DETAILS.*` committal-header merge rides only the SM-native `create_party(UpdateVersion)` path, not this wire route; audit is the server default (`demographic_audit`).

### GET /demographic/{kind}/{uid_based_id} — `{kind}_get` (serves: agent|group|organisation|person|role)
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::{kind}_get` → `dispatch::run` →
`party.rs::run("get")` → `EhrbaseService::party_get` →
`read_party` (`load_party_version`: `versioning::read::object_kind`
kind-gate + `support::load_ehrless` full version read) →
`attach_party_item_tags`.
sql: 4 round trips — SELECT vo_version (kind of current version); SELECT vo_version⋈audit (+LATERAL attestations; current / by-OVID / at-instant per the id form and `?version_at_time`); SELECT node (reassembly); SELECT item_tag.
notes: `uid_based_id` accepts a bare HIER_OBJECT_ID or a full OBJECT_VERSION_ID (specific version); `?version_at_time` time-travels (RM common master08 §Change Management). A wrong-kind object (a PERSON under `/agent/`) or EHR-scoped object → 404; a logically deleted current version reads `Null` → **204** (mirroring composition_get). 200 sets `ETag`/`Location` + the item-tag response headers (`person_get.yaml`); the `uid` is injected into the body on read (PARTY `Uid_mandatory`, `demographic.party.adoc`).

### PUT /demographic/{kind}/{uid_based_id} — `{kind}_update` (serves: agent|group|organisation|person|role)
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::{kind}_update` → `dispatch::run` →
`party.rs::run("update")` → `EhrbaseService::party_update`
(`party_current` lean resolve → `ensure_full_ovid_if_match` →
`commit_party_update`: not-deleted gate → `validate_party_body` →
`versioning::change::update`) → handler `persist_request_tags` (optional).
sql: 6 round trips — SELECT vo_version⋈audit (lean current, shared by If-Match compare and write gate — resolved ONCE); tx: SELECT pg_advisory_xact_lock; SELECT vo_version (placement: tip + next ordinal + now()); UPDATE vo_version (close superseded tip); INSERT audit+contribution+vo_version (folded CTE); INSERT node. On a 412 the handler re-reads the latest meta (SELECT vo_version⋈audit) to echo `ETag`/`Location`.
notes: `If-Match` is **mandatory**; a full-OVID token must equal the current latest in ALL segments (object + creating system + tree — ITS-REST overview §Concurrency control), mismatch → 412 with the latest `version_uid` in `ETag` (`person_update.yaml`); a bare trunk number keeps lenient tree addressing (stale → 409 from the placement check). 200/204 per `Prefer`. Deleted current → 404. Same committal-header asymmetry as create (wire passes `update_audit = None`). A same-system preceding version continues the lineage (trunk N → N+1); a foreign-system preceding forks a branch (extra SELECT for next branch number — master06 §Distributed Versioning).

### DELETE /demographic/{kind}/{uid_based_id} — `{kind}_delete` (serves: agent|group|organisation|person|role)
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::{kind}_delete` → `dispatch::run` →
`party.rs::run_delete` → `EhrbaseService::party_delete` (`party_current`
lean resolve → `ensure_full_ovid_if_match` → `commit_party_delete`:
already-deleted → 400, stale expected → 409 → `versioning::change::delete`).
sql: 5 round trips — SELECT vo_version⋈audit (lean current); tx: SELECT pg_advisory_xact_lock; SELECT vo_version (placement); UPDATE vo_version (close tip); INSERT audit+contribution+vo_version (folded CTE; no node rows — logical delete). On the 409 the handler re-reads latest meta (SELECT vo_version⋈audit) for the `ETag` echo.
notes: G-2 wire shape (`person_delete.yaml`): the **path** `uid_based_id` carries the preceding OBJECT_VERSION_ID (no `If-Match` declared by the contract); `If-Match` is retained as a fallback preceding-version source for older clients, and a malformed `If-Match` is rejected (`precondition_violation`), never silently ignored. Responses: 204 + `ETag`/`Location` of the *deleted* version; 400 already-deleted; 404 unknown/wrong-kind; 409 stale uid with the latest `version_uid` echoed. Logical delete = a new `523|deleted|` version (RM common master06 §Logical Deletion).

### GET /demographic/versioned_party/{versioned_object_uid} — `versioned_party_get`
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::versioned_party_get` → `dispatch::run` →
`api/demographic/versioned_party.rs::run` →
`EhrbaseService::versioned_party_get` → `service/demographic/versioned.rs::versioned_party`
(`ensure_any_party` → `support::versioned_wrapper`).
sql: 2 round trips — SELECT vo_version (kind: any of the five party kinds); SELECT vo_version⋈audit LIMIT 1 (earliest commit time → `time_created`).
notes: assembles the `VERSIONED_PARTY` wrapper in-process (RM common master06 §Versioned Objects). `owner_id` references the object's own vo_id — no EHR owner exists for a demographic versioned object (flagged PORT NOTE, our own design). Plain 200, no ETag.

### GET /demographic/versioned_party/{versioned_object_uid}/revision_history — `versioned_party_revision_history`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `versioned_party.rs::run` →
`EhrbaseService::versioned_party_revision_history` →
`versioned.rs::party_revision_history` (`ensure_any_party` →
`support::demographic_revision_history`).
sql: 2 round trips — SELECT vo_version (kind); SELECT vo_version⋈audit ORDER BY sys_version (all version metadata, no node reassembly).
notes: one `REVISION_HISTORY_ITEM` per version with its OBJECT_VERSION_ID + commit `AUDIT_DETAILS` (RM common master04 §Revision History) — lean metadata rows, bodies never reassembled.

### GET /demographic/versioned_party/{versioned_object_uid}/version[?version_at_time=] — `versioned_party_version_get_at_time`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `versioned_party.rs::run` →
`EhrbaseService::versioned_party_version_get_at_time` →
`versioned.rs::party_version_at_time` (`ensure_any_party` →
`support::demographic_original_version_at`: `load_ehrless` full read →
`versioning::wire::original_version` + signing).
sql: 3 round trips — SELECT vo_version (kind); SELECT vo_version⋈audit (+LATERAL attestations; `sys_period @> at` or current when omitted); SELECT node (reassembly).
notes: 200 with the `ORIGINAL_VERSION` + `ETag`/`Location` pointing at the concrete VERSION resource (`…/versioned_party/{uid}/version/{version_uid}`). Signature verification/assembly is in-process.

### GET /demographic/versioned_party/{versioned_object_uid}/version/{version_uid} — `versioned_party_version_get_by_id`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `versioned_party.rs::run` →
`EhrbaseService::versioned_party_version_get_by_id` →
`versioned.rs::party_version` (`ensure_any_party` →
`support::demographic_original_version`).
sql: 3 round trips — SELECT vo_version (kind); SELECT vo_version⋈audit (by tree-id columns); SELECT node.
notes: plain 200 `ORIGINAL_VERSION` (no ETag headers — asymmetric with the at-time read, which sets them). A version miss surfaces as `object_version_does_not_exist` → 404.

### POST /demographic/contribution — `contribution_create`
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::contribution_create` → `dispatch::run` →
`api/demographic/contribution.rs::run` →
`EhrbaseService::demographic_contribution_create`
(`service/demographic/contribution.rs::create_demographic_contribution`) →
`versioning::contribution::commit_version_set(ehr_id = None, party_only = true)`
→ per change `versioning::change::apply_change` under one shared
CONTRIBUTION → read-back `demographic_contribution`.
sql: 3 + tx round trips for K changes — SELECT vo_version WHERE vo_id = ANY($1) (ONE batched kind/existence pre-check of all modification targets); tx: INSERT audit+contribution (folded CTE, the CONTRIBUTION's own audit); then per change: [SELECT pg_advisory_xact_lock; SELECT vo_version placement; UPDATE vo_version close — modify/delete only]; INSERT audit+vo_version (folded CTE, per-version commit_audit under the shared contribution id); INSERT node (creates/modifies); [INSERT outbox if eventing on]; then read-back: SELECT contribution⋈audit; SELECT vo_version ∪ vo_attestation-join (version refs).
notes: body is the `NewContribution` wrapper (`schemas/demographic/NewContribution.yaml`), **JSON only** (`negotiate::json_value` — no XML on this op). `party_only = true`: an EHR-kind version inside → 422 (engine scope check). One `now()` per transaction — the whole set shares the commit instant; the CONTRIBUTION audit's committer/system_id copy down into each version's commit_audit (RM common master06 §Committal m4). Version payloads route through the same `validate_party_body`/`validate_relationship_body` seams via `validate_for_commit`. `666|attestation|` items attest existing versions without creating new ones. 201 + `ETag` = contribution uid; body per `Prefer`. A client-supplied CONTRIBUTION uid is honoured when unused, rejected when malformed/in use.

### GET /demographic/contribution/{contribution_uid} — `contribution_get`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `contribution.rs::run` →
`EhrbaseService::demographic_contribution_get` →
`service/demographic/contribution.rs::demographic_contribution`.
sql: 2 round trips — SELECT contribution⋈audit (scoped `ehr_id IS NULL` — an EHR-scoped contribution uid is 404 here); SELECT vo_version UNION vo_version⋈vo_attestation (committed rows ∪ attested rows = the full change-set).
notes: the shared EHR-scoped `get_contribution` cannot serve ehr-less contributions, so this chapter assembles the CONTRIBUTION wire shape itself (`OBJECT_REF` versions list, namespace `demographic`).

### GET /demographic/tags — `demographic_tags_get`
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::demographic_tags_get` → `dispatch::run` →
`api/demographic/tags.rs::run_collection` →
`EhrbaseService::demographic_tags_get` →
`service/demographic/tags.rs::demographic_tags` → `storage::tag_repo::list_tags`.
sql: 1 round trip — SELECT item_tag WHERE ehr_id IS NULL (optional key/value/target_path filters) ORDER BY key.
notes: the kind-agnostic collection filter (`demographic_tags_get.yaml`); wire shape assembled in-process (RM `common.item_tag`; `owner_id` = the tagged party itself — flagged own-design, no EHR owner exists).

### GET /demographic/{kind}/{uid_based_id}/tags — `{kind}_tags_get` (serves: agent|group|organisation|person|role)
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::{kind}_tags_get` → `dispatch::run` →
`tags.rs::run("tags_get")` → `EhrbaseService::party_tags_get` →
`service/demographic/tags.rs::party_tags` → `tag_repo::list_tags`.
sql: 1 round trip — SELECT item_tag WHERE ehr_id IS NULL AND target_vo_id = $1 ORDER BY key.
notes: asymmetry worth knowing: the routed kind is **ignored** (`_kind`) and no party-existence check runs — an unknown id returns an empty list 200, and a PERSON's tags are readable via `/agent/{uid}/tags`. Canonical (not LOCATABLE) content negotiation.

### PUT /demographic/{kind}/{uid_based_id}/tags — `{kind}_tags_update` (serves: agent|group|organisation|person|role)
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::{kind}_tags_update` → `dispatch::run` →
`tags.rs::run("tags_update")` → `EhrbaseService::party_tags_update` →
`service/demographic/tags.rs::replace_party_tags` (`ensure_party`
kind+live gate → RM invariant checks → `tag_repo::replace_tags` →
read-back `party_tags`).
sql: 6 + N round trips — SELECT vo_version (kind); SELECT vo_version⋈audit + SELECT node (ensure_party full current read; deleted → 404); tx: DELETE item_tag (whole collection); N × INSERT item_tag (ON CONFLICT upsert); then SELECT item_tag (read-back, ORDER BY key).
notes: PUT full-collection semantics — an empty array clears all tags; duplicate keys in the body are last-wins (pre-deduplicated in a BTreeMap, since the NULL-scope unique index never collides). RM `ITEM_TAG` invariants enforced pre-write: `Inv_key_valid` (non-empty, no surrounding whitespace), `Inv_value_valid` (present ⇒ non-empty) → 422. G-4 (`person_tags_update.yaml`): 200 + tag list on `Prefer: return=representation`, else 204. This same service call implements the `openehr-item-tag` request-header persistence on party create/update.
PERF note: `ensure_party` pays a full version read (node reassembly) where the lean meta resolve would do.

### DELETE /demographic/{kind}/{uid_based_id}/tags/{key} — `{kind}_tags_delete` (serves: agent|group|organisation|person|role)
spine: standard api/** dispatch
chain: handler `openapi_routes.rs::{kind}_tags_delete` → `dispatch::run` →
`tags.rs::run("tags_delete")` → `EhrbaseService::party_tags_delete` →
`service/demographic/tags.rs::delete_party_tag` → `tag_repo::delete_tag`.
sql: 1 round trip — DELETE FROM item_tag WHERE ehr_id IS NULL AND target_vo_id AND key (rows_affected = 0 → 404).
notes: 204 on success. Same asymmetry as tags_get: kind ignored, no party-existence gate — the 404 distinguishes only "no such tag".

---

## PARTY_RELATIONSHIP extension (`relationship.rs` — our own wire; no ITS-REST operation governs it; excluded from conformance claims; realizes SM `i_party_relationship.adoc`)

The envelope mirrors the party CRUD with one fixed `party_relationship` /
`versioned_party_relationship` segment; the generated party `*Params`
structs are reused by analogy. Same dispatcher spine (`relationship_routes`
→ `guarded_dispatch` → `dispatch::run` → `relationship::run`).

### POST /demographic/party_relationship — `party_relationship_create`
spine: standard api/** dispatch
chain: handler `api/demographic/relationship.rs::party_relationship_create`
→ `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::party_relationship_create` →
`service/demographic/relationship.rs::create_relationship`
(`validate::validate_relationship_body` → `versioning::change::create`).
sql: 3 round trips — tx: SELECT now(); INSERT audit+contribution+vo_version (folded CTE); INSERT node; [INSERT outbox if eventing on].
notes: body decodes through the concrete `PartyRelationship` RM type (JSON or XML). Validation: root `_type` = `PARTY_RELATIONSHIP`; `source`/`target` must be present continuant refs — a `HIER_OBJECT_ID` (the party's version **container**), an `OBJECT_VERSION_ID` is rejected 422 (RM demographic master02 §Modelling of Parties and Relationships); `PARTY_REF.Type_validity` + `OBJECT_REF.namespace` enforced. 201 + `ETag`/`Location`; body per `Prefer`. No item-tag headers on this family (unlike parties). No DV_MULTIMEDIA read-back branch (relationship bodies always served from the in-hand canonical).

### GET /demographic/party_relationship/{uid_based_id} — `party_relationship_get`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::party_relationship_get` → `read_relationship`
(`load_relationship_version`: `object_kind` gate + `support::load_ehrless`).
sql: 3 round trips — SELECT vo_version (kind = PARTY_RELATIONSHIP); SELECT vo_version⋈audit (+LATERAL attestations); SELECT node.
notes: same id forms and `?version_at_time` semantics as the party get; deleted current → `Null` → 204; 200 sets `ETag`/`Location`; `uid` injected on read. No item-tag read (relationships carry no tags surface).

### PUT /demographic/party_relationship/{uid_based_id} — `party_relationship_update`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::party_relationship_update` (`relationship_current` lean
resolve → `ensure_full_ovid_if_match` → `commit_relationship_update`:
not-deleted gate → `validate_relationship_body` →
`versioning::change::update`).
sql: 6 round trips — SELECT vo_version⋈audit (lean current); tx: SELECT pg_advisory_xact_lock; SELECT vo_version (placement); UPDATE vo_version (close tip); INSERT audit+contribution+vo_version (folded CTE); INSERT node. On 412: SELECT vo_version⋈audit (latest meta echo via `party_relationship_latest_meta`).
notes: mandatory `If-Match`, same full-OVID 412 discipline as parties. 200/204 per `Prefer`. Asymmetry recorded in code: the relationship `ResourceMeta` carries **no** `Last-Modified` (the party meta does).

### DELETE /demographic/party_relationship/{uid_based_id} — `party_relationship_delete`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::party_relationship_delete` (`relationship_current` lean
resolve → `ensure_full_ovid_if_match` → `commit_relationship_delete`).
sql: 5 round trips — SELECT vo_version⋈audit (lean current); tx: SELECT pg_advisory_xact_lock; SELECT vo_version (placement); UPDATE vo_version (close tip); INSERT audit+contribution+vo_version (folded CTE; no node rows).
notes: mirrors `party_delete`: preceding version from `If-Match` when supplied (malformed → rejected), else the path OVID, else unconditional; already-deleted → 400; stale → 409 (but — asymmetry — the handler has **no** 409-with-`ETag` echo arm like the party delete; the conflict maps through the plain error path). 204 + deleted-version `ETag`/`Location`.

### GET /demographic/versioned_party_relationship/{versioned_object_uid} — `versioned_party_relationship_get`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::versioned_party_relationship_get` →
`service/demographic/relationship.rs::versioned_relationship`
(`ensure_any_relationship` → `support::versioned_wrapper`).
sql: 2 round trips — SELECT vo_version (kind); SELECT vo_version⋈audit LIMIT 1 (`time_created`).
notes: wire `_type` is `VERSIONED_OBJECT` (not a spec `VERSIONED_PARTY_RELATIONSHIP` — none exists); same own-design `owner_id` PORT NOTE as the party wrapper.

### GET /demographic/versioned_party_relationship/{versioned_object_uid}/revision_history — `party_relationship_revision_history`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::party_relationship_revision_history` →
`relationship.rs::relationship_revision_history` (`ensure_any_relationship`
→ `support::demographic_revision_history`).
sql: 2 round trips — SELECT vo_version (kind); SELECT vo_version⋈audit ORDER BY sys_version.
notes: same `REVISION_HISTORY` assembly as the party family (RM common master04 §Revision History).

### GET /demographic/versioned_party_relationship/{versioned_object_uid}/version[?version_at_time=] — `party_relationship_version_get_at_time`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::party_relationship_version_get_at_time` →
`relationship.rs::relationship_version_at_time` (`ensure_any_relationship`
→ `support::demographic_original_version_at` + signing).
sql: 3 round trips — SELECT vo_version (kind); SELECT vo_version⋈audit (`sys_period @> at` / current); SELECT node.
notes: 200 `ORIGINAL_VERSION` + `ETag`/`Location` on the VERSION resource (`versioned_party_relationship/{uid}/version/{ovid}`).

### GET /demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid} — `party_relationship_version_get_by_id`
spine: standard api/** dispatch
chain: handler → `dispatch::run` → `relationship.rs::run` →
`EhrbaseService::party_relationship_version_get_by_id` →
`relationship.rs::relationship_version` (`ensure_any_relationship` →
`support::demographic_original_version`).
sql: 3 round trips — SELECT vo_version (kind); SELECT vo_version⋈audit (by tree-id columns); SELECT node.
notes: plain 200 `ORIGINAL_VERSION` (no ETag — same asymmetry as the party by-id read).

---

## Op inventory (50)

| Shape | Ops | Count |
|---|---|---|
| party create | agent/group/organisation/person/role `_create` | 5 |
| party get | `{kind}_get` | 5 |
| party update | `{kind}_update` | 5 |
| party delete | `{kind}_delete` | 5 |
| versioned_party reads | `versioned_party_get`, `_revision_history`, `_version_get_at_time`, `_version_get_by_id` | 4 |
| contribution | `contribution_create`, `contribution_get` | 2 |
| tags collection | `demographic_tags_get` | 1 |
| per-kind tags get/update/delete | `{kind}_tags_get`/`_update`/`_delete` | 15 |
| relationship CRUD | `party_relationship_create`/`_get`/`_update`/`_delete` | 4 |
| relationship versioned reads | `versioned_party_relationship_get`, `party_relationship_revision_history`, `_version_get_at_time`, `_version_get_by_id` | 4 |
| **Total** | | **50** |

---

# Endpoint → function-chain map — ADMIN · EXTENSIONS · PUBLIC/OPERATIONAL (+ background appendix)

Sections authored from the code on branch `claude/w14-audit` (read-only pass, 2026-07-16).
File paths are repo-relative under `/Users/rubentalstra/RustroverProjects/ehrbase-rs/`.

**The shared spine ("spine: standard api/** dispatch").** Every group mounted inside the
API nest (`{base_path}` = `/ehrbase/rest/openehr/v1`) runs:
`#[utoipa::path]` handler → `api::into_parts` (snapshot path/query/headers/body) →
`api::guarded_dispatch` (`app/ehrbase-rest/src/api/mod.rs`): EHR_ACCESS gate →
ABAC PEP pre-check → group dispatcher → PEP post-check → ATNA `AuditOpId` response tag —
all under the API-subtree layers (tenant resolution when tenancy is on → authn →
ATNA audit middleware → HTTP metrics + root span → **overload shed**, outermost), and the
whole tree under the shared tower-http stack (request-id, trace, catch-panic, CORS,
16 MiB body limit, 30 s timeout, compression) (`app/ehrbase-rest/src/router.rs`).

**Surfaces OUTSIDE auth + overload shedding** (siblings of the API nest, never shed, so an
operator can always probe an overloaded server): `/ehrbase/rest/status`, `/health`,
`/ehrbase/rest/status/health`, the SMART discovery document, the Swagger UI + OpenAPI JSON
documents, and the whole `/management/*` surface (which carries its own per-endpoint
access-level guard and may live on a separate port). The System `OPTIONS` manifest sits
**above even the CORS layer** (CORS would eat `OPTIONS` as a preflight).

---

## 1. ADMIN API (ITS-REST Admin, DEVELOPMENT status)

Group gate: `AppConfig::admin.enabled` (default **false**) — when off every admin route
answers `404` inside the dispatcher, mirroring EHRbase's `ADMINAPI_ACTIVE` prior art
(never a 403). Files: `app/ehrbase-rest/src/api/admin/{openapi_routes,dispatch}.rs`,
service `app/ehrbase/src/service/admin/delete.rs`.

### DELETE /admin/ehr/{ehr_id}
chain: `admin_ehr_delete` (openapi_routes.rs) → spine: standard api/** dispatch →
`admin::dispatch::run` → `EhrbaseService::admin_ehr_delete` → `delete_ehr` (delete.rs)
sql: 3 round trips in one transaction (+1 read outside, +GC when multimedia is on) —
  - SELECT node.data for the EHR (blob-key collection; **only** when DV_MULTIMEDIA externalization is configured, else skipped)
  - tx: SELECT audit ids (UNION over vo_version + contribution)
  - tx: DELETE FROM ehr (FK cascade → vo_version → node/vo_attestation, contribution, item_tag; 0 rows → 404, rollback)
  - tx: DELETE FROM audit WHERE id = ANY(captured) (audit has no FK from ehr — swept explicitly)
  - post-commit GC (multimedia only): per candidate blob 1 EXISTS scan over node + an S3 delete (failures logged, not fatal)
notes: SM `I_ADMIN_SERVICE.physical_ehr_delete` (SM `i_admin_service.adoc` — precondition
`has_ehr`, error `ehr_id_does_not_exist` → 404); sync success 204. The cascade SQL/FK graph
is flagged in-code: no openEHR spec governs it — our own design over the greenfield schema.

### DELETE /admin/ehr/all?ehr_id=…
chain: `admin_ehr_delete_all` → spine → `admin::dispatch::run` (raw-query `ehr_id_list`,
accepting both repeated and comma-separated forms) → `EhrbaseService::admin_ehr_delete_all`
→ `delete_ehr_set`
sql: 1 SELECT id FROM ehr when the selector is empty (empty list = "delete ALL EHRs",
per ITS-REST `admin_ehr_delete_all.yaml`), then per target EHR the full `delete_ehr`
transaction above (one transaction **per EHR**; missing ids skipped — idempotent bulk).
notes: 204 always (bodyless per the OAS). The mounted path is the plain `/admin/ehr/all`
(the generated route's RFC 6570 `{?ehr_id*}` suffix is normalisation-stripped). The bulk
call exists in the ITS-REST OAS but not the abstract SM interface — a recorded
spec-internal inconsistency; skip-missing semantics are our own design (spec-silent).

### DELETE /admin/template/{template_id}
chain: `admin_template_delete` (openapi_routes.rs) → spine: standard api/** dispatch →
`admin::dispatch::run` → `EhrbaseService::admin_template_delete` → `delete_template_by_id`
(service/admin/delete.rs)
sql: 3 round trips in one transaction (+cache eviction outside) —
  - tx: SELECT template_id FROM template_store WHERE lower(template_id)=lower($1) (case-insensitive resolve; absent → 404, rollback)
  - tx: SELECT count(*) FROM vo_version WHERE template_id = $1 (FK-reference count; > 0 → 409, rollback)
  - tx: DELETE FROM template_store WHERE template_id = $1
  - post-commit: evict the WebTemplate moka cache entry for the canonical key
notes: OUR OWN EXTENSION — no openEHR spec governs it (the ITS-REST Admin API defines only
EHR deletes). Admin-gated (`AppConfig::admin.enabled` → 404 when off; RBAC Admin class by the
`/admin/` path). 204 on success; unknown id → 404; a template still referenced by a committed
version → 409 naming the count (physical deletes never orphan clinical data — the
`vo_version.template_id` FK is the hard guard, the count is the friendly message).

### DELETE /admin/query/{qualified_query_name}/{version}
chain: `admin_query_delete` (openapi_routes.rs) → spine: standard api/** dispatch →
`admin::dispatch::run` → `EhrbaseService::admin_query_delete` →
`delete_stored_query_version` (service/definition/query.rs)
sql: 1 round trip — DELETE FROM stored_query WHERE lower(reverse_domain_name)=lower($1)
AND lower(semantic_id)=lower($2) AND semver=$3 (case-insensitive name as on the PUT store
path, exact version; 0 rows → 404)
notes: OUR OWN EXTENSION — no openEHR spec governs it (the ITS-REST Admin API defines only
EHR deletes). Admin-gated as above. Deletes exactly one `(name, version)` row (the SM
`I_DEFINITION_QUERY.delete_query` deletes every version by name; this admin surface is
single-version). 204 on success; unknown name/version → 404.

### GET /admin/config
chain: `admin_config` (openapi_routes.rs) → spine: standard api/** dispatch →
`admin::dispatch::run` → serves `state.observability().env_snapshot` (the redacted
effective-config JSON the binary builds at boot via
`EhrbaseConfig::to_redacted_json`)
sql: none — no database access; reads the in-memory boot-time config snapshot only.
notes: OUR OWN EXTENSION — no openEHR spec governs configuration (the ITS-REST Admin API
defines only EHR deletes). Admin-gated as above (`AppConfig::admin.enabled` → 404 when off;
RBAC Admin class by the `/admin/` path → 401 unauthenticated / 403 non-admin). 200 returns
the effective configuration tree as JSON with every secret-bearing leaf redacted
**structurally** — redaction is a property of the `Secret`/`SecretUrl` leaf types
(`***` / `scheme://***@host`), never a key-name scan, so it is fail-closed for any new
secret that follows the config discipline (`Secret`/`SecretUrl` + `*_file` sibling).

---

## 2. EXTENSIONS

All extension groups are mounted **inside** the API nest (full spine, auth, ATNA tagging,
overload shed) and are always mounted — the config gate is enforced inside each dispatcher
(disabled → 404 without touching the backend). None are part of the generated ITS-REST
contract; each file carries the explicit flag where applicable.

### 2.1 Terminology extension (6 endpoints) — `extensions/terminology.rs`

Gate: `AppConfig::terminology_api_enabled` (default false). Operation semantics: SM
`I_TERMINOLOGY_SERVICE` (SM `master12-terminology_service.adoc` + `i_terminology_service.adoc`);
the **wire shape** is explicitly flagged: no openEHR spec defines a terminology REST
contract — our own design/extension. Provider routing (`service/terminology/routing.rs`):
enumeration is always the in-process `openehr-term` bundle; lookup/validation goes to the
bundle when it knows the terminology, else to the opt-in external FHIR R4 TS provider
(`service/terminology/fhir.rs`, reqwest/rustls), else falls back to the bundle's
`Pre_has_terminology` → 404. The `has_*` boolean calls are surfaced implicitly through
200-vs-404 of their `get` counterparts (deliberate, documented).

### GET /terminology
chain: `terminology_ids` → spine → `terminology::run` → `EhrbaseService::get_terminology_ids` (bundle)
sql: 0 round trips — compile-time-embedded bundle
notes: body `{"terminology_ids": [...]}`.

### GET /terminology/{terminology_id}
chain: `terminology_description` → spine → `run` → `get_terminology_description` (bundle)
sql: 0 round trips
notes: doubles as the `has_terminology` existence check (404 on unknown).

### GET /terminology/{terminology_id}/term/{code}?at_date=
chain: `terminology_get_term` → spine → `run` → `get_term` (bundle, else FHIR `CodeSystem/$lookup`)
sql: 0 round trips — 1 HTTPS round trip to the external FHIR TS when routed there
notes: the SM `attributes` allow-list is not surfaced on the wire (PORT-NOTEd, passed `None`).

### GET /terminology/{terminology_id}/subsumes?ref_code=&candidate=
chain: `terminology_subsumes` → spine → `run` → `subsumes` (bundle: always `false` — flat
bundle, PORT-NOTEd; hierarchical answers via FHIR `CodeSystem/$subsumes`)
sql: 0 SQL — 1 HTTPS round trip when routed to the FHIR TS
notes: missing required query param → 400.

### GET /terminology/{terminology_id}/value_set/{value_set_id}
chain: `terminology_value_set` → spine → `run` → `get_value_set` (bundle, else FHIR `ValueSet/$expand`)
sql: 0 SQL — 1 HTTPS round trip when external

### GET /terminology/{terminology_id}/value_set/{value_set_id}/validate?candidate_code=[&at_date=]
chain: `terminology_value_set_validate` → spine → `run` → `value_set_validate`
(bundle, else FHIR `ValueSet/$validate-code`)
sql: 0 SQL — 1 HTTPS round trip when external
notes: body `{"valid": bool}`; missing `candidate_code` → 400.

### 2.2 Event-subscription admin extension (5) — `extensions/event_subscription.rs`

Gate: `AppConfig::events_admin_api` (default false). Mounted under `/admin/` so the coarse
RBAC gate fail-safe classes it `Admin`. No openEHR spec governs eventing — our own
enterprise extension. No ABAC resource kind (generic PEP `Skip`s) and no ATNA audit-table
entry (subscriptions are configuration, not PHI access). Service:
`app/ehrbase/src/extensions/events/subscription.rs` (table `event_subscription`).

### GET /admin/event_subscription
chain: `event_subscription_list` → spine → `run` → `EhrbaseService::event_subscription_list`
sql: 1 round trip — SELECT event_subscription ORDER BY created_at DESC

### POST /admin/event_subscription
chain: `event_subscription_create` → spine → `run` → `event_subscription_create`
sql: 1 round trip — INSERT INTO event_subscription … RETURNING
notes: `name` required, `[A-Za-z0-9_.-]` (it is the AMQP queue-name suffix
`<exchange>.<name>`); duplicate name → 409; predicates NULL = wildcard; 201.

### GET /admin/event_subscription/{subscription_id}
chain: `event_subscription_get` → spine → `run` → `event_subscription_get`
sql: 1 round trip — SELECT event_subscription WHERE id (404 when absent)
notes: malformed UUID → 400.

### PUT /admin/event_subscription/{subscription_id}
chain: `event_subscription_update` → spine → `run` → `event_subscription_update`
sql: 1 round trip — UPDATE event_subscription SET predicates+enabled … RETURNING (404 on miss)
notes: `name` immutable (the queue key).

### DELETE /admin/event_subscription/{subscription_id}
chain: `event_subscription_delete` → spine → `run` → `event_subscription_delete`
sql: 1 round trip — DELETE FROM event_subscription WHERE id (0 rows → 404)
notes: the bound broker queue is NOT torn down (service is broker-free; PORT-NOTEd —
operators reap orphaned durable queues out of band); the publisher loop re-syncs bindings.

### 2.3 Tenant admin extension (5) — `extensions/tenant_routes.rs`

Gate: `AppConfig::tenancy.enabled` (default false). Mounted under `/admin/` (coarse RBAC
`Admin` class). No openEHR spec governs multi-tenancy — our own extension (master13 is
informative deployment guidance only). Service: `app/ehrbase/src/extensions/tenancy.rs`
(table `tenant` — deliberately NOT RLS-scoped: it is the registry isolation is defined
against). Every CRUD write clears the in-process claim→tenant resolver cache.

### GET /admin/tenant
chain: `tenant_list` → spine → `run` → `EhrbaseService::tenant_list`
sql: 1 round trip — SELECT tenant ORDER BY created_at DESC

### POST /admin/tenant
chain: `tenant_create` → spine → `run` → `tenant_create`
sql: 1 round trip — INSERT INTO tenant (name, system_id) RETURNING
notes: both fields required non-empty (400); duplicate name → 409; 201.

### GET /admin/tenant/{tenant_id}
chain: `tenant_get` → spine → `run` → `tenant_get`
sql: 1 round trip — SELECT tenant WHERE id (404 when absent)

### PUT /admin/tenant/{tenant_id}
chain: `tenant_update` → spine → `run` → `tenant_update`
sql: 1 round trip — UPDATE tenant SET name, system_id RETURNING (404 on miss)

### DELETE /admin/tenant/{tenant_id}
chain: `tenant_delete` → spine → `run` → `tenant_delete`
sql: 3 round trips in one transaction —
  - SELECT set_config('ehrbase.tenant_id', $1, true)  (SET LOCAL: scope RLS to the *target* tenant)
  - SELECT summed count over ehr + template_store + stored_query + archetype_store + adl2_artefact (emptiness check; >0 → 409)
  - DELETE FROM tenant WHERE id (0 rows → 404)
notes: the reserved default tenant (nil UUID, `ext.current_tenant_id()` fallback) can
never be deleted (409, no SQL run).

### 2.4 FHIR R4 connector + mapping store (7) — `extensions/fhir.rs`

Gate: `AppConfig::fhir_api_enabled` (default false; disabled → 404 as an
`OperationOutcome`). No openEHR spec governs FHIR interop — our own enterprise extension
(distinct from SM Subject Proxy: this connector *commits*/serves; SPS *reads*). **Every
error on this surface is a FHIR `OperationOutcome`** (`application/fhir+json`), never the
openEHR error body — the FHIR boundary. Starter resource set only: Patient, Observation,
Condition, DocumentReference (anything else → typed 501 before the backend is touched).
Service: `app/ehrbase/src/extensions/fhir/mod.rs` (table `fhir_mapping`).

### POST /fhir/r4/{resource_type}
chain: `fhir_ingest` → spine → `fhir::run` → `ingest` → `EhrbaseService::fhir_ingest` —
resolve mapping → resolve-or-create EHR from the resource's subject → WebTemplate →
`openehr_its::flat::from_flat` → FEEDER_AUDIT stamp → `create_composition` (the NORMAL
validated commit path)
sql: 3–4 reads + the standard composition-commit transaction —
  - SELECT fhir_mapping (resource_type + profile match, NULL-profile default fallback; none → 404)
  - EHR-by-subject lookup (`get_ehrs_for_subject`; miss → a full EHR-create commit transaction first)
  - WebTemplate: moka cache hit = 0, miss = 1 SELECT template_store (lower(template_id) match)
  - then the COMPOSITION commit tx (the folded write path — contribution + audit + vo_version + node rows, + the event_outbox row in the same tx when eventing/outbound is enabled; see the EHR-chapter map for its internals)
notes: 201 with an informational OperationOutcome + `ETag`/`Location` pointing at the
openEHR COMPOSITION; validator rejections → 422 with the validator message verbatim in
`diagnostics` (never partially stored); mapping resolution prefers exact `meta.profile[0]`
match over the NULL-profile default.

### GET /fhir/r4/{resource_type}?patient=…[&_count=N]
chain: `fhir_search` → spine → `fhir::run` → `search` → `EhrbaseService::fhir_search` →
`fhir_search_bundle` — resolve patient scope → per enabled mapping: AQL
(`SELECT v/uid/value … CONTAINS VERSION v CONTAINS COMPOSITION c WHERE
c/archetype_details/template_id/value = $templateId`, template id bound as a parameter —
no injection) → per hit `version_repo::read::read_version_by_ordinal` → reverse-map
sql: per request — 0–1 SELECT ehr.subject_id (when `patient` is an EHR UUID) +
1 SELECT fhir_mapping (enabled definitions for the type); then **per mapping**:
WebTemplate (cache; miss +1) + 1 AQL SELECT; then **per result row**: the versioned read
(vo_version + node reads for one version).
notes: `patient` is mandatory (400 — explicit scope only, never generic FHIR Search);
returns a `searchset` Bundle; `total` = returned page size, no paging links (PORT-NOTEd);
type with no enabled mapping → empty Bundle, not an error.

### GET /fhir/r4/AuditEvent (ITI-81, the audit surface — not the connector)
chain: `audit_event_search` → spine → `fhir::run` → `audit_search` →
`EhrbaseService::audit_event_search` → `AuditStore::search`
sql: 2 round trips — SELECT count(*) FROM audit.audit_event (filtered) +
SELECT fhir … ORDER BY recorded_at DESC, stored_at DESC LIMIT/OFFSET
(sea-query-built; filters: recorded_at ge/le, patient_id, principal,
resource_id, outcome, action)
notes: the RESTful-ATNA **ITI-81 Retrieve ATNA Audit Event** over the local
Audit Record Repository (`[audit.store]`; 404 when off — independent of
`fhir_api_enabled`); admin-only under RBAC (enforced in-handler: the FHIR-base
template would class Clinical); returns a `searchset` Bundle of the stored
FHIR R4 `AuditEvent` documents (IHE BALP shape) with the full match `total`;
supported params `date` (ge/le) / `patient` / `agent` / `entity` / `outcome`
/ `action` / `_count` / `_offset`, unknown params ignored (lenient search).

### GET /admin/fhir_mapping
chain: `fhir_mapping_list` → spine → `run` → `fhir_mapping_list`
sql: 1 round trip — SELECT fhir_mapping ORDER BY created_at DESC

### POST /admin/fhir_mapping
chain: `fhir_mapping_create` → spine → `run` → `fhir_mapping_create`
sql: 1 round trip — INSERT INTO fhir_mapping … RETURNING (definition stored verbatim,
`resource_type`/`profile_url`/`template_id` projected into columns)
notes: definition validated on upload (400 malformed); duplicate name → 409; unknown
`template_id` (FK) → 400 "ingest the OPT first"; 201.

### GET /admin/fhir_mapping/{mapping_id}
chain: `fhir_mapping_get` → spine → `run` → `fhir_mapping_get`
sql: 1 round trip — SELECT fhir_mapping WHERE id (404 when absent)

### PUT /admin/fhir_mapping/{mapping_id}
chain: `fhir_mapping_update` → spine → `run` → `fhir_mapping_update`
sql: 1 round trip — UPDATE fhir_mapping … RETURNING (404 on miss)
notes: `name` immutable (the deployable identity).

### DELETE /admin/fhir_mapping/{mapping_id}
chain: `fhir_mapping_delete` → spine → `run` → `fhir_mapping_delete`
sql: 1 round trip — DELETE FROM fhir_mapping WHERE id (0 rows → 404)

### 2.5 FLAT / STRUCTURED composition wire — `src/formats/dispatch.rs`

Not separate endpoints: content-negotiation glue on the standard COMPOSITION
endpoints (create/update/get), engaged when `Content-Type`/`Accept` is
`application/openehr.wt.flat+json` or `.wt.structured+json` (Better/EHRbase interop
formats, `openehr_its::flat`).

### input path (FLAT/STRUCTURED body on composition create/update)
chain: composition dispatcher → `composition_from_flat` / `composition_from_structured` →
`EhrbaseService::web_template` (the one shared moka WebTemplate cache) →
`openehr_its::flat::validate_flat_other` + `from_flat`/`from_structured` → the normal
composition commit
sql: 0–2 round trips of its own — WebTemplate cache hit = 0; miss = 1 SELECT
template_store, and (when the id is not an ADL 1.4 template) a fall-back to the
ADL2/OPT2 store (`web_template_adl2_cached` → `adl2_resolve`/`adl2_get`, cached
under a dialect-namespaced key), so a commit keyed to an ADL2-registered template
also resolves + archetype-conformance-validates (`build_web_template_am24` carries
the same constraints); only after both stores miss is it the 422 "operational
template not known" (then the underlying composition operation's SQL)
notes: template id from `template_id`/`templateId` query param or the
`openEHR-TEMPLATE_ID` header (a FLAT body carries none) — absent → 400; invalid JSON →
400; well-formed-but-non-conformant simSDT/structSDT → **422** (ITS-REST
`Requests_and_responses.md` §HTTP status codes, row 422; SM `simplified_im_b`); the
`|other` open-value-set MUST-rules are enforced pre-conversion (SM SDF master02/04/05).

### output path (FLAT/STRUCTURED Accept on composition get/create/update echo)
chain: composition dispatcher → `composition_flat_response` / `composition_structured_response`
→ `web_template` (template id read from the stored `archetype_details/template_id`) →
`openehr_its::flat::to_flat` / `to_structured`
sql: 0–1 round trips (WebTemplate cache miss only)
notes: an output conversion failure is a server fault → 500 (stored data is the server's
own and must always convert).

### 2.6 SMART discovery (1) — `src/smart/discovery.rs`

### GET /ehrbase/rest/.well-known/smart-configuration
chain: pre-serialized `Bytes` closure route (built once at router assembly from
`SmartConfig` + the OIDC issuer + the FHIR base when the connector is on) → body write
sql: 0 round trips — pure function of static configuration
notes: **outside the auth layer and overload shed** (pre-auth by spec: ITS-REST
`smart_app_launch/master04` §Service Discovery; `application/json`, R-02). Gate:
`smart.enabled` — disabled yields an *empty router* (the path is absent → 404). The CDR
advertises only what its configured AS offers; it implements none of the OAuth2 endpoints.

### 2.7 OpenAPI documents + Swagger UI — `extensions/openapi.rs`

Gate: `cfg.server.swagger_ui`. **Outside auth/overload** (public discoverability surface;
inside the shared tower-http stack). No openEHR spec governs an OAS-serving endpoint —
our own surface. Owner rule honoured in-code: only server-generated documents are served,
never a vendored OAS. All documents are **prebuilt once at router assembly**
(`prebuild_docs`) and served as ref-counted `Bytes` — a request never re-runs utoipa
reflection.

### GET /ehrbase/rest/api-docs/openapi.json
chain: static-closure route serving the prebuilt composed document (every
`#[utoipa::path]` handler: ITS-REST groups + extensions + operational endpoints, with the
config-driven single security scheme — bearer JWT when OIDC, else Basic, none when auth off)
sql: 0 round trips

### GET /ehrbase/rest/api-docs/ehrbase-{slug}.openapi.json  (12 family documents)
chain: one static route per API family (openEHR — EHR/Query/Definition/Demographic/Admin
by resource path; EHRbase — Status & Management/Terminology/Party Relationships/Event
Subscriptions/Multi-tenancy/FHIR Connector/SMART Discovery by tag), each filtered from
the one composed document at assembly
sql: 0 round trips

### GET /ehrbase/rest/swagger-ui  (+ /{*file})
chain: `serve_ui_file` → `utoipa_swagger_ui::serve` (embedded dist assets; bare path
serves index.html directly — the SwaggerUi router's 303-redirect would loop with the
serve-time NormalizePathLayer, deliberately avoided)
sql: 0 round trips
notes: the spec selector offers the 12 family documents + the complete surface last.

---

## 3. PUBLIC / OPERATIONAL

### OPTIONS {base_path}  (and the bare-`/` alias)
chain: `system::options::route` closure → `SystemManifest::respond`
(`app/ehrbase-rest/src/api/system/options.rs`; manifest built once at wiring from
`cfg.server.identity` + the **live** mounted-group list — `/ehr`, `/definition`,
`/query`, `/demographic`, plus `/admin` iff enabled)
sql: 0 round trips
notes: ITS-REST System API (`system-codegen.openapi.yaml`: `OPTIONS /`, `security: []`,
the `Options` schema + `Allow` header). Mounted **above the CORS layer** (CORS treats
every OPTIONS as a preflight and would short-circuit a conformance probe); the
CORS-wrapped application is the fallback service. JSON only — an exclusively-XML Accept
→ 406 (the manifest is not a spec-typed RM object). `restapi_specs_version` /
`conformance_profile` default to the shared provenance constants (the tested contract
identity + the last machine-computed ECC verdict — the manifest must not out-claim it).

### GET /ehrbase/rest/status
chain: `overview::status::status` (`src/overview/status.rs`)
sql: 0 round trips
notes: outside auth/overload; body `{status, server_version, openehr_rest_api_version,
timestamp}` — the ITS-REST version is the shared provenance identity (the released
Release-1.1.0). No openEHR spec governs a status endpoint — our own surface. This is also
the URL the container `ehrbase healthcheck` subcommand probes.

### GET /health · GET /ehrbase/rest/status/health
chain: `root_health` / `status_health` — static `200 OK` text
sql: 0 round trips
notes: outside auth/overload; pure process-liveness probes.

### Management surface — `extensions/management/` (gate: `management.enabled`, each endpoint opt-in via its own access level; optionally on a separate internal port)

Every mounted route carries a per-route access-level layer (`Off` = not mounted,
`Public`, `Private`, `AdminOnly` — reusing the shared `Authenticator`; AdminOnly
additionally requires the configured admin scope → 403). Sits outside the API-subtree
auth/overload layers. No openEHR spec governs this — our own operational surface.

### GET /management/health
chain: `aggregate_health` → `HealthRegistry::evaluate` — indicators: `DbHealth`
(SELECT 1 ping), `MigrationsHealth` (one `to_regclass(...)` schema probe),
`AuditHealth` / `EventsHealth` (in-memory flags)
sql: 2 round trips (db ping + migrations probe); the rest is in-memory
notes: 200 UP/DEGRADED, 503 DOWN.

### GET /management/health/liveness
chain: `liveness` — constant UP body, **no I/O** (reaching the handler = alive)
sql: 0 round trips
notes: public when `probes_enabled` (no access layer).

### GET /management/health/readiness
chain: `readiness` → the same registry evaluate as aggregate health
sql: 2 round trips (as above)
notes: public when `probes_enabled`; 503 when not ready.

### GET /management/info
chain: `info_view` → `BuildInfo` (build/git/spec provenance, captured at boot)
sql: 0 round trips

### GET /management/prometheus
chain: `prometheus_text` → `PrometheusHandle::render`
sql: 0 round trips
notes: 503 when the recorder is not installed.

### GET /management/metrics · GET /management/metrics/{name}
chain: `metrics_list` / `metrics_detail` → actuator-style views over the Prometheus handle
sql: 0 round trips
notes: 404 unknown metric; 503 when no recorder.

### GET /management/env
chain: `env_view` → the redacted effective-config snapshot (built once at boot;
secrets `***` by construction)
sql: 0 round trips

### GET/POST/DELETE /management/loggers
chain: `loggers_get` / `loggers_post` / `loggers_reset` → `LogReload` (live
tracing-subscriber filter view/swap/reset)
sql: 0 round trips
notes: POST body `{"filter": "…"}`; 400 on malformed directives; 503 when no reloadable
filter is installed.

---

## APPENDIX — background paths (not request-driven)

All spawned by the binary (`app/ehrbase-server/src/main.rs`) in boot order: migrations →
ATNA sender → events publisher → telemetry samplers → FHIR outbound emitter; each owns a
shutdown handle drained on exit (5 s bound each). No openEHR spec governs any of these —
our own designs — except the ATNA drain, whose mandate is SM master02's "IHE
ATNA-compliant system log" line.

### background: startup migrations
trigger: once at boot, before serving (`db::run_migrations`, `app/ehrbase/src/db/mod.rs`);
also gates readiness via `MigrationsHealth`
loop shape: not a loop — one dedicated detached connection, closed after
sql: per boot — the BOOTSTRAP statement set (schemas + extensions), `SET search_path TO ext`
→ the embedded `ext` migrator (own `_sqlx_migrations`), `SET search_path TO ehr, ext` →
the embedded `ehr` migrator (own `_sqlx_migrations`)
notes: failure aborts boot (`anyhow` context "applying migrations").

### background: event-outbox publisher (contribution eventing)
trigger: `events.enabled` (default off); spawned at boot; single tokio task
(`app/ehrbase/src/extensions/events/publisher.rs`)
loop shape: poll every `poll_interval_ms` → subscription-topology sync → drain the outbox
to empty in batches of `batch_size` → retention prune on its own `prune_interval_secs`
cadence → sleep/shutdown select; best-effort final drain on shutdown
sql/broker per cycle:
  - 1 SELECT event_subscription WHERE enabled (every cycle — cheap local read); broker
    queue-declare/bind round trips **only** when the desired (queue, binding) set or the
    publisher's topology epoch changed (declared on connect/change, never per cycle)
  - per drain batch, one transaction: SELECT seq, envelope FROM event_outbox WHERE
    published_at IS NULL ORDER BY seq LIMIT n **FOR UPDATE SKIP LOCKED** (concurrent
    instances safe); per row, one AMQP confirmed publish **per version entry** (routing
    key per version), then UPDATE event_outbox SET published_at = now(); COMMIT
  - prune pass: 1 DELETE FROM event_outbox WHERE published_at < now() - retention
notes: strict `seq` order — a publish failure stops the batch (never skips ahead;
per-EHR ordering preserved), backs off (`backon` exponential, `publish_max_retries`), and
flips the `events` health flag (degraded-tolerable — never blocks readiness). Outbox rows
are written **inside the commit transaction** by the storage layer (PHI-free envelopes);
this task only drains. At-least-once — consumers deduplicate on
(contribution_id, version_index). Broker down at start is tolerated (rows buffer).

### background: FHIR outbound emitter
trigger: `fhir.outbound.enabled` (default off — a **separate** switch from the REST
connector because this stream carries PHI); single tokio task
(`app/ehrbase/src/extensions/fhir/outbound.rs`)
loop shape: poll every `poll_interval_ms` → process outbox rows **past its own persistent
cursor** (`fhir_outbound_cursor.last_seq` — never touches the events drainer's
`published_at` watermark) in `seq` order, batches of `batch_size`
sql/broker per cycle:
  - 1 SELECT last_seq FROM fhir_outbound_cursor + 1 SELECT seq, envelope FROM event_outbox WHERE seq > cursor ORDER BY seq LIMIT n
  - per COMPOSITION version in a row's envelope: the versioned read
    (`read_version_by_ordinal` — vo_version + node reads), 1 SELECT fhir_mapping WHERE
    template_id AND enabled, 0–1 SELECT ehr.subject_id, WebTemplate (moka; miss +1 SELECT
    template_store); one confirmed AMQP publish per enabled mapping
    (routing key `<resource_type>.<template_id>`, PHI exchange)
  - 1 UPDATE fhir_outbound_cursor SET last_seq (the fully-published prefix, persisted even
    on a mid-batch failure)
notes: at-least-once (downstream upserts by resource id). Poison rows (deterministic
reverse-map failures) are retried 5 passes then **parked** — dead-lettered to the log at
`error` and skipped by advancing the cursor — so one bad mapping never head-of-line-blocks
later commits; broker/DB failures are transient and never park. Template id is read from
the COMPOSITION body itself (the envelope's/`vo_version.template_id` column is NULL on the
commit path — PORT-NOTEd). EHR_STATUS/FOLDER versions and logical deletes are skipped.

### background: ATNA audit drain
trigger: `atna.enabled`; spawned at boot (fail-open: a transport boot failure logs an
error and the server continues un-audited); bounded `tokio::mpsc` drained by one task
(`app/ehrbase/src/system_log/sender.rs`)
loop shape: `rx.recv()` per record (event-driven, no polling); exits when every sender
clone drops (shutdown flush, 5 s bound)
sql/transport per record: 0–1 SQL — the optional subject enrichment (`resolve_subject`:
SELECT subject_id FROM ehr WHERE id, supplied by the binary as a resolver closure —
background only, never on the request path) — then render the DICOM `AuditMessage` XML,
frame RFC 5424/5425, one syslog send (UDP or TLS)
notes: the request path only `try_send`s (never awaits). Full queue maps through the
configured `FailMode`: open → drop + counter (request proceeds), closed → the REST layer
returns 503. Every loss path is metered (emitted/dropped/serialize-failed/sent/send-failed)
— SM master02 names the System Log "IHE ATNA-compliant"; silent audit loss would undermine
that, so serialize-drops are explicitly counted.

### background: telemetry samplers
trigger: always, once the pool exists (`telemetry.start_samplers`); one task
(`app/ehrbase/src/telemetry/samplers.rs`), aborted by the telemetry guard on shutdown
loop shape: fixed 5 s `tokio::time::interval` (missed ticks skipped)
sql per cycle: 0 round trips — pool gauges are in-memory pool counters
(`pool.size()`/`num_idle()`), tokio runtime gauges from the stable `Handle::metrics()`
subset; plus `PrometheusHandle::run_upkeep()` (histogram fold / idle drain); the same
values mirrored through the OTel meter when OTLP push is on (dual path)
notes: DB pool acquire-wait histograms are recorded inline on hot paths by the
`samplers::acquire` wrapper, not by this task.

---

Counts: ADMIN 2 · terminology 6 · event-subscription 5 · tenant 5 · FHIR 7 ·
FLAT/STRUCTURED 2 glue paths (input/output, on the COMPOSITION endpoints) · SMART 1 ·
OpenAPI/docs 2 + 12 family documents + the UI asset route · public/operational: OPTIONS
(2 mounts) + status/health 3 + management 11 operations · background paths 5.
