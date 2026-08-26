#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# The ferroehr party statement's product version must equal the workspace
# version (#2576): the statement is the published conformance claim for THE
# product this repo ships, and it sat at 3.6.0 while the record reached
# 3.20.0 — the rendered CONFORMANCE_STATEMENT.md contradicted the report
# beside it for fourteen releases with nothing failing. The release cut
# bumps both in one PR (.claude/rules/changelog.md); this guard is the
# failing check that keeps them together. The ehrbase party statement is
# deliberately NOT checked — it declares another vendor's product.
#
# Usage: scripts/checks/statement-version.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

workspace=$(grep -m1 '^version = "' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
statement=$(jq -r '.product.version' docs/conformance/party/ferroehr/statement.json)

if [[ -z "$workspace" ]]; then
  echo "statement-version: cannot read the workspace version from Cargo.toml" >&2
  exit 1
fi
if [[ "$workspace" != "$statement" ]]; then
  echo "statement-version: the ferroehr party statement declares product \
version $statement but the workspace is $workspace — bump \
docs/conformance/party/ferroehr/statement.json (and regenerate the derived \
documents: bash scripts/render/conformance-docs.sh) in the same PR." >&2
  exit 1
fi
echo "statement-version: OK ($workspace)."
