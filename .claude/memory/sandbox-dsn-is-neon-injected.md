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

**The branching incident's ops facts** (2026-08-27, resolved by the owner
disabling the toggle; a post-toggle fresh deploy came up Ready):

- The toggle lives under **Vercel → Storage → the Neon database → the
  connected project → Deployments Configuration** — NOT Settings →
  Integrations (the native/marketplace integration configures through the
  Storage tab; sending the owner to Integrations cost a search loop).
  Docs: neon.com/docs/guides/vercel-managed-integration.
- The diagnostic that settled it: Vercel's deploy list as a natural
  experiment — fresh builds (which run "Provisioning Integrations") all
  Error, redeploys (which SKIP provisioning) all Ready. Compounding
  mechanism: every deploy created a Neon branch nobody deleted, so creation
  slowed toward Vercel's provisioning timeout; release days (most deploys)
  tipped first.
- The sandbox's DB provider is VERSION-LOCKED to hosts serving Postgres 18
  (uuidv7 + WITHOUT OVERLAPS in the baseline migrations): Neon serves 18;
  Supabase tops out at 17 (verified against supabase.com/changelog +
  upgrade docs, 2026-08-27) — a Supabase switch is a version wall, not a
  preference.
