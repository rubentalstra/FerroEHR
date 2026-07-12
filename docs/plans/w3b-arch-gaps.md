# W-3b — Architecture Overview gap closure (W-6 … W-9)

**Owner directive (2026-07-12):** fix and implement properly ALL the gaps the
W-3a Architecture Overview checklist surfaced
(`docs/spec-audit/architecture-overview/CHECKLIST.md`, gap → worklist table).
Properly = implemented against the vendored spec text, not PORT-NOTEd away —
a PORT NOTE remains acceptable only where the specs are genuinely silent and
a design decision is recorded with its reason.

**Greenfield rule (owner, 2026-07-12, this phase):** this is a greenfield
application — complete rewrites are the expected shape, never quick fixes.
Schema changes go into the re-authored baseline
(`app/ehrbase/migrations/ehr/0001_baseline.sql` — nothing is deployed), not
appended patch migrations.

Standing rules apply in full (blueprint §4): spec text first (`/spec-lookup`),
spec-only citations in code, compiling tested increments, zero-drift ECC at
close, checklist/worklist updated in the same commits.

## Scope (one task block per gap cluster)

### T1 — W-6: `EHR.folders` multiple hierarchies
Checklist row 6.3. RM defines `EHR.folders` alongside `directory`; the server
currently hard-enforces a single root FOLDER
(`app/ehrbase/src/service/directory.rs:25`,
`app/ehrbase/src/service/contribution.rs:481`).

- [x] Spec extraction (RM EHR IM `EHR.folders`/`directory` + invariants, RM
      common versioned folders, ITS-REST directory API, CNF folder cases)
      reviewed and recorded in this file's notes. *(Notes: `EHR.folders:
      List<OBJECT_REF> 0..1`, every member a `VERSIONED_FOLDER` ref
      (`Folders_valid`); `directory` = `folders[1]` (`Directory_in_folders`,
      RM UML `org.openehr.rm.ehr.ehr.adoc` §EHR Class); each hierarchy is
      its own versioned object, "an entirely new Folder hierarchy may be
      added … referenced by a new member of `EHR._folders_`", CONTRIBUTION-
      wrapped (RM ehr master04 §Folders); FOLDER holds refs only, multiple
      refs to one target allowed (RM common master05 §Overview). ITS-REST
      binds only `/ehr/{ehr_id}/directory` = `folders[1]`; SM
      `I_EHR_DIRECTORY.create_directory` precondition `Pre_no_directory`
      caps the *directory* at one; CNF tests single directory only —
      wire/SM/CNF silent on hierarchies 2..n, so the surface for them is
      the CONTRIBUTION commit path.)*
- [ ] Storage redesign: model multiple root FOLDER hierarchies per EHR in
      the baseline schema (re-author `0001_baseline.sql`; distinguish the
      `directory` hierarchy from additional named hierarchies).
- [ ] Service rewrite: create/update/delete of any folder hierarchy via
      CONTRIBUTION; `EHR.folders` populated correctly on EHR reads;
      directory semantics preserved exactly.
- [ ] Wire: ITS-REST `/directory` unchanged; decide + implement the surface
      for additional hierarchies (contribution commits at minimum; explicit
      spec-silence flag for anything beyond ITS-REST).
- [ ] Tests: multi-hierarchy round-trip, per-hierarchy versioning, logical
      delete, `EHR.folders` refs; ECC DirectoryOps/ChangeSets zero-drift.
- [ ] Checklist row 6.3 → verified; W-6 closed.

### T2 — W-7: AQL archetype-specialisation subsumption
Checklist row 10.2.2. A parent-archetype predicate must match data created
with specialised archetypes (Architecture Overview master10 §Design-time
Relationships); AQL matching today is exact equality
(`app/ehrbase/src/aql/sql.rs:632`).

- [ ] Spec extraction (AM Archetype Identification HRID structure +
      reference-matching semantics, AQL 1.1 archetype predicate, CNF query
      cases) reviewed and recorded.
- [ ] Matching rule designed in-session and recorded with spec citations
      (concept-segment `-` boundary semantics, version-part semantics,
      conflicts between QUERY silence and the Architecture Overview mandate
      resolved explicitly).
- [ ] Engine implementation (predicate lowering + SQL generation over the
      node table), including whatever storage support does it properly
      (promoted columns/indexes in the re-authored baseline if pattern
      matching needs them).
- [ ] Tests: parent matches 1- and 2-level specialised data; sibling
      concepts do not match; version-part behaviour; ECC AqlBasic
      zero-drift.
- [ ] Checklist row 10.2.2 → verified; W-7 closed.

### T3 — W-8: paths/locators tail
Checklist rows 11.2.1, 11.2.4.3, 11.3.1: `//` path patterns, positional
`[n]` predicates, `ehr:` URI grammar + resolution beyond `Scheme_valid`.

- [ ] Spec extraction (normative home of the path grammar, DV_EHR_URI /
      LOCATABLE_REF obligations, PATHABLE path-function semantics, AQL 1.1
      grammar boundaries, CNF coverage) reviewed and recorded.
- [ ] Generic RM path resolver: implement `//` patterns and positional
      predicates where locator resolution lives (PATHABLE functions /
      `crates/openehr-rm`), not in the AQL grammar (AQL 1.1 defines
      neither — the typed reject there stays, documented).
- [ ] `ehr:` URI: typed parser for the full URI model (system_id / ehr_id /
      top-level structure locator / path) + local resolution in the service
      layer (LINK targets, extract OBJECT_REF use).
- [ ] Tests: pattern/positional resolution over the corpus; `ehr:` URI
      parse/format round-trip; resolution against a live store.
- [ ] Checklist rows → verified; W-8 closed.

### T4 — W-9: EHR_ACCESS realization
Checklist rows 5.5.1.5, 7.3.2.2, 7.4.1. EHR_ACCESS is stored/versioned but
never consulted; no access list / gate-keeper / privacy levels. openEHR
publishes no concrete `ACCESS_CONTROL_SETTINGS` scheme.

- [ ] Spec extraction (RM `EHR_ACCESS`/`ACCESS_CONTROL_SETTINGS`, SM
      authorization placement, ITS-REST security, CNF SEC cases) reviewed
      and recorded.
- [ ] Scheme design (in-session, recorded in `docs/design/`): a concrete
      access-control scheme as our own explicitly-flagged extension
      (access list + gate-keeper + per-Composition privacy levels with a
      jurisdiction-configurable vocabulary), scheme identity recorded in
      the EHR_ACCESS object; schema support re-authored into the baseline
      if needed.
- [ ] Implementation: per-EHR evaluation in the `ehrbase-rest` access layer
      (after authn, before dispatch), gate-keeper rule on EHR_ACCESS
      writes, privacy-level filtering on composition reads; default-open
      when no settings exist (compatibility with anonymous/default EHRs).
- [ ] Tests: gate-keeper enforcement, allow/deny, privacy filtering,
      default-open compatibility; ECC SEC zero-drift.
- [ ] Checklist rows 5.5.1.5 / 7.3.2.2 / 7.4.1 → verified (as flagged
      extension); W-9 closed.

## Exit criteria

- [ ] All four gap clusters implemented; checklist rows re-verdicted with
      file evidence; W-6..W-9 closed in `WORKLIST.md`.
- [ ] `cargo nextest run --workspace` green; clippy clean; fmt clean.
- [ ] ECC run zero-drift vs the 341/315/0 baseline (only newly-green
      deltas allowed); report re-baselined.
- [ ] Changelog entry + docs-website pages for the user-visible surface
      (folders, AQL matching, `ehr:` URIs, access evaluation).
