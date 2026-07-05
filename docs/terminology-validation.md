# External terminology validation (FHIR R4) — Rust-native design

- **Status:** design (not yet implemented)
- **Stage:** Stage 1, **P15 (composition validation)** — external terminology is
  one validator in the validation walker; requires P13/P14 (templates/
  WebTemplate) as prerequisites.
- **Date:** 2026-07-05
- **Reference (prior art, not a port target):** EHRbase → Explore → Terminology
  (`https://docs.ehrbase.org/docs/EHRbase/Explore/Terminology`). EHRbase uses
  Spring `WebClient` + Apache HTTP Client and Spring Security OAuth2. We
  implement the same behaviour natively with `reqwest`/`rustls`/`oauth2`
  (ADR-006/008).

## 1. What it does

openEHR templates can bind a coded element to an **external value set**. During
composition validation, each such coded element (`DV_CODED_TEXT` /
`CODE_PHRASE`) is checked against a remote **FHIR R4 terminology server (TS)**;
if the code is not a member of the value set, the composition is rejected.

The binding lives in the template's AOM as a `C_CODE_REFERENCE` constraint whose
`referenceSetUri` names a FHIR operation + value set, e.g.:

```
terminology://fhir.hl7.org/ValueSet/$expand?url=http://hl7.org/fhir/ValueSet/surface
```

A composition satisfying it carries a `CODE_PHRASE` whose `terminology_id.value`
is the value-set URL and whose `code_string` is a member code (e.g. `B` →
"Buccal" from `.../ValueSet/surface`). openEHR/local terminologies remain served
by `openehr-term`; **only external bindings** go to the FHIR TS.

## 2. Where it fits (P15 validation walker)

The composition validator walks the WebTemplate/AOM constraints against the
incoming COMPOSITION. When it reaches a coded node bound by a `C_CODE_REFERENCE`
with a `terminology://…` reference set:

1. Parse the `referenceSetUri` → `(provider host, FHIR operation, ValueSet url)`.
2. Resolve the coded value's `system` (`terminology_id.value`) + `code`
   (`code_string`).
3. Ask the configured FHIR TS whether the code is valid in that value set.
4. On "not valid" → a validation error (reject). On a **connectivity/TS error**
   → behaviour governed by `fail-on-error` (see §4): reject (fail-closed) or
   accept (fail-open).

This is one `Validator` among the RM-invariant and template validators; it is
skipped entirely when the feature is disabled or a node has no external binding.

## 3. Rust-native architecture

A small **FHIR terminology client** + a validator, no FHIR SDK:

- **HTTP:** `reqwest` (rustls) async client, one per configured provider, with
  connect/read timeouts and `backon` retry on transient failures.
- **Operation:** prefer FHIR **`$validate-code`** on the value set
  (`POST {base}/ValueSet/$validate-code` with `url`, `system`, `code`) — a
  direct yes/no with the least payload. Fall back to **`$expand`** + membership
  test where a server lacks `$validate-code`. Only the tiny FHIR `Parameters` /
  `ValueSet.expansion.contains` subset is modelled with `serde` (we do not pull
  in a FHIR model crate).
- **Caching:** a `moka` cache keyed by `(provider, valueset, system, code)` with
  a TTL, so repeated codes in a batch don't re-hit the TS; `$expand` results
  cached per value set.
- **Auth to the TS (this is where the `oauth2` crate belongs):** OAuth2
  **client-credentials** grant to obtain a bearer token for the TS. Note the
  contrast with our own API auth (ADR-006, `docs`/`auth`): there the CDR is a
  *resource server* (validate tokens, no `oauth2` crate); here the CDR is an
  OAuth2 **client** of the TS, which is exactly the `oauth2` crate's job. Tokens
  are cached and refreshed before expiry.
- **Mutual TLS (two-way SSL):** for TS that require client certs, build the
  `reqwest`/`rustls` client with a client identity (PEM or PKCS#12 → rustls
  `CertifiedKey`) and a custom trust root store. `rustls` + `rustls-pemfile`
  (already in the stack) rather than Java keystores.

Suggested layout: a `terminology` module in the `ehrbase` app (the FHIR client +
validator), invoked by the P15 validation walker; `openehr-term` keeps the
local/openEHR terminology. If it grows, split a `ehrbase-terminology` crate.

## 4. Configuration (Rust-native)

`figment`, `EHRBASE_VALIDATION_`- / `EHRBASE_TERMINOLOGY_`-prefixed. Providers
are a map (one or several). Only `fhir` (R4) is supported, matching the
reference.

```toml
[validation.external_terminology]
enabled = true
fail_on_error = true   # true = reject on TS/connectivity error (fail-closed);
                       # false = accept (fail-open), matching EHRbase's default

[validation.external_terminology.providers.ontoserver]
type = "fhir"
url  = "https://r4.ontoserver.csiro.au/fhir"
oauth2_client = "fhir-ts"          # optional; references an oauth2 client below
# operation = "validate_code"      # validate_code (default) | expand

[terminology.oauth2.fhir-ts]        # OAuth2 client-credentials for a TS
token_uri     = "http://idp.example/realms/test/protocol/openid-connect/token"
client_id     = "EHRBase-Test"
client_secret = "…"                 # Redacted in /management/env
scope         = "openid"

[terminology.mtls]                  # optional two-way SSL, applied per client
enabled          = true
client_cert_pem  = "/etc/ehrbase/tls/client.pem"
client_key_pem   = "/etc/ehrbase/tls/client.key"
trust_store_pem  = "/etc/ehrbase/tls/ca.pem"
```

Equivalent env keys use `__` nesting (e.g.
`EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED=true`,
`EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__ONTOSERVER__URL=…`).

Improvement over the reference: EHRbase supports only **one** auth method for
**all** TS; our design allows a **per-provider** `oauth2_client` (and per-client
mTLS), which is strictly more flexible at no extra cost. `fail_on_error` and
`enabled` match EHRbase's semantics and defaults (`enabled=false`,
`fail_on_error=false`).

## 5. Testing

- **Unit:** parse the `terminology://…$expand?url=…` reference into
  operation + value-set url; membership decision from a mocked `$validate-code`
  Parameters result and from an `$expand` list.
- **Integration (`wiremock`):** a mock FHIR TS returns `$validate-code`
  `result=true/false` → composition accepted/rejected; connectivity failure with
  `fail_on_error=true` → rejected, with `false` → accepted (fail-open); OAuth2
  token fetched from a mock token endpoint and attached; cache hit avoids a
  second TS call. mTLS handshake is a focused manual/opt-in test.
- **Oracle:** the reference example (`ValueSet/surface`, code `B` = "Buccal")
  as a golden accept case; an out-of-set code as the reject case.

## 6. Relationship to `openehr-term`

`openehr-term` holds the **local + openEHR** terminology bundle (codes served
in-process). This design covers **external** value sets only, selected by the
`terminology://` reference-set binding in the template. The validation walker
routes each coded node to whichever applies; both feed the same validation-error
accumulator so a composition reports all terminology failures at once.
