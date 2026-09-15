-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ext: the grants over the helper functions.
--
-- One file per schema carries every grant, so the relation and function files
-- stay declarations of shape. `ext.stamp_posture` writes the deployment
-- posture the server declares at boot, so it is the clinical pair's alone and
-- revoked from PUBLIC.
--
-- No openEHR spec governs database roles — our own design/extension.

REVOKE ALL ON FUNCTION stamp_posture(text, text) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT EXECUTE ON FUNCTION ext.stamp_posture(text, text)
            TO ferroehr_app, ferroehr_ehr;
    ELSE
        RAISE NOTICE 'skipping ext.stamp_posture grant (roles absent — see the role block NOTICE)';
    END IF;
END $$;
