-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- party: the grants, moved onto the domain-named roles.
--
-- The sealed identifier store keeps its narrower shape: only the party writer
-- reaches the ciphertext and the lookup digest, and the party reader holds
-- every other column — a holder of the lookup subkey could otherwise ask
-- whether a known identifier is present without ever decrypting anything.
--
-- No openEHR spec governs database grants: our own operational design.
--
-- Runs with search_path = party, ext, public.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_party') THEN
        GRANT USAGE ON SCHEMA party
            TO ferroehr_party, ferroehr_party_reader;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA party
            TO ferroehr_party;
        GRANT SELECT ON ALL TABLES IN SCHEMA party
            TO ferroehr_party_reader;
        ALTER DEFAULT PRIVILEGES IN SCHEMA party
            GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ferroehr_party;
        ALTER DEFAULT PRIVILEGES IN SCHEMA party
            GRANT SELECT ON TABLES TO ferroehr_party_reader;

        REVOKE SELECT ON party.national_identifier FROM ferroehr_party_reader;
        GRANT SELECT (id, party_id, scheme, created_at)
            ON party.national_identifier TO ferroehr_party_reader;
        GRANT EXECUTE ON FUNCTION party.resolve_national_identifier(text, bytea)
            TO ferroehr_party;

        -- And the explicit denial in both directions, by the new names.
        REVOKE ALL ON SCHEMA party
            FROM ferroehr_clinical, ferroehr_clinical_reader, ferroehr_linkage;
        REVOKE ALL ON ALL TABLES IN SCHEMA party
            FROM ferroehr_clinical, ferroehr_clinical_reader, ferroehr_linkage;
        REVOKE ALL ON FUNCTION party.resolve_national_identifier(text, bytea)
            FROM ferroehr_clinical, ferroehr_clinical_reader, ferroehr_linkage;
        REVOKE ALL ON SCHEMA clinical
            FROM ferroehr_party, ferroehr_party_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA clinical
            FROM ferroehr_party, ferroehr_party_reader;
    ELSE
        RAISE NOTICE 'skipping party domain-role grants (roles absent — see the ext role block NOTICE)';
    END IF;
END $$;

-- The predecessors, emptied of this schema.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_demographic') THEN
        ALTER DEFAULT PRIVILEGES IN SCHEMA party
            REVOKE ALL ON TABLES FROM ferroehr_demographic, ferroehr_demographic_reader;
        REVOKE ALL ON ALL TABLES IN SCHEMA party
            FROM ferroehr_demographic, ferroehr_demographic_reader;
        REVOKE ALL ON ALL FUNCTIONS IN SCHEMA party
            FROM ferroehr_demographic, ferroehr_demographic_reader;
        REVOKE ALL ON SCHEMA party FROM ferroehr_demographic, ferroehr_demographic_reader;
    END IF;
END $$;
