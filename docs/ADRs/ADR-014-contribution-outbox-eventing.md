# ADR-014: Contribution-outbox eventing (AMQP-first change events)

- **Status:** accepted (design owner-confirmed 2026-07-10 via the product
  roadmap §2.2 decision)
- **Date:** 2026-07-10
- **Spec basis:** SM master02 §General Assumptions (Kafka/AMQP named as
  protocol adapters over the nominal native API) + §Interface Calls
  (adapters must preserve pre/post-conditions transactionally). Event /
  subscription semantics are **spec-silent** — this ADR is the design
  record filling that seam (roadmap §2.2; B8 spec-grounding pass).

## Decision

1. **Transactional outbox.** Every CONTRIBUTION commit writes one
   `event_outbox` row **in the same transaction** as the contribution +
   versions + audits (the commit path already has the single atomic point,
   `vobject`). No commit without its event; no event without its commit —
   this is the transactional-equivalence requirement of master02 applied to
   an eventing adapter.
2. **PHI-free envelope.** The event payload is the contribution envelope
   only: contribution id, ehr_id, per-version `(vo_id, kind, sys_version,
   change_type, template_id)`, committed_at, a monotonic sequence. Clinical
   content is NEVER in the event; consumers fetch through the authenticated
   REST/native API. (Least-exposure default for PHI; also keeps events
   small and broker-agnostic.)
3. **Delivery semantics: at-least-once, per-EHR ordered.** A background
   publisher drains the outbox in `(ehr_id, seq)` order, publishes with
   broker confirms, marks rows published only after confirm; crash/retry
   may duplicate, never lose. Consumers deduplicate on `(contribution_id)`.
   Retry with exponential backoff (`backon`); the outbox is the buffer when
   the broker is down (no back-pressure on commits).
4. **Broker abstraction, AMQP first.** An `EventPublisher` trait; the first
   implementation is AMQP 0.9.1 via `lapin` (RabbitMQ — the compose/test
   broker via testcontainers). Kafka is a later impl of the same trait.
   Publishing is **off by default** (config: `[events] enabled`, broker
   URL, exchange, TLS).
5. **Filters ("Event Trigger" parity).** Server-side subscriptions stored
   as config rows: predicates over `kind` / `template_id` / `archetype`
   (root) / `change_type`; each subscription maps to a routing key (AMQP
   topic exchange `ehrbase.events`, key
   `<kind>.<change_type>.<template_id|->`), so brokers do the fan-out;
   subscription CRUD via a config-gated admin extension surface. An
   AQL-shaped condition language is explicitly deferred.
6. **Retention.** Published outbox rows are pruned after a configurable
   retention window (default 7 days); the outbox is not an audit record —
   the contribution/audit tables remain the system of record.

## Consequences

- One append-only migration (0002) on the ADR-013 baseline: `event_outbox`
  (+ its indexes), following the baseline's naming/comment discipline.
- The publisher lives in the `ehrbase` binary (a tokio task with graceful
  shutdown), surfaced in `/management` health + metrics.
- Gates: outbox atomicity proven under testcontainers PG; end-to-end
  publish/consume + broker-down/retry proven under testcontainers RabbitMQ;
  full ECC zero drift (the wire is untouched).

## Alternatives considered

Webhooks-only (weaker enterprise story, no broker buffering); logical
decoding/CDC (ties consumers to storage internals — the outbox keeps the
event contract stable across schema changes, incl. future partitioning);
publishing inside the commit transaction directly to the broker (couples
commit latency/availability to the broker — rejected by the outbox
pattern's whole point).
