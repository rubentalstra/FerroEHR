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
  the 5-row pattern table (`*` / `**` / `ns::*` / `*::name` / exact), 5 permission
  letters c/r/u/d/s, the 8-row maximal scope table.
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
