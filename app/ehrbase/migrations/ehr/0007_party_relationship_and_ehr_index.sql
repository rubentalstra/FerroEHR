-- SM-3: PARTY_RELATIONSHIP versioned-object storage + the EHR Index.
--
-- PARTY_RELATIONSHIP (RM demographic) is a versioned object with NO EHR scope,
-- exactly like the demographic parties (0003): it lives in the shared
-- `vo_version` + `node` tables with a NULL `ehr_id` (the demographics
-- repository is not owned by any EHR). The SM `i_party_relationship.adoc` +
-- `i_demographic_service.adoc` (create_party_relationship) drive it; the wire
-- is our own design by analogy with the party group (ADR-008, no ITS-REST
-- contract).
--
-- The EHR Index (`i_ehr_index.adoc`, master07) records N:M associations of
-- subject identifiers (OBJECT_REF) with EHR ids, plus optional RESOURCE_STATUS
-- (instance type + validity period + notes) and LOCATION_DESC. Index entries
-- are NOT versioned objects (the SM defines no versioning here — design 08
-- §4.1), so this is plain relational storage, not the `vo_version` machinery.

-- 1. `vo_version.kind` may now discriminate a PARTY_RELATIONSHIP (mirrors 0003's
--    mechanism: drop + re-add the auto-named CHECK with the new member).
ALTER TABLE vo_version DROP CONSTRAINT vo_version_kind_check;
ALTER TABLE vo_version ADD CONSTRAINT vo_version_kind_check
    CHECK (kind IN (
        'COMPOSITION', 'EHR_STATUS', 'EHR_ACCESS', 'FOLDER',
        'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE',
        'PARTY_RELATIONSHIP'
    ));

-- 2. The EHR Index: N:M subject↔EHR associations with duplicate-management
--    metadata. PK (ehr_id, subject_id, subject_namespace) is one association;
--    the same subject may associate with many EHRs and vice versa.
CREATE TABLE ehr_index (
    ehr_id            uuid NOT NULL REFERENCES ehr(id) ON DELETE CASCADE,
    subject_id        text NOT NULL,
    subject_namespace text NOT NULL,
    -- OBJECT_REF.type of the subject (defaults to PERSON — the common MPI case).
    subject_type      text NOT NULL DEFAULT 'PERSON',
    -- RESOURCE_INSTANCE_TYPE (resource_instance_type.adoc): Primary is the
    -- authoritative association; Duplicate/Supplementary flag the N:M error
    -- states master07 wants surfaced.
    instance_type     text NOT NULL DEFAULT 'Primary'
                       CHECK (instance_type IN ('Primary', 'Duplicate', 'Supplementary')),
    -- RESOURCE_STATUS.start/end_valid_time (typed `@@` placeholder in the SM —
    -- implemented as ISO date-time / timestamptz, PORT NOTE in the service).
    start_valid_time  timestamptz,
    end_valid_time    timestamptz,
    -- RESOURCE_STATUS.notes.
    notes             text,
    -- LOCATION_DESC (empty stub in the SM — designed contract
    -- {system_id, uri?, description?}, stored as canonical JSON; PORT NOTE).
    location          jsonb,
    created_at        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (ehr_id, subject_id, subject_namespace)
);
-- subject → EHRs lookup (remove_subject / subject_ehrs).
CREATE INDEX ehr_index_subject_idx ON ehr_index (subject_id, subject_namespace);
