---
name: adl2-cadl-primitive-types-location
description: Where every cADL primitive-type constraint form (string/regex, integer/real intervals, date-time/duration patterns, terminology at-/ac-codes + constraint strength, assumed values) is defined in ADL2 master04.5, plus its confirmed defects and cross-spec delegations
metadata:
  type: reference
---

# cADL primitive-type constraints — navigation

Owning file: `AM/docs/ADL2/master04.5-cadl_primitive_types.adoc` (818 lines).
Section → line map: General Structure L6-138 · Assumed Values L140-146 ·
Boolean L148-158 · Character L160-188 · String L190-267 (regex construct
table L234-265) · Ordered types L269-279 · Integer L281-336 · Real L338-354 ·
Date/Time patterns L365-430 (pattern table L383-410, timezone table L416-424) ·
Date/Time intervals L432-450 · Duration patterns L452-473 · Duration
lists/intervals L475-485 · Mixed pattern/interval L487-508 · Terminology
L510-728 (formal L521-598, soft/constraint-strength L600-685, operational
binding `[acN@ns]` L687-726) · Lists of primitives L730-741 · Intervals of
ordered primitives L743-754 · Enumerated types L756-817.

Companions:
- Syntax-error codes for primitives (S-codes, NOT V-codes) =
  `ADL2/master04.6-cadl_validity_rules.adoc` (SCIAV/SCRAV/SCDAV/SCTAV/SCDTAV/
  SCDUAV/SCSAV/SCBAV/SCOAV assumed-value; SCDPT/SCTPT/SCDTPT/SCDUPT patterns —
  **SCDUPT L54 carries the ONLY inline duration-pattern grammar**
  `P[Y|y][M|m][W|w][D|d][T[H|h][M|m][S|s]] or P[W|w] [/duration_interval]`;
  SCSRE regex; STCCP/STCDC/STCAC/STCNT terminology). The file itself says the
  authoritative list is the EXTERNAL `adl-resources .../adl_syntax_errors.txt`.
- AOM side: `AOM2/master04.2-constraint_model-semantics.adoc` L105-117 (the
  C_PRIMITIVE→constraint-type table), L162-164 (String/StringN matching),
  L194-200 (`constraint_status` required0/extensible1/preferred2/example3,
  Void⇒required, redefinition must lower the value), L247
  (`is_enumerated_type_constraint`). Class tables in
  `AM/docs/UML/classes/org.openehr.am.aom2.c_*.adoc`.
- `Primitive_node_id = "at9999"` (`"id9999"` id-coded) is defined ONLY in
  `UML/classes/org.openehr.am.aom2.adl_code_definitions.adoc` L48.
- `matches {*}` "any" form is deprecated-but-accept — `master04.3` L359-360,
  NOT in 04.5. Tuple/second-order constraints = `master04.4` (04.5 never
  mentions tuples).
- Regex escaping/quoting delegated to `ADL2/master03-file_encoding.adoc`
  (L10 names both `//` and `^^` delimiters; L49 backslash handling).

## Confirmed defects / tensions in master04.5 (released text)
1. L119-124 at-coded "Primitive_node_id" example uses `at9017`; AOM2 defines it
   as `at9999` (the id-coded tab correctly uses `id9999`).
2. L74 prose says "id-code, e.g. `id9`" but the adjacent block shows `id3`.
3. L186 "any item from the Character Classes list above" — the tables are
   BELOW (L234-265, in the String section). Forward reference.
4. L311-312 comments invert the inequalities (`|0..<1000|` commented
   "allow 0>= x <1000").
5. L324 `{|>=10|;5}` — assumed value 5 violates the stated constraint; the spec
   nowhere requires an assumed value to satisfy its own constraint (contrast
   STCAC, which DOES require the assumed at-code ∈ ac-code value set).
6. L414 says timezone patterns attach to time/date-time "but not date", yet
   L430's assumed-value example is `yyyy-mm-dd±hh; 1970-01-01+02` (a date).
7. Pattern table L383-410 vs the two generative validity rules L378-379: the
   table omits combinations the rules allow (e.g. `hh:mm:??`, `yyyy-XX-XX`);
   "The following table shows the valid patterns" reads exhaustive.
8. Duration pattern examples are lowercase (`Pd`, `PThm`, data `P5d`) while the
   mixed form uses uppercase (`PWD/|P0W..P50W|`); ISO 8601 designators are
   uppercase. Case rules only implied by SCDUPT's `[Y|y]` alternation.
9. L49 "the type can always be inferred from the syntax alone" contradicts
   L817 (enum constraints need RM inspection to be recognised).
10. Regex table claims to be the supported set yet the prose calls it "a proper
    subset of Perl"; `{,n}` is not valid Perl/PCRE; no `^`/`$` anchors, `\w`,
    `\b`, or non-greedy forms are listed.
11. `^^`-delimited regexes have no AOM representation: `C_STRING.constraint`
    (UML class table) says regexes are "delimited by the '/' character" →
    `^…^` round-trip is undefined/lossy.
12. Explicit TBD at L229-230 (string list vs single regex; "See also AOM spec").
13. `NOTE:` L372 — no way to PROHIBIT a timezone (also true in ADL1.4).

## Grammar gap (same as adl2-parser-spec-location)
Every "shown below in the Base Lexer syntax section" reference
(L374, L456) resolves to `ADL2/masterAppB-syntax_spec.adoc`, which is only
unresolved `include::{openehr_adl_antlr_include}/adl/*.g4[]` directives — the
`.g4` files are NOT vendored. The only grammar text physically present for
primitives is the one-line `duration_constraint` production at master04.5 L507.

**NOT a dangling reference (verified first-hand 2026-08-22 — do not report it as
one).** The 4 sentences citing `DATE_/TIME_/DATE_TIME_/DURATION_CONSTRAINT_PATTERN`
(ADL2 master04.5 **L374 + L456**, ADL1.4 master05-cadl **L856 + L938** — same
wording, 4 sites in 2 chapters) name the rules EXACTLY as the included
`base_lexer.g4` defines them: `crates/openehr-adl/vendor/grammar/v2_4/base_lexer.g4`
**L35-37 + L47** (unprefixed), and ADL1.4's own
`v1_4/cadl14_primitives.g4` **L74-77** too (provenance: openEHR/adl-antlr @
`8db091ec`). The `V_ISO8601_*`-prefixed names appear ONLY in the chapters' own
legacy lex/yacc listings (master05-cadl §Syntax Specification L1024-1450) and in
ZERO vendored `.g4` (grep = 0). Base-Lexer section anchors:
`ADL2/masterAppB-syntax_spec.adoc` L51, `ADL1.4/masterAppC-syntax_spec.adoc` L60.
Trap: reading the chapter-local legacy listing as "the Base Lexer section"
manufactures a false dangling-reference defect.
