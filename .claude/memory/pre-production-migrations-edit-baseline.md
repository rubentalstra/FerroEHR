---
name: pre-production-migrations-edit-baseline
description: "Owner rule — pre-production schema changes edit the baseline migration directly, never append ALTER/DROP migrations; keep migration files minimal"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 1be6641a-9768-4fd5-8149-acb2551a1d97
---

Owner rule (2026-07-14): while the app is pre-production (nothing deployed),
schema changes are made by **editing `0001_baseline.sql` (and the other
existing migration files) directly** — never by appending new ALTER/DROP
migration files. Keep the migration file count minimal; fold same-phase
additions into the baseline.

**Why:** there is no deployed database to migrate, so append-only migration
chains are pure noise; the owner wants a clean minimal schema history until
production.

**How to apply:** when a schema change is needed, rewrite the relevant SQL in
place (columns into CREATE TABLE, indexes added/removed at their definition),
delete any now-redundant migration files, and update the migration-count
guard test (`app/ehrbase/tests/persistence.rs`) in the same change. The
`sqlx migrate add` flow resumes only once something is deployed. Related:
[[owner-work-style]].
