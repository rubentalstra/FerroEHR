# RFC-grounded OAuth2 + RBAC/ABAC rewrite (tracker issue #1963)

- Status: in-progress
- Started: 2026-08-06
- Consumes: the official RFCs (6749, 6750, 7515, 7517, 7519, 7617, 8414, 8725,
  9068, 9110), OpenID Connect Discovery 1.0, NIST ANSI/INCITS 359 (RBAC) +
  NIST SP 800-162 (ABAC), the Cedar authorization docs, and the pinned crate
  docs (`jsonwebtoken` 11.0.0, `openidconnect` 4.0.1, `oauth2` 5.0.0,
  `cedar-policy` 4.12.0, `argon2` 0.5.3) — plus, for the spec-facing wire
  half only, `docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
  §Authentication and authorization, `docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc`
  §General Assumptions, and `docs/specs/openehr/ITS-REST/docs/smart_app_launch/master06`–`master08`.

This is a **full rewrite** of the access layer, not a comment sweep: the
authentication path is rebuilt so every check carries an RFC clause, and the
authorization model is re-declared as our own recorded design in NIST
vocabulary. The provenance sweep the issue names falls out of the rewrite —
after it there is nothing left in the layer whose authority is "what the
upstream Java CDR did".

---

## 1. Scope + non-goals

**Scope.** Rewrite `app/ferroehr-rest/src/extensions/access/**` (`authn/`,
`authz/`, `pep.rs`, `mod.rs`), `app/ferroehr/src/config/auth.rs`, and
`app/ferroehr/src/config/authz.rs` so that (a) the bearer-token path implements
the RFC 6750 / 7515 / 7517 / 7519 / 8414 / 9068 resource-server duties in an
explicit, ordered, test-pinned pipeline whose every step names its clause;
(b) the Basic path implements RFC 7617 with an OWASP-grounded KDF posture;
(c) the challenge/status surface implements RFC 6750 §3–§3.1 and RFC 9110
§11.6.1 / §15.5.2 / §15.5.4; (d) the authorization model is re-declared as our
own design in NIST SP 800-162 vocabulary (PEP/PDP/PIP/PAP) with the policy
combining algorithm and the deny-by-default posture stated explicitly and
matching what `cedar-policy` actually does; and (e) every surviving design
comment cites an RFC/crate doc/vendored spec section, or carries the explicit
"no openEHR spec governs this — our own design/extension" flag. `docs/architecture.md`
§access records the resulting model (issue acceptance criterion 3).

**Non-goals.** No hand-rolled crypto, token parsing, base64, or percent
codec — `jsonwebtoken`/`openidconnect`/`argon2`/`base64`/`urlencoding` keep
every byte-level duty. **No new dependency**: the rewrite uses only crates
already in the root `Cargo.toml` `[workspace.dependencies]`. **No change to
the layer ORDER**: `EHR_ACCESS` → authn → RBAC → ABAC → SMART narrowing stays
exactly as `app/ferroehr-rest/src/router.rs:110-129` and
`app/ferroehr-rest/src/api/mod.rs:213-221` compose it, and SMART keeps
narrowing only. **No widening of what is authorized** — every behaviour change
below either refuses something previously accepted or converts a silent permit
into a loud refusal; not one grants access that is refused today. Also out of
scope: becoming an authorization server (token issuance, RFC 6749 grants — the
CDR is the resource server, RFC 6749 §1.1), token introspection (RFC 7662),
sender-constrained tokens (DPoP/mTLS), replay tracking on `jti`, and any change
to `ehr_access.rs` / `tenant.rs` semantics.

---

## 2. Current state, audited

Every design decision in the access layer, where it lives, the authority it
currently claims, and its real authority. `keep` = correct and correctly
grounded (or only the citation is added). `re-ground only` = behaviour is
right, the stated authority is wrong/absent/internal/EHRbase-derived.
`change behaviour` = the code does the wrong thing.

`ITS-REST §A&a` = `docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
§Authentication and authorization. `SM §GA` = `docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc`
§General Assumptions.

### 2.1 Authentication surface (challenge, status, transport)

| # | Decision | Where | Claimed authority | Real authority | Verdict |
|---|---|---|---|---|---|
| 1 | authn is a `SHOULD`; when a framework is present the service MUST use `WWW-Authenticate` and answer 401/403/407 | `authn/mod.rs:6-13` | ITS-REST §A&a | ITS-REST §A&a **+ RFC 9110 §15.5.2** ("The origin server MUST generate a WWW-Authenticate header field containing at least one challenge") | keep |
| 2 | "a `403` (authenticated-but-refused) MUST NOT [carry a challenge]" | `authn/mod.rs:461-463` | ITS-REST §A&a | **none** — ITS-REST §A&a says services MUST *use* the headers "returning 403/401/407 as applicable"; RFC 9110 §15.5.4 only says a server wanting to signal that credentials would help should send 401. RFC 6750 §3.1 **defines `insufficient_scope` as a 403 carrying the challenge** | change behaviour |
| 3 | static challenge `Basic realm="ferroehr", Bearer` | `authn/mod.rs:238-251` | — | RFC 7617 §2 (`realm` REQUIRED ✓); RFC 7617 §2.1 (`charset="UTF-8"` — absent); RFC 6750 §3 (`error`/`error_description` — absent) | change behaviour |
| 4 | challenge falls back to `Basic realm="ferroehr"` when no mechanism is configured; boot only `warn`s | `authn/mod.rs:246-248`, `app/ferroehr-server/src/lib.rs:400-405` | — | **none** — RFC 9110 §11.6.1 requires a challenge *applicable to the target resource*; advertising an unimplemented scheme is not | change behaviour |
| 5 | `Authorization` scheme matched case-insensitively | `authn/mod.rs:270-274` | — | RFC 9110 §11.1 (scheme names are case-insensitive) | keep |
| 6 | bearer token = the header with the first non-whitespace run trimmed off, then `.trim()` | `authn/mod.rs:323-327` | — | RFC 6750 §2.1 `credentials = "Bearer" 1*SP b64token`, `b64token = 1*( ALPHA / DIGIT / "-" / "." / "_" / "~" / "+" / "/" ) *"="` — not enforced | change behaviour |
| 7 | `KeyResolution` (JWKS/discovery unreachable) → 401 | `authn/mod.rs:154-158`, `config/auth.rs:132-139` | — | **none** — an issuer outage is a dependency failure, not a statement about the credential; the sibling `VerificationUnavailable` → 500 (`authn/mod.rs:154`) already establishes the rule. RFC 9110 §15.6.4 grounds 503 | change behaviour |
| 8 | verified-credential cache: SHA-256 of the presented header → `Principal`, TTL-bounded | `authn/mod.rs:170-178,281-314` | — | no spec governs this — our own design (the TTL is the revocation-lag bound); RFC 7617 §4 password-storage guidance is adjacent | re-ground only |
| 9 | Argon2 runs on `spawn_blocking` | `authn/mod.rs:301-311` | — | no spec — our own design (tokio blocking-pool docs) | keep |
| 10 | Basic base64 decode is padding-**indifferent** | `authn/basic.rs:26-34` | RFC 7617 gives no reason to reject unpadded | RFC 7617 §2 cites RFC 4648 §4 (canonical form is padded); accepting unpadded is our own leniency | re-ground only (see §8 Q4) |
| 11 | unknown username returns before any KDF work | `authn/basic.rs:61-65` | — | **none** — a username-enumeration timing oracle; RFC 7617 §4 requires only that stored passwords not be trivially recoverable | change behaviour |
| 12 | `Argon2::default().verify_password` — cost parameters come from the stored PHC string (`argon2` 0.5.3 `impl TryFrom<&PasswordHash> for Params`), with no floor | `authn/basic.rs:67-71` | — | OWASP Password Storage Cheat Sheet §Argon2id: minimum `m=19456 (19 MiB), t=2, p=1` (= `argon2` 0.5.3 `Params::DEFAULT_M_COST = 19 * 1024`, `DEFAULT_T_COST = 2`, `DEFAULT_P_COST = 1`) — a weak stored hash verifies happily | change behaviour |
| 13 | `split_once(':')` — a userid can never contain a colon | `authn/basic.rs:57-59` | — | RFC 7617 §2 ("A user-id containing a colon character is invalid") | keep |
| 14 | a Basic principal carries no scopes and no claims | `authn/basic.rs:82-89` | — | own design (Basic conveys no OAuth2 grant) — RFC 7617 defines no claim carriage | keep |

### 2.2 JWT validation pipeline

| # | Decision | Where | Claimed authority | Real authority | Verdict |
|---|---|---|---|---|---|
| 15 | `header.alg` must be a member of the configured algorithm list | `authn/jwt.rs:110-115` | — | RFC 8725 §3.1 ("MUST enable the caller to specify a supported set of algorithms and MUST NOT use any other algorithms") | keep |
| 16 | `Validation::new(header.alg)`, then `validation.algorithms` overwritten with the configured set | `authn/jwt.rs:117-118` | — | `jsonwebtoken` 11.0.0 `Validation` docs; RFC 8725 §3.1 wants the set chosen by the *caller*, so seeding from the token's own header is backwards even though the overwrite repairs it | re-ground only |
| 17 | `audiences` empty → `validation.validate_aud = false` | `authn/jwt.rs:120-124` | — | **none** — RFC 7519 §4.1.3: "If the principal processing the claim does not identify itself with a value in the `aud` claim when this claim is present, then the JWT MUST be rejected"; RFC 9068 §4 step 4 makes `aud` validation unconditional; RFC 8725 §3.9 | change behaviour |
| 18 | `required_spec_claims` left at the `jsonwebtoken` default `{"exp"}` | `authn/jwt.rs:117-124` (omission) | — | `jsonwebtoken` 11.0.0 `Validation` docs: `iss`/`aud`/`sub` "validation only occurs if the claim is present in the token" — so a token with **no** `iss` and **no** `aud` passes today. RFC 9068 §2.2 makes `iss`/`exp`/`aud`/`sub`/`client_id`/`iat`/`jti` REQUIRED | change behaviour |
| 19 | `validate_nbf` left at the `jsonwebtoken` default `false` | `authn/jwt.rs:117-124` (omission) | — | RFC 7519 §4.1.5 ("MUST NOT be accepted for processing" before `nbf`) | change behaviour |
| 20 | the `typ` header is decoded (`decode_header`) but never inspected | `authn/jwt.rs:109` | — | RFC 9068 §2.1 + §4 step 1 ("MUST verify that the `typ` header value is `at+jwt` or `application/at+jwt` and reject tokens carrying any other value"); RFC 8725 §3.11 (explicit typing), §3.12 (mutually exclusive validation rules per JWT kind) | change behaviour |
| 21 | `sub` is REQUIRED and must be non-blank, for audit attributability | `authn/jwt.rs:135-150` | RFC 7519 §4.1.2 makes `sub` optional (correctly stated) + own audit reasoning | RFC 9068 §2.2 makes `sub` REQUIRED for an access token — the stronger, on-point ground | keep (re-cite) |
| 22 | scopes = the `scope` string split on whitespace, plus every string in the `scp` array | `authn/jwt.rs:152-161` | — | RFC 9068 §2.2.3 grounds `scope`; **`scp` has no RFC** — a vendor claim we accommodate (our own design) | re-ground only |
| 23 | RBAC roles are mined from the `scope` claim as well as `realm_access.roles` | `authn/jwt.rs:164-165`, `authz/roles.rs:20-24`, `config/authz.rs:56-58,368-370` | "mirrors EHRbase v1's authority converter" / "matching the EHRbase v1 authorities converter" | **none, and it is wrong**: RFC 6749 §3.3 scopes are delegation grants, ANSI/INCITS 359 roles are subject↔permission assignments. Mining them makes the `Clinical` gate (`roles.rs:105-113`, "at least one role") vacuous for **every** OIDC token, since `openid` becomes the role `OPENID`. RFC 9068 §2.2.3.1 + RFC 7643 §4.1.2 give the standards-grounded role carriers: `roles`, `groups`, `entitlements` | change behaviour |
| 24 | JWK selection: `kid` → `JwkSet::find`, else `set.keys.first()`; doc says "only unambiguous when the set has exactly one key" but the code takes the first of any set | `authn/jwt.rs:188-196` | — | RFC 7517 §4.4 (`alg`), §4.3 (`key_ops`), §4.5 (`use`) — none consulted; RFC 8725 §3.1 (a key is used with exactly **one** algorithm, "checked when the cryptographic operation is performed") and §3.8 (keys must belong to the issuer) | change behaviour |
| 25 | JWKS positive TTL 5 min, negative TTL configurable, both cached in one `moka` slot with `get_with` coalescing | `authn/jwt.rs:198-237,266-270` | "No openEHR spec governs this — our own design" | correct as stated (own design; `moka` `Expiry` docs) | keep |
| 26 | discovery HTTP client: redirects disabled, connect + request timeouts | `authn/jwt.rs:246-252` | "SSRF hardening" | RFC 8725 §3.10 (protect against SSRF from key-URL headers) + RFC 8414 §6.2 (TLS) + own design for the timeouts | keep (re-cite) |
| 27 | `CoreProviderMetadata::discover_async(...)`, then a **second** independent `GET` of `jwks_uri` | `authn/jwt.rs:272-292` | — | `openidconnect` 4.0.1 `src/discovery/mod.rs:307-338` — `discover_async` **already** fetches the JWKS (`JsonWebKeySet::fetch_async(provider_metadata.jwks_uri(), …)`) and stores it on the returned metadata; the second GET doubles the outbound requests per refresh and re-reads an unvalidated URL | change behaviour |
| 28 | `IssuerUrl::new(cfg.issuer)` accepts any scheme | `authn/jwt.rs:273`, `config/auth.rs:103-106` | — | RFC 8414 §2: the issuer identifier "is a URL that uses the `https` scheme and has no query or fragment components"; OIDC Discovery §4/§7.1; RFC 8414 §6.2 "Implementations MUST support TLS" | change behaviour |
| 29 | issuer-match validation of the discovery document | `authn/jwt.rs:276-278` (delegated) | — | already correct via the crate: `openidconnect` 4.0.1 `src/discovery/mod.rs:383-390` returns `DiscoveryError::Validation` when `provider_metadata.issuer() != issuer_url` — exactly OIDC Discovery §4.3 / RFC 8414 §3.3 | keep (document the delegation) |
| 30 | default `algorithms = ["RS256"]` | `config/auth.rs:108-109,159-161` | — | OIDC Discovery §4.2 (`id_token_signing_alg_values_supported`, RS256 mandatory to implement) | keep |
| 31 | `hmac_secret` — a symmetric HS256 key taken from configuration | `config/auth.rs:110-114` | "the simplest key source (tests/dev)" | RFC 8725 §3.5: "Human-memorizable passwords MUST NOT be directly used as the key to a keyed-MAC algorithm such as `HS256`"; §2.2 weak symmetric keys — no length/entropy floor is enforced, and nothing prevents HS256 being accepted **alongside** a JWKS key source (RFC 8725 §2.1/§3.1 algorithm confusion) | change behaviour |
| 32 | secret/`*_file` and hmac-vs-JWKS mutual exclusion, at boot | `config/mod.rs:151-174` | — | own design (config discipline) | keep |
| 33 | `issuer` has **no** boot validation (empty string is accepted; `[auth.oidc]` with a blank issuer fails at the first request instead) | `config/auth.rs:103-106`, `config/mod.rs:151-174` | — | RFC 8414 §2 + our own boot-strictness rule | change behaviour |

### 2.3 Authorization — model, RBAC, classification

| # | Decision | Where | Claimed authority | Real authority | Verdict |
|---|---|---|---|---|---|
| 34 | authorization is spec-silent; the SM puts it out of band | `authz/mod.rs:6-11`, `pep.rs:9-14`, `config/authz.rs:7-8` | SM §GA | correct — SM §GA; no CNF profile carries an authz requirement | keep |
| 35 | "RBAC is a Stage-2 concern" cited to **`CLAUDE.md`** | `access/mod.rs:19-20` | `CLAUDE.md` | **banned**: the citation hard rule forbids citing an internal markdown file | re-ground only |
| 36 | "Stage-1" / "Stage-2" phase vocabulary throughout the module docs | `access/mod.rs:14,19,24,66`, `authn/mod.rs:80`, `authn/basic.rs:3`, `authn/jwt.rs:128` | — | phase/plan markers, banned by `.claude/rules/comments.md` §Annotation vocabulary | re-ground only |
| 37 | RBAC classes: `Public`→allow, `Clinical`→≥1 role, `Admin`→`admin_role`, `Management`→tri-state | `authz/roles.rs:100-131` | — | no spec — our own design; the vocabulary (users, roles, permissions) is ANSI/INCITS 359 **Core RBAC** only: no role hierarchies, no SSD/DSD, no sessions | keep (re-cite) |
| 38 | `readonly_role` restriction overrides every grant | `authz/roles.rs:141-160` | already flagged own design + SM §GA | correct; note it is a *restriction*, which Core RBAC does not model — our own extension | keep |
| 39 | `extract_roles`: upper-case, dedupe first-seen, arrays element-wise, plain strings whitespace-split | `authz/roles.rs:33-62` | — | own design; the whitespace-split arm exists only to serve `scope` and becomes vestigial once row 23 lands | re-ground only |
| 40 | unmapped matched route → `Admin` if the template contains `/admin/`, else `Clinical`; no `MatchedPath` (a 404) → `Public` | `authz/mod.rs:317-334` | — | own design, fail-safe | keep |
| 41 | `EXTENSION_READ_ROUTES` — `POST /message/export` is a read for the read-only gate | `authz/mod.rs:253-280` | SM `i_ehr_extract_service.adoc` + own design | correct as stated | keep |
| 42 | route-map construction silently `continue`s an unparsable method / unclassified op | `authz/mod.rs:298-303` | "impossible for the generated tables (the coverage guard proves classification)" | the guard makes it unreachable, but the silent skip would *downgrade* the route to `Clinical` — an impossible state must be loud, not skipped | re-ground only |
| 43 | `Clinical` is the default class ("v1's rule that only `/rest/admin/**` … require ADMIN") | `authz/classify.rs:11-14` | v1's rule | own design; the OPTIONS/System reasoning at `classify.rs:52-56` is already correctly ITS-REST-cited | re-ground only |
| 44 | demographic + definition families are coarse-RBAC only ("v1 ABAC never covered these") | `authz/classify.rs:113,128` | v1 | own design (the ABAC attribute model has no demographic attributes) | re-ground only |

### 2.4 Authorization — ABAC, PDP engines, PEP

| # | Decision | Where | Claimed authority | Real authority | Verdict |
|---|---|---|---|---|---|
| 45 | multi-valued attributes fan out as the full cartesian product, **all must permit**, short-circuit on the first deny | `authz/request.rs:9-10,119-151`, `authz/engine.rs:5-8` | "EHRbase v1 parity" | no spec — our own design, and sound on its own terms (every resource a request touches must be permitted) | re-ground only |
| 46 | `Attr::Set(vec![])` → zero combinations → "a vacuous permit — EHRbase v1's empty-result behaviour" | `authz/request.rs:84-86,215-219` | EHRbase v1 | **the rationale is false**: the branch is unreachable from production code — both builders map an empty list to `None` (`pep.rs:171`, `pep.rs:416`), and `None` yields exactly ONE combination with the attribute absent (`request.rs:145-151`), which IS evaluated. The branch is the defensive identity of the all-must-permit fold, nothing more | re-ground only |
| 47 | any `AuthzError` → 500, never a permit | `authz/engine.rs:6-8,18-31`, `pep.rs:553-561` | "v1 parity" | no spec — our own fail-closed design (RFC 9110 §15.6.1 for the status) | re-ground only |
| 48 | Cedar is deny-by-default with `forbid` overriding `permit` | `authz/cedar.rs:13-14` | "the shipped example policies document that" | the **Cedar authorization docs** (`cedar-policy/cedar-docs` `_auth/authorization.md`): default deny; forbid overrides permit; **skip on error** | keep (re-cite) |
| 49 | `Authorizer::is_authorized(...)` — the `Decision` is read, `response.diagnostics()` is **discarded** | `authz/cedar.rs:173-175` | — | **none, and it is dangerous**: Cedar's documented third property is "skip on error — if a policy's evaluation returns `error`, the policy does not factor into the authorization response; it is skipped", *including `forbid` policies*. Discarding the diagnostics means a `forbid` that errors silently stops denying | change behaviour |
| 50 | the Cedar principal is the constant `User::"caller"`; `roles`/`scopes` are declared in the schema but always **empty** | `authz/cedar.rs:210-238,227-235` | "empty under the v1 wire attribute model" | **none** — this makes RBAC- and scope-aware Cedar policies unwritable, i.e. the shipped ABAC engine cannot express the model NIST SP 800-162 calls ABAC (subject attributes are a mandatory attribute source) | change behaviour |
| 51 | `Context::empty()` — the operation id, access mode, and tenant are not visible to policy | `authz/cedar.rs:165-171` | — | **none**; NIST SP 800-162 §2.2 makes environment/action attributes a first-class attribute source | change behaviour |
| 52 | boot: schema built from the `ResourceKind × AccessMode` enums, operator policies strict-validated, empty policy dir refused | `authz/cedar.rs:82-106,263-306` | — | own design; `cedar-policy` 4.12.0 `Validator`/`ValidationMode::Strict` | keep |
| 53 | periodic reload swaps on validity; an invalid reload is logged and **ignored** (the previous set keeps serving) | `authz/cedar.rs:309-324` | — | own design (fail-safe: a broken edit must not open the gate) | keep (re-cite) |
| 54 | remote PDP wire contract: `POST {server}{policy-name}`, flat JSON body, **HTTP 200 = permit, any other status = deny**, body ignored | `authz/remote.rs:1-10,84-100` | "Byte-compatible with EHRbase v1" | must become **our own operational contract**; and "any other status = deny" silently converts a PDP `5xx`/`3xx` into a blanket 403 — the same dependency-failure-as-client-answer defect as row 7 | change behaviour |
| 55 | a resource kind with **no configured policy** → `Decision::Permit`; `AuthzConfig::validate` does **not** require a policy per kind when `engine = remote` (verified: `config/authz.rs:316-352` checks only kind-name validity, the illegal `template` parameter, and the server URL) | `authz/remote.rs:106-109`, `config/authz.rs:193-197` | "EHRbase v1 parity for `directory`" | **none** — a silent permit on missing configuration. A runtime deny would break live traffic; a **boot error cannot**, and `config/authz.rs` already fails at boot for every other ABAC misconfiguration | change behaviour |
| 56 | `AbacParam::wire_key` — `organization`/`patient`/`template` | `config/authz.rs:103-112` | "the exact wire key EHRbase v1 uses" | our own operational contract with the external PDP | re-ground only |
| 57 | remote PDP connect/request timeouts (2 s / 5 s) | `config/authz.rs:146-152` | "EHRbase v1 had none, so a blackholed PDP parked the request" | own design; the narration is change-history that belongs on the PR | re-ground only |
| 58 | a `template` parameter on `ehr`/`ehr_status` is a **boot** error | `config/authz.rs:259-262` | "EHRbase v1 made this a runtime 500; here it is a boot error" | the rule is right and grounded in our own model (an EHR/EHR_STATUS carries no template); the parenthetical is banned change-narration | re-ground only |
| 59 | `directory_checked = abac.policy.contains_key("directory")` — the Cedar engine's directory gating is keyed off a **remote-PDP-shaped** config map | `authz/mod.rs:212-214,234`, `pep.rs:209-212` | "v1 parity" | **none** — with `engine = cedar` an operator can only enable DIRECTORY checks by adding a dummy remote-PDP policy entry; the switch belongs to the ABAC section, not the PDP wire map | change behaviour |
| 60 | composition order `EHR_ACCESS` → RBAC → ABAC → SMART, AND-composed, SMART narrows only | `pep.rs:22-27`, `api/mod.rs:198-221`, `router.rs:110-129` | RM `ehr_access` + master07/master08 + `.claude/rules/auth.md` layer rule | correct | keep |
| 61 | `mode_of` Pre/Post/Skip enforcement matrix keyed by operation id | `pep.rs:95-120` | own design | own design (pre-checks have path/query/body; post-checks have the recorded `AuditObject`) | keep |
| 62 | "a configured-but-missing patient claim is a 403 (v1 threw 500 — NOTE: the improvement)" | `pep.rs:319-321` | v1 comparison | the 403 is right (the caller is authenticated but unattributable to a patient); the parenthetical is banned change-narration | re-ground only |
| 63 | `ehr_get_by_subject`: the claim is compared to the `subject_id` query parameter, and an **absent** parameter passes | `pep.rs:354-365` | "(v1)" | **none** — fail-open on a malformed request, the exact shape `resolve_target` (`pep.rs:527-540`) was hardened against | change behaviour |
| 64 | a **subject-less** EHR passes the patient gate | `pep.rs:371-389`, test comment `pep.rs:806-817` | "EHRbase v1 parity" | no authority either way; it is a genuine policy choice, and `SEC-ANONYMOUS_EHRS-anonymous_lifecycle` requires anonymous EHRs to stay operable (for an *authorized* principal) | re-ground only + §8 Q1 |
| 65 | a `UID_BASED_ID` the strict decoder rejects is **denied**, never degraded | `pep.rs:509-540` | BASE `master05-identification_package.adoc` §Syntaxes + own fail-closed rule | correct and well grounded | keep |
| 66 | the CONTRIBUTION template set is read with a **JSON pointer** `/archetype_details/template_id/value`, while the composition template beside it uses the typed decode "never a JSON pointer that a shape change could silently stop matching" | `pep.rs:463-494` vs `pep.rs:446-459` | — | self-contradictory: the pointer path is exactly what the sibling comment condemns, and a shape change silently yields "no template" → for CONTRIBUTION that path *denies*, so the failure mode is a false 403 | change behaviour |
| 67 | `fallback_principal()` — "a principal placeholder for the (unreachable) no-principal path", subject `"UNKNOWN"`, method `Bearer`, no scopes | `pep.rs:563-572`, used at `pep.rs:139,168,707` | "unreachable" | **the claim is false**: with `auth.enabled = false` the middleware scopes `REQUEST_PRINCIPAL` to `None` (`authn/mod.rs:367-369`) while `abac.enabled` can still be true, so the fabricated principal *is* reached — and `pre_check` handles the same case by refusing (`pep.rs:214-215`), so the two paths disagree. Fabricating an identity is also what `jwt.rs:135-150` refuses to do for audit | change behaviour |
| 68 | an empty query outcome (nothing touched) permits | `pep.rs:164-166` | "v1 parity" | own design, and correct — no resource was touched | re-ground only |
| 69 | SMART patient-compartment permits bind the launch context through the ABAC subject gate; target-less operations pass vacuously | `pep.rs:600-628,636-670,689-719` | master07 §Context Selection + master08 §Resource Scopes | correct | keep |
| 70 | module-scope `#![expect(clippy::disallowed_types, reason = "RFC 7519 leaves the claim set open")]` | `pep.rs:56-60`, `authn/jwt.rs:18-22`, `authn/mod.rs:32-36`, `authn/basic.rs:10-14`, `authz/roles.rs:8-12` | RFC 7519 | the reason is right for the claim-map modules; `basic.rs` handles **no claims at all**, so its copy is unfounded, and module scope is broader than the rule's "smallest item" | re-ground only |
| 71 | unknown tenant → unscoped (default tenant, an engine-level empty set); resolution error → 503 | `access/tenant.rs:16-18,44-58` | "no openEHR spec governs multi-tenancy — our own extension" | correct as stated | keep |
| 72 | `ferroehr.access_control.v1` EHR_ACCESS scheme; AQL privacy filtering not evaluated | `access/ehr_access.rs:34-40` | RM `ehr_access.adoc` + own scheme | correct; the "# v1 scope boundary" heading is a phase marker (comments rule) | re-ground only |

**Verdict tally: 22 `keep`, 23 `re-ground only`, 27 `change behaviour`.**

---

## 3. RFC conformance matrix

| Clause | What it requires | What we do today | Verdict | Action |
|---|---|---|---|---|
| RFC 6749 §1.1 | the resource server hosts protected resources and accepts access tokens | that is exactly our role; token issuance is refused as out of scope (`authn/jwt.rs:9-16`) | conforms | keep the doc, re-cite to §1.1 |
| RFC 6749 §7 | validate the access token; the validation method is out of scope, "defined by companion specifications such as [RFC6750]" | we implement the RFC 6750 bearer method only | conforms | record that the query-string (§2.3) and form-body (§2.2) methods are deliberately unimplemented — own-design refusal |
| RFC 6750 §2.1 | `credentials = "Bearer" 1*SP b64token`, with the `b64token` character set | header split by whitespace-trimming, no charset check (`authn/mod.rs:323-327`) | diverges | G3 — strict parse |
| RFC 6750 §2.2/§2.3 | form-encoded body and URI-query methods (query SHOULD NOT be used; if supported, `Cache-Control: no-store` is MUST) | neither is accepted | conforms (by refusal) | document the refusal |
| RFC 6750 §3 | the challenge may carry `realm`, `scope`, `error`, `error_description`, `error_uri`, each at most once | static `Basic realm="ferroehr", Bearer`; no `error` ever (`authn/mod.rs:238-251`) | diverges | G1 |
| RFC 6750 §3.1 `invalid_request` | 400 | never emitted; a malformed `Authorization` header yields 401 `InvalidCredentials` (`authn/mod.rs:266-268,338`) | diverges | G2 — a malformed *header* (not a bad credential) becomes 400 `invalid_request`; a wrong credential stays 401 |
| RFC 6750 §3.1 `invalid_token` | 401 | 401 ✓, but with no `error` parameter | partly conforms | G1 |
| RFC 6750 §3.1 `insufficient_scope` | **403** with the challenge | 403 ✓ with **no** challenge, and `authn/mod.rs:461-463` asserts one MUST NOT be sent | diverges | G1 (SMART scope + RBAC/ABAC denies gain `WWW-Authenticate: Bearer error="insufficient_scope"`) |
| RFC 6750 §3.1 (no-credentials rule) | "If the request lacks any authentication information … SHOULD NOT include an error code" | we include none — correct by accident | conforms | pin it with a test so it stays deliberate |
| RFC 6750 §5.2 | protect tokens in transit; restrict the audience | TLS via `[server.tls]`; `SetSensitiveRequestHeadersLayer` on `authorization` (`router.rs:186`); audience optional | partly conforms | G6 (audience mandatory) |
| RFC 7519 §4.1.1 `iss` | optional, but an application that validates it must reject a mismatch | `set_issuer` ✓; a token with **no** `iss` is not rejected (row 18) | diverges | G5 |
| RFC 7519 §4.1.2 `sub` | optional | required + non-blank | stricter than required, deliberately | keep, re-cite to RFC 9068 §2.2 |
| RFC 7519 §4.1.3 `aud` | "if the principal … does not identify itself with a value in the `aud` claim when this claim is present, then the JWT MUST be rejected" | with `audiences = []` we set `validate_aud = false`, so a token minted for another resource server is **accepted** | **diverges** | G6 |
| RFC 7519 §4.1.4 `exp` | not accepted at/after `exp`; small leeway allowed | `validate_exp` default true, leeway 60 s | conforms | make leeway configurable with a cap (G7) |
| RFC 7519 §4.1.5 `nbf` | MUST NOT be processed before `nbf` | `validate_nbf` default **false** | diverges | G4 |
| RFC 7519 §4.1.6 `iat` / §4.1.7 `jti` | optional in JWT; REQUIRED for an access token by RFC 9068 §2.2 | neither checked | diverges (RFC 9068 path only) | G8 |
| RFC 7519 §5.1 `typ` | optional; "ignored by JWT implementations; any processing … is performed by the JWT application" | not processed | diverges vs RFC 9068 | G8 |
| RFC 7519 §6 / RFC 9068 §4 step 5 | `alg: none` must be rejected | `jsonwebtoken` has no `none` variant, so `"none".parse::<Algorithm>()` fails at boot | conforms | assert it as a test (G9) |
| RFC 7519 §7.2 (10 steps) | the structural decode/validate sequence | performed inside `jsonwebtoken` `decode`/`decode_header` | conforms by delegation | cite the delegation; add a `crit`-header note (RFC 7515 §4.1.11) |
| RFC 7515 §4.1.1 | `alg` MUST be present and understood | `decode_header` yields it; membership checked | conforms | keep |
| RFC 7515 §5.2 | the recipient MUST run the full validation and reject on any failure; unacceptable algorithms SHOULD make the JWS invalid | delegated to `jsonwebtoken` + our algorithm allow-list | conforms | keep, cite the delegation |
| RFC 7515 §10.4/§10.7 | algorithm-substitution defence: the algorithm must correspond to the key, not merely to the header | the configured algorithm set is not bound to the key source, and the selected JWK's own `alg` is never compared | diverges | G10 |
| RFC 7517 §4.3/§4.4/§4.5 | `key_ops`, `alg`, `use` constrain how a key may be used | not consulted; `kid`-less tokens take `keys.first()` | diverges | G10 |
| RFC 8725 §3.1 | caller-specified algorithm set; each key used with exactly one algorithm, checked at operation time | set ✓; per-key binding ✗ | partly conforms | G10 |
| RFC 8725 §3.2 | only cryptographically current algorithms; `none` only when otherwise protected | RS256 default, `none` unrepresentable | conforms | keep |
| RFC 8725 §3.3 | all cryptographic operations validated, whole JWT rejected on any failure | delegated to `jsonwebtoken`; nested JWTs (`cty: JWT`) are not accepted | conforms | record the nested-JWT refusal |
| RFC 8725 §3.4 | validate cryptographic inputs | delegated to `jsonwebtoken` + `aws-lc-rs` | conforms | keep |
| RFC 8725 §3.5 | human-memorizable passwords MUST NOT be HS256 keys | `hmac_secret` is a config string with no floor | diverges | G11 |
| RFC 8725 §3.6 | do not compress before encryption | we never encrypt (JWE unsupported) | not applicable | state it |
| RFC 8725 §3.7 | UTF-8 only | `serde_json` is UTF-8-only | conforms | keep |
| RFC 8725 §3.8 | keys used MUST belong to the issuer; validate `sub` | keys come only from the configured source (HMAC / static JWKS / the issuer's own discovered `jwks_uri` with the issuer-match check); `sub` non-blank | conforms | G10 tightens key selection *within* the set |
| RFC 8725 §3.9 | if applicable, the audience MUST be validated; absent-or-unassociated → reject | optional today | **diverges** | G6 |
| RFC 8725 §3.10 | do not trust `kid`; protect against SSRF from `jku`/`x5u` | `kid` is only a lookup key in a fixed set; `jku`/`x5u` are never followed; discovery redirects disabled | conforms | pin with a test that a `jku`-bearing token gains nothing |
| RFC 8725 §3.11 | use explicit typing | `typ` unprocessed | diverges | G8 |
| RFC 8725 §3.12 | validation rules for different JWT kinds MUST be mutually exclusive, rejecting the wrong kind | nothing distinguishes an **ID token** from an access token today (with `audiences = []` an ID token whose `aud` is a client id validates and authenticates) | **diverges** | G6 + G8 (a mandatory RS audience is the discriminator: OIDC ID-token `aud` is the client id) |
| RFC 8414 §2 | the issuer identifier is an `https` URL with no query or fragment; `jwks_uri` locates the keys | any URL accepted, no boot check | diverges | G12 |
| RFC 8414 §3 | metadata at the well-known URI inserted between host and path | we use the OIDC Discovery §4 *append* form via `openidconnect` (`.well-known/openid-configuration`) | not applicable / deliberate | record: we do OIDC Discovery, not RFC 8414's `oauth-authorization-server` path |
| RFC 8414 §3.3 / OIDC Discovery §4.3 | the returned `issuer` MUST be identical to the requested one, else the response MUST NOT be used | enforced by `openidconnect` 4.0.1 (`src/discovery/mod.rs:383-390`, `DiscoveryError::Validation`) | conforms by delegation | document + pin with a wiremock test |
| RFC 8414 §6.1/§6.2 | TLS MUST be supported, with certificate checking | `reqwest` + rustls; `http://` issuers are not refused | partly conforms | G12 |
| OIDC Discovery §4.2 | REQUIRED members: `issuer`, `authorization_endpoint`, `token_endpoint`, `jwks_uri`, `response_types_supported`, `subject_types_supported`, `id_token_signing_alg_values_supported` | enforced by `CoreProviderMetadata` deserialization | conforms by delegation | keep the test fixture that documents the minimum |
| RFC 9068 §2.1 | `typ` MUST be `at+jwt` | unprocessed | diverges | G8 |
| RFC 9068 §2.2 | REQUIRED: `iss`, `exp`, `aud`, `sub`, `client_id`, `iat`, `jti` | only `exp` + `sub` enforced | diverges | G5, G6, G8 |
| RFC 9068 §2.2.3 / §2.2.3.1 | `scope` carries delegation; `groups`/`roles`/`entitlements` (RFC 7643 §4.1.2) carry non-delegated authorization | roles are mined *from `scope`* | **diverges** | G13 |
| RFC 9068 §4 steps 1–6 | ordered: `typ` → decrypt → `iss` exact → `aud` contains us → signature per RFC 7515, reject `alg: none` → `exp` in the future (small leeway) | steps 3, 5, 6 done; 1 missing; 4 optional; 2 n/a (no JWE) | diverges | G4–G10 rebuild the pipeline in this order |
| RFC 9068 §4 (error) | "In case of any failure in the validation checks listed above … MUST include the error code `invalid_token`" | no `error` code is ever emitted | diverges | G1 |
| RFC 9068 §5 | explicit typing distinguishes access tokens from ID tokens; distinct `aud` per resource | neither enforced | diverges | G6, G8 |
| RFC 7617 §2 | scheme `Basic`, `realm` REQUIRED; `user-pass = user-id ":" password`; Base64 per RFC 4648 §4; a colon-bearing user-id is invalid | realm ✓, `split_once(':')` ✓, RFC 4648 alphabet ✓ | conforms | keep |
| RFC 7617 §2.1 | the `charset` parameter's only allowed value is `UTF-8`, matched case-insensitively | we decode as UTF-8 but never advertise `charset` | partly conforms | G1 (add `charset="UTF-8"`) |
| RFC 7617 §4 | not secure without TLS; stored passwords must not be trivially recoverable | Argon2id PHC only ✓; TLS is a deployment property; no cost floor | partly conforms | G14 |
| RFC 9110 §11.1 | scheme names are case-insensitive | ✓ | conforms | keep |
| RFC 9110 §11.6.1 / §15.5.2 | a 401 MUST carry at least one **applicable** challenge | a challenge is always sent on 401; it can advertise a scheme no mechanism implements | partly conforms | G15 |
| RFC 9110 §15.5.4 | 403 = understood but refused; a server wanting to say credentials would help should send 401 instead | our 403s are all post-authentication refusals | conforms | keep; adding the RFC 6750 challenge to a 403 does not contradict this |
| RFC 9110 §15.6.4 | 503 for a transient inability to handle the request | issuer/PDP outages surface as 401/403 | diverges | G16 |
| ITS-REST §A&a | if authn/authz are required, services MUST properly use `WWW-Authenticate`/`Proxy-Authenticate`, returning 403/401/407 as applicable | 401 + challenge, 403 without | conforms; the "MUST NOT on 403" gloss is ours, not the spec's | G1 removes the gloss |
| SM §GA | authorization is an out-of-band precondition | RBAC/ABAC declared as our own extension | conforms | keep the flag everywhere |
| NIST SP 800-162 (§2.2 attribute sources; §3 PEP/PDP/PIP/PAP) | conceptual reference only | the components exist but are not named as such, and subject attributes never reach the PDP | partly aligned | G17 (naming) + G18/G19 (attributes) |
| ANSI/INCITS 359 Core RBAC | users, roles, permissions; hierarchies and SSD/DSD are separate components | Core RBAC only, plus a non-RBAC restriction | aligned, deliberately partial | G17 records the boundary |

---

## 4. The target design

### 4.1 Layering (unchanged order, renamed in NIST vocabulary)

```text
tower-http ingress (request-id, sensitive-headers, catch-panic, CORS, TLS)
  └─ overload shed                                  router.rs:153
     └─ ATNA audit layer                            router.rs:133
        └─ AUTHENTICATION  (Basic | Bearer)         authn::middleware   ← §4.2
           └─ RBAC gate  (the coarse PEP)           authn::middleware   ← §4.4
              └─ tenant scope (when enabled)        tenant::middleware
                 └─ guarded_dispatch                api/mod.rs:207
                    ├─ EHR_ACCESS gate (spec-grounded, always first)
                    ├─ ABAC PEP pre-check  → PDP (Cedar | remote)  ← §4.5
                    ├─ the operation
                    ├─ ABAC PEP post-check → PDP
                    └─ SMART narrowing gate (AND-composed, narrows only)
```

NIST SP 800-162 component naming, adopted verbatim in the module docs so the
model has a name instead of a description:

| NIST component | Realization |
|---|---|
| **PEP** | `authn::middleware` (the coarse role gate) + `pep.rs` (the fine-grained gate) — the only two places a 401/403 is minted |
| **PDP** | `authz::engine::PolicyEngine`, behind it `authz::cedar::CedarEngine` (default) and `authz::remote::RemotePdp` |
| **PIP** | the validated claim set on `Principal` (subject attributes) + `authz::AuthzResolvers` (resource attributes from the database) |
| **PAP** | the Cedar policy directory (`abac.cedar.policy_dir`) / the external PDP's own policy store |
| **attribute sources** | subject (claims, roles, scopes), resource (EHR subject, template id, resource kind), action (access mode, operation id), environment (tenant) |

### 4.2 Authentication paths

| Path | Trigger | Key material | Clause set |
|---|---|---|---|
| Basic | `Authorization: Basic <base64>` and `[auth.basic]` present | Argon2id PHC store | RFC 7617 §2, §2.1, §4; OWASP §Argon2id |
| Bearer / discovery | `Authorization: Bearer <jwt>`, `[auth.oidc]` with neither `hmac_secret` nor `jwks_json` | JWKS from the issuer's OIDC discovery document, cached | RFC 6750, 7515, 7517, 7519, 8414 §3.3, OIDC Discovery §4/§4.3, 8725, 9068 |
| Bearer / static JWKS | `jwks_json[_file]` set | the configured JWK set | same, minus discovery |
| Bearer / symmetric | `hmac_secret[_file]` set | one HS* key | same, plus RFC 8725 §3.5 (entropy floor, dev-only warning) |

Key-source precedence stays `hmac_secret` → `jwks_json` → discovery
(`authn/jwt.rs:85-93`), and `config/mod.rs:151-174` already refuses two
explicit sources at once.

### 4.3 The bearer-token validation pipeline (ordered; each step names its clause)

Implemented as one explicit sequence so the order is auditable, matching
RFC 9068 §4's own ordering. Every failure yields `invalid_token` (RFC 9068 §4)
except where noted.

| # | Check | Clause | Failure |
|---|---|---|---|
| 1 | the header value parses as `"Bearer" 1*SP b64token` with the §2.1 character set | RFC 6750 §2.1 | 400 `invalid_request` (RFC 6750 §3.1) |
| 2 | `decode_header` succeeds; `alg` present and understood; no unsupported `crit` entry | RFC 7515 §4.1.1, §4.1.11 | 401 `invalid_token` |
| 3 | `typ`: accepted values are `at+jwt` / `application/at+jwt` (RFC 9068 §2.1), or — when `oidc.require_at_jwt = false` — absent / `JWT` / `Bearer`; any other value is refused | RFC 9068 §2.1 + §4 step 1; RFC 8725 §3.11 | 401 `invalid_token` |
| 4 | `alg` ∈ the configured set, **and** the configured set is legal for the key source (HS* only with `hmac_secret`; asymmetric only with a JWK source) | RFC 8725 §3.1, §2.1; RFC 7515 §10.4 | boot error (config) / 401 |
| 5 | key selection: `kid` → the matching JWK; no `kid` → the sole JWK, else refuse ambiguity. The chosen JWK must satisfy `use = "sig"` (or absent), `key_ops ∋ "verify"` (or absent), and `alg` equal to the header `alg` (or absent) | RFC 7517 §4.3–§4.5; RFC 8725 §3.1, §3.8 | 401 `invalid_token` |
| 6 | signature verification over the JWS signing input | RFC 7515 §5.2; RFC 9068 §4 step 5 | 401 `invalid_token` |
| 7 | `alg` is never `none` | RFC 7519 §6; RFC 9068 §4 step 5 | unrepresentable in `jsonwebtoken` 11; asserted by test |
| 8 | required claims present: `exp`, `iss`, `aud`, `sub` via `Validation::set_required_spec_claims`, plus `iat`, `jti`, `client_id` checked on the claim map when the `at+jwt` profile applies | RFC 9068 §2.2 | 401 `invalid_token` |
| 9 | `iss` exactly equals the configured issuer | RFC 9068 §4 step 3; RFC 8725 §3.8 | 401 `invalid_token` |
| 10 | `aud` contains one of the configured audiences — **always**, never disabled | RFC 7519 §4.1.3; RFC 9068 §4 step 4; RFC 8725 §3.9 | 401 `invalid_token` |
| 11 | `exp` in the future, within the configured leeway (default 60 s, capped) | RFC 7519 §4.1.4; RFC 9068 §4 step 6 | 401 `invalid_token` |
| 12 | `nbf`, when present, is in the past (same leeway) | RFC 7519 §4.1.5 | 401 `invalid_token` |
| 13 | `sub` is a non-blank string (audit attributability) | RFC 9068 §2.2 + our own audit rule | 401 `invalid_token` |
| 14 | scopes = `scope` (whitespace-split) ∪ `scp` (array; a vendor claim, our own accommodation) | RFC 9068 §2.2.3 | — |
| 15 | roles = the configured role claims only, never `scope` | RFC 9068 §2.2.3.1; RFC 7643 §4.1.2 | — |

Steps 1–13 live in `authn::jwt`; steps 14–15 build the `Principal`.
Steps 3–5 and 8–12 are new or newly enforced. Nested JWTs (`cty: JWT`,
RFC 7519 §5.2) and JWE are refused outright — recorded as an own-design
boundary.

**Challenge + status surface.**

| Outcome | Status | `WWW-Authenticate` | Clause |
|---|---|---|---|
| no `Authorization` header | 401 | every enabled scheme, **no** `error` | RFC 9110 §15.5.2; RFC 6750 §3.1 no-credentials rule |
| malformed `Authorization` header | 400 | `Bearer error="invalid_request"` | RFC 6750 §3.1 |
| bad Basic credential | 401 | `Basic realm="ferroehr", charset="UTF-8"` | RFC 7617 §2, §2.1 |
| bad bearer token (any pipeline step) | 401 | `Bearer realm="ferroehr", error="invalid_token"` — no `error_description` (the opaque-outcome discipline) | RFC 6750 §3.1; RFC 9068 §4 |
| issuer / JWKS unreachable | 503 + `Retry-After` | none | RFC 9110 §15.6.4 |
| credential verification unavailable (KDF task failure) | 500 | none | already correct, `authn/mod.rs:154` |
| RBAC / ABAC / SMART deny on a bearer principal | 403 | `Bearer error="insufficient_scope"` | RFC 6750 §3.1 |
| RBAC / ABAC deny on a Basic principal | 403 | none (RFC 6750 does not apply to Basic) | RFC 9110 §15.5.4 |

### 4.4 RBAC (our own design, Core-RBAC vocabulary)

Unchanged in shape, re-grounded and cleaned:

- **Roles** are upper-cased strings drawn from configured JWT claim paths
  (Bearer) or the user's configured list (Basic). New default claim paths:
  `["roles", "groups", "entitlements", "realm_access.roles"]` — the first three
  from RFC 9068 §2.2.3.1 / RFC 7643 §4.1.2, the fourth a documented Keycloak
  accommodation. **`scope` is removed** (§2 row 23).
- **Permissions** are the four operation classes (`Public`, `Clinical`,
  `Management`, `Admin`) assigned per generated operation id
  (`authz/classify.rs`, total-coverage-guarded) and per route template for the
  non-generated surface.
- **The gate**: `Public` → allow; `Clinical` → at least one role;
  `Admin` → `admin_role`; `Management` → the `management_access` tri-state.
- **The restriction**: a principal carrying `readonly_role` is refused on every
  write, overriding any grant. Core RBAC has no restriction concept — flagged
  as our own extension.
- Explicitly **absent**: role hierarchies, static/dynamic separation of duty,
  sessions. Recorded as a boundary, not an omission.

### 4.5 ABAC (our own design, SP 800-162 vocabulary)

**Evaluation order, stated explicitly (the issue's acceptance criterion 3):**

1. The **spec-grounded** `EHR_ACCESS` gate runs first and unconditionally
   (`api/mod.rs:213`) — RM `org.openehr.rm.ehr.ehr_access.adoc`.
2. The coarse **RBAC** gate must allow (`authn::middleware`). A deny is final.
3. The **local patient gate** runs before any PDP call (`pep.rs:346-389`): a
   configured patient claim must match the target EHR's subject. A deny is
   final and costs no engine call.
4. Attributes are resolved (PIP): subject from the claim set, resource from
   `AuthzResolvers`, action from the access mode, environment from the tenant.
   **Any resolution failure denies** — never a permit on an absent attribute.
5. The **PDP** is consulted once per attribute combination (the cartesian
   product of the multi-valued `patient` × `template` candidates).
   **All combinations must permit**; the first deny short-circuits. A request
   with a single absent-attribute combination is still evaluated.
6. Within one combination the engine's own algorithm decides:
   - **Cedar** (`cedar-policy` 4.12.0): **deny by default** — no request is
     authorized unless a `permit` policy is satisfied; **`forbid` overrides
     `permit`**; and Cedar **skips a policy whose evaluation errors**. Because
     skip-on-error can silently disarm a `forbid`, we treat any non-empty
     `Response::diagnostics().errors()` as `AuthzError::Evaluation`, i.e. a
     fail-closed 500 (§2 row 49).
   - **Remote PDP** (our own operational contract, no RFC): `POST
     {server}{policy-name}` with the flat resolved-attribute body;
     **2xx = permit, 4xx = deny, anything else (3xx/5xx/transport) =
     `AuthzError::Unreachable` → 500**. Every resource kind the PEP consults
     must have a configured policy — enforced at **boot**, so the runtime
     "no policy" branch becomes a loud `Deny`, not a `Permit`.
7. The **SMART** narrowing gate runs last, AND-composed after a PDP permit,
   and can only narrow (master08 §Resource Scopes; master07 §Context
   Selection).
8. Any engine or attribute failure anywhere → **fail closed**: 403 for a
   policy deny, 500 for an engine fault, 503 for a dependency outage. Never a
   permit.

**Cedar entity model (rewritten).** `User::"<sub>"` with
`{ organization?, patient?, roles: Set<String>, scopes: Set<String> }` actually
populated from the `Principal`; the resource entity per `ResourceKind` with
`{ patient?, template? }`; the action `"<kind>.<mode>"`; and a **request
context** carrying `{ operation_id, tenant? }`. The schema stays generated from
the `ResourceKind` × `AccessMode` enums so it cannot drift, and the shipped
example policies are updated to demonstrate role- and scope-aware rules.

---

## 5. Gap list → concrete changes

Wire-visible items need CNF SEC evidence **and** a `CHANGELOG.md` entry;
config-schema items additionally need the default TOML template, the website
book page, and the Helm values + golden renders in the same PR
(`.claude/rules/configuration.md`). Every test named below asserts the clause
in its name.

| # | Defect | Fix | Files | Test (asserted clause) | Visibility |
|---|---|---|---|---|---|
| G1 | the challenge carries no RFC 6750 `error` parameter, no `charset`, and is asserted never to appear on a 403 | build the challenge per outcome (§4.3 table): `error="invalid_token"` on a bearer 401, no error code when no credentials were presented, `charset="UTF-8"` on Basic, `error="insufficient_scope"` on a bearer 403 | `authn/mod.rs` (`challenge`, `middleware`), `pep.rs` (`forbidden`), `overview/error.rs` | `challenge_carries_invalid_token_error` (RFC 6750 §3.1); `no_credentials_carries_no_error_code` (RFC 6750 §3.1); `basic_challenge_advertises_utf8_charset` (RFC 7617 §2.1); `scope_deny_carries_insufficient_scope` (RFC 6750 §3.1) | **wire** |
| G2 | a malformed `Authorization` header is answered 401 like a wrong credential | split the outcomes: unparsable header / unknown scheme → 400 `invalid_request`; credential rejected → 401 | `authn/mod.rs:262-274,338`, `authn/basic.rs:47-56` | `malformed_authorization_header_is_400_invalid_request` (RFC 6750 §3.1) | **wire** |
| G3 | bearer extraction does not enforce the RFC 6750 §2.1 grammar | parse `"Bearer" 1*SP b64token`, validating the `b64token` character set; reject empty or malformed | `authn/mod.rs:321-337` | `bearer_credentials_follow_rfc6750_abnf` (RFC 6750 §2.1) | **wire** (400 instead of 401 for garbage) |
| G4 | `nbf` never validated | `validation.validate_nbf = true` | `authn/jwt.rs:117-124` | `not_yet_valid_token_rejected` (RFC 7519 §4.1.5) | internal (401 either way for valid tokens; a not-yet-valid token now 401s) |
| G5 | a token with no `iss` is accepted | `set_required_spec_claims(["exp","iss","aud","sub"])` | `authn/jwt.rs:117-124` | `token_without_iss_rejected` (RFC 9068 §2.2) | internal |
| G6 | `audiences = []` disables audience validation entirely — an ID token or another resource server's token authenticates | make `audiences` **non-empty mandatory** at boot whenever `[auth.oidc]` is present; delete the `validate_aud = false` branch | `config/auth.rs:106-107`, `config/mod.rs:151-174`, `authn/jwt.rs:120-124` | `oidc_without_audiences_is_a_boot_error`; `token_for_another_audience_rejected` (RFC 7519 §4.1.3, RFC 9068 §4 step 4); `id_token_shaped_token_rejected` (RFC 8725 §3.12) | **config-schema break** (a deployment with `audiences = []` fails to boot) |
| G7 | leeway is fixed at the crate default | expose `oidc.clock_skew_leeway_seconds` (default 60, boot error above 300) | `config/auth.rs`, `authn/jwt.rs` | `leeway_above_the_cap_is_a_boot_error` (RFC 9068 §4 step 6, "no more than a few minutes") | config-schema (additive) |
| G8 | `typ` unprocessed; `iat`/`jti`/`client_id` unchecked | verify `typ` per §4.3 step 3; add `oidc.require_at_jwt` (own-design flag, default `false`); when the `at+jwt` profile applies, require `iat`, `jti`, `client_id` | `authn/jwt.rs`, `config/auth.rs` | `typ_at_jwt_accepted`; `typ_id_token_rejected`; `require_at_jwt_rejects_untyped` (RFC 9068 §2.1, §2.2, §4 step 1; RFC 8725 §3.11) | config-schema (additive); wire only when opted in |
| G9 | `alg: none` refusal is implicit in the crate | assert it | `authn/jwt.rs` tests, `config/auth.rs` tests | `algorithm_none_is_unconfigurable` (RFC 9068 §4 step 5) | internal |
| G10 | JWK selection ignores `use`/`key_ops`/`alg` and takes `keys.first()` when `kid` is absent | filter the set by `use`/`key_ops`/`alg`; refuse ambiguity when `kid` is absent and more than one candidate remains; bind the configured algorithm set to the key source (HS* only with `hmac_secret`) | `authn/jwt.rs:188-196`, `config/mod.rs` | `kidless_token_with_multiple_keys_rejected`; `signing_key_must_match_header_alg`; `hs256_with_a_jwks_source_is_a_boot_error` (RFC 7517 §4.3–§4.5; RFC 8725 §3.1; RFC 7515 §10.4) | internal (+ boot error for a contradictory config) |
| G11 | `hmac_secret` has no entropy floor | boot error below 32 bytes; boot `warn` that a symmetric secret is a development posture | `config/mod.rs`, `config/auth.rs` | `short_hmac_secret_is_a_boot_error` (RFC 8725 §3.5) | config-schema |
| G12 | `issuer` is unvalidated (blank / `http://` / query / fragment all accepted) | boot-validate: non-empty, `https` scheme, no query, no fragment; add `oidc.allow_insecure_issuer` (default `false`, dev/test only — the wiremock suites set it) | `config/auth.rs`, `config/mod.rs` | `http_issuer_is_a_boot_error`; `issuer_with_query_is_a_boot_error` (RFC 8414 §2, §6.2) | config-schema |
| G13 | roles are mined from the `scope` claim, making the `Clinical` role gate vacuous for any OIDC token | default `role_claims = ["roles","groups","entitlements","realm_access.roles"]`; `scope` removed; keep `scopes` on the `Principal` untouched | `config/authz.rs:56-58,368-370`, `authz/roles.rs:20-24,33-62`, `authn/jwt.rs:164-165` | `scope_claim_does_not_grant_roles` (RFC 6749 §3.3 vs ANSI/INCITS 359); `rfc9068_role_claims_extracted` (RFC 9068 §2.2.3.1, RFC 7643 §4.1.2) | **wire** (a token whose only "roles" were scopes now 403s) + config-schema |
| G14 | a weak stored Argon2 PHC verifies happily; the unknown-user path skips the KDF | boot-validate every configured PHC against the OWASP floor (`m≥19456, t≥2, p≥1`, Argon2id) — error below it; on an unknown username run a verification against a fixed dummy PHC so the timing does not distinguish | `authn/basic.rs:61-71`, `config/mod.rs` | `weak_argon2_params_are_a_boot_error` (OWASP §Argon2id); `unknown_user_pays_the_kdf` (own design) | config-schema + internal |
| G15 | the challenge can advertise a scheme no mechanism implements; `auth.enabled && !has_mechanism()` is only a `warn` | make it a boot **error**; the challenge lists only implemented schemes | `app/ferroehr-server/src/lib.rs:400-405`, `authn/mod.rs:238-251` | `auth_enabled_without_a_mechanism_is_a_boot_error` (RFC 9110 §11.6.1) | config-schema |
| G16 | an issuer/JWKS outage is reported as 401; a remote-PDP 5xx as 403 | `AuthError::KeyResolution` → 503 + `Retry-After` (the existing `ServiceUnavailable` path); remote-PDP non-2xx/non-4xx → `AuthzError::Unreachable` → 500 | `authn/mod.rs:148-159`, `authz/remote.rs:84-100`, `config/auth.rs:132-139` | `unreachable_issuer_is_503_not_401` (RFC 9110 §15.6.4); `pdp_5xx_is_a_server_fault_not_a_deny` (own design) | **wire** |
| G17 | the model has no name; authority claims cite `CLAUDE.md`, "Stage-1/2", and EHRbase | rewrite every module doc in SP 800-162 / Core-RBAC vocabulary; delete the `CLAUDE.md` citation and every phase marker; every remaining rationale carries an RFC/crate/vendored-spec citation or the own-design flag; scope the `disallowed_types` expectations to the smallest item and drop the unfounded one in `basic.rs` | `access/mod.rs`, `authn/**`, `authz/**`, `pep.rs`, `config/auth.rs`, `config/authz.rs`, `docs/architecture.md` §access | `scripts/check-comment-style.sh --all`; a grep gate asserting zero `EHRbase`/`v1 parity`/`Stage-[12]` occurrences under the access layer | internal (+ docs) |
| G18 | the Cedar principal is a constant with empty `roles`/`scopes`; the context is empty | `User::"<sub>"` with populated `roles`/`scopes`/`organization`/`patient`; request context `{ operation_id, tenant? }`; update the shipped example policies | `authz/cedar.rs:155-176,210-238`, `authz/request.rs` (carry the subject attributes), `pep.rs` (pass the `Principal`), the example `*.cedar` files | `role_aware_policy_permits`; `scope_aware_policy_denies`; the existing Cedar↔remote differential test extended | internal (opt-in feature; ABAC is off by default) |
| G19 | Cedar `diagnostics()` discarded → an erroring `forbid` silently stops denying | any non-empty `diagnostics().errors()` → `AuthzError::Evaluation` → 500 | `authz/cedar.rs:173-175` | `erroring_forbid_policy_is_a_fail_closed_500` (Cedar docs §Authorization, "skip on error") | internal (ABAC opt-in) |
| G20 | an unconfigured resource kind permits on the remote PDP | boot-require a `policy` entry for every kind the PEP consults (`ehr`, `ehr_status`, `composition`, `contribution`, `query`) when `engine = remote`; the runtime branch becomes `Deny` + `tracing::error!` | `config/authz.rs:313-352`, `authz/remote.rs:105-109` | `remote_engine_without_a_policy_per_kind_is_a_boot_error`; `unconfigured_kind_denies` (own design, fail-closed) | config-schema + internal |
| G21 | DIRECTORY gating is keyed off the remote-PDP policy map, so Cedar deployments cannot enable it | add `abac.check_directory: bool` (default `false`, preserving today's behaviour); `directory_checked` reads it | `config/authz.rs`, `authz/mod.rs:212-214,234`, `pep.rs:209-212` | `directory_checked_is_engine_independent` (own design) | config-schema (additive) |
| G22 | `ehr_get_by_subject` fails open when `subject_id` is absent | with a configured patient claim, an absent/unparsable `subject_id` denies | `pep.rs:354-365` | `by_subject_without_a_subject_param_denies` (own design, fail-closed) | **wire** (403 before the 400 the route would otherwise give) — see §8 Q3 |
| G23 | the CONTRIBUTION template set is read via a JSON pointer, contradicting the typed decode beside it | decode the contribution payload through the typed seam and read `ARCHETYPED.template_id` | `pep.rs:463-494` | `contribution_templates_read_from_the_typed_value` (own design; mirrors `pep.rs:446-459`) | internal (removes a false-403 path) |
| G24 | `fallback_principal()` is documented unreachable but is reached with `auth.enabled = false` + `abac.enabled = true`, and the three call sites disagree with `pre_check` | delete the fabricated principal; every gate takes `Option<&Principal>` and refuses (403) when absent, exactly as `pep.rs:214-215` does | `pep.rs:139,168,563-572,707` | `abac_without_a_principal_denies_everywhere` (own design, consistency with `pep.rs:214`) | internal (ABAC/SMART opt-in) |
| G25 | `password-hash = "0.6.1"` sits in `[workspace.dependencies]` with no consumer (the lock resolves only `password-hash 0.5.0`, pulled by `argon2 0.5.3`) | drop the unused pin or wire it where the PHC traits are used | root `Cargo.toml` | `cargo deny check` / `cargo machete` stay green | internal |

**Sequencing.** G17 last (it rewrites the prose the other items change).
G6/G12/G15 land together (one config-validation pass, one book/Helm/template
update). G13 and G6 are the two changes an operator must read the changelog
for; they should ship in the same release note. G18–G21, G23, G24 are
behind `abac.enabled` (off by default), so they carry no default-deployment
wire risk.

---

## 6. What must NOT change

| Behaviour | Pinned by |
|---|---|
| unauthenticated request to any protected operation → **401** | `SEC-AUTHENTICATED_ACCESS-unauthenticated_sweep` (ehr create, composition-at-version get, ad-hoc query, OPT upload); universal `unauthenticated → 401` mapping in `selectors.yaml` |
| a 401 carries a `WWW-Authenticate` header (scheme deliberately unasserted) | `SEC-AUTHENTICATED_ACCESS-www_authenticate_challenge` |
| an authenticated read-only principal → **403** on every write | `SEC-AUTHORIZATION_SEPARATION-readonly_write_denied` (create_ehr, create_composition, upload_opt) |
| an anonymous (subject-less) EHR stays fully operable for an authorized principal | `SEC-ANONYMOUS_EHRS-anonymous_lifecycle` — G22/§8 Q1 must not regress it |
| `EHR_STATUS` subject stays opaque | `SEC-EHR_DEMOGRAPHIC_SEPARATION-status_subject_opaque` |
| the server sets the commit audit | `SEC-AUDIT_ACCOUNTABILITY-server_set_commit_audit` |
| every 401/403 carries the `Principal` on the response extensions so the ATNA layer records the denial | `authn/mod.rs:400-404`, `pep.rs:543-548`; `tests/it/audit_e2e.rs`, `tests/it/audit_iti81.rs` |
| a genuine authentication event (Basic KDF miss) emits exactly one ATNA login record; a cache hit and a Bearer request do not | `authn/mod.rs:96-117,289-319`; `verified_credential_cache_skips_the_kdf_on_a_hit` |
| `verified_cache_ttl_seconds = 0` disables the cache | `verified_cache_ttl_zero_disables_caching` |
| a KDF task failure is **500**, never 401 | `verification_unavailable_is_500_not_401` |
| the error body shape stays `{ error, message }` (and `{ message, validationErrors[] }` for 422) | `overview/error.rs:148-155,238-288`; `tests/it/http.rs` |
| the layer order and the AND-composition (SMART narrows only; disabled SMART = zero wire drift) | `router.rs:110-129`, `api/mod.rs:207-225`, `pep.rs` SMART tests, `tests/it/smart_http.rs` |
| the `EHR_ACCESS` gate runs first and unconditionally | `api/mod.rs:213`, `tests/it/ehr_access_gate.rs` |
| RBAC defaults: enabled, `ADMIN`/`USER`/`READONLY`, `management_access = admin_only`, ABAC off | `config/authz.rs:389-402`, `defaults_are_valid` |
| ABAC off by default → zero behaviour change for a default deployment | `build_engine_none_when_abac_disabled`, `handle_absent_when_rbac_disabled` |
| Cedar and remote PDP remain behaviourally interchangeable on the same request corpus | `tests/it/authz_cedar_engine.rs`, `tests/it/authz_remote_pdp.rs` (the differential corpus) |
| all-must-permit fan-out with short-circuit deny; a malformed `UID_BASED_ID` denies | `request.rs` combination tests, `malformed_resource_ids_are_denied` |
| public endpoints (`/rest/status`, the health family, discovery documents) stay outside the auth layer | `router.rs:155` `mount_public_surface`, `tests/it/management.rs` |
| `Authorization` stays a sensitive header (never traced) | `router.rs:186` |

---

## 7. Verification plan

**Gates (all must be green before the PR merges).**

```bash
cargo fmt --all
cargo clippy --workspace --exclude ferroehr-admin-ui --all-targets --all-features -- -D warnings
cargo nextest run -p ferroehr -p ferroehr-rest        # while iterating
cargo nextest run --workspace                          # once, before commit
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --exclude ferroehr-admin-ui --all-features --no-deps
cargo deny check
bash scripts/check-comment-style.sh --all
bash scripts/conformance.sh                            # the acceptance instrument
```

Plus: the config template-sync tests (`ferroehr::config::tests`) for every new
key, `deploy/helm/validate.sh --update` golden renders, the
`website/book/src/installation/configuration.md` page, and a `CHANGELOG.md`
`[Unreleased]` entry (the guard fails otherwise).

**CNF SEC coverage — claim → case.**

| Claim | Existing case | New case needed |
|---|---|---|
| protected operations refuse without credentials (401) | `SEC-AUTHENTICATED_ACCESS-unauthenticated_sweep` | — |
| a 401 carries a challenge | `SEC-AUTHENTICATED_ACCESS-www_authenticate_challenge` | — |
| the no-credentials 401 carries **no** `error` code (RFC 6750 §3.1) | — | `SEC-AUTHENTICATED_ACCESS-challenge_omits_error_without_credentials` |
| an invalid bearer token yields `error="invalid_token"` (RFC 6750 §3.1, RFC 9068 §4) | — | `SEC-AUTHENTICATED_ACCESS-invalid_token_error_code` |
| a malformed `Authorization` header yields 400 `invalid_request` (RFC 6750 §3.1) | — | `SEC-AUTHENTICATED_ACCESS-malformed_authorization_is_bad_request` |
| the Basic challenge advertises `charset="UTF-8"` (RFC 7617 §2.1) | — | `SEC-AUTHENTICATED_ACCESS-basic_challenge_charset` |
| a read-only principal is forbidden on writes (403) | `SEC-AUTHORIZATION_SEPARATION-readonly_write_denied` | — |
| an authorization deny on a bearer principal carries `error="insufficient_scope"` (RFC 6750 §3.1) | — | `SEC-AUTHORIZATION_SEPARATION-insufficient_scope_challenge` |
| a token for another audience is refused (RFC 7519 §4.1.3) | — | `SEC-AUTHENTICATED_ACCESS-wrong_audience_refused` |
| an issuer outage is 503, not 401 (RFC 9110 §15.6.4) | — | out of the runner's reach (needs a controlled IdP outage) → covered by the crate-level wiremock test, recorded as a boundary |
| anonymous EHRs stay operable | `SEC-ANONYMOUS_EHRS-anonymous_lifecycle` | — |

Every new case is one behaviour, spec-cited, with the SEC-BASIC guard the
existing cases carry, and the SUT ixit gains the principals/tokens each one
needs. **Coverage only ratchets up** — no existing case is narrowed.

**Drift discipline.** The rows marked **wire** in §5 (G1, G2, G3, G13, G16,
G22) will move the baseline. Each one is adjudicated **individually** on the
PR: the clause it implements, the request/response before and after, and the
new or updated CNF case that pins it. No wire delta is absorbed into an
aggregate "SEC lane updated" note, and no expectation is edited to match
observed behaviour — the RFC clause decides, then the catalogue records it
(`.claude/rules/cnf-triage.md`).

**Task checklist.**

- [ ] G6, G12, G15 (one config-validation pass) + template/book/Helm/changelog
- [ ] G1, G2, G3 (the challenge + status surface) + 4 new SEC cases
- [ ] G4, G5, G7, G8, G9, G10 (the RFC 9068 §4 pipeline rewrite)
- [ ] G11, G14 (key + KDF posture)
- [ ] G13 (roles ≠ scopes) + SEC case + changelog entry (nothing deployed — no migration path)
- [ ] G16 (dependency failures are not client errors)
- [ ] G18, G19, G20, G21, G23, G24 (the ABAC/PDP rewrite)
- [ ] G25 (dead workspace pin)
- [ ] G17 (the provenance + vocabulary rewrite; `docs/architecture.md` §access)
- [ ] `bash scripts/conformance.sh` — zero **unexplained** drift; every wire row adjudicated

**Exit criteria** (mirroring the issue's acceptance criteria):

- [ ] zero `EHRbase` / `v1 parity` / `Stage-[12]` / internal-doc citations under
      `app/ferroehr-rest/src/extensions/access/**`, `app/ferroehr/src/config/auth.rs`,
      `app/ferroehr/src/config/authz.rs` (grep-asserted)
- [ ] every §3 row reads `conforms`, `conforms by delegation`, or
      `not applicable` with a stated reason
- [ ] the RBAC/ABAC model, its evaluation order, and the deny-by-default
      posture recorded in `docs/architecture.md` §access
- [ ] CNF SEC lane green, every drift row adjudicated case-by-case

---

## 8. Owner decisions (resolved 2026-08-06)

Every question below is settled. The grounds are the vendored openEHR specs and
the RFCs only — no CNF case is cited as authority, because the catalogue is
partly our own design and cannot adjudicate its own expectations.

**Standing fact that removed three questions:** nothing is deployed. This is a
greenfield product, so there is no migration path, no compatibility shim and no
opt-out key anywhere in this work — each decision picks the correct behaviour
outright.

### 1. A subject-less EHR and the patient-scope gate → **DENY for patient-scoped callers**

`RM/docs/ehr/master04-ehr_package.adoc` §EHR Status is decisive: the subject is
a `PARTY_SELF` "enabling it to be made completely anonymous", and — the
operative sentence — "**If the anonymous form is used, the change will be made
elsewhere in a cross-reference table**". The specification therefore places the
patient↔record association OUTSIDE the EHR whenever the anonymous form is
chosen (`master04` §EHR Identifiers restates it: "`subject_id` — optional; the
use of `PARTY_SELF` allows completely anonymous EHRs").

So the CDR structurally cannot resolve the subject of an anonymous EHR, and a
patient-scope gate inside the CDR cannot be satisfied for one. Permitting is
the wrong reading of that: in a deployment that uses the anonymous form, EVERY
EHR is anonymous, so a permitting gate degenerates to "allow everything" — a
control that looks present and is vacuous. Denying keeps the linkage where the
specification puts it: resolved to an `ehr_id` before the request arrives, or
supplied through the attribute pipeline (`abac.patient_claim`).

Scope of the denial is narrow and must stay so: it applies ONLY to a caller
carrying a patient scope. A principal without one (administrative, or clinical
without patient narrowing) is unaffected, so anonymous EHRs remain fully
operable — which is what the specification requires of them.

### 2. Role carriers → **configurable claim path, RFC defaults**

Default to the RFC 9068 §2.2.3.1 carriers in order (`roles`, then `groups`,
then `entitlements`; `roles`/`entitlements` are SCIM attributes per RFC 7643
§4.1.2), with one configuration key naming a dotted claim path so a deployment
whose issuer nests them (a `realm_access.roles` shape) is configuration rather
than a code change. No vendor-specific path is hardcoded.

`scope` is removed as a role source outright. It is an OAuth2 authorization
scope, not a role assertion — and reading it as one made the "at least one
role" gate vacuous for every OIDC token, since `openid` became a role.

### 3. Error precedence for `ehr_get_by_subject` → **resolve the 400 first, then gate**

The released ITS-REST specification declares the parameter required:
`specifications/parameters/query/subject_id.yaml` carries `required: true`, and
`specifications/operations/ehr_get_by_subject.yaml` declares no 400 among its
responses (only 404) — the general status-code rules in the overview supply it.

A request with no `subject_id` is therefore invalid for EVERY caller: no
authorization decision can make it succeed. RFC 9110 §15.5.1 defines 400 as the
client-error case the server will not process, which is exactly this. Answering
403 asserts something false — that permission is the obstacle — and sends an
integrator hunting entitlements for a malformed request.

The usual "authorize before you validate" instinct does not apply: parameter
presence is a property of the caller's own request and reveals nothing about
server state. Validating first is also strictly cheaper, since it avoids a
remote-PDP round trip for a request that cannot succeed.

### 4. Basic base64 strictness → **tighten to padded-only**

RFC 7617 §2 defers to RFC 4648 §4, and RFC 4648 §3.2 requires the pad
characters "unless the specification referring to this document explicitly
states otherwise" — RFC 7617 does not. Padded is therefore the canonical form
and the current unpadded tolerance is a leniency with no authority behind it.
The wire outcome is unchanged either way (malformed credentials were already a
401), so this is strictness without behavioural risk, and the refusal gets a
test naming RFC 4648 §3.2.

### 5. RFC 9068 `typ` enforcement → **enforce §4 when `typ` IS `at+jwt`**

§2.1 makes `at+jwt` a SHOULD for the authorization server, and §4 step 1's MUST
attaches to tokens claiming the profile — the RFC deliberately prescribes
nothing for tokens that do not claim it. So `oidc.require_at_jwt` defaults
`false`: a token carrying `typ: at+jwt` gets the complete §4 rule set enforced,
while a token without it is validated under the general JWT rules (RFC 7519
§7.2, RFC 7515 §5.2, RFC 8725). Requiring the profile by default would reject
issuers that do not set it, including the identity provider our own quickstart
overlay ships.

### 6. Token revocation → **accepted boundary, documented**

We validate JWTs offline against the issuer's keys and add no RFC 7662
introspection path, so a revoked token stays acceptable until it expires. No
RFC obliges a resource server to introspect; RFC 7662 defines the mechanism
without requiring its use, and introspecting per request would put the issuer's
availability in the request path (cached introspection only trades the lag for
a shorter one).

The boundary is therefore deliberate and stated where a deployer will read it:
**revocation latency equals the configured access-token lifetime**, which the
authorization server owns, so short lifetimes are the control. This goes on the
security page, not only in a code comment — an accepted limit that only the
implementer knows about is not accepted, it is hidden.

