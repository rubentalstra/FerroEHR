---
name: storage-generation-2-review
description: Verified findings from the generation-2 storage rewrite review (#3342/#3344/#3378, branch feat/storage-generation-2) — the correlated vo_head subquery, the write-once-read-never promoted columns, the immutability escape hatch, and the stale-claim clusters to re-check
metadata:
  type: feedback
---

Verified 2026-09-15 against `feat/storage-generation-2` (PR #3390), design
authority `docs/plans/storage-redesign.md`.

**Recurring defect shapes in this subsystem — check these first on any storage
change:**

- **"Current version" as a correlated scalar subquery.** 27 sites spell it
  `v.sys_version = (SELECT h.trunk_head_sys_version FROM vo_head h WHERE
  h.vo_id = v.vo_id)` (`storage/version_repo/read.rs`, `meta.rs`,
  `ehr_repo.rs`, `service/message/export.rs`,
  `service/linkage/cohort/predicate.rs`). PostgreSQL does not pull up an
  EXPR_SUBLINK, so the probe runs once per candidate version row — the
  opposite of the "one primary-key probe on `vo_head`" the design claims.
  The AQL emitter's `is_trunk_head` uses an EXISTS sublink (pullable) and is
  the shape to hold the others to. Zero `JOIN vo_head` exists in the tree.
- **Promoted columns written and never read.** `node.name_code` /
  `name_terminology` are populated by `storage/codec.rs` but
  `aql/sql/predicate.rs::name_cond` still `jsonb_path`s `$.name.defining_code`
  — while the migration COMMENT, the CHANGELOG and three doc pages all claim
  the predicate uses them. Same class as the `citem_num` the plan deleted.
- **Stale generation-1 vocabulary survives in doc comments after a rename
  sweep**: "union view", "`*_all`", "close the superseded lineage tip",
  "`sys_period` untouched", "the mirrors are FK-free". Grep
  `union view|_all views|sys_period|close-out|lineage tip` over `app/` after
  any storage change.
- **`tier::freeze` stamps `archived_at = now()`**, so any caller that first
  restores a recorded `archived_at` (the archive LOAD path) silently loses it;
  and the demographic load path restores the marker without freezing, leaving
  `tier='hot'` beside a non-null `archived_at`.
- **Archive/dump format changes are unguarded**: `Manifest.archive_version` is
  written at dump and read nowhere, so an incompatible old archive fails on a
  NOT NULL violation rather than a typed refusal.

**Discipline:** `scripts/checks/migration-immutability.sh` grew a permanent
bypass keyed on the `ext.storage_generation()` literal. `.claude/rules/
reliability.md` §"A migration that has shipped is never edited" and
`sqlx-conventions.md` §Migrations both state there is DELIBERATELY no escape
hatch, and the owner set the rule aside ONCE. Its safety story (the boot
refusal) is not machine-linked: `db::mod.rs FIRST_GENERATION_SCHEMAS` is a
hardcoded list a future generation cut would have to remember to extend.

**What is genuinely right (do not re-litigate):** the append-only `version`
table and derived validity; `version_at` restricted to the trunk with the
master06 §Copying adjudication and a branch-only test; the FK `ON UPDATE
CASCADE` cross-partition proof (`service_admin.rs
archive_physically_moves_rows_to_the_cold_tier_and_back` — `tier::freeze`
names only `version`, so the node/attestation assertions ARE the proof); the
DDL-template drift + differ-only-in-CHECK tests; the old-database boot refusal
with its test; the single-tenant catalog test; migration comments citing only
vendored specs and PostgreSQL docs.
