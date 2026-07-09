# B2 — Validation depth (the big rock: 81 ECC ArchetypeValidation cases)

- Status: in-progress
- Started: 2026-07-09   Owner: Ruben
- Governing plan: `docs/blueprint/00-THE-BLUEPRINT.md` §3 B2 (contents in
  dependency order) + §2.3 rows 1/8 · chapter detail `docs/blueprint/03-am.md`,
  `02-base-term.md`
- Oracle: the ECC ArchetypeValidation data sets (`tools/conformance`,
  `Area::Val`) + vendored AOM/AM spec (`docs/specs/openehr/AM/`), BASE
  (`docs/specs/openehr/BASE/`)
- Baseline entering the phase: ECC 318 executed · 211 passed · 106 failed, of
  which **81 ArchetypeValidation** (~76 % of all failures)

## Objectives

Close the ArchetypeValidation gap — template/archetype-constraint validation
depth (cardinality/occurrence/value-constraint cases the P15 validator does not
yet enforce) — with the ECC data sets as the oracle. Exit = the 81 VAL failures
green (minus any B5-adjudicated corpus defects), zero drift elsewhere.

## Tasks (blueprint §3 B2, dependency order)

- [x] 1. Constraint-evaluation primitives: `multiplicity_interval_impl.rs` +
      `cardinality_impl.rs` + BASE `Interval` functions
      (`has`/`intersects`/`contains`, occurrence/cardinality math) — blueprint
      ch 2 items 2/7 (§2.3 row 8). *Done 2026-07-09: `interval_impl.rs` (shared
      boundary algebra: `has`/`intersects`/`contains`/`is_equal` + accessors on
      `Interval<T>`/`Point_interval`/`Proper_interval`),
      `multiplicity_interval_impl.rs` (`is_open`/`is_optional`/`is_mandatory`/
      `is_prohibited` + inherited algebra + `Validate`), `cardinality_impl.rs`
      (`is_bag`/`is_list`/`is_set`); 33 new spec-cited tests (64 total in
      `openehr-base`), 4 PORT NOTEs (informal `intersects` prose, reflexive
      `contains`, type-erased `Multiplicity_interval` enum variant,
      `PartialOrd` bound); emit + drift clean.*
- [ ] 2. Closed-world semantics ADR + implementation (F-07-05), after checking
      CNF fixtures for tolerated RM metadata. *ADR-012 written (closed-archetype
      semantics, RM-metadata tolerance, zero-drift gate); implementation
      delegated — pending.*
- [ ] 3. Slot enforcement (F-07-10): WebTemplate nodes for open
      `ARCHETYPE_SLOT`s (rm_type + occurrences + include/exclude regexes).
- [ ] 4. Leaf completion: temporal interval constraints + timezone patterns,
      decimal precision, `DV_ORDINAL` (symbol,value) pairing +
      alternative-block joint matching (F-07-06), fail-closed C_STRING
      patterns (F-07-11).
- [ ] 5. Type conformance via the BMM-generated `openehr_rm::model` (F-07-13).
- [ ] 6. Ingestion-side artefact validity on OPT upload: VCOC/VACMCO,
      VATID/VTLC, VTTBK/VTCBK, VCORM/VCARM/VCAEX/VCACA/VCAM → 400 with the
      AOM2 code.
- [ ] 7. Commit-path guards that ride along: `is_modifiable = False` write
      blocking (ch 1 item 2), incomplete-lifecycle (553) relaxed validation
      (ch 1 item 3), case-insensitive identifier equality (ch 2 item 1),
      calendar-exact `Day_valid` (ch 2 item 3).
- [ ] 8. Reconcile the open spec-audit area-07/12 findings.

## Exit criteria

- [x] The 81 ArchetypeValidation ECC failures → green (minus any
      B5-adjudicated corpus defects), verified by a full
      `scripts/conformance.sh` run. *2026-07-09: full run 319 executed ·
      293 passed · 25 failed — ArchetypeValidation 0 failing (119/119 in the
      filtered run incl. the new ECC-VAL-119); root causes: the defective
      all_types fixtures (owned register), the renamed-sibling walker skip,
      and three defective case authorings.*
- [x] Zero ECC drift elsewhere (baseline ratchets 211/318 → **293/319**;
      regressed 0 · newly-green 81).
- [ ] Workspace green (`cargo nextest run --workspace`), clippy clean.
- [ ] Blueprint §2 + ch 2/3 state tables updated; `current-phase.md` advanced.

## Decisions made this phase

- ADR-012 — closed-archetype validation semantics for OPT 1.4 commits
  (F-07-05): closed for archetyped content, open for RM-permitted metadata,
  landed only behind an ECC zero-drift run.
- Owned fixture register policy (`tools/conformance/testdata/fixtures/` +
  REGISTER.md + companion negative cases; conformance-framework.md §5.3).
- The in-process self-host SUT mode stays removed (B1); all conformance runs
  go through the Docker compose stack.

## Handoff for next session

Phase opened from develop @ B1 merge (PR #36; ECC re-converged 211/318
zero-drift). Next action: task 1 — read the BASE spec sections for
`Multiplicity_interval`/`Cardinality`/`Interval`, then implement the
`*_impl.rs` siblings.
