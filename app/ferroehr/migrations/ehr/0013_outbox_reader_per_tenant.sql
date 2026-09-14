-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- Per-tenant reader cursors (#3355).
--
-- `event_outbox` is tenant-scoped by row policy, so a cursor reader drains one
-- tenant's rows at a time under that tenant's scope, and its high-water mark
-- is per tenant: one global mark advanced in one tenant's pass would skip the
-- other tenants' lower sequence numbers. The rows this table holds today
-- belong to the reserved default tenant, which is what every reader drained
-- until now; the migrator declares that tenant (#3341), so the column DEFAULT
-- back-fills them. The table is tenant-scoped like every other: the boot
-- reconciliation of `active` runs once per registered tenant, in its scope.
--
-- No openEHR spec governs eventing or tenancy: our own design/extension.
ALTER TABLE event_outbox_reader
    ADD COLUMN tenant_id uuid NOT NULL DEFAULT ext.current_tenant_id();
ALTER TABLE event_outbox_reader DROP CONSTRAINT pk_event_outbox_reader;
ALTER TABLE event_outbox_reader
    ADD CONSTRAINT pk_event_outbox_reader PRIMARY KEY (reader, tenant_id);
COMMENT ON COLUMN event_outbox_reader.tenant_id IS
    'The tenant whose rows this cursor covers; the reserved default tenant in a single-tenant deployment. RLS-enforced (tenant_isolation policy).';

ALTER TABLE event_outbox_reader ENABLE ROW LEVEL SECURITY;
ALTER TABLE event_outbox_reader FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON event_outbox_reader
    USING (tenant_id = ext.current_tenant_id())
    WITH CHECK (tenant_id = ext.current_tenant_id());
