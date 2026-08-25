#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# One copyright holder, stated identically everywhere it is stated.
#
# `LICENSE` said "Ruben Talstra" while `REUSE.toml`, the codegen header constant
# and 2470 file headers said "FerroEHR contributors". Both are defensible
# positions; asserting both means a downstream redistributor reading a file
# header and a lawyer reading LICENSE come away with different answers about who
# holds the copyright — the exact ambiguity per-file licensing exists to remove.
#
# The divergence survived because nothing compared the three sources. This does.
#
# NOT compared: `CITATION.cff` authors and `.zenodo.json` creators. Those record
# AUTHORSHIP for citation, which is a different datum from the copyright holder
# and is correctly a named person.
set -euo pipefail

cd "$(dirname "$0")/../.."

readonly HOLDER='FerroEHR contributors'

fail=0
check() {
  local what="$1" found="$2"
  if [[ "$found" != "$HOLDER" ]]; then
    echo "error: $what states the copyright holder as:" >&2
    echo "         ${found:-<not found>}" >&2
    echo "       every source must state: $HOLDER" >&2
    fail=1
  fi
}

# `LICENSE` — the MIT notice line.
check "LICENSE" \
  "$(sed -n 's/^Copyright (c) [0-9]\{4\} //p' LICENSE | head -1)"

# `REUSE.toml` — the first-party annotation (the openEHR Foundation appears as a
# SECOND holder on vendored-derived files, which is a different statement).
check "REUSE.toml" \
  "$(sed -n 's/^SPDX-FileCopyrightText = "\(.*\)"$/\1/p' REUSE.toml | head -1)"

# The emitter constant every generated header is stamped from.
check "tools/openehr-codegen/src/render/spdx.rs" \
  "$(sed -n 's/^pub(crate) const PROJECT_COPYRIGHT: &str = "\(.*\)";$/\1/p' \
       tools/openehr-codegen/src/render/spdx.rs | head -1)"

if [[ "$fail" -ne 0 ]]; then
  echo >&2
  echo "Changing the holder means changing ALL of them together, plus the file" >&2
  echo "headers (re-run the codegen emit set for the generated half)." >&2
  exit 1
fi

echo "ok: one copyright holder — $HOLDER"
