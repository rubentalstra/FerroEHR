#!/usr/bin/env bash
# Vendor the normative Expression Language (EL) ANTLR4 grammars.
#
# WHY: the LANG EL syntax appendix
# (docs/specs/openehr/LANG/docs/EL/masterAppA-syntax.adoc) is, in full, an
# `include::` of ElParser.g4 + ElLexer.g4 from "the openEHR Antlr4 Git
# repository" — the grammars ARE the normative syntax, and no other vendored
# tree carries them (openEHR/adl-antlr, the ODIN/BEL/cADL source, has no EL
# grammar — verified 2026-08-04). Building the openehr-lang EL parser
# (tracker #1878) from prose or memory is forbidden (spec-adherence), so the
# grammars are vendored here first, pinned by commit.
#
# Source: https://github.com/openEHR/openEHR-antlr4 (Apache-2.0 — the GitHub
# license API reports SPDX Apache-2.0; LICENSE vendored alongside).
#
# Files -> crates/openehr-lang/vendor/grammar/ (beside the ODIN/BEL set;
# PROVENANCE.md records both sources).
set -euo pipefail

REPO="openEHR/openEHR-antlr4"
COMMIT="3494da942f3ed35963279837447b3039dd098e20"   # master 2025-12-15
# Grammars are VERSION-SCOPED vendored artifacts (the multi-generation
# foundation, #1936/#1942): each LANG component version keeps its grammars
# under its own vX_Y directory, mirroring the crate's generation modules.
DEST="$(cd "$(dirname "$0")/.." && pwd)/crates/openehr-lang/vendor/grammar/v1_1"

fetch() {
  local path="$1" out="$2"
  curl -fsSL "https://raw.githubusercontent.com/${REPO}/${COMMIT}/${path}" -o "${out}"
  echo "vendored ${out} @ ${COMMIT}"
}

fetch "reader_common/src/main/antlr/ElLexer.g4"  "${DEST}/ElLexer.g4"
fetch "reader_common/src/main/antlr/ElParser.g4" "${DEST}/ElParser.g4"
# The transitive grammar imports (ElLexer imports Cadl2Lexer/SymbolsLexer/
# GeneralIdsLexer; ElParser imports Cadl2Parser) — vendored so the normative
# syntax resolves standalone in-tree.
fetch "reader_common/src/main/antlr/Cadl2Lexer.g4"      "${DEST}/Cadl2Lexer.g4"
fetch "reader_common/src/main/antlr/Cadl2Parser.g4"     "${DEST}/Cadl2Parser.g4"
fetch "reader_common/src/main/antlr/SymbolsLexer.g4"    "${DEST}/SymbolsLexer.g4"
fetch "reader_common/src/main/antlr/GeneralIdsLexer.g4" "${DEST}/GeneralIdsLexer.g4"
fetch "LICENSE" "${DEST}/LICENSE-openEHR-antlr4"
