# The openEHR compliance blueprint

The master build documentation for the mission: **the first fully
spec-compliant openEHR CDR**. Written 2026-07-09 from a full extraction of the
vendored specs (`docs/specs/openehr/`) verified line-by-line against the
working tree.

**Start here → [`00-THE-BLUEPRINT.md`](00-THE-BLUEPRINT.md)** — mission, the
one-table compliance map (every spec area × state × remaining work ×
priority), the numbered build order (B1 rebuild → B2 validation depth → B3
SM-5/6 → B4 terminology-server integration → B5 conformance-tooling update →
B6 P19 full conformance), and the standing rules.

## Chapters (per spec component)

Each chapter = normative requirements (spec-cited) → verified implementation
state (DONE/PARTIAL/MISSING with file:line evidence) → ordered remaining work
→ spec defects/TBDs recorded verbatim.

| Chapter | Component | Headline |
|---|---|---|
| [01-rm.md](01-rm.md) | RM 1.2.0 — change control/versioning, EHR, entry, structures, data types, demographic, extract | 54 requirements; versioning audited 1:1; biggest gaps: validation depth, `is_modifiable` guard, EHR Extract (SM-5) |
| [02-base-term.md](02-base-term.md) | BASE 1.3.0 + TERM 3.1.0 — identifiers, intervals, ISO 8601, terminology bundle | 27 requirements; gaps: case-insensitive identifier equality, `Multiplicity_interval`/`Cardinality` impls (feed the validation rock) |
| [03-am.md](03-am.md) | AM 1.4/2.4 — archetype/template constraint semantics | 20 requirements; **the big rock lives here: ArchetypeValidation depth, 81 of 106 ECC failures** |
| [04-query.md](04-query.md) | QUERY (AQL 1.1) | 38 requirements; parser complete; engine gaps: OR-CONTAINS, single-row functions, terminology family |
| [05-its.md](05-its.md) | ITS-REST 1.0.3 + ITS-JSON + ITS-XML | 34 requirements; MUST-level gap: `openEHR-VERSION.*` committal-header merge; plus `Last-Modified`, `OPTIONS /` |
| [06-sm.md](06-sm.md) | SM — Platform Service Model, SIM-B, SDF | 39 requirements; 26 done; missing: Message service (SM-5), Subject Proxy (SM-6), Admin dump/load |
| [07-cnf.md](07-cnf.md) | CNF — conformance framework + the runner audit | 11 requirements + the runner-vs-spec audit: ~17/106 failures are mis-booked runner/spec-gap issues (D1–D6) |

## Related documents

- `docs/GAP_REGISTER.md` — the consolidated gap ledger (proven vs known-missing)
- `docs/plans/current-phase.md` — the live phase pointer under this blueprint
- `docs/ADRs/ADR-008` (greenfield storage), `ADR-010`/`ADR-011` (SM native API)
- `docs/design/sm-platform/` — the SM design set (SM-5/6 designs live here)
- `docs/spec-audit/SPEC_AUDIT.md` — the 2026-07-06 finding-level audit (82 open)

## Maintenance

Update at every phase close: the affected chapter's state table, the
compliance map + build-order status in `00-THE-BLUEPRINT.md`, and the gap
register. State columns record *verified* reality (file:line or ECC evidence),
never intent.
