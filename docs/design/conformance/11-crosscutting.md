# Conformance register 11 — Cross-cutting: security, signing, terminology, extensions

W-10 area audit (read-only, 2026-07-13) of the **cross-cutting non-functional +
extension** surfaces of `tools/conformance`: authentication (`SEC`), version
signing (`SIG`), and terminology-server integration (`TS`). Method is spec-first
(README + owner ruling), but with a twist recorded up front:

**There is no CNF schedule chapter for these areas.** The
`platform_test_schedule/` directory has functional suites master04–master13; it
has **no** security, signing, or terminology-integration suite. So the spine here
is **not** an enumerated schedule chapter — it is the
`docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` **capability
matrix** (the §Non-Functional table + the §Functional *Querying* "AQL &
terminology" row) plus openEHR's placement of authorization **out of band**. Each
capability row is enumerated below; the existing ECC cases are mapped onto it;
everything with no capability row is flagged **ECC-original extension**. This is
the honest consequence of the spec: these surfaces are tested against the
*profiles* oracle, not a test-schedule oracle.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — the capability
  × CORE/STANDARD/OPTIONS matrix. §Non-Functional: *Security & Privacy* →
  **Signing = STANDARD**, **Anonymous EHRs = CORE + STANDARD**. §Functional:
  *Querying* → **AQL & terminology = OPTIONS**. §Other Non-Functional: External
  Data Format = XML, JSON. There is **no** "Authentication" capability row —
  openEHR models authentication/authorization out of band.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  §Scope — the two tested aspects (API conformance, Data Validation conformance);
  neither is a security/signing aspect.
- `docs/specs/openehr/CNF/tests/platform/robot/SECURITY_TESTS/I_OAuth2_Keycloak`
  — the vendored Robot SECURITY suite. **Coverage evidence + raw material only**
  (ECC law: never mapped as machinery). Its substance ("05 Base URL is secured",
  "06 API endpoints are secured") is reproduced in intent, not id-mapped.
- `docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc` §Functions/Other
  functions/TERMINOLOGY (lines 748–767) — the AQL `TERMINOLOGY()` family the
  *AQL & terminology* capability tests.

**Mapped suites:** `suites/security.rs` (`ECC-SEC-001..002`), `suites/signing.rs`
(`ECC-SIG-001..005`), `suites/terminology.rs` (`ECC-TS-001..009`).

---

## 1. Verdict

Each cross-cutting area is **well-grounded in the profiles oracle and honest
about what it can and cannot observe over the wire**:

- **Signing (STANDARD)** — the `SIG` suite is the capability's *entire* evidence
  base (upstream ships zero signing material). Four digest cases prove
  presence + recomputation from the served canonical form (RFC 8785), the
  strongest being `sig/digest-recomputes` which recomputes `sha256:base64(...)`
  and would catch a genuine integrity divergence; the fifth (pgp) is
  `SKIPPED(SutConfig)` with the four digest cases proving the capability.
- **Anonymous EHRs (CORE)** — a non-functional capability, but its ECC evidence
  lives in **register 03** (`ECC-EHR-013` `ehr/create-anonymous-ehr`), not here.
  This register records the cross-reference so the CORE claim is traceable.
- **AQL & terminology (OPTIONS)** — the `TS` suite splits cleanly into three
  dispositions: bundle-expansion cases are real passes against any SUT with our
  engine; the FHIR-provider case passes when configured else
  `SKIPPED(SutConfig)`; fault-injection cases are `SKIPPED(SutConfig)` with
  off-wire fixture/wiremock evidence cited (the MSG precedent).
- **Authentication (no profile row)** — the `SEC` suite reproduces the Robot
  SECURITY intent (401 unauthenticated, 403 wrong role) with the honest
  adjudication that a black-box SUT's auth mode is not wire-readable, so a
  non-401/403 is `SKIPPED(SutConfig)`, never a fabricated pass or fail.
  `Capability::Authentication` is deliberately **not** profile-gated
  (`security.rs:74`), matching the spec (security is Non-Functional, and
  authorization is out of band).

The fairness picture for foreign SUTs is the load-bearing cross-cutting concern
(§4): **Signing and `/terminology` (+ our `TERMINOLOGY()` AQL family) are
ehrbase-rs extensions** already seeded as `extension` in the adjudication
register; the **SEC 401/403 surface is generic** (any auth-enforcing CDR) and
must stay live. Two rewrite themes dominate: (a) move the SUT-mode probing out of
case bodies into a declared runner capability-probe so a skip reason is uniform,
and (b) stop pinning the digest recompute + canonical-form to RM 1.2.0.

---

## 2. The spine (profiles capability rows → ECC map)

Rows are the `master03-profiles.adoc` capabilities, not a schedule chapter.
Profile column is that capability's CORE/STANDARD/OPTIONS placement. ECC
file:line as noted.

### *Security & Privacy* → **Signing** — §Non-Functional · STANDARD

Normative backing: `master03-profiles.adoc` §Non-Functional (the single line
"Signing … STANDARD"). No prose defines *how* — the openEHR RM only provides the
`VERSION.signature`/`commit_audit` slots; the `sha256:` digest format is an
**ehrbase-rs design** (no openEHR spec governs the digest algorithm — our own
extension). All rows therefore **ECC-original** (profile-anchored capability,
extension mechanism).

| Capability assertion | Condition | Data sets | ECC map — verdict |
|---|---|---|---|
| digest present on served VERSION | served `ORIGINAL_VERSION.signature` is a `sha256:<base64-32B>` (JSON + XML) | 1 composition, both formats | `ECC-SIG-001` `sig/digest-present` (`signing.rs:247`) — **conformant** (asserts shape in JSON body + `<signature>` XML element). |
| digest recomputes | served digest == `sha256:base64(SHA-256(canonical_form))` of the served version | 1 composition | `ECC-SIG-002` `sig/digest-recomputes` (`signing.rs:286`) — **conformant** (uses `openehr_rm::…::version_impl::canonical_form_of_json`; a mismatch is a real integrity finding). **VERSION-SPECIFIC** (canonical form is RM-1.2.0-shaped — G-3). |
| all object kinds signed | EHR_STATUS update, multi-version CONTRIBUTION, FOLDER write all yield signed versions | 4 data sets | `ECC-SIG-003` `sig/all-kinds` (`signing.rs:299`) — **conformant** for status + composition v1/v2; FOLDER signature is **not API-observable** (bare FOLDER, no ORIGINAL_VERSION wrapper) so proven off-wire by `app/ehrbase` `service_signing` SQL sweep — instrument-encodes-server-behaviour (documented PORT NOTE, `signing.rs:373`). |
| client signature verbatim | a CONTRIBUTION version's client-supplied signature is stored + served unchanged (never re-signed) | 1 CONTRIBUTION | `ECC-SIG-004` `sig/client-verbatim` (`signing.rs:418`) — **conformant** (distinctive non-digest sig round-trips). |
| pgp mode verifies | a `pgp`-keyed SUT serves an RFC 4880 detached signature | needs pgp SUT | `ECC-SIG-005` `sig/pgp-verifies` (`signing.rs:449`) — **SKIPPED(SutConfig)**: compose SUT boots in `digest` mode; the four digest cases prove the capability. |

### *Security & Privacy* → **Anonymous EHRs** — §Non-Functional · CORE + STANDARD

Normative backing: `master03-profiles.adoc` §Non-Functional. Evidence lives in
**register 03**, not this suite.

| Capability assertion | Condition | ECC map — verdict |
|---|---|---|
| anonymous EHR create | default `EHR_STATUS.subject` carries no `external_ref` (RM ehr master04 §EHR Status / common §PARTY_SELF) | `ECC-EHR-013` `ehr/create-anonymous-ehr` (`suites/ehr.rs:194`) — **conformant** (register 03 §3). Cross-referenced here so the CORE claim is traceable; no crosscutting-suite case duplicates it. |

### *Querying* → **AQL & terminology** — §Functional · OPTIONS

Normative backing: `master03-profiles.adoc` §Functional (*Querying* "AQL &
terminology" OPTIONS) + `QUERY/master03-syntax.adoc` §TERMINOLOGY (748–767). The
`TERMINOLOGY('expand', 'openehr', …)` bundle family is an **ehrbase-rs AQL
extension** (our engine feature); the FHIR-provider path realizes the spec
`service_api` mechanism.

| Capability assertion | Condition | Data sets | ECC map — verdict |
|---|---|---|---|
| bundle expand accepted | `matches TERMINOLOGY('expand','openehr',vs)` → 200 well-formed RESULT_SET | any SUT w/ engine | `ECC-TS-001` `ts/expand-bundle-accepted` (`terminology.rs:229`) — **conformant** (real pass). |
| bundle expand constrains | expansion yields the value set's *specific* codes (positive VS matches, disjoint VS does not) | fresh EHR, cat 433 | `ECC-TS-002` `ts/expand-bundle-constrains` (`terminology.rs:255`) — **conformant** (2 data sets). |
| mixed list merge | `matches {'433', TERMINOLOGY('expand',…)}` merges explicit + expansion (master03 line 759) | fresh EHR | `ECC-TS-003` `ts/expand-bundle-mixed-list` (`terminology.rs:293`) — **conformant**. |
| unknown value set | expand naming an unknown VS → typed 400 | any SUT | `ECC-TS-004` `ts/expand-unknown-value-set-rejected` (`terminology.rs:317`) — **conformant**. |
| unknown service_api | expand naming an unknown `service_api` → typed 400 | any SUT | `ECC-TS-005` `ts/expand-unknown-service-rejected` (`terminology.rs:328`) — **conformant**. |
| FHIR provider | FHIR `service_api` expand → 200 when a provider is configured | needs FHIR provider | `ECC-TS-006` `ts/expand-fhir-provider` (`terminology.rs:343`) — **conformant-when-configured**; else `SKIPPED(SutConfig)` on the typed `UnknownTerminologyService` reject (never fabricated). |
| fault → 500 (timeout / 5xx / malformed) | terminology-server fault maps to a server fault | fault-injecting tx | `ECC-TS-007/008/009` `ts/fault-*` (`terminology.rs:411/422/433`) — **SKIPPED(SutConfig)**: HTTP-only ECC cannot wire a fault-injecting tx into an external SUT; fault→500 proven off-wire by `ts::fixture` tests + `app/ehrbase/tests/terminology_fhir.rs` wiremock (cited). |

### **Authentication** — no profile capability row (out of band)

Normative backing: **none as a profile capability** —
`master03-profiles.adoc` has no Authentication row; openEHR places
authentication/authorization out of band (SEC test material exists only as the
Robot `SECURITY_TESTS/I_OAuth2_Keycloak` suite, coverage evidence only). These
rows are therefore **ECC-original (generic auth surface, Robot-intent)**, and
`Capability::Authentication` blocks no profile (`security.rs:74`, `profiles: &[]`).

| Capability assertion | Condition | ECC map — verdict |
|---|---|---|
| unauthenticated → 401 | unauthenticated GET of a protected route → 401 (Robot §06 "API endpoints are secured") | `ECC-SEC-001` `sec/unauthenticated-401` (`security.rs:96`) — **conformant-when-enforced**: 401 → pass; any other status → `SKIPPED(SutConfig)` (auth not enforced or mode indeterminable over the wire). Threads `schedule_ref` to the Robot §06 (`security.rs:55`). |
| wrong role → 403 | regular credential on an ADMIN-only route → 403 (role distinguished) | `ECC-SEC-002` `sec/forbidden-role-403` (`security.rs:116`) — **conformant-when-enforced**: 403 → pass; 401 (no regular cred) or other → `SKIPPED(SutConfig)` (role mode indeterminable). |

**Capability coverage:** the profiles matrix's cross-cutting rows — Signing
(STANDARD), Anonymous EHRs (CORE, evidenced in register 03), AQL & terminology
(OPTIONS) — are all evidenced; Authentication is evidenced as a generic
non-profile surface. No profiles cross-cutting capability is unmapped.

---

## 3. Existing ECC cases with no capability home

The suites map cleanly onto the profiles rows above; the ones to flag are
**extension mechanisms** riding on a profile-anchored capability:

| ECC | Suite | Nature | Flag |
|---|---|---|---|
| `ECC-SIG-001..005` | SIG | The `sha256:`/pgp digest mechanism is an ehrbase-rs design; no openEHR spec governs the digest algorithm (only the RM `VERSION.signature` slot). | **Keep, flag extension-mechanism**: profile-anchored (Signing STANDARD) but the *how* is ours; cited to `docs/design/version-signing.md`, not a spec. |
| `ECC-TS-001..005` | TS | The `TERMINOLOGY('expand','openehr',…)` bundle family is an ehrbase-rs AQL engine feature (the spec defines the `service_api` mechanism, not an `openehr` in-process flavour). | **Keep, flag extension**: real passes but engine-specific; a foreign SUT lacking our engine cannot satisfy them. |
| `ECC-TS-007/008/009` | TS | Fault-injection cases with no wire path on an external SUT. | **Keep, SKIPPED(SutConfig)** with off-wire evidence — the MSG precedent. |

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (SUT-mode probing belongs in a declared capability probe, not case
  bodies).** `SEC` and `TS`/`SIG` cases each re-implement "observe the status,
  decide pass vs `SKIPPED(SutConfig)`" inline (`security.rs:101`,
  `terminology.rs:343`, `signing.rs:449`). The rewrite hoists this into a runner
  capability-probe (auth-enforced? FHIR provider configured? signing mode?) run
  once, so each case asserts against a *known* SUT capability and the skip reason
  is uniform and machine-classified — not re-derived per case.

- **G-2 (fairness for foreign SUTs — the load-bearing cross-cutting concern).**
  The adjudication register (`adjudications/ehrbase-java-2.34.toml`) already
  seeds **`SIG` → `extension`** (version signing is ours) as a NotApplicable
  area, and the `TS` note defers per-case triage (bundle `openehr` cases are
  extension candidates; generic FHIR-tx cases are not). The rewrite must: (a)
  keep `SIG` a fairness-N/A area for any SUT that does not sign versions; (b)
  split `TS` so the bundle-`openehr` cases are `extension` while
  `ts/expand-fhir-provider` + faults stay generic (they test the spec
  `service_api` mechanism); and (c) keep **`SEC` fully generic** — 401/403 is
  every auth-enforcing CDR's surface, never an extension, and must stay live for
  foreign SUTs. `/terminology` wire exposure (config-gated, register would sit in
  ITS) is likewise an ehrbase-rs extension per the register note.

- **G-3 (signing canonical-form pinned to RM 1.2.0 — no ladder).
  VERSION-SPECIFIC.** `sig/digest-recomputes` recomputes via
  `canonical_form_of_json` over an RM-1.2.0-shaped served version
  (`signing.rs:227`). A SUT signing an RM-1.1.0-era version would fail the
  recompute even if its digest is internally correct. The rewrite must recompute
  against the served version's *own* shape at whatever RM edition the SUT
  advertises (version ladder), and record the satisfied level — or reduce the
  cross-edition assertion to shape+presence when the recompute basis differs.

- **G-4 (Anonymous EHRs evidence is cross-register — keep the link explicit).**
  The CORE Anonymous EHRs capability is evidenced only by `ECC-EHR-013` in
  register 03. The rewrite must ensure the profile math attributes that CORE
  capability correctly (it is not in any crosscutting suite) and that this
  register's cross-reference stays valid if register 03's case is renumbered.

- **G-5 (Robot SECURITY suite is evidence, never machinery).** `sec/*` cite the
  Robot `I_OAuth2_Keycloak` §05/§06 intent and reproduce it over Basic auth (the
  compose SUT runs no Keycloak). The rewrite keeps the Robot suite as coverage
  evidence + raw material (ECC law) — never id-mapped to Robot test names, never a
  Keycloak dependency — and may add an OAuth2/OIDC probe path for a SUT that does
  run federated auth, without making Keycloak a requirement.

- **G-6 (pgp signing has no compose profile).** `sig/pgp-verifies` is
  permanently `SKIPPED(SutConfig)` because no pgp-keyed SUT profile exists
  (`signing.rs:449`). The rewrite should add a pgp-mode compose profile so the pgp
  path is exercised at least once in CI, rather than resting solely on the digest
  cases for a STANDARD capability.

---

*Register 80 owns the composition/CONTRIBUTION/value-set fixtures the SIG/TS
cases consume; register 90 owns the capability-probe architecture (G-1), the
version ladder (G-3), and the fairness-register wiring (G-2/G-5) — plus the
`/terminology` extension-wire exposure note.*
