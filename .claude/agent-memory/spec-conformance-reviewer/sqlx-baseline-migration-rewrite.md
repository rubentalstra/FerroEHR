---
name: sqlx-baseline-migration-rewrite
description: Recurring repo pitfall — post-release PRs edit app/ferroehr/migrations/ehr/0001_baseline.sql in place while 0002+ exist, which sqlx's applied-migration checksum check turns into a boot failure for any existing deployment
metadata:
  type: feedback
---

Confirmed 2026-08-19 while verifying #1787. `app/ferroehr/migrations/ehr/`
carries `0001_baseline.sql` **plus** `0002_event_outbox` … `0008_spec_profile…`,
yet `git log -- 0001_baseline.sql` shows it edited by #1804, #1812, #1826, #2339.

**Why it matters:** `db/mod.rs:408` uses `sqlx::migrate!("migrations/ehr")` with
no checksum relaxation, and sqlx stores a checksum per applied migration — a
changed applied migration is `MigrateError::VersionMismatch`
(<https://docs.rs/sqlx/latest/sqlx/migrate/enum.MigrateError.html>), surfaced by
`DbError::Migrate`. A fresh database and every test clone pass (they apply the
new bytes from scratch), so **CI is green and only an in-place upgrade of an
existing deployment fails** — and the product ships releases + a published Helm
chart, so such deployments exist.

`.claude/rules/sqlx-conventions.md` §Migrations already forbids it ("schema
changes are migrations on top, never a rewrite of shipped history") and pins the
official CLI (`sqlx migrate add --sequential`).

**How to apply:** on any review of a PR whose diff touches a `0001_baseline.sql`,
check whether higher-numbered migrations exist in that directory; if they do,
report it as a release-blocking finding, not a style note.
