# W-3f — the `ehrbase` platform-crate redesign: the session prompt

> Paste everything below the line into a fresh session on a clean
> `develop` checkout. It carries the full mandate, method, oracles, rulings
> and exit criteria. WORKLIST row: W-3f.

---

Execute **W-3f: the complete redesign and rewrite of the `ehrbase` platform
crate** (`app/ehrbase` — the binary + `Platform` implementation: storage,
service layer, AQL engine, versioning). `ehrbase-sm` and `ehrbase-rest` were
already rebuilt specification-first (W-3c/W-3e, merged); this crate is the
last unstructured one: ~34k lines, a flat `service/` grab-bag, and files of
1,000–2,000 lines (`vobject.rs` ~2,100, `contribution.rs` ~1,350,
`opt_validation.rs` ~1,320, `message.rs` ~1,190, `ehr.rs` ~1,130, …).

## The oracles (in precedence order)

1. **The FULL BASE component** — the foundation oracle (owner ruling
   2026-07-12: check everything under `docs/specs/openehr/BASE/docs/` — a
   strong foundation needs all four sets, not only the overview):
   - `architecture_overview/` (18 masters) — the *structural* oracle: its
     chapters define how the concerns compose; read the relevant chapter
     BEFORE designing each area. Worked example of the ruling:
     **§Integrity places digital signature inside the versioning/integrity
     design**, so the standalone `signing` module dissolves into the
     versioning area rather than staying a bolt-on. The W-3a distillation
     (`docs/spec-audit/architecture-overview/CHECKLIST.md`, 149 rows) is
     the index — re-read the spec text for anything you build.
   - `base_types/` (7 masters) — identification is the versioning core's
     law: `OBJECT_VERSION_ID`/`VERSION_TREE_ID` lexical forms, `PARTY_REF`,
     `OBJECT_REF` namespaces, `ARCHETYPE_ID`/`TEMPLATE_ID` — the
     `versioning/` and `storage/` registers audit against these masters
     directly.
   - `foundation_types/` (11 masters) — the primitive semantics under
     everything: ISO-8601 time types, `Interval`/`Multiplicity_interval`
     functions, terms/containers — the `validation/` and `storage/`
     registers cite these for every primitive-handling decision.
   - `resource/` (4 masters) — `AUTHORED_RESOURCE` (description,
     translations, revision history of authored artefacts) — governs the
     `templates/` area's treatment of archetypes/OPTs as resources.
   The A1 audit and B2 validation phase already *behaviour-verified* much
   of base_types/foundation_types — the registers cross-reference those
   verdicts (`docs/spec-audit/`, blueprint ch 02) instead of re-deriving
   them, but every structural reading is fresh.
2. **The SM chapter map** — the service layer mirrors
   `app/ehrbase-sm/src/services/` (one folder per SM chapter, rebuilt at
   W-3c): the impl side should mirror the same chapter structure.
   Registers: `docs/design/sm-platform/` (per-chapter G-row tables — any
   still-open impl-side rows land in this phase; W-3d overlaps and can be
   absorbed).
3. **RM common change-control** (`docs/specs/openehr/RM/docs/common/` —
   VERSION/VERSIONED_OBJECT/CONTRIBUTION) for the versioning core, and the
   storage/AQL design docs (`docs/design/aql-engine.md`,
   `docs/architecture.md` §Storage) for the spec-silent internals (flag
   those: "no openEHR spec governs this — our own design").

## The method (identical to W-3c/W-3e — proven twice)

1. **Register first.** Create `docs/design/platform/` (mirror
   `docs/design/its-rest/`): audit the crate area-by-area against the
   Architecture Overview chapters — one document per area (versioning +
   integrity, storage/node-codec, service/<sm-chapter>, aql, validation,
   templates, terminology, system_log, extensions/enterprise) with verified
   file:line state, cited G-row gap register, target design, PORT-NOTE
   residue. Fan out read-only Opus auditors in parallel (one per area, each
   handed its spec paths; they write one register file each; template:
   `docs/design/sm-platform/10-subject-proxy.md`).
2. **Then the big-bang rewrite** (owner rulings, all standing):
   - Chapter/area at a time: read the spec, create the folder, author the
     files fresh — **never migrate legacy**; audited-faithful logic may be
     carried into the fresh structure but every file is re-grounded and
     re-verified.
   - **Intermediate steps need not compile.** Rewrite everything first;
     ONE fix pass at the end (never fix in between — no circles).
   - Fan implementation out to parallel Opus workers with disjoint file
     ownership, each on its own build lane
     (`CARGO_TARGET_DIR=$PWD/target/agent-t1..t4`); cross-folder needs
     become `// TODO(w3f-integrate): …` markers.
   - **Zero-TODO mandate:** the phase does not close while any
     `TODO(w3f-*)` (or other actionable TODO) remains — a final inventoried
     elimination sweep implements every marker. Spec-text TODO quotes and
     cited PORT NOTEs are records, not debt.
   - `dead_code = "deny"` and `clippy::todo = "deny"` are already
     workspace-enforced — delete or wire, never allow.
   - Spec citations ONLY in code (file + section), never ADR numbers;
     spec-silent design gets the explicit flag.
   - No `use X as Y` renaming; `urlencoding` for all percent-coding; the
     official CLIs for tool-managed artifacts (`sqlx migrate add` etc.).

## Target structure (the register refines this; directional)

```
app/ehrbase/src/
├── main.rs / lib.rs      binary + crate map
├── db/                   pool, migrators (exists — re-ground docs)
├── storage/              the node codec + decomposed node model
│                         (ADR-008-era internals; spec-silent, flagged;
│                         split out of the current service/vobject tangle)
├── versioning/           change control per RM common + Architecture
│                         Overview §Versioning/§Integrity: VERSIONED_OBJECT
│                         lifecycle, CONTRIBUTION commits, audits,
│                         attestations, AND digital signature (the current
│                         signing/ module dissolves here per §Integrity)
├── service/              one folder per SM chapter, mirroring ehrbase-sm:
│   ├── ehr/  demographic/  ehr_index/  definition/  query/  message/
│   ├── subject_proxy/ (exists — pattern donor)  terminology/  admin/
│   └── validity/
├── aql/                  the engine (exists; re-ground + split sql.rs)
├── validation/           composition/OPT validation (opt_validation.rs +
│                         adl2_validation.rs split along AM boundaries)
├── templates/            OPT ingestion + WebTemplate cache
├── system_log/           ATNA emitter (exists)
└── extensions/           enterprise, spec-silent, quarantined + flagged:
    events, fhir connector, multimedia/S3, tenancy, ehr_access cache
```

## Constraints & context

- **The schema is settled** — `0001_baseline.sql` changes only if a
  register G-row demands it (schema comments cite specs only).
- The `Platform` trait surface (`ehrbase-sm`) is the fixed contract; the
  rewrite reorganises the implementation, it does not change the seam
  (impl-side G-rows from `docs/design/sm-platform/` may add behaviour).
- Tests: the integration suites under `app/ehrbase/tests/` are the safety
  net — they must all pass at close (update only where a register row
  changes spec behaviour, with the citation; never weaken).
- **Debt owed from the W-3c/W-3e merge** (checks skipped at merge by owner
  ruling — settle them early in this phase): full workspace
  `cargo nextest run --workspace` triage (stale pre-rewrite expectations:
  weak `W/` ETags, Location-on-201-only, admin 204s — update those
  assertions spec-correctly), workspace clippy under the deny rules, and a
  full ECC run (`scripts/conformance.sh`, zero drift vs the 341/315/0
  baseline; the newly-adjudicated admin delete-all cases + any wire deltas
  must be re-baselined honestly).
- Close per the standing loop: tick `docs/plans/w3f-platform-redesign.md`
  checkboxes (author it from this prompt at start), changelog + website
  book same-PR rules, WORKLIST row closed with the merged PR, then
  PR → merge → next row.

## Exit criteria

- [ ] `docs/design/platform/` register complete (every area audited,
      G-rows cited).
- [ ] Every `src/` area maps to its Architecture-Overview/SM oracle (or
      sits in `extensions/` flagged); signing dissolved into versioning
      per §Integrity; no file > ~700 lines without a documented reason.
- [ ] Every register G-row closed in code or a re-verified cited PORT NOTE.
- [ ] Zero actionable TODO markers; dead-code/todo denies green.
- [ ] Workspace build + full `nextest` + clippy green; **ECC zero-drift**
      (the owed run included); changelog + book updated.
