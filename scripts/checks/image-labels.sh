#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Image-metadata guard: the OCI annotation keys are declared once, correctly, and
# the two places that declare them agree.
#
# Three defects motivated this, all found on the published images (#2146):
#
#   * No annotations existed at all. A LABEL lives in the image config blob; an
#     ANNOTATION lives in the manifest or the index. GHCR reads a package's
#     description from the INDEX annotation, so three images carrying a perfectly
#     good description LABEL all displayed "No description provided".
#   * Six of the spec's fourteen predefined keys were unset, including
#     `documentation` (we publish a docs site and never pointed at it) and
#     `base.name`/`base.digest`, whose values were already pinned in the `FROM`.
#   * All three Dockerfiles declared `licenses="Apache-2.0"` for an MIT project.
#     That was invisible on GHCR because `build-push-action`'s `labels:` input
#     overrides a Dockerfile LABEL, so CI published the correct `MIT` — and
#     anyone building the Dockerfile directly, the documented compose path,
#     shipped an image asserting a licence the project does not use.
#
# The third is the one this guard exists for. The same metadata is declared in
# two places — the Dockerfile, so a direct build is correct, and the publishing
# workflow, whose repository-derived defaults are wrong for a repo that ships
# three images (title comes from the repo NAME, so all three would claim to be
# "FerroEHR"). Two declarations of one fact drift; this makes them fail instead.
#
# Ownership, which is what the checks below encode:
#   build-INDEPENDENT (title, description, url, documentation, source, vendor,
#     authors, licenses, base.name, base.digest) — both places, identical values
#   build-DEPENDENT (created, version, revision, ref.name) — the workflow only,
#     since a Dockerfile cannot know them
#   ref.name — deliberately unset; see the waiver below
#
# Usage: scripts/checks/image-labels.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

WORKFLOW=.github/workflows/containers.yml
# image name : Dockerfile : the workflow env var naming that lane's image.
# The lane is keyed by the env var, not by the image name: `ferroehr` is a prefix
# of `ferroehr-admin-ui`, so a name match reads the wrong lane's labels.
IMAGES="ferroehr:docker/Dockerfile:APP_IMAGE
ferroehr-admin-ui:docker/admin-ui/Dockerfile:ADMIN_UI_IMAGE
ferroehr-postgres:docker/postgres/Dockerfile:POSTGRES_IMAGE"

# The keys both places must declare, identically.
SHARED="title description url documentation source vendor authors licenses base.name base.digest"

failures=0
report() { printf '%s\n' "$*" >&2; failures=$((failures + 1)); }

# The value of `org.opencontainers.image.<key>` in a Dockerfile LABEL block.
dockerfile_label() {
  awk -v key="org.opencontainers.image.$2=" '
    index($0, key) {
      i = index($0, key) + length(key)
      v = substr($0, i)
      sub(/^"/, "", v); sub(/"[[:space:]]*\\?[[:space:]]*$/, "", v)
      print v; exit
    }' "$1"
}

# The value declared in the workflow's `labels:` block for one lane. The lane
# opens at its `images: ${{ env.<VAR> }}` line and closes at the next lane's, so
# a key is always read from the lane that declared it.
workflow_label() {
  awk -v var="images: \${{ env.$1 }}" -v key="org.opencontainers.image.$2=" '
    index($0, "images: ${{ env.") && index($0, "_IMAGE }}") { inlane = index($0, var) > 0 }
    inlane && index($0, key) {
      i = index($0, key) + length(key)
      v = substr($0, i)
      sub(/^[[:space:]]*/, "", v); sub(/[[:space:]]*$/, "", v)
      print v; exit
    }' "$WORKFLOW"
}

for entry in $IMAGES; do
  image=${entry%%:*}
  rest=${entry#*:}
  dockerfile=${rest%%:*}
  envvar=${rest#*:}
  [ -f "$dockerfile" ] || { report "image-labels: missing $dockerfile"; continue; }

  for key in $SHARED; do
    d=$(dockerfile_label "$dockerfile" "$key" || true)
    w=$(workflow_label "$envvar" "$key" || true)
    if [ -z "$d" ]; then
      report "image-labels: $dockerfile declares no org.opencontainers.image.$key"
      continue
    fi
    if [ -z "$w" ]; then
      report "image-labels: $WORKFLOW declares no org.opencontainers.image.$key for $image"
      continue
    fi
    # The Dockerfile substitutes ${VERSION}/${REVISION} from build args; the
    # shared keys are all literals, so a plain comparison is right.
    if [ "$d" != "$w" ]; then
      report "image-labels: $image .$key disagrees —
    $dockerfile: $d
    $WORKFLOW: $w"
    fi
  done

  # base.name/base.digest must match the runtime stage's actual FROM pin, or the
  # image claims a parent it was not built on — worse than claiming none.
  from=$(grep -E '^FROM [^ ]+@sha256:' "$dockerfile" | tail -1 || true)
  if [ -z "$from" ]; then
    report "image-labels: $dockerfile has no digest-pinned FROM to check base.* against"
  else
    ref=$(printf '%s' "$from" | awk '{print $2}')
    want_name=${ref%@*}
    want_digest=${ref#*@}
    got_name=$(dockerfile_label "$dockerfile" base.name || true)
    got_digest=$(dockerfile_label "$dockerfile" base.digest || true)
    [ "$got_name" = "$want_name" ] \
      || report "image-labels: $dockerfile base.name is '$got_name', but its FROM is '$want_name'"
    [ "$got_digest" = "$want_digest" ] \
      || report "image-labels: $dockerfile base.digest is '$got_digest', but its FROM pins '$want_digest'"
  fi
done

# Annotations must reach the INDEX, which is the only place GHCR reads a package
# description from. Three lanes, three declarations.
levels=$(grep -c 'DOCKER_METADATA_ANNOTATIONS_LEVELS: index,manifest' "$WORKFLOW" || true)
[ "$levels" -eq 3 ] \
  || report "image-labels: expected 3 metadata steps with DOCKER_METADATA_ANNOTATIONS_LEVELS: index,manifest, found $levels"
annots=$(grep -c 'annotations: ${{ steps.meta.outputs.annotations }}' "$WORKFLOW" || true)
[ "$annots" -eq 3 ] \
  || report "image-labels: expected 3 build steps passing the annotations output, found $annots"

# The one predefined key we deliberately leave unset, recorded rather than
# silently skipped: `ref.name` is "name of the reference for a target", which the
# spec leaves to the consumer and which our tags already express — an image
# carries several tags, so a single ref.name would have to pick one arbitrarily.
# `created` is set by the action from the build time and needs no declaration.

if [ "$failures" -gt 0 ]; then
  echo "image-labels: $failures problem(s) — see above." >&2
  exit 1
fi
echo "image-labels: OK."
