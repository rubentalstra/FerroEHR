-- SM-2: ADL2 artefact storage (I_DEFINITION_ADL2, native API).
--
-- In ADL2, archetypes and 'templates' are ALL instances of archetypes:
-- source archetypes, templates, and Operational Templates (OPTs) can all be
-- uploaded, and are "identified in the same way, via an Archetype
-- human-readable identifier (ARCHETYPE_HRID) and a UUID" — see
-- `docs/specs/openehr/SM/docs/openehr_platform/master04-definition_package.adoc`
-- (§Archetypes and Templates). One unified table therefore holds every ADL2
-- artefact, keyed by its ARCHETYPE_HRID (e.g.
-- `openEHR-EHR-OBSERVATION.bp.v1.0.0`, optionally namespace-qualified), with
-- `kind` discriminating archetype / template / operational_template so the
-- per-concrete-type list/count calls (list_archetypes / list_templates /
-- list_opts, archetypes_count / …) can filter.
--
-- The source ADL2 text is stored verbatim (the authoritative artefact, and
-- what `get_artefact` returns). `upload_artefact` replaces an existing HRID
-- (I_DEFINITION_ADL2.upload_artefact: "If an artefact with the same physical
-- identifier and namespace exists, replace it") via ON CONFLICT DO UPDATE.
--
-- PORT NOTE: the SM exchanges AOM2 AUTHORED_ARCHETYPE objects; the native API
-- exchanges ADL2 source text (there is no ADL2/cADL source parser in the tree
-- yet — am24 is generated AOM2 types only), so `kind` + `hrid` are extracted
-- structurally from the source header. Full AOM2 modelling lands with the
-- parser.
CREATE TABLE adl2_artefact (
    hrid       text PRIMARY KEY,
    kind       text NOT NULL CHECK (kind IN ('archetype', 'template', 'operational_template')),
    adl        text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
