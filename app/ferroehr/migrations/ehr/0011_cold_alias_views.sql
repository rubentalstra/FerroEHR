-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The clinical cold-tier alias views, owned by the set whose schema they live
-- in (#3298).
--
-- `ehr.cold_vo_version`, `ehr.cold_node` and `ehr.cold_vo_attestation` are the
-- unqualified names the storage layer freezes, thaws and purges through
-- (`storage/version_repo/tier.rs`). The demographic baseline created them
-- beside their `demographic.*` twins, which put three `ehr` objects under the
-- demographic set's bookkeeping: a database whose `ehr` schema was dropped and
-- rebuilt (the sandbox reseed's wipe) came back without them, because the
-- demographic set had nothing left to run, and every versioned-object update
-- failed on the placement read with 42P01. The views are also `SELECT *`,
-- which PostgreSQL expands at creation, so a view created before `0010` added
-- `vo_version.origins` lacks that column and the freeze's `INSERT … SELECT *`
-- fails on it.
--
-- CREATE OR REPLACE repairs both: it creates a missing view and widens an
-- existing one (a replacement may append columns; PostgreSQL 18, CREATE VIEW,
-- https://www.postgresql.org/docs/18/sql-createview.html). The one path it
-- must yield on is a fresh database: the clinical set runs before the
-- demographic set, whose shipped baseline still issues a plain CREATE for
-- these views and cannot change, so this migration acts only once the
-- demographic baseline has run (its `demographic.vo_version` exists), which is
-- exactly the repair and widening case. security_invoker is required, not
-- decorative: a view runs with its owner's rights by default, which would let
-- a caller read past the tenant policy on the mirror table. No openEHR spec
-- governs storage tiering — our own design/extension. Runs with
-- search_path = ehr, ext.

DO $$
BEGIN
    IF to_regclass('demographic.vo_version') IS NULL THEN
        RAISE NOTICE 'cold alias views: the demographic baseline has not run yet and creates them itself';
        RETURN;
    END IF;

    CREATE OR REPLACE VIEW ehr.cold_vo_version WITH (security_invoker = true) AS
        SELECT * FROM cold.vo_version;
    CREATE OR REPLACE VIEW ehr.cold_node WITH (security_invoker = true) AS
        SELECT * FROM cold.node;
    CREATE OR REPLACE VIEW ehr.cold_vo_attestation WITH (security_invoker = true) AS
        SELECT * FROM cold.vo_attestation;

    COMMENT ON VIEW ehr.cold_vo_version IS 'The clinical cold tier under the name the storage layer uses unqualified, so one statement serves either pseudonymisation domain by search_path alone.';
    COMMENT ON VIEW ehr.cold_node IS 'The clinical cold tier of node under its unqualified alias; see cold_vo_version.';
    COMMENT ON VIEW ehr.cold_vo_attestation IS 'The clinical cold tier of vo_attestation under its unqualified alias; see cold_vo_version.';

    -- The same privileges the primary relations carry, for whichever role set
    -- the database provisioned; an absent role is skipped as the baseline does.
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON ehr.cold_vo_version, ehr.cold_node, ehr.cold_vo_attestation TO ferroehr_app;
        GRANT SELECT ON ehr.cold_vo_version, ehr.cold_node, ehr.cold_vo_attestation TO ferroehr_reader;
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON ehr.cold_vo_version, ehr.cold_node, ehr.cold_vo_attestation TO ferroehr_ehr;
        GRANT SELECT ON ehr.cold_vo_version, ehr.cold_node, ehr.cold_vo_attestation TO ferroehr_ehr_reader;
    END IF;
END $$;
