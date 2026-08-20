---
name: greenfield-migrations-editable
description: "Owner ruling 2026-08-20 (#2452) — greenfield: migration files are edited in place, deployments recreate; NEVER add checksum reconciliation/immutability machinery or re-file the edits as a defect"
metadata:
  type: feedback
---

Owner ruling 2026-08-20 (issue #2452, delivered bluntly): FerroEHR is a
GREENFIELD app — no installation upgrades a database in place; deployments
recreate. Editing the squashed `0001_baseline.sql` files (and any existing
migration) in place is the deliberate policy for this phase, and the files
stop being touched once things stabilize.

**Why:** I filed the in-place edits as a P1 upgrade-breaking defect and
started building checksum-reconciliation machinery (an allowlist of
released byte-states repaired at boot + a CI immutability guard). The owner
rejected the whole premise: there is nobody to protect — the machinery is
dead weight.

**How to apply:** never re-file in-place migration edits as a defect, never
add reconciliation/immutability guards, and never treat sqlx's
applied-migration immutability as binding here UNTIL the owner explicitly
declares stabilization (then migrations become append-only). The durable
rule lives in `.claude/rules/sqlx-conventions.md` §Greenfield migration
policy. Related: [[rewrite-not-inherited-code]] (this is a rewrite;
breaking changes preferred), [[en-route-findings-always-filed]].
