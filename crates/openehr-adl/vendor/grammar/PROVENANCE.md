# Provenance — the normative ADL/ODIN ANTLR4 grammars

- Source: https://github.com/openEHR/adl-antlr (Apache-2.0; `LICENSE`
  vendored alongside)
- Commit: `8db091ec3d810371cc41cd072fee81ce893fea47` (2024-04-06)
- Files: `src/main/antlr/adl/*.g4` + `src/main/antlr/PCRE.g4`, copied
  verbatim — except `odin.g4` + `odin_values.g4`, which live with the
  LANG component at `crates/openehr-lang/vendor/grammar/` (ODIN is a LANG
  spec; the `openehr_lang::odin` parser implements it). `base_lexer.g4`
  is vendored in BOTH dirs (same commit): the ADL grammars here and
  `odin_values.g4` there each import it, and both vendor dirs stay
  self-contained.

Why vendored: the ADL2 spec's normative syntax appendix
(`docs/specs/openehr/AM/docs/ADL2/masterAppB-syntax_spec.adoc`) does not
reproduce the grammar — it `include::`s these files from the adl-antlr
repository. They are the normative token regexes and productions for
ADL2/cADL2 (`adl2.g4`, `cadl2.g4`, `cadl2_primitives.g4`), the rules/slot
assertion language (`base_expressions.g4`), ODIN terminals
(`odin.g4`, `odin_values.g4`), the shared lexer (`base_lexer.g4`,
`adl_keywords.g4`, `PCRE.g4`) — plus the ADL 1.4 grammar set
(`adl14.g4`, `cadl14.g4`, `cadl14_primitives.g4`) used as reference for
the 1.4→2 conversion front end.

These files are **reference input for the hand-written `logos`/`chumsky`
parser** in this crate — they are not a build input and no ANTLR runtime
is ever a dependency. Never hand-edit; re-vendor from upstream and update
the commit pin here.
