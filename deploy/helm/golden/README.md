# Golden Helm renders

These are the committed `helm template` outputs the chart is expected to
produce, used by `deploy/helm/validate.sh` (and CI) to catch unintended drift.

- `default.yaml` — `ci/default-values.yaml` (chart defaults + an external-Secret
  DB DSN; all optional integrations OFF, the hardened default security posture on).
- `all-features.yaml` — `ci/all-features-values.yaml` (every optional
  integration enabled, separate management port, autoscaling, egress policy,
  mounted config files).

## Regenerating

Whenever a chart template or the `ci/*-values.yaml` overlays change and the
render is *intended* to differ, regenerate and review the diff:

```bash
deploy/helm/validate.sh --update
git diff deploy/helm/golden
```

`deploy/helm/validate.sh` (no args) fails on any drift, so a stale golden is
caught before merge. Renders are pinned to release name `ferroehr`, namespace
`ferroehr`. Do not hand-edit these files.
