# Conformance

EHRbase-rs makes a measured claim: it is an openEHR-spec-conformant Clinical
Data Repository, and that claim is backed by a test run you can reproduce, not
by prose. This chapter explains what conformance means here, how to run the
suite yourself, and how to read the artefacts it produces — the report, the
statement, and the certificate.

## What is measured

Conformance is checked by the **ehrbase-rs Conformance Catalogue (ECC)** — an
enumerated set of test cases derived from the openEHR platform specifications
the server implements: every ITS-REST operation and documented status code,
every AQL 1.1 language construct exercised against a corpus with golden result
sets, and the Reference Model data-type and archetype-constraint semantics
turned into accept/reject matrices. Each case has a stable id
(`ECC-<AREA>-<NNN>`, for example `ECC-EHR-005` or `ECC-VAL-042`), grouped into
areas:

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
> ITS-REST 1.0.3). It is not a port of any external test harness — the
> vendored openEHR conformance corpus is design-time reading and a source of
> input payloads only.

## The current result

The published run reports:

- **341 case-by-format executions, 315 passed, 0 failed.**
- **Core: PASS. Standard: PASS. Options: OBTAINED.**

The executions that did not pass are documented skips, each with a stated
reason, not failures. Options is _obtained_ because it aggregates optional
capabilities under an "any passes" rule, and the demographic, terminology, and
admin APIs are evidenced.

## Running the suite yourself

The suite runs against a real, composed server — the same container image and
stack a deployment uses — so the wire under test is always the production
artefact, never a re-wired in-process stub. From a checkout with Docker
available:

```bash
bash scripts/conformance.sh
```

The script builds and starts the server and its PostgreSQL 18 database with
Docker Compose, runs the full catalogue in both formats against it, writes the
artefacts to `docs/conformance/`, and tears the stack down. Exit code `0`
means every executed case passed; `1` means there were failures (the report is
still written so you can inspect them); `2` means the runner or the system
under test could not start.

To run against an already-deployed server instead of the composed stack, the
runner accepts a base URL and credentials:

```text
conformance run --base-url https://your-host/ehrbase/rest/openehr/v1 \
                --auth basic:user:password \
                --format both
```

You can also narrow a run to one area with `--filter`, or regenerate the
artefacts from a previous run's machine record without re-running
(`conformance report --from results.json`).

## Reading the artefacts

A run writes one machine record and three human-readable documents to
`docs/conformance/`. Each has a distinct job.

### The machine record

`results.json` is the single source of truth for a run: one entry per case
with its id, title, capability, the profiles it feeds, the formats exercised,
the outcome, the number of data sets, and its duration, alongside the identity
of the system under test and the specification versions. Every other artefact
is generated from this file — nothing downstream is hand-edited.

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

`CONFORMANCE_CERTIFICATE.md` is a self-assessed certificate whose structure
follows the openEHR conformance certificate template: the system under test,
the scope of test, and a per-capability profile report showing which
capabilities are required in each profile and whether each passed. This is the
document to hand to a procurement or evaluation reviewer who wants the
capability-by-capability picture.

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
