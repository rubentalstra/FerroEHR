# Change events (AMQP)

When something is committed to the CDR, downstream systems often need to
know — an analytics pipeline, a care-coordination service, a cache
invalidator. Rather than have them poll, EHRbase-rs can publish a small event
for every commit to an AMQP 0.9.1 broker (RabbitMQ). The events are designed
so you can fan them out broadly without leaking clinical data: they carry only
identifiers and metadata, never the record content.

## Delivery guarantees

The publisher is built on a **transactional outbox**, which gives it three
properties that matter for integration:

- **At-least-once delivery.** Every commit writes its event to an outbox table
  in the _same database transaction_ as the change itself — no commit without
  its event, no event without its commit. A background task drains the outbox to
  the broker and marks a row published only after the broker confirms it. A
  crash or retry may deliver a message more than once, so consumers deduplicate.
- **Per-EHR ordering.** Rows drain in global sequence order, and the drainer
  stops a batch on the first publish failure rather than skipping ahead — so
  an earlier event for an EHR is never overtaken by a later one.
- **PHI-free envelopes.** The message body carries only ids, version numbers,
  and metadata. To read the actual clinical content, a consumer calls back
  through the authenticated REST or native API.

```mermaid
flowchart LR
    commit["commit<br/>(composition / status / folder)"]
    tx[("same DB transaction")]
    node["clinical data"]
    outbox["event_outbox row<br/>(published_at = NULL)"]
    drain["outbox drainer<br/>(background task)"]
    broker["AMQP topic exchange<br/>ehrbase.events"]
    consumer["your consumer<br/>(bound queue)"]

    commit --> tx
    tx --> node
    tx --> outbox
    drain -->|"poll pending, publish, await confirm"| broker
    outbox -.->|"drained in seq order"| drain
    broker --> consumer
    consumer -.->|"fetch bodies via authenticated API"| commit
```

## The event envelope

Each published message is JSON (`application/json`). One contribution can touch
several versioned objects, and the publisher emits **one message per version**,
each under its own routing key. The envelope carries:

| Field | Meaning |
|---|---|
| `contribution_id` | the contribution this change belongs to |
| `ehr_id` | the EHR (may be null for a demographic contribution) |
| `committed_at` | the commit instant |
| `versions[]` | one entry per changed versioned object |
| `seq` | the delivery sequence number (monotonic) |
| `version_index` | which entry in `versions` this message is for |

Each `versions[]` entry has `vo_id`, `kind` (the RM type — `COMPOSITION`,
`EHR_STATUS`, `FOLDER`, `EHR_ACCESS`), `sys_version`, `change_type` (a numeric
audit change-type code — `249` creation, `251` modification, `523` deleted,
`666` attestation), and `template_id` (or null).

> [!TIP]
> Deduplicate on the pair `(contribution_id, version_index)` and process in
> `seq` order. That handles the at-least-once redelivery and preserves per-EHR
> ordering at the consumer.

## Routing keys and subscriptions

Messages are published to a **topic exchange** (default name `ehrbase.events`),
with a three-field routing key:

```text
<kind>.<change_type>.<template_id>
```

For example, `COMPOSITION.249.openEHR-EHR-COMPOSITION_encounter_v1`. When there
is no template, the last field is `-`; characters outside `[A-Za-z0-9_-]` are
collapsed to `_` so the key always has exactly three fields. Bind a queue with
the usual AMQP topic wildcards to select what you care about — for example
`COMPOSITION.*.*` for all composition changes, `*.523.*` for all deletions
(change type 523), or `#` for everything.

The server can also manage subscriptions for you. When the event-subscription
admin API is enabled (`EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED`), each enabled
subscription row causes the server to declare and bind a durable queue named
`<exchange>.<name>` (for the default exchange, `ehrbase.events.<name>`) with a
binding key built from the subscription's `kind` / `change_type` / `template_id`
predicates (a wildcard for any predicate left unset).

## Enabling it

Publishing is off by default. Configuration uses the `EHRBASE_EVENTS_` prefix
(the server loads configuration from defaults, an optional TOML file, then
environment variables, with `__` separating nested keys):

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE_EVENTS_ENABLED` | `false` | master switch |
| `EHRBASE_EVENTS_URL` | `amqp://guest:guest@localhost:5672/%2f` | broker connection URL |
| `EHRBASE_EVENTS_EXCHANGE` | `ehrbase.events` | topic exchange name (also the queue-name prefix) |
| `EHRBASE_EVENTS_TLS` | `false` | when true, upgrades an `amqp://` URL to `amqps://` |
| `EHRBASE_EVENTS_BATCH_SIZE` | `128` | rows drained per cycle |
| `EHRBASE_EVENTS_POLL_INTERVAL_MS` | `1000` | poll interval while the outbox is idle |
| `EHRBASE_EVENTS_PUBLISH_MAX_RETRIES` | `3` | retries per message before the batch stops |
| `EHRBASE_EVENTS_RETENTION_DAYS` | `7` | how long published rows are kept |
| `EHRBASE_EVENTS_PRUNE_INTERVAL_SECS` | `3600` | how often published rows are pruned |

> [!WARNING]
> The broker URL carries credentials, so keep it in a secret, not a plain
> environment file. For anything beyond a local broker, use a TLS connection
> (`EHRBASE_EVENTS_TLS=true` or an `amqps://` URL). The commit path never blocks
> on the broker — if it is down, events buffer in the outbox and drain when it
> recovers.

## Consuming events

A minimal consumer declares nothing new — it binds a queue to the exchange and
reads. In shell form with the RabbitMQ tooling:

```bash
# bind a queue to every composition creation, then consume
rabbitmqadmin declare queue name=my-consumer durable=true
rabbitmqadmin declare binding source=ehrbase.events destination=my-consumer \
  routing_key='COMPOSITION.249.*'
```

Each delivery is a JSON envelope as described above. Your consumer records the
`(contribution_id, version_index)` it has seen, and for anything it needs the
content of, it calls the CDR's REST API (for example
`GET /ehr/{ehr_id}/composition/{vo_id}`) with its own credentials — the event
told it _what_ changed; the authenticated API is where it reads the _data_.
