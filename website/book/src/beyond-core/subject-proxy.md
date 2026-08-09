# Subject Proxy

The Subject Proxy Service lets applications read facts *about a subject* —
"date of birth", "latest systolic blood pressure", "current medications" —
without knowing which system holds them, what standard it speaks, or what
query language it uses. You register **variables** describing what you want
and bind them to **data frames** describing how to fetch it (an AQL query
against the CDR itself, a FHIR read against a remote server, or a manual
feed); the service executes the frames, tracks a sample history per variable,
and serves fresh values from that history without re-querying the source.

> [!NOTE]
> In this release the Subject Proxy is a **service capability, not a REST
> API**: there are no HTTP endpoints for it. It runs inside the server —
> exercised by in-process integrations — and what you configure is the set of
> external FHIR systems its frames may reach (below). The openEHR
> specification defines the service model; the wire exposure is future work.

<!-- toc -->

## The model

- **Subject** — the person (or other entity) the variables are about,
  registered by an external subject id (with a free-text category,
  default `individual`). For openEHR-backed variables the subject id is
  resolved to an EHR — a literal EHR id first, then a subject-id lookup.
- **Variable** — a named, typed fact about a subject: a `name` (optionally
  qualified by a `namespace`, giving a canonical `namespace::name` identity),
  a type, an optional `currency` (how fresh a value must be), and either a
  binding to a data frame (`frame_id` + `frame_path`) or the `is_manual`
  flag.
- **Data set** — an application's working set of variables for a subject,
  under **local aliases** (your app can call the canonical
  `date_of_birth` variable `dob`). Data sets track which applications use
  them (`using_app_ids`); when the last using application deregisters, the
  empty data set is dropped automatically.
- **Binding** — an environment's catalogue of **data frames**. Each frame
  names a retrieval method — an `API_CALL` (for example a FHIR read) or a
  `QUERY_CALL` (an AQL query) against a named system — plus an optional
  fallback method.

## Defining frames

A binding is a plain document (YAML or JSON — the two are interchangeable).
Frames reference systems by `system_id`, and `$subject_id` in a `query_text`
is substituted with the subject's id at retrieval time:

```yaml
env_id: prod
description: deployment environment
data_frames:
  - id: "openEHR::vital_signs"
    model_type: openEHR-EHR
    primary_method:
      _type: QUERY_CALL
      system_id: ehr1.nhs.org.uk
      call_name: aql_query
      query_text: SELECT c FROM EHR e CONTAINS COMPOSITION c
  - id: "fhir::demographics"
    model_type: HL7-FHIR_DSTU4_UK
    primary_method:
      _type: API_CALL
      system_id: pas
      call_name: fhir_get
      query_text: Patient/$subject_id
    fallback_method:
      _type: QUERY_CALL
      call_name: aql_query
      query_text: SELECT e/ehr_id/value FROM EHR e
```

A variable then points at a frame and a path within its result — for example
a `dob` variable bound to `fhir::demographics` with the frame path
`/birthDate`.

**Primary → fallback:** the primary method runs first; if it yields data, that
is the sample. If the primary is unavailable (source down, non-2xx, timeout,
malformed response) and a fallback is defined, the fallback runs and its
outcome — available or not — wins. Every attempt produces a sample either
way, so "the source was unreachable at 14:02" is itself recorded history.

## Sample history and currency

Every retrieval attempt for a variable is persisted as a **sample**: the
retrieve time, the real-world `effective_time` the data pertains to (for
FHIR reads, the resource's `meta.lastUpdated`), the value — or an
unavailability marker with the reason. The service keeps the most recent 100
samples per variable, newest first, so a variable read returns not just a
value but its recent history and provenance.

A variable's **`currency`** is an ISO 8601 duration expressing how fresh a
served value must be. On a read, if the newest stored sample's effective time
is within the currency window, it is served **without** re-querying the
source; otherwise the frame executes again. A variable with no currency means
"the most recent available value is valid" — any stored sample serves.
When an application registers a data set whose variables request a tighter
(shorter) currency than the stored definition, the variable's currency is
**tightened** to the stricter value — registration can only make data
fresher, never staler.

## Connecting FHIR systems

Frames of kind `API_CALL`/`fhir_get` read from remote HL7 FHIR R4B servers.
Which servers are reachable is **opt-in and fail-closed** configuration: only
systems named here can ever be called, and a frame naming an unconfigured
`system_id` is rejected with a typed error — never an arbitrary outbound
request. By default no system is configured and every FHIR frame is
rejected.

These keys live in the `[subject_proxy]` section of `ferroehr.toml`; each can
be overridden with the shown `FERROEHR__SUBJECT_PROXY__*` environment variable,
nested keys separated by `__`. Systems are a map keyed by the name frames use as
`system_id` (shown as `<NAME>`):

| Key | Type | Default | Meaning |
|---|---|---|---|
| `FERROEHR__SUBJECT_PROXY__SYSTEMS__<NAME>__BASE_URL` | URL | none (**required per system**) | FHIR R4B base URL; the frame's `query_text` (after `$subject_id` substitution) is resolved relative to it. |
| `FERROEHR__SUBJECT_PROXY__SYSTEMS__<NAME>__CONNECT_TIMEOUT_MS` | integer (ms) | `2000` | Per-system TCP connect timeout. |
| `FERROEHR__SUBJECT_PROXY__SYSTEMS__<NAME>__REQUEST_TIMEOUT_MS` | integer (ms) | `10000` | Per-system overall request timeout. |

For example, to let the `fhir::demographics` frame above reach a patient
administration system:

```bash
export FERROEHR__SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL=https://fhir.example.org/r4
```

Requests are sent with `Accept: application/fhir+json`; a timeout, error
status, or malformed body becomes an unavailable sample, which is what
triggers the frame's fallback.

### On Kubernetes

Systems are a map, so they are supplied as chart values and rendered verbatim
into `ferroehr.toml`
([Any server setting is reachable](../installation/kubernetes.md#any-server-setting-is-reachable-not-only-the-ones-listed-here)):

```yaml
# values.yaml
config:
  subject_proxy:
    systems:
      pas:
        base_url: https://fhir.example.org/r4
        request_timeout_ms: 10000
```

**Before you enable it:** a reachable FHIR R4B server per system, and — if the
chart's default-deny egress policy is on — a `networkPolicy.egress.rules` entry
that admits it, or the calls fail as timeouts. **To turn it off**, remove the
systems: with none configured every FHIR frame is rejected, which is the
fail-closed default.

## Manual variables

A variable with `is_manual: true` has no frame: its values are **pushed** in
by a notifier — typically a worker or device observing the subject — through
the service's sample-notification call. Reads serve the stored history;
until a first sample is pushed, reads return an unavailable sample saying
so. Pushing is accepted only for variables marked manual (or flagged
`ask_user`); pushing to a frame-bound variable is refused.
