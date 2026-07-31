# Pinned Version Matrix

The single source of truth for every version pin in this project. If this file
and a config file (`Cargo.toml`, `rust-toolchain.toml`, CI workflows) disagree,
fix the drift — do not let either silently win. For third-party Rust crate
versions specifically, the root `Cargo.toml` `[workspace.dependencies]` table
is authoritative (see "Rust dependency pins" below); this file records the
platform, language, database, and openEHR specification pins.

## Product and crate versioning

The **product** version (the workspace `version` in the root `Cargo.toml`,
inherited by the `ferroehr-*` application crates, the tools, and the codegen
tooling) follows its own SemVer line, starting at **3.0.0**
(2026-07-11 — the fork's inherited upstream tags/releases were removed; this
project releases as the successor of the EHRbase 2.x line). Releases are
changelog-driven (`CHANGELOG.md`, Keep a Changelog 1.1.0) and published as
official releases (owner sign-off 2026-07-31); only an explicitly suffixed
tag (`vX.Y.Z-rc1`, ...) publishes as a pre-release.

The `openehr-*` **spec crates** are versioned by the openEHR specification
they implement (the pins below): `openehr-base` 1.3.0, `openehr-rm` 1.2.0,
`openehr-am` 2.4.0, `openehr-adl` 2.4.0, `openehr-term` 3.1.0, `openehr-lang` 1.0.0,
`openehr-query` 1.1.0, `openehr-its` 1.1.0. They bump only on a spec-pin
bump, never with the product version.

## Language and runtime

| Item | Pin |
|---|---|
| Rust toolchain | stable 1.96 (1.96.1) |
| MSRV | 1.96 |
| Edition | 2024 |
| Cargo resolver | v3 |

Rationale: the deliverable is a binary/service, not a published library, so
MSRV can track current stable rather than trailing it. A crate later split out
for standalone publication may relax to 1.94/1.95 at that time.

MSRV bump policy (the documented policy the Cargo book asks for — an MSRV
change is "possibly-breaking", semver class `env-new-rust`,
https://doc.rust-lang.org/cargo/reference/semver.html#env-new-rust): the
workspace `rust-version` and `rust-toolchain.toml` bump together, in their
own PR, with a `CHANGELOG.md` `### Changed` entry; the declared MSRV is
verified in CI by the `msrv` job (`cargo hack check --rust-version` — the
book's named tool), not just by `clippy::incompatible_msrv`. Consumers can
escape with `cargo build --ignore-rust-version`.

## Database

| Item | Pin |
|---|---|
| PostgreSQL | 18, target 18.4 or newer (CI runs `postgres:18.4`) |
| Required extensions | `uuid-ossp`, `pgcrypto`, `pg_trgm` |

PostgreSQL is the only component in the stack with a meaningful version delta
(EHRbase Java targets PG 15/16; we target PG 18). PG 18 brings asynchronous I/O,
native `uuidv7()`, B-tree skip scan, temporal `PRIMARY KEY`/`UNIQUE`/`FOREIGN
KEY`, `RETURNING OLD/NEW`, virtual generated columns, self-join elimination, and
OR-to-`ANY` planning, plus `JSON_TABLE()` + the SQL/JSON functions inherited
from PG 17. **See `docs/postgres-features.md`** for the full PG 17+18 feature
delta over PG 16 and where each app phase (P09/P12/P16/P11/P20) uses it —
"the best possible system" means exploiting these. Minor releases (18.1–18.4,
17.x) are bugfix/security only (no new features); pin the latest patch (18.4,
2026-05-14). Managed providers may lag on `io_uring`; still target PG 18 and let
the AIO benefit follow.

## openEHR specification matrix

There is no umbrella "openEHR Release" version; each component versions
independently. **The spec crates are generated from the vendored BMM
meta-model, so the version pins below are load-bearing** — they are the
actual codegen input, not just documentation. Pins bumped 2026-07-03 (the
versions available as clean `*.bmm.json`); the release-ladder columns were
verified against specifications.openehr.org on 2026-07-20.

**Pin honesty:** several pins are development-generation snapshots AHEAD of
the latest official release — the "Our pin" column says which. That is safe
by openEHR's own release strategy (see "Spec version policy" below): within
a major line every release is a compatible superset, so a newer-generation
pin accepts every valid older-minor instance.

| Component | Our pin | Latest official release | Upstream WIP | Notes |
|---|---|---|---|---|
| BASE (Foundation + Base Types) | **1.3.0** (pre-release generation; text + BMM refreshed 2026-07-25 @ `e4879576` — SPECBASE-48 invariants, SPECAM-82 CODE_PHRASE move, SPECPR-426/386/460) | 1.2.0 (09-Apr-2021) | 1.3.0 | generated → `openehr-base` (foundation + base types; `openehr-foundation` folded in) |
| RM (Reference Model) | **1.2.0** (development generation; text + BMM refreshed 2026-07-26 @ `66d3ac45` — asterisk-entity normalization only, zero generated-output change) | 1.1.0 (29-Sep-2020) | dev | generated → `openehr-rm` |
| AM (Archetype Model) | **1.4 + 2.4.0** (1.4 released; 2.4.0 WIP generation) | 2.3.0 (20-Mar-2024) | 2.4.0, **3.0.0** | generated → `openehr-am`, both majors side by side as `am14` (ADL 1.4) + `am24` (ADL 2) — the spec-mandated dual-generation case |
| QUERY (AQL) | 1.1.0 (= the release) | 1.1.0 (14-May-2021) | 1.2.0 | `openehr-query`; grammar-driven (AqlLexer/Parser `.g4`), not BMM |
| LANG (BMM / ODIN / EL) | master snapshot beyond 1.0.0 (development toward 1.1.0) | 1.0.0 (11-May-2020) | dev | `openehr-lang` — the ODIN + BMM reader that feeds codegen; the crate carries 1.0.0 as its spec version |
| TERM (Terminology) | **3.1.0** (WIP generation) | 3.0.0 (26-Jun-2023) | 3.1.0 | `openehr-term` — **hand-written** (BMM has only interface classes; bundle/assets/logic are not derivable) |
| ITS-XML (XSDs) | **both published lineages vendored**: tag `Release-1.0.2v2` @ `f7a93777` (namespace `…/v1`, the STABLE bundle) + tag `Release-2.0.0v2` @ `de8b37ba` (namespace `…/v2`, TRIAL upstream) | 2.0.0 TRIAL (26-Apr-2021) | 2.1.0 | canonical XML in `openehr-its`, one generated codec for both (the lineages differ only in the root `xmlns`); bundles at `crates/openehr-its/schemas/xml/`. **Served wire = v1 by default, v2 on request** (owner ruling 2026-07-28): a client selects the lineage with the `version` media-type parameter on `application/xml` — our own extension, no openEHR spec governs namespace selection on the REST wire. OPT 1.4 template XML is always v1. |
| ITS-REST (REST API) | **Release-1.1.0** @ `24058992d` | 1.1.0 (19-Jul-2026) | 1.2.0 | policy: single version, always the latest released; spec text at `docs/specs/openehr/ITS-REST/` and the OAS at `crates/openehr-its/vendor/rest-oas/` are the **same commit** (tag Release-1.1.0). Per-API lifecycle within the release: Overview/System/EHR/Query/Definition/Formats **STABLE**; Demographic/Admin/SMART **DEVELOPMENT** (the OAS bundle artifacts are marked TRIAL) — all 7 API groups vendored |
| ITS-JSON (JSON Schemas) | development @ `5acae056` | (1.0.0 itself still WIP) | dev | validation oracle for the fidelity gate; `openehr_rm_1.1.0_all.json` vendored at `crates/openehr-its/schemas/json/` |
| ITS-BMM (BMM meta-model, JSON) | per-component (see above) | per-schema | — | **the codegen input**; vendored `*.bmm.json` at `tools/openehr-codegen/vendor/bmm/` with provenance. Exception: the BASE 1.3.0 json is taken from `specifications-BASE` @ `e4879576` (2026-07-25) pending ITS-BMM republication — see that PROVENANCE.md |
| SM (Service Model) | master snapshot | 1.1.0 | dev | vendored spec text only (the service layer's design authority) |
| CNF (Conformance) | master snapshot | 1.0.0 | dev | vendored spec text + CNF test schedule (the conformance oracle) |

**Spec text vendored in-repo:** the normative documentation for every
component above — plus SM (platform service model / SDT) and **CNF (the
conformance guide + Platform Conformance Test Schedule + Robot suite)** — is
vendored at `docs/specs/openehr/` by `scripts/vendor-spec-docs.sh`, pinned to
these same versions (exact commits in each component's `PROVENANCE.md` and in
the script). It is the read/conformance oracle; codegen still consumes only
`tools/openehr-codegen/vendor/**` and `crates/openehr-its/schemas/**`.

## Spec version policy (owner rulings 2026-07-20)

Grounded in the official openEHR release strategy
(specifications.openehr.org/governance/release_strategy, read 2026-07-20):
patch = "error corrections and minor additions that do not change the
semantics"; minor = "significant additions that do not change the semantics
of the existing part of the release"; major = "changes to the semantics or
large changes", incompatible and "most likely requiring software upgrade and
possibly data migration".

- **Single pin per component.** Within a major line every release is a
  compatible superset, so the newest-generation pin accepts every valid
  older-minor instance — no version negotiation, no second generation.
  Proven in practice: the fidelity gate validates our canonical wire against
  the 1.1.0-era ITS-JSON schema while the types are RM 1.2.0-generation, and
  stock-EHRbase's RM 1.1.0-era wire round-trips the corpus.
- **Dual generations exist ONLY across major boundaries, decided per
  component when a major actually releases.** The one live case is AM: the
  BASE Architecture Overview (`master05-package_structure.adoc`) mandates
  ADL 1.4 and ADL 2 "maintained side by side", and `openehr-am` ships both
  as `am14`/`am24`. A future major (AM 3.0.0 is in WIP upstream) gets the
  same treatment only if the ecosystem runs both generations; otherwise
  cutover.
- **ITS-REST is single-version: always the latest RELEASED API** — no legacy
  REST surface. The 1.1.0 adoption is tracked on issue #178; future releases
  are detected automatically (the spec-update/release watcher workflows).
- **ITS-XML is the one place a lineage is negotiated, and it is not a second
  generation** (owner ruling 2026-07-28). The `Release-2.0.0` restructure
  changed the schemas' target namespace and nothing else about the document
  shape, and upstream marks 2.0.0 TRIAL while directing stable consumers to
  `Release-1.0.2`. So there is exactly ONE generated codec: the two bundles
  merge into one emission closure and the lineage is a serialize-time choice
  of root `xmlns`. The CDR serves v1 by default (the released-STABLE bundle)
  and v2 when a request asks for it via the `version` media-type parameter on
  `application/xml`. That parameter is our own extension — no openEHR spec
  governs namespace selection on the REST wire — and it is not the
  dual-generation exception the AM rule above describes, because no second
  model, crate module, or impl set exists.
- The acceptance instrument is the openEHR CNF conformance schedule, **not**
  an EHRbase parity harness; EHRbase is prior art. Stock EHRbase/`archie`
  emits an RM 1.1.0-era wire format — compatible under this policy, and the
  fidelity gate is where any divergence surfaces.

## EHRbase reference point

This is a fork; its import point is recorded so provenance is unambiguous:

| Item | Value |
|---|---|
| Original reasoning authored against | EHRbase v2.31.0 |
| Tree actually imported at (Phase 0) | EHRbase v2.33.0 |
| `reference/v1` git ref | v0.32.0 (last pre-v2 tag) |

The original design reasoning was authored against v2.31.0. By the time this
fork's Phase 0 reorganization ran, upstream had advanced to v2.33.0, and that
is the tree actually imported into the Cargo workspace. (With the greenfield storage pivot the
in-tree EHRbase Java was later removed and behaviour-parity was retired for
openEHR CNF conformance; EHRbase v2.33.0 remains the *prior-art* reference, not
a parity oracle.)

`reference/v1` is a read-only git reference pinned at v0.32.0, the last tag
before the v1→v2 architectural break. It is consulted only during the Stage 2
enterprise-feature archaeology and restoration work and is never merged into the
working tree during Stage 1.

## Rust dependency pins

The authoritative, fully-pinned third-party Rust dependency set lives in the
root `Cargo.toml` under `[workspace.dependencies]`. `CLAUDE.md` carries a
categorized narrative summary for orientation. On any discrepancy between this
file, `CLAUDE.md`, and the manifest, **the manifest wins**. Add a dependency to
a crate with `dep.workspace = true`; do not hand-pin a version at the crate
level.
