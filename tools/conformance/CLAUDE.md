# `conformance` — the ECC runner (tooling, not part of the app)

**Our own conformance framework** (ECC): own case numbering/taxonomy, own
generated data sets, latest-spec-versions-only. It is the ADR-008
acceptance instrument — never map cases to, or import from, the legacy
Robot/Python CNF suites.

- **Spine-first authoring (owner ruling):** every case's expectation
  traces to the CNF schedule / ITS-REST spec text
  (`docs/specs/openehr/CNF/`, `.../ITS-REST/`) — never to observed server
  behaviour. A case failing against our server is a correct instrument
  outcome, not a bug in the case.
- **Corpus/golden defects are handled ONLY via the adjudication registers**
  (`adjudications/`, skip-with-reason, recorded in the report) — never by
  editing a case to pass (blueprint §4 rule 3).
- Every case carries citation + schedule trace + binding (the derivation-
  square guard enforces this); no id literals, no silent fallbacks.
- Runs against the Docker-composed SUT only (`scripts/conformance.sh`);
  the in-process self-host mode was removed by owner ruling. Multi-SUT
  (`--sut-url`) exists for the X1 comparison; the fairness register gates
  SUT-kind-specific adjudications.
- Profile verdicts (CORE/STANDARD/OPTIONS) are machine-computed by the
  runner — never hand-asserted in reports or docs.
- **ECC gate policy:** phase-close runs must show zero drift vs the
  committed baseline (`docs/conformance/results.json`); the baseline only
  ratchets upward.
- Gates: `cargo clippy -p conformance --all-targets` +
  `cargo nextest run -p conformance` (the catalogue/guard tests).
