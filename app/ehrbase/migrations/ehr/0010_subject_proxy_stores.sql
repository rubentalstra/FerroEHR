-- SM-6 Subject Proxy Service (SPS) configuration stores.
--
-- Spec: SM master10 (Subject Proxy Service) — subject proxies, variables,
-- application data sets and environment bindings. master10 §Persistence: "the
-- configuration contents (i.e. not data frame or variable results) of the SPS
-- are persisted for the life of the system", cleared by reset(). These are
-- therefore plain relational configuration tables — NOT versioned objects, so
-- none of the `vo_version`/`node` machinery applies. Sample history / last-frame
-- caching (the design 08 §4.4 `sp_sample` store) is deferred: get_variable
-- re-executes the bound frame each call.
--
-- Design: docs/design/sm-platform/08-target-architecture.md §4.4;
-- docs/design/sm-platform/04-message-subject-proxy-terminology-admin.md §2.

-- SUBJECT_PROXY (subject_proxy.adoc): one proxy per subject.
CREATE TABLE sp_subject (
    subject_id       text PRIMARY KEY,
    -- SUBJECT_PROXY.subject_category (default when register_subject omits it;
    -- "TODO: currently not controlled" in the spec, so a free string).
    subject_category text NOT NULL DEFAULT 'individual',
    create_time      timestamptz NOT NULL DEFAULT now()
);

-- ENV_BINDING (env_binding.adoc): one binding per execution environment.
CREATE TABLE sp_binding (
    env_id      text PRIMARY KEY,
    description text
);

-- DATA_FRAME (data_frame.adoc): a retrieval frame within a binding. frame_id is
-- referenced globally by SUBJECT_VARIABLE.frame_id and by I_DATA_BINDING.
-- get_frame(subject_id, frame_id) (which takes no env_id), so it is UNIQUE
-- across all bindings — the frame_id addresses one frame service-wide (the SM's
-- open "SPS is 1:1 with an environment?" TODO; PORT NOTE in the service).
CREATE TABLE sp_data_frame (
    env_id          text NOT NULL REFERENCES sp_binding(env_id) ON DELETE CASCADE,
    frame_id        text NOT NULL,
    model_type      text NOT NULL,
    -- canonical JSON of the FrameMethod (QUERY_CALL AQL text for the openEHR
    -- frame; FHIR/HL7v2 descriptors for the stubbed seams).
    primary_method  jsonb NOT NULL,
    fallback_method jsonb,
    PRIMARY KEY (env_id, frame_id),
    UNIQUE (frame_id)
);

-- SUBJECT_VARIABLE (subject_variable.adoc) attached to a subject's proxy, keyed
-- by canonical_name (namespace::name, or name) — unique per subject.
CREATE TABLE sp_variable (
    subject_id     text NOT NULL REFERENCES sp_subject(subject_id) ON DELETE CASCADE,
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
    PRIMARY KEY (subject_id, canonical_name)
);

-- SUBJECT_DATA_SET (subject_data_set.adoc): a set of variables registered for a
-- subject by an application, keyed by (subject_id, id). The variable set (keyed
-- by *local* name → full SUBJECT_VARIABLE) is stored verbatim as canonical JSON
-- because the data-set-local aliases differ from the canonical variable names.
CREATE TABLE sp_data_set (
    subject_id      text NOT NULL REFERENCES sp_subject(subject_id) ON DELETE CASCADE,
    id              text NOT NULL,
    creating_app_id text,
    using_app_ids   jsonb NOT NULL DEFAULT '[]'::jsonb,
    variables       jsonb NOT NULL,
    PRIMARY KEY (subject_id, id)
);
-- remove_application(application_id) / has_application scan by creating app.
CREATE INDEX sp_data_set_creating_app_idx ON sp_data_set (creating_app_id)
    WHERE creating_app_id IS NOT NULL;
