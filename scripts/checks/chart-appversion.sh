#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
#
# scripts/checks/chart-appversion.sh — what the chart's COMMITTED release facts
# still have to be true about, now that the published ones are injected (#2779).
#
# Until #2779, `appVersion` and the `artifacthub.io/images` tags were per-release
# facts kept equal to the workspace version by hand at every cut, and guarded in
# three places. They are not per-release facts any more: `build-chart.yml`
# packages with `helm package --app-version <released version>` and rewrites the
# image annotation's tags at the same moment, exactly as it already injects
# `artifacthub.io/changes` — "facts about a release, and a committed copy would
# be stale from the next merge onwards". So the PUBLISHED chart is correct by
# construction and no longer depends on the tree at all.
#
# What the committed values still ARE: the defaults for a chart packaged OUTSIDE
# a release — the between-releases `publish-chart.yml` dispatch — and for anyone
# who runs `helm template`/`helm install` against this directory. That leaves
# three properties with real content, and this script is the one home for them:
#
#   1. `appVersion` names a version this project has actually RELEASED. A default
#      image tag that was never published produces an ImagePullBackOff, and a
#      future version is a promise the registry cannot keep. CHANGELOG.md is the
#      machine authority: a released version has a `## [X.Y.Z]` section.
#   2. Every `artifacthub.io/images` tag equals `appVersion`. The two describe
#      one thing — the product version this chart defaults to — and the injection
#      rewrites both from one input, so a tree where they already disagree would
#      publish an annotation the package's own appVersion contradicts.
#   3. Chart.yaml carries NONE of the injected annotations. A committed
#      `artifacthub.io/changes`, `containsSecurityUpdates` or `prerelease` key
#      would collide with the injected one and leave the packaged metadata
#      depending on YAML key-collision behaviour.
#   4. The chart README — a GENERATED file, and the one Artifact Hub renders as
#      the package front page — restates the same appVersion. It is generated
#      FROM Chart.yaml, so a disagreement means it was never regenerated, and
#      the injection has nothing to move: the v4.0.5 cut left it a release
#      behind (6.0.19/4.0.4 against a 6.0.20/4.0.5 chart) and only a local
#      `deploy/helm/validate.sh` noticed, because the CI drift check skips
#      itself when helm-docs is not installed on the runner.
#   5. `appVersion` is FRESH: one of the two newest STABLE releases in
#      CHANGELOG.md (#2804). With the cut no longer bumping it (#2779), nothing
#      moved the committed default and the gap between it and the tree's
#      advancing `config` defaults grew unboundedly, unseen by any lane
#      (chart-boot overrides the image; the dispatch judge uses sha-/develop).
#      "One stable behind" is the deliberate slack that keeps #2779's design:
#      the release PR that cuts X.Y.Z does NOT have to refresh (that would
#      re-impose the hand edit), but the one after it fails here until someone
#      does — bounded lag, enforced by the guard tier that already runs,
#      instead of a scheduled watcher filing issues. Stable sections only:
#      an -rc default would hand production installs a pre-release image, so
#      an rc never satisfies this arm even though property 1 accepts it as
#      "released".
#
# What is deliberately NOT checked here any more: equality with the workspace
# version. Asserting it would re-impose the hand edit this change removes.
#
# Usage: scripts/checks/chart-appversion.sh
# Callers: the `chart-appversion` job in ci.yml, and the `plan` job of
# release.yml (which re-runs the whole guard tier at the tagged commit).

set -euo pipefail
cd "$(dirname "$0")/../.."

CHART=deploy/helm/ferroehr/Chart.yaml
CHANGELOG=CHANGELOG.md

for required in "$CHART" "$CHANGELOG"; do
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

# 1. A released version, per the changelog.
if ! grep -qE "^## \[${app//./\\.}\]" "$CHANGELOG"; then
  report "$CHART appVersion is $app, which has no '## [$app]' section in $CHANGELOG — the committed appVersion is the default image tag for a chart packaged outside a release, so it must name a version that was actually published. (A release no longer needs to bump it: build-chart.yml injects the released version with 'helm package --app-version'.)"
fi

# 5. Fresh: one of the two newest stable releases (rc/pre-release sections are
#    skipped — a suffixed default would hand production installs a pre-release).
stable=$(grep -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' "$CHANGELOG" | head -2 | tr -d '[]# ')
stable1=$(printf '%s\n' "$stable" | sed -n 1p)
stable2=$(printf '%s\n' "$stable" | sed -n 2p)
if [[ -z "$stable1" ]]; then
  report "$CHANGELOG has no stable '## [X.Y.Z]' section to hold the appVersion against."
elif [[ "$app" != "$stable1" && "$app" != "$stable2" ]]; then
  report "$CHART appVersion is $app, but the two newest stable releases are ${stable1} and ${stable2:-—}. The committed default may lag the newest cut by at most one stable release (#2804) — refresh it (appVersion + the artifacthub.io/images tags + the generated README: helm-docs --chart-search-root deploy/helm/ferroehr --template-files README.md.gotmpl)."
fi

# 2. The image annotation agrees with it.
bad=$(grep -oE 'image: ghcr\.io/[^:]+:[^ ]+' "$CHART" | grep -v ":${app}\$" || true)
if [[ -n "$bad" ]]; then
  report "$CHART artifacthub.io/images tags disagree with appVersion ($app):
$bad"
fi
if ! grep -q 'image: ghcr\.io/' "$CHART"; then
  report "$CHART declares no artifacthub.io/images entry — the publish lane rewrites those tags at package time and would rewrite nothing."
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
echo "chart-appversion: appVersion $app is a stable release at most one behind the newest ($stable1), the artifacthub.io/images tags and the generated README agree with it, and no injected annotation is committed — OK."
