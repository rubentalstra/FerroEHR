-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The split clinical runtime role reaches the audit trail (#3267).
--
-- 0002 granted the local Audit Record Repository to the single-domain pair
-- (ferroehr_app, ferroehr_reader) only. The store writes through the CLINICAL
-- pool, so a deployment whose clinical DSN is a member of ferroehr_ehr alone,
-- the posture the two-DSN documentation recommends, could not write one access
-- record: dropped and metered under fail_mode = "open", every auditable
-- operation refused under "closed". The audit trail is not a pseudonymisation
-- domain; the clinical role holds in `audit` exactly what ferroehr_app holds,
-- and the clinical reader what ferroehr_reader holds. No openEHR spec governs
-- database roles — our own design/extension.

DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        GRANT USAGE ON SCHEMA audit TO ferroehr_ehr, ferroehr_ehr_reader;
        GRANT SELECT, INSERT ON audit_event TO ferroehr_ehr;
        GRANT UPDATE (delivered_syslog_at, delivered_fhir_feed_at)
            ON audit_event TO ferroehr_ehr;
        GRANT SELECT ON audit_event TO ferroehr_ehr_reader;
        GRANT SELECT ON audit_chain_state, audit_chain_gap
            TO ferroehr_ehr, ferroehr_ehr_reader;
        GRANT EXECUTE ON FUNCTION audit.reap_audit_events(integer) TO ferroehr_ehr;
        GRANT EXECUTE ON FUNCTION audit.verify_audit_chain()
            TO ferroehr_ehr, ferroehr_ehr_reader;
    ELSE
        RAISE NOTICE 'skipping audit grants for the split roles (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
