# E1 — Eventing: contribution outbox → AMQP

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Governing design: docs/enterprise/product-roadmap.md §2.2 (owner-confirmed)
  → ADR-014; spec basis SM master02 §General Assumptions/§Interface Calls
  (brokers as protocol adapters; transactional equivalence); event semantics
  are spec-silent (our design, ADR-014).
- Gates: workspace suites green; full ECC zero drift (341/315/0); eventing
  integration tests against a real broker (testcontainers RabbitMQ).

## Tasks

- [x] 1. ADR-014 — eventing design record (outbox shape, at-least-once +
      per-EHR ordering, PHI-free envelope payload, filter model, broker
      abstraction AMQP-first/Kafka-ready).
- [x] 2. Outbox: migration 0002 (append-only on the ADR-013 baseline) —
      `event_outbox` table written in the SAME transaction as every
      contribution commit (vobject commit path); envelope = contribution id,
      ehr_id, per-version (vo_id, kind, version, change_type, template_id),
      committed_at, seq.
- [x] 3. Publisher: background task draining the outbox to a pluggable
      `EventPublisher` trait (lapin/AMQP impl first), at-least-once,
      per-EHR ordering, confirm-based ack + retry/backoff (backon), config
      (figment, off by default), graceful shutdown; management/health
      indicator + metrics.
- [ ] 4. Event filters ("Event Trigger" parity): subscription store
      (kind/template/archetype/change_type predicates) → routing keys /
      per-subscription queues; admin CRUD surface (config-gated extension
      routes).
- [x] 5. Tests: outbox written atomically with commits (testcontainers PG);
      end-to-end publish against testcontainers RabbitMQ (consume + assert
      envelope, PHI-free); failure injection (broker down → retry, no loss);
      filter routing.

## Exit criteria

- [ ] Suites green incl. the new broker tests; full ECC zero drift; ADR-014
      accepted; roadmap §1 scorecard row flipped to ✅.
