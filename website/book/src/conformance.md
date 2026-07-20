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

Conformance is checked by the **ehrbase-rs Conformance Catalogue (ECC)** — an
enumerated set of test cases derived from the openEHR platform specifications:
every case's expected status code, header, and body condition traces to the
openEHR Conformance test schedule or the ITS-REST specification text, never to
any server's observed behaviour. The catalogue covers every ITS-REST operation
and documented status code, every AQL 1.1 language construct exercised against
a corpus with golden result sets, and the Reference Model data-type and
archetype-constraint semantics turned into accept/reject matrices. Each case
has a stable id (`ECC-<AREA>-<NNN>`, for example `ECC-EHR-005` or
`ECC-VAL-042`), grouped into areas:

| Area | Scope |
|---|---|
| `EHR` / `STA` | EHR and EHR_STATUS operations |
| `COM` / `CTB` / `DIR` | Composition, contribution (change sets), directory |
| `TPL` / `SQR` | Template (OPT) and stored-query provisioning |
| `QRY` / `VAL` | AQL execution and content/archetype validation |
| `DEM` / `ADM` / `MSG` | Demographic, admin, and messaging services |
| `SEC` / `SIG` / `TS` | Security, version signing, terminology-server integration |

The run is what turns cases into a claim. A **profile verdict** — Core,
Standard, or Options — is computed all-or-nothing per capability directly from
the run; no verdict is ever hand-asserted. Every case runs in **both** wire
formats (JSON and XML), so the format is a first-class part of the result. A
case that cannot run in the current configuration (for example, a native-API
operation with no REST binding) is recorded as _skipped with a reason_ rather
than silently omitted.

> [!NOTE]
> The catalogue is the project's own framework, built from the currently
> pinned specifications (Reference Model 1.2.0, AQL 1.1.0, Terminology 3.1.0,
> and the ITS-REST edition the run identifies). It is not a port of any
> external test harness — the vendored openEHR conformance corpus is
> design-time reading and a source of input payloads only.

## The current result

The published run against EHRbase-rs reports:

<!-- Generated at build time from docs/conformance/ehrbase-rs/results.json by
     scripts/render-conformance-stats.sh — never hand-type numbers here (CI:
     scripts/check-conformance-numbers.sh). -->
{{#include ../generated/conformance-stats.md}}

The executions that did not pass are documented skips, each with a stated
reason, not failures. Options is _obtained_ because it aggregates optional
capabilities under an "any passes" rule, and the demographic, terminology, and
admin APIs are evidenced.

## Any server can be assessed

The runner is deliberately not tied to EHRbase-rs. It assesses **any openEHR
CDR reachable over HTTP** and emits the same artefact set for each system
under test, into its own directory:

- **EHRbase-rs** (the default) — the composed stack built from the current
  sources. This is the project's own gate: a phase can only close on a run
  with zero drift against the committed baseline.
- **Upstream EHRbase (Java)** — the official upstream image, composed
  automatically. Its results are recorded as comparison data, never as a
  gate, and a **fairness register** is applied: cases that exercise an
  EHRbase-rs extension (for example the demographic REST API or version
  signing) are reclassified as not-applicable rather than counted as upstream
  failures, each with a written reason.
- **Bring your own endpoint** — point the runner at any deployed CDR by URL
  and credentials. No code or adapter is needed; a target is a configuration
  entry.

### The specification-edition ladder

Different CDRs implement different editions of the openEHR REST
specification. In `auto` mode the runner starts each assertion at the highest
pinned edition and steps down until the server's wire matches. A lower-edition
match is never a silent pass — it is recorded as an **edition finding**, so
one run tells you which specification edition a server actually speaks. A
failure is only reported when no supported edition form matches the normative
assertion. For EHRbase-rs's own CI runs the edition is pinned instead, so the
ladder can never mask a wire regression.

## Running the suite yourself

The suite runs against a real, deployed server — the same container image and
stack a deployment uses — so the wire under test is always the production
artefact, never a re-wired in-process stub. From a checkout with Docker
available:

```bash
# our server, from the current sources (the default)
bash scripts/conformance.sh

# upstream EHRbase (Java), official image — comparison data
CONF_SUT=ehrbase-java bash scripts/conformance.sh

# any deployed CDR, by URL
CONF_SUT=byo CONF_BASE_URL=https://your-host/ehrbase/rest/openehr/v1 \
  CONF_AUTH=basic:user:password bash scripts/conformance.sh
```

The script brings up the selected stack (for `byo` it manages nothing), runs
the full catalogue in both formats, writes the artefacts to
`docs/conformance/<sut-name>/`, and tears the stack down. Exit code `0` means
every executed case passed; `1` means there were failures (the report is
still written so you can inspect them); `2` means the runner or the system
under test could not start.

Useful knobs (environment variables of the script, or flags of the underlying
`conformance run` CLI): `CONF_FORMAT` (`json`/`xml`/`both`), `CONF_PROFILE`
(restrict to one profile), `CONF_EDITION` (`auto`, or pin one), a case-id
`--filter`, and `conformance report --from results.json` to regenerate the
artefacts from a previous run without re-running.

## Reading the artefacts

A run writes one machine record and three human-readable documents to
`docs/conformance/<sut-name>/`. Each has a distinct job.

### The machine record

`results.json` is the single source of truth for a run: one entry per case
with its id, title, capability, the specification citation and schedule
reference it derives from, the formats exercised, the outcome, the number of
data sets, and its duration, alongside the identity of the system under test
and the specification versions. Every other artefact is generated from this
file — nothing downstream is hand-edited.

### The conformance report

`CONFORMANCE_REPORT.md` is the honest, scoped record of _this run_: the system
under test and its specification versions, a per-area execution matrix (how
many cases passed, failed, errored, or were skipped in each area), a per-case
detail table, the machine-computed profile verdicts, a failures section, and a
deviations section that lists every skip with its reason. Read this when you
want to know exactly what happened and why any case did not run.

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

When more than one system has been assessed, `conformance compare` merges
their machine records into a single side-by-side matrix
(`docs/conformance/COMPARISON.md`):

```bash
conformance compare \
  --from docs/conformance/ehrbase-rs/results.json \
  --from docs/conformance/ehrbase-java/results.json
```

The matrix reports what each server measured on the identical case set, with
the fairness reclassifications visible — measured numbers only, no editorial
adjustment.

> [!TIP]
> The four conformance badges in the project README (overall, Core, Standard,
> Options) are generated from the same run. A badge can never show PASS unless
> the machine verdict does — so a green badge is a claim you can immediately
> reproduce with `scripts/conformance.sh`.

## What conformance does not cover

The catalogue measures the openEHR platform surface. It deliberately does not
stand in for a performance benchmark (durations are telemetry only) and does
not cover the Better-style FLAT/STRUCTURED interoperability formats, which
have their own test suite. Optional capabilities left "not evidenced" in the
certificate (for example ADL 2 provisioning or the more advanced AQL
constructs) are exactly that — untested in this configuration — and are
reported as such rather than claimed.
