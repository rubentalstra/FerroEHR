#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Compose image-tag guard (the CITATION.cff / Helm appVersion pattern).
#
# The standalone quickstart docker-compose.yml pulls the PUBLISHED images by an
# explicit tag, so a release cut that forgets to bump those defaults ships a
# quickstart pinned to the previous release. This guard fails when any default
# tag in the `${FERROEHR_*_IMAGE:-ghcr.io/...:X.Y.Z}` fallbacks disagrees with
# the workspace version in Cargo.toml (.claude/rules/changelog.md §Cutting a
# release). Dependency-free: grep + sed only.
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker-compose.yml"
MANIFEST="$ROOT_DIR/Cargo.toml"

version="$(sed -nE 's/^version = "(.*)"$/\1/p' "$MANIFEST" | head -1)"
if [ -z "$version" ]; then
  echo "::error::could not read the workspace version from $MANIFEST" >&2
  exit 1
fi

rc=0
for var in FERROEHR_IMAGE FERROEHR_POSTGRES_IMAGE FERROEHR_ADMIN_UI_IMAGE; do
  # The default is the `:-` fallback inside `${VAR:-ghcr.io/owner/name:tag}`.
  ref="$(grep -oE "\\\$\{$var:-[^}]+\}" "$COMPOSE_FILE" | head -1 \
    | sed -E "s/^\\\$\{$var:-//; s/\}$//")"
  if [ -z "$ref" ]; then
    echo "::error::$COMPOSE_FILE declares no \${$var:-…} default image reference" >&2
    rc=1
    continue
  fi
  tag="${ref##*:}"
  if [ "$tag" != "$version" ]; then
    echo "::error::$var default is '$ref' (tag $tag) but the workspace version is $version — the release cut bumps the docker-compose.yml image tags (.claude/rules/changelog.md)." >&2
    rc=1
  else
    echo "$var default $ref matches the workspace version."
  fi
done

exit "$rc"
