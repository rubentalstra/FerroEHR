---
name: adl2-parser-spec-location
description: Where ADL2/cADL2/ODIN source-parser grammar + rules live in the vendored AM/LANG specs, and the vendored-text grammar gap
metadata:
  type: reference
---

ADL2 (ADL 2.4) parser spec lives at `docs/specs/openehr/AM/docs/ADL2/`.
File-to-topic map:
- `master03-file_encoding.adoc` — UTF-8 only official; `\r \n \t \\ \" \'` escapes; regex delimiters `//` or `^^`; unicode `\uHHHH`/`\uHHHHHHHH`.
  ESCAPE DETAIL (verified 2026-07-31): the chapter has exactly two sections,
  §File Encoding (L25-30: the `\u` escaped **UTF-16** rule — `\uHHHH` for U+0000-U+FFFF,
  `\uHHHHHHHH` for U+10000-U+10FFFF, "the algorithm is described in IETF RFC 2781")
  and §Special Character Sequences (L36-49: the 6-escape list + "**Any other character
  combination starting with a backslash is illegal**"). Those two sections CONTRADICT
  each other on `\u` — confirmed released-text defect, present identically in BOTH
  ADL2 master03 and ODIN `LANG/docs/odin/master03-basics.adoc` (L13-16 + L34-43, verbatim
  twins). No `\b`/`\f`/`\0`/`\/` escape exists in either text. The 8-digit form's meaning
  (surrogate pair per RFC 2781 vs zero-filled scalar) is AMBIGUOUS in the released text;
  "`\u` escaped UTF-16" + the RFC 2781 pointer + the fact the "same as the code-point
  number" gloss is given ONLY for the 4-digit case favour the surrogate-pair reading.
- `master04.1..04.6` — cADL: overview / basics (keywords, comments `--`, symbol equivalents ∈/∼/∗) / complex types (existence, cardinality, occurrences, any, subtype narrowing, use_node proxy, use_archetype external ref, allow_archetype slots include/exclude) / second-order (tuple `[a,b]∈{[{},{}]}`, group, tuple path child-index) / primitive types (all leaf constraint syntaxes) / **validity rules catalogue (SUNK,SARID,SASID...STCNT)**.
- `master05-paths.adoc` — ADL path grammar `path: '/'? path_segment ('/' path_segment)+ ; path_segment: attr_name ('[' object_id ']')? ;` movable `//`.
- `master06-default_values.adoc` — `_default = (TYPE) <odin>` or `(json) <# ... #>`.
- `master07.01..07.14` — ADL artefact sections: 04 basics (keywords, node/term/value-set code prefixes id/at/ac, dot-specialisation), 05 identification (HRID, adl_version, rm_release, uid/build_uid, namespaces, version N.M.P-alpha.N/-rc.N, generated/controlled), 06 specialise, 07 language, 08 description, 10 definition, **11-adl_rules.adoc = the INCLUDED/current rules chapter** (verified 2026-07-31 against `master.adoc` L123; the also-present `11-adl_rulesNEW.adoc`, with `check`/`defined()`/a `symbols` section, is NOT included by master.adoc and has no grammar backing — earlier note here said the opposite and was wrong), 12 rm_overlay, 13 terminology (ODIN: term_definitions/value_sets/term_bindings), 14 annotations.
- `master09.02-spec_concepts.adoc` — specialisation levels: **spec level = number of '.' in node id**; specialisation paths; redefinition catalogue table.
- `master10-templates.adoc` — template / template_overlay / operational_template + component_terminologies.
- `masterAppB-syntax_spec.adoc` — normative grammar appendix.

ODIN spec (used in language/description/terminology/rm_overlay/annotations/rules-symbols + terminal value types): `docs/specs/openehr/LANG/docs/odin/`, esp. `master07-leaf_data.adoc` (literal syntax for String/Integer/Real/Boolean/Date-Time/Duration/interval `|N..M|`/URI/coded-term `[terminology_id::code]`/lists) and `masterAppB-syntax_spec.adoc`.

**GRAMMAR GAP (in the SPEC TEXT):** masterAppB is `include::`-only — it references external ANTLR4 `.g4` files in openEHR/adl-antlr; they are NOT under `docs/specs/`. So the vendored *spec text* gives prose + examples + lexer-rule NAMES (DATE_CONSTRAINT_PATTERN etc.) but no productions; top-level section ORDERING appears only in examples + an image (adl_text_overview.svg).
**CORRECTION (verified 2026-07-30):** the normative `.g4` files ARE vendored in-repo at **`crates/openehr-adl/vendor/grammar/`** (adl2.g4, cadl2.g4, cadl2_primitives.g4, adl_keywords.g4, base_lexer.g4, base_expressions.g4, odin*.g4, PCRE.g4, adl14/cadl14*.g4, + PROVENANCE.md) — reference input only, no ANTLR runtime. Use them for prose-vs-grammar checks. Key facts: `base_lexer.g4` L66-70 `ROOT_ID_CODE:'id1' '.1'*`, `ID_CODE/AT_CODE/AC_CODE : prefix CODE_STR`, `CODE_STR : ('0'|[1-9][0-9]*)('.'('0'|[1-9][0-9]*))*` — **zero-padded at-codes (`at0000`, `at0078.2`) are NOT lexable**, and cadl2.g4 node-id slots accept ID_CODE/ROOT_ID_CODE ONLY (AT_CODE only inside `c_terminology_code`), so the whole "at-coded ADL2" example flavour used across master04.x/09.x has no grammar production.

**Rules language:** `rules` section = subset of BMM assertions/assignments using openEHR Expression Language (EL) — spec at `docs/specs/openehr/LANG/docs/EL/` and BEL at `LANG/docs/BEL/` (BEL masterAppA-syntax.adoc has the grammar). Slot assertions use the same base_expressions grammar.

**cADL validity-rule catalogue (master04.6)** is itself a snapshot of external `adl-resources/messages/ADL/adl_syntax_errors.txt`.
