# The hosted sandbox — how it deploys (sandbox.ferroehr.eu)

The sandbox is Vercel (container runtime) + Neon (PostgreSQL 18). It runs
the latest published release image with a demo posture baked on top, and it
resets to known demo data every night.

## The delivery pipeline — one owner, explicit ordering

Vercel never builds on its own: git-triggered deployments are OFF
(`vercel.json` `git.deploymentEnabled` is false for every branch) and no
ignore script exists. Every deploy is a POST to the project's Deploy Hook,
and the only place that POST happens is `.github/workflows/sandbox-deploy.yml`:

1. **A release publishes.** The Containers pipeline builds and pushes
   `ghcr.io/rubentalstra/ferroehr:X.Y.Z` and moves `:latest` (release tags
   only, never prereleases). When that pipeline SUCCEEDS for a release tag,
   `sandbox-deploy.yml` runs (called by the release pipeline after the scanned tags apply), pings the hook, and Vercel
   builds `Dockerfile.vercel` — `FROM ghcr.io/rubentalstra/ferroehr:latest`,
   a pull + retag measured in seconds. The image provably exists before the
   ping, so no ordering race exists to guard against.
2. **A posture change lands on develop** (`Dockerfile.vercel`,
   `vercel.json`, `deploy/vercel/**`): the same workflow fires on the push
   and redeploys the current release image with the new posture.
3. **Manual**: `workflow_dispatch` on Sandbox deploy (reseed skippable).

After the deploy, the workflow polls `/ferroehr/rest/status` until the new
deployment serves (cold boots run the migrations), then calls the reseed
(the reusable half of `sandbox-reseed.yml`): wipe, wait out the serverless
scale-in window, wake, seed through the public API. A release that edits
the baseline migration therefore heals at deploy time; the nightly reseed
remains as the janitor.

## What can fail, and where it shows

Sandbox failures live in the two sandbox-named workflows (Sandbox deploy,
Sandbox reseed) and nowhere else. CI, Containers, the chart and release
lanes carry no sandbox steps, so a sandbox outage can never redden them.

## What is deliberately NOT here

- No hand-maintained image pin: `:latest` is moved by the image lane on
  release tags; the release procedure carries zero sandbox steps.
- No ignore script / skip-until-exists guard: nothing needs skipping when
  the only trigger fires after the image exists.
- No secrets beyond two: `SANDBOX_DEPLOY_HOOK_URL` (a plain trigger URL)
  and `SANDBOX_DATABASE_URL` (Neon direct endpoint, environment-scoped,
  Neon-fenced so it can never wipe a non-Neon database).

## Files

| File | Role |
|---|---|
| `vercel.json` | routing, container service, git auto-deploys OFF |
| `Dockerfile.vercel` | `FROM …:latest` + the baked sandbox posture |
| `deploy/vercel/ferroehr.sandbox.toml` | the demo configuration |
| `.github/workflows/sandbox-deploy.yml` | the ONLY deploy trigger + verify + reseed call |
| `.github/workflows/sandbox-reseed.yml` | nightly janitor + the reusable wipe/seed |
| `scripts/sandbox/reseed.sh` | seeds demo data through the public API |
