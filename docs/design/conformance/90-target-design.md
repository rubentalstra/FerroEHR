# W-10 target design — the rewritten conformance framework

Orchestrator-owned architecture register (2026-07-13; finalized after the
area registers 01–13 landed). The framework's mission (owner ruling): **the
best possible openEHR CNF testing framework, able to certify not just our
server but ANY openEHR CDR** — driven by URL, honest about spec editions,
emitting the CNF result artefacts.

Oracle grounding (read end-to-end at session start):

- `CNF/docs/guide/master03-overview.adoc` §Product Scope + §What Conformance
  Claims are Possible — SUT model; deployed-system testing; product claims
  inferred from representative deployment.
- `CNF/docs/guide/master04-framework.adoc` §From Specifications to Runnable
  Tests — the derivation square: SM operation → REST binding → abstract test
  case → executable runner. Every case therefore carries BOTH the schedule
  trace (`I_*.op-case`) and the ITS-REST binding it concretizes.
- `CNF/docs/platform_test_schedule/master03-overview.adoc` §API Conformance
  Test Design — a *test* = case × data set; NOTE: "The supported RM
  version(s) by the SUT should be stated in the Conformance Statement …
  minimum required version is RM 1.0.2" — the normative basis for the
  version ladder (§4 below).
- `CNF/docs/profiles/master03-profiles.adoc` — capability × profile matrix;
  CORE/STANDARD all-of, OPTIONS any-of; non-functional Signing (STANDARD),
  Anonymous EHRs (CORE); external data formats XML + JSON.
- `CNF/docs/certificate/master03-certificate.adoc` — Statement/Certificate
  shapes (SUT block, scope, detailed per-conformance-point report, profile
  report).

Carried law (unchanged): ECC identity (own numbering `ECC-<AREA>-<NNN>`,
tsv allocation, numbers never reused; own taxonomy; no Robot/legacy mapping
machinery); instrument honesty (identity from provenance; adjudication
register for spec-contradicting cases; every coverage bound logged);
pure-Rust reqwest runner; Docker-composed SUTs; X1 fairness constitution
(absorbed: measured-only, upstream fairness triage, extensions are N/A not
failures, no certification claims for foreign runs, "where EHRbase wins"
stated plainly).

---

## 1. Crate layout (`tools/conformance`, fresh)

```
src/
  lib.rs
  bin/conformance.rs        # CLI: run | report | compare | catalog
  model/                    # identity + claim model (pure data)
    catalog.rs              # ECC tsv allocation (carried law, re-authored)
    case.rs                 # CaseMeta: id/title/area/capability/profiles/
                            #   formats/citation/schedule_ref/binding
    profile.rs              # profiles master03 matrix; CORE/STANDARD all-of,
                            #   OPTIONS any-of per capability
    versions.rs             # SpecVersions DERIVED from vendored provenance
    adjudication.rs         # sanctioned-skip register (per SUT class)
    fairness.rs             # foreign-SUT fairness register (X1 absorption)
  wire/                     # THE spec-grade client layer (W-3f lesson):
    headers.rs              #   ETag (weak W/"…" + deprecated bare), Location,
                            #   Last-Modified, openEHR-VERSION/-AUDIT_DETAILS
    ids.rs                  #   version_uid / object_uid / contribution_uid /
                            #   ehr_id extraction — the ONLY place wire ids
                            #   are parsed; suites never scrape ad hoc
    negotiate.rs            #   Accept/Content-Type/Prefer construction
  edition/                  # the version ladder (§4)
    mod.rs                  #   Edition enum + EditionLadder<T> assertion forms
    probe.rs                #   per-SUT edition discovery (OPTIONS /, probes)
  sut/                      # multi-SUT (§3)
    descriptor.rs           #   SutDescriptor: name, base URLs, auth, edition
                            #   policy, capability hints — config, not code
    builtin.rs              #   ehrbase-rs + ehrbase-java descriptors
    boot.rs                 #   compose adapters (boot/await-healthy/teardown)
  engine/                   # execution
    transport.rs            #   reqwest Transport (carried shape)
    harness.rs              #   HttpRequest/Response, RunContext, CaseRun
    assert.rs               #   status/header/payload assertions (jsonlib modes)
    registry.rs             #   suite registration, coverage guard hooks
    run.rs                  #   scheduling, per-format execution, skip logic
  testdata/                 # owned data sets (§5)
    fixtures.rs             #   loader over testdata/fixtures/ ONLY
    author.rs               #   programmatic OPT/composition authoring
  ts/                       # terminology fixture server (wiremock) — carried
  suites/                   # the case universe, fresh-authored per registers
    <area>.rs …             #   one module per schedule chapter + crosscutting
  reporting/
    results.rs              #   per-SUT results.json (+ run stamps, digests)
    report.rs               #   CONFORMANCE_REPORT.md per SUT
    statement.rs            #   Conformance Statement (incl. RM versions per
                            #   master03 NOTE + edition findings)
    certificate.rs          #   Certificate — every SUT (self-identifying
                            #   framework assessment, never official openEHR)
    compare.rs              #   cross-SUT comparison matrix (fairness-gated)
    badges.rs
```

Files ≤ ~700 lines; suites split per chapter as today but authored fresh
from the registers.

## 2. Case model

`CaseMeta` (re-authored) carries, per case:

- `id` (slug) + ECC number via the catalogue (carried allocation law).
- `schedule_ref: Option<&str>` — the `I_*.<operation>-<id>` schedule trace +
  chapter locus. REQUIRED for every case whose spine row exists (registers
  01–13 enumerate them); `None` only for ECC-original/extension cases.
- `binding: Binding` — the ITS-REST concretization (`method + path template
  + expected statuses`) or `Binding::NoRestBinding` (the D2 lesson: an SM op
  with no REST binding is a first-class, machine-readable fact → the case is
  a documented skip or an extension probe, never a fabricated URL).
- `capability` + `profiles` per profiles master03 (full OPTIONS surface
  modeled; CORE evidencing rules explicit — Versioning/AnonymousEhrs/
  Adl14ArchetypeProvisioning each have tagged cases or a documented
  evidencing statement).
- `formats` (JSON/XML — profiles master03 §Other Non-Functional).
- `citation` — CNF schedule file + §, plus the ITS-REST/RM sections the
  assertion enforces.
- `edition_sensitivity` — which assertions in the case are
  normative-invariant vs edition-specific (§4).

## 3. Multi-SUT (owner rulings, session start)

Three target classes, ONE case universe (never per-SUT case forks):

1. **ehrbase-rs** — default; compose boot via the root stack
   (`docker-compose.yml`; the retired `docker/conformance/` harness was
   removed at D1), `scripts/conformance.sh` stays the entry point.
2. **EHRbase (Java, upstream)** — official image compose adapter; its
   results are DATA (comparison input), never a gate; fairness register
   applies (extension areas → `not-applicable(extension)`, never failures).
3. **Bring-your-own-endpooint** — `--sut-url <base>` (+ `--sut-name`,
   `--auth`, `--admin-auth`, `--admin-base-url`, `--edition auto|…`): anyone
   points the runner at a deployed CDR and gets the full spec-cited report +
   Statement. No boot adapter; capability discovery via `OPTIONS /` +
   graceful `not-evidenced` reporting where the SUT lacks surface.

`SutDescriptor` is config (TOML/CLI), not code: adding a target is a config
entry. Boot adapters exist only for the two first-class targets.

## 4. The edition/version ladder (owner ruling, mid-session)

Problem (W-3f evidence + VERSIONS.md divergence note): our server speaks the
ITS-REST *development* edition (weak `W/"…"` ETags, lowercase committal
headers, RM 1.2.0 wire); upstream EHRbase speaks Release-1.0.3-era forms
(bare ETags historically, RM 1.1.0 wire). A single-edition instrument fails
foreign SUTs on edition deltas, not defects — dishonest comparison.

Design:

- Every assertion is split into its **normative core** (what every edition
  mandates — e.g. "an ETag identifying the created VERSION is returned") and
  its **edition forms** (the concrete wire shape per edition), ordered
  newest→oldest: `development@e8a093e` → `Release-1.0.3` (→ older only if a
  register proves a real delta exists).
- The runner tries the **highest form first and steps down** the ladder;
  the satisfied level is recorded per case (`edition_level` in
  results.json). A lower-level match is an **edition finding** (reported;
  feeds the per-SUT edition profile in the Statement — master03-overview
  NOTE mandates stating supported RM versions), never a silent pass.
- A failure is only "no supported form satisfies the normative core".
- Per-SUT edition policy: `auto` (ladder, default for BYO) or pinned (our
  CI runs pin `development` so drift in OUR server is still caught — the
  ladder must not mask a regression in ehrbase-rs; zero-drift gate compares
  at pinned level).
- The same mechanism covers RM wire versions (1.2.0 vs 1.1.0 payload
  shapes) where registers identify concrete deltas; the fixture adaptation
  layer (today's `testdata/fixtures.rs` RM-version bridging) becomes an
  explicit rung of this ladder rather than an ad-hoc normalizer.

## 5. Data-set strategy (the register-80 decision)

ECC law says *generated data sets, never a Robot mapping*. Ruling:

- The Robot corpus under `CNF/tests/` is **raw material only** (it is the
  schedule's own referenced data): a fixture may be *derived* from it, but
  every fixture in `testdata/fixtures/` carries a register entry
  (`REGISTER.md`) stating `owned` (authored/generated by us) or `derived`
  (corpus source path + our adaptations, cited).
- The runner loads fixtures ONLY from `testdata/fixtures/` — no path
  constant into `docs/specs/openehr/CNF/tests/` (deletes today's
  `CORPUS_ROOT` seam). A build-time check (`cnf coverage guard`) fails on
  any fixture without a register entry.
- Programmatic authoring (`testdata/author.rs`, carrying the master15–17
  "archetypes should be generated" approach) is the preferred source for
  constraint variants; corpus derivation is the fallback for large clinical
  payloads.

## 6. Outputs (per run)

Per SUT: `results.json` (SUT identity, image digest where composed, run
stamps, spec identities from provenance, per-case verdict + edition_level +
data-set counts), `CONFORMANCE_REPORT.md`, badges, **Conformance Statement**
(capability scope, RM/edition findings per master03 NOTE, adjudicated
skips), **Certificate** (certificate/master03 table shapes — emitted for
EVERY SUT, owner ruling 2026-07-13: the framework certifies any openEHR
CDR; the artefact itself states assessor = framework self-assessment,
never an official openEHR certification, and scopes the claim to the
applicable capabilities incl. fairness N/A adjudications).
Cross-SUT: `COMPARISON.md` — per capability, fairness-gated (X1 rules 1–10),
extension rows marked N/A, every cell stamped (date, versions, digests).

## 7. Honesty invariants (carried B5, verified in rewrite)

1. Spec identity derived from provenance files, never hand-asserted.
2. A case contradicting the vendored spec text is adjudicated (spec-cited)
   in `adjudications/`; the server is never bent to a wrong case.
3. Every coverage bound (chapter TBD stubs, no-REST-binding ops, skipped
   formats) is logged in the report — silent truncation is a defect.
4. Profile verdicts machine-computed only (CORE/STANDARD all-of, OPTIONS
   any-of per capability).
5. Foreign runs: fairness triage before publication; the Certificate is
   available to every operator but always self-identifies as a framework
   assessment (no official openEHR certification exists), and OUR published
   comparison never makes certification claims on upstream's behalf.

## 8. Cross-register rulings (orchestrator, after registers 01–13 landed)

Consolidated from the area registers' G-rows; these bind the rewrite:

1. **`schedule_ref` is threaded on every case** (register 10's pattern is
   the model): the schedule id + chapter locus where a spine row exists;
   `ecc-original(<reason>)` markers for stub-derived cases (SQR/QRY over
   master05/11 stubs; DEM/ADM/MSG over master10/12/13 stubs) — a
   stub-derived case is never presented as schedule-conformant.
2. **`Binding` is first-class**: the ITS-REST concretization or
   `NoRestBinding` (TPL delete, bare SQR list, CTB list_contributions,
   Messaging, 6 native-only Admin ops) → machine-readable skip-with-reason
   or edition-ladder probe, never a fabricated URL, never a booked failure.
3. **The wire layer owns ALL id/header extraction** — replaces
   `support::version_uid`, `contribution.rs::contribution_uid`/
   `version_uid_at`, and every inline scrape; silent fallbacks die (the
   `directory.rs:544 unwrap_or_else(|_| v1.clone())` class of bug).
4. **SUT-specific wire facts come from the SutDescriptor, never literals**:
   the `::conformance::` creating-system-id If-Match constructions,
   `template_id` format (master04 §Test Environment says server-specific),
   admin mount point. Edition-specific forms (ladder rungs): weak `W/"…"`
   vs bare ETag; 400-vs-422 reject codes; RM 1.2.0 vs 1.1.0 payload shapes
   (DV_SCALE needs RM ≥ 1.1.0); `openehr::523` lifecycle literal
   (terminology-version-sensitive); RESULT_SET `_schema_version`;
   timestamp precision forms.
5. **Postcondition depth is restored** (the schedule's own mandate):
   retrieved == committed content checks on every `get_*` (register 04
   G-1, via the jsonlib compare modes); `update` → change_type MODIFY and
   `delete` → lifecycle 523 verified (04 G-2); directory at-time cases
   actually drive between-version instants (06 G-1); the D.2
   `full_ehr_status` precondition actually set (05).
6. **Real coverage gaps to author** (new ECC numbers): `has_composition`
   positive; the 6 `I_PARTY_RELATIONSHIP` ops + `get_party_at_time`;
   DV_INTERVAL per-variant constraint depth (27 cases); the OBS/EVENT
   state/protocol authoring dimension; SEC cases stay generic for foreign
   SUTs.
7. **Coverage bounds are reported per case**: `schedule_rows` vs
   `driven_variants` (register 13 quantified ≈1,130 → ≈160; the report
   prints the ratio per case — logged, never silent) — widening the driven
   set is data (`testdata/author.rs` sweeps), not new case ids.
8. **The golden normalizer + dialect adjudications externalize** to the
   committed adjudication/golden registers (07 G-3); a golden is never
   edited.
9. **Fairness register seeds** (foreign SUTs): DEM wire, SIG, bundle-TS +
   `/terminology` = `extension → N/A`; SEC + the normative-core assertions
   of every other area = live.

## 9. CI bindings

- `cargo nextest run -p conformance` — the runner's own tests (wire layer,
  ladder, catalog, registry, reporting; wiremock fixtures).
- `cnf coverage guard` — every registered case has a catalogue line, a
  schedule_ref-or-documented-none, a citation, and every fixture a register
  entry; register↔suite drift fails.
- `scripts/conformance.sh` — the phase-close ECC run vs ehrbase-rs
  (compose), pinned edition, zero-drift gate vs the committed baseline.
