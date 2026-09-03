-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ehr schema: the COLD archival storage tier realizing SM I_ADMIN_ARCHIVE's
-- "Move selected EHRs to archival storage" as an actual physical move.
--
-- No openEHR spec governs storage tiering — our own design/extension. The SM
-- (`docs/specs/openehr/SM/docs/UML/classes/i_admin_archive.adoc`) says only
-- "Move selected EHRs to archival storage" / "Move selected Parties and
-- relationships to archival storage" and defines no storage form; 0001's
-- `vo_archive` marker records WHICH objects are archived, and this migration
-- adds WHERE their rows live once they are.
--
-- Design points (all storage mechanics — spec-silent, our own):
--   * a same-database `cold` schema, so archival needs no tablespace, no
--     partitioning, no external store and no new operational component: the
--     tier is portable to any PostgreSQL 18 deployment and every move is an
--     ordinary transactional statement;
--   * mirror relations built with `CREATE TABLE … (LIKE …)` so the column set,
--     order, types, defaults, check constraints and per-column storage of the
--     primary relations are reproduced exactly — `INSERT INTO cold.x SELECT *
--     FROM x` is then a total, lossless transfer in either direction;
--   * deliberately FOREIGN-KEY-FREE: the archival tier holds rows whose
--     `contribution`/`audit`/`ehr`/`template_ref` parents stay in the primary
--     tier, and a move must never be blocked or cascaded by referential
--     bookkeeping. Integrity is upheld by the service layer, which only ever
--     moves whole versioned objects inside one transaction;
--   * MINIMAL indexes: the cold tier is written by archival sweeps and read
--     only by the marker-gated fallback, so it carries primary keys plus the
--     lookup indexes those paths use — never the full query-serving index set
--     of the primary tables (that index bloat is what the move sheds);
--   * ROW LEVEL SECURITY mirrored from 0004 on the two tenant-scoped mirrors,
--     so archiving can never become a tenant-isolation hole;
--   * `*_all` union views over both tiers for the whole-repository readers
--     (export, physical delete), which must see archived content by definition.
-- Runs with search_path = ehr, ext.

CREATE SCHEMA IF NOT EXISTS cold;

COMMENT ON SCHEMA cold IS 'The cold archival storage tier: mirror relations holding the rows of vo_archive-marked versioned objects, physically moved out of the primary tier. No openEHR spec governs storage tiering — our own design/extension realizing SM I_ADMIN_ARCHIVE ("Move … to archival storage").';

-- ── mirror relations ─────────────────────────────────────────────────────────
-- Indexes are NOT copied (see the header): the minimal set is created
-- explicitly below. CONSTRAINTS copies the CHECKs, so an archived row stays as
-- well-formed as it was in the primary tier.
CREATE TABLE cold.vo_version (
    LIKE vo_version
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE cold.node (
    LIKE node
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE cold.vo_attestation (
    LIKE vo_attestation
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

ALTER TABLE cold.vo_version
    ADD CONSTRAINT pk_cold_vo_version PRIMARY KEY (vo_id, sys_version);
ALTER TABLE cold.node
    ADD CONSTRAINT pk_cold_node PRIMARY KEY (vo_id, sys_version, num);
ALTER TABLE cold.vo_attestation
    ADD CONSTRAINT pk_cold_vo_attestation PRIMARY KEY (id);

-- The EHR-scoped lookups: the archive/restore/purge sweeps and the EHR-keyed
-- read fallbacks address whole EHRs, never single rows.
CREATE INDEX idx_cold_vo_version_ehr ON cold.vo_version (ehr_id, kind);
-- Mirror of idx_vo_version_current_ehr so a union-view current lookup plans
-- the same index path on both legs.
CREATE INDEX idx_cold_vo_version_current_ehr ON cold.vo_version (ehr_id, kind)
    WHERE upper_inf(sys_period) AND branch_number = 0;
CREATE INDEX idx_cold_node_ehr ON cold.node (ehr_id);
CREATE INDEX idx_cold_vo_attestation_version ON cold.vo_attestation (vo_id, sys_version);

COMMENT ON TABLE cold.vo_version IS 'Cold-tier mirror of vo_version: the version rows of vo_archive-marked objects, physically moved out of the primary tier (SM I_ADMIN_ARCHIVE "Move … to archival storage"; no openEHR spec governs storage tiering — our own design). Deliberately FK-free; the service layer moves whole versioned objects transactionally.';
COMMENT ON TABLE cold.node IS 'Cold-tier mirror of node: the content rows of archived versions, moved and restored together with their cold.vo_version rows in one transaction.';
COMMENT ON TABLE cold.vo_attestation IS 'Cold-tier mirror of vo_attestation: the ATTESTATIONs of archived versions (RM common master06 §Attestation), moved with their versions.';

-- ── tenant isolation (mirrors 0004 for the two scoped mirrors) ───────────────
-- `vo_attestation` is not a tenant-scoped table in 0004, so its mirror is not
-- either; `vo_version` and `node` are, and their mirrors must be, otherwise the
-- archival tier would be a cross-tenant read hole. The tenant_id column, its
-- DEFAULT and its NOT NULL came across with LIKE; the FK to `tenant` did not
-- (LIKE never copies foreign keys) and is intentionally not re-added.
DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['vo_version', 'node'] LOOP
        EXECUTE format('ALTER TABLE cold.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE cold.%I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON cold.%I '
            'USING (tenant_id = ext.current_tenant_id()) '
            'WITH CHECK (tenant_id = ext.current_tenant_id())', t);
    END LOOP;
END $$;

-- ── both-tier union views ────────────────────────────────────────────────────
-- For the readers that must see the WHOLE repository regardless of tier: the
-- admin export/dump and the physical-delete sweeps. Serving reads never use
-- these (they consult the cold tier only on a primary miss) — a view that
-- always touches both tiers would put the archival scan back on the hot path,
-- which is the cost the move exists to remove.
--
-- security_invoker so the underlying tables' RLS policies are evaluated as the
-- QUERYING role rather than the view owner
-- (https://www.postgresql.org/docs/18/sql-createview.html).
CREATE VIEW vo_version_all WITH (security_invoker = true) AS
    SELECT * FROM vo_version
    UNION ALL
    SELECT * FROM cold.vo_version;

CREATE VIEW node_all WITH (security_invoker = true) AS
    SELECT * FROM node
    UNION ALL
    SELECT * FROM cold.node;

CREATE VIEW vo_attestation_all WITH (security_invoker = true) AS
    SELECT * FROM vo_attestation
    UNION ALL
    SELECT * FROM cold.vo_attestation;

COMMENT ON VIEW vo_version_all IS 'Both storage tiers of vo_version (primary UNION ALL cold). Every object-addressed serving read goes through this view — one statement serves both tiers (the empty cold side costs one index probe); AQL and the EHR-wide enumerations stay primary-only.';
COMMENT ON VIEW node_all IS 'Both storage tiers of node (primary UNION ALL cold) — the whole-repository readers (admin export, physical delete).';
COMMENT ON VIEW vo_attestation_all IS 'Both storage tiers of vo_attestation (primary UNION ALL cold) — joined by every full version read.';

-- ── vo_archive: the marker now drives a physical move ────────────────────────
COMMENT ON TABLE vo_archive IS 'Archive markers realizing SM I_ADMIN_ARCHIVE (SM openehr_platform master15-admin_service.adoc); the SM defines no storage form — our own design. A marker records that the object''s rows have been MOVED to the cold archival tier (schema `cold`): serving reads find them there on a primary-tier miss, so archival stays read-neutral on the wire. Intentionally FK-less (vo_id is not a per-version key).';

-- ── Grants ───────────────────────────────────────────────────────────────────
-- Role-guarded like every other migration (the baseline's role block explains
-- the graceful degradation when the migrator has no CREATEROLE).
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT USAGE ON SCHEMA cold TO ferroehr_app, ferroehr_reader;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA cold TO ferroehr_app;
        GRANT SELECT ON ALL TABLES IN SCHEMA cold TO ferroehr_reader;
        GRANT SELECT ON vo_version_all, node_all, vo_attestation_all
            TO ferroehr_app, ferroehr_reader;
        ALTER DEFAULT PRIVILEGES IN SCHEMA cold
            GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ferroehr_app;
        ALTER DEFAULT PRIVILEGES IN SCHEMA cold
            GRANT SELECT ON TABLES TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping cold-tier grants (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
