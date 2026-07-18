# DIR — DIRECTORY full spec audit + total rewrite

Owner (2026-07-18): "currently it's almost nothing" + in-session: a TOTAL
rewrite, fully spec-first. Oracles read first-hand: the five ITS-REST
directory operations (`docs/specs/openehr/ITS-REST/specifications/operations/
directory_*.yaml` + responses/parameters/headers + `docs/overview/
Requests_and_responses.md`), RM common master05 directory package + the
FOLDER/VERSIONED_FOLDER/EHR class tables (`docs/specs/openehr/RM/docs/UML/
classes/`), and CNF master09 (`func_tc_ehr_directory.adoc`). This file is
deleted in the PR that lands the rewrite.

## Audit verdict (spec ↔ code diff)

The CDR already implements all five REST operations end-to-end (service
`app/ehrbase/src/service/ehr/directory.rs`, REST
`app/ehrbase-rest/src/api/ehr/directory.rs`, storage via the generic
`node`/`vo_version` versioning + the `ehr_folder` rank index), with ECC
covering 37 `dir/*` cases. The genuine gaps:

### CDR gaps (fix)

- [ ] **G1 — `?path=` dropped on `GET /directory/{version_uid}`**: the spec
      operation and the generated contract both define it; the handler and
      `get_directory_at_version` ignore it. Wire the subtree selection into
      the version read (same `select_subfolder` semantics).
- [x] **G2 — `Prefer: return=identifier`**: already implemented centrally
      (`negotiate::write_rm` → `identifier_response`); pinned by G6 tests.
- [ ] **G3 — root-FOLDER `uid` population** (RM FOLDER class NOTE, strongly
      recommended): stamp the tree-root FOLDER's `uid` with the enclosing
      VERSION's `OBJECT_VERSION_ID` at commit (create + update), so every
      read returns an identified root. (Superset-safe for ECC fidelity.)
- [x] **G4 — `Last-Modified`**: already emitted centrally
      (`negotiate::set_versioning_headers`); pinned by G6 tests.
- [x] **G5 — malformed `version_at_time` → 400**: already implemented
      (`parse_at_time` → typed SmError → 400); pinned by G6 tests.
- [ ] **G6 — REST wire-test battery**: only one directory wire test exists
      (malformed If-Match). New `app/ehrbase-rest/tests/directory_http.rs`
      covering all 5 ops: status ladders (201/200/204/400/404/409/412),
      ETag/Location on create/update, 412 carrying the latest version
      ETag, Prefer minimal/representation/identifier, `?path=` on both
      GETs (root `/`, nested, missing → 404), `version_at_time` (absent →
      latest, between versions → v1, before creation → 404, deleted → 204,
      malformed → 400), XML Accept round-trip, committal headers accepted.
- [ ] **G7 — ECC directory suite extension** (our own instrument):
      `dir/get-at-version-path`, `dir/create-prefer-identifier`,
      `dir/update-prefer-identifier`, `dir/get-at-time-malformed-time`.
      Baseline ratchets upward; zero drift elsewhere.

### Repo-wide wire compliance (owner directive 2026-07-18: the goal is a
### purely spec-compliant CDR — audited against the overview text, VERIFIED
### ALREADY COMPLIANT in `ehrbase-rest::overview::negotiate`, now PINNED)

The central `negotiate` module already implements the modern wire rules:
weak `W/"{uid}"` ETags (`resource_etag`), `Last-Modified` from the commit
time (`set_versioning_headers`/`http_date`), the full
minimal/identifier/representation `Prefer` triad incl. the `{uid}`
identifier body (`write_rm`/`identifier_response`), `Preference-Applied`,
and Location-on-creation-only. G2/G4 from the gap list below are therefore
already implemented — the remaining work is to PIN them for the directory
surface with wire tests (G6) and ECC cases (G7) so they can never
regress silently.

### Conformant as-is (audited, keep + note)

- Create-on-existing-directory → **409** (CNF E.2 requires an error; the
  REST operation is silent on the code — 409 flagged as our choice).
- Update/delete with EHR-but-no-directory → the 404/412 ladder (CNF
  H.2/I.1 require an error; spec silent on the code).
- Deleted version read by `version_uid` → **204** (spec leaves it
  undefined; we mirror the at-time 204).
- 404-for-empty-directory GETs (CNF NOTES: an error status is conformant).
- Duplicate sibling folder names ACCEPTED (RM has no uniqueness
  invariant — the master05 "uniqueness modifier" is a path convention);
  `items[].type` unrestricted (RM silent; any restriction would be our
  own design — none imposed).
- ETag form stays repo-consistent (the dev-branch OAS says weak `W/` MUST
  with plain form deprecated-but-MAY; the whole CDR emits the plain form
  uniformly — changing it is a repo-wide wire decision outside this row;
  noted, not changed).
- No `/versioned_directory` REST route exists in the spec (CNF §L is
  realized via `directory/{version_uid}`; predicates via the GETs) — no
  extension invented.

### Console — the complete directory experience (total rewrite of the tab)

- [ ] **U1 — structured tree editor** (no more raw-JSON-only): add
      subfolder, rename folder, remove folder at any node; add/remove
      OBJECT_REF items with a composition picker (the EHR's compositions
      listed; namespace/type prefilled, `local`/`COMPOSITION` default —
      RM-silent, our convention); details left untouched (advanced JSON
      editor stays available as a secondary mode).
- [ ] **U2 — version history + at-time**: versions enumerated over pure
      ITS-REST (derive `vo_id::system` from the current `version_uid`,
      walk trunk versions 1..N via `GET /directory/{version_uid}`), view
      any version's tree, plus a `version_at_time` datetime picker and a
      `?path=` subtree query box.
- [ ] **U3 — delete directory** (DELETE with If-Match + confirmation
      dialog), with the deleted state rendered as a first-class empty
      state (recreate flow available).
- [ ] **U4 — icondata only** (owner hard rule): LuFolder/LuFolderOpen tree,
      LuFileText item refs, LuPlus/LuPencil/LuTrash/LuHistory/LuClock
      actions — zero glyph/emoji text.
- [ ] **U5 — e2e journeys** for edit/history/delete in the composed
      battery + book screenshots recaptured; `/ui-gates` green.

### Docs / governance

- [ ] utoipa `#[utoipa::path]` updated for every wire-visible change (the
      native served OpenAPI is the only served document).
- [ ] `docs/endpoint-map.md` directory rows updated.
- [ ] Website book: REST directory page + admin-console chapter updated in
      the same PR; CHANGELOG `[Unreleased]` entries (user-visible wire +
      console changes).
- [ ] Full gates: workspace clippy/nextest/fmt, `/ui-gates`, composed
      ui-e2e battery, ECC zero-drift-plus-ratchet, `/phase-done` (worklist
      close, this file deleted).
