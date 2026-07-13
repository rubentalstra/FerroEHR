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
- Runs against a deployed SUT over HTTP only (`scripts/conformance.sh`;
  the in-process self-host mode was removed by owner ruling). Multi-SUT
  is first-class (owner 2026-07-13): `CONF_SUT=ehrbase-rs|ehrbase-java|byo`
  / CLI `--sut byo --base-url …` — the framework assesses ANY openEHR CDR
  and emits the full artefact set incl. the Certificate for every SUT
  (always a framework self-assessment, never official openEHR
  certification). The fairness register applies to foreign SUTs only; the
  edition ladder is pinned to `development` for our own CI runs so it can
  never mask a regression.
- Profile verdicts (CORE/STANDARD/OPTIONS) are machine-computed by the
  runner — never hand-asserted in reports or docs.
- **ECC gate policy:** phase-close runs must show zero drift vs the
  committed baseline (`docs/conformance/ehrbase-rs/results.json` — per-SUT
  artefact dirs); the baseline only ratchets upward.
- Gates: `cargo clippy -p conformance --all-targets` +
  `cargo nextest run -p conformance` (the catalogue/guard tests).
