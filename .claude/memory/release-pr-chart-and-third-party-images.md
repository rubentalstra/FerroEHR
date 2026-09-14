---
name: release-pr-chart-and-third-party-images
description: Every release PR bumps the chart version (chart-version-guard fires otherwise); the chart lane's image read-back judges first-party images only, third-party tags (FerroTERM) are legitimately not X.Y.Z
metadata:
  type: feedback
---

At the v4.3.0 cut (2026-09-14) the release PR went red on `chart-version-guard` because only `appVersion` was bumped: the chart's own `version` must bump on EVERY release. The `chart` leg then refused the packaged chart because `artifacthub.io/images` carried `ghcr.io/rubentalstra/ferroterm:0.1.3`, a third-party image whose tag can never equal the FerroEHR version; #3376 made the read-back judge first-party images only, and `publish-chart.yml` dispatch was the recovery lane.

**Why:** the release procedure has two chart steps that are easy to skip, and a sibling product's image tag is not a release-consistency defect.

**How to apply:** the cut script bumps `version:` in `Chart.yaml` (8.3.0→8.3.1 style) beside `appVersion`; a chart-leg refusal naming a `ferroterm` (or other sibling) image is a lane bug, not a release bug. Related: [[session-workflow-gotchas]].
