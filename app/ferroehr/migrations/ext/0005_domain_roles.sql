-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ext: the domain-named runtime roles.
--
-- The schemas are `clinical` and `party`; the roles that serve them were still
-- spelled for the first generation's schema names. A role name is what an
-- operator reads in a grant listing and what a DSN authenticates as, so it
-- names its domain: ferroehr_clinical / ferroehr_clinical_reader and
-- ferroehr_party / ferroehr_party_reader. ferroehr_linkage already did.
--
-- This file creates them and gives them the ext privileges their predecessors
-- held; each domain's own set moves that domain's grants, and the audit set —
-- which runs last — drops the old names once nothing depends on them.
--
-- No openEHR spec governs database roles: our own design/extension. What the
-- roles realize is the pseudonymisation-domain separation GDPR Art. 4(5) and
-- Art. 32(1)(a) ask for
-- (https://eur-lex.europa.eu/eli/reg/2016/679/oj).
--
-- Runs with search_path = ext.

-- NOLOGIN and NOINHERIT, as their predecessors: a domain role that could
-- inherit another domain's grants would make the boundary a naming convention
-- (PostgreSQL 18, CREATE ROLE, https://www.postgresql.org/docs/18/sql-createrole.html).
DO $$
BEGIN
    BEGIN
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_clinical') THEN
            CREATE ROLE ferroehr_clinical NOLOGIN NOINHERIT;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_clinical_reader') THEN
            CREATE ROLE ferroehr_clinical_reader NOLOGIN NOINHERIT;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_party') THEN
            CREATE ROLE ferroehr_party NOLOGIN NOINHERIT;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_party_reader') THEN
            CREATE ROLE ferroehr_party_reader NOLOGIN NOINHERIT;
        END IF;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'skipping role creation (no CREATEROLE privilege): create ferroehr_clinical/ferroehr_clinical_reader/ferroehr_party/ferroehr_party_reader at deployment';
    END;
END $$;

-- A deployment whose login role was granted membership in ferroehr_ehr or
-- ferroehr_demographic grants it the new name instead; nothing here carries a
-- membership across, because the grant topology of a deployment is the
-- deployment's, not this file's. DROP ROLE revokes memberships by itself
-- (PostgreSQL 18, DROP ROLE, https://www.postgresql.org/docs/18/sql-droprole.html),
-- so the old names leave cleanly once the audit set drops them.

DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_clinical') THEN
        GRANT USAGE ON SCHEMA ext TO
            ferroehr_clinical, ferroehr_clinical_reader,
            ferroehr_party, ferroehr_party_reader;
        -- Future ext functions reachable without a manual grant (PostgreSQL 18
        -- docs, ALTER DEFAULT PRIVILEGES).
        ALTER DEFAULT PRIVILEGES IN SCHEMA ext
            GRANT EXECUTE ON FUNCTIONS TO
                ferroehr_clinical, ferroehr_clinical_reader,
                ferroehr_party, ferroehr_party_reader;
        GRANT EXECUTE ON FUNCTION ext.storage_generation() TO
            ferroehr_clinical, ferroehr_clinical_reader,
            ferroehr_party, ferroehr_party_reader;
        -- The posture writer is the clinical pair's alone, as it was the
        -- clinical role's before.
        GRANT EXECUTE ON FUNCTION ext.stamp_posture(text, text)
            TO ferroehr_clinical;
    ELSE
        RAISE NOTICE 'skipping ext grants for the domain roles (roles absent — see the role block NOTICE)';
    END IF;
END $$;
