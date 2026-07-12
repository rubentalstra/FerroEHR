# A1 Spec Audit — Verify + Fix — chapter `sm-tdd`

- **Chapter:** SM SIM-B / Serial Data Formats / TDD transformation duties
- **Date:** 2026-07-11
- **Scope:** all 47 requirements `sm-tdd-R1 … R47`
- **Result (defer-nothing pass):** 1 fix — the SDT **path+terse coded form**
  (`"terminology::code|value|"`) now parses into the full `DV_CODED_TEXT`
  (it previously fell through to the free-text alternative — the one
  silent-misinterpretation risk in the register). The FLAT/STRUCTURED
  surface implements the Better `web-template` dialect as the primary
  oracle per the standing serialization rule (SDT/SDF are
  development-status specs — the SIM-B value classes are literally
  `TODO: define`); the draft-only string encodings are documented as such
  and shown to FAIL CLEANLY (typed miss → 422), never to misparse. Zero
  deferrals.

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1 | verified | all four S_DV_TEXT forms: bare terse + `\|value` regular (FLAT), nested regular/terse (STRUCTURED entry parsing) |
| R2 | fixed | terse coded parse (`parse_terse_coded`: `term::code\|value\|` and value-less `term::code`); ordering corrected in the coded-with-`other` arm (terse first, free-text fallback); round-trip-equality test `terse_coded_text_parses` (Corona corpus template) |
| R3 | verified | `\|code`/`\|terminology`/`\|value` regular form (the emitted shape) |
| R4, R34 | verified-draft-safe | the two contradictory terse quantity delimiters (SDT space vs SDF comma) sit on a class the spec marks `TODO: define`; the regular `\|magnitude`/`\|unit` form is first-class, and a bare string on a quantity slot returns a clean typed miss (`quantity_from_flat` requires `\|magnitude`) — no misparse path exists |
| R5 | verified | `\|magnitude` (number) + `\|unit` (+ precision/status; Better extras quirks-gated) |
| R6 | verified | `\|formalism`/`\|value`; an absent formalism surfaces as the RM `Formalism_valid` violation at commit (mandatory enforced downstream, never silently valid) |
| R7–R14 | verified | `ctx/` handling: language/territory/composer mandatory with documented defaults; time defaults to now; category via the composition-category terminology group (walker); ism-transition state integer; facility = PARTY_IDENTIFIED; workflow id = OBJECT_REF shape |
| R15–R25 | verified | the simplified classes' mandatory attributes are enforced by the post-conversion RM validation (typed deserialize + walker): content/composer/language/territory/name/value/code/terminology/scheme all RM-mandatory |
| R26, R27 | verified | native JSON boolean / integer leaves (`bare_typed`, `count_from_flat`) |
| R28, R29 | fixed (term/term-code) + verified-draft-safe (ODIN bracket) | the `terminology::code\|text\|` structure now parses (R2); the SDF ODIN `[term::code]` bracket rendering is a draft-SDF-only alternative the Better dialect does not use — a bracketed string is not silently split (clean typed handling) |
| R30 | verified | ISO 8601 leaf validity at commit (`valid_iso8601_*` via the RM validation + walker temporal checks) |
| R31 | verified | URI leaves as strings |
| R32, R33 | verified-draft-safe | ordinal/scale use `\|ordinal`/`\|scale` + coded keys; a bare `"1\|…"` string returns a clean typed miss (`ordinal_from_flat` requires the numeric suffix) — no corruption path |
| R35 | verified | proportion kind validated against the template kind set (walker `check_proportion`) + the RM `Type_validity`; the SDF `n/d;KIND` string form is draft-only and cleanly missed |
| R36, R37, R41, R42 | verified | multimedia/identifier suffixed-key objects (the EhrScape `\|`-prefixed variant field set is the same vocabulary) |
| R38 | verified-draft-safe | version-parenthetical terminology ids are carried opaquely in the terminology field (never folded into the code) |
| R39, R40 | verified | JSON arrays with `:n` indexes; interval leaves as objects (the ODIN interval string is draft-SDF-only, cleanly missed) |
| R43 | verified-superior | `formatting` is carried (not skipped) — lossless where the transformation table drops data; interop-superior, PORT-NOTE-worthy not defect |
| R44 | verified | HISTORY collapse/reconstruction driven by the WebTemplate tree — the corpus round-trip suite proves reconstruction |
| R45, R46 | verified | temporal/coded leaf constraint conversions in the WebTemplate builder + the TDD converter (`from_tdd`, corpus-verified at B3) |
| R47 | verified | party-proxy `id`/`id_namespace`/`id_scheme` (GENERIC_ID-guarded) + OBJECT_REF `id_namespace`/`id_type` mappings in the ctx handling |

## Fixes applied

- `crates/openehr-flat/src/flat/mappers.rs` — `parse_terse_coded` +
  `coded_parts` (terse-first ordering in the coded-with-`other` arm);
  test `terse_coded_text_parses`.

## Deferred

None. (The P17 SIM-B/SDF interop audit remains scheduled for breadth —
this chapter closed every register row; the draft-only encodings are
documented spec-status decisions, not open work.)

## Uncertain / runtime probes

None remaining.
