#!/usr/bin/env bash
# Vendor the official openEHR ADL 2 archetype libraries — the ADL 2.4 half of
# the two-dialect archetype corpus.
#
# WHY THIS SCRIPT EXISTS (verified 2026-08-01 — do not re-derive):
# the openEHR CKM publishes ADL **1.4** only. `GET /archetypes/{cid}/adl`
# answers `adl_version=1.4`; `/adl2`, `/adl14`, `/adl2.4`, `/opt2` and
# `/source` all 404; `?format=ADL2` / `?version=2` are silently ignored and
# return byte-identical 1.4 text. So the ADL 1.4 corpus comes from the live
# CKM (`scripts/vendor-ckm-archetypes.sh`) and the ADL 2 corpus comes from
# here — the openEHR ADL archetype repository, pinned by commit.
#
# NEVER fill the ADL 2 side by running our own ADL 1.4->2 converter
# (`openehr_adl::adl14::convert`) over CKM output: the converter has no spec
# basis (it is our own design) and feeding it its own output back as the
# oracle would test it against itself.
#
# Source: https://github.com/openEHR/adl-archetypes ("ADL test, reference and
# example archetypes"). Two trees are vendored:
#
#   ADL2-reference/            -> crates/openehr-adl/tests/corpus/adl2-reference
#     The ADL 2 regression library (its `_repo_lib.idx`: "openEHR ADL 2
#     regression test archetypes"). File names encode the expected AOM2/cADL
#     rule code, so this tree is the validator-conformance oracle keyed by
#     rule code. Provenance for it lives in
#     `crates/openehr-adl/tests/corpus/PROVENANCE.md` (that file stays the
#     record; this script is how the tree is reproduced).
#
#   Reference/CKM_2013_12_09/  -> tools/cnf-runner/artifacts/corpus/archetypes/adl2
#     A CKM export carrying BOTH dialects of the same archetypes side by side
#     (`*.adl` = 1.4, `*.adls` = ADL 2). The PAIRING is the point: it is
#     upstream's own 1.4/2 correspondence for real clinical archetypes, so it
#     grounds the ADL 2 wire cases of the DEFINITION API and gives the 1.4->2
#     conversion an INDEPENDENT reference (upstream's conversion, not ours).
#
# Licensing: the repository carries no top-level LICENSE file; the content is
# openEHR Foundation test material (openEHR specifications and associated
# artefacts are published under CC-BY), and individual archetypes may carry
# their own `licence` field. Recorded as-is, provenance retained.
#
# Usage:
#   scripts/vendor-adl2-archetypes.sh            # vendor at the pin below
#   ADL2_PIN=<sha> scripts/vendor-adl2-archetypes.sh
#   scripts/vendor-adl2-archetypes.sh --check    # report drift, write nothing
set -Eeuo pipefail

REPO="openEHR/adl-archetypes"
# The pin. Bump deliberately (a bump is a corpus change: re-run the
# openehr-adl gates and update crates/openehr-adl/tests/corpus/PROVENANCE.md
# in the same commit).
PIN="${ADL2_PIN:-093c77ea003742b9540e3dd377d615e2b26f2996}"

ADL2_REFERENCE_DEST="crates/openehr-adl/tests/corpus/adl2-reference"
CKM_PAIRS_DEST="tools/cnf-runner/artifacts/corpus/archetypes/adl2"

CHECK=0
[[ "${1:-}" == "--check" ]] && CHECK=1

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
STAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

echo "==> fetching $REPO @ ${PIN:0:12}"
curl -fsSL "https://codeload.github.com/$REPO/tar.gz/$PIN" -o "$WORK/repo.tar.gz"
mkdir -p "$WORK/src"
tar -xzf "$WORK/repo.tar.gz" -C "$WORK/src" --strip-components=1

for tree in ADL2-reference Reference/CKM_2013_12_09; do
  [[ -d "$WORK/src/$tree" ]] || {
    echo "::error::$tree is absent at ${PIN:0:12} — the upstream layout moved" >&2
    exit 1
  }
done

sync_tree() {
  local src=$1 dest=$2
  if [[ $CHECK == 1 ]]; then
    echo "==> check $dest"
    diff -rq "$src" "$dest" || true
    return
  fi
  echo "==> vendor $dest"
  mkdir -p "$dest"
  # --delete so a file upstream removed disappears here too; the tree is
  # vendored verbatim and never hand-edited
  rsync -a --delete "$src/" "$dest/"
}

sync_tree "$WORK/src/ADL2-reference" "$ADL2_REFERENCE_DEST"
sync_tree "$WORK/src/Reference/CKM_2013_12_09" "$CKM_PAIRS_DEST/ckm-2013-12-09"

[[ $CHECK == 1 ]] && exit 0

# ── provenance for the CNF-corpus tree (the openehr-adl tree keeps its record
# in crates/openehr-adl/tests/corpus/PROVENANCE.md) ───────────────────────────
python3 - "$CKM_PAIRS_DEST" "$STAMP" "$REPO" "$PIN" <<'PY'
import collections
import pathlib
import sys

dest, stamp, repo, pin = sys.argv[1:5]
root = pathlib.Path(dest) / "ckm-2013-12-09"
adl = sorted(root.rglob("*.adl"))
adls = sorted(root.rglob("*.adls"))

def hrid(path):
    # openEHR-EHR-OBSERVATION.apgar.v1.0.0.adls -> openEHR-EHR-OBSERVATION.apgar
    stem = path.name.split(".v")[0]
    return stem

paired = sorted({hrid(p) for p in adl} & {hrid(p) for p in adls})
classes = collections.Counter(p.name.split(".")[0] for p in adls)

with open(pathlib.Path(dest) / "PROVENANCE.md", "w") as fh:
    w = fh.write
    w("# ADL 2 archetype pack (with ADL 1.4 twins) — provenance\n\n")
    w(f"Vendored verbatim from `https://github.com/{repo}`\n")
    w(f"(`Reference/CKM_2013_12_09/`) at commit `{pin}` by\n")
    w(f"`scripts/vendor-adl2-archetypes.sh` on {stamp}.\n\n")
    w("Upstream describes the tree as archetypes exported from the Clinical\n")
    w("Knowledge Manager (export time Mon Dec 09 15:42:23 CET 2013).\n\n")
    w("## Why this source and not CKM\n\n")
    w("The live openEHR CKM publishes **ADL 1.4 only** — `/archetypes/{cid}/adl`\n")
    w("returns `adl_version=1.4` and there is no ADL 2 export endpoint\n")
    w("(`/adl2`, `/opt2` 404; `?format=ADL2` is ignored). The ADL 1.4 corpus is\n")
    w("therefore vendored live (`corpus/archetypes/ckm/`, ADL 1.4) and the ADL 2\n")
    w("corpus comes from this pinned upstream library.\n\n")
    w("The ADL 2 side is NEVER produced by running our own ADL 1.4->2 converter\n")
    w("over CKM output: that converter has no spec basis (our own design) and\n")
    w("would then be validated against its own output.\n\n")
    w("## Contents\n\n")
    w(f"- ADL 2 archetypes (`*.adls`): **{len(adls)}**\n")
    w(f"- ADL 1.4 twins (`*.adl`): **{len(adl)}**\n")
    w(f"- archetypes present in BOTH dialects: **{len(paired)}**\n\n")
    w("The dual-dialect pairing is the value here: the same clinical archetype\n")
    w("in 1.4 and in 2, as published upstream — an INDEPENDENT reference for\n")
    w("the conversion path and matched inputs for the DEFINITION API's ADL 1.4\n")
    w("and ADL 2 wire cases.\n\n")
    w("| RM class | ADL 2 files |\n|---|---|\n")
    for cls, n in sorted(classes.items(), key=lambda kv: (-kv[1], kv[0])):
        w(f"| {cls} | {n} |\n")
    w("\nNever hand-edit a vendored fixture; re-run this script and bump the pin.\n")

print(f"==> {len(adls)} ADL 2 + {len(adl)} ADL 1.4 files ({len(paired)} paired) → {dest}")
PY

echo "==> ADL 2 regression library → $ADL2_REFERENCE_DEST"
echo "    (its provenance record: crates/openehr-adl/tests/corpus/PROVENANCE.md)"
git status --short "$ADL2_REFERENCE_DEST" "$CKM_PAIRS_DEST" | head -20
