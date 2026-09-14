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
--
-- Both functions are SECURITY DEFINER over the migrator-owned table: the
-- reader runs inside every row policy for every role, and the split runtime
-- roles do not exist yet when this set runs on a fresh database (the
-- demographic set creates them), so the table itself is granted to nobody and
-- callers hold EXECUTE alone.
CREATE TABLE ext.posture (
    key        text        NOT NULL,
    value      text        NOT NULL,
    stamped_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_ext_posture PRIMARY KEY (key)
);
COMMENT ON TABLE ext.posture IS
    'Deployment posture the server stamps at boot and the ext functions read: tenancy = multi | single. Owned by the migrator, read and written through the definer functions only. Not tenant-scoped, no RLS.';

-- Evaluated only when the GUC is unset (COALESCE stops at the first non-null
-- argument), so a scoped statement never pays the table read.
CREATE FUNCTION ext.default_tenant_or_refuse() RETURNS uuid
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = ext, pg_catalog
AS $$
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

-- The stamp the server writes at boot. Executable by the runtime writers only:
-- ferroehr_app here, ferroehr_ehr in demographic/0007 once that set has
-- created it.
CREATE FUNCTION ext.stamp_posture(a_key text, a_value text) RETURNS void
LANGUAGE sql SECURITY DEFINER
SET search_path = ext, pg_catalog
AS $$
    INSERT INTO ext.posture (key, value) VALUES (a_key, a_value)
    ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, stamped_at = now()
$$;
COMMENT ON FUNCTION ext.stamp_posture(text, text) IS
    'Write one posture key; the server calls it at boot with the state the configuration declares.';
REVOKE ALL ON FUNCTION ext.stamp_posture(text, text) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT EXECUTE ON FUNCTION ext.stamp_posture(text, text) TO ferroehr_app;
    ELSE
        RAISE NOTICE 'skipping ext.stamp_posture grant (roles absent — see the role block NOTICE)';
    END IF;
END $$;
