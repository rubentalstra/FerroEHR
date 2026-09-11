-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- Tenant isolation for the sealed identifiers, which the baseline's RLS loop
-- never reached.
--
-- `0001_baseline.sql` enables and FORCEs row-level security on every relation
-- it creates and gives each one the same `tenant_isolation` policy.
-- `demographic.national_identifier` was created afterwards, by
-- `0003_national_identifier.sql`, so the loop had already run and the table
-- joined its siblings in every respect but this one. The omission is not
-- visible from the table's own definition, which is why it survived: the
-- tenant column, its DEFAULT and its uniqueness are all there.
--
-- What was exposed is the unscoped read. `uq_national_identifier_value` is
-- tenant-scoped, so an equality lookup by digest could never return another
-- tenant's party, and the resolve function goes through it. But a bare
-- `SELECT` by the demographic role returned every tenant's rows, on the table
-- holding sealed national identifiers and the keyed digests that resolve
-- them. GDPR Art. 32(1)(a) names the measures
-- (https://eur-lex.europa.eu/eli/reg/2016/679/oj); no openEHR spec governs
-- database policies — our own design/extension.
--
-- FORCE, like the siblings, so the policy applies to the table owner too. That
-- is safe for `demographic.resolve_national_identifier` even though it is
-- SECURITY DEFINER and therefore runs as that owner: every caller derives its
-- tenant from the same task-local context that stamps the
-- `ferroehr.tenant_id` GUC, so the function's `WHERE tenant_id = p_tenant`
-- and this policy's `ext.current_tenant_id()` are the same value by
-- construction. A caller that ever passed a different tenant would now be
-- refused by the database rather than trusted, which is the point.
--
-- `demographic.identifier_scheme` deliberately gets NO policy: it is the
-- global registry of which identifier kinds this deployment may store, it
-- carries no tenant column, and every tenant reads the same rows.

ALTER TABLE demographic.national_identifier ENABLE ROW LEVEL SECURITY;
ALTER TABLE demographic.national_identifier FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON demographic.national_identifier
    USING (tenant_id = ext.current_tenant_id())
    WITH CHECK (tenant_id = ext.current_tenant_id());
