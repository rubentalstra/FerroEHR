# Try it in Codespaces

The fastest way to try FerroEHR is a GitHub Codespace: one click boots the
published images in your browser, with nothing installed on your machine.

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/rubentalstra/FerroEHR)

<!-- toc -->

## What you get

Creating a Codespace on the FerroEHR repository starts a container that pulls
the published quickstart images and runs `docker compose up` for you:

- the FerroEHR server with a preconfigured PostgreSQL 18,
- the admin console,
- the Swagger UI for the full REST API.

The stack boots automatically. When the terminal prints `FerroEHR is up.`,
open the **PORTS** panel and follow the forwarded ports:

| Port | What it serves |
|---|---|
| `8080` | the REST API, with the Swagger UI at `/ferroehr/rest/swagger-ui` |
| `3000` | the admin console |

Sign in with the quickstart credentials, `ferroehr` / `ferroehr`. From there
the [Getting started](../getting-started.md) walkthrough applies unchanged:
create an EHR, upload a template, commit a composition, and query it back
with AQL, either from the Swagger UI or with `curl` in the Codespace
terminal.

## What it is, and what it is not

The Codespace is a **tester sandbox** and always runs the published release
images pinned in the standalone `docker-compose.yml`. It does not build the
checkout you opened it on: the `COMPOSE_FILE` environment variable inside the
container pins every `docker compose` command to that file, so the
repository's development override (which switches to from-source builds) does
not apply. To develop FerroEHR itself, use a local checkout as described
under [Repository development](compose.md#repository-development).

The Codespace runs on your own GitHub account. The smallest machine type
(2 cores, 8 GB) is enough, and GitHub's free monthly Codespaces allowance
covers a long evaluation. Stop or delete the Codespace when you are done;
a stopped Codespace restarts the stack automatically on resume.

## The hosted sandbox

A public demo at `sandbox.ferroehr.eu` is being set up as the second
zero-install path: no GitHub account needed, point any REST client at it.
So everyone knows what it runs on and what to expect from it:

| | |
|---|---|
| Compute | Vercel Fluid (a container function that scales to zero when idle) |
| Database | Neon serverless PostgreSQL 18, region Frankfurt (`fra1`) |
| Database plan | Neon free tier: 0.5 GB storage, up to 2 CU / 8 GB RAM, 100 CU-hours of compute per month |
| Data durability | none by design; demo data only, wiped and reseeded on a schedule |

Both layers scale to zero, so the first request after an idle period pays a
double cold start and can take a few seconds; after that it responds at
normal speed. The free compute budget means the sandbox may be unavailable
near the end of a heavy month. It is a demo, never a place for real data.

The sandbox deploys on release tags only, so it always runs the latest
FerroEHR release rather than a development snapshot.

## If the stack is not up

The boot log is in the terminal that ran `start-stack.sh`. To restart the
stack by hand:

```bash
bash .devcontainer/start-stack.sh
```

`docker compose ps` shows the three services; the server is healthy when
`curl http://localhost:8080/health` answers `200`.
