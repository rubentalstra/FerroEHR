---
name: sandbox-dsn-is-neon-injected
description: The sandbox runs on the Hetzner box (deploy/hosted); its Neon DSN lives ONLY in the box's /opt/ferroehr-sandbox/.env — CI never holds it
metadata:
  type: project
---

Since #2974 (2026-08-31) the hosted sandbox runs on a Hetzner CPX22 (deploy/hosted/ is the whole posture — Veredictum's pattern), not on Vercel. The database is a STANDALONE Neon project consumed over its DIRECT (non-pooled) endpoint; the DSN lives in exactly two places: the box's operator-written `/opt/ferroehr-sandbox/.env` (as `DATABASE_URL`, mapped to db.url by config/alias.rs) and the `SANDBOX_DATABASE_URL` secret in the `sandbox-reseed` environment (the nightly wipe connects from the runner). The `hosted` environment holds only SSH material (`HOSTED_SSH_KEY`, `HOSTED_KNOWN_HOSTS`).

**Why:** transaction-mode PgBouncer (the `-pooler` host) breaks session GUCs and boot migrations — never dial the pooled endpoint from the CDR. CI never forwards the DSN to the box: the deploy key runs one script and carries nothing.

**How to apply:** posture changes travel inside the ferroehr image (`/opt/sandbox-posture`, docker/Dockerfile) and reach the box at a release via the `hosted` deploy lane; the box holds no checkout. Deploys: `.github/actions/hosted-deploy` (release `sandbox` job + `hosted-deploy.yml` dispatch); reseed restarts the CDR through the same restricted key (verb `restart`). The alias layer (DATABASE_URL/PORT etc.) stays product surface for any PaaS user. See [[session-workflow-gotchas]].
