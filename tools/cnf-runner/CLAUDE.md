# `cnf-runner` — the CNF 2.0 reference runner (tooling, not part of the app)

The ground-up rebuild of the conformance instrument (#202, W1–W7): a
data-driven interpreter over the CNF 2.0 machine-readable schedule. The
schedule is the ONLY DSL — no cucumber, no Hurl, no second test-definition
language, ever.

- **Artifact families are the contract** (case cores, operation bindings,
  vocabularies incl. the capability→family→tier matrix, corpus manifest,
  ambiguity register, party artifacts statement/results/ixit). The committed
  JSON Schemas under `schemas/` are the published norm — emission is
  deterministic and drift-guarded by a nextest test; never hand-edit an
  emitted schema.
- **Every closed vocabulary is a Rust enum/newtype** (outcome kinds,
  selectors, header matchers, capture sources, the `${…}` reference grammar,
  dispositions, sentinels): illegal states unrepresentable. New vocabulary
  values enter only by schedule release, never ad hoc.
- **Cases speak SM + outcome kinds only** — nothing wire-level (no HTTP
  status, header, or media type) in a case core; wire lives in per-ITS
  operation bindings, each mapping cited to its OAS source.
- **Expectations trace to spec text** (`docs/specs/openehr/CNF/`, `QUERY`,
  `ITS-REST`) — never to observed SUT behaviour; EHRbase and ECC
  (`tools/conformance`, running untouched until the W2 comparison gate) are
  prior art, not oracles. Spec silences go through the ambiguity register
  with a typed `disposition`, never private resolution.
- **Verdicts are computed, never asserted** — pure functions of
  (statement, results, catalogue, capability matrix).
- Gates: `cargo clippy -p cnf-runner --all-targets` +
  `cargo nextest run -p cnf-runner` (schema drift, pilot acceptance,
  seeded-defect rejection, the §8.13-derived cross-artifact guards).
