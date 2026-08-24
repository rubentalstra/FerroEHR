# Subject Proxy

The Subject Proxy Service lets an application read facts *about a subject*
("date of birth", "latest systolic blood pressure", "current medications")
without knowing which system holds them, what standard it speaks, or what query
language it uses. You register **variables** describing what you want and bind
them to **data frames** describing how to fetch it (an AQL query against this
CDR, a FHIR read against a remote server, or a manual feed). The service runs the
frames, keeps a sample history per variable, and serves fresh values out of that
history without re-querying the source.

> [!IMPORTANT]
> In this release the Subject Proxy is a **service-layer capability, not a REST
> API**: no HTTP endpoints expose it. What you can configure today is the set of
> external FHIR systems its frames are allowed to reach ([below](#connecting-fhir-systems)),
> and the server builds that executor at startup when you name at least one
> system. The openEHR service model defines the operations; the wire exposure is
> future work.

<!-- toc -->

## The model

- **Subject:** the person (or other entity) the variables are about, registered
  by an external subject id, with a free-text category (default `individual`).
  For openEHR-backed variables the subject id is resolved to an EHR: a literal
  EHR id first, then a subject-id lookup.
- **Variable:** a named, typed fact about a subject: a `name` (optionally
  qualified by a `namespace`, giving a canonical `namespace::name` identity), a
  type, an optional `currency` (how fresh a served value must be), and either a
  binding to a data frame (`frame_id` + `frame_path`) or the `is_manual` flag.
- **Data set:** an application's working set of variables for one subject,
  under **local aliases** (your app can call the canonical `date_of_birth`
  variable `dob`). Data sets track which applications use them; when the last
  using application deregisters, the empty data set is dropped.
- **Binding:** an environment's catalogue of **data frames**. Each frame names a
  retrieval method: an `API_CALL` (for example a FHIR read) or a `QUERY_CALL`
  (an AQL query) against a named system, plus an optional fallback method.

## Defining frames

A binding is a plain document; YAML and JSON are interchangeable. Frames name
their system with `system_id`, and `$subject_id` inside a `query_text` is
substituted with the subject's id at retrieval time:

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

A variable then points at a frame and a path within its result, for example a
`dob` variable bound to `fhir::demographics` with the frame path `/birthDate`.

**Primary, then fallback.** The primary method runs first; if it yields data,
that is the sample. If it is unavailable (the source is down, answers a non-2xx
status, times out, or returns a body that will not parse) and a fallback is
defined, the fallback runs and its outcome wins, available or not. Every attempt
produces a sample either way, so "the source was unreachable at 14:02" is itself
recorded history rather than a gap.

## Sample history and currency

Every retrieval attempt is persisted as a **sample**: the retrieve time, the
real-world `effective_time` the data pertains to (for FHIR reads, the resource's
`meta.lastUpdated`), and the value, or an unavailability marker carrying the
reason. The most recent hundred samples per variable are kept, newest first, so a
variable read returns a value with its recent history and provenance. The
history is a bounded ring by design, not an unbounded log.

A variable's **`currency`** is an ISO 8601 duration saying how fresh a served
value must be. On a read, if the newest stored sample's effective time falls
inside the currency window, it is served **without** touching the source;
otherwise the frame runs again. Freshness is judged against the moment of
evaluation, which is the only reading that makes a duration with nominal parts
(months, years) decidable. A variable with no currency means "the most recent
available value is valid", so any stored sample serves. An unparseable timestamp
counts as stale rather than fresh.

When an application registers a data set whose variables ask for a **tighter**
currency than the stored definition, the variable's currency is tightened to the
stricter value: registration can only make data fresher, never staler.

## Connecting FHIR systems

Frames of kind `API_CALL` / `fhir_get` read from remote FHIR servers (the
FHIR release is the remote's property; the proxy relays `fhir+json` bodies
and decodes nothing release-specific). Which
servers are reachable is **opt-in and fail-closed**: only systems named in
configuration can ever be called, and a frame naming an unconfigured `system_id`
is a typed rejection, never an arbitrary outbound request. By default no system
is configured and every FHIR frame is rejected.

Systems are a map keyed by the name frames use as their `system_id`, under
`[subject_proxy.systems.<name>]`:

| Key | Type | Default | Description |
|---|---|---|---|
| `base_url` | string | required per system | The remote FHIR server's base URL. The frame's query text is resolved relative to this. Blank or absent is a boot error naming the system. |
| `connect_timeout_ms` | int | `2000` | TCP connect timeout. |
| `request_timeout_ms` | int | `10000` | Overall request timeout. |

```toml
[subject_proxy.systems.pas]
base_url = "https://pas.example.com/fhir"
```

To let the `fhir::demographics` frame above reach a patient administration
system:

```bash
export FERROEHR__SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL=https://fhir.example.org/r4
```

> [!TIP]
> The environment form takes a double underscore after `FERROEHR` **and** between
> every segment, and the map key is just another segment, so the system named
> `pas` becomes `…__SYSTEMS__PAS__BASE_URL`. Getting it wrong is not a silent
> no-op: any unrecognised variable in the reserved `FERROEHR_` namespace is a boot
> error with a did-you-mean suggestion, so a setting that never arrived cannot
> masquerade as a setting that had no effect.

Requests are sent with `Accept: application/fhir+json`, and the frame's
`query_text` (after `$subject_id` substitution) is resolved relative to the
system's base URL. A timeout, an error status, or a body that does not parse
becomes an unavailable sample, which is exactly what triggers the frame's
fallback.

### On Kubernetes

Systems are a map, so they are supplied as chart values and rendered verbatim
into the server's configuration file
([Any server setting is reachable](../installation/kubernetes.md#any-server-setting-is-reachable)):

```yaml
# values.yaml
config:
  subject_proxy:
    systems:
      pas:
        base_url: https://fhir.example.org/r4
        request_timeout_ms: 10000
```

**Before you enable it:** a reachable FHIR server per system, and (if the
chart's default-deny egress policy is on) an egress rule that admits it, or the
calls fail as timeouts. **To turn it off**, remove the systems: with none
configured every FHIR frame is rejected, which is the fail-closed default.

## Manual variables

A variable marked `is_manual` has no frame: its values are **pushed in** by a
notifier (typically a worker or a device observing the subject) through the
service's sample-notification call. Reads then serve the stored history; until a
first sample arrives, a read returns an unavailable sample saying so.

Pushing is accepted only for variables marked manual (or flagged `ask_user`).
Pushing to a frame-bound variable is refused, so a notifier cannot quietly
override a value the service is supposed to retrieve.
