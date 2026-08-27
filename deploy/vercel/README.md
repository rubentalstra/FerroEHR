# The hosted sandbox — how it deploys (sandbox.ferroehr.eu)

The sandbox is Vercel (container runtime) + Neon (PostgreSQL 18). It runs
the latest published release image with a demo posture baked on top, and it
resets to known demo data every night.

The Neon↔Vercel integration is LOAD-BEARING for exactly one thing: its
injected `DATABASE_URL`/`DATABASE_URL_UNPOOLED` is the server's DSN (the
config aliases map it to `db.url`, unpooled preferred — #2716). No hand-set
`FERROEHR__DB__URL` exists on the project, so the integration must stay
connected — established the hard way on 2026-08-27, when disconnecting it
was almost the "fix" for its OTHER half: the per-deployment branch
provisioning, which is pure overhead here (nothing reads the per-deploy
branch) and has a measured record of timing out and killing release-day
deploys (#2846). If that step misbehaves again, disable deployment
branching in the integration's settings — never disconnect the integration
without first hand-setting the DSN.

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

The deploy job verifies in two layers (#2846, after the v4.0.7 cut spent
20 blind minutes on a Vercel-side build failure):

1. **Precondition** (release calls): `:latest`'s digest must equal the
   release tag's image digest on GHCR before any ping — the needs-edge's
   guarantee, re-asserted where it is consumed, with anonymous registry
   reads.
2. **The served-version poll** is the acceptance; the hook response's job
   id is captured for the record, and the timeout message states what was
   already proven (the image side) so the remaining suspect (Vercel's own
   build/promotion — historically a flaky Neon integration step, #2846) is
   named, not guessed.

Watching the triggered deployment's own state was considered and REFUSED
(#2846): Vercel offers no read-only tokens, so it would cost a stored
full-access credential for a diagnostics-only win — against this
repository's no-long-lived-tokens posture.

## Two build-log warnings that are understood, not unnoticed (#2773)

Every Vercel build of `Dockerfile.vercel` prints two warnings. Both were
adjudicated first-hand against Vercel's own documentation (read 2026-08-27)
and neither has a configuration that removes it, so they will keep printing —
this section is what makes them read as understood.

1. `HEALTHCHECK is not supported for OCI image format and will be ignored.`
   The published app image bakes a `HEALTHCHECK` (`docker/Dockerfile`) for
   compose/podman consumers; Vercel's buildah builds the sandbox derivative
   in OCI format, which has no healthcheck field, so the instruction is
   dropped from the sandbox image only. The drop is provably inert here:
   the service configuration reference
   (<https://vercel.com/docs/services/config-reference>) offers no
   healthCheck or probe key, and the container-images model expects exactly
   one thing of the image — an HTTP server on `PORT`
   (<https://vercel.com/docs/functions/container-images>). Nothing on the
   platform ever reads a container healthcheck; readiness on this pipeline
   is our own post-deploy poll of `/ferroehr/rest/status`. Removing the
   instruction from the base image would silence the warning by taking the
   healthcheck away from every compose user — the wrong trade.
2. `Build output contains no "functions" or "static" directory` — the
   framework-output check firing on a container-only project. The service
   config reference has no key that declares container-only output
   (`outputDirectory` configures where a framework build writes, not what
   kind of output exists), so the warning is a known false alarm of the
   container preset: the deploy is real (the `services`/`rewrites` config
   routes it) and the workflow's version poll proves it serves.

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
