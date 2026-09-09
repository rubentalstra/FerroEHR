-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- Access-logging fields on the audit record: which pseudonymisation domain the
-- operation read, why, on what legal basis, how much it returned, and which
-- request it belonged to.
--
-- The write-side trail (openEHR CONTRIBUTION + AUDIT_DETAILS) and the ATNA
-- record repository already answer who did what to which resource. What a
-- read-side access log additionally has to answer is the purpose and the
-- authority: NEN 7510/7513 require per-access logging of who, when, which
-- record, which action and on whose authority, EHDS Art. 9 requires logging of
-- access to electronic health data for primary use
-- (https://eur-lex.europa.eu/eli/reg/2025/327/oj), and GDPR Art. 30 requires
-- records of processing (https://eur-lex.europa.eu/eli/reg/2016/679/oj).
-- No openEHR spec governs the read-side model — our own design/extension; the
-- SM names only "IHE ATNA-compliant system log" and defines no wire.
--
-- These are columns on the EXISTING audit_event rather than a second table:
-- one operation produces one audit record, and a parallel access-event table
-- would split one trail across two places, so a subject access-log export
-- would have to union them and the hash chain would cover only half.
--
-- Append-only is unchanged and needs no new machinery: the BEFORE UPDATE
-- trigger compares the whole row as jsonb minus the two delivery stamps, so
-- every column added here is covered by it the moment it exists.

ALTER TABLE audit_event
    -- The pseudonymisation domain the operation read or wrote:
    -- `ehr` (clinical), `demographic` (parties), `linkage` (the resolve map),
    -- or `system` for an operation that touches no domain (authentication,
    -- templates, stored queries, node management).
    ADD COLUMN domain       text,
    -- The purpose of use the caller declared, as a code from the deployment's
    -- own allow-list. NULL when the caller declared none.
    ADD COLUMN purpose      text,
    -- The legal basis the deployment attributes to this access, as a code.
    ADD COLUMN legal_basis  text,
    -- How many records the operation served: the row count of an AQL result
    -- set, the entry count of a search. NULL for an operation that serves no
    -- countable set.
    ADD COLUMN result_count bigint,
    -- The request correlation id (the `x-request-id` the response carries), so
    -- an access record joins to the request it belonged to.
    ADD COLUMN request_id   text;

-- NULL is a real answer on the three code columns and means "not recorded":
-- records written before this migration carry no domain, and inventing one for
-- them would be a guess about what a past operation touched.
ALTER TABLE audit_event
    ADD CONSTRAINT ck_audit_event_domain
        CHECK (domain IS NULL OR domain IN ('ehr', 'demographic', 'linkage', 'system')),
    ADD CONSTRAINT ck_audit_event_result_count
        CHECK (result_count IS NULL OR result_count >= 0);

COMMENT ON COLUMN audit_event.domain IS
    'The pseudonymisation domain read or written: ehr | demographic | linkage | system (no domain). NULL on records written before the column existed.';
COMMENT ON COLUMN audit_event.purpose IS
    'The purpose-of-use code the caller declared, from the deployment allow-list; NULL when none was declared.';
COMMENT ON COLUMN audit_event.legal_basis IS
    'The legal basis code the deployment attributes to this access; NULL when the deployment configures none.';
COMMENT ON COLUMN audit_event.result_count IS
    'The number of records served (AQL result rows, search entries); NULL when the operation serves no countable set.';
COMMENT ON COLUMN audit_event.request_id IS
    'The request correlation id, matching the x-request-id header on the response.';

-- The subject access-log export (IHE ITI-81, GET /fhir/r4/AuditEvent?patient=)
-- reads ix_audit_event_patient, which the baseline already declares; a
-- per-domain slice of one subject's log filters that index's result and needs
-- no index of its own.
