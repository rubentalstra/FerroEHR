# FHIR connectors

Many systems around a CDR speak FHIR. FerroEHR ships a set of FHIR R4B
connectors so it can take FHIR resources in, hand openEHR data back out as FHIR,
and emit FHIR resources to downstream systems — all driven by mappings you
control. It is not a full FHIR server; it is a focused, mapping-driven bridge
between the FHIR and openEHR worlds.

There are **two independent switches**, an inbound/read-façade one and an
outbound-emission one, because the two have very different data-exposure
characteristics. All FHIR routes are relative to the API base path
(`/ferroehr/rest/openehr/v1`) and speak `application/fhir+json`; every response
on this surface — success or failure — is a FHIR resource, so an error arrives as
an `OperationOutcome` rather than the openEHR error body.

<!-- toc -->

## Inbound ingestion

`POST /fhir/r4/{resource_type}` takes a FHIR resource and stores it as a
validated openEHR composition. The connector resolves the mapping for the
resource type (and its `meta.profile`, when the resource declares one), resolves
or creates the EHR from the resource's subject, builds a composition from the
mapping, stamps it with a `FEEDER_AUDIT` recording the FHIR origin, and commits it
through the *normal* validated write path.

Outcomes worth designing against:

- A successful ingest is `201`, with `ETag` and `Location` headers pointing at the
  openEHR composition that was created.
- A mapped composition that fails validation is `422`, and nothing is stored.
- A resource type outside the connector's starter set — `Patient`,
  `Observation`, `Condition`, `DocumentReference` — is `501`, refused before the
  backend is touched.
- A type inside the starter set with **no enabled mapping** stored for it is
  `404`. This is the common first-run surprise: the connector is on, but nothing
  is mapped yet.

Provenance is not optional: the composition the CDR stores carries a
`FEEDER_AUDIT` naming the FHIR origin and the source resource's own id, so an
ingested record is always distinguishable from one authored in openEHR.

## Validating without committing (`$validate`)

`POST /fhir/r4/{resource_type}/$validate` is the ingest door's dry twin,
following FHIR R4's own
[validation operation](https://hl7.org/fhir/R4/resource-operation-validate.html)
convention. It runs the whole ingest pipeline — mapping resolution, the FLAT
build, the `FEEDER_AUDIT` stamp, and the *same validation the real commit
runs* — and **commits nothing**: no composition, no version, and no EHR is
created (the target EHR is resolved and reported, never touched).

The response is a FHIR `OperationOutcome`, and a completed validation is
`200` whichever way the verdict falls:

- **Valid**: `information` issues — the verdict naming the resolved template,
  plus the EHR disposition (`would commit into existing EHR <id>`, or
  `would create a new EHR for subject '<id>'`).
- **Invalid**: an `error` issue carrying the openEHR validator's rejection
  **verbatim** — the exact text the real ingest would refuse with as a `422` —
  plus the same disposition issue.

Operation-level failures mirror the ingest door: no enabled mapping is `404`,
a type outside the starter set is `501`, a malformed body is `400`, and the
disabled connector is `404`. This is what makes mapping development safe:
iterate on a mapping with `$validate` against real sample resources, and only
switch to the real `POST` once the outcome reads valid.

```bash
curl -s -X POST "$CDR/fhir/r4/Observation/\$validate" \
  -H 'Content-Type: application/json' \
  --data-binary @observation.json | jq '.issue[].diagnostics'
```

## Read façade

`GET /fhir/r4/{resource_type}?patient=<subject>` returns openEHR data
reverse-mapped into a FHIR `searchset` Bundle. Each entry is produced from a
stored composition by running the mapping in reverse.

The `patient` parameter is **mandatory** — a missing or blank one is a `400`. This
is a targeted façade, not a general FHIR search engine: there is no free-text
search, no chained parameter, and no `_include`. An optional `_count` caps the
entries returned per mapping.

## Outbound emission

Outbound emission publishes the mapped FHIR resource for every relevant commit —
but the target is an **AMQP broker (RabbitMQ), not an HTTP FHIR server**. A
background task drains the same commit outbox used by
[change events](amqp.md), reverse-maps each committed **composition** through
every enabled mapping bound to its template, and publishes each resulting
resource to a topic exchange (default `ferroehr.fhir`) with a routing key of
`<resource_type>.<template_id>`, both segments sanitised the way the change-event
keys are. Delivery is at-least-once.

Only composition versions produce messages: an `EHR_STATUS` or a `FOLDER` carries
no mappable template. A row that fails to reverse-map *deterministically* — a
defective stored mapping or template — is retried a few times and then parked:
logged at error level, naming the row, and skipped, so one bad commit cannot
head-of-line-block every later one. Broker and database failures are treated as
transient and never park a row.

> [!WARNING]
> Outbound FHIR messages carry **PHI** — the payload *is* the mapped clinical FHIR
> resource, unlike the PHI-free [change-event](amqp.md) envelopes. That is exactly
> why they are a separate switch on a separate exchange (`ferroehr.fhir`, not
> `ferroehr.events`): broker access control can then isolate the PHI-bearing
> stream. Enable it only against a TLS, access-controlled broker, and treat every
> consumer as a PHI processor.

> [!NOTE]
> The change-event publisher has a health indicator; the outbound emitter does
> not have one of its own today, so treat its broker as something to monitor at
> the broker rather than through the CDR's readiness surface. See
> [Operations](../operations.md).

## Mappings are data you manage

There are no bundled mapping files. Each mapping is a stored definition managed
through an admin API (classed under admin authorization):

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/admin/fhir_mapping` | list mappings |
| `POST` | `/admin/fhir_mapping` | create a mapping (`201`) |
| `GET` | `/admin/fhir_mapping/{mapping_id}` | get a mapping |
| `PUT` | `/admin/fhir_mapping/{mapping_id}` | update a mapping |
| `DELETE` | `/admin/fhir_mapping/{mapping_id}` | delete a mapping (`204`) |

A mapping definition binds **one FHIR resource type** (optionally scoped to a
`meta.profile` URL) to **one openEHR template**, and lists field bindings — each
mapping an openEHR FLAT path to a FHIR path, or to a constant, shaped by a
transform.

**Resolution is two-step and deterministic.** An incoming resource resolves by
its type plus the *first* entry of `meta.profile` (only `meta.profile[0]` is
consulted): an enabled mapping whose `profile_url` **exactly matches** that URL
wins; otherwise the type's enabled mapping with **no `profile_url`** — the
type default — applies. A resource declaring no profile matches only the type
default. When neither exists, the ingest (and `$validate`) answer `404`.

The stored definition is the deployable artifact — the CDR stores it verbatim
and interprets it at ingest time, so a mapping deploys, updates, and rolls
back without a server release. Its shape (this is the whole contract — no
openEHR specification governs FHIR interop; the wire vocabulary follows
[HL7 FHIR R4](https://hl7.org/fhir/R4/)):

```json
{
  "resource_type": "Observation",
  "profile_url": "http://hl7.org/fhir/StructureDefinition/bp",
  "template_id": "blood_pressure.en.v1",
  "subject": {
    "reference_path": "subject.reference",
    "namespace": "fhir",
    "strip_prefix": "Patient/"
  },
  "context": {
    "ctx/language": "en",
    "ctx/territory": "US",
    "ctx/composer_name": "fhir-connector"
  },
  "entries": [
    { "openehr_path": "blood_pressure/blood_pressure:0/systolic",
      "fhir_path": "component.where(code.coding[0].code = '8480-6').valueQuantity.value",
      "transform": { "kind": "quantity",
        "unit_path": "component.where(code.coding[0].code = '8480-6').valueQuantity.unit" },
      "required": true },
    { "openehr_path": "blood_pressure/blood_pressure:0/diastolic",
      "fhir_path": "component.where(code.coding[0].code = '8462-4').valueQuantity.value",
      "transform": { "kind": "quantity",
        "unit_path": "component.where(code.coding[0].code = '8462-4').valueQuantity.unit" },
      "required": true }
  ]
}
```

The example binds the HL7 FHIR R4 core
[blood-pressure profile](https://hl7.org/fhir/R4/bp.html) (systolic LOINC
`8480-6`, diastolic `8462-4`, each a `component` of one `Observation`) to a
blood-pressure template's two quantity leaves. `subject` names where the
patient identity lives in the resource and how it becomes the EHR subject
(`Patient/p-42` → subject id `p-42` in namespace `fhir`); `context` supplies
the FLAT `ctx/` defaults every built composition carries (an omitted
`ctx/time` defaults to the ingestion instant). The transforms:

| Transform | What it writes |
|---|---|
| plain text | the bare FLAT leaf |
| date | an ISO 8601 date or date-time leaf |
| quantity | the magnitude and unit leaves, the unit read from the resource or fixed |
| coded | the code, the resolved openEHR terminology id, and optionally the display text |

A coded transform carries its own FHIR-code-system-to-openEHR-terminology map,
with `*` as the fallback for any unmatched system. An entry can be marked
required, which turns an absent source value into an error instead of a skipped
field.

A coded transform can also declare `translate`, asking for **cross-terminology
code translation** at ingest time:

```json
{ "openehr_path": "…/problem",
  "fhir_path": "code.coding[0].code",
  "transform": { "kind": "coded",
    "system_path": "code.coding[0].system",
    "translate": { "target_system": "http://snomed.info/sct",
                   "concept_map": "http://example.org/ConceptMap/my-map" } },
  "code_map": { "http://snomed.info/sct": "SNOMED-CT" } }
```

The server resolves each such code through a configured FHIR terminology
server's `ConceptMap/$translate` (routed by the openEHR terminology the
`code_map` binds the target system to; `concept_map` optionally pins one map).
Only a **strictly equivalent** match is taken — a `wider`, `narrower`, or
`relatedto` mapping is treated as no translation, because writing a
non-equivalent code would silently change clinical meaning. When no
translation exists, a required entry refuses the ingest and an optional one
writes nothing — the untranslated source code is never passed through under
the target terminology. A mapping that declares `translate` on a deployment
with no terminology server configured is refused as a server configuration
error rather than silently skipped.

The FHIR-path support is a deliberate subset of
[FHIRPath](https://hl7.org/fhirpath/): object-field navigation, zero-based
array indexing, `first()`, and single-condition `where(path = literal)`
filters — for example `code.coding[0].code`,
`code.coding.where(system = 'http://loinc.org').code`, and
`component.where(code.coding[0].code = '8480-6').valueQuantity.value` — not
the full FHIRPath language (no other functions, unions, or arithmetic).
FHIRPath's `where()` filters a collection; because a FLAT leaf holds a single
value, this subset takes the first matching element. The mapping
is symmetric — the same definition drives inbound ingest, the read façade, and
outbound emission — so a field you can ingest is a field you can serve back
(a translated entry serves back the stored, translated coding).

Each mapping also carries an `enabled` flag (default on). Only enabled mappings
resolve, for ingest, for the façade, and for outbound emission, which makes
disabling one a reversible way to take a resource type out of service.

> [!NOTE]
> The template a mapping references must already be ingested (see
> [Templates & validation](../templates-validation.md)) — creating a mapping
> against an unknown template is a `400`. A mapping's name is immutable once set,
> because it is its deployable identity, and a duplicate name is a `409`.

## The one route here that is not the connector

`GET /fhir/r4/AuditEvent` sits under the same path prefix but is **not** part of
the FHIR connector: it is the audit trail's own retrieval surface, returning
stored audit records as FHIR `AuditEvent` resources. It is gated by the local
audit record repository rather than by the connector switch, and it is admin-only.
See [Audit trail (IHE ATNA)](../audit.md).

## Enabling the connectors

Both switches are off by default. The inbound/read-façade switch lives under
`[fhir]` and the outbound emitter under `[fhir.outbound]`; the full table with
every default is on
[Integrations](../installation/config-integrations.md#fhir). The essentials:

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__FHIR__API_ENABLED` | `false` | mount inbound ingest, the read façade, and the mapping API |
| `FERROEHR__FHIR__OUTBOUND__ENABLED` | `false` | run the outbound emitter (carries PHI) |
| `FERROEHR__FHIR__OUTBOUND__URL` | a local development broker | outbound broker URL |
| `FERROEHR__FHIR__OUTBOUND__URL_FILE` | unset | read the broker URL from a mounted file instead |
| `FERROEHR__FHIR__OUTBOUND__EXCHANGE` | `ferroehr.fhir` | outbound topic exchange, kept distinct from the event stream |
| `FERROEHR__FHIR__OUTBOUND__TLS` | `false` | upgrade an `amqp://` URL to `amqps://` |

Batch size, poll interval, and publish retries are tunable as well.

When the inbound switch is off, `/fhir/r4/*` and `/admin/fhir_mapping` answer
`404` without touching the backend. When the outbound switch is off, no emitter
task runs. With authentication on, an unauthenticated request to a disabled group
is answered `401` first — the group gate sits behind authentication.

> [!NOTE]
> The connectors are also a **cargo feature** (`fhir`), on in the published images
> and any default build, and enabling it also enables `events` because the
> emitter drains the commit outbox. A slim `--no-default-features` build contains
> none of their code and **refuses to boot** when `fhir.outbound.enabled`, a
> configured external FHIR terminology provider, or a FHIR `AuditEvent` audit sink
> asks for it. `fhir.api_enabled` is the exception: those routes are simply not
> compiled in, so the setting has no effect rather than failing loudly. See
> [From source → Build features](../installation/from-source.md#build-features).

### On Kubernetes

Both switches are reachable through the chart's `config` passthrough
([Any server setting is reachable](../installation/kubernetes.md#any-server-setting-is-reachable-not-only-the-ones-listed-here)).
The outbound broker URL carries credentials, so it goes through
`secrets.fhirOutboundUrl`, which the chart mounts as a file:

```yaml
# values.yaml
config:
  fhir:
    api_enabled: true          # the read façade + mapping API
    outbound:
      enabled: true            # the emitter — carries PHI
      exchange: ferroehr.fhir
      tls: true
secrets:
  fhirOutboundUrl: "amqps://user:pass@broker.example:5671/%2f"
```

**Before you enable outbound:** a reachable broker, an egress rule that admits it
if the chart's default-deny egress policy is on, and a deliberate decision — it
carries PHI off this system. **To turn either off**, set its switch to `false` and
upgrade: the inbound routes go back to answering `404`, and no emitter task runs.
