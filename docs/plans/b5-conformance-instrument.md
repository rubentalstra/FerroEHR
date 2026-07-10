# B5 — tools/conformance spec-version update (chapter 7's findings)

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Governing plan: blueprint §3 B5; detail `docs/blueprint/07-cnf.md` (D1–D5)
- Oracle: CNF certificate/profiles docs; ECC baseline 298 passed / 338
  executed, zero-drift gate

## Tasks (blueprint §3 B5)

- [x] 1. D1 — ITS-REST identity: derive `SpecVersions.its_rest` from the
      vendored provenance (owner ruling: the tested identity is the vendored
      `-codegen` tree, labeled honestly — not a hand-asserted "1.0.3");
      CI-check the two vendored ITS-REST trees are the same ref.
- [x] 2. D2 — re-adjudicate the 12 SM-op-without-REST-binding failures:
      rebind `get_versioned_directory` to `GET /directory/{version_uid}`;
      `list_contributions` / bare `list_queries` / `delete_opt` →
      skip-with-reason/extension cases; fix citations.
- [x] 3. D3 — golden-corpus adjudication: LIMIT-before-ORDER-BY goldens →
      corpus-dialect skip; `e/ehr_status` stays failing until the engine fix.
- [x] 4. D5 — CORE claimability: tag Versioning/AnonymousEhrs cases; decide +
      document Adl14ArchetypeProvisioning evidencing.
- [x] 5. D4 — full OPTIONS surface model (ADL 2, AQL advanced/terminology,
      Admin sub-capabilities, Messaging) with per-capability "any passes".
- [x] 6. Conformance Statement + Certificate artefacts from results.json per
      certificate/master03-certificate.adoc.
- [x] 7. SEC cases (auth 401/403 surface) + `schedule_ref` on CaseMeta.

*Tasks 1–4 done 2026-07-10 (wave 1): its_rest identity derived from vendored
provenance (development@e8a093e) + tree-reconciliation guard test; the 12
mis-booked SM-op cases re-adjudicated (get_versioned_directory rebound to the
real at-version route + Versioning-tagged; list_contributions/bare
list_queries/delete_opt → cited skips); 7 LIMIT-before-ORDER-BY goldens →
corpus-dialect skips (e/ehr_status untouched, stays failing for B6); CORE
claimability: Versioning/AnonymousEhrs tagged incl. new ECC-EHR-013
create-anonymous-ehr, Adl14ArchetypeProvisioning evidenced via the OPT
upload (decision documented).*

*Tasks 5–7 done 2026-07-10 (wave 2): (5) D4 — the full OPTIONS surface is
modeled (`Capability` gained Adl2Provisioning, AqlAdvanced, and the six Admin
sub-caps; Authentication for SEC), and `profile::verdict` is now any-passes for
OPTIONS (all-or-nothing kept for CORE/STANDARD) with per-capability
pass/fail/not-evidenced labels — wire-unreachable caps never block (B3 kept).
(6) New report artefacts `CONFORMANCE_STATEMENT.md` + `CONFORMANCE_CERTIFICATE.md`
(`reporting/statement.rs`), emitted by `report::write_all` (so `report --from`
regenerates them) per certificate/master03-certificate.adoc: SUT + scope tables,
a Profile Report (the §4 verdict tables), and a per-conformance-point Detailed
Test Report keyed on `schedule_ref`. (7) `suites/security.rs` — ECC-SEC-001
(unauthenticated → 401) + ECC-SEC-002 (regular credential on ADMIN route → 403),
mirroring SECURITY_TESTS/I_OAuth2 intent, skip-with-reason when the SUT auth
mode can't be read off the wire; `schedule_ref` added to CaseMeta + CaseOutcome
and threaded (via `CaseEntry::with_schedule_ref`) onto the D2 SM-op cases
(delete_opt, list_queries, get_versioned_directory, list_contributions) + all 10
Messaging cases. Gates: nextest -p conformance 48/48 green, conformance
clippy-clean, fmt clean. ECC baseline unchanged pending a fresh run (SEC adds 2
cases; OPTIONS/certificate are report-shape changes).*

## Exit criteria

- [ ] The instrument is honest: every failure a real server defect, the
      report claims the version it actually tests, CORE/STANDARD claimable;
      full ECC zero drift (only adjudication-sanctioned deltas).

## Handoff

Opened from develop @ B4 merge (PR #39). Process rules: ONE cargo runner;
agents isolate CARGO_TARGET_DIR; foreground verification; ECC centrally only.
