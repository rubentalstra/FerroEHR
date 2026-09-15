-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- audit: the grants, moved onto the domain-named clinical role, and the
-- retirement of the first generation's role names.
--
-- The audit repository is not a pseudonymisation domain: openEHR endorses
-- in-system access logging and rules it out of the EHR content (BASE
-- architecture_overview/master07-security.adoc §Access logging), so the trail
-- is written by the clinical runtime role and read by its read-only twin. What
-- the grants withhold is as deliberate as what they give — no role may UPDATE a
-- record's content or DELETE one; the reaper is the only sanctioned removal
-- path and it is a function.
--
-- This set runs last, so it is the first place the predecessors hold nothing
-- anywhere and can be dropped.
--
-- No openEHR spec governs database roles — our own design/extension.

DO $grants$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_clinical') THEN
        GRANT USAGE ON SCHEMA audit
            TO ferroehr_clinical, ferroehr_clinical_reader;
        ALTER DEFAULT PRIVILEGES IN SCHEMA audit
            GRANT SELECT, INSERT ON TABLES TO ferroehr_clinical;
        ALTER DEFAULT PRIVILEGES IN SCHEMA audit
            GRANT SELECT ON TABLES TO ferroehr_clinical_reader;

        -- Revoke first: a table-level REVOKE also removes column-level grants,
        -- so the column grant below has to come after it (PostgreSQL 18 docs,
        -- REVOKE: https://www.postgresql.org/docs/18/sql-revoke.html).
        REVOKE ALL ON audit_event FROM ferroehr_clinical, ferroehr_clinical_reader;
        GRANT SELECT, INSERT ON audit_event TO ferroehr_clinical;
        GRANT UPDATE (delivered_syslog_at, delivered_fhir_feed_at)
            ON audit_event TO ferroehr_clinical;
        GRANT SELECT ON audit_event TO ferroehr_clinical_reader;

        REVOKE ALL ON audit_chain_state, audit_chain_gap
            FROM ferroehr_clinical, ferroehr_clinical_reader;
        GRANT SELECT ON audit_chain_state, audit_chain_gap
            TO ferroehr_clinical, ferroehr_clinical_reader;

        GRANT EXECUTE ON FUNCTION audit.verify_audit_chain()
            TO ferroehr_clinical, ferroehr_clinical_reader;
        GRANT EXECUTE ON FUNCTION audit.reap_audit_events(integer)
            TO ferroehr_clinical;
    ELSE
        RAISE NOTICE 'skipping audit domain-role grants (roles absent — see the ext role block NOTICE)';
    END IF;
END
$grants$;

-- The predecessors, emptied of this schema and then dropped.
--
-- DROP ROLE refuses while the role holds a privilege in any database of the
-- cluster (PostgreSQL 18, DROP ROLE,
-- https://www.postgresql.org/docs/18/sql-droprole.html). Every set above
-- withdrew this database's; a deployment that spreads its domains over several
-- databases of one cluster still has the other databases' grants in place when
-- the first one reaches this file, so a refusal is reported and the old name is
-- left behind rather than failing the migration. Memberships need no
-- withdrawal: DROP ROLE revokes them itself.
DO $retire$
DECLARE
    retired text;
BEGIN
    FOREACH retired IN ARRAY ARRAY['ferroehr_ehr', 'ferroehr_ehr_reader',
                                   'ferroehr_demographic', 'ferroehr_demographic_reader']
    LOOP
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = retired) THEN
            CONTINUE;
        END IF;
        BEGIN
            EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA audit REVOKE ALL ON TABLES FROM %I', retired);
            EXECUTE format('REVOKE ALL ON ALL TABLES IN SCHEMA audit FROM %I', retired);
            EXECUTE format('REVOKE ALL ON ALL FUNCTIONS IN SCHEMA audit FROM %I', retired);
            EXECUTE format('REVOKE ALL ON SCHEMA audit FROM %I', retired);
            EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA ext REVOKE ALL ON FUNCTIONS FROM %I', retired);
            EXECUTE format('REVOKE ALL ON ALL FUNCTIONS IN SCHEMA ext FROM %I', retired);
            EXECUTE format('REVOKE ALL ON SCHEMA ext FROM %I', retired);
            EXECUTE format('DROP ROLE %I', retired);
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'role % could not be retired (%); revoke its remaining privileges and DROP ROLE it by hand', retired, SQLERRM;
        END;
    END LOOP;
END
$retire$;
