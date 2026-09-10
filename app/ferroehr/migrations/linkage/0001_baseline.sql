-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The linkage domain: which party is the subject of which EHR.
--
-- The clinical schema holds an opaque subject pseudonym and the demographic
-- schema holds the person. Neither can reach the other on its own: the
-- pseudonym identifies nobody, and a party carries no reference to a record.
-- This schema holds the one fact that joins them, and holds it apart from
-- both, under a role neither of them has and which has neither of theirs.
--
-- GDPR Art. 4(5) defines pseudonymisation as processing where attribution to
-- a person needs "additional information" that is "kept separately and
-- subject to technical and organisational measures"
-- (https://eur-lex.europa.eu/eli/reg/2016/679/oj); Art. 32(1)(a) names
-- pseudonymisation a security measure for health data, and EDPB Guidelines
-- 01/2025 require the separation to hold against internal actors, database
-- operators included
-- (https://www.edpb.europa.eu/system/files/2025-01/edpb_guidelines_202501_pseudonymisation_en.pdf).
--
-- No openEHR spec governs storage layout or database roles — our own
-- design/extension, and nothing on the openEHR wire depends on this table:
-- `ehr_get_by_subject` obliges a match against the EHR's own
-- `EHR_STATUS.subject.external_ref.id.value` and `.namespace` (ITS-REST
-- `specifications/operations/ehr_get_by_subject.yaml`), which the clinical
-- schema serves from its own promoted columns.
--
-- The identity-to-party half of the join already exists as
-- `demographic.national_identifier`, sealed and resolved by keyed digest.
-- This schema deliberately does NOT repeat it: two stores of the same mapping
-- drift, and then neither is the answer.
--
-- This migration is INERT for a running server. It creates a schema, one
-- table and one role; no existing relation is touched, no row moves, and no
-- pool connects here.
--
-- Runs with search_path = linkage, ext.

-- ── the role ─────────────────────────────────────────────────────────────────
-- Guarded exactly like the demographic baseline's role block: created when the
-- migrator holds CREATEROLE (production), skipped with a NOTICE otherwise
-- (dev, compose, the test harness), where role provisioning is a deployment
-- step.
--
-- NOINHERIT and no membership in any other domain role: a role that could
-- inherit the clinical or demographic grants would make this boundary a naming
-- convention (PostgreSQL 18 CREATE ROLE, "INHERIT / NOINHERIT",
-- https://www.postgresql.org/docs/18/sql-createrole.html).
--
-- One role, not the writer/reader pair the other two domains carry: this
-- domain has one table, one writer, and no reporting surface for a read-only
-- twin to serve.
DO $$
BEGIN
    BEGIN
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_linkage') THEN
            CREATE ROLE ferroehr_linkage NOLOGIN NOINHERIT;
        END IF;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'skipping linkage role creation (no CREATEROLE privilege): create ferroehr_linkage at deployment';
    END;
END $$;

-- ── schema ───────────────────────────────────────────────────────────────────

CREATE SCHEMA IF NOT EXISTS linkage;

COMMENT ON SCHEMA linkage IS 'The linkage pseudonymisation domain: the map from a demographic party to the EHR whose subject it is. The additional information that re-joins a pseudonymised clinical record to a person (GDPR Art. 4(5)), held apart from both domains it joins and reachable only by ferroehr_linkage. No openEHR spec governs storage layout — our own design/extension.';

-- ── the party-to-EHR map ─────────────────────────────────────────────────────
-- Temporal rather than current-state: a merge or a split of two person records
-- CLOSES a mapping and opens another, so the question "which party was the
-- subject of this EHR when that composition was written" stays answerable.
-- Deleting the old row would answer it wrongly and silently.

CREATE TABLE linkage.party_ehr (
    -- The demographic VERSIONED_OBJECT id of the party.
    --
    -- No foreign key, for the reason the whole schema exists: a reference into
    -- `demographic` would couple this table to a schema whose role cannot read
    -- it and whose role cannot read this one, and PostgreSQL enforces a
    -- foreign key by reading the referenced row. The same shape as
    -- `demographic.national_identifier.party_id`, which declines an FK for the
    -- related reason that a party's versions come and go under change control.
    party_id   uuid      NOT NULL,
    -- The EHR the party is the subject of. No foreign key into `ehr` either,
    -- and for the stronger form of the same reason: that one would cross the
    -- clinical boundary.
    ehr_id     uuid      NOT NULL,
    -- The owning tenant, spelled and defaulted as every other scoped table
    -- spells it. It is a key part rather than a filter, so no lookup here can
    -- span two tenants even before the row policy below is consulted.
    tenant_id  uuid      NOT NULL DEFAULT ext.current_tenant_id(),
    -- Validity interval of the mapping, half-open [opened, closed). An open
    -- upper bound is the mapping in force now. Committal time is the only
    -- server-managed temporal axis openEHR speaks about (RM common master06
    -- §Committal and Audits); the interval itself is our own storage design.
    sys_period tstzrange NOT NULL DEFAULT tstzrange(now(), NULL, '[)'),
    -- The temporal primary key: one party has at most ONE mapping in force at
    -- any instant, enforced by the database rather than by whichever code path
    -- happens to write. PostgreSQL 18 builds a GiST index for a key carrying
    -- WITHOUT OVERLAPS, and the equality key parts need the btree_gist
    -- operator classes (PostgreSQL 18 CREATE TABLE, "PRIMARY KEY", and
    -- btree_gist, https://www.postgresql.org/docs/18/sql-createtable.html);
    -- the extension is installed in `ext` by the bootstrap.
    --
    -- The clinical schema pays a plain btree primary key on its version table
    -- instead, because GiST exclusion serializes concurrent inserts
    -- (PostgreSQL 18, "Exclusion Constraints") and that is its hot write path.
    -- This table is written once per EHR and again only on a merge or a split,
    -- so the constraint costs nothing measurable here and the invariant is
    -- worth more than the microseconds.
    CONSTRAINT pk_party_ehr PRIMARY KEY (tenant_id, party_id, sys_period WITHOUT OVERLAPS)
);

-- A resolve runs in both directions — party to EHR through the primary key,
-- EHR to party through this one. Plain btree rather than a second GiST: a
-- given EHR carries one row per mapping epoch, so narrowing to the EHR leaves
-- a handful of rows for the range predicate to recheck.
CREATE INDEX idx_party_ehr_by_ehr ON linkage.party_ehr (tenant_id, ehr_id);

COMMENT ON TABLE linkage.party_ehr IS
    'Which demographic party is the subject of which EHR, temporally: a merge or split closes a row and opens another rather than deleting. Holds identifiers ONLY — no name, no address, no plaintext identifier of any kind — because a row here is already the additional information that re-joins a record to a person (GDPR Art. 4(5)), and adding an attribute would make it identifying on its own.';
COMMENT ON COLUMN linkage.party_ehr.sys_period IS
    'Validity interval of the mapping, half-open [opened, closed); an open upper bound is the mapping in force now. Part of the temporal primary key, so one party cannot hold two mappings at one instant.';

-- ── tenant isolation ─────────────────────────────────────────────────────────
-- The same predicate the clinical and demographic relations carry
-- (`ext.current_tenant_id()` resolves an unset `ferroehr.tenant_id` GUC to the
-- reserved default tenant), so a single-tenant deployment behaves exactly as
-- it did before tenancy existed. FORCE, so the policy applies to the table
-- owner too.

ALTER TABLE linkage.party_ehr ENABLE ROW LEVEL SECURITY;
ALTER TABLE linkage.party_ehr FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON linkage.party_ehr
    USING (tenant_id = ext.current_tenant_id())
    WITH CHECK (tenant_id = ext.current_tenant_id());

-- ── grants ───────────────────────────────────────────────────────────────────
-- Explicit in both directions and non-overlapping with either other domain.
-- The revokes are not redundant with "never granted": PUBLIC holds EXECUTE on
-- functions by default and earlier blanket grants exist on the clinical schema
-- (PostgreSQL 18, GRANT, "Notes",
-- https://www.postgresql.org/docs/18/sql-grant.html).
--
-- No DELETE for the linkage role: a mapping is closed by setting the upper
-- bound of its period, never removed, and a privilege the design does not use
-- is a privilege a defect can.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_linkage') THEN
        GRANT USAGE ON SCHEMA linkage TO ferroehr_linkage;
        GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA linkage
            TO ferroehr_linkage;
        ALTER DEFAULT PRIVILEGES IN SCHEMA linkage
            GRANT SELECT, INSERT, UPDATE ON TABLES TO ferroehr_linkage;
        -- The row policy calls ext.current_tenant_id(), so the role needs the
        -- helper schema exactly as the other domain roles do.
        GRANT USAGE ON SCHEMA ext TO ferroehr_linkage;

        -- Neither direction: linkage out of the two domains it joins, and both
        -- of them out of linkage. A role that could read this schema and one
        -- of the others would hold the join this schema exists to withhold.
        REVOKE ALL ON SCHEMA ehr, cold, demographic, cold_demographic
            FROM ferroehr_linkage;
        REVOKE ALL ON ALL TABLES IN SCHEMA ehr, cold, demographic, cold_demographic
            FROM ferroehr_linkage;
        REVOKE ALL ON FUNCTION
            demographic.resolve_national_identifier(uuid, text, bytea)
            FROM ferroehr_linkage;
    ELSE
        RAISE NOTICE 'skipping linkage grants (role absent — see the role block NOTICE)';
    END IF;

    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        REVOKE ALL ON SCHEMA linkage
            FROM ferroehr_ehr, ferroehr_ehr_reader,
                 ferroehr_demographic, ferroehr_demographic_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA linkage
            FROM ferroehr_ehr, ferroehr_ehr_reader,
                 ferroehr_demographic, ferroehr_demographic_reader;
    ELSE
        RAISE NOTICE 'skipping the clinical/demographic revokes on linkage (roles absent — see the demographic baseline role block)';
    END IF;
END $$;
