-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The origin set of a version body, stamped at commit (#3212).
--
-- EHDS Annex II 3.2(e) asks the logging component to record "the origin or
-- origins of data" an access served (Regulation (EU) 2025/327,
-- https://eur-lex.europa.eu/eli/reg/2025/327/oj). openEHR models that
-- provenance on the content itself: FEEDER_AUDIT.originating_system_audit
-- (1..1) names "the IT system owned by the organisation legally responsible
-- for handling the data, and at which the data were previously created"
-- (RM common, FEEDER_AUDIT_DETAILS.system_id), and a FEEDER_AUDIT may sit on
-- any LOCATABLE, so one body can carry several. A body carrying none was
-- created here, through this API, and its origin is this server's system id.
--
-- The set is derived ONCE, when the body is accepted, and stored as a JSON
-- array of system ids; a read aggregates over the rows it touched instead of
-- parsing bodies on the serving path — the `stable_compatible` stamp's own
-- pattern (0008). The values are the committer's claims about provenance, as
-- FEEDER_AUDIT is, and are recorded as such.
--
-- Nullable with no backfill: NULL is the honest value for a row nothing
-- stamped (a pre-column row, or a verbatim-replay archive load), which a read
-- assesses on the fly from the body it has in hand, or reports as the
-- creating system where it has none. Appended LAST on both tiers: the archive
-- move and restore are `INSERT INTO … SELECT *` and column-order dependent
-- (0007, 0008). Runs with search_path = ehr, ext.

ALTER TABLE vo_version
    ADD COLUMN origins jsonb;

COMMENT ON COLUMN vo_version.origins IS 'The distinct originating systems of this version''s body, as a JSON array of FEEDER_AUDIT.originating_system_audit.system_id values found anywhere in it, or this server''s own system id when it carries none; NULL when nothing stamped it (pre-column row, verbatim archive load) — assessed at read. EHDS Annex II 3.2(e); our own design/extension.';

ALTER TABLE cold.vo_version
    ADD COLUMN origins jsonb;

COMMENT ON COLUMN cold.vo_version.origins IS 'Cold-tier mirror of vo_version.origins; carried across the archive move and restore verbatim.';

DROP VIEW vo_version_all;

CREATE VIEW vo_version_all WITH (security_invoker = true) AS
    SELECT * FROM vo_version
    UNION ALL
    SELECT * FROM cold.vo_version;

COMMENT ON VIEW vo_version_all IS 'Both storage tiers of vo_version (primary UNION ALL cold). For whole-repository readers only (admin export, physical delete); serving reads consult the cold tier on a primary miss instead, so the hot path never scans it.';

DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT ON vo_version_all TO ferroehr_app, ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping vo_version_all grants (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
