-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- party: the domain's own transactional change-event outbox.
--
-- The clinical outbox cannot serve it: that table's foreign key points at the
-- clinical contribution relation, and a party commit does not live there — so
-- a party commit would either violate that key or leave the domain, which is
-- what the separation removes. Written in the same transaction as the commit
-- it announces, exactly as the clinical one is; the envelope is PHI-free by
-- construction (the contribution id, a NULL EHR id, the per-version kind and
-- change type), so pseudonymisation is not weakened by announcing that a party
-- changed.
--
-- No openEHR spec governs eventing: our own extension.
--
-- Runs with search_path = party, ext, public.
CREATE TABLE event_outbox (
    seq             bigint GENERATED ALWAYS AS IDENTITY,
    contribution_id uuid NOT NULL,
    -- Always NULL here (a party has no owning EHR); kept so the envelope and
    -- the drainer read the same column set in both domains.
    ehr_id          uuid,
    envelope        jsonb NOT NULL,
    committed_at    timestamptz NOT NULL,
    published_at    timestamptz,
    CONSTRAINT pk_event_outbox PRIMARY KEY (seq),
    CONSTRAINT fk_event_outbox_contribution FOREIGN KEY (contribution_id)
        REFERENCES contribution (id) ON DELETE CASCADE
);

CREATE INDEX idx_event_outbox_pending ON event_outbox (seq)
    WHERE published_at IS NULL;
CREATE INDEX idx_event_outbox_published ON event_outbox (published_at)
    WHERE published_at IS NOT NULL;

COMMENT ON TABLE event_outbox IS 'Contribution-outbox eventing for the party domain: one PHI-free event row per party CONTRIBUTION commit, written in the same transaction and drained by the same publisher as the clinical outbox. Separate because its foreign key must stay inside this domain.';

CREATE TABLE event_outbox_reader (
    reader     text        NOT NULL,
    last_seq   bigint      NOT NULL DEFAULT 0,
    active     boolean     NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_event_outbox_reader PRIMARY KEY (reader),
    CONSTRAINT ck_event_outbox_reader_name CHECK (reader ~ '^[a-z][a-z0-9-]{0,62}$'),
    CONSTRAINT ck_event_outbox_reader_seq CHECK (last_seq >= 0)
);

COMMENT ON TABLE event_outbox_reader IS 'Cursor readers over the party outbox: one row per reader with its high-water mark and whether the deployment runs it.';
