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

## If the stack is not up

The boot log is in the terminal that ran `start-stack.sh`. To restart the
stack by hand:

```bash
bash .devcontainer/start-stack.sh
```

`docker compose ps` shows the three services; the server is healthy when
`curl http://localhost:8080/health` answers `200`.
