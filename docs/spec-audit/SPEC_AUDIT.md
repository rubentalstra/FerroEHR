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
- [x] W1-A **Version/audit render + coded values** — F-06-01, F-06-02, F-06-03,
      F-06-05, F-01-06/07/08, F-02-06/07, F-11-01: emit `commit_audit`,
      `preceding_version_uid`, `time_created`; numeric change_type codes;
      real `lifecycle_state` incl. `523|deleted|`.
- [x] W1-B **Deleted-composition read path** — F-02-01, F-02-05: deleted →
      204-style outcome (no reassemble-before-check), delete honors
      `preceding_version_uid` (409 stale / 400 already-deleted).
- [x] W1-C **409 conflict semantics** — F-09-01, F-03-02, F-03-03: replace
      upserts with insert-or-conflict for template upload + stored queries.
- [x] W1-D **Validation gate everywhere** — F-07-01, F-07-02: CONTRIBUTION
      path validates; templateless compositions still get RM + terminology
      passes.
- [x] W1-E **XML `archetype_node_id` attribute closure** — F-05-01: extend the
      emitter's XSD closure (v2 `Ehr.xsd` is vendored) so EHR_STATUS etc. emit
      valid canonical XML.

### Wave 2 — major divergences + structural rewrites (full rewrites preferred)
- [x] W2-A **Typed response envelope** (headers channel): ETag/Location/
      Last-Modified + `Prefer` handling across all groups — F-01-01/02/03,
      F-02-02/03, F-03-01/04, F-09-04. Backend seam returns typed results, not
      bare `Value`.
- [x] W2-B **`ObjectVersionId`/UID value types in BASE `*_impl.rs`** —
      F-12-03 + F-13-01: one parser/accessor set; delete the 5 hand-rolled
      copies. (BASE accessors/strict `FromStr` landed with W2-L; 2026-07-06 the
      app followed — all four hand-rolled `::` decoders deleted, one strict
      `ehrbase::service::version_id` module over
      `openehr_base::ObjectVersionId`; divergent splits resolved to the BASE
      3-part lexical form, branch ids rejected with a trunk-only error.)
- [x] W2-C **Lifecycle state machine** — F-06-04: real `lifecycle_state`
      column/enum (complete/incomplete/deleted/abandoned/inactive), change-kind
      fidelity (amendment vs modification) — F-06-06. (F-06-04 landed with
      Wave 1; 2026-07-06 F-06-06: full `audit_change_type` fidelity — client
      codes validated against the group and preserved verbatim
      (250/252/253/816/817 never narrowed to `modification`), spec-invalid
      combos → 422 (creation-on-existing, non-creation first version,
      deleted-with-data, attestation), contribution audit follows the spec
      aggregate rule; `service/codes.rs` is the single code⇄rubric home.)
- [x] W2-D **Explicit `_type` dispatcher** in `openehr-derive` for abstract
      slots — F-04-01/02/03.
- [x] W2-E **AQL front-end fixes** — F-08-01/02/03 + corpus-harness expansion.
- [x] W2-F **ITEM_TAG conformance** — F-03-05/06: PUT = full replace,
      OBJECT_REF-shaped response. (`replace_tags` full-collection replace incl.
      empty-list clear; OAS `ItemTag`-shaped wire form; RM key/value invariants
      → F-03-10 also closed.)
- [x] W2-G **Stored-query semver/name matching** — F-03-07/08. (SEMVER-prefix
      GET resolves to the highest match; LIST matches the qualified name as a
      prefix pattern, empty = wildcard.)
- [x] W2-H **Terminology validation completeness** — F-07-03/04, F-11-02/03/04.
- [x] W2-I **XML ToXml/FromXml field-set symmetry + C14N gate** — F-05-02/03.
      Emitter reconciles BMM↔XSD field sets (guard + cited allowlist; 44
      previously-dropped fields now appended, no silent drops); C14N byte-parity
      gate wired against CNF canonical fixtures (`xml_c14n.rs`).
- [x] W2-J **EHR_ACCESS + duplicate-subject 409** — F-06-07, F-01-04;
      version-at-time ops implemented (F-01-05, F-02-04). (Real versioned
      `EHR_ACCESS` created with the EHR in one CONTRIBUTION + `ehr_access` ref
      → F-01-10 also closed; DB-level subject uniqueness (`ehr_subject_uq`) →
      409; both `*_version_get_at_time` ops on the envelope seam with
      `200_VERSION_at_time` headers → F-02-13 also closed.)
- [x] W2-K **WebTemplate single-source resolution** — F-13-02 (one cache,
      service-owned); WebTemplate required-fields vs ITS-REST schema —
      F-10-03/04. (F-10-03/04 landed with the openehr-flat bundle; 2026-07-06
      F-13-02: new `WebTemplateService` seam on `Backend`, the service-owned
      moka cache is the only cache; `AppState.web_templates` + the REST
      OPT-XML re-fetch/re-parse path deleted — FLAT/STRUCTURED/`wt+json` now
      share the exact WebTemplate composition validation uses.)
- [x] W2-L **RM spec functions layer** — F-12-01/02/04/05 (paths, magnitude,
      comparison, EVENT invariants) — feeds P16 AQL.

### Wave 3 — minor divergences, hygiene, consolidation
- [x] W3-A One RM-path module — F-13-20/21, F-13-50/51/52. Both `openehr-flat`
      path parsers + their predicate-matching/navigation deleted and routed
      through the single `openehr_rm::paths` (BASE master11-paths) via a thin
      crate-local `openehr-flat/src/path.rs`; `openehr-rm` gained only
      spec-general surface (`Predicate::matches`/`is_empty` pub +
      `select_children`). Two validator leniencies tightened to the spec
      (predicate re-checked on single-valued attributes; `name/value` matches
      canonical `DV_TEXT` only). `openehr-query` hygiene: F-13-50 (VERSION/node
      standard-predicate arms) already landed in W2-E; F-13-51 double
      `path_parsers()` build collapsed to one; F-13-52 stale doc fixed (lossy
      numeric parse already fixed by W2-E). The `openehr-query` AQL *grammar*
      parser stays distinct from RM-instance navigation — unifying the two is
      P16 work (per F-12-02 note).
- [x] W3-B Generic NotImplemented dispatcher (~500 LoC gone) — F-13-03.
      (2026-07-06 — demographic/query/admin dispatch files deleted (546 LoC),
      one generic `not_implemented` dispatcher over the generated `ROUTES`
      tables; `Backend` slimmed to `EhrService + DefinitionApi +
      WebTemplateService` (each generated trait rejoins in the phase that
      implements it — query at P16); 501 wire body proven identical by an
      http.rs body-equality test per group.)
- [x] W3-C `ehrbase-quirks` feature actually gates Better-isms — F-13-25.
      (2026-07-06 — `|unit_system`/`|unit_display_name` emit + read-back gated
      behind `#[cfg(feature = "ehrbase-quirks")]`; `ehrbase-compat` enables the
      feature. Decision recorded: the RM 1.2.0 fields are genuine but their FLAT
      suffix form is a Better extra, so gating is correct.)
- [x] W3-D opt14 ↔ am14 constraint-model consolidation or ADR — F-09-02.
      (2026-07-06 — verdict: **not reconcilable**, keep both deliberately;
      **ADR-009** records the field-by-field divergence; drift-guard sentinel
      `openehr-its/tests/opt14_am14_divergence.rs` added. Also fixed the opt14
      minors: F-09-03 (T_CONSTRAINT generated — default_value overlays
      preserved), F-09-05 (IndexMap document order + 91-file parse→ToXml→
      re-parse round-trip gate), F-09-06/07 (PORT NOTEs). F-09-04/08 remain
      app-crate work.)
- [x] W3-E FLAT context fabricated codes (`openehr::0`) — F-10-07 (done earlier);
      hardcoded `rm_version 1.0.4` — F-10-09 (2026-07-06 — single
      `flat::defaults::RM_VERSION = "1.2.0"` constant tied to the RM pin).
- [ ] W3-F Remaining minors per findings files.

## Status snapshot (2026-07-06, end of first fix push)

**109 of 191 checkboxed findings fixed** (82 open — overwhelmingly minor/info
plus deliberately-deferred decision items). All 8 criticals and every major
scheduled in Waves 1-2 are closed; the Wave-3 structural consolidations
(paths, dispatcher, quirks gating, opt14 ADR) are done. Branch health:
**491/491 workspace tests pass, clippy 0 warnings, codegen drift green.**

Per area (fixed/total): 01: 9/11 - 02: 8/13 - 03: 8/20 - 04: 3/6 - 05: 3/11 -
06: 7/12 - 07: 4/15 - 08: 13/14 - 09: 6/8 - 10: 11/15 - 11: 5/8 - 12: 7/11 -
13: 9/30 - 14: 16/17. The larger deferred clusters: area 03 QUERY-execution
items (P16 scope), area 05 XML minors (interval flags, f32 runtime), area 07
spec-underdetermined AOM 1.4 decision points (need ADR + CNF fixtures),
area 13 openehr-flat builder-split refactors (deferred for regression risk).

## Progress log

- 2026-07-06: audit executed (13 parallel area audits); 180 findings; this
  document + findings committed on `claude/spec-audit-full`.
- 2026-07-06: area 14 added (ADR/documentation fact-check) — 17 findings,
  16 fixed same day (ADR status/amendment banners; living docs realigned;
  PROGRESS.md reconciled — actual current phase is P16).
- 2026-07-06: **Wave 1 complete** — all 8 criticals fixed:
  W1-A/B/D (service: commit_audit, preceding_version_uid, numeric
  change_type, lifecycle_state incl. 523|deleted|, deleted-read 204, delete
  preconditions, contribution + templateless validation), W1-C (409 on
  duplicate template/stored-query), W1-E (XML archetype_node_id attribute
  closure + codegen guard).
- 2026-07-06: Wave 2 progress — **W2-A done** (typed `ServiceResponse`/
  `ResourceMeta` envelope; ETag/Location per operation; Prefer
  minimal/representation; bare EHR_STATUS version GET), **W2-D done**
  (explicit `_type` dispatch for polymorphic slots), **W2-E done** (AQL
  lexer/parser conformance, tests 21→48). W2-L (RM/BASE spec functions) and
  W2-F/G/J (item tags, stored-query matching, EHR_ACCESS, duplicate-subject
  409, at-time ops) in flight.
- 2026-07-06: **W2-I done** (F-05-02/03) — emit-xml now reconciles the BMM and
  XSD field sets per type: a codegen guard fails on any XSD-covered BMM field
  with no XSD slot unless allowlisted with a cited RM-1.2.0/BASE-1.3.0-vs-
  ITS-XML skew reason, and allowlisted fields are appended as deterministic
  trailing canonical-XML elements. Surfaced + fixed **44 fields that ToXml was
  silently dropping** (workflow_id rename, DV_QUANTITY units_system/-display,
  ELEMENT.null_reason, ISM_TRANSITION.reason, FOLDER/FEEDER_AUDIT details,
  EHR.tags, BASE 1.3.0 resource-class growth, EhrExtract includes_*→include_*,
  VERSIONED_OBJECT base fields). C14N byte-parity gate wired
  (`openehr-its/tests/xml_c14n.rs`, `xmllint --noblanks --c14n`) against the CNF
  canonical-XML COMPOSITION fixtures: 4 fixtures byte-identical modulo the cited
  cabolabs verbose-`xsi:type` convention (minimal-subset verified), 0 real
  divergences. F-05-04/05 assessed (do not block; left with notes).
- 2026-07-06: **openehr-flat conformance + hygiene bundle** — W3-C
  (F-13-25 quirks gating + decision), W3-E (F-10-09 rm_version → single
  `RM_VERSION="1.2.0"` constant), and the W2-K F-10 part (F-10-03 root `Tree`
  required-fields via `serialize_root`; F-10-04 subset-note) landed, plus the
  F-10 minors: F-10-01/05 (Better-oracle + accepted-form-envelope docs), F-10-06
  (`|formatting` PORT NOTE), F-10-08/10 (round-trip-boundary doc listing
  non-surfaced RM attrs), F-10-11 (dropped dead `actions` arm + RM-model TODO),
  F-10-14 (`DEFAULT_TIME`/setting defaults → one `flat/defaults.rs`), and F-13-22
  partial (`context.rs` `code_phrase` routed through `graph.rs`). `openehr-flat`
  76/76 tests green in **both** feature configs, clippy+fmt clean; `ehrbase-compat`
  + workspace build green. Two golden WebTemplate snapshots gained the two
  spec-required empty root rubric maps (Tree.required) — spec-mandated additions,
  nothing weakened. Remaining openehr-flat hygiene (F-13-23/24/26-full/27/28 —
  larger tree-walk/fill-pass/builder rewrites) deferred with notes; F-10-02
  skipped (lives in `ehrbase-rest`, another agent's crate).
- 2026-07-06: **W3-A done** (F-13-20/21, F-13-50/51/52). The two byte-identical
  `openehr-flat` AQL-path parsers + their duplicated predicate-matching and
  RM-navigation are deleted and consolidated onto the single
  `openehr_rm::paths` implementation (F-12-02, BASE master11-paths) via a thin
  crate-local `openehr-flat/src/path.rs`. `openehr-rm` additions were minimal
  and spec-general only (`Predicate::matches`/`is_empty` made pub, a
  `select_children` per-step primitive). Two validator leniencies resolved to
  the spec (predicate now re-checked on single-valued attributes; `name/value`
  matches only the canonical `DV_TEXT` form) — both only affect non-canonical
  input. `openehr-query` hygiene: F-13-50 already fixed by W2-E; F-13-51
  duplicate `path_parsers()` build collapsed; F-13-52 stale doc corrected.
  `openehr-rm`/`openehr-flat`/`openehr-query` build + clippy + fmt clean, 279
  tests pass, workspace builds green.
- 2026-07-06: **W3-D done** (F-09-02 + opt14 minors F-09-03/05/06/07). Verdict
  from the field-by-field comparison: the XSD-shaped `opt14` and BMM-shaped
  `am14` constraint models are **not reconcilable** (disjoint domain-type sets,
  typed vs `Any` assumed values, `DV_ORDINAL` vs `ORDINAL` lists, `IntervalOf*`
  vs `Interval<T>`, OPT-only envelope types) — consolidation would be the lossy
  shortcut; both are kept deliberately per **ADR-009** (rationale duplicated in
  the `emit_opt.rs` header PORT NOTE) with a new drift-guard sentinel
  (`openehr-its/tests/opt14_am14_divergence.rs`: exhaustive matches over both
  models + pinned asymmetry inventory). Emitter changes (never hand-edits):
  `T_CONSTRAINT` generated (`Option<TConstraint>` with typed `default_value`
  overlays; `T_VIEW` stays the one documented opaque type; `rm_type_name`
  joined the lenient defaults for differential overlay children);
  `StringDictionaryItem` groups moved `BTreeMap` → order-preserving `IndexMap`
  (new `OrderedDict` target in `emit_xml.rs`; RM `Hash` path byte-identical);
  public `opt14::to_xml` added; PORT NOTEs for the verbatim `*_KIND` codes
  (F-09-06) and the `0..1` multiplicity fallback (F-09-07). New corpus gates:
  parse→`ToXml`→re-parse structural round-trip over all 91 `.opt` files,
  dictionary-order preservation on a non-alphabetical fixture, T_CONSTRAINT
  default-value assertion. `openehr-its` 21/21 + `openehr-flat` 76/76 green,
  regeneration idempotent, `emit-xml` output unchanged, clippy + fmt clean.
  F-09-04/08 left open (app-crate scope, W2-A stream).
- 2026-07-06: **W2-B, W2-C, W2-K, W3-B done** (F-13-01/02/03, F-06-06).
  App-side OBJECT_VERSION_ID handling consolidated onto the BASE value types:
  the four hand-rolled `::` splitters deleted, one strict
  `ehrbase::service::version_id` decoder over
  `openehr_base::ObjectVersionId::from_str` (malformed `::` shapes now
  rejected instead of mis-split; branch ids → explicit trunk-only error,
  PORT NOTE F-06-09). Change-kind fidelity: `contribution.rs::classify`
  validates every inbound `change_type` against the full `audit_change_type`
  group and preserves it verbatim (amendment/synthesis/unknown/restoration/
  format-conversion), rejecting spec-invalid combos per RM change_control
  §Contributions (creation-on-existing, non-creation first version,
  deleted-with-data, attestation) — `codes.rs` is the one code⇄rubric home
  and the contribution audit defaults to the spec aggregate rule. WebTemplate
  resolution single-sourced: new `WebTemplateService` on the `Backend` seam;
  the REST layer's second cache + DEFINITION-API re-fetch/re-parse deleted —
  FLAT/STRUCTURED/`wt+json` consume the service cache validation uses. The
  ~546 LoC of demographic/query/admin 501 dispatch arms collapsed to one
  generic `not_implemented` dispatcher (Backend slimmed to
  `EhrService + DefinitionApi + WebTemplateService`); 501 body equality
  pinned by test. ehrbase 53/53 (PG18 e2e incl. new amendment round-trip +
  invalid-combo tests), ehrbase-rest 72/72, workspace build + clippy + fmt
  clean.
