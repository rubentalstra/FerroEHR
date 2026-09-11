-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- Tenant isolation for the demographic change-control records, which the
-- baseline's RLS loop also never reached.
--
-- Found by the catalog sweep added with `0004`, which asks which relations
-- carry a `tenant_id` and are not isolated rather than trusting a list. The
-- clinical loop (`ehr/0004_multitenancy.sql`) scopes `contribution` and
-- `audit`; the demographic baseline loop names only `vo_version`, `node`,
-- `item_tag` and `event_outbox`. The demographic relations were built with
-- `CREATE TABLE … LIKE`, which copies columns, defaults and constraints but
-- NOT row-level security, so both tables arrived carrying a `tenant_id` that
-- nothing enforced.
--
-- What was exposed is the same shape as `0004`: an unscoped read. These rows
-- are the change-control trail over the demographic domain — who committed a
-- party version, when and under which contribution — so cross-tenant
-- visibility here says which tenants hold records for whom, without the
-- records themselves. GDPR Art. 32(1)(a)
-- (https://eur-lex.europa.eu/eli/reg/2016/679/oj); no openEHR spec governs
-- database policies — our own design/extension.
--
-- FORCE, like every sibling, so the owner is covered too.

DO $$
DECLARE
    rel text;
BEGIN
    FOREACH rel IN ARRAY ARRAY['contribution', 'audit'] LOOP
        EXECUTE format('ALTER TABLE demographic.%I ENABLE ROW LEVEL SECURITY', rel);
        EXECUTE format('ALTER TABLE demographic.%I FORCE ROW LEVEL SECURITY', rel);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON demographic.%I '
            'USING (tenant_id = ext.current_tenant_id()) '
            'WITH CHECK (tenant_id = ext.current_tenant_id())',
            rel);
    END LOOP;
END $$;
