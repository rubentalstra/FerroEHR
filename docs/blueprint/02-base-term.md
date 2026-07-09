# Blueprint 2 — BASE + TERM

Component scope: BASE 1.3.0 foundation/base types (`docs/specs/openehr/BASE/docs/`)
and TERM 3.1.0 (`docs/specs/openehr/TERM/`). Oracle = the vendored spec text; all
citations below are repo-relative paths into it. Codebase state verified by
reading/grepping `crates/openehr-base`, `crates/openehr-rm`, `crates/openehr-term`,
`crates/openehr-flat`, `app/*`, plus `docs/GAP_REGISTER.md` and
`docs/spec-audit/SPEC_AUDIT.md` (+ `findings/11-terminology.md`,
`findings/12-rm-base-types.md`). Date: 2026-07-09.

---

## Normative requirements (what a compliant CDR MUST do)

### A. Identifiers — primitive UIDs

- **R1. Three primitive UID subtypes.** Provide `UUID`, `ISO_OID`, `INTERNET_ID`
  under abstract `UID`, all with a string representation.
  — `BASE/docs/base_types/master05-identification_package.adoc` §"Primitive
  Identifiers" (lines 67–73).
- **R2. UID subtype discrimination from the string.** When setting a `UID`-typed
  attribute from a string (DB read, JSON/XML deserialisation), code MUST inspect
  the string and pick the subtype; "all three subtypes have mutually exclusive
  string patterns". — same file, line 73.
- **R3. UID grammar.** `uid = iso_oid | uuid | internet_id`; `iso_oid` =
  dot-separated numbers; `uuid` = five hex groups; `internet_id` per RFC
  1034/1035/2181 with underscores allowed. — same file, §Syntaxes (lines
  228–243).

### B. Identifiers — composite

- **R4. OBJECT_ID hierarchy.** `OBJECT_ID` → `UID_BASED_ID`
  (`HIER_OBJECT_ID`, `OBJECT_VERSION_ID`), `ARCHETYPE_ID`, `TEMPLATE_ID`,
  `TERMINOLOGY_ID`, `GENERIC_ID`; plus references `OBJECT_REF`, `PARTY_REF`,
  `LOCATABLE_REF`. All identifiers are single strings; sub-parts are exposed by
  parsing functions. — master05-identification_package.adoc §Design (line 65),
  §Composite Identifiers (77–89), §References (185).
- **R5. UID_BASED_ID root/extension.** Lexical form
  `root ['::' extension]` where `root` is a `uid`; functions `root()`,
  `extension()`, `has_extension()`. — §Syntaxes lines 244–248;
  `BASE/docs/UML/classes/org.openehr.base.base_types.uid_based_id.adoc`.
- **R6. OBJECT_VERSION_ID structure.** Globally unique version identifier with
  lexical form `object_id '::' creating_system_id '::' version_tree_id`;
  functions `object_id(): UID`, `creating_system_id(): UID`,
  `version_tree_id(): VERSION_TREE_ID`, `is_branch(): Boolean`. Example:
  `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2`. `object_id` is the
  version-container uid; `creating_system_id` must be "unique per system"
  (UUID, reverse-domain, or OID — all via `UID`).
  — `org.openehr.base.base_types.object_version_id.adoc` (whole file);
  master05-identification_package.adoc §"Identifying Versions within openEHR
  Versioned Containers" (lines 138–154) + grammar lines 250–253.
- **R7. VERSION_TREE_ID.** Lexical form
  `trunk_version ['.' branch_number '.' branch_version]`; numbering of all
  three parts starts at 1; invariants `Trunk_version_valid` (integer ≥ 1),
  `Branch_number_valid`/`Branch_version_valid` (integer ≥ 1 if present),
  `Branch_validity` (branch number and version both absent xor both present),
  `Is_branch_validity`, `Is_first_validity` (`trunk_version = "1"`).
  — `org.openehr.base.base_types.version_tree_id.adoc` (invariants lines
  45–65); grammar master05 lines 255–259.
- **R8. ARCHETYPE_ID multi-axial grammar.**
  `qualified_rm_entity '.' domain_concept '.v' version_id` with
  `qualified_rm_entity = rm_originator '-' rm_name '-' rm_entity` and
  `domain_concept = concept_name {'-' specialisation}`; `version_id` numeric.
  All parts immutable; two versions of an archetype are two distinct
  archetypes. — master05 §"Archetype Identifiers" (93–115) + grammar 261–270.
- **R9. TERMINOLOGY_ID syntax.** `name-str ['(' name-str ')']` — a globally
  unique name with optional parenthesised version, e.g. `"SNOMED-CT"`,
  `"ICD9(1999)"`, `"ICD10AM(3rd_ed)"`. — master05 §"Terminology Identifiers"
  (117–132) + grammar line 273.
- **R10. Case rules for composite identifiers.** All composite identifiers MUST
  be **case-preserving** (never change case through persistence/copy/transfer)
  and **case-insensitive** ("two identifiers identical apart from case are
  considered to be identical, and therefore to identify the same thing").
  — master05 §"Composite Identifiers and Case" (lines 164–177).
- **R11. OBJECT_REF for distributed references.** Refer to identified objects
  via `OBJECT_REF` (id + namespace + type, where type may be the concrete type
  or any proper ancestor); `LOCATABLE_REF` adds a path and `as_uri()`.
  — master05 §References (line 185);
  `org.openehr.base.base_types.locatable_ref.adoc`.

### C. Intervals

- **R12. Interval<T> semantics.** Abstract `Interval<T>` with `lower`/`upper`
  (0..1), `lower_unbounded`/`upper_unbounded`, `lower_included`/`upper_included`
  and functions `has(e)`, `intersects(other)`, `contains(other)`, `is_equal`.
  Invariants: `Lower_included_valid` (`lower_unbounded implies not
  lower_included`), `Upper_included_valid` (mirror), `Limits_consistent`
  (bounded ⇒ `lower <= upper`), `Limits_comparable` (bounded ⇒ limits strictly
  comparable). — `org.openehr.base.foundation_types.interval.adoc` (lines
  14–87).
- **R13. Point vs proper intervals.** `Point_interval<T>` and
  `Proper_interval<T>` are the concrete runtime types for any `Interval<X>`
  model slot; `Multiplicity_interval` and `Cardinality` are derived types for
  multiplicity/optionality/cardinality in models (i.e. AM constraint
  evaluation). — `BASE/docs/foundation_types/master05-interval.adoc` (lines
  5–7).

### D. ISO 8601 time types (partial-date semantics)

- **R14. String-represented ISO 8601 (2019) types** for date, time, date-time,
  duration, timezone, supporting **partial** and **extended** semantics.
  — `BASE/docs/foundation_types/master06-time_types.adoc` lines 5, 15.
- **R15. Accept both extended and compact forms.** Extended (`yyyy-mm-dd`,
  `hh:mm:ss`) is strongly recommended for writing/display, "Nevertheless, both
  forms should be supported" for straight-through processing of legacy data.
  — master06-time_types.adoc line 19.
- **R16. Partial dates.** `Iso8601_date` values may be `YYYY`, `YYYY-MM`
  (/`YYYYMM`), `YYYY-MM-DD` (/`YYYYMMDD`); invariants `Year_valid`,
  `Month_valid` (if known), `Day_valid: not day_unknown implies
  valid_day(year, month, day)`, `Partial_validity: month_unknown implies
  day_unknown`. Only 4-digit years (no ISO 'expanded' dates); no week-dates
  (`YYYY-Www-D`). — `org.openehr.base.foundation_types.iso8601_date.adoc`
  (lines 9–15, 100–111); master06-time_types.adoc lines 23–24.
- **R17. Partial date-times beyond ISO.** Partial `Iso8601_date_time` may omit
  hours, days, months (a deliberate openEHR deviation from ISO 8601:2019
  §4.3.3 c, matching HL7v2/v3 TS). Only fractional **seconds** are supported
  (no fractional minutes/hours). — master06-time_types.adoc lines 25, 32–34.
- **R18. `24:00:00` is rejected everywhere** ("The time `24:00:00` (or
  `240000`) is not allowed anywhere"); midnight is `00:00:00`.
  — master06-time_types.adoc line 35.
- **R19. Duration deviations.** The `W` (week) designator MAY be combined with
  the other designators (pregnancy durations); durations MAY take a leading
  negative sign (e.g. `'-P3M'` adjusted age). — master06-time_types.adoc lines
  30–31.
- **R20. Definite vs nominal date arithmetic.** Date/time types define
  `add`/`subtract`/`diff` (definite, using `Time_definitions`
  `Average_days_in_month`/`Average_days_in_year`) and
  `add_nominal`/`subtract_nominal` (calendar semantics: `'P1Y'` = same date
  next year etc.). — master06-time_types.adoc lines 47–60;
  iso8601_date.adoc lines 66–98.

### E. Foundation terminology types

- **R21. Terminology_code.** `terminology_id: String [1]`,
  `terminology_version: String [0..1]`, `code_string: String [1]`,
  `uri: Uri [0..1]` — a standalone reference to any referenceable terminology
  entity (single term, value set, or other). `Terminology_term` = code + one
  rubric. — `org.openehr.base.foundation_types.terminology_code.adoc` (whole
  file); `BASE/docs/foundation_types/master07-terminology.adoc` lines 5–13
  (the package also defines `CODE_PHRASE`, line 25).

### F. TERM — openEHR terminology content

- **R22. Two kinds of coded entities.** (a) **Code sets** — self-describing
  codes with no rubric (ISO languages/countries, IANA character sets/media
  types, plus openEHR-internal `normal_statuses`, `compression_algorithms`,
  `integrity_check_algorithms`), represented as `CODE_PHRASE`; (b)
  **vocabulary groups** — true coded terms with per-language rubrics,
  represented as `DV_CODED_TEXT`, each group identified by a logical name
  (e.g. "audit change type"). — `TERM/docs/SupportTerminology/
  master02-overview.adoc` lines 5–7; master03-terminology.adoc lines 5–15.
- **R23. XML representation is the computable form.** A single XML file for all
  code sets (`<codeset issuer=… openehr_id=… external_id=…><code value=…/>`)
  and one XML file **per translation** of the vocabularies
  (`<group openehr_id=…><concept id=… rubric=…/>`); the repository's
  `computable/XML` content "is the definitive expression".
  — master04-representation.adoc lines 27–101; master02-overview.adoc line 3.
- **R24. Codes are per-group; rubrics are group-scoped and language-scoped.**
  Concept ids are numeric and may recur across groups with different rubrics
  (see the id=532 SPECPR-51 case below); rubric lookup must therefore be
  scoped by group **and** language; code validity is language-independent.
  — master04-representation.adoc lines 11, 65–99; the vendored
  `TERM/computable/XML/en/openehr_terminology.xml` lines 137/195.
- **R25. RM coded attributes MUST use group codes, not rubrics.** A
  `CODE_PHRASE.code_string` bound to an openEHR group carries the numeric
  concept id (e.g. `audit_change_type`: 249 creation, 250 amendment, 251
  modification, 252 synthesis, 523 deleted, 666 attestation, 253 unknown);
  the rubric is the display text. — master04-representation.adoc lines 74–82.
- **R26. Access via `rm.support.terminology` service interfaces.** "Access to
  the terminology in the openEHR reference model is via the classes defined in
  the package `rm.support.terminology`" (TERMINOLOGY_SERVICE /
  TERMINOLOGY_ACCESS / CODE_SET_ACCESS, incl. the named identifier constants
  `Terminology_id_openehr`, `Group_id_*`, `Code_set_id_*`).
  — master02-overview.adoc line 3; RM support
  `org.openehr.rm.support.openehr_terminology_group_identifiers.adoc` /
  `…openehr_code_set_identifiers.adoc` (cited via SPEC_AUDIT F-11-07).
- **R27. The TERM model is non-normative for internal representation.** "this
  model is not intended as a normative model for internal terminology
  representation in openEHR services" — internal shape is free; behaviour
  (R22–R26) is the contract. — master04-representation.adoc line 5.

---

## Current implementation state (verified, not assumed)

Legend: **DONE** (evidence) / **PARTIAL** (what is missing) / **MISSING**.

| Req | State | Evidence |
|---|---|---|
| R1 UID subtypes | **DONE** | Generated `crates/openehr-base/src/base_types/identification/{uuid,iso_oid,internet_id,uid}.rs` + hand-written `iso_oid_impl.rs`, `internet_id_impl.rs`. |
| R2 UID discrimination | **DONE** | `identification/lexical.rs:72 make_uid()` picks the subtype from the string (`is_oid` line 89; test `make_uid_picks_subtype` line 105). |
| R3 UID grammar | **DONE** | `lexical.rs` (`all_digits`, `is_positive_int`, `is_oid`) + `internet_id_impl.rs`/`iso_oid_impl.rs` format invariants; strict `FromStr` per SPEC_AUDIT F-12-03 (fixed 2026-07-06 W2-L). |
| R4 OBJECT_ID hierarchy | **DONE** | All 16 identification classes generated (incl. `access_group_ref.rs`, `template_id.rs`, `generic_id.rs`); `uid_based_id.rs`/`object_id.rs` as closed enums. |
| R5 root/extension | **DONE** | `uid_based_id_impl.rs`: `root`/`extension`/`has_extension` on HIER_OBJECT_ID, OBJECT_VERSION_ID and the `UidBasedId` enum (F-12-03 fix note). |
| R6 OBJECT_VERSION_ID | **DONE** | `object_version_id_impl.rs`: `object_id`/`creating_system_id`/`version_tree_id`/`is_branch`, strict three-part `FromStr`, `Value_format_valid` invariant wired into `validate_rm_value` (F-12-03 ✅). App consolidated onto it: `ehrbase::service::version_id` over `openehr_base::ObjectVersionId` (SPEC_AUDIT W2-B, "malformed `::` shapes now rejected"); per-version `creating_system_id` proven in storage (GAP_REGISTER §1, PR #33). |
| R7 VERSION_TREE_ID | **DONE** | `version_tree_id_impl.rs`: format invariant + `trunk_version`/`is_branch`/`is_first`/`branch_number`/`branch_version` + `FromStr`, branch segments ≥ 1 per `Branch_*_valid` (F-12-03 fix note). |
| R8 ARCHETYPE_ID | **DONE** | `archetype_id_impl.rs`: `qualified_rm_entity`/`domain_concept`/`rm_originator`/`rm_name`/`rm_entity`/`specialisation`/`version_id` + strict `FromStr` (F-12-07 ✅). |
| R9 TERMINOLOGY_ID | **DONE** | `terminology_id_impl.rs`: `name`/`version_id` (F-12-07 ✅). Note: AQL front end once could not lex hyphenated terminology ids (SPEC_AUDIT summary item 9) — front-end concern, tracked there. |
| R10 Identifier case rules | **PARTIAL** | Case-*preserving*: yes (identifiers are opaque `value: String`, never re-cased). Case-*insensitive equality*: **missing** — `grep -rn "eq_ignore\|to_lower" crates/openehr-base/src/base_types/identification/` matches nothing but a doc comment; all comparisons are derived `PartialEq` (byte-exact). Two `OBJECT_VERSION_ID`s differing only in UUID hex case (`…4E3D…` vs `…4e3d…`) compare unequal and would miss on lookup. No SPEC_AUDIT finding covers this. |
| R11 OBJECT_REF family | **DONE** | `object_ref_impl.rs`, `party_ref_impl.rs`, `locatable_ref_impl.rs` (`as_uri`, F-12-07 ✅). |
| R12 Interval invariants + functions | **PARTIAL** | `proper_interval_impl.rs` enforces `Lower/Upper_included_valid` **and** `Limits_consistent` (via `PartialOrd`, lines 19–37); `point_interval_impl.rs` present. RM-side `DV_INTERVAL` enforces `Limits_consistent` over DV_ORDERED magnitudes and provides `has()`; `REFERENCE_RANGE.is_in_range()` exists (F-12-04 ✅, `dv_ordered_impl.rs` + `dv_interval_impl.rs`). **Missing:** `has`/`intersects`/`contains`/`is_equal` on the BASE `Interval`/`Point_interval`/`Proper_interval` types themselves (grep: no such fns in `point_interval_impl.rs`; 6 fns total in `proper_interval_impl.rs`, all invariant plumbing). |
| R13 Multiplicity_interval / Cardinality | **PARTIAL** | Generated (`multiplicity_interval.rs`, `cardinality.rs`) but **no `*_impl.rs`** — none of their functions (`is_open`, `is_optional`, occurrence math) exist. These are the constraint-evaluation primitives for the **81 failing ArchetypeValidation ECC cases** (GAP_REGISTER §2.1). |
| R14 ISO 8601 types | **DONE** (by policy) | Foundation `Iso8601_*` types generated as bare `{value: String}`; the wire carries strings validated by hand-written helpers in `crates/openehr-rm/src/validate.rs`. Recorded as a known non-gap in SPEC_AUDIT F-12-11 ✅ ("near-zero fidelity impact"). |
| R15 Extended + compact | **DONE** | `validate.rs:139 is_valid_iso_date` accepts `YYYY[-MM[-DD]]` and `YYYYMMDD`/`YYYYMM`; `is_valid_time_core` accepts `HH:MM:SS` and `HHMMSS` forms (lines 175–205). |
| R16 Partial dates | **PARTIAL** | Partials `YYYY`/`YYYY-MM` accepted; `Partial_validity` holds structurally; `Month_valid` enforced (1–12). **`Day_valid` is not calendar-exact**: day only range-checked 1–31 (`validate.rs:144`), so `2021-02-31` passes, violating `valid_day(year, month, day)` (iso8601_date.adoc line 107). Wired into DV_DATE/DV_DATE_TIME `Value_valid` (`dv_date_impl.rs`). |
| R17 Partial date-times | **DONE** | `is_valid_iso_date_time` (`validate.rs:226`): `T`-less value = date partial; time part accepts bare `HH`/`HH:MM` (missing hours/days/months per the openEHR deviation); fractional seconds only (fraction parsed after the seconds field). |
| R18 Reject 24:00 | **DONE** | `is_valid_time_core` caps hours at 0–23 in both extended and compact branches (`validate.rs`, `in_range(h, 0, 23)`), so `24:00:00`/`240000` are rejected. |
| R19 Duration deviations | **DONE** | `is_valid_iso_duration` (`validate.rs:263`): optional leading `+`/`-`, `W` mixed freely with `Y/M/D` (`parse_duration_components(date_part, b"YMWD", …)`), fractional components with `.`/`,`. Duration magnitude uses `Average_days_in_year` 365.24 / `Average_days_in_month` 30.42 per BASE `Time_definitions` (F-12-04/F-12-11 fix notes). |
| R20 Date arithmetic fns | **MISSING** (accepted) | No `add`/`subtract`/`diff`/`add_nominal` on any date type. F-12-11 ✅ records this as a known non-gap "until a consumer appears"; DV_ORDERED magnitude/comparison (the part with live consumers) landed in `dv_ordered_impl.rs`, and the indexed path is the `openehr_magnitude` SQL function (ADR-008). |
| R21 Terminology_code/term | **DONE** | Generated `foundation_types/terminology/{terminology_code,terminology_term,code_phrase}.rs` matching the BASE 1.3.0 shape. |
| R22 Code sets + vocabularies | **DONE** | `crates/openehr-term` bundle: all 17 vocabulary groups (14 RM + 3 EHR_EXTRACT), 3 internal code sets, 4 external code sets parsed and reachable; embedded assets **byte-identical** to `docs/specs/openehr/TERM/computable/XML/` (verified by `diff` — SPEC_AUDIT findings/11 Summary). `bundle.rs` exposes `is_valid_code`, per-group validators (`is_valid_audit_change_type` … lines 263–438), `is_valid_language`/`is_valid_country`/`is_valid_external_code`, `code_set()`, property↔unit lookups (`PropertyUnitData.xml`). |
| R23 XML as computable form | **DONE** | Assets `crates/openehr-term/assets/{en,es,ja,pt,zh}/openehr_terminology.xml` + `openehr_external_terminologies.xml` + `PropertyUnitData.xml` + `schema/`; single parser path (the FHIR mirror deliberately not consumed — findings/11 hygiene note). |
| R24 Group-scoped rubric incl. SPECPR-51 | **DONE** | `bundle.rs:253 rubric(group_id, code, lang)`; the id=532 dual-rubric quirk proven by test `version_lifecycle_state_specpr51_quirk`; service emits the group-correct rubric ("complete" for `ORIGINAL_VERSION.lifecycle_state` 532 in `versioned.rs`) — findings/11 Summary. Code validity resolved against canonical `en` only (language-independent), rubrics per-language. |
| R25 Codes not rubrics on the wire | **DONE** | F-11-01 (audit `change_type` emitted rubric as `code_string`) **fixed** — numeric group codes stored/emitted, rubric as `value` (findings/11, checked box). Terminology-bound RM invariants now validated: `DV_TEXT.language/encoding` (F-11-02 ✅), `ISM_TRANSITION.transition` (F-11-03 ✅), `TERM_MAPPING.purpose` (F-11-04 ✅), `DV_ORDERED.normal_status` + `PARTY_RELATED.relationship` (F-11-05 ✅) — all in `crates/openehr-flat/src/validation/terminology.rs`. |
| R26 Terminology service interfaces | **PARTIAL** | The SM native-API trait exists: `app/ehrbase-sm/src/services/terminology.rs` (`trait TerminologyService`, line 237, mirroring `i_terminology_service.adoc`, with `TerminologyDescription`/`TermCode`/`DefinedTerm` extract model). The bundle's flat `OpenehrTerminology` API is an accepted structural deviation (ADR-006/008; findings/11 Summary "every spec operation has a semantic equivalent"). **Missing:** wire exposure — "EHR Index / Terminology wire exposure (extension OAS)" is *designed, not built* (GAP_REGISTER §2.3, design `docs/design/sm-platform/08-target-architecture.md` §7). Spec identifier constants (`Terminology_id_openehr`, `Group_id_*`, `Code_set_id_*`) not exposed; consumers hardcode `"openehr"` etc. — **F-11-07 open**. Internal code-set membership is an O(n) scan (`bundle.rs:382`) — **F-11-06 open** (hygiene). |
| R27 Internal shape free | **DONE** | By construction (own struct set `terminology/{terminology,code_set,code,terminology_group,terminology_concept,terminology_status}.rs` mirroring the TERM model classes of master04-representation.adoc). |

Cross-cutting, verified adjacent state: version **branching** is deliberately
trunk-only (`is_branch` parses correctly but the CDR never creates branches) —
deliberate deferral, GAP_REGISTER §2.4 ("Version branching (trunk-only,
F-06-09) + version merging out of scope"). The emitter inconsistency on
`DV_ORDERED.normal_range` monomorphisation (**F-12-08 open**, info) and the
single `serde_json::Value` degradation in experimental `ehr_extract`
(**F-12-09 open**, minor) are the only structural blemishes in generated
BASE/RM (zero `Value` fallbacks in generated BASE — findings/12 Summary).

---

## Remaining work (ordered, concrete)

1. **Case-insensitive composite-identifier equality (R10)** — the only
   *unregistered* conformance gap found by this chapter. Add
   spec-cited case-insensitive `is_equal`/lookup normalisation for
   `UID_BASED_ID`/`OBJECT_VERSION_ID` (and a canonicalisation rule at the
   storage boundary — e.g. lowercase UUID hex on ingest, preserving the
   original for echo, per master05 §"Composite Identifiers and Case").
   Verify: an ETag/version lookup with case-flipped UUID hex must hit.
   File a new SPEC_AUDIT finding (area 12) so it is tracked.
2. **`Multiplicity_interval`/`Cardinality` function impls (R13)** — write
   `multiplicity_interval_impl.rs` + `cardinality_impl.rs` (`is_open`,
   `is_optional`, `has`, upper-unbounded handling). Do this **inside the
   ArchetypeValidation push** (GAP_REGISTER §2.1: 81 failing ECC cases —
   occurrence/cardinality evaluation is exactly this type's job).
3. **Calendar-exact `Day_valid` (R16)** — tighten
   `openehr-rm/src/validate.rs::is_valid_iso_date` to
   `valid_day(year, month, day)` (month lengths + leap years) per
   `iso8601_date.adoc` invariant `Day_valid`; add `2021-02-31`/`2021-04-31`
   reject tests (both extended and compact forms).
4. **Expose TERM spec identifier constants (F-11-07, R26)** — a
   `terminology_id::OPENEHR` + `group_id::*` + `code_set_id::*` module in
   `openehr-term`; migrate the hardcoded literals in `versioned.rs`,
   `contribution.rs`, `openehr-flat/validation/terminology.rs`.
5. **Index internal code sets (F-11-06)** — `HashSet` index for
   `normal_statuses` etc., mirroring `external_codes`.
6. **Terminology wire exposure** — build the designed extension-OAS surface
   over the `TerminologyService` trait (GAP_REGISTER §2.3; design 08 §7).
   Sequenced with SM close, before/at P19.
7. **BASE `Interval` function surface (R12)** — `has`/`intersects`/`contains`
   on `Proper_interval`/`Point_interval` when a consumer appears (likely the
   same ArchetypeValidation push as item 2); note the defective spec
   postcondition below before implementing (implement the *meaning*, cite the
   defect).
8. **Emitter determinism for `normal_range` (F-12-08)** — emit
   `DvInterval<DvOrdered>` uniformly; regenerate; low priority, no wire impact.
9. **Date arithmetic (R20)** — remains consumer-driven per F-12-11 ✅; if AQL
   temporal functions (P16 envelope growth) or EHR_EXTRACT need
   `add/diff/nominal`, implement in `*_impl.rs` against master06-time_types
   §Computational Functions, keeping parity with `openehr_magnitude` SQL.

---

## Spec defects/TBDs encountered (verbatim, cited)

1. **Typo in the identifier-groups table** —
   `BASE/docs/base_types/master05-identification_package.adoc` line 83:
   > `|`OBJECT_VERSION_ID`            |`TERMINOLOGY_ID`` +
   > `|`ARCHEYTPE_ID`                 |`GENERIC_ID``

   ("ARCHEYTPE_ID" for `ARCHETYPE_ID`.)

2. **Garbled `Interval.has()` postcondition (parameter mismatch + precedence)** —
   `BASE/docs/UML/classes/org.openehr.base.foundation_types.interval.adoc`
   lines 48–53: the parameter is declared `e: T[1]` but the postcondition uses
   `v`, and the boolean grouping is inconsistent:
   > `__Post_result__: `Result = (lower_unbounded or lower_included and v >= lower) or v > lower and (upper_unbounded or upper_included and v \<= upper or v < upper)``

   Read literally this is not a correct containment predicate; the intended
   meaning (lower test **and** upper test) must be implemented instead.

3. **`Iso8601_date.month()/day()` precondition contradicts its meaning** —
   `org.openehr.base.foundation_types.iso8601_date.adoc` lines 29–38:
   > `*month* (): … __Pre__: `not month_unknown`` … "Extract the month part of
   > the date as an Integer, **or return 0 if not present**."

   A precondition forbidding absence and a documented 0-return for absence
   cannot both hold.

4. **TERM Maintenance section is empty (TBD)** —
   `TERM/docs/SupportTerminology/master06-maintenance.adoc`, entire content:
   > `= Maintenance`
   >
   > `== Assignment of New Codes`

   No body text: the code-assignment process is unspecified in the vendored
   TERM 3.1.0 text.

5. **SPECPR-51 — concept id 532 has two rubrics** (spec-acknowledged defect,
   handled group-scoped by our bundle) —
   `TERM/computable/XML/en/openehr_terminology.xml` lines 137 and 195:
   > `<concept id="532" rubric="complete"/><!-- warning: the rubric for this concept is 'completed' in the 'instruction states' group (known issue, see SPECPR-51) -->`
   > `<concept id="532" rubric="completed"/><!-- warning: the rubric for this concept is 'complete' in the 'version lifecycle state' group (known issue, see SPECPR-51) -->`

6. **Nonconforming legacy archetype version ids acknowledged by the spec** —
   master05-identification_package.adoc line 115:
   > `WARNING: some archetype authoring tools have historically allowed a nonconforming version part within archetype identifiers which included the lifecycle status. This has led to some archetypes having an incorrect identifier whose version part is of the form `.v1draft` or similar.`

   (Implication: an ingest path may encounter `.v1draft`-style ids; our strict
   `ARCHETYPE_ID` parser rejects them — any tolerance decision needs a
   `// PORT NOTE:`.)

7. **Typographic quotes inside a VERSION_TREE_ID invariant** —
   `org.openehr.base.base_types.version_tree_id.adoc` line 64:
   > `__Is_first_validity__: `not is_first xor trunk_version.is_equal(“1”)``

   (Curly quotes where the expression language expects `"1"`; cosmetic but
   verbatim-nonparseable.)

8. **TERM model non-normativity note** (a scoping statement, not a defect, but
   load-bearing for implementers) — master04-representation.adoc line 5:
   > "Note that this model is not intended as a normative model for internal
   > terminology representation in openEHR services."
