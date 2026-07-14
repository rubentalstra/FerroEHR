# THE BLUEPRINT — the master build document

Created 2026-07-09 from the seven verified blueprint chapters (01–07, each
spec-cited against the vendored oracle at `docs/specs/openehr/` and verified
against the working tree). This is the single document the project is driven
from — it is both the roadmap and the consolidated gap ledger:
`docs/plans/current-phase.md` is the live pointer under it, §2 holds the proven
foundations + full gap surface, and the chapters (`01-rm.md` … `07-cnf.md`) are
the per-component detail. Update this file and the affected chapter at every
phase close.

---

## 1. Mission

**Build the first fully spec-compliant openEHR CDR** — compliance measured, not
asserted: every claim traceable to a vendored-spec citation and an ECC test
execution. "Fully compliant" means all of:

| Capability | Governing spec | Where it stands (chapter) |
|---|---|---|
| **REST API** — ITS-REST 1.0.3 wire, canonical JSON + XML, all resource APIs | ITS-REST / ITS-JSON / ITS-XML | Ch 5 — routes/headers/formats DONE; protocol tail open (MUST-level `openEHR-VERSION.*` header merge, `Last-Modified`, `OPTIONS /`) |
| **Security / authn** — Basic + OAuth2/OIDC, 401/403 discipline; auth out of band per SM | ITS-REST §Auth; SM master02 | DONE (P11 + ADR-011 `access/` unification); SEC ECC cases pending. Fine-grained RBAC is not a spec-compliance item (the spec places authorization out of band) — it is a distinct enterprise track that follows this compliance mission, not blueprint work |
| **ATNA auditing / System Log** — IHE ATNA-compliant system log | SM master02 (the only normative line); IHE ATNA | DONE (`SystemLog` trait + DICOM-over-syslog impl, `app/ehrbase/src/system_log/`); ECC evidencing pending |
| **Version update semantics** — CONTRIBUTION-wrapped commits, `UPDATE_VERSION` envelope, server-side audit completion, lifecycle states, logical delete, time-travel | RM common master06; SM master03 | DONE and formally audited 1:1 (PR #33, §2.1); open: committal-header merge (ch 5 R4), `is_modifiable` write guard, incomplete-lifecycle relaxation |
| **EHR Extract** — export/import, IMPORTED_VERSION, clone EHRs, TDD import | RM ehr_extract; SM master09 | MISSING but fully designed (SM-5, `docs/design/sm-platform/10-message-integration.md`); all RM types generated |
| **Terminology-server integration** — external tx server (FHIR TS), AQL `TERMINOLOGY()`/matches-URI, `I_TERMINOLOGY_SERVICE` | QUERY master03; SM master12 | openEHR-bundle provider DONE behind the `TerminologyService` trait; external-server provider + AQL family MISSING (typed rejects) |

Foundation already banked (not re-litigated here): the generated spec layer
(ADR-004/005, all fidelity gates green), greenfield PG18 storage with audited
change-control semantics (ADR-008, §2.1), the SM-literal native API
(ADR-010/011, §2.1), the AQL engine core (P16), and our own ECC conformance
framework (`tools/conformance`, 310 catalogued cases).

---

## 2. Compliance state

Baseline conformance signal (re-derived 2026-07-13 by the **W-10 instrument
rewrite** — spine-first case derivation, multi-SUT + BYO endpoint, edition
ladder, per-SUT artefacts under `docs/conformance/<sut>/`;
`docs/plans/w10-conformance-redesign.md`): ECC = **369 executions · 334
pass · 0 fail** (35 documented adjudication skips; ratcheted 2026-07-13 by ECC-TPL-017, the ADL 1.4 example→commit round-trip probe born from the W-11 CKM-pack defect); machine profile verdicts
**CORE PASS · STANDARD PASS · OPTIONS OBTAINED**. Ancestor (B7-era
instrument): 341 · 315 · 0 · 26 — every delta explained in the W-10 plan
file (new coverage +27; +9 adjudicated skips in that new coverage; all
first-run failures spec-triaged into instrument fixes or real, separately
committed server fixes). The B1–B6 build order is executed; §2.2's failure
surface is empty; §2.3's State column was refreshed at B7 close (per-row
deltas live in each build step's close note in §3 and the phase files).
Remaining trajectory: P20 optimization (re-planned per WORKLIST W-5), P17
interop audit. (P99 removed 2026-07-12.)

### 2.1 Proven foundations (evidence, not intent)

Measured, not felt — every claim traces to an audit, a test, or a run:

| Claim | Evidence |
|---|---|
| Storage change-control semantics realize RM common master06 1:1 | Formal audit 2026-07-09 (no blockers; indelibility, logical delete, contribution atomicity, EHR creation, change_type preservation, attestations, signatures, revision history all cited) + **all 7 findings fixed same day** (five-state lifecycle, per-version `creating_system_id`, audit copy rule, full-corpus jsonb round-trip, `System_id_valid` CHECK, merge/indelibility PORT NOTEs) — PR #33 |
| Stored data is canonical openEHR JSON, lossless | Node codec: full corpus round-trip **in memory and through jsonb** |
| The wire passes 211/318 ECC executions, zero-drift-gated per phase | `docs/conformance/CONFORMANCE_REPORT.md` (pre-rewrite baseline; ECC suspended during the ADR-011 rebuild, re-converges at B1/P19) |
| Deliberate spec deviations are recorded, never silent | 49 source files carry `PORT NOTE`s with citations |

**On "our SQL tables aren't proper":** openEHR defines **no SQL schema** —
nothing exists to follow 1:1 at the table level. What it *does* define —
versioning/change-control semantics, canonical data fidelity — is exactly what
the audit verified and fixed. The remaining schema-adjacent items are ordered
work in §3: the constraint-evaluation primitives at B2, IMPORTED_VERSION
storage at B3, and the archive storage-movement PERF item in the tail.

### 2.2 ECC failure breakdown (2026-07-09 baseline: 106 failing)

**Refreshed 2026-07-10: zero failing.** The 2026-07-09 breakdown below was
burned down as planned (B2 took ArchetypeValidation 81→0; B6 took every
tail); kept for the record:

| Capability | Was failing (2026-07-09) | Burned down at |
|---|---|---|
| ArchetypeValidation | 81 | B2 (0 failing since) |
| DemographicApi | 6 | B6 |
| ChangeSets | 5 | B6 |
| AqlBasic | 5 | B6 |
| QueryProvisioning | 4 | B6/B5 adjudications |
| Adl14OptProvisioning | 2 | B2/B6 |
| CompositionOps / DirectoryOps / Signing | 1 each | B6 |

### 2.3 The spec-area map

One row per spec area, distilled from chapters 01–07. **State** was verified
2026-07-09 and **refreshed 2026-07-10 (B7): every "Remaining work" cell whose
build step is B1–B6 is now DONE** (see the per-step close notes in §3); the
still-open residue is exactly: SIM-B/SDF interop audit (row 17, P17), PERF
items (P20 — incl. the ADR-013 speculative magnitude-index repricing), and
the Stage-2 enterprise track. Schema foundation re-authored enterprise-grade
at B7 (ADR-013).

| # | Spec area (chapter) | Current state | Remaining work | Priority | Build step |
|---|---|---|---|---|---|
| 1 | **AM — archetype/template constraint validation** (ch 3) | PARTIAL — WebTemplate walk enforces existence/cardinality/occurrences + most leaves, but skips slots, closed-world, temporal ranges, precision, ordinal pairs; **81/106 ECC failures** | The validation-depth phase: closed-world ADR (F-07-05), slot enforcement (F-07-10), leaf completion (temporal intervals, precision, `DV_ORDINAL` pairs, C_STRING regex fail-closed), BMM type model (F-07-13); ingestion-side artefact validity (VCOC/VACMCO, VATID/VTLC, VTTBK/VTCBK, VCORM/VCARM/VCAEX/VCACA/VCAM); CONSTRAINT_REF policy PORT NOTE | **CRITICAL — the big rock** | B2 |
| 2 | RM — change control & versioning (ch 1 §A) | DONE, formally audited 1:1 (PR #33) | `EHR_STATUS.is_modifiable = False` write guard (currently never checked on write); incomplete-lifecycle (553) relaxed validation; 5 ChangeSets ECC edges; branching/merging stay PORT-NOTEd (trunk-only) until SM-5 | HIGH | B2 (guards) / B6 (edges) |
| 3 | RM — EHR/directory/tags/EHR_ACCESS (ch 1 §C–D) | DONE | `EHR.folders` multi-hierarchy beyond `directory` (or PORT NOTE + CNF cross-check); DirectoryOps 1 ECC edge | LOW | B6 |
| 4 | RM — data structures + data types invariants (ch 1 §F–G) | DONE | Spec-audit area 12 residue (DV_AMOUNT/DV_QUANTIFIED accuracy semantics, 5 major + 6 minor) | MED | B2/B6 |
| 5 | RM — demographic (ch 1 §H, own wire design) | DONE (service + versioning) | 6 DemographicApi ECC OPTIONS-profile wire edges | MED | B6 |
| 6 | **RM — EHR Extract / IMPORTED_VERSION / clone EHR** (ch 1 §J, ch 6 §I) | MISSING — designed (SM-5), all X_* types generated | `I_EHR_EXTRACT_SERVICE` (X_VERSIONED_* assembly, EXTRACT_VERSION_SPEC, OBJECT_REF rewrite, multimedia/links flags); import path (clone-EHR with reused ehr_id, IMPORTED_VERSION commits); `I_TDD_SERVICE.import_tdd`; MSG ECC cases | HIGH (mission item) | B3 |
| 7 | BASE — identifiers (ch 2 §A–B) | DONE except one unregistered gap | Case-insensitive composite-identifier equality (R10) + canonicalisation at the storage boundary; file the SPEC_AUDIT finding | MED | B2 |
| 8 | BASE — `Multiplicity_interval`/`Cardinality`/`Interval` functions (ch 2 §C) | PARTIAL — types generated, no `*_impl.rs` | Implement the occurrence/cardinality math + `has/intersects/contains` — these are the constraint-evaluation primitives of the 81-case rock | HIGH | B2 |
| 9 | BASE — ISO 8601 time types (ch 2 §D) | DONE by policy (string-validated) | Calendar-exact `Day_valid` (reject `2021-02-31`); date arithmetic stays consumer-driven | MED | B2 |
| 10 | TERM — bundle + binding (ch 2 §F) | DONE (byte-identical assets, SPECPR-51 handled, codes-not-rubrics fixed) | Spec identifier constants (F-11-07); code-set index (F-11-06); terminology **wire** exposure (extension OAS, designed) | LOW/MED | B4 |
| 11 | AQL — language core (ch 4) | DONE — parser complete; engine executes the core envelope, every reject typed | **OR-CONTAINS** (normative, currently rejected); the whole single-row function set (Q-20/21/22 + `CURRENT_TIMEZONE` whitelist); semantic pass (duplicate vars, LIMIT/OFFSET bounds — F-08-14); `e/ehr_status` on EHR (RM-model special case); 5 AqlBasic + 4 QueryProvisioning ECC edges | HIGH | B2-adjacent / B6 |
| 12 | **AQL — terminology family** (ch 4 Q-15/16/23) | MISSING (typed rejects) | `TERMINOLOGY()` (expand/validate/…) + `matches {uri}` + mixed lists, expansion merged at semantic analysis; staged: expand → validate-as-boolean → URI operand | HIGH (mission item) | B4 |
| 13 | ITS-REST — general protocol (ch 5 §A) | PARTIAL | **`openEHR-VERSION.*`/`openEHR-AUDIT_DETAILS.*` request-header parse + merge (spec MUST — currently unimplemented; B6, headers early if cheap)**; `Last-Modified` emission (plumbed in `-sm`, never emitted); If-Match hardening (full OVID compare, reject unparseable — F-01-09/F-02-08); `OPTIONS /` conformance endpoint (R32); `Prefer: resolve_refs` (SHOULD) | HIGH | B6 (headers early if cheap) |
| 14 | ITS-REST — resource APIs (ch 5 §B–D) | DONE as routes + headers (W2-A) | Status-code mapping fixes (F-02-10, F-03-09, F-03-13, F-03-14, F-01-11); query wire tail (RESULT_SET `ETag`, query-level 408, `query_type`); DIRECTORY `path` semantics (F-02-12); composition body-uid cross-check (F-02-11) | MED | B6 |
| 15 | Canonical JSON/XML payloads (ch 5 §F) | DONE at the COMPOSITION wire (C14N gate green) | Version-family/CONTRIBUTION XML shape (F-05-06, currently 406 — needed for "XML on every endpoint" + ECC-COM-022); JSON minors (DV_COUNT i32, RM-1.1.0 schema ceiling doc) | MED | B6 |
| 16 | SM — platform services (ch 6) | 26 DONE / 4 PARTIAL / 4 MISSING; ADR-011 rebuild **in flight** | Finish rebuild (test porting, forwarding-layer deletion, workspace green); SM-4 wave 3 Admin dump/load; SM-5 Message (row 6); SM-6 Subject Proxy; ADL 1.4/2 source parsers (B3, ahead of AOM2 semantic validation); SM-only wire exposure (EHR Index/Terminology/Admin) | HIGH | B1/B3 |
| 17 | SM — SIM-B / SDF formats (ch 6 §M) | PARTIAL — Better semantics implemented (P14) | P17 audit vs the transformation-rule tables + `ctx/` vocabulary; accept SDF-normative leaf encodings; document the quantity-encoding divergence. Not CNF-gated — interop quality | LOW | P17 (interleaved) |
| 18 | Security / authn | DONE | SEC ECC cases (401/403 surface, mirroring the Robot `I_OAuth2_Keycloak` suite) — cheap, precede SM-5 | LOW | B5 |
| 19 | ATNA / System Log | DONE | Capability evidencing in ECC (Logging is a STANDARD-profile capability) | LOW | B5 |
| 20 | **CNF — the conformance instrument itself** (ch 7) | PARTIAL — ECC runs, but ~17/106 failures are runner/spec-gap mis-bookings and CORE is structurally unclaimable | D1 ITS-REST identity (dev OAS labeled 1.0.3); D2 re-adjudication (12 failures: SM ops with no REST binding); D3 golden adjudication (5: 2019-era `LIMIT` dialect); D5 CORE claimability (tag Versioning/AnonymousEhrs, decide Adl14ArchetypeProvisioning); D4 full OPTIONS surface; Statement + Certificate artefacts; MSG/SEC areas; `schedule_ref` on `CaseMeta` | HIGH | B5 |
| 21 | Terminology-server **integration testing** | MISSING | wiremock + real-tx-server harness in `tools/conformance`; the real server is an off-the-shelf Dockerised FHIR R4 TS (HAPI default / Snowstorm) pointed at by URL — `docs/design/terminology-server-integration.md` (see B4) | HIGH (mission item) | B4 |

---

## 3. The build order

Numbered sequence to "first fully spec-compliant". Each step closes behind the
standing gates (§4); ECC re-baselines at B1 and must never regress thereafter.

### B1 — Finish the ADR-011 rebuild — **DONE** (PR #36, 2026-07-09)
Close SM-4 wave 2: port the remaining `ehrbase`-crate tests to the new seams
(rest suite already 218/218), delete residual forwarding layers, re-tick the
stale SM-4 checkboxes, `cargo nextest run --workspace` green, clippy clean.
**Exit: workspace green + ECC re-converged at the 211/318 zero-drift baseline.**
*Closed 2026-07-09: ECC re-converged at exactly 211/318 (zero drift; two rebuild
regressions root-caused and fixed — the raw-wire CONTRIBUTION seam and the
template_id-keyed OPT GET); `ehrbase-rest` 218/218; the in-process `self-host`
SUT mode was removed (owner ruling) — conformance always runs against the
Docker-composed server (`scripts/conformance.sh`).*

### B2 — Validation depth (the big rock: 81 ECC ArchetypeValidation cases) — **DONE** (2026-07-10, `docs/plans/b2-validation-depth.md`)
A dedicated phase with the ECC data sets as the oracle (the big rock, §2.2).
Contents, in dependency order:
1. `multiplicity_interval_impl.rs` + `cardinality_impl.rs` + BASE `Interval`
   functions (ch 2 items 2/7) — the constraint-evaluation primitives.
2. Closed-world semantics ADR + implementation (F-07-05), after checking CNF
   fixtures for tolerated RM metadata.
3. Slot enforcement (F-07-10): WebTemplate nodes for open `ARCHETYPE_SLOT`s
   (rm_type + occurrences + include/exclude regexes).
4. Leaf completion: temporal interval constraints + timezone patterns, decimal
   precision, `DV_ORDINAL` (symbol,value) pairing + alternative-block joint
   matching (F-07-06), fail-closed C_STRING patterns (F-07-11).
5. Type conformance via the BMM-generated `openehr_rm::model` (F-07-13).
6. Ingestion-side artefact validity on OPT upload: VCOC/VACMCO, VATID/VTLC,
   VTTBK/VTCBK, VCORM/VCARM/VCAEX/VCACA/VCAM → 400 with the AOM2 code.
7. Commit-path guards that ride along: `is_modifiable = False` write blocking
   (ch 1 item 2), incomplete-lifecycle (553) relaxed validation (ch 1 item 3),
   case-insensitive identifier equality (ch 2 item 1), calendar-exact
   `Day_valid` (ch 2 item 3).
8. Reconcile the open spec-audit area-07/12 findings.
**Exit: the 81 VAL failures → green (minus any B5-adjudicated corpus defects);
zero drift elsewhere.**
*Closed 2026-07-10: ECC 319 executed · 293 passed (baseline ratcheted from
211/318); ArchetypeValidation 0 failing incl. the new ECC-VAL-119 negative
case; owned fixture register instituted; ADR-012 closed-archetype semantics;
all 8 phase tasks done — remaining ECC failures are the B6 tails.*

### B3 — SM-5 / SM-6 (the designed-but-unbuilt services) — **DONE** (2026-07-10, `docs/plans/b3-sm-services.md`)
1. SM-4 wave 3 — Admin dump/load (`export_ehrs`/`load_ehrs`, `EXPORT_SPEC`,
   segmenting, `DUMP_LOAD_FAIL_REPORT`; round-trip test + duplicate-id failure).
2. **SM-5 — Message service**: `I_EHR_EXTRACT_SERVICE` (export whole-EHR +
   spec-driven; import into fixed/existing EHR) over the existing `vobject`
   machinery + generated `ehr_extract` types; the import path lands
   IMPORTED_VERSION storage, clone-EHR with reused ehr_id, and versioning
   scenarios Case 2/3 (ch 1 reqs 13, 31, 35, 50–53); `I_TDD_SERVICE.import_tdd`
   as TDD → COMPOSITION over OPT/WebTemplate. Design:
   `docs/design/sm-platform/10-message-integration.md`. Decide version
   branching here (required before distributed import of modified copies) or
   keep the typed rejection PORT-NOTEd.
3. SM-6 — Subject Proxy: subject/variable/data-set/binding stores,
   `I_DATA_BINDING` with the openEHR frame = AQL over our Query service;
   FHIR/HL7v2 frame seams stubbed.
4. MSG ECC cases land with SM-5 (`Area::Msg` exists, zero cases today).
**Exit: rows 6 and 16 of the map fully DONE; MSG area evidenced.**
*Closed 2026-07-10: Admin dump/load; EhrExtractService export + import
(IMPORTED_VERSION, master06 Cases 2/3, clone-EHR); the TDD → COMPOSITION
converter (openehr_flat::from_tdd, corpus-verified); SM-6 Subject Proxy
(I_SUBJECT_PROXY_SERVICE + I_DATA_BINDING, openEHR frame over AQL);
ECC-MSG-001..010 (native-API-only, skip-with-reason). ECC 329 executed ·
293 passed · zero drift. Remaining PORT-NOTEd: version branching
(trunk-only), FHIR/HL7v2 frames, TDD constructs outside the corpus.*

### B4 — Terminology-server integration (+ its test harness) — **DONE** (2026-07-10, `docs/plans/b4-terminology.md`)
Design set: the terminology **client** is `docs/terminology-validation.md`; the
self-hostable **server** to run in Docker and point at by URL (HAPI FHIR
default, Snowstorm opt-in) is `docs/design/terminology-server-integration.md` —
we run an off-the-shelf FHIR R4 TS, never build one.
1. External tx-server provider (FHIR TS via `reqwest`) behind the existing
   `TerminologyService` trait (`ehrbase-sm/src/services/terminology.rs`) —
   real `subsumes`, `value_set_validate`, `get_value_set` against a remote
   server; the openEHR-bundle provider remains the local default.
2. AQL terminology family (Q-15/16/23): `TERMINOLOGY('expand'|'validate'|…,
   service_api, params_uri)` with expansion merged into `matches` value lists
   at semantic-analysis time (master03 lines 756–759); staged expand →
   validate-as-boolean → URI-operand; keep typed rejects until each lands.
3. Terminology wire exposure (extension OAS, doc 08 §7) for
   `I_TERMINOLOGY_SERVICE` (+ EHR Index/Admin wire while in the area).
4. **Test harness (new scope, into `tools/conformance`):** a `TS` case area
   driven by (a) a **wiremock-backed FHIR-tx fixture server** spun up by the
   runner (canned expand/validate/lookup/subsumes ValueSet/CodeSystem
   responses, fault injection: timeouts, 5xx, malformed), and (b) an optional
   **real-server mode** (`--tx-server-url`) that runs the same cases against a
   live terminology service — the Dockerised HAPI FHIR (or Snowstorm) TS from
   `docs/design/terminology-server-integration.md` — and is skip-with-reason
   when unset. Cases assert
   both the AQL surface (`matches {TERMINOLOGY(…)}` result sets) and the
   service surface, with the wiremock exchange recorded in the report.
**Exit: mission item "terminology-server integration" demonstrable in CI
without a network, and against a real server on demand.**
*Closed 2026-07-10: FhirTerminologyProvider (validate-code/expand/subsumes/
lookup, config opt-in); AQL TERMINOLOGY('expand') merged into matches at
semantic analysis (stages b/c typed rejects, PORT-NOTEd); /terminology
extension wire (config-gated); TS ECC area (ECC-TS-001..009, wiremock
fixture + --tx-server-url). ECC 338 executed · 298 passed · zero drift.*

### B5 — tools/conformance spec-version update (chapter 7's findings) — **DONE** (2026-07-10, `docs/plans/b5-conformance-instrument.md`)
1. **D1** — resolve the ITS-REST identity: pick Release-1.0.3 vs
   development@e8a093e, re-vendor `crates/openehr-its/vendor/rest-oas/`
   accordingly, derive `SpecVersions.its_rest` from provenance (not a
   literal), CI-check the two vendored ITS-REST trees are the same ref.
2. **D2** — re-adjudicate the 12 SM-op-without-REST-binding failures: rebind
   `get_versioned_directory` to `GET /directory/{version_uid}`; convert
   `list_contributions`, bare `list_queries`, `delete_opt` to
   skip-with-reason/extension cases; fix citations.
3. **D3** — golden-corpus adjudication list: LIMIT-before-ORDER-BY goldens →
   corpus-dialect skip; keep `e/ehr_status` failing until the engine fix (B2/B6).
4. **D5** — make CORE claimable: tag Versioning/AnonymousEhrs cases; decide +
   document `Adl14ArchetypeProvisioning` evidencing.
5. **D4** — model the full OPTIONS surface (ADL 2, AQL advanced/terminology,
   Admin sub-capabilities, Messaging) and switch OPTIONS to per-capability
   "any passes".
6. Emit Conformance **Statement + Certificate** artefacts from `results.json`
   per `certificate/master03-certificate.adoc`.
7. Add SEC cases (auth 401/403 surface) and the `schedule_ref` trace field on
   `CaseMeta`.
**Exit: the instrument is honest — every failure is a real server defect, the
report claims the version it actually tests, CORE/STANDARD are claimable.**
*Closed 2026-07-10: ECC 341 executed · 303 passed · 12 failed (all real
defects — the B6 list); identity provenance-derived (development@e8a093e) +
reconciliation guard; D2/D3 adjudications; CORE claimable (Versioning/
AnonymousEhrs/Adl14ArchetypeProvisioning evidenced); full OPTIONS surface
any-passes; Statement + Certificate artefacts; ECC-SEC-001/002 pass live;
schedule_ref threaded.*

### B6 — P19: full conformance — **DONE** (2026-07-10, `docs/plans/b6-full-conformance.md`)
The convergence phase: burn down every remaining honest failure and wire tail.
- ITS-REST protocol tail (map row 13): committal-header merge (MUST),
  `Last-Modified`, If-Match hardening, `OPTIONS /`, status-code fixes, query
  wire tail, DIRECTORY path semantics, version-family XML (F-05-06).
- AQL tail (row 11): OR-CONTAINS, the single-row function set,
  semantic-pass constraints, `e/ehr_status`, the AqlBasic/QueryProvisioning
  edges.
- The small-capability ECC tails: ChangeSets (5), DemographicApi (6),
  Adl14OptProvisioning (2), CompositionOps/DirectoryOps/Signing (1 each).
- Close the spec-audit backlog (82 open findings) or PORT-NOTE each residual.
**Exit: full ECC green (100% minus documented skip-with-reason adjudications);
CORE + STANDARD claimed with the Statement/Certificate artefacts; OPTIONS
report produced. This is the "first fully spec-compliant openEHR CDR" claim.**
*Closed 2026-07-10: ECC 341 executed · 315 passed · 0 failed; machine
verdicts CORE PASS · STANDARD PASS · OPTIONS OBTAINED. The claim state is
reached — measured, not asserted. Remaining trajectory: P20 optimization,
P17 interop audit (not conformance-gated). P99 removed 2026-07-12.*

### Tail — P20 optimization (re-plan first)
Unchanged from the phase plan: PERF(port) items, AIO tuning, index passes
(P20 — the stale plan is re-evaluated before execution, WORKLIST W-5;
P99 was removed 2026-07-12, the release machinery having shipped with
v3.0.0). P17's EhrScape/SIM-B/SDF audit interleaves where
scheduled (not conformance-gated).

---

## 4. Standing rules

These govern every step above (they restate the hard rules of `CLAUDE.md` and
the ADRs; deviations are defects):

1. **Docs first — the vendored spec is the oracle.** Before implementing or
   reviewing any spec-facing behaviour, read the governing section under
   `docs/specs/openehr/` (`/spec-lookup`) and cross-check the CNF schedule.
   Never resolve a spec question from memory or from EHRbase behaviour; cite
   the spec file + section in code and commits for conformance-relevant
   decisions. The blueprint chapters carry the extracted requirements — trust
   but re-verify citations when a chapter is >1 phase old.
2. **PORT NOTE discipline.** Every deliberate deviation or spec-defect
   workaround carries a `// PORT NOTE:` with the citation (49 files already
   do). Spec defects are recorded verbatim in the chapter's "Spec
   defects/TBDs" section — we implement the evident *meaning* and note the
   defect, never silently guess. (We do not defer our own work items: every
   gap is ordered work in §3, never an open-ended deferral.)
3. **Never weaken, skip, or delete a test** to make a build pass; never edit a
   test to route around a bug it exposes. Corpus/golden defects are handled
   only through the B5 adjudication process (skip-with-reason, recorded in the
   report), never by editing the case to pass.
4. **ECC gate policy.** ECC is suspended only during the ADR-011 rebuild
   (owner ruling); from B1 close onward every phase ends with an ECC run that
   must show **zero drift** vs the standing baseline — the only permitted
   delta is newly-green cases. The baseline ratchets upward at each phase
   close and is committed (`docs/conformance/ehrbase-rs/results.json` +
   report + badges — per-SUT artefact dirs since W-10).
   Profile claims (CORE/STANDARD/OPTIONS) are machine-computed by the runner,
   never hand-asserted.
5. **Generated layer discipline.** Never hand-edit a `// @generated` file —
   change the emitter and regenerate (`/regen-codegen`); the drift check must
   stay green. Spec behaviour goes in `*_impl.rs` siblings.
6. **Compiling, tested increments** (ADR-006/008): every phase lands green
   (`cargo nextest run --workspace`, clippy clean, testcontainers PG18 where
   DB is involved). Branches are `claude/*`; tick the phase checkbox and
   commit before ending a session; no AI attribution anywhere, ever.
7. **Blueprint maintenance.** At each phase close: update the affected
   chapter's state table, this file's §2 (proven foundations + ECC breakdown
   + spec-area map) and build-order status, and `docs/plans/current-phase.md`.
   The map's "State" column must always reflect *verified* reality (file:line
   or ECC evidence), never intent.

---

*Chapters: [01 RM](01-rm.md) · [02 BASE+TERM](02-base-term.md) ·
[03 AM](03-am.md) · [04 QUERY](04-query.md) · [05 ITS](05-its.md) ·
[06 SM](06-sm.md) · [07 CNF](07-cnf.md) — index in [README.md](README.md).*
