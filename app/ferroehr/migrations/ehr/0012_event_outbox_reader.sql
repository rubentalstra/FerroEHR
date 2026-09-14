-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- The outbox reader registry (#3330).
--
-- `event_outbox` has more than one consumer: the AMQP drainer, which stamps
-- `published_at`, and every cursor reader that keeps its own high-water mark
-- (the FHIR outbound emitter today, a read-model projector next). The
-- retention prune deleted on `published_at` alone, so a row a cursor reader
-- had not reached (a parked poison row, a long outage of its target, a paused
-- emitter) was gone once the drainer's stamp aged past the window, and the
-- reader then advanced over a gap it never saw.
--
-- Every cursor reader now owns one row here, and the prune's floor is the
-- lowest `last_seq` among the ACTIVE readers, read in the same DELETE
-- statement. `active` is reconciled from the configuration at boot: an
-- enabled reader is active, a disabled one is not, so a reader that was
-- switched off cannot hold the outbox forever; a reader that advances its
-- cursor registers itself active by that act.
--
-- No openEHR spec governs eventing or storage: our own design/extension.
CREATE TABLE event_outbox_reader (
    reader     text        NOT NULL,
    last_seq   bigint      NOT NULL DEFAULT 0,
    active     boolean     NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_event_outbox_reader PRIMARY KEY (reader),
    CONSTRAINT ck_event_outbox_reader_name CHECK (reader ~ '^[a-z][a-z0-9-]{0,62}$'),
    CONSTRAINT ck_event_outbox_reader_seq CHECK (last_seq >= 0)
);

COMMENT ON TABLE event_outbox_reader IS
    'Cursor readers over event_outbox: one row per reader with its high-water mark and whether the deployment runs it. The retention prune deletes a published row only at or below the lowest active last_seq. Infra watermark, not tenant-scoped, no RLS.';
COMMENT ON COLUMN event_outbox_reader.reader IS
    'The reader''s registry key (fhir-outbound is the FHIR outbound emitter).';
COMMENT ON COLUMN event_outbox_reader.last_seq IS
    'Highest event_outbox.seq the reader has fully processed; rows above it are pending for that reader. Advanced monotonically, never moved back.';
COMMENT ON COLUMN event_outbox_reader.active IS
    'Whether the deployment runs this reader. Reconciled from the configuration at boot; only active readers hold the prune floor.';

-- The FHIR outbound emitter's singleton cursor moves here with its watermark
-- intact, inactive until the server reconciles it from the configuration.
INSERT INTO event_outbox_reader (reader, last_seq, active)
SELECT 'fhir-outbound', last_seq, false FROM fhir_outbound_cursor;

DROP TABLE fhir_outbound_cursor;

-- The baseline's default privileges cover ferroehr_app / ferroehr_reader; the
-- split runtime roles read the floor and the clinical writer advances it.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        GRANT SELECT, INSERT, UPDATE ON event_outbox_reader TO ferroehr_ehr;
        GRANT SELECT ON event_outbox_reader TO ferroehr_ehr_reader;
    ELSE
        RAISE NOTICE 'skipping event_outbox_reader grants for the split roles (roles absent)';
    END IF;
END $$;
