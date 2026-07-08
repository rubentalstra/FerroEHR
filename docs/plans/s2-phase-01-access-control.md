# Phase S2-01 — Access control (RBAC + ABAC)

- Status: in-progress
- Started: 2026-07-07   Owner: —
- Consumes (spec/layer): `docs/enterprise/access-control.md` (binding design);
  prior art `reference/v1` ABAC/Spring-Security (not a port target)
- Compile required: yes (compiling, tested increments — ADR-006/008)

## Objectives

Restore the enterprise access-control capability EHRbase removed at the v1→v2
break (`docs/enterprise/v1-vs-v2-delta.md` §1), as two composable layers on the
generated ITS-REST surface: coarse **RBAC** (always on when auth is enabled) and
opt-in fine-grained **ABAC** (embedded Cedar / v1-compatible remote PDP). The
acceptance behaviour is the openEHR spec surface plus the v1 external wire
contract; EHRbase is prior art, not an oracle.

## Preconditions

- [x] P11 authentication (Basic + OAuth2/OIDC) shipped (`ehrbase-rest::auth`).
- [x] ATNA audit trail shipped (the leaf-crate + total-coverage-guard pattern
  this design replicates — `docs/enterprise/atna-audit.md` §8.1/§8.3).

## Scope

In: the nine implementation steps of `docs/enterprise/access-control.md` §11.
Out: multi-tenancy (own S2 design; the tenant attribute slot is reserved),
`EHR_ACCESS`-driven policies (§10), demographic-API policies beyond the coarse
RBAC gate.

## Tasks (design steps §11)

- [x] 1. **Principal extension** (§5.1): `roles` + `claims` on `Principal`;
  jwt/basic producers + `AuthConfig` roles; unit tests (§9.2). — Done: roles
  (upper-cased) + retained JWT claim map on `Principal`; jwt mines
  `realm_access.roles` + `scope` via configurable dotted paths; Basic users gain
  a configured `roles` field (default `["USER"]`).
- [x] 2. **`ehrbase-authz` crate scaffold** (§4.1): types, `classify.rs` table +
  total-coverage guard (§9.1), `config.rs` + boot validation (§8, §9.7). — Done:
  leaf crate with `config`/`roles`/`classify`; `class_of` covers every generated
  op id (guard test over the five `ROUTES` tables); RBAC boot validation.
- [x] 3. **RBAC gate** (§5.2): classification wired into the auth middleware via
  a route-template→op map from `ROUTES`; `authorize_admin`/`is_admin_path`
  deleted; `admin_scope` kept as a deprecated alias; e2e admin-gate tests
  (§9.6 subset). — Done: RBAC gate in `auth::middleware`; deny = 403 + Principal
  on response extensions (ATNA audits it); `rbac.enabled` default true when auth
  enabled, disabling restores auth-only behaviour.
- [x] 4. **PDP seam + RemotePdp** (§5.4, §5.5): trait, fan-out semantics, reqwest
  client, wiremock contract suite (§9.3). — Done: `ehrbase_authz::engine::PolicyEngine`
  (async, fail-closed `AuthzError`); `request::{AuthzRequest, Combination}` owns the
  cartesian fan-out (all-must-permit, short-circuit, empty→permit); `remote::RemotePdp`
  is the byte-compatible v1 client (flat body, 200=permit, explicit timeouts); 7 wiremock
  contract tests.
- [x] 5. **CedarEngine** (§5.6): schema, action mapping (guarded), policy loading
  + validation, example policies, differential tests vs RemotePdp (§9.4). — Done:
  cedar-policy 4.11.2; schema built from the `ResourceKind`×`AccessMode` enums (no drift);
  `*.cedar` loaded + strict-validated at boot (invalid = refuse to start); `arc-swap`
  reload; `examples/policies/consent.cedar`; differential test proves Cedar≡RemotePdp over
  a request corpus.
- [x] 6. **Resolvers + template read-back** (§6): `template_id` into `VersionRead`
  + the read SELECTs; `AuthzResolvers` in the binary; unit tests. — Done: additive
  `template_id` on `VersionRead` + `read_current`/`read_version`/`version_at`;
  `EhrbaseService::template_of_version`; `ehrbase_rest::{AuthzResolvers, build_engine}`
  (pool/service closures wired in the binary); DB read-back test + resolver unit test.
- [x] 7. **ABAC PEP — non-query** (§5.7, §7): patient gate + pre/post checks in
  dispatch `mount`; 403/500 mapping; ATNA deny e2e (§9.5, §9.6). — Done: `dispatch::abac`
  PEP driven from the generic `mount` (pre before backend, post on success via the
  `AuditObject`); local patient gate (subject mismatch = 403 without engine call);
  403 carries `Principal` (ATNA audits it); engine failure = 500; behind `abac.enabled`.
  Patient-gate + matrix unit tests + full HTTP e2e (`tests/abac_e2e.rs`).
- [x] 8. **ABAC PEP — query** (§6.4): `SqlCtx.subject_scope` + executor attribute
  collection + post-check; projection-independence regression test. — Done:
  `SqlCtx.subject_scope` adds `ehr_id IN (SELECT id FROM ehr WHERE subject_id=$s)` at every
  VO root; `sql::build_scope` + `exec::collect_scope` gather the touched EHR/template sets
  independently of the projection; `QueryOutcome` surfaces them; the query dispatcher runs
  `abac::query_post` (template PDP fan-out, empty→permit) before serialization.
  Projection-independence DB regression test (testcontainers PG18).
- [x] 9. **Docs + close-out**: config reference (§8 table in this design doc); design
  Status flipped to *implemented (RBAC+ABAC)*; workspace gate. — Done. (Cedar-decision ADR
  content lives in access-control.md §1.1 per the orchestrator's instruction, in lieu of a
  separate `/write-adr`.)

## Exit criteria

- [ ] All nine steps complete; workspace `cargo nextest run --workspace`,
  clippy `-D warnings`, `cargo fmt`, `cargo deny`/`audit`/`machete` green.
- [ ] ABAC behind config defaults that preserve current behaviour until opt-in.

## Decisions made this phase

- Steps 1–3 (RBAC) ship as one standalone PR (independently valuable; removes the
  `// TODO(port): Stage 2 RBAC` seam). Steps 4–8 are the ABAC PR(s).
- `definition_*` and `demographic_*` generated ops classify as **Clinical**
  (any authenticated principal with ≥1 role), matching v1's "everything except
  `/rest/admin/**` + management endpoints → any authenticated user" (§2.4). Only
  the two `admin_*` ops are Admin-class among generated routes.
- The non-generated management surface keeps its own per-endpoint `AccessLevel`
  gate (unchanged); the RBAC `management_access` tri-state + `Management` class
  are modelled/tested in `ehrbase-authz` for the ABAC PR's use, not double-gated
  onto the management router in this PR.
- **Cedar decision (§1.1):** recorded in `docs/enterprise/access-control.md`
  §1.1 (the ADR content) rather than a separate `/write-adr`, per the ABAC-PR
  orchestrator instruction — Cedar over casbin for the typed, boot-validated
  schema and one language for RBAC+ABAC; the `PolicyEngine` seam keeps it swappable.
- **§7 matrix decisions the design left open, resolved during the ABAC PEP:**
  - *Composition read template source* — taken from the returned version uid
    (`AuditObject`) via the `template_of_version` resolver, not by re-parsing the
    response body (the design allowed either; the uid path is format-agnostic).
  - *Query patient gate* — enforced by the `subject_scope` SQL pre-filter (rows
    outside the caller's patient are never fetched); the post-check therefore runs
    only the template PDP fan-out, not a redundant per-EHR subject re-check
    (`// PORT NOTE:` in `dispatch::abac::query_post`).
  - *Cedar resource attributes* — a uniform `patient?`/`template?` on every
    resource entity type (populated per fan-out combination), rather than the
    design's per-kind `subject`/`template` split, for a simpler total schema that
    passes strict validation. The example policy guards optional-attribute access
    with `resource has X &&` so it validates strictly.
  - *ABAC query collection* — the touched EHR/template sets are collected across
    **all** bound VO roots (`sql::build_scope` emits an `(ehr_id, template_id)`
    column pair per root; `exec::collect_scope` unions them), so multi-variable
    CONTAINS is fully covered.

## Coverage

Fully integrated and tested — no residual TODOs:

- **Engine paths:** the `RemotePdp` wiremock contract suite (7 cases) + the
  `CedarEngine` golden/validation tests + the Cedar≡RemotePdp differential test.
- **Patient gate + matrix:** `dispatch::abac` unit tests (subject mismatch denies
  without an engine call, missing-claim 403, the pre/post matrix, uid parsing).
- **Full ABAC HTTP e2e** (`app/ehrbase-rest/tests/abac_e2e.rs`): through the
  assembled router with a bearer `patient_id` token — a composition **create**
  for another patient's EHR is a pre-check 403 (and the ATNA layer records the
  deny), an own-patient create clears the gate, a composition **read** of another
  patient's EHR is a post-check 403, an own-patient read is served, a missing
  patient claim is 403, and disabling ABAC restores today's behaviour.
- **Query path** (`app/ehrbase/tests/persistence.rs`, testcontainers PG18):
  the projection-independence regression — subject scope filters rows and the
  touched EHR/template sets are collected even when the query projects neither
  `ehr_id` nor a template path.

## Handoff for next session

RBAC (steps 1–3) is complete on `claude/s2-access-control`. Next: the ABAC PR —
the `PolicyEngine` trait + `RemotePdp` (§5.5) and `CedarEngine` (§5.6) in
`ehrbase-authz`, the resolvers/template read-back (§6), and the dispatch PEP
(§7/§6.4). `AppState` already carries an `Option<Arc<AuthzHandle>>`; the
`PolicyEngine`/resolver slots are added there next.
