---
name: adl2-parser-spec-location
description: Where ADL2/cADL2/ODIN source-parser grammar + rules live in the vendored AM/LANG specs, and the vendored-text grammar gap
metadata:
  type: reference
---

ADL2 (ADL 2.4) parser spec lives at `docs/specs/openehr/AM/docs/ADL2/`.
File-to-topic map:
- `master03-file_encoding.adoc` — UTF-8 only official; `\r \n \t \\ \" \'` escapes; regex delimiters `//` or `^^`; unicode `\uHHHH`/`\uHHHHHHHH`.
- `master04.1..04.6` — cADL: overview / basics (keywords, comments `--`, symbol equivalents ∈/∼/∗) / complex types (existence, cardinality, occurrences, any, subtype narrowing, use_node proxy, use_archetype external ref, allow_archetype slots include/exclude) / second-order (tuple `[a,b]∈{[{},{}]}`, group, tuple path child-index) / primitive types (all leaf constraint syntaxes) / **validity rules catalogue (SUNK,SARID,SASID...STCNT)**.
- `master05-paths.adoc` — ADL path grammar `path: '/'? path_segment ('/' path_segment)+ ; path_segment: attr_name ('[' object_id ']')? ;` movable `//`.
- `master06-default_values.adoc` — `_default = (TYPE) <odin>` or `(json) <# ... #>`.
- `master07.01..07.14` — ADL artefact sections: 04 basics (keywords, node/term/value-set code prefixes id/at/ac, dot-specialisation), 05 identification (HRID, adl_version, rm_release, uid/build_uid, namespaces, version N.M.P-alpha.N/-rc.N, generated/controlled), 06 specialise, 07 language, 08 description, 10 definition, **11-adl_rulesNEW.adoc = current rules** (11 old is legacy; both present), 12 rm_overlay, 13 terminology (ODIN: term_definitions/value_sets/term_bindings), 14 annotations.
- `master09.02-spec_concepts.adoc` — specialisation levels: **spec level = number of '.' in node id**; specialisation paths; redefinition catalogue table.
- `master10-templates.adoc` — template / template_overlay / operational_template + component_terminologies.
- `masterAppB-syntax_spec.adoc` — normative grammar appendix.

ODIN spec (used in language/description/terminology/rm_overlay/annotations/rules-symbols + terminal value types): `docs/specs/openehr/LANG/docs/odin/`, esp. `master07-leaf_data.adoc` (literal syntax for String/Integer/Real/Boolean/Date-Time/Duration/interval `|N..M|`/URI/coded-term `[terminology_id::code]`/lists) and `masterAppB-syntax_spec.adoc`.

**GRAMMAR GAP (confirmed defect for implementers):** masterAppB is `include::`-only — it references external ANTLR4 `.g4` files (adl2.g4, cadl2.g4, cadl2_primitives.g4, base_expressions.g4, odin_values.g4, base_lexer.g4) in the openEHR/adl-antlr GitHub repo. **Those .g4 files are NOT vendored** (only AqlLexer/AqlParser g4 exist, under crates/openehr-query/vendor/grammar). So the vendored text gives prose + examples + the date/time/duration constraint-pattern lexer-rule NAMES (DATE_CONSTRAINT_PATTERN etc.) but NOT literal token/production rules. Top-level section ORDERING is shown only in examples + an image (adl_text_overview.svg), never as a vendored normative production.

**Rules language:** `rules` section = subset of BMM assertions/assignments using openEHR Expression Language (EL) — spec at `docs/specs/openehr/LANG/docs/EL/` and BEL at `LANG/docs/BEL/` (BEL masterAppA-syntax.adoc has the grammar). Slot assertions use the same base_expressions grammar.

**cADL validity-rule catalogue (master04.6)** is itself a snapshot of external `adl-resources/messages/ADL/adl_syntax_errors.txt`.
