#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
#
# The Vercel "Ignored Build Step" for the sandbox (#2710): build only when
# the image Dockerfile.vercel pins actually exists on GHCR. At a release
# cut the version-bump merge reaches develop minutes before CI publishes
# that tag's image, so the deploy Vercel triggers on the push would fail on
# a missing FROM; skipping it cleanly closes that race. The containers
# pipeline pings the project's Deploy Hook once the image is published, so
# the skipped deploy is re-run at exactly the right moment.
#
# Vercel semantics: exit 0 = skip the build, exit 1 = proceed.
set -u

tag=$(sed -nE 's/^FROM ghcr\.io\/rubentalstra\/ferroehr:(.+)$/\1/p' Dockerfile.vercel | head -1)
if [ -z "$tag" ]; then
  echo "no ghcr FROM pin found in Dockerfile.vercel; building" >&2
  exit 1
fi

token=$(curl -fsS "https://ghcr.io/token?scope=repository:rubentalstra/ferroehr:pull" 2>/dev/null \
  | sed -nE 's/.*"token" *: *"([^"]+)".*/\1/p')
if [ -z "$token" ]; then
  echo "could not fetch a GHCR pull token; building anyway" >&2
  exit 1
fi

if curl -fsSI -o /dev/null \
  -H "Authorization: Bearer $token" \
  -H "Accept: application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json" \
  "https://ghcr.io/v2/rubentalstra/ferroehr/manifests/$tag"; then
  echo "ghcr.io/rubentalstra/ferroehr:$tag exists; building" >&2
  exit 1
fi

echo "ghcr.io/rubentalstra/ferroehr:$tag is not published yet; skipping this deploy (the containers pipeline re-triggers it via the Deploy Hook once the image lands)" >&2
exit 0
