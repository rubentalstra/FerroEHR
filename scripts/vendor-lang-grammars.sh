#!/usr/bin/env bash
# Vendor the LANG component's normative ANTLR4 grammars, VERSION-SCOPED.
#
# Layout (the multi-generation foundation, #1936/#1942): each LANG component
# version keeps its complete grammar set under its own
# crates/openehr-lang/vendor/grammar/vX_Y/ directory, mirroring the crate's
# generation modules — so a spec bump vendors a NEW directory and no
# generation's reference input can silently drift under another's.
#
# v1_1 — the 1.1.0-line normative grammars, from two upstream repos:
#   - ODIN + BEL: https://github.com/openEHR/adl-antlr (the LANG odin/BEL
#     syntax appendices reference these files as the normative syntax).
#   - EL: https://github.com/openEHR/openEHR-antlr4 (the EL syntax appendix
#     masterAppA-syntax.adoc is an `include::` of ElParser.g4 + ElLexer.g4),
#     plus the transitive grammar imports so the set resolves standalone.
#
# v1_0 — the released LANG 1.0.0 has NO machine-readable grammar upstream:
#   openEHR-antlr4 was created 2021-07-15, AFTER the 1.0.0 release
#   (11-May-2020; verified first-hand 2026-08-05), and adl-antlr carries no
#   release tag for it. Until the 1.0.0 set is extracted from that release's
#   own docs text (tracker #1946), v1_0/ is a STOPGAP COPY of the v1_1 set,
#   marked by STOPGAP.md — honest scaffolding, never a claim of 1.0.0 text.
#
# Both repos are Apache-2.0 (LICENSE vendored alongside).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)/crates/openehr-lang/vendor/grammar"

ADL_ANTLR_REPO="openEHR/adl-antlr"
ADL_ANTLR_COMMIT="8db091ec3d810371cc41cd072fee81ce893fea47" # 2024-04-06
ANTLR4_REPO="openEHR/openEHR-antlr4"
ANTLR4_COMMIT="3494da942f3ed35963279837447b3039dd098e20" # master 2025-12-15

fetch() {
  local repo="$1" commit="$2" path="$3" out="$4"
  curl -fsSL "https://raw.githubusercontent.com/${repo}/${commit}/${path}" -o "${out}"
  echo "vendored ${out} @ ${commit}"
}

# ── v1_1: the pinned normative set ──────────────────────────────────────────
DEST="${ROOT}/v1_1"
mkdir -p "${DEST}"

# ODIN + BEL (adl-antlr)
for f in odin.g4 odin_values.g4 base_lexer.g4 base_expressions.g4; do
  fetch "${ADL_ANTLR_REPO}" "${ADL_ANTLR_COMMIT}" "src/main/antlr/adl/${f}" "${DEST}/${f}"
done

# EL + transitive imports (openEHR-antlr4)
for f in ElLexer.g4 ElParser.g4 Cadl2Lexer.g4 Cadl2Parser.g4 SymbolsLexer.g4 GeneralIdsLexer.g4; do
  fetch "${ANTLR4_REPO}" "${ANTLR4_COMMIT}" "reader_common/src/main/antlr/${f}" "${DEST}/${f}"
done
fetch "${ANTLR4_REPO}" "${ANTLR4_COMMIT}" "LICENSE" "${ROOT}/LICENSE-openEHR-antlr4"

# ── v1_0: the stopgap set (see the header; re-derived under #1946) ──────────
DEST10="${ROOT}/v1_0"
mkdir -p "${DEST10}"
cp "${DEST}"/*.g4 "${DEST10}/"
cat > "${DEST10}/STOPGAP.md" <<'EOF'
# STOPGAP grammar set — not LANG 1.0.0 text

The released LANG 1.0.0 publishes no machine-readable grammar (the
openEHR-antlr4 repository postdates the release; adl-antlr has no 1.0.0
tag). These files are a verbatim copy of the pinned `../v1_1/` set so the
`openehr_lang::v1_0` readers have an in-tree reference input; the honest
1.0.0 set is extracted from that release's own docs text under tracker
issue #1946, which replaces this file.
EOF
echo "stopgap-copied v1_0 set (see ${DEST10}/STOPGAP.md; tracker #1946)"
