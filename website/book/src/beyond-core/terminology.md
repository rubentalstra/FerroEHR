# Terminology servers

openEHR records carry coded values: a diagnosis, a route of administration, a
laboratory unit. Some codes come from openEHR's own terminology; others must be
checked against an external code system such as SNOMED CT or LOINC. FerroEHR
serves the bundled openEHR terminology in-process, and can additionally validate
coded values against any number of external FHIR terminology servers at once.

<!-- toc -->

## The bundled openEHR terminology

The server ships the openEHR terminology bundle (Terminology 3.1.0) and uses it
by default, with no configuration and no external dependency. It answers the
questions the platform needs while validating and querying: which terminologies
exist, whether a code belongs to one, what a term's rubric is, whether one code
subsumes another, and whether a code is a member of a value set.

Enumeration is always the bundle's job. A lookup or a membership test goes to the
bundle when the bundle knows that terminology, and only otherwise to a
configured external server.

### Exposing the lookups over REST

You can also expose these lookups over a small read-only REST surface. It is a
FerroEHR extension (no openEHR REST contract defines a terminology API) and it
is **off by default**; while disabled, every route answers `404` as if it were
not mounted. Turn it on with `FERROEHR__TERMINOLOGY__API_ENABLED=true` and it
serves:

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/terminology` | list terminologies |
| `GET` | `/terminology/{terminology_id}` | describe one terminology |
| `GET` | `/terminology/{terminology_id}/term/{code}` | look up a term |
| `GET` | `/terminology/{terminology_id}/subsumes?ref_code=&candidate=` | subsumption test |
| `GET` | `/terminology/{terminology_id}/value_set/{value_set_id}` | get a value set |
| `GET` | `/terminology/{terminology_id}/value_set/{value_set_id}/validate?candidate_code=&at_date=` | test membership |

All paths are relative to the API base path, `/ferroehr/rest/openehr/v1`. An
unknown terminology or value set is a `404`; a missing required query parameter
is a `400`.

## External FHIR terminology servers

> [!NOTE]
> **The CDR is only ever a client of the terminology server.** FerroEHR does not
> implement one: you run an off-the-shelf FHIR terminology server and point the
> CDR at it by URL. HAPI FHIR is a good open, single-container default for
> development and CI; Snowstorm is the choice for genuine SNOMED CT subsumption
> (heavier: it needs Elasticsearch and a SNOMED CT licence).

External terminology is off by default, and while it is off nothing is
requested: validation uses the in-process bundle alone. The keys live under
`[terminology.external]`; the full table, including timeouts, caching, OAuth2
and mutual TLS, is on
[Integrations](../installation/config-integrations.md#terminology). The minimum
is a master switch and one provider:

```toml
[terminology.external]
enabled = true

[terminology.external.providers.default]
type = "fhir"
url = "https://tx.example.org/fhir"
```

Enabling the section with no provider configured is a **boot error**, not a
silent fall back to the bundle. So is an empty provider URL, an
`oauth2_client` naming no configured client, and half a mutual-TLS identity:
a control you configured either works or the server refuses to start.

### What it changes at commit time

The mechanism is openEHR's own **archetype constraint binding**. Where a
template binds an archetype constraint code (an `ac` code) to a query against an
external terminology, the specification puts the resolver *outside* the CDR: the
archetype holds an identifier for a query, and the query itself is defined in the
terminology server.

With external terminology enabled, committing a composition resolves those
bindings for every bound coded value the instance actually carries, against the
server the binding's terminology routes to:

- The code **is** in the value set → the commit proceeds.
- The code is **not** in the value set → `422`, naming the path, the code, and
  the bound query. That is a real constraint violation, so `fail_on_error` does
  not change it.
- The value set **could not be resolved** → `fail_on_error` decides, see
  [below](#when-the-terminology-server-cannot-answer).

Only bound external queries leave the process. openEHR and the bundle's own
local terminologies are still answered in-process.

> [!WARNING]
> The composition's `terminology_id` travels verbatim as the FHIR `system`
> parameter, and no openEHR specification defines a mapping between
> `terminology_id` values (`SNOMED-CT`) and FHIR system URIs
> (`http://snomed.info/sct`). If your archetypes and your terminology server
> disagree on that spelling, align them in the terminology-server configuration.
> The CDR does not rewrite the value.

### Which FHIR operation is used

Membership is tested with `ValueSet/$validate-code` by default: one direct
yes/no with the least payload. A server that does not offer it can be switched to
`$expand` plus a membership test with `operation = "expand"` on that provider.
This is a per-provider configuration choice, not an automatic fallback: the
server does not retry a failed `$validate-code` as an `$expand`, so set the
operation your server actually supports.

Responses are cached per provider (decoded, not raw), so a burst of commits
validating the same codes does not become one HTTPS round trip per code. A
response that is not a valid FHIR resource is treated as an upstream fault rather
than partially read, and takes the same path as an unreachable server.

### Several terminology servers at once

Real deployments bind to more than one terminology: SNOMED CT from one server,
LOINC or a national code system from another. **Every** entry under
`[terminology.external.providers]` is materialised at startup, and
`[terminology.external.routes]` decides which one answers each call:

```toml
[terminology.external]
enabled = true

[terminology.external.providers.snomed]
type = "fhir"
url = "https://snomed.example.org/fhir"

[terminology.external.providers.loinc]
type = "fhir"
url = "https://loinc.example.org/fhir"

# Terminology id or system URI -> provider name.
[terminology.external.routes]
"SNOMED-CT" = "snomed"
"http://snomed.info/sct" = "snomed"
"http://loinc.org" = "loinc"
```

Selection is deliberately mechanical, so you can predict which server answers:

1. The caller offers candidate keys in priority order: a terminology id, a
   system URI, a value-set URL, the AQL service flavour.
2. The first candidate with a route entry wins. Keys are matched
   **case-insensitively and whole-string**, never as a prefix.
3. Otherwise the provider named `default` answers, or, when exactly one
   provider is configured, that one.
4. With two or more providers and no `default`, an unrouted terminology has no
   server at all, and the call falls back to local behaviour.

Step 4 is a useful way to make routing mistakes loud instead of silent: add a
`default` only when you genuinely want a catch-all server. A route naming a
provider that does not exist fails at **startup**, never at request time.

Routing applies everywhere terminology is consulted: the `/terminology/*`
extension API, AQL `TERMINOLOGY(…)` resolution, and the commit-time binding
checks above.

### Authenticating to a server

A provider that needs a bearer token references a client-credentials client by
name; the token is cached and renewed shortly before it expires. The client
secret should come from a mounted file rather than an inline value;
`client_secret_file` under
`[terminology.external.oauth2_clients.<name>]`. A provider that authenticates
with a certificate instead gets its mutual-TLS identity **per provider**, because
the certificate is issued by that server's PKI. Both are configured on
[Integrations](../installation/config-integrations.md#authenticating-to-a-terminology-server).

There is no option to disable certificate verification. Server-certificate and
hostname verification are always on; a private-PKI trust bundle changes *which*
anchors are trusted, never *whether* the server is verified.

### When the terminology server cannot answer

`fail_on_error` decides what happens when a bound value set cannot be resolved at
all, whether the server is unreachable, answers an error, or does not know the
value set:

- `false` (the default, *fail-open*): the composition is accepted and a warning
  is logged. The availability of an external service does not block clinical
  writes.
- `true` (*fail-closed*): the composition is rejected with a validation error
  naming the unresolved binding.

A code that *is* resolved and turns out not to be a member is a different
matter: that is a constraint violation and is rejected under either setting.

Pick deliberately. Fail-closed means commits stop while your terminology server
is down; fail-open means they are accepted unvalidated. There is no third option
that gives you both.

## Running one locally (development and CI)

From a checkout of the repository, the conformance stack can start a real HAPI
FHIR JPA server beside the CDR, seeded with a small set of synthetic test code
systems and value sets:

```bash
docker compose -p ferroehr-cnf --project-directory . --profile terminology \
  -f docker/sut-ferroehr.yml -f docker/sut-terminology.yml up -d --wait ferroehr
```

The profile starts the terminology server (host port `8090` by default,
`FERROEHR_TERMINOLOGY_PORT`) plus a one-shot seeding container that uploads the
fixtures over the server's own FHIR API and verifies `$validate-code` and
`$expand` before exiting, so a misconfigured server fails there rather than
inside a later run. The overlay file is what points the CDR at it, by switching
on the `[terminology.external]` providers the development configuration already
carries in the disabled state.

None of this touches the downloadable quickstart Compose file, which has no
terminology server and uses the in-process openEHR terminology only.

> [!WARNING]
> The seeded content is synthetic and lives under the reserved `example.test`
> domain: one hierarchical code system shaped like SNOMED CT and one shaped like
> LOINC, each with an enumerated value set. It carries no licensed terminology
> content. Point the providers at a real server, and for SNOMED CT hold the
> appropriate licence, for anything beyond experimentation.

The terminology container mounts no volume, so its seeded content lives inside
the container only: re-create it and the seed is gone. Re-run the profile (or
just the seeding container) afterwards.

> [!TIP]
> Seeding your own server is the same shape: upload the CodeSystem and ValueSet
> resources your templates reference over plain FHIR REST (`PUT` to
> `/fhir/CodeSystem/<id>` and `/fhir/ValueSet/<id>`). A FHIR terminology server
> starts empty, and a value set it does not hold is an unresolved binding, which
> your `fail_on_error` setting then decides the fate of.

## On Kubernetes

Providers and routes are maps, so they are supplied as chart values rather than
environment variables; the `config` passthrough renders them verbatim into the
server's configuration file
([Any server setting is reachable](../installation/kubernetes.md#any-server-setting-is-reachable)):

```yaml
# values.yaml
config:
  terminology:
    api_enabled: true
    external:
      enabled: true
      fail_on_error: true      # fail-closed: an unresolvable binding rejects the commit
      providers:
        default:
          type: fhir
          url: https://tx.example.org/fhir
secrets:
  # only when a provider uses OAuth2; keyed by client name
  terminologyOauth2ClientSecrets: {}
```

**Before you enable it:** a reachable terminology server, a decision on
`fail_on_error`, and (if the chart's default-deny egress policy is on) an
egress rule that admits the server, or every call fails as a timeout.
**To turn it off**, set `config.terminology.external.enabled: false`; validation
falls back to the in-process bundle and no external call is made.

## Terminology in AQL

Query authors can constrain a match to a value set with the AQL `TERMINOLOGY()`
function: `TERMINOLOGY('expand', …)` resolves a value set and merges its codes
into a `matches` list at query-analysis time, so the planner sees an ordinary
value list. A `terminology://…` operand in a `matches` list is expanded the same
way. See [Querying with AQL](../querying-aql.md) for the query surface.

Operations with no defined comparison semantics in AQL are typed rejections
rather than silent wrong answers.
