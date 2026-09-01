# The hosted sandbox (sandbox.ferroehr.eu)

sandbox.ferroehr.eu is the public demo: the viewer as the landing
surface over a live CDR, the demo credentials `ferroehr` / `ferroehr`, demo
data wiped nightly. This directory is the whole deployment posture, mirrored
from Veredictum's `deploy/hosted` (#2974).

## Why a box, and not a platform

The sandbox used to run on Vercel's container platform, and the platform kept
fighting the server: instances scale in and out under a request model, boots
re-run the migrations on visitor traffic, and the nightly reset had to promote
whole deployments to get one fresh boot (#2941, #2947, #2948 are the
workaround trail — and it was slow). So the sandbox runs on one machine that
does not stop. The cost of that choice is ownership: the firewall, the
certificate, the deploy path and the answer to "is it up" are all things this
directory states.

## The machine

A Hetzner **CX33** — 4 shared vCPU, 8 GB RAM, 80 GB NVMe SSD, 20 TB included
traffic; €8.49/month net (€0.0136/h, DE list price since 15 June 2026,
<https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/>) —
in Nuremberg (`eu-central`), dual-stack (167.233.172.220 /
2a01:4f8:1c16:5d4::/64), provisioned once from `cloud-init.yaml` and resized
in place from the original CPX22 on 2026-09-01. The compose memory limits are
sized for this box; on a resize they move with it, and neither is a code
change.

The database is NOT on the box: a standalone Neon project (PostgreSQL 18,
Frankfurt) consumed over its **direct** (non-pooled) endpoint — Neon's pooler
is PgBouncer in transaction mode, which breaks the session GUCs and the
boot-time migrations (https://neon.com/docs/connect/connection-pooling). The
connection string lives in the box's `.env` and nowhere else; CI never holds
it.

## What is in this directory

| File | What it is | How the box gets it |
|---|---|---|
| `cloud-init.yaml` | A fresh box to serving state on first boot: the `deploy` user, key-only SSH, both firewalls, unattended upgrades, Docker, capped logs, and the one command the CI key may run | the server's user data at creation, once |
| `docker-compose.yml` | What runs on the box: the CDR and the console with their healthchecks and memory limits, behind Caddy | baked into the ferroehr image at `/opt/sandbox-posture/`, and `deploy.sh` installs it from the image it pulled |
| `Caddyfile` | Automatic TLS, and the routing table (the CDR owns `/ferroehr/*`, `/health*`, `/management*`, `/.well-known/*`; the console is everything else) | the same way; a change to it restarts Caddy |
| `ferroehr.sandbox.toml` | The CDR's sandbox posture (demo user, admin API on per #2965, management off) | the same way |
| `ferroehr-viewer.sandbox.toml` | The console's sandbox posture | the same way |
| `env.example` | A copy-to-`.env` template: the Neon direct DSN and the image references | never. `.env` is the operator's file, written by hand on the box |

The box holds **no checkout of this repository** and fetches nothing from it
over the network: the posture travels inside the published ferroehr image, so
a posture change arrives at a release, with that artifact's provenance. The
`hosted-deploy-script` CI job extracts `deploy.sh` out of `cloud-init.yaml`
and shellchecks it, and checks that `docker/Dockerfile` bakes every posture
path the script reads.

## How a deploy happens

`.github/actions/hosted-deploy` is the only thing that deploys, and it carries
every step. Three callers:

1. **A real release** — the `sandbox` job in `release.yml`, after scan-and-tag
   moves `:latest`. A prerelease moves no `:latest`, so it deploys nothing.
   The `sandbox-reseed` leg then wipes, restarts and reseeds.
2. **A manual `workflow_dispatch`** of `.github/workflows/hosted-deploy.yml`.
3. **The reseed** (`sandbox-reseed.yml`, nightly + after every release
   deploy) — verb `restart` only: recreate the CDR so a fresh boot re-runs
   the migrations against the wiped database.

The deploy key is restricted on the box to `deploy.sh` (`command=` in
`cloud-init.yaml`), which reads `SSH_ORIGINAL_COMMAND` and accepts exactly
`deploy` and `restart`. The lane builds nothing; it asserts by digest that
`:latest` is the tagged image, refuses an image that does not declare
`eu.ferroehr.image.carries-sandbox-posture=true`, deploys through
[`rubentalstra/hetzner-deploy-action`](https://github.com/rubentalstra/hetzner-deploy-action),
and then fetches `https://sandbox.ferroehr.eu/ferroehr/rest/status` FROM the
runner, requiring the served `server_version` — a bare 200 is not proof, the
deployment being replaced answers 200 too.

`HOSTED_SSH_KEY` and `HOSTED_KNOWN_HOSTS` are ENVIRONMENT secrets in the
`hosted` environment, so nothing else in this repository can reach the key.
No Hetzner API token exists in CI at all — the deploy talks to the host and
nothing else, so a leaked key cannot destroy the server.

## Bootstrap and recovery

The first posture on a fresh box (or recovery from a broken one) is manual by
design: copy this directory's four posture files plus a filled-in `.env` to
`/opt/ferroehr-sandbox/` and run `docker compose up -d` as the `deploy` user.
From the next release on, `deploy.sh` keeps them current from the image, and
each replaced file leaves a `.prev` copy beside it for rollback.
`deploy.sh` itself does not self-update: a new version reaches the box by
hand, or by rebuilding the box from `cloud-init.yaml`.

## How it is watched

Four layers, each answering a different question: the containers' own
healthchecks plus `restart: unless-stopped`; the deploy's own verification;
`.github/workflows/hosted-watch.yml` every fifteen minutes (up, the served
version against the release pointer, and the console's landing markup — one
reused issue, closed on recovery); and size-capped log rotation, because a
box that fills its disk fails in a way that looks like nothing at all.

## What the box keeps, and what it never holds

Nothing durable and nothing secret beyond the one `.env`: the containers are
rebuilt from published images, the data is wiped nightly by design
(`sandbox-reseed.yml` — the wipe runs from CI against Neon directly, never
through the box), and no signing key, token or credential beyond the database
DSN exists on the host. **No backups**, deliberately.
