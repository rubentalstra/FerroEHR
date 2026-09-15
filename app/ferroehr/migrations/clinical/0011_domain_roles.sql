-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- clinical: the grants, moved onto the domain-named roles.
--
-- The privileges ferroehr_ehr and ferroehr_ehr_reader held over this schema
-- move to ferroehr_clinical and ferroehr_clinical_reader, and the reciprocal
-- revokes are re-issued against the party and linkage roles by their new names.
-- The old names are revoked here and dropped by the audit set, which runs last.
--
-- No openEHR spec governs database grants: our own operational design.
--
-- Runs with search_path = clinical, ext, public.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_clinical') THEN
        GRANT USAGE ON SCHEMA clinical
            TO ferroehr_clinical, ferroehr_clinical_reader;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA clinical
            TO ferroehr_clinical;
        GRANT SELECT ON ALL TABLES IN SCHEMA clinical
            TO ferroehr_clinical_reader;
        ALTER DEFAULT PRIVILEGES IN SCHEMA clinical
            GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ferroehr_clinical;
        ALTER DEFAULT PRIVILEGES IN SCHEMA clinical
            GRANT SELECT ON TABLES TO ferroehr_clinical_reader;
        -- The subject guard runs as the writer that triggers it.
        GRANT EXECUTE ON FUNCTION clinical.subject_pseudonym_guard()
            TO ferroehr_clinical;

        -- The party and linkage roles hold nothing here, by their new names.
        -- The revoke is not redundant with never having granted: PUBLIC holds
        -- privileges by default on some object kinds (PostgreSQL 18, GRANT,
        -- "Notes", https://www.postgresql.org/docs/18/sql-grant.html).
        REVOKE ALL ON SCHEMA clinical
            FROM ferroehr_party, ferroehr_party_reader, ferroehr_linkage;
        REVOKE ALL ON ALL TABLES IN SCHEMA clinical
            FROM ferroehr_party, ferroehr_party_reader, ferroehr_linkage;
    ELSE
        RAISE NOTICE 'skipping clinical domain-role grants (roles absent — see the ext role block NOTICE)';
    END IF;
END $$;

-- The predecessors, emptied of this schema so nothing depends on them when the
-- audit set drops them. A default-privilege entry counts as a dependency, so it
-- is withdrawn too (PostgreSQL 18, ALTER DEFAULT PRIVILEGES).
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        ALTER DEFAULT PRIVILEGES IN SCHEMA clinical
            REVOKE ALL ON TABLES FROM ferroehr_ehr, ferroehr_ehr_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA clinical
            FROM ferroehr_ehr, ferroehr_ehr_reader;
        REVOKE ALL ON ALL FUNCTIONS IN SCHEMA clinical
            FROM ferroehr_ehr, ferroehr_ehr_reader;
        REVOKE ALL ON SCHEMA clinical FROM ferroehr_ehr, ferroehr_ehr_reader;
    END IF;
END $$;
