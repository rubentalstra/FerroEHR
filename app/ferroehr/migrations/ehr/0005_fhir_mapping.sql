-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ehr schema: the FHIR-connector mapping store (an extension — no openEHR
-- spec governs FHIR interop; E3 — FHIR R4
-- connectors, "mapping-as-data").
--
-- Append-only on the baseline (0001) + eventing (0002/0003) +
-- multi-tenancy (0004). A FHIR mapping is a versioned, deployable data artefact
-- (uploaded/validated like a template) binding one openEHR
-- template ↔ one FHIR resource profile: its `definition` JSON carries the
-- field-path bindings (FHIRPath-lite → simplified openEHR flat paths) the
-- inbound connector uses to build a COMPOSITION from an incoming FHIR resource,
-- which then commits through the NORMAL validated path (never a bypass
-- §Decision 3). Mapping CRUD is a config-gated admin extension surface
-- (`ferroehr-rest`, off by default), like the event-subscription/terminology
-- groups. Runs with search_path = ehr, ext.
--
-- Baseline discipline: named constraints (pk_/uq_/fk_), COMMENT ON everything,
-- role-guarded grants. Tenant-scoped like its siblings: tenant_id
-- DEFAULT ext.current_tenant_id() + ENABLE/FORCE RLS + the tenant_isolation
-- policy, so a single-tenant deployment (GUC unset) is byte-identical to
-- pre-tenancy behaviour and a multi-tenant one isolates mappings per tenant.

-- ── fhir_mapping ───────────────────────────────────────
CREATE TABLE fhir_mapping (
    -- Stable mapping identity (uuidv7, PG18): time-ordered, index-friendly, the
    -- addressable id of the admin CRUD surface.
    id            uuid NOT NULL DEFAULT uuidv7(),
    -- The human-chosen mapping name. UNIQUE so a mapping is addressable/replaceable
    -- by a stable name across deployments (the "deployable data" identity).
    name          text NOT NULL,
    -- The FHIR resource type this mapping consumes (e.g. Observation, Patient,
    -- Condition, DocumentReference — the starter set). The
    -- inbound router resolves a POST /fhir/r4/{resourceType} by this + profile.
    resource_type text NOT NULL,
    -- The FHIR profile canonical URL this mapping binds (matched against the
    -- resource's meta.profile). NULL = the default mapping for the resource type
    -- (used when the resource declares no matching profile).
    profile_url   text,
    -- The openEHR template (OPT) the built COMPOSITION targets. FK → the OPT's
    -- wire address template_store.template_id (as vo_version does), so a mapping
    -- cannot reference an un-ingested template.
    template_id   text NOT NULL,
    -- The mapping definition: the FHIRPath-lite → openEHR
    -- flat-path field bindings, code-system translations, and subject/context
    -- rules. Validated on upload (deserialised into the connector's definition
    -- schema); stored verbatim so it round-trips.
    definition    jsonb NOT NULL,
    -- Whether the mapping is active: the inbound resolver considers only enabled
    -- mappings. Disabled = retained but not applied.
    enabled       boolean NOT NULL DEFAULT true,
    -- Creation instant (audit/ordering).
    created_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_fhir_mapping PRIMARY KEY (id),
    CONSTRAINT uq_fhir_mapping_name UNIQUE (name),
    CONSTRAINT fk_fhir_mapping_template FOREIGN KEY (template_id)
        REFERENCES template_store (template_id)
);

-- The inbound resolver's lookup: enabled mappings for a resource type, chosen by
-- profile (exact profile_url, else the NULL-profile default).
CREATE INDEX idx_fhir_mapping_resolve ON fhir_mapping (resource_type, profile_url)
    WHERE enabled;

COMMENT ON TABLE fhir_mapping IS 'FHIR-connector mapping store (an extension — no openEHR spec governs FHIR interop; "mapping-as-data"): versioned, deployable artefacts binding one openEHR template ↔ one FHIR resource profile. The inbound connector resolves a mapping by resource_type + meta.profile, builds a COMPOSITION from its definition, and commits it through the normal validated path with FEEDER_AUDIT provenance. Config-gated admin CRUD (ferroehr-rest, off by default).';
COMMENT ON COLUMN fhir_mapping.name IS 'Mapping name; the stable, UNIQUE deployable identity.';
COMMENT ON COLUMN fhir_mapping.resource_type IS 'FHIR resource type consumed (Observation/Patient/Condition/DocumentReference — the starter set). The POST /fhir/r4/{resourceType} router keys on this.';
COMMENT ON COLUMN fhir_mapping.profile_url IS 'FHIR profile canonical URL bound (matched against meta.profile). NULL = the default mapping for the resource type.';
COMMENT ON COLUMN fhir_mapping.template_id IS 'Target openEHR template. FK → template_store.template_id (the OPT wire address): a mapping cannot reference an un-ingested template.';
COMMENT ON COLUMN fhir_mapping.definition IS 'The mapping definition: FHIRPath-lite → openEHR flat-path bindings + code-system translations + subject/context rules. Validated on upload, stored verbatim.';
COMMENT ON COLUMN fhir_mapping.enabled IS 'Whether the mapping is applied by the inbound resolver. Disabled rows are retained but not applied.';

-- ── tenant scoping + RLS FORCE ──────────────────────────────────
-- New scoping table added after the 0004 loop, so it carries its own tenant_id
-- column + ENABLE/FORCE RLS + the tenant_isolation policy (same shape as 0004).
ALTER TABLE fhir_mapping ADD COLUMN tenant_id uuid NOT NULL
    DEFAULT ext.current_tenant_id()
    CONSTRAINT fk_fhir_mapping_tenant REFERENCES tenant (id);
COMMENT ON COLUMN fhir_mapping.tenant_id IS 'Owning tenant (FK → tenant; multi-tenancy is an extension). DEFAULT ext.current_tenant_id() auto-stamps the request''s tenant; an unset session ⇒ the reserved default tenant. RLS-enforced (tenant_isolation policy).';
ALTER TABLE fhir_mapping ENABLE ROW LEVEL SECURITY;
-- FORCE so the table OWNER is filtered too; superusers / BYPASSRLS
-- roles still bypass unconditionally (a Postgres invariant).
ALTER TABLE fhir_mapping FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON fhir_mapping
    USING (tenant_id = ext.current_tenant_id())
    WITH CHECK (tenant_id = ext.current_tenant_id());

-- ── Grants ──────────────────────────────────────────────────────
-- The baseline set ALTER DEFAULT PRIVILEGES for ferroehr_app/ferroehr_reader, so a
-- table the migrator creates afterwards is auto-granted; repeated explicitly
-- (role-guarded, like 0002/0003/0004) so this migration is self-contained and a
-- no-op on the normal run order.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON fhir_mapping TO ferroehr_app;
        GRANT SELECT ON fhir_mapping TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping fhir_mapping grants (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
