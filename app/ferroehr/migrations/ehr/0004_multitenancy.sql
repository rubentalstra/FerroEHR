-- SPDX-FileCopyrightText: FerroEHR contributors
-- SPDX-License-Identifier: MIT

-- ehr schema: multi-tenancy — tenant registry + tenant_id scoping + RLS FORCE
-- (an extension — no openEHR spec governs multi-tenancy; E2).
--
-- Append-only on the baseline (0001) + eventing (0002/0003). Adds:
--   * `tenant` — the registry: one row per logical openEHR system (its own
--     system_id), seeded with the reserved DEFAULT tenant;
--   * `tenant_id` (uuid, NOT NULL, FK → tenant) on the 17 scoping tables, its
--     DEFAULT reading ext.current_tenant_id() so a write auto-stamps the
--     request's tenant (unset session ⇒ the reserved default → today's rows);
--   * ENABLE + FORCE ROW LEVEL SECURITY + a `tenant_isolation` policy on each,
--     USING/WITH CHECK `tenant_id = ext.current_tenant_id()`.
-- Runs with search_path = ehr, ext.
--
-- Single-tenant default, ZERO behaviour change: tenancy is OFF by
-- default; no session sets ferroehr.tenant_id, so every row is the reserved
-- default tenant and the policy is a true-check. Note that the existing test /
-- ECC suites connect as the postgres SUPERUSER, which BYPASSES RLS entirely
-- (superuser/BYPASSRLS bypass is unconditional and separate from FORCE) — so
-- they are unaffected; engine-level isolation is proven separately by the E2
-- integration test connecting as a non-superuser role member of ferroehr_app.
--
-- Baseline discipline: named constraints (pk_/uq_/fk_/ck_), COMMENT ON
-- everything, role-guarded grants.
--
-- INDEXING ("measured at P20"): no dedicated tenant_id
-- indexes are created here. RLS appends a constant-equality predicate
-- `tenant_id = ext.current_tenant_id()` to every statement; within a tenant the
-- existing selective indexes (the uuidv7 PKs on vo_id/id, the composite keys)
-- stay selective, and in single-tenant mode a tenant_id index is non-selective
-- (one tenant). A leading-tenant_id composite index on the hot read paths is a
-- multi-tenant-only perf concern deferred to P20 (RLS cost "paid only in
-- multi-tenant mode").
--
-- NOTE: the pre-existing service-wide UNIQUE
-- constraints (uq_ehr_subject, uq_template_store_template_id, pk_template_ref
-- (the template registry mirrors the global template_store/adl2_artefact keys),
-- pk_stored_query, uq_event_subscription_name, uq_sp_data_frame_frame_id,
-- and uq_item_tag_identity — practically unreachable cross-tenant since a
-- target_vo_id belongs to one tenant, listed for completeness)
-- remain GLOBAL, not tenant-scoped, in this pass. RLS isolates row visibility
-- (engine-enforced isolation); making those keys
-- per-tenant so two tenants may independently reuse a name is a separate,
-- documented follow-up (a cross-tenant name reuse surfaces as a constraint
-- violation rather than being allowed). The E2 isolation test avoids subjects /
-- duplicate names, so it is unaffected.

-- ── tenant ──────────────────────────────────────────────────────
CREATE TABLE tenant (
    id         uuid NOT NULL DEFAULT uuidv7(),
    name       text NOT NULL,
    -- The tenant's own logical-system id: per-tenant version
    -- identity / audits / EHR.system_id when tenancy is ON. Distinct per tenant.
    system_id  text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_tenant PRIMARY KEY (id),
    CONSTRAINT uq_tenant_name UNIQUE (name),
    -- A logical system id is 1..1, non-void (mirrors ehr/audit system_id).
    CONSTRAINT ck_tenant_system_id_nonempty CHECK (system_id <> '')
);

COMMENT ON TABLE tenant IS 'Tenant registry (an extension — no openEHR spec governs multi-tenancy): one row per logical openEHR system (its own system_id). NOT itself RLS-scoped — it is the registry the tenant-resolution middleware resolves a claim/header against. The reserved default tenant (nil uuid) owns all rows when tenancy is OFF.';
COMMENT ON COLUMN tenant.system_id IS 'The tenant''s own logical-system id: stamped into version ids / audits / EHR.system_id for this tenant when tenancy is ON.';

-- The reserved DEFAULT tenant: fixed nil uuid, owns every row
-- created with tenancy OFF (the ext.current_tenant_id() fallback). Seeded before
-- the ALTER loop so the FK backfill of any existing rows resolves. Its system_id
-- is the documented pre-tenancy default (service::DEFAULT_SYSTEM_ID =
-- 'ferroehr.local'); with tenancy OFF the service uses its own configured
-- system_id and never reads this row — it is consulted only when tenancy is ON,
-- so seeding the pre-tenancy default keeps the record self-describing.
INSERT INTO tenant (id, name, system_id)
VALUES ('00000000-0000-0000-0000-000000000000', 'default', 'ferroehr.local');

-- ── tenant_id scoping + RLS FORCE ───────────────────────────────
-- One uniform pass over the scoping tables: add the FK column (auto-stamping
-- DEFAULT), ENABLE + FORCE RLS, and the tenant_isolation policy. A loop keeps
-- the 17 tables in lockstep (deterministic named constraint fk_<table>_tenant,
-- one named policy tenant_isolation) far more safely than 17× hand-repetition.
DO $$
DECLARE
    t text;
    scoped_tables text[] := ARRAY[
        'ehr', 'contribution', 'vo_version', 'node', 'item_tag', 'audit',
        'template_store', 'archetype_store', 'adl2_artefact', 'stored_query',
        'sp_subject', 'sp_binding', 'sp_data_frame', 'sp_variable', 'sp_data_set',
        'event_outbox', 'event_subscription'
    ];
BEGIN
    FOREACH t IN ARRAY scoped_tables LOOP
        EXECUTE format(
            'ALTER TABLE %I ADD COLUMN tenant_id uuid NOT NULL '
            'DEFAULT ext.current_tenant_id() '
            'CONSTRAINT fk_%I_tenant REFERENCES tenant (id)', t, t);
        EXECUTE format('COMMENT ON COLUMN %I.tenant_id IS %L', t,
            'Owning tenant (FK → tenant; multi-tenancy is an extension). DEFAULT ext.current_tenant_id() '
            'auto-stamps the request''s tenant; an unset session ⇒ the reserved default '
            'tenant. RLS-enforced (tenant_isolation policy).');
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        -- FORCE so the table OWNER is filtered too; superusers /
        -- BYPASSRLS roles still bypass unconditionally (a Postgres invariant).
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON %I '
            'USING (tenant_id = ext.current_tenant_id()) '
            'WITH CHECK (tenant_id = ext.current_tenant_id())', t);
    END LOOP;
END $$;

-- ── Grants ──────────────────────────────────────────────────────
-- The scoping tables were granted by the baseline (their new tenant_id column
-- inherits the table grant). Only the new `tenant` registry needs an explicit,
-- role-guarded grant (a no-op on the normal order via ALTER DEFAULT PRIVILEGES,
-- repeated for a self-contained migration, like 0002/0003).
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON tenant TO ferroehr_app;
        GRANT SELECT ON tenant TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping tenant grants (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
