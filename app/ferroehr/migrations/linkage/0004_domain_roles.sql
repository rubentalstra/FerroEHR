-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- linkage: the reciprocal revokes, re-issued against the domain-named roles.
--
-- Neither direction: linkage out of the two domains it joins, and both of them
-- out of linkage. A role that could read this schema and one of the others
-- would hold the join this schema exists to withhold.
--
-- No openEHR spec governs database grants: our own operational design.
--
-- Runs with search_path = linkage, ext, public.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_clinical') THEN
        REVOKE ALL ON SCHEMA clinical, party FROM ferroehr_linkage;
        REVOKE ALL ON ALL TABLES IN SCHEMA clinical, party FROM ferroehr_linkage;

        REVOKE ALL ON SCHEMA linkage
            FROM ferroehr_clinical, ferroehr_clinical_reader,
                 ferroehr_party, ferroehr_party_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA linkage
            FROM ferroehr_clinical, ferroehr_clinical_reader,
                 ferroehr_party, ferroehr_party_reader;
    ELSE
        RAISE NOTICE 'skipping linkage domain-role revokes (roles absent — see the ext role block NOTICE)';
    END IF;
END $$;

-- The predecessors, emptied of this schema.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        ALTER DEFAULT PRIVILEGES IN SCHEMA linkage
            REVOKE ALL ON TABLES FROM ferroehr_ehr, ferroehr_ehr_reader,
                                     ferroehr_demographic, ferroehr_demographic_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA linkage
            FROM ferroehr_ehr, ferroehr_ehr_reader,
                 ferroehr_demographic, ferroehr_demographic_reader;
        REVOKE ALL ON SCHEMA linkage
            FROM ferroehr_ehr, ferroehr_ehr_reader,
                 ferroehr_demographic, ferroehr_demographic_reader;
    END IF;
END $$;
