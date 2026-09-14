-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- The clinical runtime role stamps the deployment posture at boot (#3341).
--
-- `ext.stamp_posture` is granted to `ferroehr_app` by ext/0003; the split
-- writer `ferroehr_ehr` is created by demographic/0001, which runs after the
-- ext set on a fresh database, so its grant lives here, after the role exists.
-- The read side needs no grant: `ext.default_tenant_or_refuse()` is a
-- SECURITY DEFINER over the migrator-owned table and is executable by every
-- role whose row policies call the tenant reader.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        GRANT EXECUTE ON FUNCTION ext.stamp_posture(text, text) TO ferroehr_ehr;
    ELSE
        RAISE NOTICE 'skipping ext.stamp_posture grant for ferroehr_ehr (role absent)';
    END IF;
END $$;
