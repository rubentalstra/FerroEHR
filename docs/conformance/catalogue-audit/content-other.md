# Catalogue semantic audit — chapter CONTENT, slice B (non-slice-A families)

- Date: 2026-07-24 (issue #231, milestone v3.9.0)
- Scope: every YAML in `tools/cnf-runner/artifacts/schedule/content/` except the slice-A prefixes (`CONT-DV_INTERVAL_*`, `CONT-DV_PROPORTION*`, `CONT-DV_QUANTITY*`, `CONT-DV_DURATION*`, `CONT-DV_COUNT*`, `CONT-DV_ORDINAL*`, `CONT-DV_SCALE*`) — **37 cases**: COMPOSITION (2), DV_BOOLEAN (3), DV_CODED_TEXT (3), DV_TEXT (2), DV_IDENTIFIER (2), DV_DATE (3), DV_TIME (3), DV_DATE_TIME (3), DV_URI (3), DV_EHR_URI (3), DV_MULTIMEDIA (2), DV_PARSABLE (2), EVENT (2), HISTORY (2), ITEM_STRUCTURE (1), OBSERVATION (1).
- Verdict tally: **ok 22 · DEFECT 11 · AMBIGUITY 4**
- Method: every decision-table row recomputed against the vendored CNF schedule chapter for its family (master15, master16, master17.1/.2/.4/.6/.7), the RM UML class adocs (invariants + attribute multiplicities read first-hand), the AOM1.4 UML class adocs (C_BOOLEAN/C_STRING/C_DATE/C_TIME/C_DATE_TIME/C_CODED_TEXT/CONSTRAINT_REF/C_ATTRIBUTE/C_MULTIPLE_ATTRIBUTE/C_OBJECT/C_INTEGER), and BASE `foundation_types` master06 + `Iso8601_time` for the ISO 8601 profile. Fixture keys checked against `corpus/MANIFEST.yaml` + `corpus/templates/` (all present); per-row OPT synthesis (`exec/opt_synth.rs`, `exec/content_synth.rs`, `recipes::synth_template_id`) confirmed to realize every declared constraint column **except `constraint_bindings`** (see D10) and to stamp per-case+per-row template ids (no cross-case collision). AMB-29 guard applicability checked: no slice-B range row compares expressions with a shared leading component where ordering is undecidable, so the absence of AMB-29 guards is sound (the two shared-prefix rows in DV_DATE_TIME-validate_range follow the schedule's own full-interval interpretation notes).

## Per-case verdicts

| case id | verdict | evidence — spec text actually read, what was checked | resolution |
|---|---|---|---|
| CONT-COMPOSITION-content_cardinality | DEFECT | master15 §CONT-COMP-content_card_{any,1plus,3plus,opt,mand,3to5}-context_{any,mand} (12 official test-case headings + full tables, read in full); RM `org.openehr.rm.composition.composition.adoc` (content 0..1 List; invariants **Category_validity, Territory_valid, Language_valid, Content_valid, Is_archetype_root**); AOM1.4 c_multiple_attribute.adoc (cardinality 1..1). All 24 row verdicts recomputed — consistent with the official tables where they overlap; the count-6 rows are derivable extensions (and add the 3to5 upper-bound coverage the official tables miss). | fix ground claims + map official ids + add missing coverage (D1) |
| CONT-COMPOSITION-context_existence | DEFECT | Same master15 reading; RM composition.adoc (context 0..1, no context invariant — confirmed). 4 rows recomputed, verdicts consistent with the official context_mand blocks. | fix ground claim + official-id mapping + combined-axis rows (D2) |
| CONT-DV_BOOLEAN-anything_allowed | ok | master17.1 §CONT-DV_BOOLEAN-anything_allowed (both rows verbatim); RM dv_boolean.adoc (value 1..1); AOM1.4 c_boolean.adoc (true_valid/false_valid 1..1). | none |
| CONT-DV_BOOLEAN-only_false_allowed | DEFECT | master17.1 §CONT-DV_BOOLEAN-only_false_allowed row 1 is internally inconsistent (`accepted` + violates `C_BOOLEAN.true_valid`); AOM1.4 c_boolean.adoc derives `rejected` (the encoded row is right); AMB-28's handling text and the case's own test_purpose contradict the encoded row. | fix test_purpose + AMB-28 handling wording (D9) |
| CONT-DV_BOOLEAN-only_true_allowed | ok | master17.1 §CONT-DV_BOOLEAN-only_true_allowed — both rows verbatim; AOM1.4 C_BOOLEAN recomputation agrees. | none |
| CONT-DV_CODED_TEXT-validate_ext_term | DEFECT | master17.2 §CONT-DV_CODED_TEXT-validate_ext_term row 4 reads `ABC/local/ac0001/[SNOMED_CT] → rejected, constraint_binding: terminology_id not found`; the YAML flips it to `accepted` via an in-file re-adjudication with no register entry. Also `constraint_bindings` is realized nowhere (grep `binding` in `exec/opt_synth.rs` and `templates/dt_coded_text_constraint_ref.opt`: zero hits). | restore schedule verdict or register (D10) |
| CONT-DV_CODED_TEXT-validate_local_codes | ok | master17.2 §validate_local_codes — 5 rows verbatim incl. the SNOMED-CT terminology mismatch rejection; RM dv_coded_text.adoc (defining_code 1..1, no added invariants — the standing value≠rubric adjudication honored: no rejection-direction value row present). | none |
| CONT-DV_CODED_TEXT-validate_open | ok | master17.2 §validate_open — 5 rows verbatim; RM CODE_PHRASE mandatory fields via dv_coded_text.adoc. No value≠rubric rejection row. | none |
| CONT-DV_DATE-validate_constraint | ok | master17.4 §CONT-DV_DATE-validate_constraint — all 15 rows verbatim; recomputed from AOM1.4 c_date.adoc VALIDITY_KIND semantics (mandatory component absent / prohibited component present ⇒ that flag violated): every verdict + violates set derives. | none |
| CONT-DV_DATE-validate_open | ok | master17.4 §CONT-DV_DATE-validate_open — 10 rows verbatim (incl. the schedule's duplicated `''` row with its two alternative attributions); BASE master06 (4-digit year assumption) + the chapter's C_DATE year-mandatory note. | none |
| CONT-DV_DATE-validate_range | ok | master17.4 §CONT-DV_DATE-validate_range — 9 rows verbatim; same-precision comparability per the chapter note; test_purpose states the shared-precision rule. | none |
| CONT-DV_DATE_TIME-validate_constraint | ok | master17.4 §CONT-DV_DATE_TIME-validate_constraint — all 11 value blocks × 15 rows (165) diffed block-by-block, spot-recomputed against AOM1.4 c_date_time.adoc VALIDITY_KIND rule; full parity. | none |
| CONT-DV_DATE_TIME-validate_open | ok | master17.4 §CONT-DV_DATE_TIME-validate_open — 28 rows verbatim; BASE master06 (partial date-times may omit hours/days/months; only fractional seconds). | none |
| CONT-DV_DATE_TIME-validate_range | AMBIGUITY | master17.4 §CONT-DV_DATE_TIME-validate_range T-precision block uses value `2021-10-24T10` against `1900-03-13` ranges yet expects `accepted` — arithmetically impossible under BASE foundation_types Interval total-order containment (and under the chapter's own full-interval interpretation). The YAML re-anchors the value to `1900-03-13T10` (in-file comment, 2026-07-22), preserving each row's verdict; recomputed: all 37 rows now derive. Divergence is real but unregistered; test_purpose also cites the nonexistent REQUIREMENTS-rest.md. | register entry + fix pointer (A1) |
| CONT-DV_EHR_URI-validate_list | ok | master17.7 §CONT-DV_EHR_URI-validate_list — 3 rows verbatim. | none |
| CONT-DV_EHR_URI-validate_open | ok | master17.7 §CONT-DV_EHR_URI-validate_open — 17 rows verbatim; RM dv_ehr_uri.adoc invariant `Scheme_valid: scheme.is_equal (Ehr_scheme)` confirms every non-ehr-scheme rejection. | none |
| CONT-DV_EHR_URI-validate_pattern | ok | master17.7 §CONT-DV_EHR_URI-validate_pattern — 3 rows verbatim; AOM1.4 c_string.adoc (pattern 0..1). | none |
| CONT-DV_IDENTIFIER-validate_all_list | ok | master17.1 §CONT-DV_IDENTIFIER-validate_all_list — all 4 per-attribute tables (12 rows) verbatim incl. NULL→C_STRING.list attributions and the id RM-mandatory row; RM dv_identifier.adoc (id 1..1 + `Id_valid`; issuer/assigner/type 0..1). | none |
| CONT-DV_IDENTIFIER-validate_all_pattern | ok | master17.1 §CONT-DV_IDENTIFIER-validate_all_pattern — 12 rows verbatim, same RM grounding. | none |
| CONT-DV_MULTIMEDIA-validate_media_type | ok | master17.6 §CONT-DV_MULTIMEDIA-validate_media_type — 8 rows verbatim incl. the dual-violation row; AOM1.4 c_integer.adoc (list/range 0..1). | none |
| CONT-DV_MULTIMEDIA-validate_open | ok | master17.6 §CONT-DV_MULTIMEDIA-validate_open — 4 rows verbatim; RM dv_multimedia.adoc (media_type 1..1, size 1..1, invariant `Media_type_valid` — the rm_invariant name the case uses is real). | none |
| CONT-DV_PARSABLE-validate_open | ok | master17.6 §CONT-DV_PARSABLE-validate_open — 4 rows verbatim; RM dv_parsable.adoc (value 1..1, formalism 1..1, `Formalism_valid`). | none |
| CONT-DV_PARSABLE-validate_value_formalism | ok | master17.6 §CONT-DV_PARSABLE-validate_value_formalism — 7 rows verbatim; per-field C_STRING columns realized by `opt_synth::field_constraint`. | none |
| CONT-DV_TEXT-validate_list | ok | master17.2 §CONT-DV_TEXT-validate_list — 3 rows verbatim; RM dv_text.adoc (value 1..1). | none |
| CONT-DV_TEXT-validate_open | DEFECT | master17.2 carries the duplicate `CONT-DV_TEXT-validate_open` heading (two tables); the merge is registered as AMB-28 and the 6 merged rows are verbatim — but the test_purpose cites the nonexistent `REQUIREMENTS-rest.md` instead of AMB-28. | point at AMB-28 (D11) |
| CONT-DV_TIME-validate_constraint | AMBIGUITY | master17.4 §CONT-DV_TIME-validate_constraint — all 7 blocks × 9 rows recomputed against AOM1.4 c_time.adoc VALIDITY_KIND: full verdict parity. The value literals drop the schedule's leading `T` (schedule NOTE: "our test data sets all include the T time marker") on the strength of BASE `Iso8601_time` (`hh:mm:ss…` / `hhmmss…`, no T form) — a genuine schedule-vs-BASE contradiction resolved in-file, unregistered. | register entry for the T-marker divergence (A2) |
| CONT-DV_TIME-validate_open | AMBIGUITY | master17.4 §CONT-DV_TIME-validate_open — 23 rows verbatim (modulo the T-strip, same divergence as above); fractional-hour/minute rejections confirmed against BASE master06 ("only fractional seconds are supported"). | shared register entry (A2) |
| CONT-DV_TIME-validate_range | AMBIGUITY | master17.4 §CONT-DV_TIME-validate_range — all 8 value blocks × 24 rows diffed: full parity, `>=`/`<=` limits decomposed to lower/upper with null (semantics-preserving). Same unregistered T-strip; test_purpose also cites the nonexistent REQUIREMENTS-rest.md. No undecidable shared-prefix comparison present, so no AMB-29 guard needed. | shared register entry + fix pointer (A2) |
| CONT-DV_URI-validate_list | ok | master17.7 §CONT-DV_URI-validate_list — 2 rows verbatim. | none |
| CONT-DV_URI-validate_open | ok | master17.7 §CONT-DV_URI-validate_open — 11 rows verbatim; RM dv_uri.adoc (value 1..1, `Value_valid`); RFC 3986 requirement is the schedule's own ground. | none |
| CONT-DV_URI-validate_pattern | ok | master17.7 §CONT-DV_URI-validate_pattern — 2 rows verbatim. | none |
| CONT-EVENT-state_existence | DEFECT | master16 §CONT-EVENT-state_ex_opt + §CONT-EVENT-state_ex_mand (official headings + 8 rows — the chapter is NOT empty); RM event.adoc (data 1..1, state 0..1). The 5 encoded rows' verdicts match the official tables; 3 official rows are missing and the ids/ground claim are wrong. | fix ground claim + ids + rows (D5) |
| CONT-EVENT-type_narrowing | DEFECT | master16 §CONT-EVENT-type_any/type_point_event/type_interval_event — all 6 official rows present with identical verdicts; RM interval_event.adoc (width 1..1, math_function 1..1) confirms the case note. Defect is solely the false "master16 is empty" ground claim + non-official ids. | fix ground claim + ids (D6) |
| CONT-HISTORY-events_cardinality | DEFECT | master16 §CONT-HIST-events_card_*-summary_ex_opt (official tables); RM history.adoc invariant `Events_valid: (events /= Void and then not events.is_empty) or summary /= Void`. Official `events_card_any/opt` accept 0 events + absent summary; the RM invariant rejects — a real spec-vs-spec conflict the case resolves in-file (RM side) without a register entry, on top of the false "no test-case headings" claim and non-official ids. Cardinality verdicts otherwise recomputed and consistent. | register the conflict + fix ground/ids (D7) |
| CONT-HISTORY-summary_existence | DEFECT | master16 §CONT-HIST-events_card_any-summary_ex_{opt,mand} — the 4 encoded verdicts match; false ground claim, non-official ids, missing official row combinations (0/3-event rows with summary present). | fix ground claim + ids + rows (D8) |
| CONT-ITEM_STRUCTURE-type_narrowing | DEFECT | master16 §CONT-ITEM_STR-type_{any,item_tree,item_list,item_table,item_single} — official 20 rows: each narrowed type rejects ALL three siblings; the case tests only one sibling per narrowed type (8 official rejection rows missing), plus the false ground claim and non-official ids. Encoded verdicts consistent. | fix ground claim + ids + add rows (D4) |
| CONT-OBSERVATION-state_protocol_existence | DEFECT | master16 §CONT-OBS-state_ex_{opt,mand}-protocol_ex_{opt,mand} — official 32 rows; RM observation.adoc (data 1..1, state 0..1) + care_entry.adoc (protocol 0..1). The 9 encoded verdicts match the official tables; 23 official rows (absent-data combinations, mixed present/absent) missing; false ground claim + non-official ids. | fix ground claim + ids + rows (D3) |

## Defects

### D1–D8 (one root cause): the six structural cases claim the official chapters carry no test cases — they do

**What the YAMLs say** (headers + `spec_refs` of `CONT-COMPOSITION-content_cardinality`, `CONT-COMPOSITION-context_existence`, `CONT-OBSERVATION-state_protocol_existence`, `CONT-EVENT-state_existence`, `CONT-EVENT-type_narrowing`, `CONT-HISTORY-events_cardinality`, `CONT-HISTORY-summary_existence`, `CONT-ITEM_STRUCTURE-type_narrowing`): "The official CNF content chapter master15 carries no test-case headings" / "master16 (ENTRY) is empty" / "CNF platform_test_schedule master15/master16 (content chapter — no test-case headings; re-adjudicated to AOM/RM)", with ids in a private namespace ("`ecc-` id namespace pending upstream adoption").

**What the spec says**: `docs/specs/openehr/CNF/docs/platform_test_schedule/master15-content_tc_composition.adoc` carries **12 official test cases** (`=== Test Case CONT-COMP-content_card_any-context_any` … `CONT-COMP-content_card_3to5-context_mand`, lines 51–424) each with a full decision table; `master16-content_tc_entry.adoc` carries **26 official test cases** (`CONT-OBS-state_ex_opt-protocol_ex_opt` …, `CONT-HIST-events_card_any-summary_ex_opt` …, `CONT-EVENT-state_ex_opt`/`state_ex_mand`/`type_any`/`type_point_event`/`type_interval_event`, `CONT-ITEM_STR-type_any` … `type_item_single`) with full tables. `git log --follow` shows both files unchanged since their original vendoring — the claims were false when authored (2026-07-21).

**Consequences per case** (all encoded verdicts were recomputed against the official tables — no verdict conflicts except D7):

- **D1 CONT-COMPOSITION-content_cardinality**: ids/ground unmapped to the 12 official `CONT-COMP-*` cases; the official combined content×context tables (e.g. `card_1plus-context_mand`: "no entries | no context | rejected | COMPOSITION.content: cardinality.lower, COMPOSITION.context occurrences.lower") and the "context without/with other_context" data-set split are untested. Additionally the spec_refs claim "Category_validity/Territory_valid only, no content invariant" is false: RM composition.adoc §Invariants includes `Content_valid: content /= Void implies not content.is_empty` (plus `Language_valid`, `Is_archetype_root`). The 0-item rows are only sound if the fixture omits the `content` attribute entirely (never an empty list) — that representation choice must be pinned.
  *Proposed fix*: re-ground the case(s) on the official `CONT-COMP-*` headings (official ids), add the combined-axis rows and the other_context data-set split, correct the spec_refs invariant enumeration, and pin the absent-vs-empty `content` representation for the 0 rows.
- **D2 CONT-COMPOSITION-context_existence**: same re-grounding; add the official combined rows (they live in the same 12 tables).
- **D3 CONT-OBSERVATION-state_protocol_existence**: map to `CONT-OBS-state_ex_*-protocol_ex_*`; add the 23 missing official rows (all four absent-data combinations per table, and the mixed rows such as `present|absent|present → accepted` under state-opt/protocol-mand).
- **D4 CONT-ITEM_STRUCTURE-type_narrowing**: map to `CONT-ITEM_STR-type_*`; add the 8 missing sibling-rejection rows (official narrowed tables reject all three siblings each).
- **D5 CONT-EVENT-state_existence**: map to `CONT-EVENT-state_ex_opt`/`state_ex_mand`; add the 3 missing official rows (`absent|present` under opt; `absent|absent` and `absent|present` under mand).
- **D6 CONT-EVENT-type_narrowing**: rows are already at full parity with `CONT-EVENT-type_any`/`type_point_event`/`type_interval_event`; only the ground claim + id mapping need fixing.
- **D7 CONT-HISTORY-events_cardinality**: beyond the ground claim, an unregistered spec-vs-spec conflict: official `CONT-HIST-events_card_any-summary_ex_opt` row 1 ("no events | absent | **accepted**", master16 line 127; likewise `events_card_opt` row 1) contradicts RM history.adoc `Events_valid: (events /= Void and then not events.is_empty) or summary /= Void`, which forces rejection. The case sides with the RM (defensible) but resolves the contradiction privately — the schedule text is the conformance oracle and a divergence from its explicit `accepted` cell requires an `ambiguities.yaml` entry (the AMB-27/28 pattern). Also untested: the official rows with 0/3 events + summary **present** (the invariant satisfied by the summary side).
  *Proposed fix*: register the Events_valid-vs-master16 contradiction with a typed disposition (the RM-invariant side as the handling), map ids to `CONT-HIST-events_card_*-summary_ex_opt`, and add the summary-present rows.
- **D8 CONT-HISTORY-summary_existence**: map to `CONT-HIST-events_card_any-summary_ex_{opt,mand}`; add the 0-/3-event rows the official tables cross with the summary axis.

### D9 CONT-DV_BOOLEAN-only_false_allowed — three mutually contradictory statements about row 1

- **YAML test_purpose**: "the schedule's first row is internally inconsistent … **reproduced verbatim**; see REQUIREMENTS-rest.md" — but the row is NOT reproduced verbatim: the row comment says "expected corrected 2026-07-22" and encodes `rejected`.
- **AMB-28 handling** (`registers/ambiguities.yaml`): "the inconsistent DV_BOOLEAN row **follows the verdict cell**" — the schedule's verdict cell is `accepted` (master17.1 line 37: `true | false | true | accepted | C_BOOLEAN.true_valid`), so the register mandates the opposite of what the case encodes.
- **The spec**: AOM1.4 c_boolean.adoc — `true_valid: Boolean` ("True if the value true is allowed"); `true_valid=false` with committed value `true` must reject. The **encoded row (`rejected`) is the spec-correct one**; the schedule's `accepted` cell is the editorial defect (symmetric with §only_true_allowed row 2, which rejects `false` under `false_valid=false`).
- `REQUIREMENTS-rest.md` does not exist anywhere in the repo (dead pointer; internal-markdown citations are banned anyway).
- *Proposed fix*: keep the row; rewrite the test_purpose to state the correction with the AOM citation and the AMB-28 tag; amend AMB-28's handling to "follows the violates cell / the AOM-derived verdict (rejected)" so register and catalogue agree.

### D10 CONT-DV_CODED_TEXT-validate_ext_term — schedule verdict flipped without a register entry, and the binding column is never realized

- **What the YAML says**: row 4 `["ABC","local","ac0001","[SNOMED_CT]"] → accepted`, justified by an in-file comment ("AOM1.4 §Reference Objects defines CONSTRAINT_REF as a proxy …; the spec is silent on commit-time enforcement when no binding covers the instance terminology … VATDF/VACDF constrain the TEMPLATE, not the data").
- **What the schedule says** (master17.2 §CONT-DV_CODED_TEXT-validate_ext_term, line 103): `ABC | local | ac0001 | [SNOMED_CT] | rejected | constraint_binding: terminology_id not found` — an explicit rejection cell, in a chapter whose preamble explains exactly this binding-resolution mechanism ("Without that, the SUT doesn't know which terminology_id can be used in that DV_CODED_TEXT"). The excluded-from-scope case the preamble names is *no bindings at all* ("The cases where there are no constraint_bindings are not tested here"), not an unmatched terminology_id. The standing value≠rubric adjudication does not cover this row — it is a defining_code/binding check, not a value/rubric check.
- The AOM prose (AOM1.4 master04 §Reference Objects, read first-hand) indeed frames CONSTRAINT_REF as a proxy for an external query — but the schedule's explicit cell is the conformance ground, and overriding it is exactly the "adjust an expectation" move the triage law forbids without an `ambiguities.yaml` entry.
- **Fixture gap**: the decision-table column `constraint_bindings` is realized nowhere — `exec/opt_synth.rs` synthesizes the CONSTRAINT_REF (`fn constraint_ref`, line 344) but emits no ontology `constraint_binding`, `grep -i binding` over `opt_synth.rs` and `templates/dt_coded_text_constraint_ref.opt` returns nothing, and no runner code reads the column. The committed OPT is therefore the very no-binding template the schedule excludes from data-validation scope.
- *Proposed fix*: either (a) restore `rejected` per the schedule cell and emit a real `constraint_binding` (ac0001 → SNOMED-CT terminology query URI) in the synthesized OPT so the row is realizable, or (b) take the AOM-prose-vs-schedule-cell tension through `ambiguities.yaml` with a typed disposition and tag the case. In both branches, make the `constraint_bindings` column real or remove it.

### D11 CONT-DV_TEXT-validate_open — stale pointer to a nonexistent file

The duplicate-heading merge is correct and registered (AMB-28: "the duplicate DV_TEXT heading is one case with the two tables merged"), but the test_purpose cites `REQUIREMENTS-rest.md`, which does not exist in the repo. *Proposed fix*: replace the pointer with the AMB-28 tag. (Same dead pointer also appears in CONT-DV_BOOLEAN-only_false_allowed, CONT-DV_TIME-validate_range, CONT-DV_DATE_TIME-validate_range — and in slice-A's CONT-DV_DURATION-validate_fields_range; scrub all in one pass.)

## Ambiguities (register candidates)

### A1 CONT-DV_DATE_TIME-validate_range — master17.4's T-precision block contradicts BASE Interval semantics

master17.4 lines 977–1000 test value `2021-10-24T10` against ranges anchored on `1900-03-13` and expect `accepted` for the `T00..T23` windows — impossible under BASE foundation_types Interval containment (total order on the full date-time: 2021 > 1900 exceeds every upper bound) and under the chapter's own reduced-precision full-interval interpretation. The catalogue re-anchors the value to `1900-03-13T10` (in-file comment 2026-07-22), preserving every verdict; all 37 rows recompute correctly after the re-anchor. This is a real, verified schedule editorial defect not enumerated in AMB-28 — it needs its own register line (disposition `editorial`, handling: value re-anchored to the range's calendar date, verdicts per the schedule's hour-window intent). The two shared-prefix rows (`2021-05` vs upper `2021`; `2021` vs `2020-07..2022-03`) follow the schedule's explicit full-interval interpretation notes and stay encoded as accepted.

### A2 DV_TIME family — the schedule's leading `T` vs BASE `Iso8601_time`

master17.4 (line 147) states "our test data sets all include the `T` time marker", and every DV_TIME literal in the chapter is T-prefixed (`T10`, `T10:30`, …). BASE `org.openehr.base.foundation_types.iso8601_time.adoc` defines the value forms as `hh:mm:ss[(,|.)sss][Z|±hh[:mm]]` (extended) or `hhmmss…` (compact) — no T-prefixed form — so a T-prefixed string is not a valid `Iso8601_time` value and would fail RM `DV_TIME.Value_valid: valid_iso8601_time(value)`. The catalogue strips the T across `CONT-DV_TIME-validate_{open,constraint,range}` (in-file comments 2026-07-22), which is the BASE-correct wire form, but the divergence from the schedule's explicit statement is unregistered. Needs a register entry (disposition `editorial` or `fixed_handling`: time literals encoded in the BASE `Iso8601_time` string form; upstream editorial candidate for master17.4).

## Cross-cutting notes (no verdict impact)

- **Citation form "AM AOM1.4 §C_CODE_PHRASE"**: the AOM1.4 UML names this class `C_CODED_TEXT` (attributes `terminology`, `code_list`); `C_CODE_PHRASE` is the openEHR-profile/OPT name (ADL1.4 master05/master09). The cited constraint exists; consider citing the profile document for precision.
- **Fixtures/collisions**: every `cnf.tpl.*` key used by slice-B cases exists in `corpus/MANIFEST.yaml` with a matching `template_id` and an existing `templates/*.opt` file; the runner stamps per-case+per-row synthesized template ids (`recipes::synth_template_id`), so shared static skeletons cannot collide across cases on a shared SUT. All declared `constraint_columns` (incl. the structural `cardinality`/`*_existence`/`slot_type` tokens and the DV `C_*`/`range.*`/`*_validity`/`attribute` columns) are consumed by `exec/opt_synth.rs`/`exec/content_synth.rs` — the sole unrealized column is `constraint_bindings` (D10).
- **Manifest provenance strings** for `dv_text_c_string` ("constraint per CONT-DV_TEXT-validate_list") and the per-template "validated against the SUT" phrasing are sloppy but carry no expectation weight (expectations live in the cases; OPT constraints are synthesized per row).

## Rebuild (2026-07-24)

D1–D8 resolved by rebuilding the structural family on the official master15/master16 inventory (verbatim official ids, every table recomputed against AOM1.4 `c_multiple_attribute.adoc`/`c_attribute.adoc`/`C_OBJECT` and the RM class adocs read first-hand). Register entry **AMB-51** added (master16 `CONT-HIST-events_card_any-summary_ex_opt` row 1 + `CONT-HIST-events_card_opt-summary_ex_opt` row 1 — "no events | absent | accepted" vs RM `Events_valid`; rows encoded rejected per the RM-derivable verdict, disposition editorial); the two affected cases tag it. No other recomputed cell contradicted its official verdict.

### Old id → disposition

| old ad-hoc id | disposition |
|---|---|
| CONT-COMPOSITION-content_cardinality | deleted — count 0/1/3 rows covered by the 6 new `CONT-COMP-content_card_*-context_any` files; count-6 rows converted to the addition case `CONT-COMPOSITION-content_cardinality_count6` |
| CONT-COMPOSITION-context_existence | converted — mandatory rows covered by `CONT-COMP-content_card_any-context_mand`, (optional, present) by `CONT-COMP-content_card_any-context_any`; the file now carries ONLY the (optional, absent) official "no context" cell as a flagged realization complement (the cardinality synth family always commits a context) |
| CONT-OBSERVATION-state_protocol_existence | deleted — all 9 rows covered by the 4 new `CONT-OBS-state_ex_*-protocol_ex_*` files (32 official rows, full tables) |
| CONT-EVENT-state_existence | deleted — all 5 rows covered by `CONT-EVENT-state_ex_{opt,mand}` (8 official rows, full tables) |
| CONT-EVENT-type_narrowing | deleted — all 6 rows covered by `CONT-EVENT-type_{any,point_event,interval_event}` (full tables) |
| CONT-HISTORY-events_cardinality | deleted — count 0/1/3 rows covered by the 6 new `CONT-HIST-events_card_*-summary_ex_opt` files; count-6 rows converted to the addition case `CONT-HISTORY-events_cardinality_count6` |
| CONT-HISTORY-summary_existence | deleted — all 4 rows covered by `CONT-HIST-events_card_1plus-summary_ex_{opt,mand}` (both full official tables; the summary synth family's fixed 1..* events cardinality IS these cases' constraint) |
| CONT-ITEM_STRUCTURE-type_narrowing | deleted — all 12 rows covered by the 5 new `CONT-ITEM_STR-type_*` files (20 official rows: every narrowed table rejects all three siblings) |

### Official ids added

master15 (7 of 12): `CONT-COMP-content_card_{any,1plus,3plus,opt,mand,3to5}-context_any` (3 rows each — the "context without other_context" block; the instance builder always commits a context without other_context), `CONT-COMP-content_card_any-context_mand` (2 rows — the "one entry" block; the context-existence family fixes content at one entry).

master16 (21 of 26): `CONT-OBS-state_ex_{opt,mand}-protocol_ex_{opt,mand}` (4 files, full 8-row tables), `CONT-HIST-events_card_{any,1plus,3plus,opt,mand,3to5}-summary_ex_opt` (card_1plus full 6 rows via the summary family; the other 5 carry the 3 summary-absent rows each), `CONT-HIST-events_card_1plus-summary_ex_mand` (full 6 rows), `CONT-EVENT-state_ex_{opt,mand}`, `CONT-EVENT-type_{any,point_event,interval_event}`, `CONT-ITEM_STR-type_{any,item_tree,item_list,item_table,item_single}` (all full tables).

### Not encoded (synth-vocabulary limits — no runner code invented; reported on the tracker issue)

- master15 `CONT-COMP-content_card_{1plus,3plus,opt,mand,3to5}-context_mand` (5 cases, 45 rows): the per-row OPT synthesis has no combined content-cardinality × context-occurrences family.
- master16 `CONT-HIST-events_card_{any,3plus,opt,mand,3to5}-summary_ex_mand` (5 cases, 30 rows): the summary-existence family fixes events cardinality at 1..*, which is not these cases' constraint.
- The "no context"/"context with other_context" data-set rows of the 6 encoded `context_any` cases (36 rows; the one "one entry | no context" acceptance cell IS carried by the `CONT-COMPOSITION-context_existence` complement) and the remaining 7 rows of `card_any-context_mand`; the summary-present rows of the 5 partially-encoded `summary_ex_opt` cases (15 rows). Each encoded case file carries the matching `TODO:`.

## Completion (2026-07-24, combined synth families)

The synth vocabulary extension (combined cardinality×context-existence and
events-cardinality×summary-existence template families; the three-state
`context_committed` and the `summary_committed` data axes on the single-axis
cardinality families) made every remaining master15/master16 row realizable.
The "Not encoded" list in the Rebuild section above is now empty — the
official structural inventory is fully encoded (all 12 master15 CONT-COMP
cases + all 12 master16 CONT-HIST cases, full official tables). Every verdict
recomputed against AOM1.4 `c_multiple_attribute.adoc`/`c_attribute.adoc` and
the RM `composition.adoc`/`history.adoc` invariants read first-hand.

### New case files (10 — 75 rows)

- master15 `CONT-COMP-content_card_{1plus,3plus,opt,mand,3to5}-context_mand`
  (5 files × 9 rows = 45): constraint_columns `["cardinality",
  "context_existence"]` on the combined family (one template constrains both
  axes; the constrained context carries an optional other_context ITEM_TREE
  at0011 so the "context with other_context" block commits against a
  constrained node). All 45 recomputed verdicts match the official cells.
- master16 `CONT-HIST-events_card_{any,3plus,opt,mand,3to5}-summary_ex_mand`
  (5 files × 6 rows = 30): constraint_columns `["cardinality",
  "summary_existence"]`. The zero-events + summary-present cells recompute as
  the official tables verdict them: `accepted` where the cardinality admits 0
  (`any`, `opt` — Events_valid holds via its second disjunct), `rejected` on
  cardinality.lower where it does not (`3plus`, `mand`, `3to5`). All 30
  recomputed verdicts match the official cells — no new AMB-51-style
  contradiction found.

### Extended case files (12 — +58 rows)

- The 6 `CONT-COMP-content_card_*-context_any` files: 3 → 9 rows each (+36),
  the three-state `context_committed` axis realizing the official "no
  context" / "context without other_context" / "context with other_context"
  blocks (existing rows kept as the `present` block; verdicts identical
  across blocks — context is unconstrained). TODOs removed.
- `CONT-COMP-content_card_any-context_mand`: 2 → 9 rows (+7), migrated onto
  the combined family (constraint_columns `["cardinality",
  "context_existence"]`, the full official content 0/1/3 × three context
  conditions table). TODO removed.
- The 5 partial `CONT-HIST-events_card_*-summary_ex_opt` files (`any`,
  `3plus`, `opt`, `mand`, `3to5`): 3 → 6 rows each (+15), the
  `summary_committed` axis realizing the official summary-present block. The
  two registered AMB-51 rows (zero events + absent summary verdicting
  `rejected` against the official `accepted`) are unchanged and stay tagged.
  TODOs removed.
- Comment-only: `CONT-COMPOSITION-context_existence` — its premise ("the
  cardinality synth family always commits a context") is obsolete; retained
  as the distinct-family (existence-template) realization of the "one entry |
  no context | accepted" cell, comment updated.

### Still unencodable

Nothing. No master15/master16 row remains outside the catalogue; no case file
in the structural family carries a realizability `TODO:` anymore. (The 3..5
upper-bound crossing is still not an *official* row — the official data sets
stop at 3 — and remains covered by the `*_count6` catalogue extensions.)

Validator: `cnf-runner validate --root tools/cnf-runner/artifacts --specs
docs/specs/openehr` → 429 case(s), 88 binding(s), **0 finding(s)**.
