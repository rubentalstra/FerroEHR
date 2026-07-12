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
- [x] Storage redesign: `ehr_folder (ehr_id, rank, vo_id)` membership table
      re-authored into `0001_baseline.sql` (append-only 1-based ranks;
      directory = lowest-rank live hierarchy, `Directory_in_folders` by
      construction).
- [x] Service rewrite: FOLDER creation via CONTRIBUTION appends a hierarchy
      (`vobject::apply_change` + the import path `commit_import_scoped`
      both insert the membership row); directory endpoints resolve through
      the rank-ordered lookup (`directory_vo_opt`); `EHR.folders` +
      `EHR.directory` emitted on EHR reads (`ehr.rs`); extract-import
      folder-singleton rules lifted (`message.rs`).
- [x] Wire: ITS-REST `/directory` byte-identical (single-hierarchy 409/204
      semantics preserved); additional hierarchies via CONTRIBUTION only,
      spec-silence flagged in code.
- [x] Tests: multi-hierarchy rank order + independent versioning, logical
      delete drops from `folders`, second directory-create 409, persistence
      expected-tables updated (`service_ehr.rs`, `persistence.rs`) — scoped
      suites green; ECC zero-drift gate at phase close.
- [x] Checklist row 6.3 → verified; W-6 closed.

### T2 — W-7: AQL archetype-specialisation subsumption
Checklist row 10.2.2. A parent-archetype predicate must match data created
with specialised archetypes (Architecture Overview master10 §Design-time
Relationships); AQL matching today is exact equality
(`app/ehrbase/src/aql/sql.rs:632`).

- [x] Spec extraction (AM Archetype Identification HRID structure +
      reference-matching semantics, AQL 1.1 archetype predicate, CNF query
      cases) reviewed and recorded. *(Notes: data-form id grammar
      `qualified_rm_entity '.' domain_concept '.v' version_id`, specialisation
      = `-`-appended concept segments (BASE base_types master05 §Syntaxes);
      subsumption normative in BASE arch-overview master10 §Design-time
      Relationships + AM Identification master07 §Querying/§AQL Queries;
      matching set for data X: X, older minor/patch of X, specialisation
      parents of X (+ their older variants) — master07 §Supporting
      Archetype-based Querying; major-only predicate = interface reference
      (`.v1` matches any v1 release), major boundary hard (master07
      §Referencing, master04 line 69). Conflicts flagged: AQL 1.1 master03
      §Archetype predicate literally equates to `archetype_node_id = 'x'`
      string equality; AOM2 ids carry no lineage semantics in `-` (ADL2-era
      lineage must come from templates → W-4). No CNF coverage.)*
- [x] Matching rule designed in-session and recorded with spec citations:
      same `qualified_rm_entity` + same major (interface reference) +
      concept equal-or-`-`-child; QUERY-equality and AOM2-lineage conflicts
      recorded in one PORT NOTE (`app/ehrbase/src/aql/sql.rs`).
- [x] Engine implementation: promoted comparison-normalized columns
      `node.arch_entity`/`arch_concept`/`arch_major` + `text_pattern_ops`
      index in the re-authored baseline; codec populates via the shared
      `ArchetypeId` parser (+ new `major_version()` accessor in the
      hand-written `archetype_id_impl.rs`); `archetype_predicate` emits
      equality-or-`LIKE 'concept-%'` for full HRIDs, case-folded equality
      otherwise.
- [x] Tests: SQL-generation unit test + `archetype_specialisation_subsumption`
      integration test (parent matches both, child one, sibling/major none);
      persistence/codec suites green.
- [x] Checklist row 10.2.2 → verified; W-7 closed.

### T3 — W-8: paths/locators tail
Checklist rows 11.2.1, 11.2.4.3, 11.3.1: `//` path patterns, positional
`[n]` predicates, `ehr:` URI grammar + resolution beyond `Scheme_valid`.

- [x] Spec extraction (normative home of the path grammar, DV_EHR_URI /
      LOCATABLE_REF obligations, PATHABLE path-function semantics, AQL 1.1
      grammar boundaries, CNF coverage) reviewed and recorded. *(Notes: the
      path grammar's normative home is BASE arch-overview master11 (RM uri
      package + AQL both defer to it); `//` patterns + positional `[n]` are
      normative grammar there but conformance-gated nowhere; AQL 1.1
      enumerates exactly standard/archetype/node predicates — no `//`, no
      positional → AQL untouched. DV_EHR_URI/LOCATABLE_REF are validate-only
      for a CDR (no resolution obligation; master11 §EHR URIs name-resolution
      "ad hoc" clause) → local resolution is our flagged extension.
      PATHABLE contracts: item_at_path (pre path_unique) / items_at_path /
      path_exists / path_unique (RM UML pathable.adoc). CNF touches:
      DV_URI/DV_EHR_URI content validation (master17.7) + directory
      has_path FOLDER-name paths (master09) — the latter is a different,
      simpler path concept, not the Xpath grammar.)*
- [x] Generic RM path resolver: `crates/openehr-rm/src/paths.rs` extended
      with the full predicate set (at-code, archetype HRID, name/comma
      shortcut, uid, and-chains), `//` descendant patterns, and 1-based
      positional predicates (`[0]` rejected at parse); PATHABLE functions
      (`items_at_path`/`item_at_path`/`path_exists`/`path_unique`) per the
      RM contracts. AQL grammar untouched (AQL 1.1 defines neither — typed
      rejects stay).
- [x] `ehr:` URI: typed `EhrUri` parser (four absolute forms + relative;
      `VersionLocator` uid vs exact OBJECT_VERSION_ID) in `openehr-rm`;
      local resolution in `app/ehrbase/src/service/ehr_uri.rs`
      (spec-silence-flagged extension; foreign-system URIs typed NotFound;
      attribute locators `directory`/`folders` resolve via the rank-ordered
      directory lookup).
- [x] Tests: 28 path/URI unit tests in `openehr-rm` (incl. the CNF
      master17.7 DV_EHR_URI fixture forms) + live-store resolution
      integration test (`service_ehr.rs`
      `ehr_uri_resolves_local_structures_and_item_paths`) — green.
- [x] Checklist rows → verified; W-8 closed.

### T4 — W-9: EHR_ACCESS realization
Checklist rows 5.5.1.5, 7.3.2.2, 7.4.1. EHR_ACCESS is stored/versioned but
never consulted; no access list / gate-keeper / privacy levels. openEHR
publishes no concrete `ACCESS_CONTROL_SETTINGS` scheme.

- [x] Spec extraction (RM `EHR_ACCESS`/`ACCESS_CONTROL_SETTINGS`, SM
      authorization placement, ITS-REST security, CNF SEC cases) reviewed
      and recorded. *(Notes: EHR_ACCESS mandatory + versioned (EHR.ehr_access
      1..1, Ehr_access_valid; RM ehr master04 §EHR Access);
      ACCESS_CONTROL_SETTINGS abstract + empty, "currently implementation
      dependent" — NO published concrete scheme anywhere; the "all access
      decisions must consult it" clause (RM UML ehr_access.adoc) is
      unenforceable as written; SM master02 places authn/authz out of band;
      ITS-REST: no EHR_ACCESS endpoint, 401/403 discipline only; CNF tests
      no access control (authn Robot suite only). Any decision engine =
      our own flagged extension.)*
- [x] Scheme design (in-session, recorded in `docs/design/`): a concrete
      access-control scheme as our own explicitly-flagged extension
      (access list + gate-keeper + per-Composition privacy levels with a
      jurisdiction-configurable vocabulary), scheme identity recorded in
      the EHR_ACCESS object. *(→ `docs/design/ehr-access-scheme.md`:
      `ehrbase.access_control.v1` — settings `_type
      EHRBASE_ACCESS_CONTROL_V1`, default-open, access list with
      user:/role: principals, gate-keeper write rule, integer privacy
      levels with per-composition overrides; AQL-level privacy filtering
      explicitly out of v1 scope. No baseline schema change needed —
      settings live in the versioned EHR_ACCESS payload.)*
- [x] Implementation: `EhrAccessAdapter` (ehrbase-sm, flagged extension) →
      Platform impl with moka cache + commit-hook invalidation (ehrbase) →
      `EhrAccessGate` first in the pre-dispatch chain with RBAC/ABAC
      composing on top (ehrbase-rest); gate-keeper preflight on
      EHR_ACCESS-carrying contributions; per-Composition privacy ceilings;
      default-open; fail-closed 500 on settings-read errors.
- [x] Tests: 6 gate e2e tests + settings DB round-trip + engine unit tests;
      full `ehrbase-rest` suite 293/293 (no regressions); ECC gate at
      phase close.
- [x] Checklist rows 5.5.1.5 / 7.3.2.2 / 7.4.1 → verified (as flagged
      extension); W-9 closed.

## Exit criteria

- [ ] All four gap clusters implemented; checklist rows re-verdicted with
      file evidence; W-6..W-9 closed in `WORKLIST.md`.
- [ ] `cargo nextest run --workspace` green; clippy clean; fmt clean.
- [ ] ECC run zero-drift vs the 341/315/0 baseline (only newly-green
      deltas allowed); report re-baselined.
- [ ] Changelog entry + docs-website pages for the user-visible surface
      (folders, AQL matching, `ehr:` URIs, access evaluation).
