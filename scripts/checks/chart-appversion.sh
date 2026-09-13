#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
#
# scripts/checks/chart-appversion.sh — the committed chart release facts track
# the workspace version, the compose treatment (#2890).
#
# The committed `appVersion` equals the workspace version in Cargo.toml, bumped
# in the release PR beside the docker-compose.yml image tags — one sweep, one
# philosophy for both committed artifacts. The publish lane still injects
# `--app-version ${TAG#v}` and rewrites the `artifacthub.io/images` tags at
# package time (#2779); since the tree now carries the same value, the
# injection is belt-and-braces for the release leg and remains what makes the
# dispatch-driven `publish-chart.yml` recovery lane correct between releases.
#
# The one accepted cost (adjudicated on #2890, the same window the compose
# defaults have always accepted): between a release PR's merge and the tag's
# Containers leg publishing the X.Y.Z images, a from-tree `helm install`
# references image tags that do not exist yet. The window is ~30-60 minutes,
# exists only during a cut, and buys a tree whose committed versions never lag.
#
# Four properties, each with real content:
#
#   1. `appVersion` equals the workspace version (Cargo.toml is the authority,
#      the same source compose-image-tags.sh reads). A lagging default ships a
#      chart whose images predate the tree's own configuration vocabulary; a
#      diverging one means the release sweep missed a site.
#   2. Every FIRST-PARTY `artifacthub.io/images` tag equals `appVersion`. The
#      two describe one thing — the product version this chart defaults to —
#      and the package-time injection rewrites both from one input, so a tree
#      where they disagree would publish an annotation the package's own
#      appVersion contradicts. First-party means the images built from THIS
#      repository at this version: `ghcr.io/rubentalstra/ferroehr` and its
#      `ferroehr-*` siblings. A third-party image the chart deploys —
#      `ghcr.io/rubentalstra/ferroterm` since #3305 — moves on its own release
#      line, is pinned in the chart's own values and helpers, and is neither
#      equal to appVersion nor rewritten at package time; holding it to this
#      rule would make every FerroEHR release claim a FerroTERM version that
#      does not exist.
#   3. Chart.yaml carries NONE of the injected annotations. A committed
#      `artifacthub.io/changes`, `containsSecurityUpdates` or `prerelease` key
#      would collide with the injected one and leave the packaged metadata
#      depending on YAML key-collision behaviour.
#   4. The chart README — a GENERATED file, and the one Artifact Hub renders as
#      the package front page — restates the same appVersion. It is generated
#      FROM Chart.yaml, so a disagreement means it was never regenerated.
#
# Usage: scripts/checks/chart-appversion.sh   (no arguments)
# Callers: the `chart-appversion` job in ci.yml, and the `plan` job of
# release.yml (which re-runs the whole guard tier at the tagged commit; plan
# separately asserts the tag equals the workspace version, so property 1 there
# transitively pins appVersion == ${TAG#v}).

set -euo pipefail

# shellcheck source=scripts/lib/guard-args.sh
. "$(dirname "$0")/../lib/guard-args.sh"
guard_no_args "$@"
cd "$(dirname "$0")/../.."

CHART=deploy/helm/ferroehr/Chart.yaml
MANIFEST=Cargo.toml

for required in "$CHART" "$MANIFEST"; do
  [[ -f "$required" ]] || { echo "::error::missing $required" >&2; exit 1; }
done

failures=0
report() {
  echo "::error::$1" >&2
  failures=$((failures + 1))
}

app=$(sed -nE 's/^appVersion: *"?([^"]+)"?$/\1/p' "$CHART")
if [[ -z "$app" ]]; then
  report "could not read appVersion from $CHART."
  exit 1
fi

workspace=$(sed -nE 's/^version = "(.*)"$/\1/p' "$MANIFEST" | head -1)
if [[ -z "$workspace" ]]; then
  report "could not read the workspace version from $MANIFEST."
  exit 1
fi

# 1. appVersion equals the workspace version (the compose-image-tags contract).
if [[ "$app" != "$workspace" ]]; then
  report "$CHART appVersion is $app but the workspace version is $workspace — the release cut bumps appVersion beside the compose image tags (one version sweep, .claude/rules/changelog.md). Refresh appVersion + the artifacthub.io/images tags + the generated README: helm-docs --chart-search-root deploy/helm/ferroehr --template-files README.md.gotmpl"
fi

# 2. The FIRST-PARTY image annotations agree with it. The pattern is the one
# release-facts.sh rewrites, so the set checked here and the set rewritten at
# package time are the same set by construction.
FIRST_PARTY='image: ghcr\.io/rubentalstra/ferroehr(-[a-z0-9-]+)?:[^ ]+'
bad=$(grep -oE "$FIRST_PARTY" "$CHART" | grep -v ":${app}\$" || true)
if [[ -n "$bad" ]]; then
  report "$CHART artifacthub.io/images tags disagree with appVersion ($app):
$bad"
fi
if ! grep -qE "$FIRST_PARTY" "$CHART"; then
  report "$CHART declares no first-party artifacthub.io/images entry — the publish lane rewrites those tags at package time and would rewrite nothing."
fi

# 3. None of the injected annotations is committed.
for injected in artifacthub.io/changes artifacthub.io/containsSecurityUpdates artifacthub.io/prerelease; do
  if grep -q "^  ${injected}:" "$CHART"; then
    report "$CHART commits ${injected}, which build-chart.yml injects at package time — two declarations of one key make the packaged metadata depend on YAML key-collision behaviour."
  fi
done

# 4. The generated README agrees with it.
README=deploy/helm/ferroehr/README.md
if [[ -f "$README" ]]; then
  if ! grep -qF -- "--set image.tag=${app}" "$README"; then
    report "$README teaches an image.tag other than the committed appVersion ($app) — it is generated from Chart.yaml, so regenerate it: helm-docs --chart-search-root deploy/helm/ferroehr --template-files README.md.gotmpl"
  fi
else
  report "no $README — the chart publishes a generated README and this check has nothing to read."
fi

if [[ "$failures" -gt 0 ]]; then
  exit 1
fi
echo "chart-appversion: appVersion $app equals the workspace version, the first-party artifacthub.io/images tags and the generated README agree with it, and no injected annotation is committed — OK."
