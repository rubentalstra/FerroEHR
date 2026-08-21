---
name: smart-app-launch-location
description: Where the SMART on openEHR (ITS-REST DEVELOPMENT sub-spec) text lives — 9 adoc chapters, ZERO OAS/schemas, ZERO CNF anchor — plus the released-text defects confirmed
metadata:
  type: reference
---

# SMART on openEHR — location map (ITS-REST 1.1.0, DEVELOPMENT)

**The ONLY vendored text** is the adoc chapter tree
`docs/specs/openehr/ITS-REST/docs/smart_app_launch/` (9 chapters + `master.adoc`
+ `manifest_vars.adoc`). Chapter owners:

- `master00-amendment_record` — 1.0.0 = SPECITS-69 (09 Sep 2023); 1.1.0 reword
  (20 May 2025).
- `master01-preface` §Status — lifecycle via `{spec_status}` attribute.
- `master02-overview` — §Glossary (Platform / Launcher / Application / EHR
  reservation NOTE), capability list.
- `master03-registration` — out-of-band registration recommendation, RFC 7591.
- **`master04-service_discovery`** — THE `/.well-known/smart-configuration`
  chapter: full JSON example, §Authentication Endpoints (14-attribute list),
  §Services (`org.openehr.rest` required / `org.fhir.rest` recommended;
  `baseUrl` required + description/version/documentation/openapi), §Capabilities
  (the 4 openEHR-specific ones).
- `master05-application_types` — confidential/public × patient/practitioner/backend.
- `master06-authentication` — 4 grant flows, client-auth methods, §Deprecated
  Flows (the ONE uppercase MUST NOT in the whole spec), flow-recommendation table.
- **`master07-authorization`** — Standalone vs EHR("Embedded iFrame") Launch,
  §SMART Authorization Flow 3 steps (`aud`/`launch`/`scope`/`state`/`redirect_uri`),
  §Context Selection (`launch/patient`, `launch/episode`; token params `ehrId`,
  `episodeId`) + THE HL7-boundary NOTE ("not normative in this specification"),
  §Embedded iFrame Launch (`launch` scope).
- **`master08-scopes`** — THE grammar: `<compartment>/<resource>.<permission>`,
  3 compartments, exactly 3 resource nouns (`template-`/`composition-`/`aql-`),
  the 5-row pattern table (L42-46: exact-template / exact-query / `*::Template.v0` /
  `MyHospital::*` / bare `*`) — **NO `**` ROW EXISTS** (corrected 2026-08-21: an
  earlier version of this memory wrongly listed `**` as a row; the token appears
  only in the L37 prose and the L59 NOTE) —, 5 permission letters c/r/u/d/s
  (L51-55, `s` = "Search or execute (e.g., for AQL queries)"), the 8-row maximal
  scope table (L67-74).
- `master09-experimental_features` — `launch-base64-json`, Episode context
  (`launch/episode`, `episodeId`, `context-openehr-episode`).

## Hard negatives (verified 2026-07-27, exhaustive greps)

- **NO SMART OAS/schemas/operations/responses anywhere** in ITS-REST.
  `grep -ril smart` over the whole ITS-REST tree hits only the 11 chapter files
  + `manifest.json`. `specifications/` has 7 API groups (admin, definition,
  demographic, ehr, overview, query, system) — SMART is NOT one of them, and
  `computable/OAS/` has no smart bundle. So the SMART surface is prose-only.
- **CNF has ZERO SMART anchor**: `grep -rn "SMART\|smart_app\|well-known"` over
  `docs/specs/openehr/CNF/` returns nothing. No schedule chapter, no Robot suite.
  (There IS a `CNF/tests/platform/robot/SECURITY_TESTS/I_OAuth2_Keycloak/` suite,
  but it is plain OAuth2/Keycloak, never SMART.)
- **Exactly ONE RFC2119 uppercase keyword in the entire spec**:
  `master06` L38 "MUST NOT be used" (Implicit + ROPC grants). Everything else is
  lowercase "must"/"should" prose. Requirement extraction must say so.
- Lifecycle: `manifest_vars.adoc` `:spec_status: DEVELOPMENT` (mirrored in
  ITS-REST `manifest.json` → `specifications[].id == "smart_app_launch"` →
  `"spec_status": "DEVELOPMENT"`). The chapter renders it via `{spec_status}` in
  `master01 §Status` — no literal "DEVELOPMENT" string in the chapter text.

## Cross-component pointers

- 401/403 discipline is NOT in the SMART chapters — it is
  `ITS-REST/specifications/docs/overview/Requests_and_responses.md`
  §"Authentication and authorization" (L28-35) + the status table (L224-225).
  See [[its-rest-wire-contract-location]].
- No scope→route mapping exists in any vendored text (master08 names resource
  nouns, never REST paths). Any op→family/permission map is implementer-invented.

## Confirmed released-text defects / conflicts (candidate register entries)

1. `master04` L91 says "The following attributes in the
   **`.well-known/openid-configuration`** must match…" inside the chapter that
   otherwise defines `.well-known/smart-configuration` — wrong document name.
2. `master04` L66 example `scopes_supported` advertises `"patient/*.rs"` and
   `"user/*.rs"`, which the `master08` grammar cannot parse (`*` is not one of the
   three resource nouns). Released-vs-released conflict inside one spec.
3. `master08` L16 writes the syntax as `<permission>` but the bullet at L23 is
   `<permissions>` (plural) — editorial.
4. `master08` L6 is a comma splice with a missing conjunction ("…policies, the
   authenticated user's permissions.") — editorial.
5. No status codes (200/404/405), no auth posture, no caching/`Cache-Control`
   statement for the discovery endpoint anywhere — only the `application/json`
   media type (`master04` L28). Pre-auth availability is INFERRED from
   `master07` L80 (app fetches the doc using `iss` before the OAuth flow), never
   stated.

## Re-verification pass 2026-08-21 (issues #1591-#1597) — exact line anchors

All five defects above re-confirmed first-hand against the current pin. Extra
anchors + two anti-drift corrections worth keeping:

- `master04` §Authentication Endpoints list is L93-106 = **14 items, ending with
  `code_challenge_methods_supported`**; `capabilities` is the 13th (penultimate).
  §Capabilities is **two** sections later (§Services L108, §Capabilities L149),
  not three. Any report that says "the list ends with `capabilities` … three
  sections later" is off by one on both counts.
- `master08` **L55 DOES assign the letter**: "`s` - Search or execute (e.g., for
  AQL queries)", and L72 glosses `patient/aql-<queryName>.rs` as "Execute and
  read". So "the spec never says executing a query is `s`" is refutable; what is
  genuinely absent is any scope→REST-route/method/operation mapping (zero openEHR
  route path and zero operationId in all 9 chapters).
- **CORRECTION to an earlier "ZERO HTTP methods" claim (2026-08-21):** the tree
  contains **exactly ONE** HTTP-method mention — `master07` **L21** "… by
  POSTing to the Platform’s `token_endpoint`". A `\bPOST\b` grep misses it
  (`POSTing`); use `\b(GET|POST|PUT|DELETE|PATCH|HEAD)` with no trailing `\b`.
  It targets the OAuth token endpoint, so it still supplies no openEHR
  operation — but a blanket "no HTTP method appears" claim is falsifiable.
- **`master04` L153 ASSIGNS the patient-compartment binding value**:
  `context-openehr-ehr` — "Indicates support for EHR-level launch context,
  requested via `launch/patient` scope and conveyed via the **`ehrId` token
  claim**" — which with `master08` L19/L27 (`patient`: "Access is limited to the
  current EHR") and `master07` L47-52 (the `ehrId` token-response row) means
  "nothing says by what value the `patient` compartment is matched" is an
  OVER-claim. What stays open: the no-`ehrId`-claim fallback and multi-scope
  composition.
- Exhaustive greps over `*.adoc` only (the `diagrams/*.svg` pollute a tree-wide
  grep with coordinate digits): **zero** status codes, zero
  `cache|max-age|etag|freshness`, zero `error`, exactly ONE uppercase RFC2119
  keyword (`master06:38` MUST NOT). SVG label text extracted too — also clean.
- `master07` L61 NOTE scope: "standard SMART **launch** scopes and context
  attributes … their use is not normative" — it does NOT rescue the master04
  example's `patient/*.rs` / `user/*.rs` resource scopes.
- Conformance anchors: ITS-REST `specifications/docs/overview/Preface.md`
  §Conformance = "tbd." (L35-37); the 7 OAS groups are
  `specifications/{admin,definition,demographic,ehr,overview,query,system}.openapi.yaml`
  (+ `computable/OAS/<g>-{codegen,html,validation}.openapi.yaml`) — no SMART;
  CNF `docs/profiles/master03-profiles.adoc` §Functional (L11-71) REST-APIs block
  has 6 rows (DEFINITION/EHR/DEMOGRAPHIC/QUERY/ADMIN/MESSAGE), no SMART row.

