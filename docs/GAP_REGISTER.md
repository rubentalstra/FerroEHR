# The Gap Register — everything known-missing vs the openEHR specs

One consolidated view (created 2026-07-09, owner request) of every tracked
gap between this codebase and full openEHR spec compliance, with evidence
for what is already **proven**, so the state is measured rather than felt.
Update at each phase close.

## 1. What is PROVEN (evidence, not intent)

| Claim | Evidence |
|---|---|
| Storage change-control semantics realize RM common master06 1:1 | Formal audit 2026-07-09 (no blockers; indelibility, logical delete, contribution atomicity, EHR creation, change_type preservation, attestations, signatures, revision history all cited) + **all 7 findings fixed same day** (five-state lifecycle, per-version `creating_system_id`, audit copy rule, full-corpus jsonb round-trip, `System_id_valid` CHECK, merge/indelibility PORT NOTEs) — see PR #33 |
| Stored data is canonical openEHR JSON, lossless | Node codec: full corpus round-trip **in memory and through jsonb** (m5 fix) |
| The wire passes 211/318 ECC executions, zero-drift-gated per phase | `docs/conformance/CONFORMANCE_REPORT.md` (pre-rewrite baseline; ECC suspended during the ADR-011 rebuild, re-converges at P19) |
| Deliberate spec deviations are recorded, never silent | 49 source files carry `PORT NOTE`s with citations |

**On "our SQL tables aren't proper":** openEHR defines **no SQL schema**
(nothing exists to follow 1:1 at the table level). What it defines —
versioning/change-control semantics, canonical data fidelity — is exactly
what the audit verified and fixed. The remaining schema-adjacent items are
below (§2.4), all tracked.

## 2. What is KNOWN-MISSING (the actual gap surface)

### 2.1 ECC conformance — 106/318 failing (the sharpest signal)

| Capability | Failing | What it means |
|---|---|---|
| **ArchetypeValidation** | **81** | **The one big gap**: template/archetype-constraint validation depth (cardinality/occurrence/value-constraint cases from the ECC data sets that P15's validator does not yet enforce). This alone is ~76 % of all failures — the single highest-leverage work item toward "first fully compliant CDR". Target: a dedicated validation-depth phase before/at P19 |
| DemographicApi | 6 | OPTIONS-profile wire details of our own demographic design |
| ChangeSets | 5 | contribution edge cases |
| AqlBasic | 5 | AQL feature envelope edges |
| QueryProvisioning | 4 | stored-query wire edges |
| Adl14OptProvisioning | 2 | OPT provisioning edges |
| CompositionOps / DirectoryOps / Signing | 1 each | isolated cases |

### 2.2 Spec-audit findings (2026-07-06 whole-codebase audit)

**82 open / 109 fixed** (`docs/spec-audit/SPEC_AUDIT.md` + `findings/*.md`,
per-finding citations + checkboxes). Many open ones overlap §2.1 (the
validation chapter) — reconcile when the ArchetypeValidation push starts.

### 2.3 SM platform — remaining components (designed, not built)

| Item | State |
|---|---|
| SM-4 wave 2 (ADR-011 literal-catalog rebuild) | **in flight** — mid-rewrite, workspace red by design |
| SM-4 wave 2c (dissolve `ehrbase-audit`/`ehrbase-signing` into the 3-crate layout) | in flight/queued |
| SM-4 wave 3: Admin dump/load (`EXPORT_SPEC`, segmenting, `DUMP_LOAD_FAIL_REPORT`) | designed (plan) |
| SM-5: Message/EHR Extract/TDD/sync | **fully designed** (`docs/design/sm-platform/10-message-integration.md`; all RM types already generated) |
| SM-6: Subject Proxy | designed (08 §4.4) |
| EHR Index / Terminology wire exposure (extension OAS) | designed (08 §7) |

### 2.4 Deliberate deferrals (PORT-NOTEd, each a conscious decision)

- Relaxed validation for `incomplete`-lifecycle content (master06 NOTE)
- The `openEHR-VERSION.lifecycle_state` HTTP header on direct paths
- Version branching (trunk-only, F-06-09) + version merging (out of scope,
  non-distributed) + IMPORTED_VERSION storage
- ADL 1.4/2 **source parsers** (validation is structural until then; ADL2
  `example`/`version` wire ops 501)
- Archive = marker table (storage movement at P20); 7z compression;
  `store_query_set` (upstream spec TODO)
- EhrScape adapter module (P17); FLAT/SDT audit vs SIM-B/SDF (P17)

## 3. The honest bottom line

The platform's *skeleton* (services, storage semantics, wire plumbing,
versioning) is spec-verified. The *bulk* of remaining non-compliance is
one deep subsystem — **archetype/template constraint validation** — plus
a long tail of small wire edges and the designed-but-unbuilt SM
components. Priority order toward "first fully compliant":
1. Finish the ADR-011 rebuild (green workspace).
2. **ArchetypeValidation depth** (81 cases — the big rock; needs its own
   phase with the ECC data sets as the oracle).
3. SM-4 close (dump/load) → SM-5 → SM-6.
4. P19: ECC re-convergence + the small-capability tails.
