# B5 — tools/conformance spec-version update (chapter 7's findings)

- Status: in-progress
- Started: 2026-07-10   Owner: Ruben
- Governing plan: blueprint §3 B5; detail `docs/blueprint/07-cnf.md` (D1–D5)
- Oracle: CNF certificate/profiles docs; ECC baseline 298 passed / 338
  executed, zero-drift gate

## Tasks (blueprint §3 B5)

- [ ] 1. D1 — ITS-REST identity: derive `SpecVersions.its_rest` from the
      vendored provenance (owner ruling: the tested identity is the vendored
      `-codegen` tree, labeled honestly — not a hand-asserted "1.0.3");
      CI-check the two vendored ITS-REST trees are the same ref.
- [ ] 2. D2 — re-adjudicate the 12 SM-op-without-REST-binding failures:
      rebind `get_versioned_directory` to `GET /directory/{version_uid}`;
      `list_contributions` / bare `list_queries` / `delete_opt` →
      skip-with-reason/extension cases; fix citations.
- [ ] 3. D3 — golden-corpus adjudication: LIMIT-before-ORDER-BY goldens →
      corpus-dialect skip; `e/ehr_status` stays failing until the engine fix.
- [ ] 4. D5 — CORE claimability: tag Versioning/AnonymousEhrs cases; decide +
      document Adl14ArchetypeProvisioning evidencing.
- [ ] 5. D4 — full OPTIONS surface model (ADL 2, AQL advanced/terminology,
      Admin sub-capabilities, Messaging) with per-capability "any passes".
- [ ] 6. Conformance Statement + Certificate artefacts from results.json per
      certificate/master03-certificate.adoc.
- [ ] 7. SEC cases (auth 401/403 surface) + `schedule_ref` on CaseMeta.

## Exit criteria

- [ ] The instrument is honest: every failure a real server defect, the
      report claims the version it actually tests, CORE/STANDARD claimable;
      full ECC zero drift (only adjudication-sanctioned deltas).

## Handoff

Opened from develop @ B4 merge (PR #39). Process rules: ONE cargo runner;
agents isolate CARGO_TARGET_DIR; foreground verification; ECC centrally only.
