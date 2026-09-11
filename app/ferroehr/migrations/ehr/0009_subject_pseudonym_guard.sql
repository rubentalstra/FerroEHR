-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The database's own line of defence for the subject pseudonym (#3241).
--
-- Once a deployment declares its pseudonym namespaces
-- (privacy.subject_namespaces), the service refuses an EHR_STATUS subject
-- reference that is not an opaque UUID. That rule lived only in the service:
-- a write path that bypassed the validator, a direct repair, a defect, could
-- store a national identifier as the clinical subject reference with nothing
-- in the database to refuse it (GDPR Art. 25(2),
-- https://eur-lex.europa.eu/eli/reg/2016/679/oj). openEHR itself puts no
-- shape on EHR_STATUS.subject.external_ref.id (RM ehr §EHR_STATUS), so the
-- guard cannot be an unconditional CHECK: a deployment that declares no
-- namespace accepts any identifier, as the released spec allows, and the
-- conformance suite exercises exactly that.
--
-- The condition is therefore a posture the server stamps at boot into the
-- `posture` table, which the trigger reads. No per-connection setting: the
-- runtime role writes the stamp once, every connection sees it, and a repair
-- session that connects with its own credential is bound by it too.
--
-- No openEHR spec governs storage — our own design/extension.

CREATE TABLE posture (
    key        text        PRIMARY KEY,
    value      text        NOT NULL,
    stamped_at timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE posture IS
    'Deployment posture the server stamps at boot and database guards read; `subject_pseudonyms` = required | open.';

CREATE OR REPLACE FUNCTION subject_pseudonym_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    required text;
BEGIN
    IF NEW.subject_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT value INTO required FROM ehr.posture WHERE key = 'subject_pseudonyms';
    IF required = 'required'
       AND NEW.subject_id !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
    THEN
        RAISE EXCEPTION USING
            ERRCODE = 'check_violation',
            MESSAGE = 'ehr.subject_id must be a UUID: this deployment declares privacy.subject_namespaces, so the clinical side holds an opaque subject pseudonym, never a national identifier, a medical-record number or a name',
            CONSTRAINT = 'ehr_subject_pseudonym_guard';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER ehr_subject_pseudonym_guard
    BEFORE INSERT OR UPDATE OF subject_id, subject_namespace ON ehr
    FOR EACH ROW EXECUTE FUNCTION subject_pseudonym_guard();

-- A function is executable by PUBLIC by default, which the boot gate
-- (`verify_domain_isolation`) rightly reads as the demographic and linkage
-- roles reaching a clinical-domain function. Only the clinical writers, whose
-- row writes fire the trigger, may execute it.
REVOKE ALL ON FUNCTION subject_pseudonym_guard() FROM PUBLIC;

-- The split runtime roles read the stamp and the clinical one writes it; the
-- baseline's default privileges already cover ferroehr_app / ferroehr_reader
-- on the table, and the function needs its own grants.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT EXECUTE ON FUNCTION subject_pseudonym_guard() TO ferroehr_app;
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_ehr') THEN
        GRANT SELECT, INSERT, UPDATE ON posture TO ferroehr_ehr;
        GRANT SELECT ON posture TO ferroehr_ehr_reader;
        GRANT EXECUTE ON FUNCTION subject_pseudonym_guard() TO ferroehr_ehr;
    ELSE
        RAISE NOTICE 'skipping posture grants for the split roles (roles absent)';
    END IF;
END $$;
