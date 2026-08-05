<!-- Layout note (2026-08-05, #1942): grammars are VERSION-SCOPED — each LANG
component version keeps its grammar set under its own `vX_Y/` directory,
mirroring the crate's generation modules (`v1_1/` = the 1.1.0-line normative
grammars pinned below). A future version's grammars land in their own
directory via `scripts/vendor-el-grammar.sh`, never mixed. -->

# Provenance — the normative ODIN + BEL ANTLR4 grammars

- Source: https://github.com/openEHR/adl-antlr (Apache-2.0; `LICENSE`
  vendored alongside)
- Commit: `8db091ec3d810371cc41cd072fee81ce893fea47` (2024-04-06)
- Files: `src/main/antlr/adl/{odin.g4, odin_values.g4, base_lexer.g4,
  base_expressions.g4}`, copied verbatim.

Why here: ODIN **and** the Basic Expression Language (BEL) are
LANG-component specifications (`docs/specs/openehr/LANG/docs/{odin,BEL}/`);
these grammars are the normative syntax those specs' syntax sections
reference (the BEL spec's `masterAppB`/`masterAppA` appendix is `include::`
pointers at `base_expressions.g4`), and the reference input for the
hand-written `openehr_lang::{odin, bel}` parsers (no ANTLR runtime).
`odin_values.g4` imports `base_lexer.g4`; `base_expressions.g4` imports
`cadl2_primitives` (the cADL primitive right-hand side of a `matches`
constraint — the AOM extension point that `openehr-adl` supplies over the
`openehr_lang::bel` seam) and `odin_values.g4`. The shared files (same
upstream commit) are also vendored at `crates/openehr-adl/vendor/grammar/`
for the ADL2/cADL grammar set — each crate's vendor dir stays
self-contained.

## The Expression Language (EL) grammars

- Source: https://github.com/openEHR/openEHR-antlr4 (Apache-2.0;
  `LICENSE-openEHR-antlr4` vendored alongside — the ONLY home of the EL
  grammars: `openEHR/adl-antlr` carries none, verified 2026-08-04).
- Commit: `3494da942f3ed35963279837447b3039dd098e20` (2025-12-15), via
  `scripts/vendor-el-grammar.sh`.
- Files: `reader_common/src/main/antlr/{ElLexer.g4, ElParser.g4}` plus their
  transitive grammar imports `{Cadl2Lexer, Cadl2Parser, SymbolsLexer,
  GeneralIdsLexer}.g4`, copied verbatim (the appendix includes only the two EL
  files; the imports are vendored so the syntax resolves standalone).

Why here: the LANG EL syntax appendix
(`docs/specs/openehr/LANG/docs/EL/masterAppA-syntax.adoc`) is an `include::`
of exactly these two files — they are the normative EL syntax, and the
reference input for the hand-written `openehr_lang` EL parser (no ANTLR
runtime).

Never hand-edit; re-vendor from upstream and update the commit pin here.
