# Phase SM-2 — Definitions service completion

- Status: in-progress
- Started: 2026-07-09
- Consumes: ADR-010; design `docs/design/sm-platform/` (01 §5 Definitions
  digest, 07 §1.1 gaps, 09 §SM-2)
- Compile required: yes (compiling, tested increments)

## Spec oracle (read before each task — hard rule)

- `docs/specs/openehr/SM/docs/UML/classes/i_definition_adl14.adoc` —
  archetype + OPT calls (has/valid/upload/get/list/list_matching/delete +
  counts; upload replaces-if-exists and requires validity; errors
  `invalid_archetype`/`invalid_template`/`artefact_does_not_exist`/
  `invalid_id_pattern`)
- `docs/specs/openehr/SM/docs/UML/classes/i_definition_adl2.adoc` —
  AUTHORED_ARCHETYPE artefact family keyed by ARCHETYPE_HRID
- `docs/specs/openehr/SM/docs/UML/classes/i_definition_query.adoc` —
  has/valid/store/list/list_matching/delete_query + queries_count (+ the
  `store_query_set` spec TODO); PERL regex patterns; qualified names
  `<ns>::<name>` (ns absent ⇒ "misc"); formalism `a_type` w/ optional
  `::version` (absent ⇒ major "1")
- `docs/specs/openehr/SM/docs/openehr_platform/master04-definition_package.adoc`
  — the conventions chapter
- Wire oracle: ITS-REST DEFINITION group (`openehr-its` generated
  `DefinitionApi`) + CNF/ECC — **the DEFINITION wire must not drift** (the
  new calls are native-API; ITS-REST template/query routes stay untouched).
  Zero-drift ECC gate applies (baseline 211/318, `scripts/conformance.sh`).

## Tasks

- [ ] `DefinitionAdl14Service` trait (`ehrbase-sm::services::definition`):
      the full SM call set (archetypes + OPTs + counts), doc-cited per call;
      today's generated `DefinitionApi` remains the wire adapter seam —
      the native trait sits beside it in `Backend`
- [ ] ADL 1.4 archetype store: `archetype_store` table (migration via
      `sqlx migrate add --sequential`), upload (replace-if-exists +
      validity precondition), get/list/list_matching (PERL regex via the
      `regex`/`fancy-regex` workspace crates)/delete + `archetypes_count`;
      ADL text parsed for validity via `openehr-am` am14 (scope: parseable +
      identifier well-formed — record with PORT NOTE what "valid" covers)
- [ ] OPT completion on the existing `template_store`: `delete_opt`,
      `valid_opt` (expose the existing parse+validate), `list_matching_opts`
      (regex), `opts_count`; PORT NOTE the spec's `List<ARCHETYPE_ID>`
      return-type inconsistency on `list_matching_opts` (digest 01 §5.1) —
      we return template ids
- [ ] ADL2 (`DefinitionAdl2Service`): AUTHORED_ARCHETYPE ingest over
      `openehr-am::am24` + `adl2_artefact` store; artefact CRUD + typed
      listings (archetype/template/OPT2) + counts; the ITS-REST `adl2`
      routes stop returning 501 where the generated contract has them
      (CHECK the generated DefinitionApi adl2 ops + CNF before changing any
      wire status — zero drift rule)
- [ ] Stored queries: `valid_query` (parse via `openehr-query`; formalism
      handling per master04 — case-insensitive, optional `::version`,
      default major "1"), `delete_query` (pre has_query, post not
      has_query), `queries_count`; namespace default "misc" on qualified
      names; `store_query_set` designed + PORT NOTE (spec TODO)
- [ ] e2e tests (testcontainers PG18) per store; regex-matching +
      count + delete + replace-if-exists cases; SM pre/post-conditions as
      assertions
- [ ] ECC zero-drift run (`bash scripts/conformance.sh`, expect 211/318 or
      better) + workspace gates

## Exit criteria

- [ ] Workspace green (build, nextest, clippy-neutral, fmt)
- [ ] ECC run ≥ baseline 211/318, zero regressions
- [ ] Every new trait/method doc-comment cites its SM call
- [ ] Phase checkboxes ticked; PROGRESS updated at close

## Handoff

SM-1 merged (PR #31). Branch `claude/sm-phase-02-definitions` created off
develop (6cd187e39). Next action: read `i_definition_adl14.adoc` +
`master04-definition_package.adoc` in full, then the `DefinitionAdl14Service`
trait + archetype store.
