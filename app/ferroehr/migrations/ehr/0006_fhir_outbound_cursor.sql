-- SPDX-FileCopyrightText: FerroEHR contributors
-- SPDX-License-Identifier: MIT

-- ehr schema: the FHIR **outbound** emitter's delivery cursor (an extension —
-- no openEHR spec governs FHIR interop;
-- 4a — event-driven FHIR resource emission).
--
-- Append-only on the baseline (0001) + eventing (0002/0003) +
-- multi-tenancy (0004) + FHIR mapping store (0005). The outbound emitter is a
-- background task (wired like the E1 outbox drainer) that walks
-- committed `event_outbox` rows in `seq` order, loads each COMPOSITION whose
-- template matches an enabled `fhir_mapping`, reverse-maps it to a FHIR resource,
-- and publishes it to the broker. It cannot reuse `event_outbox.published_at`
-- (that column is the E1 drainer's own at-least-once watermark), so it tracks its
-- OWN monotonic high-water-mark of the last processed `seq` here.
--
-- Single-row table (the `only_row` CHECK pins it to one row): the emitter reads
-- `last_seq`, processes rows with `seq > last_seq`, and advances `last_seq` only
-- after every FHIR message for a row confirms — a crash re-processes from the
-- unadvanced cursor (at-least-once; downstream FHIR systems upsert by resource
-- id). Runs with search_path = ehr, ext.
--
-- NOT tenant-scoped / no RLS: this is an infrastructure watermark carrying no
-- tenant data (a single bigint), consulted by a background task that runs with no
-- tenant session context (mirroring the E1 drainer's read of `event_outbox`).

CREATE TABLE fhir_outbound_cursor (
    -- Pins the table to exactly one row (the global emitter watermark).
    only_row boolean NOT NULL DEFAULT true,
    -- The highest event_outbox.seq the outbound emitter has fully processed
    -- (published every matching FHIR resource for). Rows with seq > last_seq are
    -- the pending working set; 0 = nothing processed yet (process from the start).
    last_seq bigint NOT NULL DEFAULT 0,
    CONSTRAINT pk_fhir_outbound_cursor PRIMARY KEY (only_row),
    CONSTRAINT ck_fhir_outbound_cursor_only_row CHECK (only_row)
);

-- Seed the single row so the emitter always has a cursor to read/advance.
INSERT INTO fhir_outbound_cursor (only_row, last_seq) VALUES (true, 0);

COMMENT ON TABLE fhir_outbound_cursor IS 'FHIR outbound emitter delivery cursor: the single-row high-water-mark of the last event_outbox.seq the emitter has fully processed. Separate from event_outbox.published_at (the E1 drainer''s watermark) so the two consumers do not interfere. Infra watermark — not tenant-scoped, no RLS.';
COMMENT ON COLUMN fhir_outbound_cursor.last_seq IS 'Highest event_outbox.seq fully processed by the outbound emitter; rows with seq > last_seq are pending. Advanced only after all FHIR messages for a row confirm (at-least-once).';

-- ── Grants ──────────────────────────────────────────────────────
-- The baseline set ALTER DEFAULT PRIVILEGES for ferroehr_app/ferroehr_reader, so a
-- table the migrator creates afterwards is auto-granted; repeated explicitly
-- (role-guarded, like 0002/0003/0004/0005) so this migration is self-contained
-- and a no-op on the normal run order.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON fhir_outbound_cursor TO ferroehr_app;
        GRANT SELECT ON fhir_outbound_cursor TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping fhir_outbound_cursor grants (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
