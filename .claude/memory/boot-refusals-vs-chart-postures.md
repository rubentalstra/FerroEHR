---
name: boot-refusals-vs-chart-postures
description: A new boot-time configuration refusal must be checked against the chart's rendered value sets (deploy/helm/ci/*-values.yaml) before it ships; the chart-boot CI job crash-loops on a refused documented posture
metadata:
  type: feedback
---

Before adding a boot refusal for a configuration COMBINATION, render the chart's value sets (`deploy/helm/ci/default-values.yaml`, `all-features-values.yaml`) and check the combination is not one they ship; if it is, the finding is a boot WARNING naming the tracker issue, or the fix itself, never a refusal in a patch release.

**Why:** #3360 (2026-09-14) refused `tenancy.enabled` together with the outbox readers; the chart's all-features posture combines exactly those, and the `chart-boot` CI job reported the deployment crash-looping. The refusal became a warning naming #3355.

**How to apply:** `grep -n 'enabled' deploy/helm/ci/all-features-values.yaml` for every key the refusal reads; run `bash deploy/helm/validate.sh` and the chart-boot check locally when a refusal touches keys the chart renders. See [[slim-feature-lanes-gate-helpers]] for the sibling lane trap.
