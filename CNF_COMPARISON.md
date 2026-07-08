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
| **Functional** | CORE / STANDARD / OPTIONS | not yet CORE (archetype-validation findings — see triage bucket 1) | _TBD_ |
| **Non-functional** | BASIC-SEC / BASIC-PRIV | BASIC-SEC (Basic + OAuth2/OIDC auth; 401/403) | _TBD_ |
| **External data format** | Canonical JSON / Canonical XML | Canonical JSON (run under JSON; XML partial) | _TBD_ |

A profile (`docs/specs/openehr/CNF/docs/profiles`) is achieved only if **all** its
capabilities pass. CORE and STANDARD are the meaningful certification targets;
OPTIONS is a catch-all for any optional capability passed.

---

## Per-chapter results

Numbers are `passed / implemented / total` per the schedule chapter. Fill from
`docs/conformance/<edition>/RESULTS.md`.

Passed / total, from a self-host run of ehrbase-rs on 2026-07-08
(`docs/conformance/RESULTS.md`). The EHRbase-Java column is filled by
`docker/conformance/run.sh java`.

| Chapter (Service Model area) | Profile | ehrbase-rs | EHRbase Java |
|---|---|---|---|
| master04 — DEFINITION (ADL 1.4 / OPT) | CORE | 12 / 15 | _/_ / 15 |
| master05 — DEFINITION (stored query) | STANDARD | 3 / 7 | _/_ / 7 |
| master06 — EHR | CORE | 21 / 21 ✅ | _/_ / 21 |
| master07 — COMPOSITION | CORE | 28 / 30 | _/_ / 31 |
| master08 — CONTRIBUTION | CORE | 24 / 31 | _/_ / 31 |
| master09 — DIRECTORY (FOLDER) | STANDARD | 36 / 37 | _/_ / 37 |
| master10 — DEMOGRAPHIC (`DEMO-*`, runner-defined) | OPTIONS | 18 / 24 | _/_ |
| master11 — QUERY (AQL) | STANDARD | 3 / ~12 | _/_ |
| master12 — ADMIN (`ADMIN-*`, runner-defined) | OPTIONS | 6 / 6 ✅ | _/_ |
| master13 — MESSAGING | OPTIONS | n/a (no API) | _/_ |
| master15 — content: COMPOSITION | CORE | 3 / 12 | _/_ / 12 |
| master16 — content: ENTRY | CORE | 13 / 26 | _/_ / 26 |
| master17.x — content: DATA_VALUE | CORE | 31 / 80 | _/_ / 81 |
| **SIGN-* — Version signing** (runner-defined) | STANDARD | (see SIGN suite) | _/_ |
| **Total** | | **202 / 322** | **_/_ / 322** |

## Findings triage (ehrbase-rs, 105 findings)

Grouped by root cause — the fix path toward a CORE/STANDARD rating. The single
biggest bucket (71 of 105) is one root cause in the composition validator.

| # | Root cause | Findings | Fix location | Blocks |
|--:|---|--:|---|---|
| 1 | **Archetype value/cardinality constraints not enforced** (content `CONT-*`): cardinality lower=1 / upper bounds, C_INTEGER/C_REAL lists, temporal ranges/patterns, DV_INTERVAL bounds+type, C_CODE_PHRASE code lists, subtype narrowing | **71** | `openehr-flat` (`webtemplate/builder.rs` `requires_cardinality`, `validation/leaf.rs`, subtype/interval) | CORE (Archetype Validation) |
| 2 | **AQL feature gaps**: `TIMEWINDOW` not parsed; RESULT_SET column `path` metadata missing | ~9 | `openehr-query` parser + `ehrbase::aql` result-set | STANDARD (AQL) |
| 3 | **Missing REST realizations**: `delete_opt` (master04 ×4), `list_contributions` (master08 ×5), `get_versioned_directory` (×1), `list_queries` (master05 ×2) — SM ops with no ITS-REST verb on our surface | ~12 | add the endpoints to `ehrbase-rest` / service, or record as non-ITS per guide | STANDARD/OPTIONS |
| 4 | **Service validation leniency**: `create_composition-same_opt_twice`, `update_composition-wrong_template`, `commit_contribution-*_invalid_change_type`, `-fail_create_existing_directory`, invalid EHR_STATUS partially accepted | ~5 | `ehrbase` service layer (template match, change-type, duplicate guard) | CORE |
| 5 | **Demographic CRUD** (`DEMO-*` ×6): a few PARTY lifecycle rows | 6 | `ehrbase` demographic service / `ehrbase-rest` demographic dispatch | OPTIONS |

**Rating today:** not yet CORE — the archetype-validation findings (bucket 1) are
the gate. Closing bucket 1 + bucket 4 reaches **CORE**; adding buckets 2–3 reaches
**STANDARD**.

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
