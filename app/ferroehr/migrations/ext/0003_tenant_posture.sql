-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- The tenancy posture and the fail-closed tenant reader (#3341).
--
-- `current_tenant_id()` resolved an unset or empty `ferroehr.tenant_id` to the
-- reserved default tenant, so a connection that never declared a tenant read
-- and wrote the default tenant's rows under every row policy instead of
-- failing. A single-tenant deployment wants exactly that: the GUC is never set
-- and the default tenant owns everything. A multi-tenant deployment does not:
-- there an undeclared tenant is a bypass, not a request.
--
-- The server stamps the posture into `ext.posture` at boot from
-- `tenancy.enabled`, declares the default tenant explicitly on every connection
-- it opens (the pools, the migrator), and the reader below raises for an unset
-- GUC only under the multi posture, so the refusal reaches exactly the
-- connections that bypass the server. No openEHR spec governs tenancy: our
-- own design/extension.
CREATE TABLE ext.posture (
    key        text        NOT NULL,
    value      text        NOT NULL,
    stamped_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_ext_posture PRIMARY KEY (key)
);
COMMENT ON TABLE ext.posture IS
    'Deployment posture the server stamps at boot and the ext functions read: tenancy = multi | single. Not tenant-scoped, no RLS.';

-- Evaluated only when the GUC is unset (COALESCE stops at the first non-null
-- argument), so a scoped statement never pays the table read.
CREATE FUNCTION ext.default_tenant_or_refuse() RETURNS uuid
LANGUAGE plpgsql STABLE AS $$
DECLARE
    posture text;
BEGIN
    SELECT value INTO posture FROM ext.posture WHERE key = 'tenancy';
    IF posture = 'multi' THEN
        RAISE EXCEPTION USING
            ERRCODE = 'insufficient_privilege',
            MESSAGE = 'ferroehr.tenant_id is not set on this connection and the deployment is multi-tenant; declare the tenant before reading or writing tenant-scoped rows',
            HINT = 'the server sets it on every connection it opens; a session connecting on its own must SET ferroehr.tenant_id (the reserved default tenant is 00000000-0000-0000-0000-000000000000)';
    END IF;
    RETURN '00000000-0000-0000-0000-000000000000'::uuid;
END $$;
COMMENT ON FUNCTION ext.default_tenant_or_refuse() IS
    'The tenant an undeclared connection gets: the reserved default under the single posture, a refusal under multi.';

CREATE OR REPLACE FUNCTION current_tenant_id() RETURNS uuid
LANGUAGE sql STABLE PARALLEL SAFE AS $$
    SELECT COALESCE(
        NULLIF(current_setting('ferroehr.tenant_id', true), '')::uuid,
        ext.default_tenant_or_refuse()
    )
$$;
COMMENT ON FUNCTION current_tenant_id() IS
    'The current request''s tenant id from the ferroehr.tenant_id session GUC. Unset: the reserved default tenant (nil uuid) under the single posture, a refusal under the multi posture (ext.posture, key tenancy). Read by the tenant_id column DEFAULTs and the RLS policies.';

-- Every runtime role reads the posture (its row policies call the reader);
-- the clinical runtime stamps it. Guarded like every role block in the tree.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT, INSERT, UPDATE ON ext.posture TO ferroehr_app;
        GRANT SELECT ON ext.posture TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping ext.posture grants (roles absent — see the role block NOTICE)';
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        GRANT SELECT, INSERT, UPDATE ON ext.posture TO ferroehr_ehr;
        GRANT SELECT ON ext.posture
            TO ferroehr_ehr_reader, ferroehr_demographic, ferroehr_demographic_reader;
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_linkage') THEN
        GRANT SELECT ON ext.posture TO ferroehr_linkage;
    END IF;
END $$;
