-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- The outbox reader registry for the demographic domain (#3330), the twin of
-- ehr/0012. The AMQP drainer prunes both outboxes with one statement whose
-- floor is the lowest active reader cursor; the demographic outbox has no
-- cursor reader today, so its registry starts empty and the prune deletes on
-- age alone until one registers.
--
-- No openEHR spec governs eventing or storage: our own design/extension.
CREATE TABLE demographic.event_outbox_reader (
    reader     text        NOT NULL,
    last_seq   bigint      NOT NULL DEFAULT 0,
    active     boolean     NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_dem_event_outbox_reader PRIMARY KEY (reader),
    CONSTRAINT ck_dem_event_outbox_reader_name CHECK (reader ~ '^[a-z][a-z0-9-]{0,62}$'),
    CONSTRAINT ck_dem_event_outbox_reader_seq CHECK (last_seq >= 0)
);

COMMENT ON TABLE demographic.event_outbox_reader IS
    'Cursor readers over demographic.event_outbox: one row per reader with its high-water mark and whether the deployment runs it. The retention prune deletes a published row only at or below the lowest active last_seq. Infra watermark, not tenant-scoped, no RLS.';

-- The baseline's default privileges cover ferroehr_app / ferroehr_reader; the
-- split runtime roles read the floor and the demographic writer advances it.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_demographic') THEN
        GRANT SELECT, INSERT, UPDATE ON demographic.event_outbox_reader TO ferroehr_demographic;
        GRANT SELECT ON demographic.event_outbox_reader TO ferroehr_demographic_reader;
    ELSE
        RAISE NOTICE 'skipping demographic.event_outbox_reader grants for the split roles (roles absent)';
    END IF;
END $$;
