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

| # | Area | Findings file | Status | crit | major | minor | info |
|---|------|---------------|--------|------|-------|-------|------|
| 01 | REST: EHR / EHR_STATUS / VERSIONED_EHR_STATUS | [findings/01-rest-ehr.md](findings/01-rest-ehr.md) | auditing | – | – | – | – |
| 02 | REST: COMPOSITION / DIRECTORY / CONTRIBUTION | [findings/02-rest-composition-directory-contribution.md](findings/02-rest-composition-directory-contribution.md) | auditing | – | – | – | – |
| 03 | REST: QUERY / DEFINITION / ITEM_TAG / auth | [findings/03-rest-query-definition-tags.md](findings/03-rest-query-definition-tags.md) | auditing | – | – | – | – |
| 04 | Canonical JSON (ITS-JSON) | [findings/04-canonical-json.md](findings/04-canonical-json.md) | auditing | – | – | – | – |
| 05 | Canonical XML (ITS-XML) | [findings/05-canonical-xml.md](findings/05-canonical-xml.md) | auditing | – | – | – | – |
| 06 | Versioning / CONTRIBUTION / AUDIT (change_control) | [findings/06-versioning-contribution.md](findings/06-versioning-contribution.md) | auditing | – | – | – | – |
| 07 | Composition validation (invariants + AOM + TERM) | [findings/07-validation.md](findings/07-validation.md) | auditing | – | – | – | – |
| 08 | AQL 1.1 lexer/parser/AST | [findings/08-aql-parser.md](findings/08-aql-parser.md) | auditing | – | – | – | – |
| 09 | Templates: OPT 1.4 / AOM 1.4 | [findings/09-templates-opt14.md](findings/09-templates-opt14.md) | auditing | – | – | – | – |
| 10 | WebTemplate / FLAT / STRUCTURED (SDT) | [findings/10-webtemplate-flat-sdt.md](findings/10-webtemplate-flat-sdt.md) | auditing | – | – | – | – |
| 11 | Terminology (TERM 3.1.0) | [findings/11-terminology.md](findings/11-terminology.md) | auditing | – | – | – | – |
| 12 | RM/BASE types + spec functions/invariants | [findings/12-rm-base-types.md](findings/12-rm-base-types.md) | auditing | – | – | – | – |
| 13 | Architecture / duplication / hygiene | [findings/13-architecture-hygiene.md](findings/13-architecture-hygiene.md) | auditing | – | – | – | – |

## Fix waves

Planned after triage (populated once all areas report):

- **Wave 1 — critical spec divergences** (wire-visible, CNF-failing)
- **Wave 2 — major divergences + structural rewrites** (full rewrites preferred)
- **Wave 3 — minor divergences, hygiene, dedup/consolidation**

## Triage summary

_(populated after all agents report)_
