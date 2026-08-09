# Terminology servers

openEHR records carry coded values — a diagnosis, a route of administration, a
laboratory unit. Some codes come from openEHR's own terminology; others must be
validated against an external code system such as SNOMED CT or LOINC.
FerroEHR serves the bundled openEHR terminology in-process and can
additionally validate and expand coded values against any external FHIR R4B
terminology server.

## The bundled openEHR terminology

The server ships the openEHR terminology bundle (Terminology 3.1.0) and uses it
by default, with no external dependency. It answers the questions the platform
needs during validation and querying: which terminologies exist, whether a code
belongs to one, what a term's rubric is, whether one code subsumes another, and
whether a code is a member of a value set.

You can also expose these lookups over a small read-only REST surface. It is an
extension (not part of the openEHR ITS-REST contract) and is **off by default**;
when disabled, every route returns `404` as if unmounted. Enable it with
`FERROEHR__TERMINOLOGY__API_ENABLED=true`, and it serves:

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/terminology` | list terminologies |
| `GET` | `/terminology/{terminology_id}` | describe one terminology |
| `GET` | `/terminology/{terminology_id}/term/{code}` | look up a term |
| `GET` | `/terminology/{terminology_id}/subsumes?ref_code=&candidate=` | subsumption test |
| `GET` | `/terminology/{terminology_id}/value_set/{value_set_id}` | get a value set |
| `GET` | `/terminology/{terminology_id}/value_set/{value_set_id}/validate?candidate_code=&at_date=` | validate a code |

(All paths are relative to the API base path, `/ferroehr/rest/openehr/v1`.)

## External FHIR terminology servers

A template can bind a coded element to an **external** value set via a
`terminology://…` reference — for example a FHIR value-set expand or
validate-code operation against a named value set. When external terminology is
enabled, the composition validator routes each such coded element to the
configured FHIR R4B terminology server: it resolves the coded value's system and
code and asks the server whether the code is a member of the value set. If it is
not, the composition is rejected along with any other validation errors.

The server prefers a direct code-validation check and falls back to expanding
the value set and testing membership where a server lacks direct validation.
Only the external bindings go to the FHIR server — openEHR and local
terminologies are still served by the in-process bundle.

> [!NOTE]
> **The CDR is only ever a client of the terminology server.** FerroEHR does
> not implement a terminology server; you run an off-the-shelf FHIR R4B server
> and point the CDR at it by URL. HAPI FHIR is a good open, single-container
> default for development and CI; Snowstorm is the opt-in choice for genuine
> SNOMED CT subsumption (heavier — it needs Elasticsearch and a SNOMED CT
> licence).

### Enabling and configuring it

External terminology is off by default; validation then uses only the
in-process bundle. These keys live in the `[terminology.external]` section of
`ferroehr.toml` (providers are a map, so complex blocks are usually supplied in
that TOML file); each can be overridden with the shown
`FERROEHR__TERMINOLOGY__EXTERNAL__*` environment variable, with `__` separating
nested keys:

| Key | Meaning |
|---|---|
| `FERROEHR__TERMINOLOGY__EXTERNAL__ENABLED` | master switch (default `false`) |
| `FERROEHR__TERMINOLOGY__EXTERNAL__FAIL_ON_ERROR` | on a server error: `true` rejects (fail-closed), `false` accepts (fail-open) |
| `FERROEHR__TERMINOLOGY__EXTERNAL__PROVIDERS__<NAME>__TYPE` | provider type — `fhir` (R4B) |
| `FERROEHR__TERMINOLOGY__EXTERNAL__PROVIDERS__<NAME>__URL` | the FHIR base URL, e.g. `http://terminology:8090/fhir` |

A provider can carry per-provider OAuth2 client-credentials and mutual-TLS
settings for servers that require them. A short worked example, pointing the CDR
at a HAPI FHIR container over Docker Compose:

```yaml
services:
  ferroehr:
    environment:
      FERROEHR__TERMINOLOGY__EXTERNAL__ENABLED: "true"
      FERROEHR__TERMINOLOGY__EXTERNAL__PROVIDERS__DEFAULT__TYPE: "fhir"
      FERROEHR__TERMINOLOGY__EXTERNAL__PROVIDERS__DEFAULT__URL: "http://terminology:8090/fhir"
```

> [!TIP]
> A FHIR terminology server starts empty. Seed the value sets your templates
> reference by uploading their CodeSystem and ValueSet resources over plain
> FHIR REST (`PUT` to `/fhir/CodeSystem/<id>` and `/fhir/ValueSet/<id>`);
> value sets are expanded on upload, so validation answers from the
> pre-computed expansion.

### Several terminology servers at once

Real deployments bind to more than one terminology — SNOMED CT from one
server, LOINC or a national code system from another. Every entry under
`[terminology.external.providers]` is materialised at startup, and a routing
map picks the one that answers each call:

```toml
[terminology.external]
enabled = true

[terminology.external.providers.snomed]
type = "fhir"
url = "https://snomed.example.org/fhir"

[terminology.external.providers.loinc]
type = "fhir"
url = "https://loinc.example.org/fhir"

# Terminology namespace -> provider name. Keys are matched whole-string and
# case-insensitively: a terminology id as an archetype binding writes it, a
# code-system URI, or a value-set URL.
[terminology.external.routes]
"SNOMED-CT" = "snomed"
"http://snomed.info/sct" = "snomed"
"http://loinc.org" = "loinc"
```

Selection is deliberately mechanical, so you can predict which server answers:
the caller offers candidate keys in priority order (the value set or coded
system first, then the AQL `service_api` flavour); the first key with a route
entry wins; otherwise the provider named `default` answers — or, when exactly
one provider is configured, that one. With two or more providers and no
`default`, an unrouted terminology has no server at all, which is a useful way
to make routing mistakes loud instead of silent. A route naming a provider
that does not exist fails at startup, never at request time.

### The `terminology` compose profile (development and CI)

From a checkout of the repository, the conformance stack can start a real FHIR
R4B terminology server (HAPI FHIR JPA) beside the CDR, seeded with a small set
of synthetic test code systems and value sets. Run it from the repository root:

```bash
docker compose -p ferroehr-cnf --project-directory . --profile terminology \
  -f docker/sut-ferroehr.yml -f docker/sut-terminology.yml up -d --wait ferroehr
```

The profile starts the server (host port `8090` by default,
`FERROEHR_TERMINOLOGY_PORT`) plus a one-shot seeding container that uploads the
fixtures over the server's own FHIR API and verifies `$validate-code` and
`$expand` before exiting. The `docker/sut-terminology.yml` overlay is what
points the CDR at it, by switching on the `[terminology.external]` providers
that `docker/ferroehr.dev.toml` already carries in the disabled state. None of
this touches the downloadable quickstart Compose file, which has no terminology
server and uses the in-process openEHR terminology only.

The seeded content is synthetic and lives under the reserved `example.test`
domain: one hierarchical, SNOMED-CT-*shaped* code system and one LOINC-*shaped*
one, each with an enumerated value set. It carries no licensed terminology
content — point the providers at a real server (and, for SNOMED CT, hold the
appropriate licence) for anything beyond experimentation.

The stock HAPI image keeps its database in memory, so restarting the
terminology container drops the seed; re-run the profile (or just the seeding
container) after one.

### When the terminology server cannot answer

`fail_on_error` decides what happens when a bound value set cannot be resolved
at all — the server is unreachable, returns an error, or does not know the
value set:

- `false` (the default, *fail-open*): the composition is accepted and a warning
  is logged. Availability of an external service does not block clinical
  writes.
- `true` (*fail-closed*): the composition is rejected with a validation error
  naming the unresolved binding.

A code that *is* resolved and turns out not to be a member of the bound value
set is a different matter: that is a real constraint violation and the
composition is rejected under either setting.

### On Kubernetes

Providers and routes are maps, so they are supplied as chart values rather than
env — which the `config` passthrough renders verbatim into `ferroehr.toml`
([Any server setting is reachable](../installation/kubernetes.md#any-server-setting-is-reachable-not-only-the-ones-listed-here)):

```yaml
# values.yaml
config:
  terminology:
    api_enabled: true
    external:
      enabled: true
      fail_on_error: true      # fail-closed: a server error rejects the commit
      providers:
        default:
          type: fhir
          url: https://tx.example.org/fhir
secrets:
  # only when a provider uses OAuth2; keyed by client name
  terminologyOauth2ClientSecrets: {}
```

**Before you enable it:** a reachable FHIR R4B terminology server, and a
decision on `fail_on_error` — fail-closed rejects commits while the server is
unreachable, fail-open accepts them unvalidated. **To turn it off**, set
`config.terminology.external.enabled: false`; validation falls back to the
in-process bundle and no external call is made.

## Terminology in AQL

Query authors can use the AQL `TERMINOLOGY()` function to constrain a match to a
value set — `TERMINOLOGY('expand', …)` resolves a value set and merges its
codes into a `matches` list at query-analysis time. See
[Querying with AQL](../querying-aql.md) for the query surface. Where an external
terminology operation is not yet supported, the engine returns a typed rejection
rather than a silent wrong answer.
