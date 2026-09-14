-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- The tenancy posture, readable by every runtime role (#3355).
--
-- The outbox readers drain one registered tenant at a time under that
-- tenant's scope. That only makes sense under the multi posture: with
-- tenancy off the pools never stamp a request's tenant, so a pass "for"
-- another tenant would write under the default tenant's session and be
-- refused by the row policy (seen at boot in the deployment probe). The
-- readers therefore ask the posture, through a definer over the
-- migrator-owned table like the reader in ext/0003, and list the default
-- tenant alone under `single`.
--
-- No openEHR spec governs tenancy: our own design/extension.
CREATE FUNCTION ext.tenancy_posture() RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = ext, pg_catalog
AS $$
    SELECT COALESCE((SELECT value FROM ext.posture WHERE key = 'tenancy'), 'single')
$$;
COMMENT ON FUNCTION ext.tenancy_posture() IS
    'The stamped tenancy posture: multi or single (single when nothing is stamped).';
