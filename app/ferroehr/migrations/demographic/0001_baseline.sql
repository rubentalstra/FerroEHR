-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The demographic pseudonymisation domain: parties leave the clinical schema.
--
-- Parties (PERSON, ORGANISATION, GROUP, AGENT, ROLE, PARTY_RELATIONSHIP) lived
-- in `ehr.vo_version` / `ehr.node` with `ehr_id IS NULL`, reachable by the one
-- runtime role that also reads every composition. That is correct openEHR
-- persistence and it is not a pseudonymised architecture: one role, one backup
-- and one compromised connection each hold the clinical record and the identity
-- of its subject together.
--
-- GDPR Art. 4(5) defines pseudonymisation as processing where attribution to a
-- person needs additional information "kept separately and subject to technical
-- and organisational measures"; Art. 32(1)(a) names it a security measure for
-- Art. 9 health data (https://eur-lex.europa.eu/eli/reg/2016/679/oj). EDPB
-- Guidelines 01/2025 require that separation to hold against internal actors,
-- operators with database access included
-- (https://www.edpb.europa.eu/system/files/2025-01/edpb_guidelines_202501_pseudonymisation_en.pdf).
-- No openEHR spec governs storage layout or database roles: the wire contract
-- and the versioning semantics are unchanged (ITS-REST Demographic API; RM
-- common master06 §Change Control Package), and where each side physically
-- lives is our own design.
--
-- Shape. `demographic` mirrors the clinical relations with
-- `CREATE TABLE ... (LIKE ...)`, the same idiom the cold tier uses, so the
-- versioning engine, the nested-set node codec and the AQL path machinery are
-- reused unchanged and the two schemas cannot drift in column set or CHECKs.
-- `LIKE` never copies foreign keys, which is exactly right here: nothing in
-- this schema may reference `ehr`.
--
-- This migration is INERT on purpose. It creates the domain and leaves every
-- party exactly where it is, so a server running the previous release upgrades
-- through it with nothing to notice: no row moves and no constraint appears.
-- `0002_move_parties` performs the cutover, and ships with the code that
-- follows the parties to their new home — landing the move without that code
-- would delete the parties from under a service still reading `ehr`.
--
-- Runs with search_path = demographic, ext.

-- ── roles ────────────────────────────────────────────────────────────────────
-- Guarded exactly like the `ehr` baseline's role block: created when the
-- migrator holds CREATEROLE (production), skipped with a NOTICE otherwise
-- (dev, compose, testcontainers), where role provisioning is a deployment step.
--
-- NOINHERIT and no membership in one another: a role that could inherit the
-- other domain's grants would make the boundary a naming convention. PostgreSQL
-- 18 CREATE ROLE, "INHERIT / NOINHERIT"
-- (https://www.postgresql.org/docs/18/sql-createrole.html).
DO $$
BEGIN
    BEGIN
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
            CREATE ROLE ferroehr_ehr NOLOGIN NOINHERIT;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_demographic') THEN
            CREATE ROLE ferroehr_demographic NOLOGIN NOINHERIT;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr_reader') THEN
            CREATE ROLE ferroehr_ehr_reader NOLOGIN NOINHERIT;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_demographic_reader') THEN
            CREATE ROLE ferroehr_demographic_reader NOLOGIN NOINHERIT;
        END IF;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'skipping domain role creation (no CREATEROLE privilege): create ferroehr_ehr/ferroehr_demographic/ferroehr_ehr_reader/ferroehr_demographic_reader at deployment';
    END;
END $$;

-- ── schema ───────────────────────────────────────────────────────────────────

CREATE SCHEMA IF NOT EXISTS demographic;

COMMENT ON SCHEMA demographic IS 'The demographic pseudonymisation domain: PARTY versioned objects and their change control, physically separated from the clinical schema so no single runtime role reads both (GDPR Art. 4(5), Art. 32(1)(a); EDPB 01/2025). No openEHR spec governs storage layout — our own design/extension; the ITS-REST Demographic API and RM change-control semantics are unchanged.';

-- ── mirror relations ─────────────────────────────────────────────────────────
-- Indexes are not copied (the primary set is created explicitly below, matching
-- the cold tier's reasoning); CONSTRAINTS carries the CHECKs across, so a party
-- stays as well-formed here as it was in the clinical schema.

CREATE TABLE demographic.audit (
    LIKE ehr.audit
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE demographic.contribution (
    LIKE ehr.contribution
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE demographic.vo_version (
    LIKE ehr.vo_version
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE demographic.node (
    LIKE ehr.node
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE demographic.vo_attestation (
    LIKE ehr.vo_attestation
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE demographic.item_tag (
    LIKE ehr.item_tag
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE demographic.vo_archive (
    LIKE ehr.vo_archive
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

-- ── keys, indexes and the intra-domain foreign keys ──────────────────────────
-- Every foreign key here stays inside `demographic`. The clinical ones the
-- primary tables carry (`ehr_id` into `ehr.ehr`, `template_id` into
-- `ehr.template_ref`) are deliberately NOT re-added: a party has no owning EHR
-- and no template, and a cross-schema reference would re-couple what this
-- migration separates.

ALTER TABLE demographic.audit
    ADD CONSTRAINT pk_dem_audit PRIMARY KEY (id);
ALTER TABLE demographic.contribution
    ADD CONSTRAINT pk_dem_contribution PRIMARY KEY (id);
ALTER TABLE demographic.vo_version
    ADD CONSTRAINT pk_dem_vo_version PRIMARY KEY (vo_id, sys_version);
ALTER TABLE demographic.node
    ADD CONSTRAINT pk_dem_node PRIMARY KEY (vo_id, sys_version, num);
ALTER TABLE demographic.vo_attestation
    ADD CONSTRAINT pk_dem_vo_attestation PRIMARY KEY (id);
ALTER TABLE demographic.item_tag
    ADD CONSTRAINT pk_dem_item_tag PRIMARY KEY (id);
ALTER TABLE demographic.vo_archive
    ADD CONSTRAINT pk_dem_vo_archive PRIMARY KEY (vo_id);

ALTER TABLE demographic.contribution
    ADD CONSTRAINT fk_dem_contribution_audit
        FOREIGN KEY (audit_id) REFERENCES demographic.audit (id);
ALTER TABLE demographic.vo_version
    ADD CONSTRAINT fk_dem_vo_version_contribution
        FOREIGN KEY (contribution_id) REFERENCES demographic.contribution (id),
    ADD CONSTRAINT fk_dem_vo_version_audit
        FOREIGN KEY (audit_id) REFERENCES demographic.audit (id);
ALTER TABLE demographic.node
    ADD CONSTRAINT fk_dem_node_vo_version
        FOREIGN KEY (vo_id, sys_version)
        REFERENCES demographic.vo_version (vo_id, sys_version) ON DELETE CASCADE;
ALTER TABLE demographic.vo_attestation
    ADD CONSTRAINT fk_dem_vo_attestation_vo_version
        FOREIGN KEY (vo_id, sys_version)
        REFERENCES demographic.vo_version (vo_id, sys_version) ON DELETE CASCADE,
    ADD CONSTRAINT fk_dem_vo_attestation_contribution
        FOREIGN KEY (contribution_id) REFERENCES demographic.contribution (id);

-- The tag identity constraint is part of the wire contract (ITS-REST
-- Requests_and_responses.md §openehr-item-tag: a tag is "uniquely identified by
-- their key and target_path pair" within one target), so it is re-created here
-- rather than left to the copied CHECKs.
ALTER TABLE demographic.item_tag
    ADD CONSTRAINT uq_dem_item_tag_identity UNIQUE NULLS NOT DISTINCT
        (ehr_id, target_vo_id, target_version, key, target_path);

-- The access paths the demographic surface actually uses: current-version
-- lookup by object, the version walk, the node interval join, and the tag
-- listing by target.
CREATE INDEX idx_dem_vo_version_current ON demographic.vo_version (vo_id)
    WHERE upper_inf(sys_period) AND branch_number = 0;
CREATE INDEX idx_dem_vo_version_kind ON demographic.vo_version (kind);
CREATE INDEX idx_dem_vo_version_contribution ON demographic.vo_version (contribution_id);
CREATE INDEX idx_dem_node_interval ON demographic.node (vo_id, sys_version, num, num_cap);
CREATE INDEX idx_dem_node_archetype ON demographic.node (archetype)
    WHERE archetype IS NOT NULL;
CREATE INDEX idx_dem_vo_attestation_version ON demographic.vo_attestation (vo_id, sys_version);
CREATE INDEX idx_dem_item_tag_target ON demographic.item_tag (target_vo_id);

COMMENT ON TABLE demographic.vo_version IS 'Version rows of demographic PARTY objects (RM common master06 §Change Control Package), in their own pseudonymisation domain. Deliberately free of any reference into the clinical schema.';
COMMENT ON TABLE demographic.node IS 'Decomposed content rows of demographic PARTY versions — the same nested-set codec as the clinical node table, in the demographic domain.';

-- ── cold archival tier ───────────────────────────────────────────────────────
-- The demographic domain archives like the clinical one (SM I_ADMIN_ARCHIVE),
-- into its own mirror schema. A shared `cold` would put the two domains back in
-- one place, which is the whole thing this migration removes.

CREATE SCHEMA IF NOT EXISTS cold_demographic;

COMMENT ON SCHEMA cold_demographic IS 'Cold archival tier of the demographic domain. Separate from `cold` so archiving does not re-merge the two pseudonymisation domains. No openEHR spec governs storage tiering — our own design/extension.';

CREATE TABLE cold_demographic.vo_version (
    LIKE demographic.vo_version
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE cold_demographic.node (
    LIKE demographic.node
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

CREATE TABLE cold_demographic.vo_attestation (
    LIKE demographic.vo_attestation
    INCLUDING DEFAULTS
    INCLUDING CONSTRAINTS
    INCLUDING COMMENTS
    INCLUDING STORAGE
    INCLUDING COMPRESSION
);

ALTER TABLE cold_demographic.vo_version
    ADD CONSTRAINT pk_cold_dem_vo_version PRIMARY KEY (vo_id, sys_version);
ALTER TABLE cold_demographic.node
    ADD CONSTRAINT pk_cold_dem_node PRIMARY KEY (vo_id, sys_version, num);
ALTER TABLE cold_demographic.vo_attestation
    ADD CONSTRAINT pk_cold_dem_vo_attestation PRIMARY KEY (id);

CREATE INDEX idx_cold_dem_node_version ON cold_demographic.node (vo_id, sys_version);
CREATE INDEX idx_cold_dem_vo_attestation_version
    ON cold_demographic.vo_attestation (vo_id, sys_version);

-- The union views the object-addressed reads go through, named exactly as the
-- clinical ones so the storage layer reaches them by search_path alone.
CREATE VIEW demographic.vo_version_all WITH (security_invoker = true) AS
    SELECT * FROM demographic.vo_version
    UNION ALL
    SELECT * FROM cold_demographic.vo_version;

CREATE VIEW demographic.node_all WITH (security_invoker = true) AS
    SELECT * FROM demographic.node
    UNION ALL
    SELECT * FROM cold_demographic.node;

CREATE VIEW demographic.vo_attestation_all WITH (security_invoker = true) AS
    SELECT * FROM demographic.vo_attestation
    UNION ALL
    SELECT * FROM cold_demographic.vo_attestation;

COMMENT ON VIEW demographic.vo_version_all IS 'Both storage tiers of the demographic vo_version (primary UNION ALL cold_demographic) — the object-addressed serving reads.';

-- ── tenant isolation ─────────────────────────────────────────────────────────
-- The same tenant context as the clinical schema (`ferroehr.tenant_id`), so a
-- demographic read is tenant-scoped exactly as a clinical one is. FORCE, so the
-- policy applies to the table owner too. The tenant_id column, its DEFAULT and
-- its NOT NULL came across with LIKE; the FK to `ehr.tenant` did not, and is
-- deliberately not re-added.
DO $$
DECLARE
    rel text;
BEGIN
    FOREACH rel IN ARRAY ARRAY['vo_version', 'node', 'item_tag'] LOOP
        EXECUTE format('ALTER TABLE demographic.%I ENABLE ROW LEVEL SECURITY', rel);
        EXECUTE format('ALTER TABLE demographic.%I FORCE ROW LEVEL SECURITY', rel);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON demographic.%I '
            'USING (tenant_id = current_setting(''ferroehr.tenant_id'', true)::uuid) '
            'WITH CHECK (tenant_id = current_setting(''ferroehr.tenant_id'', true)::uuid)',
            rel);
    END LOOP;
    FOREACH rel IN ARRAY ARRAY['vo_version', 'node'] LOOP
        EXECUTE format('ALTER TABLE cold_demographic.%I ENABLE ROW LEVEL SECURITY', rel);
        EXECUTE format('ALTER TABLE cold_demographic.%I FORCE ROW LEVEL SECURITY', rel);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON cold_demographic.%I '
            'USING (tenant_id = current_setting(''ferroehr.tenant_id'', true)::uuid) '
            'WITH CHECK (tenant_id = current_setting(''ferroehr.tenant_id'', true)::uuid)',
            rel);
    END LOOP;
END $$;

-- ── grants ───────────────────────────────────────────────────────────────────
-- Explicit and non-overlapping. Each domain role is granted USAGE and DML on
-- its own schema and its own cold tier, and is REVOKEd from the other, which is
-- what makes the boundary hold against a role that later gains membership
-- somewhere unexpected. The revoke is not redundant with "never granted":
-- `ferroehr_app` held blanket grants on `ehr` and PUBLIC holds EXECUTE on
-- functions by default (PostgreSQL 18, GRANT, "Notes"
-- https://www.postgresql.org/docs/18/sql-grant.html).
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_demographic') THEN
        -- The demographic domain to its own roles.
        GRANT USAGE ON SCHEMA demographic, cold_demographic
            TO ferroehr_demographic, ferroehr_demographic_reader;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA demographic
            TO ferroehr_demographic;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA cold_demographic
            TO ferroehr_demographic;
        GRANT SELECT ON ALL TABLES IN SCHEMA demographic, cold_demographic
            TO ferroehr_demographic_reader;
        ALTER DEFAULT PRIVILEGES IN SCHEMA demographic, cold_demographic
            GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ferroehr_demographic;
        ALTER DEFAULT PRIVILEGES IN SCHEMA demographic, cold_demographic
            GRANT SELECT ON TABLES TO ferroehr_demographic_reader;

        -- The clinical domain to its own roles, replacing the blanket
        -- ferroehr_app grants the baseline installed.
        GRANT USAGE ON SCHEMA ehr, cold TO ferroehr_ehr, ferroehr_ehr_reader;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA ehr, cold
            TO ferroehr_ehr;
        GRANT SELECT ON ALL TABLES IN SCHEMA ehr, cold TO ferroehr_ehr_reader;
        ALTER DEFAULT PRIVILEGES IN SCHEMA ehr, cold
            GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ferroehr_ehr;
        ALTER DEFAULT PRIVILEGES IN SCHEMA ehr, cold
            GRANT SELECT ON TABLES TO ferroehr_ehr_reader;

        -- The `ext` helper functions are needed by both domains: the node
        -- writers call ext.openehr_timestamp, and the collations live there.
        GRANT USAGE ON SCHEMA ext
            TO ferroehr_ehr, ferroehr_demographic,
               ferroehr_ehr_reader, ferroehr_demographic_reader;

        -- And the explicit denial in both directions.
        REVOKE ALL ON SCHEMA demographic, cold_demographic
            FROM ferroehr_ehr, ferroehr_ehr_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA demographic, cold_demographic
            FROM ferroehr_ehr, ferroehr_ehr_reader;
        REVOKE ALL ON SCHEMA ehr, cold
            FROM ferroehr_demographic, ferroehr_demographic_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA ehr, cold
            FROM ferroehr_demographic, ferroehr_demographic_reader;
    ELSE
        RAISE NOTICE 'skipping demographic grants (roles absent — see the role block NOTICE)';
    END IF;
END $$;
