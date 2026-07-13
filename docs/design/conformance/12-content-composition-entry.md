# Content: COMPOSITION + ENTRY data-validation (`suites/content/composition.rs` + `entry.rs`): spec-first register

W-10 area audit (read-only, 2026-07-13) of the **content data-validation**
suites for the top-level `COMPOSITION` class (schedule master15) and the
internal `ENTRY`/`HISTORY`/`EVENT`/`ITEM_STRUCTURE` classes (schedule
master16). Method (owner ruling, README §methodology): the register's spine is
the governing CNF Platform Conformance Test Schedule chapter enumerated **test
case by test case** (with citation); the existing ECC cases are then mapped
**onto** that spine with a `file:line` verdict (conformant / divergent /
missing / instrument-encodes-server-behaviour). Cases with no schedule home are
flagged (§3); G-rows carry gaps + rulings for the rewrite (§4).

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master15-content_tc_composition.adoc`
  — the `COMPOSITION` data-validation cases (12 `CONT-COMP-*`: the
  content-cardinality × context-occurrences matrix). Read whole.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master16-content_tc_entry.adoc`
  — the `ENTRY` data-validation cases (26: `CONT-OBS-*` ×4, `CONT-HIST-*` ×12,
  `CONT-EVENT-*` ×5, `CONT-ITEM_STR-*` ×5). Read whole.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  §Data Validation Conformance Test Design — these are **data-validation**
  cases: author a constraining archetype/OPT variant, commit multiple data-set
  instances, assert accept/reject ("at least one success case, one failure case
  and all border cases").
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — Archetype
  Validation is a capability of **EHR + Persistence**, required by both **CORE**
  and **STANDARD** (verified: profiles table, EHR + Persistence group).

**Not in scope here:** the `DATA_VALUE` cases (schedule master17.1–17.7,
`CONT-*` leaf-value constraints) live in `suites/content/data_types.rs` and are
audited by **register 13**. This register covers only `composition.rs` +
`entry.rs` (with their shared `author.rs`/`drive.rs`/`mutate.rs` machinery).

**Verified schedule fact** (blueprint chapter 07-cnf): master15 §Implementation
notes says the constraining archetypes "should be generated" — verbatim, "We
suggest to automate the archetype/template test cases generation instead of
creating each constraint combination manually", and the vendored corpus ships
**no** per-variant OPT. The suite honours this by **authoring** the constraint
OPT programmatically (`author.rs`, from a vendored base OPT tightened in the
typed `openehr_its::opt14` model). See §4 G-2 for the manifest ruling.

---

## 1. Verdict

The COMPOSITION spine (master15, 12 cases → ECC-VAL-001..012) is **fully and
faithfully driven**: `composition.rs` authors a real ADL 1.4 OPT per
cardinality/occurrences variant and commits the truth-table data sets, so every
one is a genuine server validation decision, not a fabricated pass. The ENTRY
spine (master16, 26 cases → ECC-VAL-013..038) is **mixed**: the `HISTORY` (12),
`EVENT` type-narrowing (3 of 5) and `ITEM_STRUCTURE` (5) cases are genuinely
driven against authored/vendored constraint OPTs; but the four `CONT-OBS-*`
cases and the two `CONT-EVENT-state_ex_*` cases are **under-tested** — each
drives only the RM/schema `data`-existence subset of its truth table and maps
several distinct schedule cases onto one identical assertion, leaving the
archetype `state`/`protocol` existence dimension untested while the ECC id
claims the whole case (§4 G-1, the load-bearing gap).

No case is missing (all 38 comp/entry schedule cases have an ECC id) and no ECC
comp/entry case is orphaned (§3 is empty). The rewrite work is: register the
authored OPTs as declared generated fixtures (G-2), assert the archetype
existence dimension the ECC ids promise (G-1), and pin the edition/version and
"constraints violated" assertions the instrument currently leaves loose (G-3).

Counts: master15 = **12**; master16 = **26**; total spine = **38**. Mapped
1:1 = 38; conformant/faithful = 21; partial/under-tests-the-schedule-case = 6
(OBS ×4, EVENT-state ×2); instrument-encodes-a-deliberate-deviation = 12
(HISTORY — an RM-invariant override + summary-present rows undriven). Missing =
0. The **ECC-VAL split**: 001–012 = COMPOSITION (this register), 013–038 =
ENTRY (this register), **039–119 = DATA_VALUE (register 13)** — 38 comp/entry
vs 81 data-type cases (119 VAL total). No VAL-area adjudications exist
(`adjudications/ehrbase-java-2.34.toml` has zero VAL rows).

---

## 2. The spine (schedule case → ECC mapping)

Each row: schedule id + citation → constraint variant + accept/reject data-set
dimensions (summarised, not copied) → ECC id + `file:line` + verdict.
Capability for every row: **ArchetypeValidation, CORE + STANDARD**
(`drive.rs:432` `meta()` sets `Capability::ArchetypeValidation`,
`profiles = [Core, Standard]`, `Area::Val`, `Format::Json`). All committed via
`POST /ehr/{id}/composition` with accept `201`, reject `422|400`
(`drive.rs:279` `check()`).

### 2.1 master15 — COMPOSITION (§COMPOSITION Test Cases)

Every `CONT-COMP-*` case constrains `COMPOSITION.content` **cardinality** (one
of the six schedule intervals `0..*`, `1..*`, `3..*`, `0..1`, `1..1`, `3..5` —
master15 §"For testing a 'multiple attribute' cardinality") crossed with
`COMPOSITION.context` **occurrences** (`_any` = unconstrained `0..*` vs `_mand`
= `1..1`). Schedule data-set matrix per case: content ∈ {no entries, one entry,
three entries} × context ∈ {no context, context w/o other_context, context w/
other_context} = **9 rows**; the accept/reject oracle depends only on the
content count vs the interval and on context present-vs-absent (other_context
never flips the outcome — see §4 G-5).

Runner (`composition.rs`): `drive_case()` (`composition.rs:70`) authors the OPT
from base `minimal_evaluation` (`BASE_OPT`, `composition.rs:40`) via
`author::set_root_multiple_cardinality` on `content`
(`composition.rs:78`) + `author::set_root_single_mandatory` on `context` for
the `_mand` cases (`composition.rs:84`), then commits **6** data sets: content ∈
{0,1,3} × context ∈ {present, absent} (`composition.rs:91`). Accept iff
`content_ok(card,count) && (context_present || !context_mand)`
(`composition.rs:43`, `:93`).

| Schedule id (master15) | Constraint variant | Accept/reject dims | ECC | `file:line` | Verdict |
|---|---|---|---|---|---|
| `CONT-COMP-content_card_any-context_any` | content `0..*`, context free | all 9 accept | ECC-VAL-001 | `composition.rs:128,207` | conformant |
| `CONT-COMP-content_card_1plus-context_any` | content `1..*`, context free | 0 entries reject (`content:cardinality.lower`) | ECC-VAL-002 | `composition.rs:134,216` | conformant |
| `CONT-COMP-content_card_3plus-context_any` | content `3..*` | 0,1 reject | ECC-VAL-003 | `composition.rs:140,221` | conformant |
| `CONT-COMP-content_card_opt-context_any` | content `0..1` | 3 reject (`cardinality.upper`) | ECC-VAL-004 | `composition.rs:146,226` | conformant |
| `CONT-COMP-content_card_mand-context_any` | content `1..1` | 0,3 reject | ECC-VAL-005 | `composition.rs:152,231` | conformant |
| `CONT-COMP-content_card_3to5-context_any` | content `3..5` | 0,1 reject | ECC-VAL-006 | `composition.rs:158,236` | conformant |
| `CONT-COMP-content_card_any-context_mand` | content `0..*`, context `1..1` | no-context rows reject (`context occurrences.lower`) | ECC-VAL-007 | `composition.rs:164,241` | conformant |
| `CONT-COMP-content_card_1plus-context_mand` | content `1..*`, context `1..1` | content-lower ∨ context-lower | ECC-VAL-008 | `composition.rs:170,246` | conformant |
| `CONT-COMP-content_card_3plus-context_mand` | content `3..*`, context `1..1` | as above | ECC-VAL-009 | `composition.rs:176,251` | conformant |
| `CONT-COMP-content_card_opt-context_mand` | content `0..1`, context `1..1` | upper ∨ context-lower | ECC-VAL-010 | `composition.rs:182,256` | conformant |
| `CONT-COMP-content_card_mand-context_mand` | content `1..1`, context `1..1` | lower/upper ∨ context-lower | ECC-VAL-011 | `composition.rs:188,261` | conformant |
| `CONT-COMP-content_card_3to5-context_mand` | content `3..5`, context `1..1` | lower ∨ context-lower | ECC-VAL-012 | `composition.rs:194,266` | conformant |

Note (context isolation): `composition.rs:20-24` states the runner relies on the
server **not** enforcing the RM `Category_validity` invariant (persistent/event
⇒ context rules), so a missing `context` is RM-accepted and only the authored
OPT's `context` existence governs the `_mand` rows. Correct for isolating the
occurrences constraint (master15 §Isolation), but a dependency on a validation
gap — §4 G-6.

### 2.2 master16 — OBSERVATION (§OBSERVATION Test Cases, 4 cases)

Schedule matrix per case: `data` ∈ {absent, present} × `state` ∈ {absent,
present} × `protocol` ∈ {absent, present} = **8 rows**. Every `data`-absent row
rejects with `OBSERVATION.data existence.lower (RM/schema constraint)` (data is
mandatory `[1]` on ENTRY); the `state`/`protocol` rejections are **archetype**
existence constraints that vary per case (`_opt` = `0..1`, `_mand` = `1..1`).

Runner: all four cases call `run_obs_data` (`entry.rs:304`) →
`drive::entry_data_existence(ctx,"OBSERVATION")` (`drive.rs:345`), which drives
**only 2 rows**: base (data present) → accept, `data` removed → reject. The
`state`/`protocol` existence dimension is **not authored and not asserted**
(`entry.rs:12-13`, `drive.rs:337-344` explicitly scope to the RM/schema rows).

| Schedule id (master16) | Constraint variant | ECC | `file:line` | Verdict |
|---|---|---|---|---|
| `CONT-OBS-state_ex_opt-protocol_ex_opt` | state `0..1`, protocol `0..1` | ECC-VAL-013 | `entry.rs:69,85,304` | **divergent — under-tests**: only `data`-existence 2 rows driven; state/protocol archetype dim untested (§4 G-1) |
| `CONT-OBS-state_ex_opt-protocol_ex_mand` | state `0..1`, protocol `1..1` | ECC-VAL-014 | `entry.rs:72,304` | **divergent — under-tests** (identical `run_obs_data`; the `protocol` `1..1` narrowing is never authored) |
| `CONT-OBS-state_ex_mand-protocol_ex_opt` | state `1..1`, protocol `0..1` | ECC-VAL-015 | `entry.rs:76,304` | **divergent — under-tests** (state `1..1` never authored) |
| `CONT-OBS-state_ex_mand-protocol_ex_mand` | state `1..1`, protocol `1..1` | ECC-VAL-016 | `entry.rs:80,304` | **divergent — under-tests** (state+protocol `1..1` never authored) |

All four ECC ids run the **same** two-row `data`-existence assertion — the
distinguishing archetype constraint in each case name is not exercised.

### 2.3 master16 — HISTORY (§HISTORY Test Cases, 12 cases)

Schedule matrix per case: `events` ∈ {no, one, three} × `summary` ∈ {absent,
present} = **6 rows**; `events` cardinality is one of the six intervals,
`summary` existence is `_opt` (`0..1`) or `_mand` (`1..1`).

Runner: `drive_hist_case` (`entry.rs:317`) **authors** a `persistent_minimal`
OPT tightening `HISTORY.events` cardinality
(`author::constrain_nested_multiple`, `entry.rs:325`) + a mandatory
`HISTORY.summary` for the `_mand` cases (`entry.rs:331`), then commits {0,1,3}
events with `summary` **removed** (`entry.rs:337-343`) — i.e. only the
`summary`-absent half of each schedule table. Accept iff
`count>=1 && events_ok(card,count) && !summary_mand` (`entry.rs:349`).

Deliberate deviation from the schedule table (`entry.rs:344-348`): the RM
`HISTORY.Events_valid` invariant (≥1 event OR a summary) makes 0 events + absent
summary **reject** regardless of archetype cardinality — this overrides the
master16 `CONT-HIST-events_card_any-summary_ex_opt` "no events, absent summary →
accepted" row. The RM invariant is spec-authoritative over the schedule table
(recorded, not silent) — an **instrument-encodes-server-behaviour** row per §4
G-7.

| Schedule id (master16) | Constraint variant | ECC | `file:line` | Verdict |
|---|---|---|---|---|
| `CONT-HIST-events_card_any-summary_ex_opt` | events `0..*`, summary `0..1` | ECC-VAL-017 | `entry.rs:99,377` | faithful (0-events row overridden by RM `Events_valid`, G-7) |
| `CONT-HIST-events_card_1plus-summary_ex_opt` | events `1..*`, summary `0..1` | ECC-VAL-018 | `entry.rs:103,383` | faithful (summary-present rows undriven, G-5) |
| `CONT-HIST-events_card_3plus-summary_ex_opt` | events `3..*`, summary `0..1` | ECC-VAL-019 | `entry.rs:108,389` | faithful |
| `CONT-HIST-events_card_opt-summary_ex_opt` | events `0..1`, summary `0..1` | ECC-VAL-020 | `entry.rs:113,395` | faithful (G-7 on 0-events) |
| `CONT-HIST-events_card_mand-summary_ex_opt` | events `1..1`, summary `0..1` | ECC-VAL-021 | `entry.rs:118,401` | faithful |
| `CONT-HIST-events_card_3to5-summary_ex_opt` | events `3..5`, summary `0..1` | ECC-VAL-022 | `entry.rs:123,407` | faithful |
| `CONT-HIST-events_card_any-summary_ex_mand` | events `0..*`, summary `1..1` | ECC-VAL-023 | `entry.rs:128,413` | faithful (all summary-absent rows reject on summary-lower) |
| `CONT-HIST-events_card_1plus-summary_ex_mand` | events `1..*`, summary `1..1` | ECC-VAL-024 | `entry.rs:133,419` | faithful |
| `CONT-HIST-events_card_3plus-summary_ex_mand` | events `3..*`, summary `1..1` | ECC-VAL-025 | `entry.rs:138,425` | faithful |
| `CONT-HIST-events_card_opt-summary_ex_mand` | events `0..1`, summary `1..1` | ECC-VAL-026 | `entry.rs:143,431` | faithful |
| `CONT-HIST-events_card_mand-summary_ex_mand` | events `1..1`, summary `1..1` | ECC-VAL-027 | `entry.rs:148,437` | faithful |
| `CONT-HIST-events_card_3to5-summary_ex_mand` | events `3..5`, summary `1..1` | ECC-VAL-028 | `entry.rs:153,443` | faithful |

Coverage note: only the `summary`-absent half of each 6-row schedule table is
driven (the `_mand` cases therefore reject all their rows; the `_opt` cases
never commit a `summary`-present accepted instance). The accept/reject oracle is
preserved for the driven rows but the summary-present positives are untested —
§4 G-5.

### 2.4 master16 — EVENT (§EVENT Test Cases, 5 cases)

Two `state`-existence cases (schedule matrix `data`×`state`, the `data`-absent
rows marked RM/schema) and three type-narrowing cases (POINT_EVENT vs
INTERVAL_EVENT; abstract EVENT accepts either).

| Schedule id (master16) | Constraint variant | ECC | `file:line` | Verdict |
|---|---|---|---|---|
| `CONT-EVENT-state_ex_opt` | EVENT `state` `0..1` | ECC-VAL-029 | `entry.rs:170,309` | **divergent — under-tests**: `run_event_data` drives only the `data`-existence 2 rows; `state` archetype dim untested (§4 G-1) |
| `CONT-EVENT-state_ex_mand` | EVENT `state` `1..1` | ECC-VAL-030 | `entry.rs:176,309` | **divergent — under-tests** (identical `run_event_data`; the `state` `1..1` narrowing is never authored) |
| `CONT-EVENT-type_any` | slot = abstract EVENT | ECC-VAL-031 | `entry.rs:182,454` | conformant — base `POINT_EVENT` accepted in the open slot (`drive_constraint_base`, no authoring) |
| `CONT-EVENT-type_point_event` | slot narrowed to `POINT_EVENT` | ECC-VAL-032 | `entry.rs:188,473` | conformant — authors the narrowing (`author::narrow_nested_child_type`, `entry.rs:478`); base accept, `_type`→INTERVAL_EVENT reject |
| `CONT-EVENT-type_interval_event` | slot narrowed to `INTERVAL_EVENT` | ECC-VAL-033 | `entry.rs:194,520` | conformant — authors the narrowing; a **fabricated** valid INTERVAL_EVENT (base POINT_EVENT + `width`+`math_function`, `entry.rs:537-551`) accept, base POINT_EVENT reject (§4 G-8: fabricated instance) |

`state_ex_opt`/`state_ex_mand` both run the same `run_event_data` — same as the
OBS collapse (G-1).

### 2.5 master16 — ITEM_STRUCTURE (§ITEM_STRUCTURE Test Cases, 5 cases)

Pure type-narrowing: a slot narrowed to one `ITEM_STRUCTURE` subtype accepts
that subtype, rejects the siblings ("Class not allowed"); the abstract slot
accepts any. Driven against the vendored `clinical_content_validation` OPT
(`CLINICAL`, `entry.rs:245`) whose four EVALUATION `data` slots are narrowed to
`ITEM_SINGLE`/`TREE`/`LIST`/`TABLE`.

| Schedule id (master16) | Constraint variant | ECC | `file:line` | Verdict |
|---|---|---|---|---|
| `CONT-ITEM_STR-type_any` | slot = abstract `ITEM_STRUCTURE` | ECC-VAL-034 | `entry.rs:208,584` | conformant — authors a re-opened slot (`retype_attr_child`→`open_complex("ITEM_STRUCTURE")`, `entry.rs:595`); ITEM_TREE + rebuilt ITEM_LIST both accepted |
| `CONT-ITEM_STR-type_item_tree` | slot `ITEM_TREE` | ECC-VAL-035 | `entry.rs:214,279` | conformant — vendored comp accept, `_type`→ITEM_LIST reject (`drive_item_str`, `entry.rs:254`) |
| `CONT-ITEM_STR-type_item_list` | slot `ITEM_LIST` | ECC-VAL-036 | `entry.rs:220,283` | conformant |
| `CONT-ITEM_STR-type_item_table` | slot `ITEM_TABLE` | ECC-VAL-037 | `entry.rs:226,287` | conformant |
| `CONT-ITEM_STR-type_item_single` | slot `ITEM_SINGLE` | ECC-VAL-038 | `entry.rs:232,291` | conformant |

Note: the type-narrowing rejection is only the `_type`-swap sibling; the schedule
"Class not allowed" reason string itself is not asserted (§4 G-3).

---

## 3. ECC comp/entry cases with no schedule home

**None.** Every ECC-VAL-001..038 maps 1:1 to a master15/master16 schedule case
(verified against `tools/conformance/inventory/ecc-catalog.tsv`). ECC-VAL-119
(`val/dv-date-day-disallowed-pattern`) is a `DV_DATE` `C_DATE`-pattern case
belonging to **register 13** (data types), not here. The instrument does **not**
invent composition/entry cases outside the schedule — a clean spine.

The full VAL area is 119 cases; the split relevant to this audit:

- **ECC-VAL-001..012** — master15 COMPOSITION (this register).
- **ECC-VAL-013..038** — master16 ENTRY (this register).
- **ECC-VAL-039..119** — master17.1–17.7 DATA_VALUE, `data_types.rs`
  (**register 13** — 81 cases, not audited here).

---

## 4. G-rows — gaps + rulings for the rewrite

### G-1 (LOAD-BEARING) — OBS/EVENT existence cases under-test their schedule case

The four `CONT-OBS-*` (ECC-VAL-013..016) and two `CONT-EVENT-state_ex_*`
(ECC-VAL-029..030) cases all collapse to a single two-row RM/schema
`data`-existence assertion (`drive.rs:345` `entry_data_existence`). The
distinguishing archetype dimension — `state`/`protocol` existence narrowing
(`0..1` vs `1..1`) — is **never authored and never committed**, so six distinct
ECC ids test the same thing and none exercises the constraint its name claims.
The suite is honest in comment (`entry.rs:12-13`) but the catalogue reads as
full coverage. **Ruling:** the rewrite must author the `state`/`protocol` (and
EVENT `state`) existence OPTs the same way `HISTORY` cardinality is authored
(`author::constrain_nested_single_mandatory` already exists, used at
`entry.rs:331`) and drive the existence rows, or the six ids must be re-scoped
(renamed/merged) so the catalogue does not over-claim. Authoring is preferred
(it closes the gap and matches the master15 "should be generated" intent).

### G-2 (LOAD-BEARING) — authored OPTs are ad-hoc, not declared generated fixtures

`author.rs` generates the constraint OPTs in-memory by parsing a vendored base
OPT into `openehr_its::opt14` and mutating the typed tree per case
(`author.rs:1-21`). This correctly realises master15 §Implementation notes
("archetypes should be generated") — a genuine, ingested, `WebTemplate`-built
OPT, not a fabricated pass. **But** the generation is per-case imperative Rust,
untracked by register 80's owned-fixture manifest, whose ruling is "**generated:
source class is preferred**". **Ruling:** register each authored OPT family in
register 80's manifest as a `generated` fixture with a recorded **source class**
(the base OPT `minimal_evaluation` / `persistent_minimal` +
the tightening transform: attribute + interval/existence), so the provenance is
declared and reproducible rather than buried in `author.rs`. The base OPTs
(`BASE_OPT`, `PERSIST_OPT`, `CLINICAL`) and the corrected `all_types`
compositions (`drive.rs:74-104`, already owned) are the source inputs to name.

### G-3 (LOAD-BEARING) — loose assertions: "constraints violated" unchecked; status + wire edition-pinned

Three loosely-pinned assertions the rewrite should tighten or explicitly mark:

1. **`constraints violated` column unverified.** The schedule names the exact
   violated constraint per reject row (e.g. `COMPOSITION.content:
   cardinality.lower`, "Class not allowed"). `check()` (`drive.rs:279`) asserts
   only accept-vs-reject, never *which* constraint fired. A server that rejects
   for the wrong reason passes. **Ruling:** where the ITS-REST error body is
   structured, assert the violated-path/reason; otherwise record explicitly that
   only the accept/reject verdict is in scope (no ad-hoc body scraping — the
   current code does none: `resp.text()` is used only inside the failure message
   at `drive.rs:288`, never parsed).
2. **Status-code tolerance is VERSION-SPECIFIC.** `check()` accepts `422` **or**
   `400` as "rejected" (`drive.rs:297`). The schedule pins no code (data
   validation says only "rejected"); ITS-REST 1.0.3 `composition_create` pins
   `422` for validation, `400` for malformed. Accepting both is a tolerance that
   must be a declared **VERSION-SPECIFIC** assertion (ITS-REST 1.0.3), not a
   silent OR — a server that returns `400` for a semantic failure is arguably
   non-conformant to 1.0.3.
3. **RM/ADL wire shapes are EDITION/VERSION-SPECIFIC.** The authored OPTs
   serialise to **ADL 1.4** XML (`opt14::to_xml`) and the committed instances
   are **RM 1.2.0** canonical JSON. Per README's version-ladder ruling
   (master03 §API Conformance) versioned assertions should carry per-edition
   forms newest→oldest; these cases carry a single RM 1.2.0 / ADL 1.4 form with
   no ladder. **Ruling:** mark the cases EDITION-SPECIFIC (ADL 1.4 OPT provisioning,
   RM 1.2.0 wire) and, if a lower RM/ADL edition is claimed in a Conformance
   Statement, generate the down-level OPT/instance forms.

### G-4 — stale module doc in `entry.rs`

`entry.rs:1-27` (module doc) describes an **earlier** state: it says HISTORY
cardinality + EVENT type narrowing are "**archetype constraints still
skipped**" and ITEM_STRUCTURE narrowing is an "**open finding** (the SUT accepts
the wrong subtype)… driven and failing". The code has moved past all three —
HISTORY is authored (`drive_hist_case`), EVENT point/interval are authored
(`run_event_point`/`run_event_interval`), and the ITEM_STRUCTURE cases are
driven expecting rejection (and pass at the B6 baseline: 315 pass / 0 fail). The
doc is misleading. **Ruling:** rewrite the module doc to the current behaviour
(and drop the "returning `Skipped`" claim, which no longer holds for these
cases).

### G-5 — dropped schedule dimensions (justified, but record the coverage delta)

Two schedule dimensions are collapsed for the accept/reject oracle:

- **COMPOSITION context** (`composition.rs:92`): schedule has 3 context values
  (no context / context w/o other_context / context w/ other_context); the
  runner drives only present-vs-absent (2). The `other_context` presence never
  flips accept/reject in the schedule tables, so the oracle is preserved, but the
  border variant is untested.
- **HISTORY summary** (`entry.rs:337-343`): only `summary`-absent rows are
  driven, never a `summary`-present accepted instance for the `_opt` cases.

**Ruling:** acceptable coverage reduction (the oracle is invariant across the
dropped values); record it in register 90 as a deliberate data-set scoping so it
is not mistaken for a gap. Add the `summary`-present positive if cheap.

### G-6 — context isolation depends on a validation gap

`composition.rs:20-24` isolates the context-occurrences constraint by relying on
the server **not** enforcing the RM `Category_validity` invariant. If the server
later enforces `Category_validity` (a real spec requirement, RM ehr §COMPOSITION
`Category_validity`), the `context_any` rows that commit a missing `context`
would reject and these cases flip to spurious failures. **Ruling:** make the
isolation explicit — either author a `category=event` base whose missing context
is genuinely RM-legal, or pin the cases to the invariant state and update when
`Category_validity` lands. Flag as fragile.

### G-7 — HISTORY 0-events row: RM invariant overrides the schedule table

`drive_hist_case` (`entry.rs:344-349`) rejects the "no events, absent summary"
row that master16 `CONT-HIST-events_card_any-summary_ex_opt` marks *accepted*,
because RM `HISTORY.Events_valid` (≥1 event OR summary) governs. This is
correct (RM invariant is spec-authoritative over the schedule table) and
recorded, not silent — an **instrument-encodes-server-behaviour** row. **Ruling:**
keep, but surface it in the report as a declared schedule-table deviation with
the RM citation, so the adjudication register (not a code comment) is the record
of why an ECC row diverges from the printed schedule expectation.

### G-8 — fabricated INTERVAL_EVENT instance (ECC-VAL-033)

`run_event_interval` (`entry.rs:534-551`) builds the accepted INTERVAL_EVENT by
mutating the base POINT_EVENT and injecting `width` (PT1H) + `math_function`
(coded `mean`). This is a hand-built RM instance, not a corpus fixture.
**Ruling:** the fabricated instance belongs in register 80's owned-fixture
manifest (a `generated`/`owned` INTERVAL_EVENT composition with source class
recorded), consistent with G-2 — not inline in the suite.
