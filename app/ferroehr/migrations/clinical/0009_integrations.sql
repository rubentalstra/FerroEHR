-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- clinical: the optional integrations' stores — the FHIR mapping registry and
-- the multimedia blob reference index.
--
-- Both are extensions with no openEHR spec behind them, kept in the clinical
-- schema because their rows are EHR-scoped and reached through the clinical
-- pool.
--
-- Where the FHIR R4 surface of a deployment lives — this product's `fhir`
-- feature or FerroBRIDGE's facade — is the owner decision #3386 records.
-- `fhir_mapping` is the mapping registry that surface reads.
-- TODO(#3386): fhir_mapping leaves with the FHIR facade if the owner's
-- decision moves it to FerroBRIDGE.
--
-- Runs with search_path = clinical, ext, public.

CREATE TABLE fhir_mapping (
    id            uuid NOT NULL DEFAULT uuidv7(),
    -- The stable, addressable deployable identity.
    name          text NOT NULL,
    -- The FHIR resource type this mapping consumes; the inbound router
    -- resolves POST /fhir/r4/{resourceType} by this plus the profile.
    resource_type text NOT NULL,
    -- The FHIR profile canonical URL matched against the resource's
    -- meta.profile; NULL = the default mapping for the resource type.
    profile_url   text,
    -- The openEHR template the built COMPOSITION targets.
    template_id   text NOT NULL,
    -- The mapping definition: the field bindings, code-system translations and
    -- subject/context rules. Validated on upload, stored verbatim so it
    -- round-trips.
    definition    jsonb NOT NULL,
    enabled       boolean NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_fhir_mapping PRIMARY KEY (id),
    CONSTRAINT uq_fhir_mapping_name UNIQUE (name),
    CONSTRAINT fk_fhir_mapping_template FOREIGN KEY (template_id)
        REFERENCES template_store (template_id)
);

CREATE INDEX idx_fhir_mapping_resolve ON fhir_mapping (resource_type, profile_url)
    WHERE enabled;

COMMENT ON TABLE fhir_mapping IS 'FHIR-connector mapping store: deployable artefacts binding one openEHR template to one FHIR resource profile. Our own extension — no openEHR spec governs FHIR interop. Moving to FerroBRIDGE with the rest of the FHIR facade.';
COMMENT ON COLUMN fhir_mapping.definition IS 'The mapping definition (field bindings, code-system translations, subject/context rules), validated on upload and stored verbatim.';

-- ── blob_ref ─────────────────────────────────────────────────────────────────
-- Which stored version references which content-addressed multimedia blob.
--
-- DV_MULTIMEDIA may carry its content by reference (RM data_types
-- master06-quantity_package.adoc is not its home; the class is
-- UML/classes/org.openehr.rm.data_types.dv_multimedia.adoc, whose `uri` is an
-- external reference), and the multimedia store holds the bytes outside the
-- database. Garbage collection then has to answer "is this blob still
-- referenced", which today scans every node row. This index answers it as an
-- anti-join instead.
--
-- DDL only in this change; the rows are not yet maintained.
-- TODO(#3347): populate blob_ref at commit and make the blob garbage
-- collection an anti-join over it rather than a scan of node.
CREATE TABLE blob_ref (
    vo_id       uuid NOT NULL,
    sys_version integer NOT NULL,
    ehr_id      uuid,
    -- The content address of the referenced blob, as the multimedia store
    -- spells it.
    digest      text NOT NULL,
    media_type  text,
    size_bytes  bigint,
    CONSTRAINT pk_blob_ref PRIMARY KEY (vo_id, sys_version, digest),
    CONSTRAINT ck_blob_ref_size CHECK (size_bytes IS NULL OR size_bytes >= 0)
);

-- The garbage collector's question: does any version still reference this
-- blob.
CREATE INDEX idx_blob_ref_digest ON blob_ref (digest);

COMMENT ON TABLE blob_ref IS 'Which stored version references which content-addressed multimedia blob, so blob garbage collection is an anti-join rather than a scan of node. Our own extension.';
COMMENT ON COLUMN blob_ref.digest IS 'The content address of the referenced blob, as the multimedia store spells it.';
