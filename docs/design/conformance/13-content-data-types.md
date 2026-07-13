# Content: data types (`suites/content/data_types.rs`): spec-first register

W-10 conformance register (read-only, 2026-07-13) for the **data-type
data-validation** slice of `tools/conformance`. Method (owner ruling
2026-07-13, mirror of `docs/design/platform/`): the spine is the governing CNF
schedule chapters `master17.1`–`master17.7`, enumerated test-case-by-test-case
with citation; the existing ECC-VAL cases are mapped **onto** each schedule
case with a `file:line` verdict (conformant / divergent / open-finding /
missing); cases with no schedule home are flagged (§3); §4 carries the
gaps/rulings for the rewrite. Register 12 (`12-content-composition-entry.md`)
owns the master15/16 composition+entry cases and the shared `author.rs` /
`drive.rs` / `mutate.rs` driving machinery cited below; this register covers
only master17.

**Spec oracles** (read before any change; do not hand-edit):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master17.1-content_tc_data_types-basic.adoc`
  — DV_BOOLEAN, DV_IDENTIFIER, DV_STATE.
- `…/master17.2-content_tc_data_types-text.adoc` — DV_TEXT, DV_CODED_TEXT, DV_PARAGRAPH.
- `…/master17.3-content_tc_data_types-quantity.adoc` — DV_ORDINAL, DV_SCALE,
  DV_COUNT, DV_QUANTITY, DV_PROPORTION, DV_INTERVAL<T> (the 1421-line chapter).
- `…/master17.4-content_tc_data_types-date_time.adoc` — DV_DURATION, DV_TIME, DV_DATE, DV_DATE_TIME.
- `…/master17.5-content_tc_data_types-time_specification.adoc` — DV_GENERAL_TIME_SPECIFICATION,
  DV_PERIODIC_TIME_SPECIFICATION (a 12-line stub, **zero test cases**; see §2.5).
- `…/master17.6-content_tc_data_types-encapsulated.adoc` — DV_PARSABLE, DV_MULTIMEDIA.
- `…/master17.7-content_tc_data_types-uri.adoc` — DV_URI, DV_EHR_URI.
- `…/guide/master03-overview.adoc` §Data Validation conformance — the methodology
  ("committing variable data sets against reference validity").
- `…/profiles/master03-profiles.adoc` §Functional — capability **Archetype
  Validation** = **CORE ✔ / STANDARD ✔** (EHR Persistence component). Every
  master17 case is scored under Archetype Validation; it is a CORE-claim
  capability, so a divergence here bears on the CORE verdict.

**Fixed contract** (do not change): the ECC ids `ECC-VAL-039`…`ECC-VAL-119`
are allocated in `tools/conformance/inventory/ecc-catalog.tsv` (never reused —
retire, don't delete). The register maps them; the rewrite may re-home or
add data sets under the same ids.

---

## 1. Verdict

The schedule defines **81 test-case headings across master17.1–17.7, but only
80 distinct case ids** (DV_TEXT-`validate_open` is duplicated in master17.2 —
G-6). The suite carries **81 ECC-VAL cases** (`ECC-VAL-039`…`ECC-VAL-119`):
**80 map 1:1 onto the distinct schedule ids** and **one (`ECC-VAL-119`) has no
schedule home** — an added negative case for a corrected fixture (§3).
Coverage of the *set of data types* is therefore **complete**: every schedule
case id is exercised at a live endpoint, none skipped, none fabricated (the
driving machinery returns `Skipped`/`Assertion` findings, never a masked pass —
`drive.rs:279` `check`).

The **depth** is where the instrument is thin, and it is thin in one
structural way: the schedule expresses each case as a **truth table of many
data-set rows** (a DV-type instance × constraint-combination → accept/reject),
totalling **≈1,130 explicitly-marked accept/reject rows** across the 80 cases
(the largest single tables: `DV_TIME-validate_range` **200 rows**,
`DV_DATE_TIME-validate_constraint` **176**, `DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint`
**68**). The suite drives **≈160 variants total — a fixed ~2 per case** (one
in-constraint accept + one out-of-constraint reject). That is a **≈7:1
data-set collapse** (G-2): each case verifies that the SUT can *discriminate*
one valid from one invalid instance for that constraint, not that it walks the
full boundary table. This is a deliberate, defensible reduction, but the
coverage bound must be **logged per case at the rewrite**, never left silent.

Two families are genuinely **divergent** (test a substitute, not the case's
constraint variant):

- **DV_INTERVAL<T> — all 27 cases (`ECC-VAL-068`…`095`)** drive one generic
  path (`data_types.rs:1357` `drive_interval`): retype a scratch leaf to an
  open `DV_INTERVAL` and assert only the RM `Interval` invariant
  `lower ≤ upper`. The per-variant bound/range/list constraints
  (`C_INTEGER.range` on bounds, `C_DV_QUANTITY.list`, temporal bounds,
  proportion kinds) **and** the interval-semantics invariants
  (`lower_included_valid`, `upper_included_valid`) the schedule tables turn on
  are **never exercised** — the suite itself flags this (`data_types.rs:1330`).
  This is the register's biggest depth gap (G-1).
- **The `validate_open` cases that carry a real semantic constraint** —
  DV_URI (RFC3986 validity), DV_EHR_URI (`ehr:` scheme), DV_PROPORTION (the RM
  kind invariants `valid_denominator`/`unitary_validity`/… ), DV_DATE_TIME
  (partial-value rejection) — are driven by `data_type_mandatory`
  (`drive.rs:388`), which only removes a mandatory RM field. The
  constraint-of-interest is not exercised, so these are conformant on the
  RM-mandatory dimension but **divergent on the case's headline dimension**
  (G-3).

The temporal constraint/range cases (`ECC-VAL-097`…`108` non-open) are marked
in the suite as **open findings** ("our validator currently defers temporal
range/pattern enforcement", `data_types.rs:1156`) — but the B2 close note
claims temporal-interval enforcement landed, so their run-state is
**ambiguous** and must be re-verified (G-3). Four data types carry **zero
schedule cases** by design (DV_STATE, DV_PARAGRAPH, and both
time_specification types — not used/supported by modelling tools); no ECC case
is owed and none exists (§2.5).

---

## 2. The spine (schedule case → ECC mapping)

Columns: **schedule id** (`CONT-…`, the `====` heading) · **rows** = schedule
truth-table dimensions (accept/reject explicitly-marked outcomes; multi-table
cases summed) · **ECC** (`ECC-VAL-NNN` / `data_types.rs` run fn) · **verdict**
(**C** conformant — the case's constraint variant is driven and the validator
enforces it; **C·m** conformant-but-partial — a substitute or subset of the
schedule table; **D** divergent — the headline constraint variant is not
exercised; **F** open-finding — validator defers, recorded as a finding).
Every ECC case = capability **ArchetypeValidation (CORE + STANDARD)**, driven
via ITS-REST `composition_create` (`201` accept / `422`|`400` reject,
`drive.rs:279`).

### 2.1 master17.1 — basic (5 cases → `ECC-VAL-039`…`043`)

| schedule id | rows a/r | ECC · run fn | verdict |
|---|---|---|---|
| CONT-DV_BOOLEAN-anything_allowed | 2/0 | 039 · `open_dv_boolean` (`:522`, `data_type_mandatory`) | C·m — no reject row in the schedule (open); ECC substitutes the RM `value`-mandatory reject |
| CONT-DV_BOOLEAN-only_true_allowed | 1/1 | 040 · `run_dv_boolean_true` (`:922`, authored `C_BOOLEAN {true}`) | C |
| CONT-DV_BOOLEAN-only_false_allowed | 2/0 | 041 · `run_dv_boolean_false` (`:948`) | C |
| CONT-DV_IDENTIFIER-validate_all_pattern | 4/8 (4 attr tables) | 042 · `run_dv_identifier_pattern` (`:1012`, `C_STRING` on `id`) | C·m — only the `id` attribute + 2 rows; `issuer`/`assigner`/`type` tables uncovered |
| CONT-DV_IDENTIFIER-validate_all_list | 4/8 (4 attr tables) | 043 · `run_dv_identifier_list` (`:1038`) | C·m — as above |

DV_STATE: schedule records **0 cases** (NOTE: "not used and not supported by
modeling tools"). No ECC owed (§2.5).

### 2.2 master17.2 — text (6 headings / 5 distinct → `ECC-VAL-044`…`048`)

| schedule id | rows a/r | ECC · run fn | verdict |
|---|---|---|---|
| CONT-DV_TEXT-validate_open (1st) | 2/1 | 044 · `open_dv_text` (`:519`) | C·m — RM `value`-mandatory only; the `C_STRING`-open rows not driven |
| CONT-DV_TEXT-validate_open (2nd — **duplicate id**, G-6) | 1/2 | — (collapsed into 044) | — schedule defect: same id, `C_STRING.pattern XYZ` table |
| CONT-DV_TEXT-validate_list | 1/2 | 045 · `run_dv_text_list` (`:817`, authored `C_STRING.list`) | C |
| CONT-DV_CODED_TEXT-validate_open | 2/3 | 046 · `open_dv_coded_text` (`:523`, `defining_code`-mandatory) | C·m — `code_string`/`terminology_id` combination table not driven |
| CONT-DV_CODED_TEXT-validate_local_codes | 1/4 | 047 · `run_dv_coded_local` (`:630`, `drive_constraint` `ALL_TYPES_V2` `C_CODE_PHRASE local`) | C |
| CONT-DV_CODED_TEXT-validate_ext_term | 1/4 | 048 · `run_dv_coded_ext_term` (`:1830`, direct `C_CODE_PHRASE SNOMED-CT`) | C·m — substitutes a direct external `C_CODE_PHRASE` for the schedule's `CONSTRAINT_REF`→`ac`-code `constraint_binding` path (G-7) |

DV_PARAGRAPH: schedule records **0 cases** (NOTE: not used/supported). No ECC owed.

### 2.3 master17.3 — quantity (47 cases → `ECC-VAL-049`…`095`)

**Scalars (`049`…`067`).**

| schedule id | rows a/r | ECC · run fn | verdict |
|---|---|---|---|
| CONT-DV_ORDINAL-validate_open | 2/3 | 049 · `open_dv_ordinal` (`:529`) | C·m — `value`-mandatory only |
| CONT-DV_ORDINAL-validate_constraint | 1/2 | 050 · `run_dv_ordinal_constraint` (`:602`, `C_DV_ORDINAL.list`) | C (enforced) |
| CONT-DV_SCALE-validate_open | 2/3 | 051 · `run_dv_scale_open` (`:1733`, retype leaf) | C·m — RM≥1.1.0-only (G-8 version note) |
| CONT-DV_SCALE-validate_constraint | 1/2 | 052 · `run_dv_scale_constraint` (`:1763`, **`C_REAL.list` substitute**) | C·m — no `C_DV_SCALE` exists in AM 1.4 (schedule NOTE / SPECPR-381); ECC substitutes a `C_REAL` on `value` |
| CONT-DV_COUNT-validate_open | 4/1 | 053 · `open_dv_count` (`:520`) | C |
| CONT-DV_COUNT-validate_range | 1/4 | 054 · `run_dv_count_range` (`:848`, `C_INTEGER.range`) | C |
| CONT-DV_COUNT-validate_list | 1/4 | 055 · `run_dv_count_list` (`:868`, `C_INTEGER.list`) | C |
| CONT-DV_QUANTITY-validate_open | 4/3 | 056 · `open_dv_quantity` (`:530`) | C·m — `magnitude`-mandatory only; the `units`-mandatory row not driven |
| CONT-DV_QUANTITY-validate_property | 4/4 | 057 · `run_dv_quantity_property` (`:1305`, `C_DV_QUANTITY.property`) | C |
| CONT-DV_QUANTITY-validate_property_units | 4/5 | 058 · `run_dv_quantity_units` (`:576`, `drive_constraint` `C_DV_QUANTITY.list`) | C (enforced) |
| CONT-DV_QUANTITY-validate_property_units_mag | 2/7 | 059 · `run_dv_quantity_units_mag` (`:662`, FLAT→canonical, mag-range + unit-list, **3 rows**) | C (enforced) |
| CONT-DV_PROPORTION-validate_open | 5/14 | 060 · `open_dv_proportion` (`:531`, `numerator`-mandatory) | **D** — the 14 RM kind-invariant rejects (`valid_denominator`, `unitary_validity`, `percent_validity`, `fraction_validity`, `is_integral_validity`, `type_validity`) are the case's whole point and are **not exercised** (G-3) |
| CONT-DV_PROPORTION-validate_ratio | 1/4 | 061 · `run_dv_proportion_ratio` (`:1120`, `C_INTEGER.list {0}` via `drive_proportion_kind`) | C |
| CONT-DV_PROPORTION-validate_unitary | 1/4 | 062 · `run_dv_proportion_unitary` (`:1124`) | C |
| CONT-DV_PROPORTION-validate_percent | 1/4 | 063 · `run_dv_proportion_percent` (`:1128`) | C |
| CONT-DV_PROPORTION-validate_fraction | 1/4 | 064 · `run_dv_proportion_fraction` (`:1138`) | C |
| CONT-DV_PROPORTION-validate_integer_fraction | 1/4 | 065 · `run_dv_proportion_integer_fraction` (`:1142`) | C |
| CONT-DV_PROPORTION-validate_any_fraction | 2/3 | 066 · `run_dv_proportion_any_fraction` (`:722`, `drive_constraint` `C_INTEGER.list {3,4}`) | C |
| CONT-DV_PROPORTION-validate_ratio_range | 1/3 | 067 · `run_dv_proportion_ratio_range` (`:1526`, `C_REAL.range` on numerator) | C·m — denominator `C_REAL.range` table not driven |

**DV_INTERVAL<T> (`068`…`095`, 27 cases) — all via `drive_interval` (`:1357`);
every one asserts only RM `Interval.lower ≤ upper` (accept `[l,u]` / reject
`[u,l]`). Verdict D across the board** — the schedule's bound/range/list
constraint variant is a substitute, and the interval-semantics `_included`
invariants (present in every `validate_open` table) are not driven either
(G-1).

| schedule id | rows a/r | ECC · run fn (`iv_*`) |
|---|---|---|
| CONT-DV_INTERVAL_DV_COUNT-validate_open | 9/3 | 068 · `ivc_open` |
| CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper | 4/3 | 069 · `ivc_lu` |
| CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper_list | 4/3 | 070 · `ivc_lul` |
| CONT-DV_INTERVAL_DV_QUANTITY-validate_open | 7/3 | 071 · `ivq_open` |
| CONT-DV_INTERVAL_DV_QUANTITY-validate_upper_lower | 4/3 | 072 · `ivq_ul` |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_open | 11/16 | 073 · `ivdt_open` |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint | 18/47 (**68 rows**) | 074 · `ivdt_luc` |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_range | 8/16 | 075 · `ivdt_lur` |
| CONT-DV_INTERVAL_DV_DATE-validate_open | 4/4 | 076 · `ivd_open` |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_constraint | 8/21 | 077 · `ivd_luc` |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_range | 1/3 | 078 · `ivd_lur` |
| CONT-DV_INTERVAL_DV_TIME-validate_open | 4/4 | 079 · `ivt_open` |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_constraint | 1/4 | 080 · `ivt_luc` |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_range | 6/3 | 081 · `ivt_lur` |
| CONT-DV_INTERVAL_DV_DURATION-validate_open | 4/5 | 082 · `ivdu_open` |
| CONT-DV_INTERVAL_DV_DURATION-validate_constraint | 2/9 (35-row table) | 083 · `ivdu_c` |
| CONT-DV_INTERVAL_DV_DURATION-validate_range | 3/7 | 084 · `ivdu_r` |
| CONT-DV_INTERVAL_DV_ORDINAL-validate_open | 2/4 | 085 · `ivo_open` |
| CONT-DV_INTERVAL_DV_ORDINAL-validate_constraint | 1/6 | 086 · `ivo_c` |
| CONT-DV_INTERVAL_DV_SCALE-validate_open | 2/4 | 087 · `ivs_open` |
| CONT-DV_INTERVAL_DV_SCALE-validate_constraint | 1/6 | 088 · `ivs_c` |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_open | 1/2 (18-row) | 089 · `ivp_open` |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio | 1/1 (12-row) | 090 · `ivp_ratio` |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_unitary | 1/1 (12-row) | 091 · `ivp_unitary` |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_percentage | 1/1 (12-row) | 092 · `ivp_percent` |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_fraction | 1/1 (12-row) | 093 · `ivp_fraction` |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_integer_fraction | 1/1 (12-row) | 094 · `ivp_intfrac` |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio_range | 1/2 (18-row) | 095 · `ivp_ratiorange` |

### 2.4 master17.4 — date_time (13 cases → `ECC-VAL-096`…`108`)

| schedule id | rows a/r | ECC · run fn | verdict |
|---|---|---|---|
| CONT-DV_DURATION-validate_open | 10/4 | 096 · `open_dv_duration` (`:539`) | C·m — `value`-mandatory only |
| CONT-DV_DURATION-validate_fields | 9/9 | 097 · `run_dv_duration_fields` (`:1257`, `C_DURATION` pattern `PTHMS`) | F — suite defers temporal enforcement (`:1156`); reverify (G-3) |
| CONT-DV_DURATION-validate_range | 14/7 | 098 · `run_dv_duration_range` (`:1273`, `C_DURATION.range`) | F |
| CONT-DV_DURATION-validate_fields_range | 2/7 | 099 · `run_dv_duration_fields_range` (`:1285`) | F |
| CONT-DV_TIME-validate_open | 14/9 | 100 · `open_dv_time` (`:538`) | C·m — `value`-mandatory only; the ISO8601-validity rows not driven |
| CONT-DV_TIME-validate_constraint | 27/36 (**70 rows**) | 101 · `run_dv_time_constraint` (`:1221`, `C_TIME` pattern) | F |
| CONT-DV_TIME-validate_range | 64/128 (**200 rows**) | 102 · `run_dv_time_range` (`:1233`, `C_TIME.range`) | F — largest schedule table → 2 driven rows (G-2) |
| CONT-DV_DATE-validate_open | 3/7 | 103 · `open_dv_date` (`:537`) | C·m — ISO8601-validity rows not driven |
| CONT-DV_DATE-validate_constraint | 8/7 | 104 · `run_dv_date_constraint` (`:1197`, `C_DATE` pattern) | F |
| CONT-DV_DATE-validate_range | 3/6 | 105 · `run_dv_date_range` (`:1209`, `C_DATE.range`) | F |
| CONT-DV_DATE_TIME-validate_open | 17/12 | 106 · `open_dv_date_time` (`:521`) | C·m — `value`-mandatory only |
| CONT-DV_DATE_TIME-validate_constraint | 63/102 (**176 rows**) | 107 · `run_dv_date_time_constraint` (`:745`, partial-value reject) | **F — explicit open finding** (SUT accepts the partial value the table rejects, `:22`) |
| CONT-DV_DATE_TIME-validate_range | 14/23 | 108 · `run_dv_date_time_range` (`:1245`, `C_DATE_TIME.range`) | F |

### 2.5 master17.5 — time_specification (0 cases) — recorded verbatim

The chapter is a **12-line stub with no test cases**. Recorded in full so the
rewrite never mistakes the gap for an omission:

> `=== DV_GENERAL_TIME_SPECIFICATION` — "TBD: this data type might not be used
> or supported by modeling tools"
> `=== DV_PERIODIC_TIME_SPECIFICATION` — "TBD: this data type might not be used
> or supported by modeling tools"

No ECC case is owed and none exists. Same disposition as **DV_STATE**
(master17.1, "not used and not supported by modeling tools") and
**DV_PARAGRAPH** (master17.2, "not used or supported"). These four
zero-case types are **conformant-by-absence**; the rewrite must keep them
case-free unless the schedule adds cases on a spec bump (G-5).

### 2.6 master17.6 — encapsulated (4 cases → `ECC-VAL-109`…`112`)

| schedule id | rows a/r | ECC · run fn | verdict |
|---|---|---|---|
| CONT-DV_PARSABLE-validate_open | 1/3 | 109 · `open_dv_parsable` (`:540`, `value`-mandatory) | C·m — the `formalism`-mandatory reject row not driven |
| CONT-DV_PARSABLE-validate_value_formalism | 3/4 | 110 · `run_dv_parsable_formalism` (`:976`, `C_STRING.list` on `formalism`) | C·m — the `value` `C_STRING` pattern/list rows not driven |
| CONT-DV_MULTIMEDIA-validate_open | 1/3 | 111 · `open_dv_multimedia` (`:541`, `media_type`-mandatory) | C·m — the `size`-mandatory and media-type-codeset rows not driven |
| CONT-DV_MULTIMEDIA-validate_media_type | 4/4 | 112 · `run_dv_multimedia_media_type` (`:1794`, `C_CODE_PHRASE` list) | C·m — the `size` `C_INTEGER.list`/`.range` half of the table not driven |

### 2.7 master17.7 — uri (6 cases → `ECC-VAL-113`…`118`)

| schedule id | rows a/r | ECC · run fn | verdict |
|---|---|---|---|
| CONT-DV_URI-validate_open | 10/3 | 113 · `open_dv_uri` (`:547`, `value`-mandatory) | **D** — the schedule's point is RFC3986 validity (invalid `xyz` rejected, 9 valid URIs accepted); ECC drives only `value`-mandatory removal (G-3) |
| CONT-DV_URI-validate_pattern | 1/1 | 114 · `run_dv_uri_pattern` (`:1569`, retype + `C_STRING.pattern`) | C |
| CONT-DV_URI-validate_list | 1/1 | 115 · `run_dv_uri_list` (`:1602`, `C_STRING.list`) | C |
| CONT-DV_EHR_URI-validate_open | 6/11 | 116 · `run_dv_ehr_uri_open` (`:1635`, retype + `value`-mandatory) | **D** — the schedule's point is the `ehr:` scheme rule (10 non-`ehr` URIs rejected); ECC drives only `value`-mandatory (G-3) |
| CONT-DV_EHR_URI-validate_pattern | 1/2 | 117 · `run_dv_ehr_uri_pattern` (`:1659`, `C_STRING.pattern`) | C |
| CONT-DV_EHR_URI-validate_list | 1/2 | 118 · `run_dv_ehr_uri_list` (`:1692`, `C_STRING.list`) | C |

---

## 3. ECC-VAL data-type cases with no schedule home

| ECC | id · run fn | disposition |
|---|---|---|
| **ECC-VAL-119** | `val/dv-date-day-disallowed-pattern` · `run_dv_date_day_disallowed` (`data_types.rs:782`) | **Added negative case, no schedule case.** Guards the corrected `all_types` fixture (owner ruling 2026-07-09 B2, `testdata/fixtures/REGISTER.md`): the vendored `all_types.composition.json` carries a day-bearing `DV_DATE` at a leaf whose OPT `C_DATE` pattern is `yyyy-??-XX` (day **disallowed**, AOM 1.4 `org.openehr.am.aom14.c_date.adoc`). A spec-correct validator must `422` it (EHRbase/archie is lenient and accepts it). Legitimate ECC extension (own numbering/taxonomy law); keep it, but at the rewrite tag it as an ECC-authored case, not a schedule case, in `schedule_ref` (empty). |

No other unmapped VAL data-type case: `ECC-VAL-039`…`118` (80 cases) map 1:1
onto the 80 distinct schedule ids; `ECC-VAL-119` is the single extra.

---

## 4. G-rows (gaps + rulings for the rewrite)

**G-1 — DV_INTERVAL<T>: 27 cases collapsed to one generic RM-invariant probe
(`ECC-VAL-068`…`095`).** All 27 run `drive_interval` (`data_types.rs:1357`),
asserting only `Interval.lower ≤ upper` on an open retyped leaf. Neither the
per-variant **constraint** (bound `C_INTEGER.range`/`.list`,
`C_DV_QUANTITY.list`, temporal bounds, proportion-kind `C_INTEGER.list`) nor
the interval-semantics **invariants** (`lower_included_valid`,
`upper_included_valid` — present in every `validate_open` table) are exercised.
The suite flags its own limitation (`:1330`, "require DV_INTERVAL constraint
support the validator does not yet have"). **Ruling:** the rewrite needs (a) a
`DV_INTERVAL` constraint carrier in the authored-OPT path (register 12's
`author.rs`) and (b) data sets driving the three interval invariants +
representative bound rows per variant. Until then these 27 stay **divergent**
and must be reported so in the run (not silently green).

**G-2 — data-set collapse (~1,130 schedule rows → ~160 driven variants,
~2/case).** The schedule expresses conformance as truth-table boundary
coverage; the suite drives one accept + one reject per case. The extreme cases
— `DV_TIME-validate_range` (200 rows), `DV_DATE_TIME-validate_constraint`
(176), `DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint` (68) — are
each a single accept/reject pair. **Ruling (owner: coverage bounds logged,
never silent):** the rewrite records **per case** `schedule_rows` vs
`driven_variants` (the `rows a/r` column here is the source count) in the
report, so the collapse is a visible, auditable number. Decide per DV-type
whether to expand toward full-table coverage (cheap for scalar leaves, costly
for the 200-row temporal tables) — but the count must always ship.

**G-3 — `validate_open` / temporal cases exercise a substitute, not the
headline constraint.** `data_type_mandatory` (`drive.rs:388`) only removes a
mandatory RM field, so the *semantic* validation each of these cases targets is
untested: DV_URI **RFC3986 validity** (`ECC-VAL-113`), DV_EHR_URI **`ehr:`
scheme** (`116`), DV_PROPORTION **RM kind invariants** (`060`), and the ISO8601
field-range rows of DV_TIME/DV_DATE/DV_DATE_TIME opens. Separately, the
temporal constraint/range cases (`097`…`108`) are marked **open findings**
("validator defers temporal range/pattern enforcement", `data_types.rs:1156`)
while the B2 close note claims temporal-interval enforcement landed — a
**stale-comment ambiguity**. `DV_DATE_TIME-validate_constraint` (`107`) is an
**explicit** open finding (partial value accepted, `:22`). **Ruling:**
re-verify each F case's live run-state at the rewrite; add a semantic-validity
data set to the URI/EHR_URI/PROPORTION opens; reconcile the suite doc comment
with the actual validator state and, where the SUT still accepts an invalid
instance, keep it a reported finding (never a masked pass or skip).

**G-4 — EDITION- / VERSION-SPECIFIC assertions (version-ladder inputs, README
owner ruling).** Log these so the runner's newest→oldest ladder can carry them:
(a) **DV_SCALE** (`051`/`052`, and interval-scale `087`/`088`) requires **RM ≥
1.1.0** — schedule NOTE (master17.3 §DV_SCALE, SPECRM-19); on an RM < 1.1.0 SUT
these cases must not run (edition finding, not a fail). (b) **Rejection status
codes are ITS-REST-pinned**: `422` (semantic) or `400` (malformed) both count
as a valid refusal (`drive.rs:297`, `composition_create.yaml`) — pinned to
ITS-REST 1.0.3; a different edition may narrow this. (c) **`C_DV_SCALE` does
not exist** in AM 1.4 (SPECPR-381) — `052` substitutes a `C_REAL` on `value`;
tie this to the AM version, not hard-coded.

**G-5 — zero-case types recorded, not omitted (DV_STATE, DV_PARAGRAPH,
DV_GENERAL/PERIODIC_TIME_SPECIFICATION).** Four types carry no schedule case by
explicit NOTE. The rewrite keeps them case-free and re-checks on every CNF
re-vendor; a spec bump that adds cases is the only trigger to add ECC ids.

**G-6 — schedule defect: DV_TEXT-`validate_open` duplicated.** master17.2
carries the heading `==== Test Case CONT-DV_TEXT-validate_open` **twice**
(lines 9 and 22 — the second is the `C_STRING.pattern XYZ` table). 81 headings,
80 distinct ids. ECC folds both into `ECC-VAL-044`, so the `C_STRING.pattern`
rows of the second table are uncovered. **Ruling:** record the spec defect
verbatim (per standing rule 2 — spec defects noted, never silently guessed) and
add a distinct ECC id for the second table's pattern data set at the rewrite.

**G-7 — DV_CODED_TEXT-`validate_ext_term` substitutes the binding path.**
`ECC-VAL-048` constrains with a direct external `C_CODE_PHRASE` (`SNOMED-CT`)
rather than the schedule's `CONSTRAINT_REF` → `ac`-code → template
`constraint_binding` path (master17.2 §validate_ext_term NOTE:
`CONSTRAINT_REF`→`C_CODE_REFERENCE` with `referenceSetUri`). Functionally close
but the binding-resolution surface is untested — coverage note for the
terminology-binding work (register 11).
