-- SPDX-FileCopyrightText: FerroEHR contributors
-- SPDX-License-Identifier: MIT

-- ext schema: multi-tenancy session context (an extension — no openEHR spec
-- governs multi-tenancy; E2).
--
-- Appended to the ext baseline (0001 — append-only forever after). This defines
-- the single function that BOTH the RLS policies AND the `tenant_id` column
-- DEFAULTs on the `ehr` scoping tables read to discover "the current request's
-- tenant". Runs with search_path = ext.
--
-- The reserved DEFAULT tenant is the nil UUID
-- 00000000-0000-0000-0000-000000000000: it owns every row created
-- while tenancy is OFF. A session that has NOT set `ferroehr.tenant_id` — every
-- connection today, every existing test, every ECC run — resolves to this
-- default tenant, so single-tenant behaviour is byte-identical and the RLS
-- policy is a true-check. A tenant-scoped request sets
-- `SET ferroehr.tenant_id = '<uuid>'` on its connection (the app pool's
-- before_acquire hook), and this function then returns that uuid, so:
--   * the tenant_id column DEFAULT auto-stamps new rows with the request's
--     tenant (no per-INSERT wiring anywhere in the service), and
--   * the RLS policy filters reads/writes to the request's tenant,
-- and the two always agree. IMMUTABLE is wrong here (the result depends on a
-- session GUC) — STABLE is correct and is legal both in a column DEFAULT and in
-- an RLS policy predicate.

CREATE FUNCTION current_tenant_id() RETURNS uuid
LANGUAGE sql STABLE PARALLEL SAFE AS $$
    SELECT COALESCE(
        NULLIF(current_setting('ferroehr.tenant_id', true), '')::uuid,
        '00000000-0000-0000-0000-000000000000'::uuid
    )
$$;

COMMENT ON FUNCTION current_tenant_id() IS 'The current request''s tenant id from the ferroehr.tenant_id session GUC, or the reserved default tenant (nil uuid) when unset. Read by both the tenant_id column DEFAULTs and the RLS policies on the ehr scoping tables, so single-tenant (GUC unset) is byte-identical to pre-tenancy behaviour.';
