# openEHR CNF Conformance — ehrbase-rs vs EHRbase (Java)

A head-to-head of two openEHR CDR implementations under the **same** openEHR
Platform Conformance Test Schedule (CNF), driven by the **same** server-agnostic
test framework in this repo (`crates/ehrbase-conformance`).

- **SUT A — ehrbase-rs** (this project): pure-Rust CDR, PostgreSQL 18.
- **SUT B — EHRbase Java** (reference implementation): `ehrbase/ehrbase:2.33.0`,
  PostgreSQL 16.

> Results tables below are **placeholders to be filled in from a real run** (see
> "How to run"). Each `run.sh` invocation regenerates `docs/conformance/<edition>/`
> (`RESULTS.md`, `results.json`, `badge.json`); copy the per-chapter numbers here.

---

## Why this is a fair comparison

The openEHR Conformance Guide (`docs/specs/openehr/CNF/docs/guide`) defines
conformance as the relationship between a **deployed system (SUT)** and the
technology-specific specifications (ITS-REST + canonical JSON/XML). Our framework
implements that methodology exactly and is **server-agnostic**: it drives any
ITS-REST server over HTTP via `--base-url`. So both editions are exercised by an
**identical** suite (the same 322-case schedule + our runner-defined `DEMO-*` /
`ADMIN-* / SIGN-*` cases), through an **identical** Basic-auth credential path,
with the **only** variable being the server under test.

Caveats to record with any result (they are environmental, not methodological):

- **DB version:** ehrbase-rs targets PG 18; EHRbase Java runs PG 16 (its
  supported line). Functional conformance is DB-version-independent.
- **RM version on the wire:** ehrbase-rs emits RM 1.2.0; stock EHRbase/`archie`
  emits an RM 1.1.0-era shape. The CNF payload fixtures are RM-1.1.0-era, so a
  handful of canonical-shape cases may diverge for reasons unrelated to logic.
- **Placeholder chapters:** master10 (demographic), master12 (admin), master13
  (messaging) ship **no** concrete cases upstream (only `aaaa`/`bbbb` stubs); our
  `DEMO-*` / `ADMIN-*` cases are runner-defined against the ITS-REST + Service
  Model spec surface, and are applied identically to both SUTs.

---

## How to run

Both editions run fully in Docker (app image + its PostgreSQL), driven over the
network — the "official", deployable-artefact form of conformance assessment.

```bash
# EHRbase Java (reference)  → docs/conformance/java/
docker/conformance/run.sh java

# ehrbase-rs (this project) → docs/conformance/rs/
docker/conformance/run.sh rs
```

Each script brings the stack up on a fixed port (Java 8091, rs 8090), waits for
the server, runs the CNF suite via `--base-url` with Basic auth
(`ehrbase`/`ehrbase`, admin `ehrbase-admin`/`ehrbase` for Java), writes the
report set, and tears the stack down.

To run the Rust edition **without Docker** (fastest inner loop — in-process app +
a PostgreSQL testcontainer):

```bash
cargo run -p ehrbase-conformance --features self-host --bin conformance -- \
  run --self-host --out docs/conformance
```

Images are pinned by tag in `docker/conformance/docker-compose.yml`; record the
resolved digests in the environment block of each result when publishing.

---

## Certificate summary (openEHR CNF)

Per `docs/specs/openehr/CNF/docs/certificate`, a certificate rates the SUT in the
Functional and Non-functional dimensions plus external data formats. Fill from the
run once available.

| Dimension | Level(s) | ehrbase-rs | EHRbase Java |
|---|---|---|---|
| **Functional** | CORE / STANDARD / OPTIONS | _TBD_ | _TBD_ |
| **Non-functional** | BASIC-SEC / BASIC-PRIV | _TBD_ | _TBD_ |
| **External data format** | Canonical JSON / Canonical XML | _TBD_ | _TBD_ |

A profile (`docs/specs/openehr/CNF/docs/profiles`) is achieved only if **all** its
capabilities pass. CORE and STANDARD are the meaningful certification targets;
OPTIONS is a catch-all for any optional capability passed.

---

## Per-chapter results

Numbers are `passed / implemented / total` per the schedule chapter. Fill from
`docs/conformance/<edition>/RESULTS.md`.

| Chapter (Service Model area) | Profile | ehrbase-rs | EHRbase Java |
|---|---|---|---|
| master04 — DEFINITION (ADL 1.4 / OPT) | CORE | _/_ / 15 | _/_ / 15 |
| master05 — DEFINITION (stored query) | STANDARD | _/_ / 7 | _/_ / 7 |
| master06 — EHR | CORE | _/_ / 21 | _/_ / 21 |
| master07 — COMPOSITION | CORE | _/_ / 31 | _/_ / 31 |
| master08 — CONTRIBUTION | CORE | _/_ / 31 | _/_ / 31 |
| master09 — DIRECTORY (FOLDER) | STANDARD | _/_ / 37 | _/_ / 37 |
| master10 — DEMOGRAPHIC (`DEMO-*`, runner-defined) | OPTIONS | _/_ | _/_ |
| master11 — QUERY (AQL) | STANDARD | _/_ / 5 | _/_ / 5 |
| master12 — ADMIN (`ADMIN-*`, runner-defined) | OPTIONS | _/_ | _/_ |
| master13 — MESSAGING | OPTIONS | n/a (no API) | _/_ |
| master15 — content: COMPOSITION | CORE | _/_ / 12 | _/_ / 12 |
| master16 — content: ENTRY | CORE | _/_ / 26 | _/_ / 26 |
| master17.x — content: DATA_VALUE | CORE | _/_ / 81 | _/_ / 81 |
| **SIGN-* — Version signing** (runner-defined) | STANDARD | _/_ | _/_ |
| **Total** | | **_/_ / 322** | **_/_ / 322** |

---

## Notable divergences (hand-written)

Record here the specific cases where the two editions differ, and why — this is
the analytical payload of the comparison (spec citation + which SUT is
spec-correct). Examples of the kinds of finding to capture:

- _(TBD)_ archetype value-constraint enforcement (cardinality bounds, C_* value
  ranges/lists, temporal ranges, DV_INTERVAL bounds) …
- _(TBD)_ AQL feature coverage (e.g. `TIMEWINDOW`, RESULT_SET column metadata) …
- _(TBD)_ versioning / `ALL_VERSIONS` support …
- _(TBD)_ canonical JSON/XML shape parity …

> Reminder (ADR-008): where a divergence is between a SUT and the **openEHR
> specification**, the spec is the authority — cite the spec/CNF section, name
> which SUT is conformant, and (for ehrbase-rs) open a finding in
> `docs/conformance/COVERAGE_GAPS.md`.
