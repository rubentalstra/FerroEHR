# Terminology servers

openEHR records carry coded values — a diagnosis, a route of administration, a
laboratory unit. Some codes come from openEHR's own terminology; others must be
validated against an external code system such as SNOMED CT or LOINC.
EHRbase-rs serves the bundled openEHR terminology in-process and can
additionally validate and expand coded values against any external FHIR R4
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
`EHRBASE_REST_TERMINOLOGY__ENABLED=true`, and it serves:

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/terminology` | list terminologies |
| `GET` | `/terminology/{terminology_id}` | describe one terminology |
| `GET` | `/terminology/{terminology_id}/term/{code}` | look up a term |
| `GET` | `/terminology/{terminology_id}/subsumes?ref_code=&candidate=` | subsumption test |
| `GET` | `/terminology/{terminology_id}/value_set/{value_set_id}` | get a value set |
| `GET` | `/terminology/{terminology_id}/value_set/{value_set_id}/validate?candidate_code=&at_date=` | validate a code |

(All paths are relative to the API base path, `/ehrbase/rest/openehr/v1`.)

## External FHIR terminology servers

A template can bind a coded element to an **external** value set via a
`terminology://…` reference — for example a FHIR value-set expand or
validate-code operation against a named value set. When external terminology is
enabled, the composition validator routes each such coded element to the
configured FHIR R4 terminology server: it resolves the coded value's system and
code and asks the server whether the code is a member of the value set. If it is
not, the composition is rejected along with any other validation errors.

The server prefers a direct code-validation check and falls back to expanding
the value set and testing membership where a server lacks direct validation.
Only the external bindings go to the FHIR server — openEHR and local
terminologies are still served by the in-process bundle.

> [!NOTE]
> **The CDR is only ever a client of the terminology server.** EHRbase-rs does
> not implement a terminology server; you run an off-the-shelf FHIR R4 server
> and point the CDR at it by URL. HAPI FHIR is a good open, single-container
> default for development and CI; Snowstorm is the opt-in choice for genuine
> SNOMED CT subsumption (heavier — it needs Elasticsearch and a SNOMED CT
> licence).

### Enabling and configuring it

External terminology is off by default; validation then uses only the
in-process bundle. Configuration uses the
`EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_` prefix, with `__` separating nested
keys (providers are a map, so complex blocks are usually supplied through a
mounted TOML file referenced by `EHRBASE_VALIDATION_CONFIG`):

| Key | Meaning |
|---|---|
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED` | master switch (default `false`) |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_FAIL_ON_ERROR` | on a server error: `true` rejects (fail-closed), `false` accepts (fail-open) |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__TYPE` | provider type — `fhir` (R4) |
| `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__<NAME>__URL` | the FHIR base URL, e.g. `http://terminology:8090/fhir` |

A provider can carry per-provider OAuth2 client-credentials and mutual-TLS
settings for servers that require them. A short worked example, pointing the CDR
at a HAPI FHIR container over Docker Compose:

```yaml
services:
  ehrbase:
    environment:
      EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED: "true"
      EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__DEFAULT__TYPE: "fhir"
      EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__DEFAULT__URL: "http://terminology:8090/fhir"
```

> [!TIP]
> A FHIR terminology server starts empty. Seed the value sets your templates
> reference by uploading their CodeSystem and ValueSet resources over plain
> FHIR REST (`PUT` to `/fhir/CodeSystem/<id>` and `/fhir/ValueSet/<id>`);
> value sets are expanded on upload, so validation answers from the
> pre-computed expansion.

## Terminology in AQL

Query authors can use the AQL `TERMINOLOGY()` function to constrain a match to a
value set — `TERMINOLOGY('expand', …)` resolves a value set and merges its
codes into a `matches` list at query-analysis time. See
[Querying with AQL](../querying-aql.md) for the query surface. Where an external
terminology operation is not yet supported, the engine returns a typed rejection
rather than a silent wrong answer.
