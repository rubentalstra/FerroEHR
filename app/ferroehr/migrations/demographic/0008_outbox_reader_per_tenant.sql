-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- Per-tenant reader cursors for the demographic domain (#3355), the twin of
-- ehr/0013: a cursor reader drains one tenant at a time, so its mark is per
-- tenant. The table stays outside row-level security for the same reason as
-- its clinical twin.
--
-- No openEHR spec governs eventing or tenancy: our own design/extension.
ALTER TABLE demographic.event_outbox_reader
    ADD COLUMN tenant_id uuid NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE demographic.event_outbox_reader DROP CONSTRAINT pk_dem_event_outbox_reader;
ALTER TABLE demographic.event_outbox_reader
    ADD CONSTRAINT pk_dem_event_outbox_reader PRIMARY KEY (reader, tenant_id);
ALTER TABLE demographic.event_outbox_reader ALTER COLUMN tenant_id DROP DEFAULT;
COMMENT ON COLUMN demographic.event_outbox_reader.tenant_id IS
    'The tenant whose rows this cursor covers; the reserved default tenant in a single-tenant deployment.';
