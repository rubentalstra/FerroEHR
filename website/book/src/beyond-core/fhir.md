# FHIR connectors

Many systems around a CDR speak FHIR. FerroEHR ships a set of FHIR R4B
connectors so it can take FHIR resources in, hand openEHR data back out as
FHIR, and emit FHIR resources to downstream systems — all driven by mappings
you control. It is not a full FHIR server; it is a focused, mapping-driven
bridge between the FHIR and openEHR worlds.

The connectors come in two independent switches — an inbound/read-façade
switch and an outbound-emission switch — because they have very different
data-exposure characteristics. All FHIR routes are relative to the API base path
(`/ferroehr/rest/openehr/v1`), use FHIR R4B, and speak `application/fhir+json`. A
resource type the connector does not map yet is answered with a FHIR
`OperationOutcome`, never a silent success.

## Inbound ingestion

`POST /fhir/r4/{resource_type}` takes a FHIR resource and stores it as a
validated openEHR composition. The connector resolves the mapping for the
resource type (and its `meta.profile`, if any), resolves or creates the EHR from
the resource's subject, builds a composition from the mapping, stamps it with a
`FEEDER_AUDIT` recording the FHIR origin, and commits it through the _normal_
validated write path. If the mapped composition fails validation, the request is
rejected with `422` and nothing is stored; a successful ingest returns `201`
with `ETag` and `Location` headers pointing at the openEHR composition. The
starter set of supported resource types is `Patient`, `Observation`,
`Condition`, and `DocumentReference`.

## Read façade

`GET /fhir/r4/{resource_type}?patient=<subject>` returns openEHR data
reverse-mapped into a FHIR `searchset` Bundle. The `patient` parameter is
mandatory (a missing one is a `400`) — this is a targeted façade, not a
general FHIR search. An optional `_count` caps the number of entries. Each
Bundle entry is a FHIR resource produced from a stored composition by running
the mapping in reverse.

## Outbound emission

Outbound emission publishes the mapped FHIR resource for every relevant
commit — but the target is an **AMQP broker (RabbitMQ), not an HTTP FHIR
server**. A
background task drains the same commit outbox used by
[change events](amqp.md), reverse-maps each committed composition through every
enabled mapping bound to its template, and publishes each resulting FHIR
resource to a topic exchange (default `ferroehr.fhir`) with a routing key of
`<resource_type>.<template_id>`. Delivery is at-least-once.

> [!WARNING]
> Outbound FHIR messages carry PHI — the payload is the mapped clinical FHIR
> resource itself, unlike the PHI-free [change-event](amqp.md) envelopes. That
> is exactly why they are a separate switch on a separate exchange
> (`ferroehr.fhir`, not `ferroehr.events`): broker access control can then isolate
> the PHI-bearing stream. Enable it only against a TLS, access-controlled
> broker, and treat every consumer as a PHI processor.

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
`meta.profile` URL) to **one openEHR template**, and lists field bindings —
each mapping an openEHR FLAT path to a FHIR path (or a constant), shaped by a
transform (plain text, date, quantity with unit, or a coded value with a
code-system-to-terminology mapping). The FHIR-path support is a deliberate
subset covering field navigation and array indexing (for example
`component[1].valueQuantity.value`), and the mapping is symmetric: the same
definition drives inbound ingest, the read façade, and outbound emission.

> [!NOTE]
> The template a mapping references must already be ingested (see
> [Templates & validation](../templates-validation.md)) — creating a mapping
> against an unknown template is a `400`. Mapping names are immutable once set,
> and a duplicate name is a `409`.

## Enabling the connectors

Both switches are off by default. The inbound/read-façade switch lives in the
`[fhir]` section of `ferroehr.toml` and the outbound emitter in
`[fhir.outbound]`; each key can be overridden with the shown `FERROEHR_*`
environment variable:

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__FHIR__API_ENABLED` | `false` | enable inbound ingest, the read façade, and the mapping API |
| `FERROEHR__FHIR__OUTBOUND__ENABLED` | `false` | enable outbound emission to AMQP |
| `FERROEHR__FHIR__OUTBOUND__URL` | `amqp://guest:guest@localhost:5672/%2f` | outbound broker URL |
| `FERROEHR__FHIR__OUTBOUND__EXCHANGE` | `ferroehr.fhir` | outbound topic exchange (kept distinct from the event stream) |
| `FERROEHR__FHIR__OUTBOUND__TLS` | `false` | upgrade an `amqp://` URL to `amqps://` |
| `FERROEHR__FHIR__OUTBOUND__BATCH_SIZE` | `128` | commits drained per cycle |
| `FERROEHR__FHIR__OUTBOUND__POLL_INTERVAL_MS` | `1000` | poll interval while idle |
| `FERROEHR__FHIR__OUTBOUND__PUBLISH_MAX_RETRIES` | `3` | retries per message |

When the inbound switch is off, the `/fhir/r4/*` and `/admin/fhir_mapping`
routes answer `404` without touching the backend. When the outbound switch is
off, no emitter task runs.

> [!NOTE]
> The connectors are also a **cargo feature** (`fhir`), on in the published
> images and any default build. A slim `--no-default-features` build contains
> none of their code and refuses to boot when `fhir.outbound.enabled` (or a
> configured FHIR terminology provider, or a FHIR `AuditEvent` audit sink) asks
> for it — see
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

**Before you enable outbound:** a reachable broker, and a deliberate decision —
it carries PHI off this system. **To turn either off**, set its switch to
`false` and upgrade: the inbound routes go back to answering `404` and no
emitter task runs.
