# ADR index — what is current

ADRs are immutable records — supersession is recorded in Status lines and
amendment banners, never by rewriting history. Precedence: the higher number
wins where two overlap (ADR-011 > ADR-010, ADR-008 > ADR-006 §3/§4, ADR-004 >
ADR-001/002); the blueprint (`docs/blueprint/00-THE-BLUEPRINT.md`) supersedes
the historical `PORT_MASTER_PLAN.md` everywhere they differ.

| ADR | Title | Status | What still stands |
|---|---|---|---|
| 000 | Template | n/a | The ADR skeleton (Context/Decision/Consequences/Alternatives). |
| 001 | Rust shapes for spec transcription (MI, covariance, generics) | superseded-in-part by 004 (as hand-authoring conventions) | The MI/covariance/closed-enum/generic *outcomes* still describe what the BMM emitter produces. |
| 002 | Canonical-JSON `_type` self-tagging | superseded by 004 (mechanism) + 008 (acceptance framing) | The requirement: `_type` self-tag emitted first, untagged closed enums on the wire — now realized by `#[derive(OpenEhrType)]`. |
| 003 | Policies for spec-underdetermined behaviour | current (only the crate name `openehr-foundation`→`openehr-base` is stale) | All temporal/modulo/URI/iteration policies still govern the hand-written `*_impl.rs` layer. |
| 004 | Spec-driven codegen of the spec crates from BMM | current (parity framing in Context/Consequences superseded by 008) | Generate the spec crates from BMM; never hand-edit `// @generated`. The load-bearing method. |
| 005 | Codegen of the ITS surfaces (XML + REST) | current (parity/P18 framing superseded by 008) | `emit-xml`/`emit-rest` generate canonical XML + the ITS-REST contract into `openehr-its`; the app implements the traits. |
| 006 | Application-layer port philosophy + stack + auth | §3/§4 superseded by 008; §1/§2/§5/§6 current | App = idiomatic Rust on the generated crates; the pinned stack; Basic + OAuth2/OIDC in Stage 1, RBAC in Stage 2. |
| 007 | Squashed sqlx baseline migrations | superseded by 008 (schema content) | Retained: the sqlx two-schema migrator, testcontainer gate, one squashed `0001_*` baseline per schema via `sqlx migrate add`. |
| 008 | Greenfield PG18 storage + AQL; CNF conformance replaces parity | current — the internals authority (read first) | `node` + temporal `vo_version`, `ALL_VERSIONS`, BMM-generated RM model, ECC/CNF as acceptance. (Roadmap P-refs are historical; the suite runs green.) |
| 009 | `opt14` stays separate from `am14` | current | Two deliberate OPT-XML vs BMM-logical constraint models + the divergence drift-guard test. |
| 010 | SM-aligned service architecture | amended by 011 (packaging) | The SM Platform Service Model as internal decomposition; SM-governs-inside / ITS-REST-governs-wire precedence; the `crates`/`app`/`tools` split; the full-coverage roadmap. |
| 011 | App-crate redesign (3 crates, `Platform`, compile-time complete) | current — the app-crate authority | Protocol-free `ehrbase-sm` native API; generic `Platform` (no dyn/stub/default bodies); audit/authz/signing as modules; the wire stays ITS-REST 1.0.3. |
| 012 | Closed-archetype validation for OPT 1.4 commits | current (B2 scope amendment inline) | Closed-archetype closure for archetyped content, RM metadata tolerated; the ECC zero-drift gate. |
| 013 | Enterprise-grade schema baseline (B7) | current — the schema authority with ADR-008 | Fresh squashed baseline; temporal PK + btree replica identity; 4-role security in migrations; spec fixes (ehr.system_id, change_type CHECK, merge provenance); named constraints + comments; perf indexes wired (owner call). |
| 014 | Contribution-outbox eventing (AMQP-first) | current | Transactional outbox on every contribution; PHI-free envelope; at-least-once per-EHR-ordered publisher behind an EventPublisher trait; filter subscriptions as routing keys; off by default. |
