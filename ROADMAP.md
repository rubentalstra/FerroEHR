# EHRbase-rs — product roadmap

*Last revised: 2026-07-14. This is the forward-looking product roadmap; the
historical build record lives in `docs/PROGRESS.md`, the spec-compliance
ledger in `docs/blueprint/00-THE-BLUEPRINT.md`, and the live work tracker in
`docs/plans/WORKLIST.md`. Everything under "Shipped" is measured or
machine-verified — the project's standing rule is **no false claims**.*

## Where the product stands (v3.0.x)

A pure-Rust, headless, API-first openEHR CDR on PostgreSQL 18:

- **Spec-compliant, machine-verified** — ITS-REST 1.0.3 + AQL 1.1 + RM 1.2.0;
  the built-in conformance runner executes the full catalogue per release:
  **CORE PASS · STANDARD PASS · OPTIONS OBTAINED**, zero failing cases
  (`docs/conformance/ehrbase-rs/`).
- **Complete platform surface** — EHR/COMPOSITION/DIRECTORY/CONTRIBUTION,
  full versioning semantics, templates + deep validation, WebTemplate +
  FLAT/STRUCTURED, EHR Extract + TDD import, demographics, terminology
  (bundled + external FHIR TS), demographic + admin APIs.
- **Enterprise capabilities, shipped** — change events (transactional AMQP
  outbox), bidirectional FHIR R4 connectors + read façade, S3 multimedia
  externalization, RBAC/ABAC, integrated multi-tenancy (PostgreSQL RLS),
  ATNA audit, Helm/distroless deployment, full observability.
- **Measured against upstream EHRbase (Java)** on an identical clinical
  workload: lower p99 in every headline class, ~10× less memory, smaller
  storage per composition (`docs/benchmarks/COMPARISON.md`; both directions
  always published).

## Now — performance (P20)

Goal: **hold the best max-sustained-throughput number honestly** on the
fully-populated clinical workload. Tracker:
`docs/plans/p20-overhead-checklist.md` (32 receipts).

- [x] Write-path folding (~4 statements/commit), plan cache, index diet,
  admission/pool parity, validation-cost rewrites (items 30/31: RM-invariant
  pass ~2.9×, archetype walk ~2.4×) — knee moved **161.9 → 396.9 req/s**.
- [ ] Item 32 — RM-invariant per-node residual (in flight).
- [ ] The definitive two-SUT knee pair on the instrument-clean benchmark
  (shared 6/6 payload set; runs on a fresh branch after the v3.0.1 cut) —
  fills the head-to-head max-sustained row with measured data.
- [ ] Group-commit A/B (item 22), knee-run bottleneck profiling (item 27),
  ECC zero-drift close (item 19).

## Next — publication & compliance depth

1. **v3.0.1 release** — the P20 wins, the benchmark/conformance instruments,
   and the honest comparison surfaces.
2. **X1 — the public comparison page** (`docs/plans/x1-comparison.md`): the
   ECC matrix (ours vs upstream 2.34.0), the benchmark ladder + overlay
   curves, per-case upstream failure triage. Measured numbers only.
3. **W-2 — ECC skip elimination** (owner ruling: a case passes, fails,
   errors, or is N/A — never "skipped"): wire the remaining native-API-only
   surfaces or adjudicate N/A with citations; zero skipped outcomes.
4. **W-4 — full ADL2** (spec-exact, no deviation): ADL2/cADL2/ODIN source
   parser, the complete AOM2 semantic-validation catalogue, specialisation
   flattening, OPT2, template semantics.
5. **W-3d — SM chapter-register gap closure**: the remaining G-rows across
   the platform-service audit registers (`docs/design/sm-platform/`).
6. **P17 — SIM-B/SDF interop audit**: FLAT/STRUCTURED transformation-rule
   verification against the SDF spec tables; interop quality, not
   conformance-gated.

## Then — the admin console

**`ehrbase-admin-ui`** — a pure-Rust (Leptos) admin console over the ITS-REST
API, shipped as a third container image (design approved-in-principle:
`docs/design/ehrbase-admin-ui.md`; feature target: template manager,
point-and-click AQL query builder, JSON + XML views, EHR/version browsing;
the CDR itself stays headless — the console is a client, never a bypass).

## Later — horizon

- **HL7v2 connectors** behind the same integration-frame seam as FHIR
  (named posture; second priority after the FHIR pair).
- **Continued performance** — PG18 AIO tuning, pipelined hot reads,
  `JSON_TABLE` codegen, the deferred speculative indexes — always
  profile-first, re-laddered per change.
- **Operational maturity** — HA/scale-out guidance, PITR/backup drills,
  upgrade rehearsals, cache tier re-evaluation (Valkey noted for
  multi-instance Stage-2 deployments; single-node stays in-process).
- **Stage-2/3 enterprise archaeology** — remaining items tracked in the
  blueprint; features land only with spec grounding or an explicit
  our-own-design flag.

## Standing rules (apply to every row above)

Vendored specs are the oracle; ECC zero-drift gates every phase; the
conformance baseline only ratchets upward; comparisons publish both
directions; a claim without a committed measurement or citation does not
ship.
