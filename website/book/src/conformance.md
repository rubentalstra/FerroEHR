# Conformance

EHRbase-rs makes a measured claim: it is an openEHR-spec-conformant Clinical
Data Repository, and that claim is backed by a test run you can reproduce, not
by prose. This chapter explains what conformance means here, how to run the
suite — against this server, against another CDR, or against any deployed
endpoint you point it at — and how to read the artefacts it produces: the
report, the statement, the certificate, and the cross-server comparison
matrix.

<!-- toc -->

## What is measured

Conformance is checked by the **CNF 2.0 reference runner** — a data-driven
interpreter over a committed, machine-readable catalogue authored from the
openEHR Conformance framework itself: protocol-neutral case cores anchored on
the official platform test schedule (case ids follow the schedule's own
naming, e.g. `I_EHR_SERVICE.create_ehr-main`), per-ITS operation bindings
mapping every outcome to its OpenAPI-cited wire expectation, closed
vocabularies for outcomes/selectors/captures, a provenance-stamped corpus
(the official openEHR Robot data sets re-adjudicated to spec-text-only
evidence), and a typed ambiguity register — a spec silence is never resolved
privately. Every expectation traces to specification text, never to any
server's observed behaviour. The catalogue spans the schedule's chapters:

| Chapter | Scope |
|---|---|
| EHR / EHR_STATUS | EHR service and status operations |
| COMPOSITION / CONTRIBUTION / DIRECTORY | Clinical content, change sets, folder trees |
| DEFINITION | ADL 1.4 + ADL 2 template and stored-query provisioning |
| QUERY | AQL query execution with committed result-set grounds |
| CONTENT | Reference-Model and archetype-constraint accept/reject tables |
| DEMOGRAPHIC / ADMIN / MESSAGING | Party, admin, and messaging services |
| SF / SEC / PERF | Simplified formats, security, performance classes |

The run is what turns cases into a claim. **Verdicts are computed, never
asserted**: a pure function rolls per-case outcomes up through the
capability→tier matrix from the CNF Profiles book into Core / Standard /
Options / SEC-BASIC profile verdicts, honouring the party statement's
declared capabilities and option selections. A case whose wire does not
exist on the technology profile, or whose ground a shared server cannot
establish, is recorded as _not applicable with a machine-readable citation_
rather than silently omitted.

> [!NOTE]
> The runner, catalogue, and verdict pipeline live in `tools/cnf-runner`;
> the published JSON Schemas for every artifact family are committed under
> `tools/cnf-runner/schemas/`. The instrument is built from the currently
> pinned specifications (Reference Model 1.2.0, AQL 1.1.0, Terminology
> 3.1.0, ITS-REST 1.1.0). The upstream Robot suites are reference material;
> their official data fixtures enter the corpus only as provenance-stamped
> re-adjudications.

## The current result

The whole conformance story in one picture — every capability of the
claims matrix, grouped by profile tier, colored AND glyph-marked by the
evidence its cases produced (both charts are generated from the committed
runner artifacts and regenerate-and-diff guarded in CI; no number on them
is hand-typed):

![Capability conformance heat grid](conformance-assets/conformance-heat-grid.svg)

The same run broken down by schedule chapter — passed, failed, errored and
cited-N/A per chapter:

![Schedule outcomes by chapter](conformance-assets/conformance-chapter-bars.svg)

The published run against EHRbase-rs reports:

<!-- Generated at build time from docs/conformance/ehrbase-rs/results.json by
     scripts/render-conformance-stats.sh — never hand-type numbers here (CI:
     scripts/check-conformance-numbers.sh). -->
{{#include ../generated/conformance-stats.md}}

Cases that did not execute are not-applicable with a machine-readable
citation (an unrealized wire on this technology profile, an undeclared
option branch, or a ground a shared server cannot establish) — never silent
omissions. Options aggregates optional capabilities under the Profiles
book's "any passes" rule.

## Any server can be assessed

The runner is deliberately not tied to EHRbase-rs. It assesses **any openEHR
CDR reachable over HTTP** and emits the same artefact set for each system
under test, into its own directory:

- **EHRbase-rs** (the default) — the composed stack built from the current
  sources. This is the project's own gate: a phase can only close on a run
  with zero drift against the committed baseline.
- **Upstream EHRbase (Java)** — `CONF_SUT=ehrbase-java` composes the
  official `ehrbase/ehrbase` image (with its companion PostgreSQL) on fresh
  volumes and runs the same catalogue with upstream's own committed party
  set. Its measured artifacts live under `docs/conformance/ehrbase-java/`
  and feed the [comparison page](comparison.md).
- **Bring your own endpoint** — point the runner at any deployed CDR by URL
  and credentials, with its own party set (an `ixit.json` naming the
  instances and credential environment variables, and a `statement.json` —
  the ICS — declaring the capabilities and ambiguity-register options the
  vendor claims; option branches the ICS does not declare are excused as
  not-applicable with a citation, ISO/IEC 9646-style test selection). No
  code or adapter is needed; a target is a configuration entry. The
  `ixit.json` may also declare deployment facts no openEHR operation
  exposes — currently `system_id`, the identifier the server stamps into the
  commit audit and into the version ids it mints. Cases that check such a
  value read the declaration; a party that declares none has those cases
  recorded not-applicable rather than checked against a guess.

## Running the suite yourself

The suite runs against a real, deployed server — the same container image and
stack a deployment uses — so the wire under test is always the production
artefact, never a re-wired in-process stub. From a checkout with Docker
available:

```bash
# our server, from the current sources (the default)
bash scripts/conformance.sh

# upstream EHRbase (Java), from the official images
CONF_SUT=ehrbase-java bash scripts/conformance.sh

# any deployed CDR, by URL (credentials via the SUT_* variables the
# ixit references)
CONF_SUT=byo CONF_BASE_URL=https://your-host/ehrbase/rest/openehr/v1 \
  SUT_USER=user SUT_PASS=password bash scripts/conformance.sh
```

The script brings up the selected stack on fresh volumes (for `byo` it
manages nothing), executes the committed catalogue, computes the verdicts
through the pure pipeline, and writes the artefacts to
`docs/conformance/<sut-name>/` before tearing the stack down.

Useful knobs: a case-id filter as the first argument, `CONF_IXIT` /
`CONF_STATEMENT` for a custom party set, `CONF_NO_COMPOSE` to run against
an already-deployed stack, and `cnf-runner verdicts` to recompute the
documents from a previous `results.json` without re-running.

## Reading the artefacts

A run writes one machine record and three human-readable documents to
`docs/conformance/<sut-name>/`. Each has a distinct job.

### The machine record

`results.json` is the party results record (its JSON Schema is published at
`tools/cnf-runner/schemas/results.schema.json`): one outcome per case with
its rows-driven coverage, failing step and reason where applicable, and the
excusing citation for every not-applicable entry, alongside the SUT
identity, the runner's verification-pack status, the technology profile,
and the ixit digest. `verdicts.json` is the computed verdict report. Every
other artefact is generated from these two — nothing downstream is
hand-edited.

### The conformance report

`CONFORMANCE_REPORT.md` is the honest, scoped record of _this run_: the
system under test, the outcome counts, the per-capability evidence rollup,
the machine-computed profile verdicts, and every not-applicable entry with
its excusing citation. Read this when you want to know exactly what happened
and why any case did not run.

### The conformance statement

`CONFORMANCE_STATEMENT.md` is the concise, generated claim: the supported
specification versions, the declared external data formats (JSON and XML), and
the profile results. Every line is a pure function of the machine verdicts, so
the statement can never claim more than the run proves.

### The conformance certificate

`CONFORMANCE_CERTIFICATE.md` follows the structure of the openEHR conformance
certificate template: the system under test, the scope of test, and a
per-capability profile report showing which capabilities are required in each
profile and whether each passed. It is emitted for **every** assessed system —
EHRbase-rs, upstream, or your own — and always identifies itself as a
framework assessment with the claim computed from the attached run; it is
never an official openEHR certification. This is the document to hand to a
procurement or evaluation reviewer who wants the capability-by-capability
picture.

### The comparison matrix

`docs/conformance/COMPARISON.md` is the fully generated multi-SUT record:
profile verdicts, the capability-by-capability evidence matrix, and failure
tables in both directions, derived from the two committed
results/verdicts sets (ours and upstream EHRbase's) by
`scripts/render-comparison.sh` — measured numbers only, no editorial
adjustment, both directions always published. The same content renders as
the [comparison page](comparison.md).

> [!TIP]
> The conformance badges in the project README are generated from the same
> run and carry the measured amounts (per-profile capability counts, the
> overall passed/driven case count). A badge can never show PASS unless the
> machine verdict does — so a green badge is a claim you can immediately
> reproduce with `scripts/conformance.sh`.

## What conformance does not cover

The catalogue measures the openEHR platform surface, including the
simplified (FLAT/STRUCTURED) formats chapter of the ITS-REST specification.
It deliberately does not stand in for a performance benchmark (durations
are telemetry only — the [benchmark harness](benchmarks.md) owns that
claim). A capability whose wire cannot exist on this technology profile
(for example ADL 1.4 archetype provisioning, which ITS-REST 1.1.0 defines
no endpoint for) is excused through the schedule's typed ambiguity register
and reported as an explicit scope exclusion on the certificate — never a
silent pass and never an unavoidable failure.
