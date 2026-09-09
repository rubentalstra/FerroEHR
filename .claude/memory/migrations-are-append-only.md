---
name: migrations-are-append-only
description: "Owner ruling 2026-09-09 — stabilization declared: migration files are APPEND-ONLY, never edited; a schema change is a new file, guarded by migration-immutability.sh"
metadata:
  type: feedback
---

Owner ruling 2026-09-09: people run FerroEHR now, so wiping a volume is no
longer an option and an installation upgrades its database in place. Migration
files are **append-only**. Never edit, rename or delete a migration that exists
on `main`, including the squashed baselines and including a comment or a typo.
A schema change is a NEW `sqlx migrate add` file; a migration that turned out
wrong is superseded by a later one, never rewritten.

**Why:** sqlx records a checksum of every applied migration and refuses a
database whose recorded checksum no longer matches the file, so an edit does
not revise history. It locks every existing installation out of its own
database at boot, reporting a checksum rather than the edit that caused it.

**How to apply:** a new file, always. Editing a migration this branch itself
added is fine, because the guard compares against the merge base. There is no
escape-hatch label, deliberately.

This SUPERSEDES the greenfield ruling of 2026-08-20 (#2452), which said
migration files are edited in place and reserved this reversal for the moment
the owner declared stabilization. That moment is now, so the machinery that
ruling rejected as dead weight is exactly what belongs here: the durable rule
is `.claude/rules/sqlx-conventions.md` §Migrations, and
`scripts/checks/migration-immutability.sh` (the `migration-immutability` CI
job) is its failing check. Related: [[rewrite-not-inherited-code]],
[[en-route-findings-always-filed]].
