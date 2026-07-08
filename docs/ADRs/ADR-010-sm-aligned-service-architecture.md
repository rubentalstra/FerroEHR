# ADR-010: SM-aligned service architecture — the openEHR Platform Service Model as the internal decomposition

- **Status:** accepted
- **Date:** 2026-07-08
- **Builds on:** ADR-008 (greenfield PG18 storage + CNF conformance target),
  ADR-006 (idiomatic application on the generated crates), ADR-004/005
  (generated spec/ITS foundation) — all unchanged.
- **Design set:** `docs/design/sm-platform/` (digests 01–06, gap analysis 07,
  architecture 08, roadmap 09).

## Context

The openEHR **SM component** (Service Model) had not been used by this
project at all. It contains three specifications, vendored at
`docs/specs/openehr/SM/` (commit `23ffc471…`):

1. **openEHR Platform Service Model** (TRIAL) — the official decomposition of
   a CDR platform into ten named components (Definitions, EHR, Demographic,
   EHR Index, Query, Terminology, Message, System Log, Subject Proxy, Admin),
   each with abstract interfaces (`I_EHR_SERVICE`, `I_QUERY_SERVICE`, …)
   whose calls carry formal pre/post-conditions, transactional semantics, a
   `CALL_STATUS` error model, and the `UPDATE_VERSION`/`UPDATE_AUDIT` commit
   envelope. ITS-REST is a protocol-adapter realization of this model.
2. **Simplified Information Model 'B'** (DEVELOPMENT) — the `S_*` class set
   + `APP_CONTEXT` underpinning the SDT/FLAT formats.
3. **Serial Data Formats** (DEVELOPMENT) — the normative leaf-value string
   encodings for simplified formats.

A full extraction (design set docs 01–06) established: the specs are
TRIAL/DEVELOPMENT with real defects and stub interfaces (catalogued
per-digest), but the Platform model's *semantics* — command/query separation,
one-call-one-transaction, the version-commit envelope, the service
decomposition and naming — are exactly the contract our service layer
already implements implicitly (`Backend` seam, `vobject` engine). The gap
analysis (doc 07) shows the EHR core near-complete and six components
missing or partial.

Owner rulings (2026-07-08): adopt the SM across the whole application;
**full coverage — nothing deferred**, explicitly including `EHR_EXTRACT`
(Message service), TDD, Subject Proxy, the Terminology surface, EHR Index,
and the full Admin set, all Stage-1 scope.

## Decision

1. **The SM Platform Service Model becomes the internal decomposition of the
   application.** A new crate **`ehrbase-sm`** is the SM "native API": one
   Rust trait per SM interface (doc 08 §2 table), shared service types
   (`UpdateVersion<T>`, `UpdateAudit`, `Page`, execute specs, summaries),
   and the unified error table. `ehrbase-rest` becomes a pure protocol
   adapter over it (as SM's assumed architecture prescribes); `ehrbase`
   implements the traits. The current `ehrbase-rest::backend` trait family
   migrates and the EHR mega-trait splits along SM interface boundaries.
2. **Physical three-directory workspace layout (executed 2026-07-08).** The
   ADR-004/008 naming split becomes directory structure: the application
   crates move to **`app/*`** (`ehrbase`, `ehrbase-sm`, `ehrbase-rest`,
   `ehrbase-compat`, `ehrbase-audit`, `ehrbase-authz`, `ehrbase-signing`);
   the dev/verification tooling — not part of the shipped application —
   moves to **`tools/*`** with renames: `ehrbase-conformance` →
   **`tools/conformance`** (the ECC runner) and `ehrbase-bench` →
   **`tools/benchmark`**; the generated openEHR spec layer and its tooling
   stay in **`crates/*`** (`openehr-*`, `openehr-codegen`,
   `openehr-derive`). Root workspace
   `members = ["crates/*", "app/*", "tools/*"]`; moved with `git mv`
   (history preserved); all path references (workspace path-deps, CI,
   scripts, docker runners, `.claude/rules` scopes, docs) updated in the
   same mechanical commit. Dependencies point one way only:
   `tools/* → app/* → crates/*`.
3. **Precedence rule:** SM governs internal decomposition, naming, and call
   semantics (pre/post-conditions become test assertions). **ITS-REST 1.0.3
   + the CNF/ECC schedule remain the wire oracle** — SM is TRIAL; where the
   two disagree the wire spec wins at the boundary, recorded with
   `// PORT NOTE:` + citation. SM spec defects/stubs (catalogued in the
   digests) are filled by explicit design decisions, never silently.
4. **Full platform coverage** per the roadmap (doc 09): SM-1 native-API
   crate + EHR-core completion (contribution listing, attestations) →
   SM-2 Definitions completion (archetypes, ADL2, query calls) →
   SM-3 PARTY_RELATIONSHIP + EHR Index → SM-4 Terminology surface + Admin
   (statistics, archive, dump/load) → SM-5 Message service (**RM
   `ehr_extract` generated from the BMM**, extract export/import, TDD) →
   SM-6 Subject Proxy (variables, data sets, bindings, openEHR data-frame
   executor). `ehrbase-audit` is recognized as the realized System Log
   component.
5. **SM service types are hand-written idiomatic Rust in `ehrbase-sm`**
   (application layer, ADR-006 discipline): the SM component publishes no
   BMM (UML is MagicDraw-only), so ADR-004 codegen does not apply — except
   the RM `ehr_extract` package, which *is* in the RM BMM and is generated
   like every other RM package.
6. **SIM-B + SDF anchor the FLAT work** (P17): the `ctx/` vocabulary and
   transformation rules are audited against SIM-B; SDF-normative leaf
   encodings are accepted alongside Better forms; divergences documented.
7. **Wire exposure for components without an ITS-REST contract** (EHR Index,
   Terminology, Message, Subject Proxy, dump/load): extension routes under
   our own OAS, excluded from the ITS-REST drift check, migrating to
   `emit-rest` if openEHR publishes contracts.

## Consequences

- **Better:** the architecture gains the openEHR-official component map and
  vocabulary (procurement-recognizable service names, per SM §Overview); the
  service seam leaves the REST crate, making every future adapter (EhrScape,
  gRPC, queue) equal citizens; SM pre/post-conditions become an executable
  test oracle on top of ECC; six missing platform capabilities get a
  designed, cited build path instead of ad-hoc growth.
- **Honestly harder:** the SM is TRIAL with stub interfaces (`I_SYSTEM_LOG`,
  `I_MESSAGE_SERVICE`, empty enums, `@@` types) — we own the filled
  contracts and must track upstream changes; Subject Proxy pulls in
  PROC-adjacent concepts (`SYSTEM_CALL`) we must type ourselves; EHR_EXTRACT
  requires extending codegen coverage and canonical serialization to a
  package previously unemitted.
- **Risk control:** SM-1 is behaviour-preserving (ECC run must not move);
  each phase gates on the conformance suite; the trait split is mechanical.
- **Path churn (accepted):** the `app/*` move touches every path reference
  (workspace path-deps, CI, scripts, `.claude/rules` scopes, doc links);
  `git mv` preserves per-file history. One mechanical commit, gated on a
  green workspace.
- Docs updated: `docs/design/sm-platform/` (the design set),
  `docs/plans/sm-phase-01-native-api.md` (first phase),
  `docs/architecture.md` gains the component map at SM-1.

## Alternatives considered

- **Ignore SM (status quo).** Rejected by the owner: the project's charter
  is spec conformance; SM is the official service-layer spec, and the
  implicit `Backend` seam already converged on its shape — naming it costs
  little and buys the official contract.
- **Treat SM as wire-normative (rename REST params etc. to SM).** Rejected:
  SM is TRIAL and self-inconsistent (`item_offset` vs `row_offset`, enum
  gaps); ITS-REST 1.0.3 + CNF are the STABLE conformance instruments
  (ADR-008). SM governs inside, ITS-REST outside.
- **Generate the SM layer.** Rejected: no machine-readable SM source exists
  (no BMM; MagicDraw UML only) — hand-written idiomatic Rust in the app
  layer per ADR-006, with citations.
- **Keep the traits in `ehrbase-rest`.** Rejected: contradicts SM's
  native-API-behind-adapters architecture and couples every future adapter
  to the REST crate.
- **Partial scope (defer Message/Subject Proxy/EHR Index).** Rejected by
  the owner (2026-07-08): full coverage, EHR_EXTRACT included, Stage 1.
- **Keep the flat `crates/*` layout.** Rejected by the owner (2026-07-08):
  the spec-vs-application split is the project's load-bearing boundary
  (generated/never-hand-edited vs ours/idiomatic); making it physical
  (`crates/*` vs `app/*`) states the rule in the tree and keeps the
  dependency direction visually checkable.
