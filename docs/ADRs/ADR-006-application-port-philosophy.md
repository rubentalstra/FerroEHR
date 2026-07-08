# ADR-006: EHRbase application-layer port philosophy + modern stack + auth

- **Status:** accepted, **§3 and §4 superseded by ADR-008** (own PG18 storage +
  AQL engine; openEHR CNF conformance replaces the EHRbase parity harness). §1,
  §2, §5, §6 (idiomatic modern-Rust app on the generated crates; the stack; auth
  in Stage 1) stand.
- **Date:** 2026-07-04
- **Amends:** `PORT_MASTER_PLAN.md` principles 1 & 3 and §10, **for the
  application layer only**. The spec layer was already superseded by ADR-004.
- **Builds on:** ADR-004 (spec crates generated from BMM), ADR-005 (ITS XML/REST
  generated). ADR-003 (spec-gap behaviour) still governs `openehr-*` `*_impl.rs`.

> ## ⚠️ AMENDMENT (2026-07-05, ADR-008): storage/engine are greenfield; conformance replaces parity
>
> §3 ("bespoke server logic **follows EHRbase's algorithm as the reference**";
> "composition versioning (current + `_history`)"; the AQL AST→ASL→SQL framing)
> and §4 ("the real EHRbase v2 schema is **reused verbatim**") are **superseded
> by ADR-008**. The application's storage, versioning, and AQL engine are now our
> own PG18-native designs (one `node` table + one temporal `vo_version` table —
> **no current/`_history` pairs**; a typed AQL IR of our own design over a
> BMM-generated RM model), and the acceptance instrument is the **openEHR CNF
> conformance schedule**, not the parity harness / `USE_REFERENCE_EHRBASE` gate.
> EHRbase is prior art, not an oracle. The rest of ADR-006 — build the app as
> modern idiomatic Rust on the generated `openehr-*` crates, the pinned stack,
> Basic + OAuth2/OIDC auth in Stage 1 — is unchanged. Read ADR-008 first.

## Context

`PORT_MASTER_PLAN.md` set two rules for reaching a working EHRbase:

1. **Literal 1:1 port** — mirror EHRbase's Java package/class/method/field
   structure and control flow (principles 1 & 3).
2. **Bun-style phased gate** — the early phases *need not compile*; a later
   phase makes the whole thing compile.

Both made sense when we expected to hand-transcribe the openEHR spec and port
~430 Java files verbatim. That world is gone:

- **The entire openEHR spec + serialization + REST-contract foundation is now
  generated and compiling** (ADR-004: `openehr-base/rm/am/term/lang`; ADR-005:
  `openehr-its` canonical XML `ToXml`/`FromXml` + the ITS-REST DTOs, server
  traits, and routes; `openehr-query` AQL lexer/parser/AST hand-written & done).
  The generated crates are idiomatic, strongly typed, and clippy-clean.
- What remains is the **EHRbase application server** — persistence, the service
  layer, the AQL execution engine, template ingestion/validation, the REST
  handlers, and auth.

The forcing question: do we still port that server as a literal 1:1 Java mirror
with a deferred-compilation gate, or build it as a modern idiomatic Rust
application on top of the generated crates? Mirroring Java class-by-class would
produce non-idiomatic Rust that fights the borrow checker, discard the
compiling foundation we already have, and re-implement (badly) what mature Rust
crates already provide (HTTP, auth, DB access, tracing).

## Decision

**Build the EHRbase application as a modern, idiomatic Rust service on top of the
generated `openehr-*` crates — spec-conformant and behavior-compatible with
EHRbase at the REST/AQL surface — not as a literal 1:1 Java-structure port.**

1. **`openehr-*` are the domain model, consumed directly.** The `ehrbase-*`
   crates depend on and use the generated spec crates (`openehr-rm`,
   `openehr-am`, `openehr-term`, `openehr-query`) and the generated ITS layer
   (`openehr-its`: canonical JSON/XML, the ITS-REST server traits + DTOs) as
   their types. We do not re-model the RM or re-serialize inside `ehrbase-*`.

2. **Modern idiomatic application stack — "the proper way", no hand-rolling what
   a good crate provides:**
   - HTTP: `axum` 0.8 + `tower`/`tower-http` (trace, cors, compression,
     timeout, request-id, sensitive-headers, catch-panic, normalize-path).
   - Persistence: `sqlx` 0.9 (typed exec, `migrate`, pool) + `sea-query` 1.0
     (dynamic SQL builder). **Not** sea-orm. `deadpool`/`tokio-postgres` only if
     a hot AQL read path needs pipelining (P20).
   - Auth: `jsonwebtoken`, `oauth2`, `openidconnect`, `argon2`, `axum-login`,
     `tower-sessions`.
   - API docs: `utoipa` 5 + Swagger UI (a drift-check against the vendored OAS,
     not the source of truth — ADR-005).
   - Caching: `moka` (template / WebTemplate). Time/ids: `jiff`, `uuid`.
     Config: `figment`/`config`. Errors: `thiserror` (libs) / `anyhow` (bin).
     Observability: `tracing` + OpenTelemetry. Tests: `cargo-nextest`, `insta`,
     `testcontainers` (real PG 18).

3. **Bespoke openEHR server logic follows EHRbase's algorithm as the reference,
   written idiomatically.** The AQL execution engine (AST → an abstract-SQL IR →
   Postgres JSONB SQL), composition versioning (current + `_history`),
   composition validation (a validation walker over the WebTemplate +
   terminology binding), and the RM↔JSONB row-per-locatable persistence mapping
   are logic no crate provides. We implement them in clean modern Rust
   **following EHRbase's proven approach** — its ASL intermediate representation,
   its row-per-locatable schema, its validation strategy — so behaviour matches;
   we do **not** mirror its Java class structure, and we do **not** reinvent the
   hard spec logic from a blank slate. The **parity harness** (drive our server
   and stock EHRbase with identical requests, diff responses; negative gate via
   `USE_REFERENCE_EHRBASE=1`) is the acceptance instrument.

4. **The real EHRbase v2 schema is reused verbatim.** The 41 Flyway SQL
   migrations already vendored in `app/ehrbase/migrations/{ehr,ext}/` are the
   schema, run via `sqlx migrate` — we do not re-author DDL.

5. **Authentication in Stage 1; RBAC in Stage 2.** Basic auth + OAuth2/OIDC
   (Keycloak-style) authentication ships with the Stage-1 server. Fine-grained
   RBAC / attribute-based authorization is a Stage-2 restoration (this mirrors
   how EHRbase Java layered authn vs the removed enterprise authz).

6. **Build compiling, tested increments** (retires the "early phases need not
   compile" gate for the app layer). Each application phase produces compiling,
   clippy-clean, tested Rust that consumes the already-compiling generated
   crates. The old "make it compile" rescue (now **P18 — Workspace integration**)
   is demoted to a final *integration* pass (wire the binary, delete the last
   ported-out Java).

## Consequences

- **Easier / better:** idiomatic Rust that uses the ecosystem instead of
  re-implementing it; the compiling generated foundation is leveraged, not
  discarded; each phase is independently verifiable; auth is first-class from the
  start; the deliverable is a maintainable modern service, not a Java transliteration.
- **Harder / honest:** "behaviour-compatible, not structure-identical" means the
  parity harness carries more weight — divergences (error bodies, AQL edge
  semantics, header handling) surface there, not by side-by-side class diffing,
  so the harness must be built early and taken seriously. The bespoke subsystems
  (above all the AQL engine) are still genuinely hard; "follow EHRbase's
  algorithm" is guidance, not a shortcut. The `ehrbase-*` Java stays in-tree as
  the read-only behavioural reference until each subsystem reaches parity, then
  is deleted (P99), same as before.
- **Docs:** all phase files, `PROGRESS.md`, `current-phase.md`, and
  `PORT_MASTER_PLAN §10` are rewritten to this philosophy and the
  dependency-ordered Stage-1 build sequence.

## Alternatives considered

- **Keep the literal 1:1 Java-structure port** (master-plan principles 1 & 3).
  Rejected: it produces non-idiomatic, borrow-checker-hostile Rust, throws away
  the generated compiling foundation, and reimplements what `axum`/`sqlx`/
  `oauth2`/`tracing` already do well. The value of a faithful port —
  drop-in behavioural compatibility — is delivered by the parity harness at the
  REST/AQL surface, not by class-level mirroring.
- **Greenfield reinvention from the openEHR spec**, consulting EHRbase only
  loosely. Rejected: re-solves problems EHRbase already solved (the ASL IR, the
  row-per-locatable schema, AQL path analysis) and maximizes the risk of subtle
  AQL/conformance divergence.
- **sea-orm as the primary data layer.** Rejected: EHRbase's data access is
  dominated by dynamically-generated JSONB-path SQL (the AQL engine) and a
  row-per-locatable decomposition that fit a query *builder* (`sea-query`), not
  an entity/ActiveModel ORM; sea-orm would be bypassed exactly where it matters.
- **Defer auth to Stage 2** (with RBAC). Rejected: a server without
  authentication is not usable or testable against real clients; EHRbase Java
  shipped Basic/OAuth2 authn in its core, with only the richer authz as the
  enterprise piece.
