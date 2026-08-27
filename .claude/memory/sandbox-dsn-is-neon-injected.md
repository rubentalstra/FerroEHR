---
name: sandbox-dsn-is-neon-injected
description: "The hosted sandbox's DSN is the Neon integration's injected DATABASE_URL(_UNPOOLED) via the config alias layer — the integration is load-bearing; never recommend disconnecting it without hand-setting the DSN first"
metadata:
  type: project
---

The hosted sandbox (sandbox.ferroehr.eu) has NO hand-set `FERROEHR__DB__URL`
on the Vercel project. Its DSN is the Neon↔Vercel integration's injected
`DATABASE_URL`/`DATABASE_URL_UNPOOLED`, which `app/ferroehr/src/config/alias.rs`
maps to `db.url` (unpooled preferred, #2716 — built for exactly this).

**Why:** on 2026-08-27 I claimed "the integration provides nothing" and urged
disconnecting it (to kill its flaky branch-per-deployment provisioning step,
#2846) — a disconnect would have removed the injected vars and taken the
sandbox's database away. The owner caught it ("i did not add a manual
FERROEHR__DB__URL"). The mistake: I verified the TOML carries no DSN and that
the db was UP, then inferred the variable NAME without grepping the config
loader for aliases.

**How to apply:** the integration's env INJECTION stays connected; only its
branch-per-deployment toggle is fair game when the provisioning step flakes.
Before ever claiming an env var / integration is unused, grep the config
alias + loader layer (`config/alias.rs`, `config/loader.rs`) — the FERROEHR__
grammar is not the only spelling the server reads. Related:
[[session-workflow-gotchas]].
