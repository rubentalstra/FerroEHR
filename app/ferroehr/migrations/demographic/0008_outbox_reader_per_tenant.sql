-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- Per-tenant reader cursors for the demographic domain (#3355), the twin of
-- ehr/0013: a cursor reader drains one tenant at a time, so its mark is per
-- tenant, and the table is tenant-scoped like every other.
--
-- No openEHR spec governs eventing or tenancy: our own design/extension.
ALTER TABLE demographic.event_outbox_reader
    ADD COLUMN tenant_id uuid NOT NULL DEFAULT ext.current_tenant_id();
ALTER TABLE demographic.event_outbox_reader DROP CONSTRAINT pk_dem_event_outbox_reader;
ALTER TABLE demographic.event_outbox_reader
    ADD CONSTRAINT pk_dem_event_outbox_reader PRIMARY KEY (reader, tenant_id);
COMMENT ON COLUMN demographic.event_outbox_reader.tenant_id IS
    'The tenant whose rows this cursor covers; the reserved default tenant in a single-tenant deployment. RLS-enforced (tenant_isolation policy).';

ALTER TABLE demographic.event_outbox_reader ENABLE ROW LEVEL SECURITY;
ALTER TABLE demographic.event_outbox_reader FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON demographic.event_outbox_reader
    USING (tenant_id = ext.current_tenant_id())
    WITH CHECK (tenant_id = ext.current_tenant_id());
