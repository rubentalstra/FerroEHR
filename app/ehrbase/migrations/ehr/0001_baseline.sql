-- ehr schema: the greenfield PG18-native CDR schema.
--
-- ENTERPRISE BASELINE (ADR-013). One squashed `0001` re-authored from the
-- accreted 0001..0010 chain (nothing deployed — ADR-013 §1, review doc 02
-- §5.1); append-only forever after. The node/vo_version architecture is
-- unchanged (ADR-008, P10 spike-validated); this baseline adds the enterprise
-- surface the accreted schema lacked: deterministic constraint/index names
-- (pk_/uq_/fk_/ck_/idx_ — ADR-013 §11), COMMENT ON everything non-obvious
-- (§12), roles + grants (§3), the spec-compliance fixes (§5–§10), and the
-- perf mechanisms (§4).
--
-- Design points (ADR-008):
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

-- ── Roles (ADR-013 §3) ───────────────────────────────────────────────────────
-- Mirror of the idempotent role block in ext/0001 (which runs first, so the
-- roles already exist here). Repeated so the ehr baseline is self-contained/
-- self-documenting; a no-op on the normal run order. See ext/0001 for the
-- role rationale (migrator/app/reader; NOLOGIN group roles).
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_migrator') THEN
        CREATE ROLE ehrbase_migrator NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_app') THEN
        CREATE ROLE ehrbase_app NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_reader') THEN
        CREATE ROLE ehrbase_reader NOLOGIN;
    END IF;
END $$;

-- ── ehr ──────────────────────────────────────────────────────────────────────
-- One row per EHR. system_id, ehr_id and time_created are immutable per-EHR
-- values recorded at creation (RM ehr §"EHR object"; review doc 03 req 2.1) —
-- system_id is a stored value, NOT merely the live service config.
CREATE TABLE ehr (
    id                uuid NOT NULL,
    -- The system that created this EHR, recorded at creation and never mutated
    -- (req 2.1). Immutable per-EHR; distinct from a version's creating_system_id.
    system_id         text NOT NULL,
    time_created      timestamptz NOT NULL DEFAULT now(),
    -- Promoted copy of the current EHR_STATUS `subject.external_ref`
    -- (`id.value` + `namespace`), kept in sync by the service on every
    -- EHR_STATUS write (vobject.rs). The partial unique index enforces one EHR
    -- per subject at the database (ITS-REST 409_EHR.yaml; CNF master06
    -- I_EHR_SERVICE.create_ehr-two_ehrs_same_patient).
    subject_id        text,
    subject_namespace text,
    CONSTRAINT pk_ehr PRIMARY KEY (id)
);
CREATE INDEX idx_ehr_time_created ON ehr (time_created DESC, id);
-- Subject uniqueness (review doc 03 req 2.8): CNF-hard (master06), RM-soft (a
-- subject MAY legitimately have multiple EHRs in the wild) — enforced only
-- where a complete (id, namespace) pair is present. Named for the service's
-- unique-violation → 409 mapping (vobject.rs).
CREATE UNIQUE INDEX uq_ehr_subject ON ehr (subject_id, subject_namespace)
    WHERE subject_id IS NOT NULL;

COMMENT ON TABLE ehr IS 'One row per EHR (RM ehr §"EHR object"). system_id/id/time_created are immutable per-EHR values recorded at creation (review doc 03 req 2.1).';
COMMENT ON COLUMN ehr.system_id IS 'The system that created this EHR, recorded at creation, never mutated (req 2.1). A stored value, not the live service config.';
COMMENT ON COLUMN ehr.subject_id IS 'Denormalized copy of the current EHR_STATUS subject.external_ref.id.value, synced by the service; backs the one-EHR-per-subject unique index (req 2.8).';
COMMENT ON COLUMN ehr.subject_namespace IS 'Denormalized copy of the current EHR_STATUS subject.external_ref.namespace (see subject_id).';

-- ── audit ──────────────────────────────────────────────────────────────────
-- AUDIT_DETAILS of every committed change (RM common master06 §AUDIT_DETAILS;
-- queryable via AQL VERSION paths).
CREATE TABLE audit (
    id             uuid NOT NULL DEFAULT uuidv7(),
    time_committed timestamptz NOT NULL DEFAULT now(),
    system_id      text NOT NULL,
    -- openEHR audit change-type group code (review doc 03 req 1.4.2). The DB
    -- CHECK restricts to the codes the service commits — 249 creation,
    -- 250 amendment, 251 modification, 523 deleted, 666 attestation (ADR-013
    -- §6); the terminology layer validates the full audit_change_type group at
    -- the wire edge (service/codes.rs).
    change_type    text NOT NULL,
    description    text,
    -- canonical PARTY_PROXY; lz4-compressed (ADR-013 §14). PG grammar: the
    -- COMPRESSION clause precedes column constraints.
    committer      jsonb COMPRESSION lz4 NOT NULL,
    CONSTRAINT pk_audit PRIMARY KEY (id),
    -- AUDIT_DETAILS.System_id_valid: system_id is 1..1, non-void (req 1.8).
    CONSTRAINT ck_audit_system_id_nonempty CHECK (system_id <> ''),
    -- The full openEHR audit_change_type group (the service validates the same
    -- set at the wire edge, service/codes.rs): 249 creation, 250 amendment,
    -- 251 modification, 252 synthesis, 253 unknown, 523 deleted, 666
    -- attestation, 816 restoration, 817 format conversion (req 1.4.2 —
    -- terminology-group-validated, not the master06 prose subset).
    CONSTRAINT ck_audit_change_type CHECK (change_type IN
        ('249', '250', '251', '252', '253', '523', '666', '816', '817'))
);

COMMENT ON TABLE audit IS 'AUDIT_DETAILS of every committed change (RM common master06 §AUDIT_DETAILS).';
COMMENT ON COLUMN audit.change_type IS 'audit_change_type group code (req 1.4.2). CHECK validates the full audit_change_type terminology group (ADR-013 §6, aligned with service/codes.rs).';
COMMENT ON COLUMN audit.time_committed IS 'Server-computed commit instant (req 1.8 — never client-supplied).';
COMMENT ON COLUMN audit.committer IS 'Canonical PARTY_PROXY of the committer (req 1.8).';

-- ── contribution ─────────────────────────────────────────────────────────────
-- The change-set envelope (RM common master06 §CONTRIBUTION): one CONTRIBUTION
-- per change set, strictly transactional (req 1.7). ehr_id is nullable: a
-- demographic (party) contribution has no owning EHR (NULL = the demographics
-- repository, ADR-008).
CREATE TABLE contribution (
    id       uuid NOT NULL DEFAULT uuidv7(),
    ehr_id   uuid,
    audit_id uuid NOT NULL,
    CONSTRAINT pk_contribution PRIMARY KEY (id),
    CONSTRAINT fk_contribution_ehr FOREIGN KEY (ehr_id) REFERENCES ehr (id) ON DELETE CASCADE,
    CONSTRAINT fk_contribution_audit FOREIGN KEY (audit_id) REFERENCES audit (id)
);
CREATE INDEX idx_contribution_ehr_id ON contribution (ehr_id);
CREATE INDEX idx_contribution_audit_id ON contribution (audit_id);

COMMENT ON TABLE contribution IS 'The change-set envelope (RM common master06 §CONTRIBUTION); one per change set, strictly transactional (req 1.7).';
COMMENT ON COLUMN contribution.ehr_id IS 'Owning EHR, or NULL for a demographic (party) contribution (the demographics repository is not EHR-owned; ADR-008).';

-- ── template_store ───────────────────────────────────────────────────────────
-- Operational templates (OPT 1.4 XML; parsed model built at P13/P14).
-- DUAL IDENTITY (ADR-013 §16, review doc 03 req 5.1.1): the uuid `id` is the
-- SM's OPT-keyed-by-UUID handle (the SM stores OPTs by UUID), while the unique
-- `template_id` is the wire address used by the ITS-REST DEFINITION API and by
-- vo_version.template_id. Both are load-bearing; neither is redundant.
CREATE TABLE template_store (
    id             uuid NOT NULL DEFAULT uuidv7(),
    template_id    text NOT NULL,
    concept        text,
    root_archetype text,
    content        text NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_template_store PRIMARY KEY (id),
    CONSTRAINT uq_template_store_template_id UNIQUE (template_id)
);

COMMENT ON TABLE template_store IS 'Operational templates (OPT 1.4 XML). Dual identity (ADR-013 §16): uuid id = the SM UUID handle (req 5.1.1); template_id = the wire address. Template versioning is NOT required — replace-in-place (req 5.1.3).';
COMMENT ON COLUMN template_store.id IS 'The SM OPT-by-UUID handle (req 5.1.1).';
COMMENT ON COLUMN template_store.template_id IS 'The wire address (ITS-REST DEFINITION API + vo_version.template_id FK target).';

-- ── vo_version ───────────────────────────────────────────────────────────────
-- One row per version of a versioned object (COMPOSITION/EHR_STATUS/EHR_ACCESS/
-- FOLDER + demographic party roots + PARTY_RELATIONSHIP). The temporal PK makes
-- overlapping validity impossible at the database. EHR_ACCESS is a versioned
-- object per RM ehr §"EHR Creation", versioned "via the normal mechanism".
-- Demographic kinds carry a NULL ehr_id (no owning EHR; ADR-008).
--
-- ADR-013 §2: the temporal PK (WITHOUT OVERLAPS = a GiST EXCLUDE under the hood)
-- STAYS, plus the plain btree UNIQUE (vo_id, sys_version) — needed as the
-- node/vo_attestation FK target AND as the logical-replication replica identity
-- (the GiST PK cannot serve it). fillfactor 90: one close-out UPDATE per
-- supersession (ADR-013 §14).
CREATE TABLE vo_version (
    vo_id           uuid NOT NULL,
    kind            text NOT NULL,
    ehr_id          uuid,
    sys_version     integer NOT NULL,
    -- Validity interval; the current version is the one with upper_inf(sys_period)
    -- (committal time is the only server-managed temporal axis, req 2.13).
    sys_period      tstzrange NOT NULL,
    -- ORIGINAL_VERSION.lifecycle_state: the numeric version_lifecycle_state code
    -- (532 complete, 553 incomplete, 523 deleted, 800 inactive, 801 abandoned).
    -- A logical delete writes a content-less version with state 523 (RM
    -- change_control §"Logical Deletion") — never a physical delete. 553 relaxes
    -- content validity, so content is never NOT NULL (req 1.5).
    lifecycle_state text NOT NULL DEFAULT '532',
    -- The immutable identity of the system that CREATED this version (req 1.2.3):
    -- the middle segment of the OBJECT_VERSION_ID {object_id, creating_system_id,
    -- version_tree_id}. Reconstructed from storage, never re-derived from live
    -- config, so a config change cannot mutate a historical uid or invalidate a
    -- signature. Written on every version (local: our system_id; import:
    -- preserved) — NO '' sentinel (ADR-013 §8).
    creating_system_id text NOT NULL,
    -- VERSION.signature (RM common §"Digital Signature"): 0..1, opaque radix-64
    -- (OpenPGP RFC 4880 or a SHA-256 digest). Canonicalisation is spec-TBD
    -- (review doc 03 S2) — PORT NOTE territory. Historical versions may carry none.
    signature       text,
    -- ORIGINAL_VERSION.other_input_version_uids: merge provenance for an imported
    -- merged version (req 1.3.4). NULL when not a merge. Trunk-only branching
    -- remains a typed rejection, but the merge identity is no longer lost on
    -- import (ADR-013 §7).
    other_input_version_uids jsonb,
    contribution_id uuid NOT NULL,
    audit_id        uuid NOT NULL,
    template_id     text,
    CONSTRAINT pk_vo_version PRIMARY KEY (vo_id, sys_period WITHOUT OVERLAPS),
    CONSTRAINT uq_vo_version_vo_id_sys_version UNIQUE (vo_id, sys_version),
    CONSTRAINT ck_vo_version_sys_version_positive CHECK (sys_version >= 1),
    CONSTRAINT ck_vo_version_kind CHECK (kind IN (
        'COMPOSITION', 'EHR_STATUS', 'EHR_ACCESS', 'FOLDER',
        'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE', 'PARTY_RELATIONSHIP'
    )),
    CONSTRAINT ck_vo_version_lifecycle_state CHECK (lifecycle_state IN ('532', '553', '523', '800', '801')),
    CONSTRAINT fk_vo_version_contribution FOREIGN KEY (contribution_id) REFERENCES contribution (id),
    CONSTRAINT fk_vo_version_audit FOREIGN KEY (audit_id) REFERENCES audit (id),
    CONSTRAINT fk_vo_version_template FOREIGN KEY (template_id) REFERENCES template_store (template_id),
    CONSTRAINT fk_vo_version_ehr FOREIGN KEY (ehr_id) REFERENCES ehr (id) ON DELETE CASCADE
) WITH (fillfactor = 90);
-- LATEST_VERSION = this partial index (GiST does not serve it); one current
-- version per vo_id.
CREATE UNIQUE INDEX uq_vo_version_current ON vo_version (vo_id) WHERE upper_inf(sys_period);
CREATE INDEX idx_vo_version_ehr ON vo_version (ehr_id, kind);
CREATE INDEX idx_vo_version_contribution ON vo_version (contribution_id);
CREATE INDEX idx_vo_version_audit ON vo_version (audit_id);
CREATE INDEX idx_vo_version_template ON vo_version (template_id) WHERE template_id IS NOT NULL;
-- The btree UNIQUE (not the GiST temporal PK) is the logical-replication replica
-- identity (ADR-013 §2, review doc 02 §6.3).
ALTER TABLE vo_version REPLICA IDENTITY USING INDEX uq_vo_version_vo_id_sys_version;

COMMENT ON TABLE vo_version IS 'One temporal row per version of a versioned object (ADR-008). Temporal PK forbids overlapping validity; ALL_VERSIONS = unfiltered, LATEST_VERSION = uq_vo_version_current.';
COMMENT ON COLUMN vo_version.sys_period IS 'Validity interval [committed, superseded); the current version has upper_inf(sys_period). Committal time is the only server-managed temporal axis (req 2.13).';
COMMENT ON COLUMN vo_version.creating_system_id IS 'Immutable per-version creating-system id — the OBJECT_VERSION_ID middle segment (req 1.2.3). Reconstructed from storage, never live config; no '''' sentinel (ADR-013 §8).';
COMMENT ON COLUMN vo_version.lifecycle_state IS 'version_lifecycle_state code (req 1.5). 523 = logical delete (content-less version). 553 relaxes content validity.';
COMMENT ON COLUMN vo_version.signature IS 'VERSION.signature (0..1), opaque radix-64. Canonicalisation is spec-TBD (review doc 03 S2 — PORT NOTE).';
COMMENT ON COLUMN vo_version.other_input_version_uids IS 'ORIGINAL_VERSION merge provenance for an imported merged version (req 1.3.4); NULL when not a merge (ADR-013 §7).';

-- ── node ─────────────────────────────────────────────────────────────────────
-- The decomposed content: one row per RM structure node, per version. The
-- nested-set interval (num..=num_cap) makes AQL CONTAINS an integer range join
-- (never a JSON walk). ehr_id is nullable (demographic content has none).
CREATE TABLE node (
    vo_id       uuid NOT NULL,
    sys_version integer NOT NULL,
    -- Pre-order number within the versioned object (root = 0).
    num         integer NOT NULL,
    -- Max num in this row's subtree: the subtree is num..=num_cap (CONTAINS).
    num_cap     integer NOT NULL,
    -- num of the parent structure node (root points at itself/0).
    parent_num  integer NOT NULL,
    -- num of the nearest ancestor carrying an archetype id.
    citem_num   integer,
    ehr_id      uuid,
    rm_type     text NOT NULL,
    archetype   text,
    name        text,
    -- Materialized path from the root; byte-order under COLLATE "C" equals tree
    -- order (used only for reassembly, not as an AQL predicate).
    path        text COLLATE "C" NOT NULL,
    -- The node's canonical openEHR JSON fragment verbatim, structure children
    -- pruned (ADR-008: no aliasing, no synthetic fields — storage == API).
    -- lz4-compressed (ADR-013 §14; COMPRESSION precedes constraints).
    data        jsonb COMPRESSION lz4 NOT NULL,
    CONSTRAINT pk_node PRIMARY KEY (vo_id, sys_version, num),
    -- num_cap >= num and parent above (pre-order) — the nested-set invariant
    -- (ADR-013 §9). The root (num = 0, parent_num = 0) is exempt from the parent
    -- ordering check.
    CONSTRAINT ck_node_num_cap CHECK (num_cap >= num),
    CONSTRAINT ck_node_parent CHECK (num = 0 OR parent_num < num),
    -- DEFERRABLE so a multi-row version commit (vo_version + its many node rows)
    -- can be reordered within one transaction (ADR-013 §13); INITIALLY IMMEDIATE
    -- keeps the default check-at-statement-end behaviour.
    CONSTRAINT fk_node_vo_version FOREIGN KEY (vo_id, sys_version)
        REFERENCES vo_version (vo_id, sys_version) ON DELETE CASCADE
        DEFERRABLE INITIALLY IMMEDIATE
);
CREATE INDEX idx_node_type_archetype ON node (rm_type, archetype);
CREATE INDEX idx_node_ehr ON node (ehr_id);
-- jsonb_ops (NOT jsonb_path_ops): $.** equality anchors need it (ADR-008).
CREATE INDEX idx_node_data_gin ON node USING gin (data jsonb_ops);
-- Magnitude expression index (ADR-013 §4, ADR-008 §2 — SPECULATIVE, P20-repriced).
-- Partial predicate: rm_type = 'ELEMENT'. ELEMENT is the sole leaf-bearing RM
-- structure node — every DV_ORDERED value lives in an ELEMENT.value — so it is
-- the smallest node set that can carry an ordered magnitude.
-- The P10 codec keeps a DV_ORDERED value INLINE inside its ELEMENT's fragment
-- (only structure types get their own node row), so the indexed expression is
-- the ELEMENT's value payload: ext.openehr_magnitude(data -> 'value') — real
-- magnitudes for every ordered leaf, NULL for non-ordered values (btree stores
-- them compactly). PERF(port): the AQL generator's ordering expression today is
-- ext.openehr_magnitude(jsonb_path_query_first(data, <jsonpath>)); for this
-- index to serve it, the generator's ELEMENT-value fast path must emit
-- ext.openehr_magnitude(data -> 'value') verbatim — wire + EXPLAIN-validate at
-- P20 (ADR-013 §4: wired now per owner decision, repriced at P20).
CREATE INDEX idx_node_magnitude ON node (ext.openehr_magnitude(data -> 'value'))
    WHERE rm_type = 'ELEMENT';

COMMENT ON TABLE node IS 'Decomposed versioned-object content: one row per RM structure node, per version (ADR-008). Nested-set interval num..=num_cap makes CONTAINS an integer range join.';
COMMENT ON COLUMN node.num IS 'Pre-order number within the versioned object (root = 0).';
COMMENT ON COLUMN node.num_cap IS 'Max num in this node''s subtree: the subtree is num..=num_cap (AQL CONTAINS).';
COMMENT ON COLUMN node.parent_num IS 'num of the parent structure node (root points at itself/0).';
COMMENT ON COLUMN node.citem_num IS 'num of the nearest ancestor carrying an archetype id.';
COMMENT ON COLUMN node.path IS 'Materialized path from the root; byte-order under COLLATE "C" equals tree order. Reassembly only — never an AQL predicate.';
COMMENT ON COLUMN node.data IS 'The node''s canonical openEHR JSON fragment verbatim, structure children pruned (ADR-008: storage == API, no synthetic fields).';

-- ── vo_attestation ───────────────────────────────────────────────────────────
-- ATTESTATION storage (RM common master06 §Change Control): a new ATTESTATION
-- is appended to an existing ORIGINAL_VERSION's attestations list WITHOUT a new
-- version (req 1.9), committed via a contribution. One row per attestation,
-- canonical RM ATTESTATION verbatim in data (ADR-008: no synthetic fields).
CREATE TABLE vo_attestation (
    id              uuid NOT NULL DEFAULT uuidv7(),
    vo_id           uuid NOT NULL,
    sys_version     integer NOT NULL,
    contribution_id uuid NOT NULL,
    time_committed  timestamptz NOT NULL DEFAULT now(),
    data            jsonb NOT NULL,
    CONSTRAINT pk_vo_attestation PRIMARY KEY (id),
    CONSTRAINT fk_vo_attestation_vo_version FOREIGN KEY (vo_id, sys_version)
        REFERENCES vo_version (vo_id, sys_version) ON DELETE CASCADE,
    CONSTRAINT fk_vo_attestation_contribution FOREIGN KEY (contribution_id) REFERENCES contribution (id)
);
CREATE INDEX idx_vo_attestation_version ON vo_attestation (vo_id, sys_version);
CREATE INDEX idx_vo_attestation_contribution ON vo_attestation (contribution_id);

COMMENT ON TABLE vo_attestation IS 'ATTESTATION storage (RM common master06 §Change Control): appended to an ORIGINAL_VERSION''s attestations list without a new version (req 1.9); canonical ATTESTATION verbatim in data.';

-- ── stored_query ─────────────────────────────────────────────────────────────
-- Stored AQL queries (semver-addressed per ITS-REST DEFINITION API; review doc
-- 03 req 5.2): qualified name (namespace default "misc"), formalism, semver.
CREATE TABLE stored_query (
    reverse_domain_name text NOT NULL,
    semantic_id         text NOT NULL,
    semver              text NOT NULL DEFAULT '0.0.0',
    query_type          text NOT NULL DEFAULT 'AQL',
    query_text          text NOT NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_stored_query PRIMARY KEY (reverse_domain_name, semantic_id, semver)
);

COMMENT ON TABLE stored_query IS 'Stored AQL queries, semver-addressed (ITS-REST DEFINITION API; review doc 03 req 5.2).';

-- ── item_tag ─────────────────────────────────────────────────────────────────
-- Item tags (ITS-REST experimental tags API; review doc 03 req 5.3). Loose
-- coupling: tags are mutable, EHR-scoped, outside the version chain, require no
-- contribution. ehr_id is nullable (a party may be tagged too).
CREATE TABLE item_tag (
    id           uuid NOT NULL DEFAULT uuidv7(),
    ehr_id       uuid,
    -- The tagged target's uid. INTENTIONALLY FK-LESS (ADR-013 §15, review doc 03
    -- req 5.3.2): a tag may target a container OR a specific VERSION, i.e. it is
    -- deliberately outside the version chain, so a FK into vo_version (which is
    -- keyed per version) would be wrong. Referential looseness is by design.
    target_vo_id uuid NOT NULL,
    target_type  text NOT NULL,
    key          text NOT NULL,
    value        text,
    target_path  text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_item_tag PRIMARY KEY (id),
    CONSTRAINT uq_item_tag_ehr_target_key UNIQUE (ehr_id, target_vo_id, key),
    CONSTRAINT ck_item_tag_target_type CHECK (target_type IN (
        'COMPOSITION', 'EHR_STATUS', 'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE'
    )),
    CONSTRAINT fk_item_tag_ehr FOREIGN KEY (ehr_id) REFERENCES ehr (id) ON DELETE CASCADE
);

COMMENT ON TABLE item_tag IS 'Item tags (ITS-REST experimental; review doc 03 req 5.3). Mutable, EHR-scoped, outside the version chain.';
COMMENT ON COLUMN item_tag.target_vo_id IS 'PORT NOTE: intentionally FK-less (ADR-013 §15, req 5.3.2) — a tag may target a container OR a specific VERSION, so it is deliberately outside the version chain.';

-- ── archetype_store (SM-2, I_DEFINITION_ADL14) ───────────────────────────────
-- ADL 1.4 source archetypes, keyed by their human-readable ARCHETYPE_ID (not a
-- UUID — reserved for OPTs/ADL2). Source ADL text stored verbatim; upload
-- replaces an existing id (ON CONFLICT DO UPDATE). Separate identity scheme per
-- formalism (ADR-013 §16, review doc 03 req 5.1.1/5.1.2).
CREATE TABLE archetype_store (
    archetype_id text NOT NULL,
    adl          text NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_archetype_store PRIMARY KEY (archetype_id)
);

COMMENT ON TABLE archetype_store IS 'SM-2 ADL 1.4 source archetypes (I_DEFINITION_ADL14), keyed by ARCHETYPE_ID; verbatim ADL text.';

-- ── adl2_artefact (SM-2, I_DEFINITION_ADL2) ──────────────────────────────────
-- ADL2 artefacts (source archetype / template / OPT), all keyed uniformly by
-- ARCHETYPE_HRID; kind discriminates for the per-type list/count calls. Source
-- ADL2 text verbatim; upload replaces an existing HRID.
CREATE TABLE adl2_artefact (
    hrid       text NOT NULL,
    kind       text NOT NULL,
    adl        text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_adl2_artefact PRIMARY KEY (hrid),
    CONSTRAINT ck_adl2_artefact_kind CHECK (kind IN ('archetype', 'template', 'operational_template'))
);

COMMENT ON TABLE adl2_artefact IS 'SM-2 ADL2 artefacts (I_DEFINITION_ADL2), keyed by ARCHETYPE_HRID; kind ∈ archetype/template/operational_template; verbatim ADL2 text.';

-- ── ehr_index (SM-3, I_EHR_INDEX) ─────────────────────────────────────────────
-- N:M subject↔EHR associations with duplicate-management metadata (master07).
-- NOT a versioned object (the SM defines no versioning here) — plain relational.
CREATE TABLE ehr_index (
    ehr_id            uuid NOT NULL,
    subject_id        text NOT NULL,
    subject_namespace text NOT NULL,
    -- OBJECT_REF.type of the subject (defaults to PERSON — the common MPI case).
    subject_type      text NOT NULL DEFAULT 'PERSON',
    -- RESOURCE_INSTANCE_TYPE: Primary is authoritative; Duplicate/Supplementary
    -- flag the N:M error states master07 wants surfaced.
    instance_type     text NOT NULL DEFAULT 'Primary',
    -- RESOURCE_STATUS.start/end_valid_time (typed @@ placeholder in the SM).
    start_valid_time  timestamptz,
    end_valid_time    timestamptz,
    notes             text,
    -- LOCATION_DESC {system_id, uri?, description?}, canonical JSON (PORT NOTE).
    location          jsonb,
    created_at        timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_ehr_index PRIMARY KEY (ehr_id, subject_id, subject_namespace),
    CONSTRAINT ck_ehr_index_instance_type CHECK (instance_type IN ('Primary', 'Duplicate', 'Supplementary')),
    CONSTRAINT fk_ehr_index_ehr FOREIGN KEY (ehr_id) REFERENCES ehr (id) ON DELETE CASCADE
);
-- subject → EHRs lookup (remove_subject / subject_ehrs).
CREATE INDEX idx_ehr_index_subject ON ehr_index (subject_id, subject_namespace);

COMMENT ON TABLE ehr_index IS 'SM-3 EHR Index (I_EHR_INDEX, master07): N:M subject↔EHR associations with duplicate-management metadata; not a versioned object.';

-- ── vo_archive (SM-4, I_ADMIN_ARCHIVE) ────────────────────────────────────────
-- Versioned-object archive MARKERS (review doc 03 req 5.4.3). The SM defines no
-- storage form; actual tier movement is P20. Serving reads never join this table
-- (zero wire drift). Plain marker keyed by vo_id (not a per-version key), so
-- INTENTIONALLY FK-LESS to the composite-keyed vo_version.
CREATE TABLE vo_archive (
    vo_id       uuid NOT NULL,
    archived_at timestamptz NOT NULL DEFAULT now(),
    reason      text,
    CONSTRAINT pk_vo_archive PRIMARY KEY (vo_id)
);

COMMENT ON TABLE vo_archive IS 'SM-4 archive markers (I_ADMIN_ARCHIVE; req 5.4.3). Marker only — serving reads never join it (zero wire drift). Intentionally FK-less (vo_id is not a per-version key).';

-- ── Subject Proxy Service config stores (SM-6, I_SUBJECT_PROXY_SERVICE) ───────
-- CONFIGURATION only (review doc 03 req 5.5.1): bindings + variable defs, kept
-- for the life of the system, cleared by reset(). Results/data frames are
-- transient — not stored. Plain relational, not versioned objects.

-- SUBJECT_PROXY: one proxy per subject.
CREATE TABLE sp_subject (
    subject_id       text NOT NULL,
    -- SUBJECT_PROXY.subject_category (free string; "not controlled" in the SM).
    subject_category text NOT NULL DEFAULT 'individual',
    create_time      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_sp_subject PRIMARY KEY (subject_id)
);
COMMENT ON TABLE sp_subject IS 'SM-6 SUBJECT_PROXY config: one proxy per subject (req 5.5.1).';

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
    -- Missing FK added (ADR-013 §15): a variable binds a frame that must exist.
    CONSTRAINT fk_sp_variable_frame FOREIGN KEY (frame_id) REFERENCES sp_data_frame (frame_id)
);
CREATE INDEX idx_sp_variable_frame ON sp_variable (frame_id);
COMMENT ON TABLE sp_variable IS 'SM-6 SUBJECT_VARIABLE config, keyed by canonical_name; frame_id FK into sp_data_frame (ADR-013 §15).';

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

-- ── Grants (ADR-013 §3, review doc 02 §3.1/§3.2/§3.6) ─────────────────────────
-- Lock down the public schema (review doc 02 §3.6): no PUBLIC CREATE.
REVOKE CREATE ON SCHEMA public FROM PUBLIC;

-- The runtime writer (DML) and the read-only role. The migrator owns the objects
-- (it ran this DDL). No sequences to grant — all generated keys use uuidv7().
GRANT USAGE ON SCHEMA ehr TO ehrbase_app, ehrbase_reader;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA ehr TO ehrbase_app;
GRANT SELECT ON ALL TABLES IN SCHEMA ehr TO ehrbase_reader;
-- Future ehr tables (later append-only migrations run as the migrator) are
-- reachable without a manual grant — the deploy-outage classic (review doc 02 §3.2).
ALTER DEFAULT PRIVILEGES IN SCHEMA ehr
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ehrbase_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA ehr
    GRANT SELECT ON TABLES TO ehrbase_reader;
