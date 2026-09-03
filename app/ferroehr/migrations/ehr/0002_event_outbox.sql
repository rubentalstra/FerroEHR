-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ehr schema: the contribution-outbox eventing table.
--
-- Append-only on the baseline (0001). One transactional outbox row is
-- written in the SAME transaction as every CONTRIBUTION commit (vobject commit
-- path) — "no commit without its event; no event without its commit"
--. The row carries a PHI-FREE envelope (contribution id, ehr_id,
-- per-version (vo_id, kind, sys_version, change_type, template_id),
-- committed_at); clinical content is NEVER in the event. A
-- background publisher drains pending rows in (ehr_id, seq) order to the broker
-- with confirms, then stamps published_at; retention prunes published rows
-- after a configurable window. Follows the baseline discipline:
-- named constraints (pk_/fk_/idx_), COMMENT ON everything, role-guarded grants.
-- Runs with search_path = ehr, ext.

-- ── event_outbox ─────────────────────────────────────────────
CREATE TABLE event_outbox (
    -- Monotonic delivery sequence: the per-EHR ordering axis. A
    -- GENERATED IDENTITY (not uuidv7) so the drainer can ORDER BY it and a
    -- consumer can reason about ordering; bigint headroom for the outbox
    -- lifetime (rows are pruned, so it never approaches the ceiling).
    seq             bigint GENERATED ALWAYS AS IDENTITY,
    -- The committed CONTRIBUTION this event announces. FK CASCADE keeps the
    -- outbox consistent if a contribution is ever removed; the outbox is not an
    -- audit record — contribution/audit remain the system of record.
    contribution_id uuid NOT NULL,
    -- Owning EHR, or NULL for a demographic (party) contribution — mirrors
    -- contribution.ehr_id. The per-EHR ordering group.
    ehr_id          uuid,
    -- The PHI-FREE event payload: contribution id, ehr_id,
    -- committed_at, and the per-version array of
    -- {vo_id, kind, sys_version, change_type, template_id}. NO clinical content
    -- — consumers fetch the bodies through the authenticated REST/native API.
    envelope        jsonb NOT NULL,
    -- The contribution's commit instant (its audit time_committed), copied so a
    -- consumer/pruner needs no join back to audit.
    committed_at    timestamptz NOT NULL,
    -- NULL = pending (not yet confirmed by the broker); set to the publish
    -- instant once the broker confirm lands (at-least-once delivery — eventing is an extension; no openEHR spec governs it).
    published_at    timestamptz,
    CONSTRAINT pk_event_outbox PRIMARY KEY (seq),
    CONSTRAINT fk_event_outbox_contribution FOREIGN KEY (contribution_id)
        REFERENCES contribution (id) ON DELETE CASCADE
);

-- The drainer's working set: pending rows only, ordered for per-EHR delivery
--. Partial (published_at IS NULL) so it stays tiny as rows drain,
-- and a covering (ehr_id, seq) key so the ordered SELECT ... FOR UPDATE SKIP
-- LOCKED reads straight off the index.
CREATE INDEX idx_event_outbox_pending ON event_outbox (ehr_id, seq)
    WHERE published_at IS NULL;
-- Retention pruning scans published rows by age.
CREATE INDEX idx_event_outbox_published ON event_outbox (published_at)
    WHERE published_at IS NOT NULL;

COMMENT ON TABLE event_outbox IS 'Contribution-outbox eventing: one PHI-free event row per CONTRIBUTION commit, written in the same transaction; drained to the broker at-least-once in (ehr_id, seq) order, then pruned by retention. Not an audit record (§6).';
COMMENT ON COLUMN event_outbox.seq IS 'Monotonic delivery sequence; the per-EHR ordering axis.';
COMMENT ON COLUMN event_outbox.envelope IS 'PHI-free event payload: contribution id, ehr_id, committed_at, per-version (vo_id, kind, sys_version, change_type, template_id). No clinical content.';
COMMENT ON COLUMN event_outbox.published_at IS 'NULL = pending; the publish/confirm instant once the broker acknowledges (at-least-once delivery; eventing is an extension — no openEHR spec governs it).';
COMMENT ON COLUMN event_outbox.committed_at IS 'The contribution commit instant (its audit time_committed), copied to avoid a join back to audit.';

-- ── Grants ──────────────────────────────────────────────────────
-- The baseline set ALTER DEFAULT PRIVILEGES for ferroehr_app/ferroehr_reader, so
-- a table the migrator creates afterwards is auto-granted; repeated explicitly
-- (role-guarded, like the baseline) so this migration is self-contained and a
-- no-op on the normal run order.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON event_outbox TO ferroehr_app;
        GRANT SELECT ON event_outbox TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping event_outbox grants (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
