-- SPDX-FileCopyrightText: FerroEHR contributors
-- SPDX-License-Identifier: MIT

-- audit schema baseline: the local IHE ATNA Audit Record Repository.
--
-- NOTE: no openEHR spec governs audit storage mechanics — our own
-- design/extension. openEHR endorses in-system access logging and rules it
-- OUT of the EHR content ("read accesses by application users to EHR data
-- should be logged in the EHR system. ... currently openEHR does not support
-- [logs as part of the EHR proper]" — BASE
-- architecture_overview/master07-security.adoc §Access logging), so this
-- schema is strictly separate from the `ehr` schema: no foreign key into any
-- EHR-content table, no AQL visibility, never versioned. The record payload
-- is the FHIR R4 AuditEvent per the IHE BALP content profiles — the same
-- document the RESTful-ATNA ITI-81 search serves; the promoted columns exist
-- only for search/browse selectivity and are derived from it.
--
-- NOT row-level-security-scoped (deliberate): the ATNA audit trail is the
-- NODE's security-surveillance record (IHE ITI TF-1 §9) — an operator/admin
-- surface that must show cross-tenant security events (rejected access
-- attempts arrive before any tenant scope exists). The record still CARRIES
-- the resolved tenant (`tenant_id`, informational, no FK — an audit write
-- must never fail because a tenant row was removed); the admin-only ITI-81
-- surface can filter on it. The background drain task also writes outside
-- any request's tenant session, so an RLS WITH CHECK would misclassify every
-- row as the default tenant.
--
-- Append-only: the only permitted mutations are the per-sink delivery
-- stamps (`delivered_*_at`, the forwarding outbox) and retention reaping
-- (deletion of rows older than the configured horizon).

CREATE TABLE audit_event (
    id                     uuid        NOT NULL DEFAULT uuidv7(),
    -- The event time (FHIR AuditEvent.recorded / DICOM EventDateTime).
    recorded_at            timestamptz NOT NULL,
    -- When the row was persisted (drain-side; ordering tiebreaker).
    stored_at              timestamptz NOT NULL DEFAULT now(),
    -- The DICOM EventActionCode (PS3.15 §A.5.1).
    action                 text        NOT NULL,
    -- The DICOM EventOutcomeIndicator (PS3.15 §A.5.1).
    outcome                smallint    NOT NULL,
    -- The DICOM EventID csd-code (110110/110112/110106/110107/110114/110100).
    event_code             text        NOT NULL,
    -- The concrete operation: the ITS-REST operation id, or the DCM
    -- login/logout EventTypeCode csd-code for authentication records.
    operation              text,
    -- The authenticated principal (Basic username / OAuth sub); NULL when the
    -- caller was unauthenticated.
    principal              text,
    -- The resolved EHR subject (patient) id, when the touched resource is
    -- patient-centric and resolution succeeded.
    patient_id             text,
    -- The resource class the operation touched (the event model's
    -- ObjectClass, lower-case).
    resource_class         text        NOT NULL,
    -- The touched resource id (version uid / template id / party uid /
    -- qualified stored-query name).
    resource_id            text,
    -- The requesting network address.
    client_ip              text,
    -- The bearer token jti (IHE BALP OAUTHaccessTokenUse.Minimal) — never
    -- token contents.
    token_id               text,
    -- The resolved tenant of the audited request (informational; see the
    -- header note — deliberately no FK and no RLS).
    tenant_id              uuid,
    -- The full FHIR R4 AuditEvent (IHE BALP shape) — the canonical stored
    -- form, served verbatim by the ITI-81 search.
    fhir                   jsonb       NOT NULL,
    -- Forwarding outbox stamps (phase-D sinks): when each forwarding sink
    -- last delivered this record; NULL = pending for that sink.
    delivered_syslog_at    timestamptz,
    delivered_fhir_feed_at timestamptz,
    CONSTRAINT pk_audit_event PRIMARY KEY (id),
    CONSTRAINT ck_audit_event_action CHECK (action IN ('C', 'R', 'U', 'D', 'E')),
    CONSTRAINT ck_audit_event_outcome CHECK (outcome IN (0, 4, 8, 12))
);

COMMENT ON TABLE audit_event IS
    'The local IHE ATNA Audit Record Repository: one row per audit record, payload = the FHIR R4 AuditEvent per the IHE BALP content profiles. Strictly outside the EHR content (BASE architecture_overview/master07-security.adoc §Access logging); append-only except per-sink delivery stamps and retention reaping. NOT RLS-scoped: the node''s security log is an operator surface (see the migration header).';
COMMENT ON COLUMN audit_event.recorded_at IS 'The event time (FHIR AuditEvent.recorded / DICOM EventDateTime).';
COMMENT ON COLUMN audit_event.stored_at IS 'When the row was persisted (drain-side).';
COMMENT ON COLUMN audit_event.action IS 'DICOM EventActionCode (PS3.15 §A.5.1): C/R/U/D/E.';
COMMENT ON COLUMN audit_event.outcome IS 'DICOM EventOutcomeIndicator (PS3.15 §A.5.1): 0/4/8/12.';
COMMENT ON COLUMN audit_event.event_code IS 'DICOM EventID csd-code (PS3.15 §A.5.1).';
COMMENT ON COLUMN audit_event.operation IS 'The concrete operation: ITS-REST operation id, or the DCM login/logout code for authentication records.';
COMMENT ON COLUMN audit_event.principal IS 'The authenticated principal; NULL when unauthenticated.';
COMMENT ON COLUMN audit_event.patient_id IS 'The resolved EHR subject (patient) id, when patient-centric.';
COMMENT ON COLUMN audit_event.resource_class IS 'The touched resource class (the audit event model''s ObjectClass).';
COMMENT ON COLUMN audit_event.resource_id IS 'The touched resource id (version uid / template id / party uid / stored-query name).';
COMMENT ON COLUMN audit_event.client_ip IS 'The requesting network address.';
COMMENT ON COLUMN audit_event.token_id IS 'The bearer token jti (IHE BALP OAUTHaccessTokenUse.Minimal); never token contents.';
COMMENT ON COLUMN audit_event.tenant_id IS 'The audited request''s resolved tenant (informational; no FK, no RLS — see the table comment).';
COMMENT ON COLUMN audit_event.fhir IS 'The FHIR R4 AuditEvent document (IHE BALP shape) — the canonical stored form served by ITI-81.';
COMMENT ON COLUMN audit_event.delivered_syslog_at IS 'When the syslog forwarding sink delivered this record; NULL = pending.';
COMMENT ON COLUMN audit_event.delivered_fhir_feed_at IS 'When the FHIR-feed forwarding sink (ITI-20 ATX:FHIR Feed) delivered this record; NULL = pending.';

-- Browse/ITI-81 access is latest-first over a date window.
CREATE INDEX ix_audit_event_recorded_at ON audit_event (recorded_at DESC);
-- Patient-centric lookup ("who accessed THIS patient's data" — the openEHR
-- record-demerging use, BASE master07-security.adoc §Record demerging).
CREATE INDEX ix_audit_event_patient ON audit_event (patient_id, recorded_at DESC)
    WHERE patient_id IS NOT NULL;
-- Per-caller lookup.
CREATE INDEX ix_audit_event_principal ON audit_event (principal, recorded_at DESC)
    WHERE principal IS NOT NULL;
-- The forwarding outbox scans (phase-D sinks): pending rows only.
CREATE INDEX ix_audit_event_pending_syslog ON audit_event (stored_at)
    WHERE delivered_syslog_at IS NULL;
CREATE INDEX ix_audit_event_pending_fhir_feed ON audit_event (stored_at)
    WHERE delivered_fhir_feed_at IS NULL;
