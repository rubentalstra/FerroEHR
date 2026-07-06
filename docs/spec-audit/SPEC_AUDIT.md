# Full openEHR Spec Audit — 2026-07-06

Whole-codebase audit against the vendored openEHR specifications
(`docs/specs/openehr/` — RM 1.2.0, BASE 1.3.0, AM 1.4/2.4, QUERY 1.1, TERM 3.1.0,
ITS-REST 1.0.3, ITS-JSON, ITS-XML, SM, CNF test schedule). **The openEHR spec is
the sole authority** — divergences that merely mirror EHRbase/Archie/Better
behaviour are findings, not excuses (ADR-008).

Branch: `claude/spec-audit-full`.

## How to use this document

- Each audit area has a findings file under `docs/spec-audit/findings/` with
  numbered findings (`F-AA-NN`), each carrying a severity, an exact spec
  citation, a code location, and a `- [ ] fixed` checkbox.
- Fix work happens in waves (below). Tick the checkbox in the findings file
  when a finding is resolved; update the counts table here.
- Preference: **clean full rewrites over patches** where the structure is wrong
  (owner directive, 2026-07-06).

## Areas

| #  | Area                                               | Findings file                                                                                                      | Status  | crit | major | minor | info |
|----|----------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|---------|------|-------|-------|------|
| 01 | REST: EHR / EHR_STATUS / VERSIONED_EHR_STATUS      | [findings/01-rest-ehr.md](findings/01-rest-ehr.md)                                                                 | audited | 1    | 7     | 2     | 1    |
| 02 | REST: COMPOSITION / DIRECTORY / CONTRIBUTION       | [findings/02-rest-composition-directory-contribution.md](findings/02-rest-composition-directory-contribution.md)   | audited | 1    | 6     | 5     | 1    |
| 03 | REST: QUERY / DEFINITION / ITEM_TAG / auth         | [findings/03-rest-query-definition-tags.md](findings/03-rest-query-definition-tags.md)                             | audited | 0    | 9     | 7     | 4    |
| 04 | Canonical JSON (ITS-JSON)                          | [findings/04-canonical-json.md](findings/04-canonical-json.md)                                                     | audited | 0    | 1     | 2     | 3    |
| 05 | Canonical XML (ITS-XML)                            | [findings/05-canonical-xml.md](findings/05-canonical-xml.md)                                                       | audited | 1    | 2     | 4     | 4    |
| 06 | Versioning / CONTRIBUTION / AUDIT (change_control) | [findings/06-versioning-contribution.md](findings/06-versioning-contribution.md)                                   | audited | 3    | 4     | 4     | 3    |
| 07 | Composition validation (invariants + AOM + TERM)   | [findings/07-validation.md](findings/07-validation.md)                                                             | audited | 1    | 3     | 8     | 3    |
| 08 | AQL 1.1 lexer/parser/AST                           | [findings/08-aql-parser.md](findings/08-aql-parser.md)                                                             | audited | 0    | 3     | 8     | 3    |
| 09 | Templates: OPT 1.4 / AOM 1.4                       | [findings/09-templates-opt14.md](findings/09-templates-opt14.md)                                                   | audited | 1    | 1     | 6     | 0    |
| 10 | WebTemplate / FLAT / STRUCTURED (SDT)              | [findings/10-webtemplate-flat-sdt.md](findings/10-webtemplate-flat-sdt.md)                                         | audited | 0    | 1     | 8     | 6    |
| 11 | Terminology (TERM 3.1.0)                           | [findings/11-terminology.md](findings/11-terminology.md)                                                           | audited | 0    | 4     | 3     | 2    |
| 12 | RM/BASE types + spec functions/invariants          | [findings/12-rm-base-types.md](findings/12-rm-base-types.md)                                                       | audited | 0    | 5     | 6     | 3    |
| 13 | Architecture / duplication / hygiene               | [findings/13-architecture-hygiene.md](findings/13-architecture-hygiene.md)                                         | audited | 0    | 13    | 13    | 4    |
| **Σ** |                                                 |                                                                                                                      |         | **8** | **59** | **76** | **37** |

Total: **180 findings**.

## Triage summary — cross-cutting themes

1. **No response-header channel (structural).** The `Value`-only backend seam and
   `negotiate.rs` builders cannot carry `ETag`/`Location`/`Last-Modified`, and
   `Prefer` is parsed but never honored (spec default `return=minimal` / 204
   ignored). Breaks optimistic concurrency + CNF header assertions across every
   endpoint group. → needs a typed response-envelope rewrite, not a patch.
   (F-01-01/02, F-02-02/03, F-03-04, F-09-04)
2. **Coded values rendered as rubrics.** `AUDIT_DETAILS.change_type.
   defining_code.code_string` emits `"creation"` instead of `"249"` etc.;
   `lifecycle_state` collapsed to a bool and hardcoded `532|complete|` even for
   deleted versions (must be `523|deleted|`). Independently found by areas
   01/02/06/11. (F-06-02/04, F-11-01, F-01-06, F-02-06/07)
3. **VERSION render edge misses mandatory RM fields.** `commit_audit` (1..1),
   `preceding_version_uid`, `VERSIONED_OBJECT.time_created`, `EHR.ehr_access`
   never emitted. Storage has the data; the render layer drops it.
   (F-06-01/03/05/07, F-01-07/08)
4. **Blind upserts erase 409 semantics.** Template upload and stored-query
   store use `ON CONFLICT DO UPDATE` → silently overwrite immutable resources
   where the spec + CNF mandate `409 Conflict`. (F-09-01, F-03-02/03)
5. **Deleted-composition reads → 500.** Logical delete writes zero nodes; every
   read path reassembles before checking `deleted`, so the spec's 204 never
   happens and the guards are dead code. (F-02-01, F-02-05)
6. **Validation bypass paths.** The CONTRIBUTION commit path never invokes the
   validator at all; compositions without `template_id` skip even the
   unconditional RM-invariant + terminology passes; only 7 of ~17 coded slots
   terminology-checked. (F-07-01/02/03, F-11-02..04)
7. **Canonical XML XSD-closure gap.** 16 LOCATABLE subtypes (incl. actively
   served EHR_STATUS) emit `archetype_node_id` as element instead of attribute;
   ToXml writes the XSD element set while FromXml reads the BMM set (silent
   field drops); C14N parity gate un-wired. (F-05-01/02/03)
8. **JSON polymorphic ingestion too lenient.** Untagged-enum fallback silently
   mis-types `_type`-less payloads (DV_TIME → DV_DATE); unknown keys silently
   dropped. Needs an explicit `_type` dispatcher in the derive. (F-04-01/02/03)
9. **AQL front-end gaps.** Hyphenated terminology ids un-lexable
   (`SNOMED-CT::…`), `VERSION[<path> op <val>]` predicate unimplemented, i64
   overflow silently parses as `0`; corpus harness only exercises 4 queries.
   (F-08-01/02/03 + hygiene)
10. **Duplication (the "everything double" problem).** `OBJECT_VERSION_ID`
    parsing reimplemented 5× with divergent splitting; two independent
    WebTemplate caches with a REST→backend→re-parse layering inversion; three
    AQL-path parsers; opt14 re-emits the C_* tree beside am14; ~500 LoC of dead
    501 dispatch arms; `ehrbase-quirks` feature gates nothing.
    (F-13-01/02/03/20/21/25, F-09-02)
11. **RM behaviour layer thin.** Spec functions essentially unimplemented
    (`item_at_path`, DV_ORDERED magnitude/comparison, OBJECT_VERSION_ID
    accessors — hence the 5× hand-parsing), EVENT invariants unvalidated.
    (F-12-01..05)

## Fix waves

### Wave 1 — critical spec divergences (wire-visible, CNF-failing)
- [ ] W1-A **Version/audit render + coded values** — F-06-01, F-06-02, F-06-03,
      F-06-05, F-01-06/07/08, F-02-06/07, F-11-01: emit `commit_audit`,
      `preceding_version_uid`, `time_created`; numeric change_type codes;
      real `lifecycle_state` incl. `523|deleted|`.
- [ ] W1-B **Deleted-composition read path** — F-02-01, F-02-05: deleted →
      204-style outcome (no reassemble-before-check), delete honors
      `preceding_version_uid` (409 stale / 400 already-deleted).
- [ ] W1-C **409 conflict semantics** — F-09-01, F-03-02, F-03-03: replace
      upserts with insert-or-conflict for template upload + stored queries.
- [ ] W1-D **Validation gate everywhere** — F-07-01, F-07-02: CONTRIBUTION
      path validates; templateless compositions still get RM + terminology
      passes.
- [ ] W1-E **XML `archetype_node_id` attribute closure** — F-05-01: extend the
      emitter's XSD closure (v2 `Ehr.xsd` is vendored) so EHR_STATUS etc. emit
      valid canonical XML.

### Wave 2 — major divergences + structural rewrites (full rewrites preferred)
- [ ] W2-A **Typed response envelope** (headers channel): ETag/Location/
      Last-Modified + `Prefer` handling across all groups — F-01-01/02/03,
      F-02-02/03, F-03-01/04, F-09-04. Backend seam returns typed results, not
      bare `Value`.
- [ ] W2-B **`ObjectVersionId`/UID value types in BASE `*_impl.rs`** —
      F-12-03 + F-13-01: one parser/accessor set; delete the 5 hand-rolled
      copies.
- [ ] W2-C **Lifecycle state machine** — F-06-04: real `lifecycle_state`
      column/enum (complete/incomplete/deleted/abandoned/inactive), change-kind
      fidelity (amendment vs modification) — F-06-06.
- [ ] W2-D **Explicit `_type` dispatcher** in `openehr-derive` for abstract
      slots — F-04-01/02/03.
- [ ] W2-E **AQL front-end fixes** — F-08-01/02/03 + corpus-harness expansion.
- [ ] W2-F **ITEM_TAG conformance** — F-03-05/06: PUT = full replace,
      OBJECT_REF-shaped response.
- [ ] W2-G **Stored-query semver/name matching** — F-03-07/08.
- [ ] W2-H **Terminology validation completeness** — F-07-03/04, F-11-02/03/04.
- [ ] W2-I **XML ToXml/FromXml field-set symmetry + C14N gate** — F-05-02/03.
- [ ] W2-J **EHR_ACCESS + duplicate-subject 409** — F-06-07, F-01-04;
      version-at-time ops implemented (F-01-05, F-02-04).
- [ ] W2-K **WebTemplate single-source resolution** — F-13-02 (one cache,
      service-owned); WebTemplate required-fields vs ITS-REST schema —
      F-10-03/04.
- [ ] W2-L **RM spec functions layer** — F-12-01/02/04/05 (paths, magnitude,
      comparison, EVENT invariants) — feeds P16 AQL.

### Wave 3 — minor divergences, hygiene, consolidation
- [ ] W3-A One AQL-path module (delete the 3 parsers) — F-13-20/21, F-13-50..52.
- [ ] W3-B Generic NotImplemented dispatcher (~500 LoC gone) — F-13-03.
- [ ] W3-C `ehrbase-quirks` feature actually gates Better-isms — F-13-25.
- [ ] W3-D opt14 ↔ am14 constraint-model consolidation or ADR — F-09-02.
- [ ] W3-E FLAT context fabricated codes (`openehr::0`) — F-10-07; hardcoded
      `rm_version 1.0.4` — F-10-09.
- [ ] W3-F Remaining minors per findings files.

## Progress log

- 2026-07-06: audit executed (13 parallel area audits); 180 findings; this
  document + findings committed on `claude/spec-audit-full`.
