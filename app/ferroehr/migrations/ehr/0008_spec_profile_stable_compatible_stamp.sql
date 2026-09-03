-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ehr schema: the commit-time spec-profile compatibility stamp on a stored
-- version, and its cold-tier mirror column.
--
-- No openEHR spec governs runtime specification-generation selection — our own
-- design/extension. The compatibility direction it relies on IS spec-governed:
-- the openEHR release strategy
-- (https://specifications.openehr.org/governance/release_strategy) makes a
-- minor release "significant additions that do not change the semantics of the
-- existing part of the release", so every object valid under a released
-- generation stays valid under a later development one — but not the reverse.
-- A deployment that moves from the development generation set to the released
-- one therefore has to know, per stored version, whether its body is readable
-- by the released generation's reader; this column is that answer, computed
-- once at commit instead of on every read.
--
--   TRUE  — the body was read by the released-generation reader at commit
--           (always true for a version committed under the `stable` profile,
--           where it holds by construction).
--   FALSE — the body uses surface only the development generation defines.
--   NULL  — not stamped at write: a row committed before this column existed,
--           or one written by a verbatim-replay path (EHR-Extract import,
--           archive load), which reproduces a foreign version's bytes rather
--           than accepting a new body. Such a row is assessed on the fly at
--           read, under the `stable` profile only.
--
-- Nullable with no backfill on purpose: NULL is the honest value for a row
-- nothing assessed, and inventing TRUE for the existing corpus would be a
-- claim this migration cannot support.
-- Runs with search_path = ehr, ext.

ALTER TABLE vo_version
    ADD COLUMN stable_compatible boolean;

COMMENT ON COLUMN vo_version.stable_compatible IS 'Whether this version''s stored body is readable by the RELEASED openEHR generation set (the `stable` spec_profile: RM 1.1.0 + BASE 1.2.0). TRUE = read by that generation''s reader at commit; FALSE = uses development-only surface; NULL = not stamped at write (pre-column row, or a verbatim-replay import/archive-load row), assessed on the fly at read. No openEHR spec governs runtime generation selection — our own design/extension.';

-- The cold mirror was created with `LIKE vo_version` in 0007, which is a
-- one-time copy: a column added to the primary table afterwards does NOT
-- appear there. It has to be added explicitly, in the same position (appended
-- last on both), because the archive move and restore are
-- `INSERT INTO … SELECT *` between the two relations and are therefore
-- column-order dependent.
ALTER TABLE cold.vo_version
    ADD COLUMN stable_compatible boolean;

COMMENT ON COLUMN cold.vo_version.stable_compatible IS 'Cold-tier mirror of vo_version.stable_compatible; carried across the archive move and restore verbatim, so archiving never changes a version''s profile compatibility.';

-- The `vo_version_all` union view expanded its `SELECT *` at creation time
-- (https://www.postgresql.org/docs/18/sql-createview.html), so it still names
-- the 0007 column list and must be rebuilt to carry the new column. Its
-- whole-repository readers (admin export, physical delete) are unaffected
-- either way, but a view that silently lacks a column of the table it claims
-- to union is a trap for the next reader.
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
