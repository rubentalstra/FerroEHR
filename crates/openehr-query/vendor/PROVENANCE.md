# Vendored openEHR QUERY (AQL) source material

Source: https://github.com/openEHR/specifications-QUERY
Pinned commit: `10cb73fe7e3dd7a7f239759f989377e963b52fe2` (master, vendored 2026-07-03)
License: CC-BY-SA 3.0 Unported (the upstream repo's `LICENSE`; root reference
copy `LICENSE-CC-BY-SA-3.0`) — redistributed verbatim with attribution.

AQL has **no BMM meta-model** — it is a query *language*, so `openehr-query` is
hand-written (a `logos` lexer + `chumsky`/`winnow` parser) against the
authoritative ANTLR4 grammar below. The grammar is the spec; the example docs
are the acceptance corpus (worked queries embedded in AsciiDoc).

| Path                           | Upstream                      | Role                          |
|--------------------------------|-------------------------------|-------------------------------|
| grammar/AqlLexer.g4            | docs/AQL/grammar/AqlLexer.g4  | token spec → `logos` lexer    |
| grammar/AqlParser.g4           | docs/AQL/grammar/AqlParser.g4 | grammar → AST + parser        |
| examples/master0{3,4,5}-*.adoc | docs/AQL_examples/            | worked-query corpus for tests |

Full AQL spec: https://specifications.openehr.org/releases/QUERY/latest/AQL.html
