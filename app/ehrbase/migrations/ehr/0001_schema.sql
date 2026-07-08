-- ehr schema: the greenfield PG18-native CDR schema (ADR-008, validated by
-- the P10 storage spike — see docs/plans/phase-10-rm-db-format.md).
--
-- Created via `sqlx migrate add --source migrations/ehr --sequential`.
-- Design points:
--   * one unified `node` table for all versioned-object content, stored
--     PER VERSION (ALL_VERSIONS queries one table uniformly), with a
--     nested-set index (num/num_cap/parent_num) so AQL CONTAINS is an
--     integer interval join;
--   * node.data holds the node's CANONICAL openEHR JSON fragment verbatim
--     (structure children pruned) — no alias compaction, no synthetic
--     fields; fragments average ~360 B (spike), well under TOAST;
--   * one temporal `vo_version` table (PG18 WITHOUT OVERLAPS, needs
--     btree_gist — created by the bootstrap) instead of current/history
--     pairs; the current version is the upper_inf partial index;
--   * every write emits a contribution + audit row (openEHR requirement);
--   * uuidv7() (PG18) for generated keys — time-ordered, index-friendly.
-- Runs with search_path = ehr, ext.

CREATE TABLE ehr (
    id                uuid PRIMARY KEY,
    time_created      timestamptz NOT NULL DEFAULT now(),
    -- Promoted copy of the current EHR_STATUS `subject.external_ref`
    -- (`id.value` + `namespace`), kept in sync by the service on every
    -- EHR_STATUS write. The partial unique index enforces one EHR per subject
    -- at the database (ITS-REST `409_EHR.yaml`; CNF master06
    -- `I_EHR_SERVICE.create_ehr-two_ehrs_same_patient`).
    subject_id        text,
    subject_namespace text
);
CREATE INDEX ehr_time_created_idx ON ehr (time_created DESC, id);
CREATE UNIQUE INDEX ehr_subject_uq ON ehr (subject_id, subject_namespace)
    WHERE subject_id IS NOT NULL;

-- AUDIT_DETAILS of every committed change (queryable via AQL VERSION paths)
CREATE TABLE audit (
    id             uuid PRIMARY KEY DEFAULT uuidv7(),
    time_committed timestamptz NOT NULL DEFAULT now(),
    system_id      text NOT NULL,
    change_type    text NOT NULL,   -- openEHR audit change type code string
    description    text,
    committer      jsonb NOT NULL   -- canonical PARTY_PROXY
);

-- CONTRIBUTION: the change-set envelope
CREATE TABLE contribution (
    id       uuid PRIMARY KEY DEFAULT uuidv7(),
    ehr_id   uuid NOT NULL REFERENCES ehr(id) ON DELETE CASCADE,
    audit_id uuid NOT NULL REFERENCES audit(id)
);
CREATE INDEX contribution_ehr_idx ON contribution (ehr_id);

-- operational templates (OPT 1.4 XML; parsed model is built at P13/P14)
CREATE TABLE template_store (
    id             uuid PRIMARY KEY DEFAULT uuidv7(),
    template_id    text NOT NULL UNIQUE,
    concept        text,
    root_archetype text,
    content        text NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now()
);

-- one row per version of a versioned object (COMPOSITION/EHR_STATUS/
-- EHR_ACCESS/FOLDER); the temporal PK makes overlapping validity impossible
-- at the database. EHR_ACCESS is a versioned object per RM ehr §"EHR
-- Creation" ("a root EHR object, an EHR Status object, and an EHR Access
-- object"), versioned "via the normal mechanism" (RM ehr §"EHR Access").
CREATE TABLE vo_version (
    vo_id           uuid NOT NULL,
    kind            text NOT NULL CHECK (kind IN ('COMPOSITION', 'EHR_STATUS', 'EHR_ACCESS', 'FOLDER')),
    ehr_id          uuid NOT NULL REFERENCES ehr(id) ON DELETE CASCADE,
    sys_version     integer NOT NULL CHECK (sys_version >= 1),
    sys_period      tstzrange NOT NULL,
    -- ORIGINAL_VERSION.lifecycle_state: the numeric openEHR terminology code
    -- from the `version_lifecycle_state` group (532 complete, 553 incomplete,
    -- 523 deleted, 800 inactive, 801 abandoned). A logical delete writes a
    -- content-less version whose lifecycle_state is 523 (RM change_control
    -- §"Logical Deletion") — never a physical delete.
    lifecycle_state text NOT NULL DEFAULT '532'
                    CHECK (lifecycle_state IN ('532', '553', '523', '800', '801')),
    contribution_id uuid NOT NULL REFERENCES contribution(id),
    audit_id        uuid NOT NULL REFERENCES audit(id),
    template_id     text REFERENCES template_store(template_id),
    PRIMARY KEY (vo_id, sys_period WITHOUT OVERLAPS),
    UNIQUE (vo_id, sys_version)
);
CREATE UNIQUE INDEX vo_version_current_idx ON vo_version (vo_id) WHERE upper_inf(sys_period);
CREATE INDEX vo_version_ehr_idx ON vo_version (ehr_id, kind);
CREATE INDEX vo_version_contribution_idx ON vo_version (contribution_id);
CREATE INDEX vo_version_template_idx ON vo_version (template_id) WHERE template_id IS NOT NULL;

-- the decomposed content: one row per RM structure node, per version
CREATE TABLE node (
    vo_id       uuid NOT NULL,
    sys_version integer NOT NULL,
    num         integer NOT NULL,
    num_cap     integer NOT NULL,
    parent_num  integer NOT NULL,
    citem_num   integer,
    ehr_id      uuid NOT NULL,
    rm_type     text NOT NULL,
    archetype   text,
    name        text,
    path        text COLLATE "C" NOT NULL,
    data        jsonb NOT NULL,
    PRIMARY KEY (vo_id, sys_version, num),
    FOREIGN KEY (vo_id, sys_version) REFERENCES vo_version (vo_id, sys_version) ON DELETE CASCADE
);
CREATE INDEX node_type_archetype_idx ON node (rm_type, archetype);
CREATE INDEX node_ehr_idx ON node (ehr_id);
-- jsonb_ops (NOT jsonb_path_ops): $.** equality anchors need it (ADR-008)
CREATE INDEX node_data_gin ON node USING gin (data jsonb_ops);

-- stored AQL queries (semver-addressed per ITS-REST DEFINITION API)
CREATE TABLE stored_query (
    reverse_domain_name text NOT NULL,
    semantic_id         text NOT NULL,
    semver              text NOT NULL DEFAULT '0.0.0',
    query_type          text NOT NULL DEFAULT 'AQL',
    query_text          text NOT NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (reverse_domain_name, semantic_id, semver)
);

-- item tags (ITS-REST experimental tags API)
CREATE TABLE item_tag (
    id           uuid PRIMARY KEY DEFAULT uuidv7(),
    ehr_id       uuid NOT NULL REFERENCES ehr(id) ON DELETE CASCADE,
    target_vo_id uuid NOT NULL,
    target_type  text NOT NULL CHECK (target_type IN ('COMPOSITION', 'EHR_STATUS')),
    key          text NOT NULL,
    value        text,
    target_path  text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (ehr_id, target_vo_id, key)
);
