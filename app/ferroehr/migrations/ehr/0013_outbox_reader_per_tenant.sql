-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- Per-tenant reader cursors (#3355).
--
-- `event_outbox` is tenant-scoped by row policy, so a cursor reader drains one
-- tenant's rows at a time under that tenant's scope, and its high-water mark
-- is per tenant: one global mark advanced in one tenant's pass would skip the
-- other tenants' lower sequence numbers. The rows this table holds today
-- belong to the reserved default tenant, which is what every reader drained
-- until now. The table stays outside row-level security: the boot
-- reconciliation of `active` spans every tenant's row of a reader.
--
-- No openEHR spec governs eventing or tenancy: our own design/extension.
ALTER TABLE event_outbox_reader
    ADD COLUMN tenant_id uuid NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE event_outbox_reader DROP CONSTRAINT pk_event_outbox_reader;
ALTER TABLE event_outbox_reader
    ADD CONSTRAINT pk_event_outbox_reader PRIMARY KEY (reader, tenant_id);
-- The code names the tenant on every write; the default served the back-fill.
ALTER TABLE event_outbox_reader ALTER COLUMN tenant_id DROP DEFAULT;
COMMENT ON COLUMN event_outbox_reader.tenant_id IS
    'The tenant whose rows this cursor covers; the reserved default tenant in a single-tenant deployment.';
