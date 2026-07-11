# A1 full spec audit — consolidated findings

- **Audit window:** 2026-07-11 → 2026-07-12, branch `claude/a1-spec-audit`
  (develop @ 717585c85). Brief: `docs/plans/a1-spec-audit-PROMPT.md`.
- **Oracle:** the vendored spec text at `docs/specs/openehr/` exclusively;
  CNF test data outranks prose readings (corpus adjudication before any new
  rejection). Every requirement and fix cites spec file + section.
- **Register:** 24 chapters, **1,126 extracted requirements** (418
  high-risk), each verified to a per-row verdict in
  `<chapter>/verification.md`. **Zero deferrals** (owner ruling: defer
  nothing) — every gap was fixed in its chapter pass; spec-silent or
  draft-spec areas carry explicit reasoned verdicts, never open ends.

## Result summary (fixes per chapter, in audit order)

| # | chapter | fixes | headline |
|---|---------|-------|----------|
| 1 | rm-common-change-control | 9 families | **Version-tree branching + merge provenance** (storage redesign: trunk/branch columns, per-lineage non-overlap, auto-fork on foreign-csid modification, stored `preceding_version_uid` + `other_input_version_uids`); audit invariants; strict committer relationship; AQL `VERSION.uid` was built from live config instead of the stored per-version `creating_system_id` |
| 2 | rm-ehr | 7 | EHR_ACCESS validation (Scheme_valid); `is_modifiable` scoping; folder OBJECT_REF item shape; ehr summary served stored `system_id` |
| 3 | rm-composition | 1 systemic | the five "present ⇒ non-empty" list invariants (JSON-level — absent ≡ empty post-deserialize) |
| 4 | rm-data-structures | 3 | ITEM_TABLE `Row_regularity`; CLUSTER.items presence; uniform HISTORY event-data types |
| 5 | rm-data-types-text-quantity | 3 | **DV_TEXT/DV_CODED_TEXT had NO invariant enforcement at all**; blank DV_SCALE symbol adjudication |
| 6 | rm-data-types-rest | 2 | DV_ENCAPSULATED/DV_MULTIMEDIA code-set invariants (charset/language/compression/integrity); DV_PERIODIC_TIME_SPECIFICATION formalism |
| 7 | rm-support | 4 | **ARCHETYPE_ID / TERMINOLOGY_ID had NO lexical validation**; version-tree csid comparison was case-sensitive; UCUM syntax validator built |
| 8 | rm-demographic | 3 | party list invariants; relationship container-ref discipline |
| 9 | rm-ehr-extract | 6 | export ignored `include_multimedia`/`link_depth`/`extract_type`; demographics chapter; import Item_validity |
| 10 | rm-integration | 1 | GENERIC_ENTRY invariant dispatch |
| 11 | base-foundation | 1 | timezone bounds were symmetric (spec: +14/−12 asymmetric) |
| 12 | base-base-types | 1 | AQL archetype-id comparison was case-sensitive (the last unfolded composite-identifier seam) |
| 13 | am-aom14-opt | 12 check families | the AOM 1.4 artefact-invariant pass at OPT upload (Existence_set, Members_valid, VARID/VARDT, VDFAI, Target_path_valid, VACDF, C_BOOLEAN, Assumed_value_valid, temporal/duration Pattern_validity, STCDC), corpus-adjudicated |
| 14 | am-aom2-adl2 | 1 subsystem | **the ADL2 surface had ZERO AOM2 validity enforcement** — built the registration validator (section splitter + ODIN-subset reader + 20 rule codes, 23 tests) |
| 15 | term | 3 | RM identifier-constant classes (`valid_terminology_group_id`/`valid_code_set_id` did not exist); `C_DV_QUANTITY.property` group check; every membership re-verified byte-exact |
| 16 | query-aql | 8 families | **the single-row function set was parsed but rejected at SQL generation** (now executes, PG18-verified); TERMINOLOGY boolean + URI forms; duplicate variables silently accepted; variable case-folding; `LIMIT 0`; LIKE escapes; SUM/AVG typing |
| 17 | sm-platform | 2 | bundle `subsumes` returned identity-true (strict subsumption is uniformly false on a flat vocabulary); subject-variable naming validity |
| 18 | sm-tdd | 1 | the SDT path+terse coded form fell through to free-text (the one silent-misinterpretation risk); draft-only encodings shown to fail cleanly |
| 19 | its-rest-general | 1 | `Prefer: resolve_refs` had no trace despite the B6 row claiming closure — implemented end-to-end |
| 20 | its-rest-ehr-composition | 2 | body-uid/path cross-check on composition PUT; client-supplied CONTRIBUTION uid was silently ignored (now honoured/409) |
| 21 | its-rest-query-definition-admin | 1 | REST ADL2 template upload replaced where the contract declares 409 (SM-native replace kept; divergence-by-surface documented) |
| 22 | its-json | 0 | verified via the generated `_type` machinery + fail-closed typed validation + fidelity gates; the unknown-key tolerance is the documented corpus-adjudicated superset |
| 23 | its-xml | 0 | verified by construction (XSD-driven codegen) + the round-trip/C14N/attribute gates |
| 24 | cnf-cross-check | 0 | every ECC-uncovered CNF behaviour has non-ECC test evidence |

## Cross-cutting corrections

- **Citations:** every file touched had ADR citations replaced with openEHR
  spec citations or explicit "no openEHR spec governs this" flags (owner
  hard rule, now in CLAUDE.md/rules); all `ehr`/`ext` migrations rewritten
  clean (no create-then-alter).
- **Corpus adjudications recorded:** terminology-name spaces (`SNOMED CT`),
  UCUM commit-time rejection OFF (°C/mmHg/pH in CNF data), `±00:00`
  timezones, Ocean placeholder property `0`, `DV_PROPORTION.is_integral` /
  `EVENT.offset` legacy attributes, CONSTRAINT_REF-without-definitions,
  VARID tooling tolerances (`v1.0.0`, parenthesized concepts), the
  closed-slot `.*`-excludes idiom.
- **Owner rules encoded during the audit:** spec-only citations; no
  `use X as Y` renaming; `urlencoding` crate for all percent codecs;
  greenfield no-backward-compat.

## Follow-up (outside the A1 register)

- **H1 — legacy ADR-citation sweep:** ~1,000 ADR mentions remain in
  pre-audit files (generated-header provenance + the eventing/tenancy/
  FHIR/multimedia extension modules). The rule is enforced as
  scrub-on-touch + never-add-new; a dedicated reviewed sweep (incl. the
  generated-header template in the emitter) is queued as its own pass.
