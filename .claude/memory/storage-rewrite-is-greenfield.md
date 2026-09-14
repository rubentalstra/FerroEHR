---
name: storage-rewrite-is-greenfield
description: Owner ruling 2026-09-14 on #3337 - the storage redesign is a GREENFIELD rewrite; no production installations, so old databases may break, no copy tool or second generation, and each domain gets a NEW migration set of natural one-concern files (never one squashed baseline) replacing the old set outright
metadata:
  type: project
---

The storage redesign (#3337, plan `docs/plans/storage-redesign.md`) is a greenfield rewrite: each domain's migration set is re-authored as a sequence of NATURAL FILES, one concern per file, numbered per domain (owner direction 2026-09-14: never one squashed baseline; policies and grants each in their own file), the old `ehr`/`demographic`/`linkage`/`audit` sets are deleted in the same PR, and a database created by an earlier release is refused at boot with a message saying it predates the rewrite and must be recreated. No `ferroehr storage migrate` tool, no `posture.storage_generation`, no gen-1/gen-2 coexistence.

**Why:** the owner, 2026-09-14: "it's a greenfield rewrite, so old setups break, because no organizations use it in production". The 2026-09-09 migration-immutability stabilisation was declared for installations that exist; the owner set it aside for this one rework.

**How to apply:** the PR that lands the new baselines deletes the old sets and re-declares the immutability guard's baseline in the same change (a one-time, owner-ruled exception; the guard has no label escape by design, so the guard itself is amended in that PR). After it lands, the new baselines are the immutable ones again. Related: [[migrations-are-append-only]], [[rewrite-not-inherited-code]].
