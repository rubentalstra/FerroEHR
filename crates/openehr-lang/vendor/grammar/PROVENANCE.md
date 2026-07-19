# Provenance — the normative ODIN ANTLR4 grammars

- Source: https://github.com/openEHR/adl-antlr (Apache-2.0; `LICENSE`
  vendored alongside)
- Commit: `8db091ec3d810371cc41cd072fee81ce893fea47` (2024-04-06)
- Files: `src/main/antlr/adl/{odin.g4, odin_values.g4, base_lexer.g4}`,
  copied verbatim.

Why here: ODIN is a LANG-component specification
(`docs/specs/openehr/LANG/docs/odin/`); these grammars are the normative
syntax the ODIN spec's syntax section references, and the reference input
for the hand-written `openehr_lang::odin` parser (no ANTLR runtime).
`odin_values.g4` imports `base_lexer.g4`, so that shared lexer file is
vendored here too; the same file (same upstream commit) is also vendored
at `crates/openehr-adl/vendor/grammar/` for the ADL2/cADL grammar set that
imports it — both vendor dirs stay self-contained.

Never hand-edit; re-vendor from upstream and update the commit pin here.
