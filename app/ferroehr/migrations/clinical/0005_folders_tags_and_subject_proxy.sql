-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- clinical: the EHR-scoped relations that stand OUTSIDE change control —
-- folder membership, item tags, and the Subject Proxy service store.
--
-- Each is mutable in place and carries no version chain, which is what keeps
-- them out of the change-control file. Runs with
-- search_path = clinical, ext, public.

-- ── ehr_folder ───────────────────────────────────────────────────────────────
-- One row per folder hierarchy of an EHR (RM ehr master04-ehr_package.adoc
-- §Folders: "at any time, an entirely new Folder hierarchy may be added, which
-- will be referenced by a new member of the EHR._folders_ attribute"). `rank`
-- is the EHR.folders list order (1-based); EHR.directory is the first LIVE
-- hierarchy (RM ehr §EHR Class Directory_in_folders: `folders /= Void implies
-- folders.item(1) = directory`). Ranks are append-only and never reused — a
-- deleted hierarchy keeps its slot. Each referenced hierarchy is its own
-- versioned object; this table records membership and order only, so vo_id
-- carries no foreign key (version is keyed per version, not per object) and a
-- service-wide UNIQUE stands in. Our own storage design.
CREATE TABLE ehr_folder (
    ehr_id uuid  NOT NULL REFERENCES ehr (id) ON DELETE CASCADE,
    rank   int   NOT NULL,
    vo_id  uuid  NOT NULL,
    CONSTRAINT pk_ehr_folder PRIMARY KEY (ehr_id, rank),
    CONSTRAINT uq_ehr_folder_vo UNIQUE (vo_id),
    CONSTRAINT ck_ehr_folder_rank_positive CHECK (rank >= 1)
);

COMMENT ON TABLE ehr_folder IS 'One row per folder hierarchy of an EHR (RM ehr master04 §Folders); rank order = EHR.folders order, EHR.directory = the first live hierarchy (RM ehr §EHR Class Directory_in_folders: folders.item(1) = directory). Ranks are append-only, never reused. No openEHR spec governs this table (our own storage design).';
COMMENT ON COLUMN ehr_folder.rank IS 'EHR.folders position (1-based, append-only). The lowest-rank LIVE hierarchy is EHR.directory (folders.item(1)).';
COMMENT ON COLUMN ehr_folder.vo_id IS 'The VERSIONED_FOLDER versioned-object id (a member of EHR.folders). FK-less (vo_version is keyed per version); UNIQUE service-wide instead.';

-- ── node ─────────────────────────────────────────────────────────────────────

-- ── item_tag ─────────────────────────────────────────────────────────────────
-- Item tags (RM common master07-tags.adoc ITEM_TAG; the ITS-REST tags routes).
-- Mutable, EHR-scoped, outside the version chain, requiring no contribution.
-- Our own storage design.
CREATE TABLE item_tag (
    id           uuid NOT NULL DEFAULT uuidv7(),
    ehr_id       uuid,
    -- The tagged target's uid. Deliberately FK-less: RM common master07
    -- ITEM_TAG.target "may be a VERSIONED_OBJECT<T> or a VERSION<T>", so it is
    -- outside the version chain by design and a key into `version` (which is
    -- keyed per version) would be wrong.
    target_vo_id uuid NOT NULL,
    -- The '{creating_system_id}::{version_tree_id}' tail of an addressed
    -- VERSION target, verbatim as addressed; NULL = the tag targets the
    -- VERSIONED_OBJECT container.
    target_version text,
    target_type  text NOT NULL,
    key          text NOT NULL,
    value        text,
    target_path  text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_item_tag PRIMARY KEY (id),
    -- ITEM_TAG identity: ITS-REST specifications/docs/overview/
    -- Requests_and_responses.md §openehr-item-tag — tags are "uniquely
    -- identified by their key and target_path pair", within one target
    -- (container or a specific VERSION). NULLS NOT DISTINCT so a NULL
    -- target_version or target_path participates in the identity.
    CONSTRAINT uq_item_tag_identity UNIQUE NULLS NOT DISTINCT
        (ehr_id, target_vo_id, target_version, key, target_path),
    -- The change-controlled resource types a tag may target. The ITS-REST
    -- wrapper headers are defined "for associating tags with change-controlled
    -- resources (e.g. COMPOSITION, EHR_STATUS, FOLDER, etc.)"
    -- (Requests_and_responses.md §openehr-item-tag and
    -- §openehr-version-item-tag).
    CONSTRAINT ck_item_tag_target_type CHECK (target_type IN (
        'COMPOSITION', 'EHR_STATUS', 'FOLDER',
        'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE'
    )),
    CONSTRAINT fk_item_tag_ehr FOREIGN KEY (ehr_id) REFERENCES ehr (id) ON DELETE CASCADE
);
-- No dedicated index on `key` alone. RM ehr master04-ehr_package.adoc §Tags
-- states the indexing OBLIGATION ("in a typical implementation, tags would be
-- indexed") but names no index and no access path, so the choice is ours.
-- uq_item_tag_identity already serves every addressed read the released wire
-- defines, and the one key-leading access — the EHR-wide listing's optional
-- tag_key filter — is scoped to one EHR by that index's leading column, with
-- PostgreSQL 18 B-tree skip scan covering the remainder
-- (https://www.postgresql.org/docs/18/indexes-multicolumn.html).

COMMENT ON TABLE item_tag IS 'Item tags (RM common master07-tags.adoc ITEM_TAG; identity per ITS-REST Requests_and_responses.md: key + target_path within one target). Mutable, EHR-scoped, outside the version chain.';
COMMENT ON COLUMN item_tag.target_vo_id IS 'The tagged object; intentionally FK-less, because RM common master07 lets ITEM_TAG.target name a container OR a specific VERSION.';
COMMENT ON COLUMN item_tag.target_version IS 'The {creating_system_id}::{version_tree_id} tail of a VERSION-addressed target; NULL = container target.';

-- ── Subject Proxy Service config stores (SM-6, I_SUBJECT_PROXY_SERVICE) ───────
-- CONFIGURATION only (SM openehr_platform master10-subject_proxy_service.adoc
-- §Persistence: "the configuration contents (i.e. not data frame or variable
-- results) of the SPS are persisted for the life of the system ... The SPS
-- includes a reset() operation that enables all content to be dumped"):
-- bindings + variable defs, cleared by reset(). Plain relational, not versioned
-- objects — no openEHR spec governs the storage form, our own design.

-- SUBJECT_PROXY: one proxy per subject.
CREATE TABLE sp_subject (
    subject_id       text NOT NULL,
    -- SUBJECT_PROXY.subject_category (free string; "not controlled" in the SM).
    subject_category text NOT NULL DEFAULT 'individual',
    create_time      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_sp_subject PRIMARY KEY (subject_id)
);
COMMENT ON TABLE sp_subject IS 'SUBJECT_PROXY config: one proxy per subject (SM openehr_platform master10-subject_proxy_service.adoc §Persistence — configuration only, cleared by reset()).';

-- ENV_BINDING: one binding per execution environment.
CREATE TABLE sp_binding (
    env_id      text NOT NULL,
    description text,
    CONSTRAINT pk_sp_binding PRIMARY KEY (env_id)
);
COMMENT ON TABLE sp_binding IS 'SM-6 ENV_BINDING config: one binding per execution environment.';

-- DATA_FRAME: a retrieval frame within a binding. frame_id is referenced
-- globally (SUBJECT_VARIABLE.frame_id, I_DATA_BINDING.get_frame which takes no
-- env_id), so UNIQUE across all bindings — it addresses one frame service-wide.
CREATE TABLE sp_data_frame (
    env_id          text NOT NULL,
    frame_id        text NOT NULL,
    model_type      text NOT NULL,
    -- canonical JSON of the FrameMethod (QUERY_CALL AQL text for openEHR;
    -- FHIR/HL7v2 descriptors for the stubbed seams).
    primary_method  jsonb NOT NULL,
    fallback_method jsonb,
    CONSTRAINT pk_sp_data_frame PRIMARY KEY (env_id, frame_id),
    CONSTRAINT uq_sp_data_frame_frame_id UNIQUE (frame_id),
    CONSTRAINT fk_sp_data_frame_binding FOREIGN KEY (env_id) REFERENCES sp_binding (env_id) ON DELETE CASCADE
);
COMMENT ON TABLE sp_data_frame IS 'SM-6 DATA_FRAME config: a retrieval frame within a binding. frame_id is UNIQUE service-wide (get_frame takes no env_id).';

-- SUBJECT_VARIABLE attached to a subject's proxy, keyed by canonical_name.
CREATE TABLE sp_variable (
    subject_id     text NOT NULL,
    canonical_name text NOT NULL,
    namespace      text,
    name           text NOT NULL,
    type_name      text NOT NULL,
    -- currency: Iso8601_duration (unset ⇒ most recent available valid).
    currency       text,
    ask_user       boolean,
    is_manual      boolean NOT NULL DEFAULT false,
    frame_id       text NOT NULL,
    frame_path     text NOT NULL,
    CONSTRAINT pk_sp_variable PRIMARY KEY (subject_id, canonical_name),
    CONSTRAINT fk_sp_variable_subject FOREIGN KEY (subject_id) REFERENCES sp_subject (subject_id) ON DELETE CASCADE,
    -- A variable binds a frame that must exist (referential integrity — no
    -- spec governs the storage).
    CONSTRAINT fk_sp_variable_frame FOREIGN KEY (frame_id) REFERENCES sp_data_frame (frame_id)
);
CREATE INDEX idx_sp_variable_frame ON sp_variable (frame_id);
COMMENT ON TABLE sp_variable IS 'SM-6 SUBJECT_VARIABLE config (SM subject_proxy_service), keyed by canonical_name; frame_id FK into sp_data_frame.';

-- SUBJECT_DATA_SET: a set of variables registered for a subject by an
-- application. The variable set (data-set-local name → SUBJECT_VARIABLE) is
-- stored verbatim as canonical JSON (the local aliases differ from canonical names).
CREATE TABLE sp_data_set (
    subject_id      text NOT NULL,
    id              text NOT NULL,
    creating_app_id text,
    using_app_ids   jsonb NOT NULL DEFAULT '[]'::jsonb,
    variables       jsonb NOT NULL,
    CONSTRAINT pk_sp_data_set PRIMARY KEY (subject_id, id),
    CONSTRAINT fk_sp_data_set_subject FOREIGN KEY (subject_id) REFERENCES sp_subject (subject_id) ON DELETE CASCADE
);
-- remove_application(application_id) / has_application scan by creating app.
CREATE INDEX idx_sp_data_set_creating_app ON sp_data_set (creating_app_id)
    WHERE creating_app_id IS NOT NULL;
COMMENT ON TABLE sp_data_set IS 'SM-6 SUBJECT_DATA_SET config: variables registered for a subject by an application (verbatim canonical JSON).';

-- SAMPLE store: the retrieve history of each SUBJECT_VARIABLE. "Every retrieval
-- attempt will generate a new Sample object, regardless of whether data was
-- actually available or not" (master10 §Samples / SAMPLE class); the rows
-- realize SUBJECT_VARIABLE.history + last_frame and, via effective_time, the
-- currency/freshness decision (master10 §Samples: effective_time "is comparable
-- to currency in order to determine the freshness of the data"). master10
-- §Persistence requires only configuration to survive re-initialisation and does
-- not forbid persisting samples — keeping them is what makes "tracked over time"
-- real across restarts; reset() truncates this table too. No openEHR spec governs
-- the storage mechanics — our own design.
CREATE TABLE sp_sample (
    id             uuid NOT NULL DEFAULT uuidv7(),
    subject_id     text NOT NULL,
    canonical_name text NOT NULL,
    -- frame_id of the producing DATA_FRAME (NULL for a manually-notified sample).
    frame_id       text,
    retrieve_time  timestamptz NOT NULL DEFAULT now(),
    effective_time timestamptz,
    is_unavailable boolean NOT NULL,
    -- the VARIABLE_SAMPLE canonical JSON (always) …
    sample         jsonb NOT NULL,
    -- … and the producing DATA_FRAME_SAMPLE canonical JSON (frame-driven only).
    frame_sample   jsonb,
    CONSTRAINT pk_sp_sample PRIMARY KEY (id),
    CONSTRAINT fk_sp_sample_variable FOREIGN KEY (subject_id, canonical_name)
        REFERENCES sp_variable (subject_id, canonical_name) ON DELETE CASCADE
);
-- Freshness + history reads are newest-first per variable.
CREATE INDEX idx_sp_sample_variable ON sp_sample (subject_id, canonical_name, retrieve_time DESC);
COMMENT ON TABLE sp_sample IS 'SM-6 SAMPLE store: retrieve history per SUBJECT_VARIABLE (master10 §Samples, §Persistence); realizes history/last_frame + currency freshness.';

