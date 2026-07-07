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
- [ ] 4. **PDP seam + RemotePdp** (§5.4, §5.5): trait, fan-out semantics, reqwest
  client, wiremock contract suite (§9.3). — ABAC PR.
- [ ] 5. **CedarEngine** (§5.6): schema, action mapping (guarded), policy loading
  + validation, example policies, differential tests vs RemotePdp (§9.4). — ABAC PR.
- [ ] 6. **Resolvers + template read-back** (§6): `template_id` into `VersionRead`
  + the two SELECTs; `AuthzResolvers` in the binary; unit tests. — ABAC PR.
- [ ] 7. **ABAC PEP — non-query** (§5.7, §7): patient gate + pre/post checks in
  dispatch `mount`; 403/500 mapping; ATNA deny e2e (§9.5, §9.6). — ABAC PR.
- [ ] 8. **ABAC PEP — query** (§6.4): `SqlCtx.subject_scope` + executor attribute
  collection + post-check; projection-independence regression test. — ABAC PR.
- [ ] 9. **Docs + close-out**: config reference into the deploy docs; `/write-adr`
  for the Cedar decision (§1.1); update `v1-vs-v2-delta.md` §1 status; workspace
  gate. — ABAC PR.

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

## Handoff for next session

RBAC (steps 1–3) is complete on `claude/s2-access-control`. Next: the ABAC PR —
the `PolicyEngine` trait + `RemotePdp` (§5.5) and `CedarEngine` (§5.6) in
`ehrbase-authz`, the resolvers/template read-back (§6), and the dispatch PEP
(§7/§6.4). `AppState` already carries an `Option<Arc<AuthzHandle>>`; the
`PolicyEngine`/resolver slots are added there next.
