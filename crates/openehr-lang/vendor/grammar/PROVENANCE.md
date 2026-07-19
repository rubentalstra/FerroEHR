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

Never hand-edit; re-vendor from upstream and update the commit pin here.
