-- SM-2: ADL 1.4 source-archetype storage (I_DEFINITION_ADL14, native API).
--
-- ADL 1.4 archetypes are identified by their ARCHETYPE_ID (the older
-- human-readable id, e.g. `openEHR-EHR-COMPOSITION.prescription.v1`), not a
-- UUID (which the spec reserves for OPTs / ADL2 artefacts) — see
-- `docs/specs/openehr/SM/docs/openehr_platform/master04-definition_package.adoc`
-- ("In ADL 1.4, archetypes are identified with the older ARCHETYPE_ID").
--
-- The source ADL text is stored verbatim (the authoritative artefact, and what
-- `get_archetype` returns). `upload_archetype` replaces an existing id
-- (`I_DEFINITION_ADL14.upload_archetype`: "If an archetype with the same id
-- already exists, replace it") via ON CONFLICT DO UPDATE. OPTs stay in
-- `template_store` (keyed by UUID); this table is archetypes only.
CREATE TABLE archetype_store (
    archetype_id text PRIMARY KEY,
    adl          text NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now()
);
