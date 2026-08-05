# Provenance — the normative ADL/ODIN ANTLR4 grammars

- Source: https://github.com/openEHR/adl-antlr (Apache-2.0; `LICENSE`
  vendored alongside)
- Commit: `8db091ec3d810371cc41cd072fee81ce893fea47` (2024-04-06)
- Vendored by: `scripts/vendor-adl-grammars.sh` (re-run it to refresh;
  never hand-edit a grammar file)
- Layout: **version-scoped by AM generation** (#1936/#1946), mirroring the
  `openehr-am` generation modules:
  - `v2_4/` — the ADL2 set (`adl2.g4`, `cadl2.g4`, `cadl2_primitives.g4`)
    plus the support grammars upstream maintains as part of the ADL2 family
    (`adl_keywords.g4`, `base_expressions.g4`, `base_lexer.g4`).
  - `v1_4/` — the ADL 1.4 set (`adl14.g4`, `cadl14.g4`,
    `cadl14_primitives.g4`), reference input for the 1.4→2 conversion front
    end. These grammars IMPORT the `v2_4/` support grammars — upstream keeps
    one flat directory serving both generations, so the split records which
    AM generation each grammar's productions target, not a self-contained
    per-directory closure.
  - `PCRE.g4` — generation-independent lexical support, top-level (upstream
    keeps it outside its `adl/` directory too).
- `odin.g4` + `odin_values.g4` live with the LANG component at
  `crates/openehr-lang/vendor/grammar/` (ODIN is a LANG spec; the
  `openehr_lang` odin parsers implement it). `base_lexer.g4` is vendored in
  BOTH crates' dirs from the same commit: the ADL grammars here and
  `odin_values.g4` there each import it, and both vendor trees stay
  independently pinned.

Why vendored: the ADL2 spec's normative syntax appendix
(`docs/specs/openehr/AM/docs/ADL2/masterAppB-syntax_spec.adoc`) does not
reproduce the grammar — it `include::`s these files from the adl-antlr
repository. They are the normative token regexes and productions for
ADL2/cADL2, the rules/slot assertion language (`base_expressions.g4`), and
the shared lexer — plus the ADL 1.4 grammar set used as reference for the
1.4→2 conversion front end.

These files are **reference input for the hand-written `logos`/`chumsky`
parser** in this crate — they are not a build input and no ANTLR runtime
is ever a dependency.
