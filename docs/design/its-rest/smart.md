# SMART App Launch on openEHR — greenfield register + design (W-3e)

Owner directive 2026-07-12: the ITS-REST **development edition** is audited
specification-by-specification ahead of the `ehrbase-rest` rewrite, and **SMART
App Launch is an implementation target, not merely a record**. Nothing of SMART
exists in the codebase today — this document is therefore a greenfield
requirements register + integration design, mirroring the method of the
`ehrbase-sm` chapter register ([`../sm-platform/`](../sm-platform/README.md)).

**Spec oracle** (development edition, DEVELOPMENT maturity — read in full before
any change):

- `docs/specs/openehr/ITS-REST/docs/smart_app_launch/master02-overview.adoc`
  (background, glossary, the seven required capabilities, foundational concepts)
- `.../master03-registration.adoc` (application registration)
- `.../master04-service_discovery.adoc` (`.well-known/smart-configuration`,
  authentication endpoints, `services`, `capabilities`)
- `.../master05-application_types.adoc` (confidential/public; patient-/
  practitioner-facing/backend)
- `.../master06-authentication.adoc` (OAuth2 flows, client-auth methods,
  deprecated flows)
- `.../master07-authorization.adoc` (standalone vs embedded-iframe launch,
  context selection, `launch`/`launch/patient` scopes, `ehrId` token param)
- `.../master08-scopes.adoc` (the resource-scope grammar — the load-bearing
  normative surface for a CDR)
- `.../master09-experimental_features.adoc` (launch-param-as-token,
  episode context)
- `.../master01-preface.adoc` (status/TBD convention),
  `.../master00-amendment_record.adoc` (1.1.0, 20 May 2025)

---

## 1. The architectural boundary (read this first)

SMART's *Platform* is a **composite**: master02 §Glossary defines it as "a
software ecosystem comprising at minimum an **Authorization Server**, an openEHR
**CDR**, and a **FHIR Server**". **This codebase is only the openEHR CDR** — the
`org.openehr.rest` service of master04 §Services. It is an OAuth2 **resource
server**, not the Authorization Server and not the Launcher.

That boundary decides what is *implementable here* versus what is an external
component our CDR only *advertises or consumes*:

| SMART responsibility (master) | Owner in the SMART topology | In this CDR? |
|---|---|---|
| Application registration (m03) | Authorization Server / registration portal | **No** — out-of-band (m03 recommends against DCR) |
| `authorization_endpoint`, `token_endpoint`, token issuance, PKCE / `client_secret` / client-credentials / JWT-bearer **grants** (m06) | Authorization Server | **No** — m06 §1 places identity mechanisms "outside the scope"; our JWT layer only *validates* issued tokens (`access/authn/jwt.rs:9`) |
| `introspection_endpoint`, `revocation_endpoint`, `management_endpoint` (m04) | Authorization Server | **No** — advertised, not implemented |
| Launch sequences, **context selection** UI, EHR/Episode picker (m07) | Launcher / Authorization Server | **No** — the CDR receives resolved context in the token, it does not select it |
| Embedded-iframe launch, base64-JSON `launch` param (m07/m09) | Launcher ↔ Application | **No** — consumed by the Application, not the CDR |
| **Service discovery** `.well-known/smart-configuration` (m04) | Platform (the gateway/base URL) | **Yes** — the CDR can serve it (advertising the external AS endpoints + its own `services.org.openehr.rest`) |
| **Access-token validation** (m06) | Resource server | **Yes — already present** (`access/authn/jwt.rs`) |
| **Scope enforcement** — the resource-scope grammar (m08) | Resource server | **Yes — MISSING, the core new work** |
| **Context enforcement** — bind `ehrId`/`patient` token context to the patient compartment (m07/m08) | Resource server | **Yes — MISSING** |

So the CDR-side SMART surface is exactly three things: **(1) serve the discovery
document, (2) parse + enforce the SMART resource-scope grammar, (3) bind the
launch-context claims to the patient compartment** — all on top of the existing
`access/` stack. Everything else is a cited PORT NOTE (§6).

---

## 2. Requirements register (normative extraction)

`[M]` = MUST / normative, `[S]` = SHOULD / recommended, `[E]` = experimental
(m09), `[X]` = out of CDR scope per §1 (recorded, not built here).

### Service discovery (m04)

| # | Requirement | Level | Spec |
|---|---|---|---|
| R-01 | Serve `/.well-known/smart-configuration` **relative to the Platform base URL** (the gateway), not the FHIR base — if base is `…/gateway/v1`, the doc is at `…/gateway/v1/.well-known/smart-configuration` | M | m04 §Service Discovery (¶3-4) |
| R-02 | Response `Content-Type` MUST be `application/json` | M | m04 §Service Discovery (¶ before example) |
| R-03 | Advertise the OIDC/FHIR-SMART metadata keys: `issuer`, `jwks_uri`, `authorization_endpoint`, `grant_types_supported`, `token_endpoint`, `token_endpoint_auth_methods_supported`, `registration_endpoint`, `scopes_supported`, `management_endpoint`, `response_types_supported`, `introspection_endpoint`, `revocation_endpoint`, `capabilities`, `code_challenge_methods_supported` | M | m04 §Authentication Endpoints |
| R-04 | Include a `services` map; **`org.openehr.rest` is required** (its `baseUrl` = the openEHR REST base); `org.fhir.rest` recommended; keys are reverse-domain names; each value may carry `baseUrl`(req)/`description`/`version`/`documentation`/`openapi` | M (openehr.rest) / S (fhir.rest) | m04 §Services |
| R-05 | Advertise `capabilities`, including the openEHR extensions actually supported: `context-openehr-ehr`, `openehr-permission-v1`, and (if enabled) `context-openehr-episode` `[E]`, `launch-base64-json` `[E]` | M | m04 §Capabilities |
| R-06 | The discovery endpoint SHOULD always be available at the Platform base | S | m04 §Service Discovery (¶2) |

### Authentication (m06) — resource-server obligations only

| # | Requirement | Level | Spec |
|---|---|---|---|
| R-07 | Validate presented access tokens (signature, `iss`, `aud`, `exp`); identity-verification mechanism (OIDC) is implementation-specific/out of scope | M/[X] | m06 §Authentication (¶1) |
| R-08 | Advertise supported flows in `.well-known` so clients can choose | M | m06 §Supported Authentication Flows (¶ last) |
| R-09 | Implicit Grant and Resource-Owner-Password-Credentials **MUST NOT** be used | M | m06 §Deprecated Flows |
| R-10 | The grant flows themselves (Auth-Code+PKCE, Auth-Code+`client_secret`, Client-Credentials, JWT-Bearer) + client-auth methods (asymmetric `private_key_jwt`, symmetric `client_secret_basic`) are Authorization-Server duties | [X] | m06 §§Supported Flows / Client Authentication Methods |

### Authorization & context (m07)

| # | Requirement | Level | Spec |
|---|---|---|---|
| R-11 | Recognise `launch` (embedded-iframe) and `launch/patient` (patient context) launch scopes in the request; `launch/episode` is experimental | M/[E] | m07 §§Context Selection / Embedded iFrame Launch |
| R-12 | The **resolved context** arrives with the access token: `ehrId` (the openEHR EHR instance for the selected Patient) and optionally `episodeId` `[E]`; the CDR consumes these, it does not select them | M | m07 §Context Selection (token-response table) |
| R-13 | Standalone vs Embedded-iframe launch, `iss`/`launch` parameters, the context-selection prompt/consent screen | [X] | m07 §§SMART Authorization Flow / Embedded iFrame Launch |

### Scopes (m08) — the load-bearing CDR surface

| # | Requirement | Level | Spec |
|---|---|---|---|
| R-14 | The Platform MUST validate requested scopes against the application registration, applicable access-control policy, and the authenticated user's permissions | M | m08 §Scopes (¶2) |
| R-15 | Enforce the resource-scope grammar `<compartment>/<resource>.<permission>` (full grammar in §3.2) | M | m08 §Resource Scopes |
| R-16 | Compartments: `patient` (restrict to the current EHR/patient in context), `user` (the authenticated user's security profile), `system` (backend, all data) | M | m08 §Resource Scopes (compartment list) |
| R-17 | Resource types: `template-<templateId>`, `composition-<templateId>`, `aql-<queryName>`; `<templateId>`/`<queryName>` support `*`/`**` glob + namespace patterns (`MyHospital::*`, `*::Template.v0`, `*`) | M | m08 §Resource Scopes (resource + pattern tables) |
| R-18 | Permissions: `c`=create, `r`=read, `u`=update, `d`=delete, `s`=search/execute | M | m08 §Resource Scopes (permission list) |

### Registration & application types (m03/m05)

| # | Requirement | Level | Spec |
|---|---|---|---|
| R-19 | Client registration (metadata submission, `client_id`/`client_secret`/`jwks`/`jwks_uri` issuance); out-of-band, DCR not required | [X] | m03 (whole) |
| R-20 | Confidential vs public client typing; patient-/practitioner-facing/backend interaction typing (drives flow selection at the AS) | [X] | m05 (whole) |

### Experimental (m09)

| # | Requirement | Level | Spec |
|---|---|---|---|
| R-21 | `launch` param MAY be a **base64-encoded JSON** object carrying `ehrId`/`patient`/`episodeId`; advertise `launch-base64-json`. Consumed by the Application, not the CDR | E | m09 §Launch Parameter as a Token |
| R-22 | Episode context: `launch/episode` scope, `context-openehr-episode` capability, `episodeId` token param — "semantics currently implementation-defined"; openEHR has no first-class Episode resource yet | E | m09 §Experimental: Episode Context |

---

## 3. Current state (what the `access/` stack already provides)

Verified 2026-07-12; file:line for every claim.

### 3.1 Present and reusable

- **Access-token validation** — `access/authn/jwt.rs`: signature + `iss`/`aud`/
  `exp` validation via `jsonwebtoken`, three key sources (HMAC / static JWKS /
  OIDC-discovered JWKS, `jwt.rs:38-45,149-220`); `RS256` default
  (`access/authn/config.rs:99,113`). The "a CDR only validates; token issuance
  is a client concern" posture is already recorded (`jwt.rs:9`) — **exactly the
  §1 boundary** (satisfies R-07; R-10 correctly out of scope).
- **Scopes are already captured** — the space-delimited `scope` string + the
  `scp` array land on `Principal.scopes` (`jwt.rs:127-134`); the full validated
  claim map is retained on `Principal.claims` (`jwt.rs:117`,
  `access/authn/mod.rs:56`). So the raw material for R-15 and the R-12 context
  claims is already on the principal — but **nothing parses or enforces it**.
- **A coarse RBAC gate** — `access/authz/roles.rs` + `RbacGate`
  (`access/authz/mod.rs:227-286`), classifying every generated op
  (`access/authz/classify.rs:class_of`).
- **A fine-grained ABAC PEP** — `dispatch/abac.rs`: per-op pre/post checks over
  `ResourceKind × AccessMode` (`access/authz/request.rs:13-74`,
  `classify.rs:kind_of/access_of`) with resolved `patient`/`template`/
  `organization` attributes, enforced by an embedded Cedar engine
  (`access/authz/cedar.rs`) or a remote PDP. **This is a near-exact structural
  match for the SMART scope model** (§3.3).
- **A patient/subject gate** — `dispatch/abac.rs:303` (`subject_gate`): compares
  a configured patient claim against the target EHR's subject external ref via
  a resolver (`access/authz/mod.rs:70` `SubjectFn`). This *is* the SMART
  `patient` compartment, minus the SMART context-claim binding.
- **A Cedar principal that already declares `scopes: Set<String>`** but leaves
  it empty (`cedar.rs:88-90` schema, `cedar.rs:223-231` "Declared for advanced
  policies; empty under the v1 wire attribute model") — a ready hook to feed
  parsed SMART scopes into policy.
- **The AND-composition layering** the design needs is already the documented
  model: `EHR_ACCESS` gate → RBAC → ABAC, each an additive restriction
  (`access/mod.rs:37-44`).
- **A public, pre-auth mount seam** for `.well-known` — the status router is
  merged *outside* the auth layer (`router.rs:83-85`,
  `overview/status.rs:81-86`); `.well-known/smart-configuration` (unauthenticated)
  belongs on the same seam.
- **The extension-group config pattern** to copy — opt-in `enabled` flags with
  `404`-when-disabled, `EHRBASE_REST_<GROUP>__*` (`config.rs:111-149`,
  terminology/admin/fhir/event_subscription).

### 3.2 Missing (the whole SMART surface)

- **No `.well-known/smart-configuration` endpoint** (grep: zero hits for
  `well.known`/`smart`/`authorization_endpoint` outside the OIDC-discovery
  comments in `jwt.rs`/`config.rs`). R-01–R-06 unmet.
- **No SMART scope grammar** — `Principal.scopes` is consumed only by the
  deprecated `admin_scope` back-compat seam (`config.rs:32-38`); the
  `<compartment>/<resource>.<permission>` grammar (R-15–R-18) is neither parsed
  nor enforced.
- **No launch-context binding** — no code reads an `ehrId`/`patient` SMART
  context claim (R-12); the ABAC patient gate is keyed by an operator-configured
  `patient_claim`, not the SMART context claim.
- **No `SmartConfig`**, no capabilities advertisement, no episode/base64-json
  handling.

### 3.3 The scope→authz correspondence (why this is mostly wiring)

The SMART scope axes map onto the existing ABAC axes almost one-to-one:

| SMART element (m08) | Existing seam |
|---|---|
| compartment `patient` | the ABAC subject gate (`dispatch/abac.rs:303`) + the R-12 context claim |
| compartment `user` | RBAC role model (`access/authz/roles.rs`) + the caller's ABAC attributes |
| compartment `system` | a client-credentials principal (no user context) → compartment-unrestricted |
| resource `composition-<tid>` | `ResourceKind::Composition` + the resolved `template` attr (`request.rs`, `abac.rs:pre_template/post_template`) |
| resource `template-<tid>` | the `definition_*` template ops (today RBAC-only, `classify.rs:96`) |
| resource `aql-<queryName>` | `ResourceKind::Query` + the stored-query name (`dispatch/abac.rs:query_pre/query_post`) |
| permission `c`/`r`/`u`/`d` | `AccessMode::Create`/`Read`/`Update`/`Delete` (`request.rs:47`) |
| permission `s` | `AccessMode::Execute` (query) / search |
| glob `*`/`**`, `ns::*` | a glob matcher over templateId/queryName |

The enforcement point already resolves op → kind → access → template → patient
(`dispatch/abac.rs`), i.e. exactly the axes a SMART scope constrains — so the
scope gate rides that PEP rather than duplicating resolution.

---

## 4. Target design

New module `app/ehrbase-rest/src/smart/`, config-gated, composed **onto** the
existing `access/` stack (never replacing it).

```
app/ehrbase-rest/src/smart/
├── mod.rs         # module docs (spec map, boundary §1, PORT NOTE register), re-exports
├── config.rs      # SmartConfig (EHRBASE_REST_SMART__*), boot validation
├── discovery.rs   # GET /.well-known/smart-configuration handler + body model (R-01..R-06)
├── scope.rs       # SMART scope grammar: SmartScope parse + glob matcher (R-15..R-18)
└── enforce.rs     # scope→authz mapping helpers consumed by the ABAC PEP (R-14, R-16)
```

### 4.1 Service discovery (`smart/discovery.rs`) — R-01..R-06

- A public GET handler mounted on the pre-auth seam (`router.rs` alongside
  `status::router`), path `/.well-known/smart-configuration` **relative to the
  configured Platform base** (`SmartConfig.platform_base_url`, default = the
  REST root `overview/status.rs`-style derivation). `application/json` (R-02).
- The body is **assembled from config, not invented**: the AS endpoints
  (`issuer`/`authorization_endpoint`/`token_endpoint`/`jwks_uri`/…) are
  copied from `SmartConfig` (operator supplies the external OIDC provider's
  values, or we derive them from the existing `oidc.issuer` via OIDC discovery,
  reusing `jwt.rs` `RemoteJwks`/`CoreProviderMetadata`). `services.org.openehr.rest.baseUrl`
  = the CDR `base_path` (R-04, the one value the CDR authoritatively owns);
  `org.fhir.rest` advertised only when the FHIR connector is enabled
  (`config.rs:143` `FhirConfig`).
- `scopes_supported`/`grant_types_supported`/`capabilities` reflect **what the
  CDR actually enforces** — never a capability we do not honour (R-05/R-08):
  `openehr-permission-v1` (once §4.3 ships), `context-openehr-ehr`, the
  `client-*` methods the AS supports; `context-openehr-episode` and
  `launch-base64-json` only when their sub-flags are on (§4.4).

### 4.2 Scope grammar (`smart/scope.rs`) — R-15..R-18

A typed, total parser (no regex hand-rolling for the codec; `urlencoding` is the
percent-decode owner):

```
SmartScope = Launch                        // "launch"
           | LaunchContext(Ctx)            // "launch/patient" | "launch/episode"[E]
           | Identity(String)              // "openid" | "profile" | "offline_access"
           | Resource { compartment, resource, permissions }
Compartment = Patient | User | System
Resource    = Template(Pattern) | Composition(Pattern) | Aql(Pattern)
Permission  = C | R | U | D | S            // parsed as a set from the ".xyz" tail
Pattern     = a glob over `ns::name` with `*` (segment) and `**` (recursive)
```

Unrecognised scope strings are retained but inert (forward-compat; SMART
tolerates non-normative standard SMART scopes per m07 note). The matcher
implements the m08 pattern table exactly (`*::Template.v0`, `MyHospital::*`,
`*`). Unit-tested against every example row in m08.

### 4.3 Scope + context enforcement (`smart/enforce.rs` + the ABAC PEP) — R-14, R-11, R-12, R-16

**One enforcement point**, AND-composed after RBAC and before/with Cedar ABAC
(preserving the `access/mod.rs:37` layering). Realised by extending
`dispatch/abac.rs` so the SMART scope check runs on the same op→kind→access→
template/patient resolution the ABAC PEP already performs:

1. **Parse** `Principal.scopes` into `Vec<SmartScope>` once per request (cached
   on the principal or a request extension).
2. **Resource scope gate** (R-15): the operation's `(ResourceKind, AccessMode)`
   (`classify.rs`) plus its resolved template/query id must be permitted by at
   least one granted `Resource` scope whose compartment the caller satisfies,
   whose resource family matches, whose pattern matches the template/query id,
   and whose permission set contains the mode (`c/r/u/d/s`). No matching scope →
   `403` (reusing `abac.rs:forbidden`, principal attached for ATNA).
3. **Compartment binding** (R-16, R-12):
   - `patient/` → require the SMART context claim (`SmartConfig.ehr_id_claim`,
     default `ehrId`; fall back to `patient`) and enforce it against the target
     EHR via the existing `subject_gate` (`abac.rs:303`) / EHR-Index subject
     resolver. A `patient/` scope with no resolvable context claim → `403`.
   - `user/` → the caller's existing RBAC/ABAC profile (no extra compartment
     restriction).
   - `system/` → a client-credentials principal (no `sub`-user); compartment
     unrestricted, still subject to RBAC + resource-scope + ABAC.
4. **Feed Cedar** (optional): populate the already-declared but empty
   `User.scopes` Cedar attribute (`cedar.rs:223-231`) with the granted scope
   strings, so operator policies can additionally reason over scopes. Purely
   additive — the built-in gate (steps 2-3) is the spec-mandated floor.

`launch`/`launch/patient` (R-11) are validated as *requested* context markers;
the CDR does not perform selection (R-13, §1) — it only checks that a
`patient`-compartment call carries the resolved `ehrId` context (R-12).

### 4.4 Config (`smart/config.rs`)

```
[smart]                              # EHRBASE_REST_SMART__*
enabled              = false         # 404 on /.well-known/smart-configuration when off; no scope gate
platform_base_url    = <rest root>   # R-01 gateway base for the discovery path
ehr_id_claim         = "ehrId"       # R-12 launch context claim (fallback "patient")
require_smart_scopes = false         # when true, a token with no SMART resource scope is denied (fail-closed)
episode.enabled      = false         # [E] R-22 advertise context-openehr-episode + accept launch/episode
launch_base64_json   = false         # [E] R-21 advertise launch-base64-json
# discovery advertisement (else derived from auth.oidc via discovery):
authorization_endpoint / token_endpoint / registration_endpoint /
introspection_endpoint / revocation_endpoint / management_endpoint  (all optional)
```

Off by default → a stock server is byte-identical to today (the extension-group
convention, `config.rs:95-149`). `require_smart_scopes=false` keeps SMART
advisory (enforce only when a scope is present) until an operator opts into
fail-closed; a boot-validation warns if `enabled` but no OIDC bearer is
configured (SMART scopes only ride Bearer tokens; Basic carries none).

### 4.5 Verification

- **Unit**: scope parser + glob matcher over every m08 example row; discovery
  body shape (R-03/R-04 keys present, `org.openehr.rest` required); capability
  list = enforced set.
- **Integration** (`tests/`): discovery `200`/`application/json` when enabled,
  `404` when off; a `patient/composition-*.r` token reads only its context EHR
  (403 cross-patient); `user/` vs `system/` compartment behaviour; permission
  mismatch (`.r` token → `403` on create); wildcard scope breadth; scope gate
  AND-composes with RBAC + Cedar.
- **ECC**: a new `SMART`/`SEC`-adjacent area — discovery-doc assertions and the
  scope/context 401/403 matrix over the live SUT (mirrors the existing
  `ECC-SEC-*` auth cases); zero `skipped` outcomes (W-2 ruling).
- Gates: workspace suites green, clippy clean, ECC zero-drift; website book page
  for the new `EHRBASE_REST_SMART__*` config + discovery endpoint (same-PR docs
  rule).

---

## 5. Work plan (execution order)

1. **`SmartConfig` + discovery endpoint** (§4.1, §4.4): the one surface fully in
   CDR scope; public route on the pre-auth seam, config-assembled body,
   capabilities = enforced set. Unit + integration + book page. (R-01..R-08)
2. **Scope grammar** (§4.2): `SmartScope` parse + glob matcher, exhaustively
   tested against m08. (R-15..R-18)
3. **Scope + context enforcement** (§4.3): extend the ABAC PEP with the resource
   scope gate and the `patient`-compartment context binding, AND-composed with
   RBAC/Cedar; feed `User.scopes` into Cedar. (R-11, R-12, R-14, R-16)
4. **Capabilities reconciliation + ECC** (§4.5): advertise only enforced
   capabilities; add the ECC SMART area.
5. **Experimental, behind sub-flags** (§4.4): advertise `launch-base64-json` +
   accept a base64-JSON `launch` claim pass-through (R-21); `context-openehr-episode`
   + `launch/episode` + carry `episodeId` context (R-22) — **provisional, no
   semantic enforcement** (see §6).

Exit: every R-row implemented or a re-verified cited PORT NOTE; suites + ECC
zero-drift; discovery + config documented on the website; WORKLIST row linked to
the merged PR.

---

## 6. PORT-NOTE residue (the honest boundary)

Recorded verbatim; each is a deliberate scope decision, not a silent gap.

- **The Authorization-Server / Launcher role is out of scope** (§1, R-10, R-13,
  R-19, R-20). m06 §Authentication places identity mechanisms "outside the scope
  of this specification"; this CDR is the `org.openehr.rest` resource server +
  discovery advertiser only. It **validates** tokens and **enforces** scopes; it
  does not issue tokens, register clients, run the authorization/token/
  introspection/revocation/management endpoints, perform launch sequences, or
  render the context-selection UI. The discovery document *advertises* those
  external endpoints from config — it does not implement them. *No openEHR spec
  requires a CDR to be the Authorization Server; SMART's Platform is explicitly a
  multi-component ecosystem (m02 §Glossary).*
- **Dynamic Client Registration is not built** (R-19). m03 §Registration: "the
  current recommendation is to handle registration out-of-band"; RFC 7591 DCR is
  named only as a possibility. No `/register` endpoint.
- **Episode context is experimental and NOT semantically enforced** (R-22). m09
  §Episode Context states episode "semantics are currently implementation-
  defined" and that openEHR "formal resource definitions and operational
  semantics are still evolving" (no first-class Episode resource). Behind
  `smart.episode.enabled`, the CDR advertises `context-openehr-episode`, accepts
  the `launch/episode` scope, and carries the `episodeId` context claim — but
  applies no episode-scoped filtering. *Spec gap recorded verbatim; revisit when
  openEHR formalises Episode.*
- **base64-JSON launch param is advertised, not consumed** (R-21). m09
  §Launch Parameter as a Token: the base64-JSON `launch` object is consumed by
  the *Application* to initialise its UI; the CDR only advertises
  `launch-base64-json` and passes any decoded context claim through.
- **`org.fhir.rest` is recommended, not required** (R-04). Advertised only when
  the FHIR connector is enabled (`config.rs:143`); a pure openEHR deployment
  advertises just `org.openehr.rest`.
- **Discovery path vs the nested `base_path`** (R-01). m04 fixes the discovery
  doc at the *Platform* base (the gateway), while the CDR's own REST base is
  `/ehrbase/rest/openehr/v1`. The Platform base is a deployment/gateway concern;
  we serve discovery at `SmartConfig.platform_base_url` (default the REST root)
  and note m04's recommendation that SMART-on-FHIR clients expect `iss` == the
  FHIR base — an operator alignment concern, "outside the scope of this
  specification" (m04 note).
- **The spec is DEVELOPMENT maturity** (m01 §Status; m00: 1.1.0, 20 May 2025).
  The register above is pinned to the vendored `e8a093e9` text; re-verify on any
  re-vendor. m01's TBD convention flags no in-text `*TBD*` paragraphs in the
  vendored masters — the material open items are the two `[E]` experimental
  features above.
