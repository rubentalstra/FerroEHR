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
# release). Dockerfile.vercel pins the same published image for the hosted
# sandbox, so its FROM tag is held to the same version. Dependency-free:
# grep + sed only.
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker-compose.yml"
VERCEL_FILE="$ROOT_DIR/Dockerfile.vercel"
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

vercel_ref="$(grep -oE '^FROM ghcr\.io/[^ ]+' "$VERCEL_FILE" | head -1 | sed 's/^FROM //')"
if [ -z "$vercel_ref" ]; then
  echo "::error::$VERCEL_FILE declares no ghcr.io FROM reference" >&2
  rc=1
else
  vercel_tag="${vercel_ref##*:}"
  if [ "$vercel_tag" != "$version" ]; then
    echo "::error::Dockerfile.vercel pins '$vercel_ref' (tag $vercel_tag) but the workspace version is $version — the release cut bumps it with the compose tags (.claude/rules/changelog.md)." >&2
    rc=1
  else
    echo "Dockerfile.vercel FROM $vercel_ref matches the workspace version."
  fi
fi

exit "$rc"
