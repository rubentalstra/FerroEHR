-- ehr schema baseline — the EHRbase v2 CDR schema (17 tables).
--
-- Created via `sqlx migrate add --source migrations/ehr --sequential baseline`.
-- Squashed end-state of the EHRbase v2.33.0 `ehr` Flyway chain (V1..V27),
-- derived by applying the vendored chain to PostgreSQL 18 and dumping the
-- result (ADR-007). The original chain is kept as a test fixture under
-- tests/resources/legacy_schema/ehr/; a schema-equality test asserts this
-- baseline produces the identical schema (constraint names, index
-- definitions, collations, storage options included).
--
-- Original SQL: Copyright (c) 2024 vitasystems GmbH, Apache License 2.0.
--
-- Deliberate deviation from the legacy end-state (see ADR-007):
--   * the orphaned `tenant_id_seq` sequence (left behind by the V5.x
--     multi-tenancy removal, referenced by nothing) is not recreated.
--
-- Runs with search_path = ehr, ext (see src/db/migrate.rs).
--
-- Layout notes preserved from the legacy schema:
--   * `COLLATE "en_US"` resolves to the pg_catalog collation (initdb-created
--     on the official postgres image); `COLLATE "C"` on entity_idx makes
--     lexicographic order equal tree order for the rm-db-format row model.
--   * `*_version_history.ov_data` holds the aggregated pre-V25 data rows;
--     toast_tuple_target + STORAGE MAIN tune its TOAST behaviour.

-- ── Enum types ───────────────────────────────────────────────────────────────

CREATE TYPE contribution_change_type AS ENUM (
    'creation',
    'amendment',
    'modification',
    'synthesis',
    'Unknown',
    'deleted'
);

CREATE TYPE contribution_data_type AS ENUM (
    'composition',
    'folder',
    'ehr',
    'system',
    'other'
);

CREATE TYPE ehr_item_tag_target_type AS ENUM (
    'ehr_status',
    'composition'
);

-- ── Root tables ──────────────────────────────────────────────────────────────

CREATE TABLE ehr (
    id            uuid NOT NULL,
    creation_date timestamp(6) with time zone,
    CONSTRAINT ehr_pkey PRIMARY KEY (id)
);

CREATE TABLE users (
    id       uuid NOT NULL,
    username text NOT NULL,
    CONSTRAINT users_pkey PRIMARY KEY (id)
);

CREATE TABLE audit_details (
    id             uuid NOT NULL,
    change_type    contribution_change_type NOT NULL,
    description    text,
    time_committed timestamp(6) with time zone NOT NULL,
    committer      jsonb,
    user_id        uuid NOT NULL,
    target_type    character varying NOT NULL,
    CONSTRAINT audit_details_pkey PRIMARY KEY (id)
);

CREATE TABLE contribution (
    id                uuid NOT NULL,
    ehr_id            uuid,
    contribution_type contribution_data_type,
    signature         text,
    has_audit         uuid,
    CONSTRAINT contribution_pkey PRIMARY KEY (id)
);

CREATE TABLE template_store (
    id             uuid NOT NULL,
    template_id    text NOT NULL,
    content        text,
    creation_time  timestamp(6) with time zone NOT NULL,
    concept        text,
    root_archetype text,
    CONSTRAINT template_store_pkey PRIMARY KEY (id)
);

CREATE TABLE stored_query (
    reverse_domain_name character varying NOT NULL,
    semantic_id         character varying NOT NULL,
    semver              character varying DEFAULT '0.0.0'::character varying NOT NULL,
    query_text          character varying NOT NULL,
    type                character varying DEFAULT 'AQL'::character varying,
    creation_date       timestamp(6) with time zone NOT NULL,
    CONSTRAINT stored_query_pkey PRIMARY KEY (reverse_domain_name, semantic_id, semver)
);

CREATE TABLE plugin (
    id       uuid DEFAULT ext.uuid_generate_v4() NOT NULL,
    pluginid text NOT NULL,
    key      text NOT NULL,
    value    text,
    CONSTRAINT plugin_pkey PRIMARY KEY (id)
);

COMMENT ON TABLE plugin IS 'key value store for plugin sub system';

-- ── Composition: version bookkeeping + row-per-locatable data ────────────────

CREATE TABLE comp_version (
    vo_id            uuid NOT NULL,
    ehr_id           uuid NOT NULL,
    contribution_id  uuid NOT NULL,
    audit_id         uuid NOT NULL,
    template_id      uuid NOT NULL,
    sys_version      integer NOT NULL,
    sys_period_lower timestamp with time zone NOT NULL,
    root_concept     text NOT NULL,
    CONSTRAINT comp_version_pkey PRIMARY KEY (vo_id)
);

CREATE TABLE comp_version_history (
    vo_id            uuid NOT NULL,
    ehr_id           uuid NOT NULL,
    contribution_id  uuid NOT NULL,
    audit_id         uuid NOT NULL,
    template_id      uuid NOT NULL,
    sys_version      integer NOT NULL,
    sys_period_lower timestamp with time zone NOT NULL,
    sys_period_upper timestamp with time zone,
    sys_deleted      boolean NOT NULL,
    ov_ref           integer,
    ov_data          text,
    CONSTRAINT comp_version_history_pkey PRIMARY KEY (vo_id, sys_version)
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY comp_version_history ALTER COLUMN ov_data SET STORAGE MAIN;

CREATE TABLE comp_data (
    vo_id            uuid CONSTRAINT comp_vo_id_not_null NOT NULL,
    num              integer CONSTRAINT comp_num_not_null NOT NULL,
    citem_num        integer,
    rm_entity        text CONSTRAINT comp_rm_entity_not_null NOT NULL,
    entity_concept   text,
    entity_name      text COLLATE "en_US",
    entity_attribute text,
    entity_idx       text CONSTRAINT comp_entity_idx_not_null NOT NULL COLLATE "C",
    entity_idx_len   integer CONSTRAINT comp_entity_idx_len_not_null NOT NULL,
    data             jsonb CONSTRAINT comp_data_not_null NOT NULL,
    parent_num       integer NOT NULL,
    num_cap          integer NOT NULL,
    CONSTRAINT comp_pkey PRIMARY KEY (vo_id, num)
);

-- ── EHR_STATUS: version bookkeeping + data ───────────────────────────────────

CREATE TABLE ehr_status_version (
    vo_id            uuid NOT NULL,
    ehr_id           uuid NOT NULL,
    contribution_id  uuid NOT NULL,
    audit_id         uuid NOT NULL,
    sys_version      integer NOT NULL,
    sys_period_lower timestamp with time zone NOT NULL,
    CONSTRAINT ehr_status_version_pkey PRIMARY KEY (ehr_id)
);

CREATE TABLE ehr_status_version_history (
    vo_id            uuid NOT NULL,
    ehr_id           uuid NOT NULL,
    contribution_id  uuid NOT NULL,
    audit_id         uuid NOT NULL,
    sys_version      integer NOT NULL,
    sys_period_lower timestamp with time zone NOT NULL,
    sys_period_upper timestamp with time zone,
    sys_deleted      boolean NOT NULL,
    ov_ref           integer,
    ov_data          text,
    CONSTRAINT ehr_status_version_history_pkey PRIMARY KEY (ehr_id, sys_version)
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY ehr_status_version_history ALTER COLUMN ov_data SET STORAGE MAIN;

CREATE TABLE ehr_status_data (
    vo_id            uuid CONSTRAINT ehr_status_vo_id_not_null NOT NULL,
    num              integer CONSTRAINT ehr_status_num_not_null NOT NULL,
    ehr_id           uuid CONSTRAINT ehr_status_ehr_id_not_null NOT NULL,
    citem_num        integer,
    rm_entity        text CONSTRAINT ehr_status_rm_entity_not_null NOT NULL,
    entity_concept   text,
    entity_name      text COLLATE "en_US",
    entity_attribute text,
    entity_idx       text CONSTRAINT ehr_status_entity_idx_not_null NOT NULL COLLATE "C",
    entity_idx_len   integer CONSTRAINT ehr_status_entity_idx_len_not_null NOT NULL,
    data             jsonb CONSTRAINT ehr_status_data_not_null NOT NULL,
    parent_num       integer NOT NULL,
    num_cap          integer NOT NULL,
    CONSTRAINT ehr_status_pkey PRIMARY KEY (ehr_id, num)
);

-- ── Directory (FOLDER): version bookkeeping + data ───────────────────────────

CREATE TABLE ehr_folder_version (
    vo_id            uuid NOT NULL,
    ehr_id           uuid NOT NULL,
    contribution_id  uuid NOT NULL,
    audit_id         uuid NOT NULL,
    sys_version      integer NOT NULL,
    sys_period_lower timestamp with time zone NOT NULL,
    ehr_folders_idx  integer NOT NULL,
    CONSTRAINT ehr_folder_version_pkey PRIMARY KEY (ehr_id, ehr_folders_idx)
);

CREATE TABLE ehr_folder_version_history (
    vo_id            uuid NOT NULL,
    ehr_id           uuid NOT NULL,
    contribution_id  uuid NOT NULL,
    audit_id         uuid NOT NULL,
    sys_version      integer NOT NULL,
    sys_period_lower timestamp with time zone NOT NULL,
    ehr_folders_idx  integer NOT NULL,
    sys_period_upper timestamp with time zone,
    sys_deleted      boolean NOT NULL,
    ov_item_uuids    uuid[],
    ov_ref           integer,
    ov_data          text,
    CONSTRAINT ehr_folder_version_history_pkey PRIMARY KEY (ehr_id, ehr_folders_idx, sys_version)
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY ehr_folder_version_history ALTER COLUMN ov_data SET STORAGE MAIN;

CREATE TABLE ehr_folder_data (
    vo_id            uuid CONSTRAINT ehr_folder_vo_id_not_null NOT NULL,
    num              integer CONSTRAINT ehr_folder_num_not_null NOT NULL,
    ehr_id           uuid CONSTRAINT ehr_folder_ehr_id_not_null NOT NULL,
    ehr_folders_idx  integer CONSTRAINT ehr_folder_ehr_folders_idx_not_null NOT NULL,
    citem_num        integer,
    rm_entity        text CONSTRAINT ehr_folder_rm_entity_not_null NOT NULL,
    entity_concept   text,
    entity_name      text COLLATE "en_US",
    entity_attribute text,
    entity_idx       text CONSTRAINT ehr_folder_entity_idx_not_null NOT NULL COLLATE "C",
    entity_idx_len   integer CONSTRAINT ehr_folder_entity_idx_len_not_null NOT NULL,
    data             jsonb CONSTRAINT ehr_folder_data_not_null NOT NULL,
    parent_num       integer NOT NULL,
    num_cap          integer NOT NULL,
    item_uuids       uuid[] NOT NULL,
    CONSTRAINT ehr_folder_pkey PRIMARY KEY (ehr_id, ehr_folders_idx, num)
);

-- ── Item tags (experimental API) ─────────────────────────────────────────────

CREATE TABLE ehr_item_tag (
    id               uuid NOT NULL,
    ehr_id           uuid NOT NULL,
    target_vo_id     uuid NOT NULL,
    target_type      ehr_item_tag_target_type NOT NULL,
    key              text NOT NULL COLLATE "C",
    value            text COLLATE "C",
    target_path      text COLLATE "C",
    creation_date    timestamp(6) with time zone NOT NULL,
    sys_period_lower timestamp(6) with time zone NOT NULL,
    CONSTRAINT ehr_item_tag_pkey PRIMARY KEY (id)
);

-- ── Indexes ──────────────────────────────────────────────────────────────────

CREATE INDEX comp_data_path_idx ON comp_data USING btree (vo_id, parent_num, entity_concept) INCLUDE (rm_entity, entity_attribute, entity_name, num, num_cap, citem_num, entity_idx);
CREATE INDEX comp_data_path_skip_idx ON comp_data USING btree (vo_id, citem_num, num) INCLUDE (entity_concept, rm_entity, entity_attribute, parent_num, num_cap, entity_idx);
CREATE INDEX comp_version_contribution_idx ON comp_version USING hash (contribution_id);
CREATE INDEX comp_version_ehr_idx ON comp_version USING btree (ehr_id, template_id) INCLUDE (vo_id, sys_version);
CREATE INDEX comp_version_history_contribution_idx ON comp_version_history USING hash (contribution_id);
CREATE INDEX comp_version_root_concept_idx ON comp_version USING btree (ehr_id, root_concept) INCLUDE (vo_id, sys_version);
CREATE INDEX comp_version_sys_period_lower_idx ON comp_version USING btree (sys_period_lower DESC, vo_id);
CREATE INDEX contribution_ehr_idx ON contribution USING btree (ehr_id);
CREATE INDEX ehr_folder_version_contribution_idx ON ehr_folder_version USING hash (contribution_id);
CREATE INDEX ehr_folder_version_history_contribution_idx ON ehr_folder_version_history USING hash (contribution_id);
CREATE UNIQUE INDEX ehr_folder_version_history_vo_id_sys_version_uniq ON ehr_folder_version_history USING btree (vo_id) WHERE (sys_version = 1);
CREATE UNIQUE INDEX ehr_folder_version_vo_id_idx ON ehr_folder_version USING btree (vo_id);
CREATE INDEX ehr_item_tag_ehr_id_target_vo_id_idx ON ehr_item_tag USING btree (ehr_id, target_vo_id);
CREATE INDEX ehr_status_data_path_idx ON ehr_status_data USING btree (ehr_id, parent_num, entity_attribute, entity_concept, rm_entity, num, num_cap);
CREATE UNIQUE INDEX ehr_status_subject_idx ON ehr_status_data USING btree (((((((data -> 'su'::text) -> 'er'::text) -> 'X'::text) -> 'V'::text) ->> 0)), (((((data -> 'su'::text) -> 'er'::text) -> 'ns'::text) ->> 0))) INCLUDE (ehr_id) WHERE (num = 0);
CREATE INDEX ehr_status_version_contribution_idx ON ehr_status_version USING hash (contribution_id);
CREATE INDEX ehr_status_version_history_contribution_idx ON ehr_status_version_history USING hash (contribution_id);
CREATE INDEX ehr_status_version_sys_period_lower_idx ON ehr_status_version USING btree (sys_period_lower DESC, ehr_id);
CREATE INDEX ehr_time_created_idx ON ehr USING btree (creation_date DESC, id);
CREATE UNIQUE INDEX template_store_id_unq ON template_store USING btree (template_id);
CREATE UNIQUE INDEX users_username_idx ON users USING btree (username);

-- ── Foreign keys ─────────────────────────────────────────────────────────────

ALTER TABLE ONLY audit_details
    ADD CONSTRAINT audit_details_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id);

ALTER TABLE ONLY comp_data
    ADD CONSTRAINT comp_data_vo_id_fkey FOREIGN KEY (vo_id) REFERENCES comp_version(vo_id) ON DELETE CASCADE;

ALTER TABLE ONLY comp_version
    ADD CONSTRAINT comp_version_audit_id_fkey FOREIGN KEY (audit_id) REFERENCES audit_details(id);
ALTER TABLE ONLY comp_version
    ADD CONSTRAINT comp_version_contribution_id_fkey FOREIGN KEY (contribution_id) REFERENCES contribution(id);
ALTER TABLE ONLY comp_version
    ADD CONSTRAINT comp_version_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);
ALTER TABLE ONLY comp_version
    ADD CONSTRAINT comp_version_template_id_fkey FOREIGN KEY (template_id) REFERENCES template_store(id);

ALTER TABLE ONLY comp_version_history
    ADD CONSTRAINT comp_version_history_audit_id_fkey FOREIGN KEY (audit_id) REFERENCES audit_details(id);
ALTER TABLE ONLY comp_version_history
    ADD CONSTRAINT comp_version_history_contribution_id_fkey FOREIGN KEY (contribution_id) REFERENCES contribution(id);
ALTER TABLE ONLY comp_version_history
    ADD CONSTRAINT comp_version_history_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);
ALTER TABLE ONLY comp_version_history
    ADD CONSTRAINT comp_version_history_template_id_fkey FOREIGN KEY (template_id) REFERENCES template_store(id);

ALTER TABLE ONLY contribution
    ADD CONSTRAINT contribution_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id) ON DELETE CASCADE;
ALTER TABLE ONLY contribution
    ADD CONSTRAINT contribution_has_audit_fkey FOREIGN KEY (has_audit) REFERENCES audit_details(id) ON DELETE CASCADE;

ALTER TABLE ONLY ehr_folder_data
    ADD CONSTRAINT ehr_folder_data_ehr_id_ehr_folders_idx_fkey FOREIGN KEY (ehr_id, ehr_folders_idx) REFERENCES ehr_folder_version(ehr_id, ehr_folders_idx) ON DELETE CASCADE;
ALTER TABLE ONLY ehr_folder_data
    ADD CONSTRAINT ehr_folder_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);

ALTER TABLE ONLY ehr_folder_version
    ADD CONSTRAINT ehr_folder_version_audit_id_fkey FOREIGN KEY (audit_id) REFERENCES audit_details(id);
ALTER TABLE ONLY ehr_folder_version
    ADD CONSTRAINT ehr_folder_version_contribution_id_fkey FOREIGN KEY (contribution_id) REFERENCES contribution(id);
ALTER TABLE ONLY ehr_folder_version
    ADD CONSTRAINT ehr_folder_version_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);

ALTER TABLE ONLY ehr_folder_version_history
    ADD CONSTRAINT ehr_folder_version_history_audit_id_fkey FOREIGN KEY (audit_id) REFERENCES audit_details(id);
ALTER TABLE ONLY ehr_folder_version_history
    ADD CONSTRAINT ehr_folder_version_history_contribution_id_fkey FOREIGN KEY (contribution_id) REFERENCES contribution(id);
ALTER TABLE ONLY ehr_folder_version_history
    ADD CONSTRAINT ehr_folder_version_history_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);

ALTER TABLE ONLY ehr_item_tag
    ADD CONSTRAINT ehr_item_tag_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);

ALTER TABLE ONLY ehr_status_data
    ADD CONSTRAINT ehr_status_data_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr_status_version(ehr_id) ON DELETE CASCADE;
ALTER TABLE ONLY ehr_status_data
    ADD CONSTRAINT ehr_status_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);

ALTER TABLE ONLY ehr_status_version
    ADD CONSTRAINT ehr_status_version_audit_id_fkey FOREIGN KEY (audit_id) REFERENCES audit_details(id);
ALTER TABLE ONLY ehr_status_version
    ADD CONSTRAINT ehr_status_version_contribution_id_fkey FOREIGN KEY (contribution_id) REFERENCES contribution(id);
ALTER TABLE ONLY ehr_status_version
    ADD CONSTRAINT ehr_status_version_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);

ALTER TABLE ONLY ehr_status_version_history
    ADD CONSTRAINT ehr_status_version_history_audit_id_fkey FOREIGN KEY (audit_id) REFERENCES audit_details(id);
ALTER TABLE ONLY ehr_status_version_history
    ADD CONSTRAINT ehr_status_version_history_contribution_id_fkey FOREIGN KEY (contribution_id) REFERENCES contribution(id);
ALTER TABLE ONLY ehr_status_version_history
    ADD CONSTRAINT ehr_status_version_history_ehr_id_fkey FOREIGN KEY (ehr_id) REFERENCES ehr(id);
