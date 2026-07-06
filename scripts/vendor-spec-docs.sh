#!/usr/bin/env bash
# Vendors the openEHR specification *documentation* (the normative spec text)
# into docs/specs/openehr/, pinned per docs/VERSIONS.md. Text formats only
# (adoc/md/txt/csv/json/yaml) — images, UML .xmi, XSDs, and other binaries are
# excluded (fetch from the upstream repo at the pinned ref if needed). .robot/
# .xml/.opt are included for the executable CNF suite + canonical examples.
#
# This is REFERENCE DOCUMENTATION for spec-adherence checks. It is NOT a build
# input: codegen consumes crates/openehr-codegen/vendor/** (BMM/XSD/OAS) and
# openehr-its/schemas/**; those vendor dirs stay authoritative for generation.
#
# Idempotent: wipes and re-vendors each component dir. Re-run after bumping a
# ref below (keep docs/VERSIONS.md in sync).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$REPO_ROOT/docs/specs/openehr"
INCLUDE_EXT=(adoc md txt csv json yaml yml robot xml opt)

# component | upstream repo | human ref | pinned commit
# Master pins: the latest published spec versions (RM 1.2.0, BASE 1.3.0,
# TERM 3.1.0, AM 2.4.0, LANG 1.1.0) have no GitHub release tags yet — they
# live on master. SHAs chosen 2026-07-06 to match the ITS-BMM/ITS-JSON pins
# already vendored for codegen.
COMPONENTS=(
  "BASE|specifications-BASE|master (BASE 1.3.0)|49f5bbe10992a645d7bd1e90c86d188b9587d13b"
  "RM|specifications-RM|master (RM 1.2.0)|c52de2b80503f3e8613dd4b7455b1b60336e9fac"
  "AM|specifications-AM|master (AM 2.4.0 + ADL/AOM/OPT 1.4)|da06d63297e8549a351c854d8b1c45cd9f1d577c"
  "TERM|specifications-TERM|master (TERM 3.1.0)|007d0dddcdd77648711681878b54ace021b2fbd5"
  "LANG|specifications-LANG|master (LANG 1.1.0)|201b647034f7b1ddfe207e4c3c6f52f6878869b8"
  "QUERY|specifications-QUERY|Release-1.1.0|a87bb51fa1c515b863c9610a9444a2d5570dc05a"
  "SM|specifications-SM|master|23ffc4711c10bae2ae43724b1948fe3b24a0964e"
  "CNF|specifications-CNF|master|33251d2abe5a75c042e11c9385d2e9a79aa15904"
  "ITS-REST|specifications-ITS-REST|Release-1.0.3|4aec22de9cdbd5427862f08013e8dce536a291cc"
  "ITS-XML|specifications-ITS-XML|master (1.0.2 target, 2.0.0 TRIAL)|de8b37ba6c9a5e126623a063cafba3b58ebf1107"
  "ITS-JSON|specifications-ITS-JSON|master (development pin)|5acae056248e917a4b4c56f7e712f4fcfeb616a6"
)
# ITS-BMM is deliberately absent: it is already vendored verbatim (all
# serializations) at crates/openehr-codegen/vendor/bmm/.

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

rsync_args=(--exclude='.git')
for ext in "${INCLUDE_EXT[@]}"; do
  rsync_args+=(--include="*.$ext")
done
rsync_args+=(--include='*/' --exclude='*')

mkdir -p "$DEST"
for entry in "${COMPONENTS[@]}"; do
  IFS='|' read -r name repo ref sha <<<"$entry"
  echo "==> $name ($repo @ $ref, $sha)"
  src="$TMP/$name"
  git init -q "$src"
  git -C "$src" remote add origin "https://github.com/openEHR/$repo.git"
  git -C "$src" fetch -q --depth 1 origin "$sha"
  git -C "$src" checkout -q FETCH_HEAD

  out="$DEST/$name"
  rm -rf "$out"
  mkdir -p "$out"
  rsync -a --prune-empty-dirs "${rsync_args[@]}" "$src/" "$out/"

  cat >"$out/PROVENANCE.md" <<EOF
# Vendored openEHR spec docs: $name

- Source: https://github.com/openEHR/$repo
- Ref: $ref
- Commit: \`$sha\`
- Vendored by: \`scripts/vendor-spec-docs.sh\` (text formats only: ${INCLUDE_EXT[*]})
- Images/UML/XSD/binaries excluded — fetch from the repo at the pinned commit.

Do not hand-edit files under this directory; re-run the script instead.
EOF
  echo "    $(find "$out" -type f | wc -l | tr -d ' ') files"
done

echo "Done. Vendored into $DEST"
