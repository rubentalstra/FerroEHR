-- SPDX-FileCopyrightText: FerroEHR contributors
-- SPDX-License-Identifier: MIT

-- ehr schema: the greenfield PG18-native CDR schema.
--
-- BASELINE. openEHR defines NO SQL schema — the relational layout here is our
-- own storage design (flagged as such throughout); what the openEHR specs DO
-- define — versioning/change-control semantics (RM common master06), canonical
-- data fidelity (ITS-JSON), contribution/audit duties — is what the
-- constraints and comments below cite. Conventions: deterministic
-- constraint/index names (pk_/uq_/fk_/ck_/idx_), COMMENT ON everything
-- non-obvious, roles + grants (no spec governs these — engineering choices).
--
-- Design points (storage mechanics — no openEHR spec governs the physical
-- schema; the cited RM/ITS duties are what each mechanism realizes):
--   * one unified `node` table for all versioned-object content, stored
--     PER VERSION (ALL_VERSIONS queries one table uniformly), with a
--     nested-set index (num/num_cap/parent_num) so AQL CONTAINS is an
--     integer interval join;
--   * node.data holds the node's CANONICAL openEHR JSON fragment verbatim
--     (structure children pruned) — no alias compaction, no synthetic
--     fields; fragments average ~360 B (spike), well under TOAST;
--   * one temporal `vo_version` table (per-lineage non-overlap held by
--     construction — see the NOTE at the table) instead of current/history
--     pairs; the version tree (trunk + branches, RM common master06) lives in
--     explicit trunk/branch columns; the current (latest trunk) version is
--     the upper_inf ∧ branch_number = 0 partial index;
--   * every write emits a contribution + audit row (openEHR requirement);
--   * uuidv7() (PG18) for generated keys — time-ordered, index-friendly.
-- Runs with search_path = ehr, ext.

-- ── Roles (no openEHR spec governs DB roles — operational design) ────────────
-- Mirror of the idempotent role block in ext/0001 (which runs first, so the
-- roles already exist here). Repeated so the ehr baseline is self-contained/
-- self-documenting; a no-op on the normal run order. See ext/0001 for the
-- role rationale (migrator/app/reader; NOLOGIN group roles).
DO $$
BEGIN
    -- Graceful degradation (deployment reality): when the migration runs as a
    -- user without CREATEROLE (dev/compose/testcontainers or a managed PG
    -- without role rights), the role architecture is skipped with a NOTICE —
    -- it is then a deployment-layer setup step (no openEHR spec governs role
    -- provisioning — our own operational design). When the migrator has the
    -- privilege (production), roles are created idempotently.
    BEGIN
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_migrator') THEN
            CREATE ROLE ferroehr_migrator NOLOGIN;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
            CREATE ROLE ferroehr_app NOLOGIN;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_reader') THEN
            CREATE ROLE ferroehr_reader NOLOGIN;
        END IF;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'skipping role creation (no CREATEROLE privilege): create ferroehr_migrator/ferroehr_app/ferroehr_reader at deployment';
    END;
END $$;

-- ── ehr ──────────────────────────────────────────────────────────────────────
-- One row per EHR. system_id, ehr_id and time_created are immutable per-EHR
-- values recorded at creation (RM ehr master04-ehr_package.adoc §Root EHR
-- Object: "The root EHR object records three pieces of information that are
-- immutable after creation: the identifier of the system in which the EHR was
-- created, the identifier of the EHR ... and the time of creation of the EHR")
-- — system_id is a stored value, NOT merely the live service config.
CREATE TABLE ehr (
    id                uuid NOT NULL,
    -- The system that created this EHR, recorded at creation and never mutated
    -- (RM ehr master04 §Root EHR Object; §EHR Identifier Allocation: on a cloned
    -- EHR "the system_id is from the receiving (cloning) system"). Immutable
    -- per-EHR; distinct from a version's creating_system_id.
    system_id         text NOT NULL,
    time_created      timestamptz NOT NULL DEFAULT now(),
    -- Promoted copy of the current EHR_STATUS `subject.external_ref`
    -- (`id.value` + `namespace`), kept in sync by the service on every
    -- EHR_STATUS write (vobject.rs). The partial unique index enforces one EHR
    -- per subject at the database (ITS-REST 409_EHR.yaml; CNF master06
    -- I_EHR_SERVICE.create_ehr-two_ehrs_same_patient).
    subject_id        text,
    subject_namespace text,
    -- Promoted copy of the current EHR_STATUS.is_queryable flag (RM ehr master04
    -- §EHR Status: EHR_STATUS.is_queryable, 1..1 Boolean), kept in lockstep with
    -- the current EHR_STATUS by the service on every status write (status.rs
    -- sync_ehr_subject) and backfilled on the import / archive-load paths. The
    -- AQL full-population gate — SM I_QUERY_SERVICE.execute_ad_hoc_query /
    -- execute_stored_query: with no ehr_ids supplied "a full population query
    -- will be performed on all EHRs whose status has the is_queryable flag set
    -- to True" (i_query_service.adoc) — filters this column directly instead of
    -- probing every current EHR_STATUS root node per query. Default true = the
    -- default EHR_STATUS a fresh EHR is created with. No index: the gate rides
    -- the PK / ehr-id join under ORDER BY id LIMIT n, and a partial index on the
    -- false rows is pointless (almost every EHR is queryable). No openEHR spec
    -- governs the promoted column itself — our own storage design.
    is_queryable      boolean NOT NULL DEFAULT true,
    -- Promoted copy of the current EHR_STATUS.is_modifiable flag (RM ehr master04
    -- §EHR Active Status: EHR_STATUS.is_modifiable, 1..1 Boolean), kept in
    -- lockstep with the current EHR_STATUS by the service on every status write
    -- (status.rs sync_ehr_subject — the same UPDATE that syncs subject_* /
    -- is_queryable) and backfilled on the import / archive-load paths. The
    -- content-write guard — RM ehr master04 §EHR Active Status: is_modifiable
    -- "is used to indicate whether the contents of an EHR are modifiable"; "an
    -- EHR's 'contents' consist of everything other than the EHR_STATUS object" —
    -- reads this column directly instead of probing the current EHR_STATUS root
    -- node per content write. Default true = the default EHR_STATUS a fresh EHR
    -- is created with (and an active EHR). No openEHR spec governs the promoted
    -- column itself — our own storage design, symmetric with is_queryable.
    is_modifiable     boolean NOT NULL DEFAULT true,
    CONSTRAINT pk_ehr PRIMARY KEY (id)
);
CREATE INDEX idx_ehr_time_created ON ehr (time_created DESC, id);
-- Subject uniqueness: wire-hard, RM-soft. The wire refuses a second EHR for a
-- subject (ITS-REST 409_EHR.yaml; CNF platform_test_schedule master06
-- I_EHR_SERVICE.create_ehr-two_ehrs_same_patient), while the RM explicitly
-- tolerates it — RM ehr master04-ehr_package.adoc §EHR Identifier Allocation:
-- "providers routinely create new EHRs for a patient regardless of how many
-- other EHRs already exist for that patient". Enforced only where a complete
-- (id, namespace) pair is present. Named for the service's unique-violation →
-- 409 mapping (vobject.rs).
CREATE UNIQUE INDEX uq_ehr_subject ON ehr (subject_id, subject_namespace)
    WHERE subject_id IS NOT NULL;

COMMENT ON TABLE ehr IS 'One row per EHR. system_id/id/time_created are the three values RM ehr master04-ehr_package.adoc §Root EHR Object makes immutable after creation.';
COMMENT ON COLUMN ehr.system_id IS 'The system that created this EHR, recorded at creation, never mutated (RM ehr master04 §Root EHR Object). A stored value, not the live service config.';
COMMENT ON COLUMN ehr.subject_id IS 'Denormalized copy of the current EHR_STATUS subject.external_ref.id.value, synced by the service; backs the one-EHR-per-subject unique index (ITS-REST 409_EHR.yaml). Our own storage design.';
COMMENT ON COLUMN ehr.subject_namespace IS 'Denormalized copy of the current EHR_STATUS subject.external_ref.namespace (see subject_id).';
COMMENT ON COLUMN ehr.is_queryable IS 'Promoted copy of the current EHR_STATUS.is_queryable (RM ehr master04 §EHR Status), synced by the service; backs the AQL full-population gate (SM I_QUERY_SERVICE, i_query_service.adoc). Our own storage design.';
COMMENT ON COLUMN ehr.is_modifiable IS 'Promoted copy of the current EHR_STATUS.is_modifiable (RM ehr master04 §EHR Active Status), synced by the service; backs the content-write guard (a deactivated EHR refuses Composition/Folder/content-CONTRIBUTION writes). Our own storage design, symmetric with is_queryable.';

-- ── audit ──────────────────────────────────────────────────────────────────
-- AUDIT_DETAILS of every committed change (RM common master04-generic_package.adoc
-- §Audit Details + UML/classes/org.openehr.rm.common.audit_details.adoc:
-- system_id / time_committed / change_type / committer 1..1, description 0..1;
-- queryable via AQL VERSION paths).
CREATE TABLE audit (
    id             uuid NOT NULL DEFAULT uuidv7(),
    time_committed timestamptz NOT NULL DEFAULT now(),
    system_id      text NOT NULL,
    -- openEHR `audit change type` group code — the code_string of
    -- AUDIT_DETAILS.change_type (a DV_CODED_TEXT, 1..1;
    -- `AUDIT_DETAILS.Change_type_valid` binds it to that group, RM common
    -- UML/classes/org.openehr.rm.common.audit_details.adoc §Invariants). The
    -- codes are TERM SupportTerminology
    -- codesets/openehr_terminology-vocabularies.adoc §Audit Change Type; the
    -- terminology layer validates the group at the wire edge
    -- (service/codes.rs) and the CHECK below repeats it at the database.
    change_type    text NOT NULL,
    -- AUDIT_DETAILS.description is a DV_TEXT (0..1) whose DV_CODED_TEXT
    -- subtype carries a defining_code (RM common
    -- UML/classes/org.openehr.rm.common.audit_details.adoc §Attributes), so the
    -- whole canonical fragment is stored — a bare text column would discard the
    -- concrete _type and every coded attribute.
    description    jsonb COMPRESSION lz4,
    -- canonical PARTY_PROXY; lz4-compressed (storage choice — no spec governs
    -- compression). PG grammar: the
    -- COMPRESSION clause precedes column constraints.
    committer      jsonb COMPRESSION lz4 NOT NULL,
    -- The ATTESTATION-declared attributes (attested_view, proof, items, reason,
    -- is_pending — RM common UML/classes/org.openehr.rm.common.attestation.adoc
    -- §Attributes), each in its canonical encoding, when this commit audit is
    -- an ATTESTATION rather than a plain AUDIT_DETAILS: "ORIGINAL_VERSION.
    -- commit_audit is of type ATTESTATION rather than AUDIT_DETAILS" (RM common
    -- master06 §Attestation; master04 §Attestation calls it "the most common
    -- scenario" when an attestation is required). NULL ⇔ a plain AUDIT_DETAILS:
    -- ATTESTATION is the ONLY AUDIT_DETAILS subtype RM 1.2.0 declares and both
    -- of these attributes are mandatory on it, so presence is the concrete
    -- class and there is no second discriminator to drift from. The INHERITED
    -- attributes stay in the promoted columns for every audit alike, so
    -- system_id / change_type / time_committed remain one queryable shape.
    -- lz4-compressed: attested_view is a DV_MULTIMEDIA and may carry inline
    -- data (storage choice — no spec governs compression).
    attestation    jsonb COMPRESSION lz4,
    CONSTRAINT pk_audit PRIMARY KEY (id),
    -- AUDIT_DETAILS.System_id_valid: `not system_id.is_empty` (RM common
    -- UML/classes/org.openehr.rm.common.audit_details.adoc §Invariants).
    CONSTRAINT ck_audit_system_id_nonempty CHECK (system_id <> ''),
    -- DV_TEXT.value is 1..1 (RM data_types
    -- UML/classes/org.openehr.rm.data_types.dv_text.adoc §Attributes), so a
    -- stored description always carries one.
    CONSTRAINT ck_audit_description_shape CHECK
        (description IS NULL OR description ? 'value'),
    -- ATTESTATION.reason and ATTESTATION.is_pending are both 1..1 (RM common
    -- UML/classes/org.openehr.rm.common.attestation.adoc §Attributes).
    CONSTRAINT ck_audit_attestation_shape CHECK
        (attestation IS NULL OR (attestation ? 'reason' AND attestation ? 'is_pending')),
    -- The full openEHR `audit change type` group (the service validates the
    -- same set at the wire edge, service/codes.rs): 249 creation, 250
    -- amendment, 251 modification, 252 synthesis, 253 unknown, 523 deleted,
    -- 666 attestation, 816 restoration, 817 format conversion — TERM
    -- SupportTerminology codesets/openehr_terminology-vocabularies.adoc
    -- §Audit Change Type, which is what `Change_type_valid` binds to; NOT the
    -- narrower subset RM common master06 §Contributions enumerates in prose.
    CONSTRAINT ck_audit_change_type CHECK (change_type IN
        ('249', '250', '251', '252', '253', '523', '666', '816', '817'))
);

COMMENT ON TABLE audit IS 'AUDIT_DETAILS of every committed change (RM common master04-generic_package.adoc §Audit Details), including the ATTESTATION subtype a version may be committed with (master06 §Attestation).';
COMMENT ON COLUMN audit.change_type IS 'openEHR `audit change type` group code (AUDIT_DETAILS.Change_type_valid, RM common UML/classes/org.openehr.rm.common.audit_details.adoc §Invariants). CHECK validates the full group as published in TERM SupportTerminology codesets/openehr_terminology-vocabularies.adoc §Audit Change Type (aligned with service/codes.rs).';
COMMENT ON COLUMN audit.time_committed IS 'Server-computed commit instant: RM common master06 §Committal and Audits — time_committed "should reflect the time of committal to an EHR server ... It should therefore be computed on the server". Never client-supplied.';
COMMENT ON COLUMN audit.committer IS 'Canonical PARTY_PROXY of the committer (AUDIT_DETAILS.committer 1..1, RM common UML/classes/org.openehr.rm.common.audit_details.adoc §Attributes).';
COMMENT ON COLUMN audit.description IS 'Canonical AUDIT_DETAILS.description fragment (DV_TEXT or its DV_CODED_TEXT subtype), stored whole so the concrete _type and defining_code survive (RM common master04 §Audit Details).';
COMMENT ON COLUMN audit.attestation IS 'The ATTESTATION-declared attributes (attested_view/proof/items/reason/is_pending) when the commit audit is an ATTESTATION, else NULL (RM common master06 §Attestation; master04 §Attestation).';

-- ── contribution ─────────────────────────────────────────────────────────────
-- The change-set envelope (RM common master06 §Contributions: "a `CONTRIBUTION`
-- object will be created, listing the affected `VERSION` objects, and including
-- its own audit object"): one CONTRIBUTION per change set, strictly
-- transactional (master06 §Committal and Audits: "Contributions are similar to
-- nested transactions. An attempt to commit a Contribution should only succeed
-- if each Version and/or Attestation in the Contribution is committed
-- successfully"). ehr_id is nullable: a
-- demographic (party) contribution has no owning EHR (NULL = the demographics
-- repository — RM demographic content is not EHR-owned).
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
-- The admin statistics' time-bounded paths (service/admin/statistics.rs) filter
-- and order on the commit instant; at measured-corpus scale (~10^6 rows) the
-- 2026-07-29 POC window put the unindexed form's p99 at 2.9 s. No openEHR spec
-- governs indexing - our own design.
CREATE INDEX idx_audit_time_committed ON audit (time_committed);

COMMENT ON TABLE contribution IS 'The change-set envelope (RM common master06 §Contributions); one per change set, strictly transactional (master06 §Committal and Audits: a Contribution commits only if every member Version/Attestation commits).';
COMMENT ON COLUMN contribution.ehr_id IS 'Owning EHR, or NULL for a demographic (party) contribution (RM demographic content is not EHR-owned).';

-- ── template_ref ─────────────────────────────────────────────────────────────
-- The registry of PROVISIONED template wire addresses across BOTH template
-- dialects: `template_store` rows (OPT 1.4 `template_id`) and `adl2_artefact`
-- template-kind rows (ADL2 template / operational_template HRIDs — the AM
-- component keeps the two generations side by side, BASE architecture_overview
-- master05 §Package Structure). `vo_version.template_id` references THIS
-- table, so a committed version's template identity is FK-guarded whichever
-- DEFINITION surface provisioned the template, and a physical delete of an
-- in-use template fails loud even under a concurrent commit (the NO ACTION
-- check at delete time) — the same race guard the former template_store-only
-- FK provided, made dialect-complete. Rows are maintained in the same
-- transaction as the owning store row (insert on provisioning; delete when the
-- last owning store row goes). No openEHR spec governs storage integrity —
-- our own design.
CREATE TABLE template_ref (
    template_id text NOT NULL,
    CONSTRAINT pk_template_ref PRIMARY KEY (template_id)
);

COMMENT ON TABLE template_ref IS 'Registry of provisioned template wire addresses (union of template_store.template_id and template-kind adl2_artefact.hrid). FK target of vo_version.template_id: the dialect-complete delete/commit race guard (an extension — no openEHR spec governs storage integrity).';

-- ── template_store ───────────────────────────────────────────────────────────
-- Operational templates (OPT 1.4 XML); the parsed model is built in the
-- application, not stored here.
-- DUAL IDENTITY (SM I_DEFINITION_ADL14 takes a UUID handle; the ITS-REST wire
-- addresses templates by template_id): the uuid `id` is the
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

COMMENT ON TABLE template_store IS 'Operational templates (OPT 1.4 XML). Dual identity: uuid id = the SM UUID handle (SM I_DEFINITION_ADL14); template_id = the ITS-REST wire address. Template versioning is not spec-required — replace-in-place.';
COMMENT ON COLUMN template_store.id IS 'The SM OPT-by-UUID handle (SM openehr_platform master04-definition_package.adoc §Archetypes and Templates: "In ADL 1.4 ... OPTs are identified with UUIDs").';
-- The ITS-REST TemplateMetadata.version (optional + deprecated) is NOT stored:
-- it is the `.vN` version axis of template_id (filter_version: "taken from
-- template_id"), a pure function of the id, so it is derived on read rather than
-- denormalised into a column (see crate::templates::identity::template_version).
COMMENT ON COLUMN template_store.template_id IS 'The wire address (ITS-REST DEFINITION API; registered in template_ref, the vo_version.template_id FK target); also the source of the reported TemplateMetadata.version (its `.vN` axis).';
-- Case-insensitive uniqueness of TEMPLATE_ID (BASE base_types master05
-- §Composite Identifiers and Case: identifier equality — and thus uniqueness —
-- is case-insensitive, so a case variant is the SAME template id and the
-- upload endpoint rejects it as a duplicate, ITS-REST
-- 409_template_already_exists). The exact-case UNIQUE above stays as the
-- natural-key anchor; this functional unique index is the race-free
-- case-insensitive guard.
CREATE UNIQUE INDEX ux_template_store_template_id_ci
    ON template_store (lower(template_id));

-- ── vo_version ───────────────────────────────────────────────────────────────
-- One row per version of a versioned object (COMPOSITION/EHR_STATUS/EHR_ACCESS/
-- FOLDER + demographic party roots + PARTY_RELATIONSHIP). EHR_ACCESS is a
-- versioned object per RM ehr §"EHR Creation", versioned "via the normal
-- mechanism". Demographic kinds carry a NULL ehr_id (no owning EHR — RM
-- demographic content stands alone).
--
-- HOLDING THE VERSIONS AS ROWS rather than physically inside a VERSIONED_OBJECT
-- is EXPLICITLY sanctioned by the RM, not merely unaddressed by it: RM common
-- master06 §Overview says of its own figure, "Although the figure implies
-- physical containment of Versions by a Versioned object, this is only one
-- possible implementation. Other implementations (e.g. using orthodox
-- relational structures) might use references, separate compressed copies, or
-- any other mechanism." The temporal MECHANICS below — sys_period, the GiST
-- exclusion/partial-index non-overlap machinery, the opaque sys_version
-- ordinal — are our own storage design; no openEHR spec governs those.
--
-- Version-tree model (RM common master06 §The 'Virtual Version Tree' / §Distributed
-- versioning — "To support branching, a further pair of numbers is added …
-- branching version identifiers [are required] when local modifications are
-- made to versions copied from elsewhere"):
--   * `sys_version` is an opaque per-vo COMMIT ORDINAL (1..n across trunk AND
--     branch commits) — the node/vo_attestation FK key and the AQL join key,
--     NOT the wire version number;
--   * the spec-facing VERSION_TREE_ID lives in `trunk_version` +
--     `branch_number`/`branch_version` (0/0 = a trunk row; >= 1 each on a
--     branch row); the wire form is `trunk[.branch_number.branch_version]`;
--   * global uniqueness is the spec tuple {object_id, creating_system_id,
--     version_tree_id} (uq_vo_version_tree);
--   * temporal non-overlap holds PER LINEAGE (a branch coexists in time with
--     the trunk by design): one EXCLUDE for the trunk lineage, one per branch.
-- fillfactor 90: one close-out UPDATE per supersession (storage tuning — no
-- spec governs fill factors).
CREATE TABLE vo_version (
    vo_id           uuid NOT NULL,
    kind            text NOT NULL,
    ehr_id          uuid,
    sys_version     integer NOT NULL,
    -- VERSION_TREE_ID columns (see the table header). For a trunk row
    -- trunk_version is the wire version number; for a branch row the trunk
    -- version the branch forks from.
    trunk_version   integer NOT NULL,
    branch_number   integer NOT NULL DEFAULT 0,
    branch_version  integer NOT NULL DEFAULT 0,
    -- Validity interval; the current (latest trunk) version is the one with
    -- upper_inf(sys_period) AND branch_number = 0. Committal time is the only
    -- server-managed temporal axis (RM common master06 §Committal and Audits:
    -- time_committed "should reflect the time of committal to an EHR server");
    -- the interval itself is our own storage design — no openEHR spec governs
    -- a validity range on a stored version row.
    sys_period      tstzrange NOT NULL,
    -- ORIGINAL_VERSION.lifecycle_state: the numeric version_lifecycle_state code
    -- (532 complete, 553 incomplete, 523 deleted, 800 inactive, 801 abandoned).
    -- A logical delete writes a content-less version with state 523 (RM common
    -- master06 §Logical Deletion) — never a physical delete. 553 `incomplete`
    -- relaxes content validity ("mandatory attributes may be absent ... data may
    -- be missing, but it may not be wrong" — master06 §Incomplete Content), so
    -- content is never NOT NULL.
    lifecycle_state text NOT NULL DEFAULT '532',
    -- The immutable identity of the system that CREATED this version (RM common
    -- master06 §Distributed versioning): the middle segment of the
    -- OBJECT_VERSION_ID {object_id, creating_system_id,
    -- version_tree_id}. Reconstructed from storage, never re-derived from live
    -- config, so a config change cannot mutate a historical uid or invalidate a
    -- signature. Written on every version (local: our system_id; import:
    -- preserved) — never an empty-string sentinel.
    creating_system_id text NOT NULL,
    -- ORIGINAL_VERSION.preceding_version_uid (0..1): the full OBJECT_VERSION_ID
    -- of the actual preceding version, STORED at commit (and preserved verbatim
    -- on import) — it cannot be synthesized arithmetically once branches and
    -- imports exist (the preceding version may carry a different
    -- creating_system_id). NULL for a first version.
    preceding_version_uid text,
    -- VERSION.signature (RM common master06 §"Digital Signature"): 0..1, opaque
    -- radix-64 (OpenPGP RFC 4880 or a SHA-256 digest). Canonicalisation is
    -- spec-TBD (master06 marks the exact serialisation "To Be Determined").
    -- Historical versions may carry none. On an IMPORTED_VERSION row this is the
    -- WRAPPER's own signature — "the IMPORTED_VERSION instance will carry its
    -- own signature which signifies the act of importing and making available
    -- locally an ORIGINAL_VERSION from another system" (master06 §Digital
    -- Signature); the wrapped original's signature lives in wrapped_original.
    signature       text,
    -- Whether `signature` was supplied verbatim by the committing client (an
    -- author signature over another agreed serialisation — RM common master06
    -- §Digital Signature) rather than generated by this server. Client-supplied
    -- signatures are stored byte-for-byte and are NEVER re-verified at read (a
    -- foreign canonical form cannot be recomputed); read-time verification
    -- applies only to our own signatures. No openEHR spec governs read-time
    -- verification timing — our own design (server-side integrity hardening).
    signature_client_supplied boolean NOT NULL DEFAULT false,
    -- IMPORTED_VERSION discriminator + the wrapped ORIGINAL_VERSION's own
    -- wrapper-level provenance, held as VERBATIM canonical openEHR JSON:
    --   { "contribution": <OBJECT_REF>, "commit_audit": <AUDIT_DETAILS>,
    --     "signature": <String, optional> }
    -- NULL ⇒ the row is an ORIGINAL_VERSION created locally; NOT NULL ⇒ the row
    -- is an IMPORTED_VERSION whose `item` is reassembled from this fragment plus
    -- the row's own identity columns and node rows.
    --
    -- RM common master06 §Committal and Audits: for a Version committed as an
    -- IMPORTED_VERSION "both the contribution and commit_audit of the latter
    -- object correspond to the local act of committal, while the knowledge of
    -- the original Contribution and committal are retained inside the wrapped
    -- ORIGINAL_VERSION instance" — so the row's contribution_id/audit_id/
    -- signature columns are the LOCAL import act and this column is the foreign
    -- one. §Copying: "the ORIGINAL_VERSION instance is never modified", hence
    -- verbatim fragments rather than re-encoded columns; and the local columns
    -- are what makes "the commit times always reflect the local (more recent)
    -- act of committal" true for time_created / Last-Modified / time travel.
    -- The wrapped contribution CANNOT be a foreign key: it names a CONTRIBUTION
    -- in the SOURCE system, which this repository does not hold.
    wrapped_original jsonb COMPRESSION lz4,
    -- ORIGINAL_VERSION.other_input_version_uids: merge provenance (RM common
    -- master06 §Version Merging). PRODUCE-only on the released wire: the commit
    -- wire declares no merge shape, so a CONTRIBUTION member carrying it is
    -- refused, and the column is written only by the routes that reproduce a
    -- FOREIGN ORIGINAL_VERSION verbatim (the EHR-Extract import and the archive
    -- load). NULL when not a merge; is_merged is its derived boolean
    -- (Is_merged_validity).
    other_input_version_uids jsonb,
    contribution_id uuid NOT NULL,
    audit_id        uuid NOT NULL,
    template_id     text,
    -- The version's assembled canonical body (openEHR canonical JSON, the
    -- openehr-its encoding), materialized at write from the SAME in-memory
    -- value the node rows are decomposed from — a point read serves one
    -- detoast instead of re-aggregating the node subtree (TOAST keeps it
    -- out of the main row, so meta-only scans stay slim). The node rows
    -- remain the AQL source of truth; NULL on a logically deleted version
    -- (no content — RM common master06 §Logical Deletion).
    body            jsonb COMPRESSION lz4,
    CONSTRAINT pk_vo_version PRIMARY KEY (vo_id, sys_version),
    CONSTRAINT uq_vo_version_tree UNIQUE
        (vo_id, creating_system_id, trunk_version, branch_number, branch_version),
    CONSTRAINT ck_vo_version_sys_version_positive CHECK (sys_version >= 1),
    CONSTRAINT ck_vo_version_trunk_version_positive CHECK (trunk_version >= 1),
    -- A row is a trunk row (0, 0) or a branch row (>= 1, >= 1) — never mixed
    -- (BASE VERSION_TREE_ID: `trunk_version [ '.' branch_number '.'
    -- branch_version ]`, both branch parts start at 1).
    CONSTRAINT ck_vo_version_branch_pair CHECK (
        (branch_number = 0 AND branch_version = 0)
        OR (branch_number >= 1 AND branch_version >= 1)
    ),
    CONSTRAINT ck_vo_version_kind CHECK (kind IN (
        'COMPOSITION', 'EHR_STATUS', 'EHR_ACCESS', 'FOLDER',
        'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE', 'PARTY_RELATIONSHIP'
    )),
    CONSTRAINT ck_vo_version_lifecycle_state CHECK (lifecycle_state IN ('532', '553', '523', '800', '801')),
    -- An IMPORTED_VERSION's wrapped original always carries the two mandatory
    -- VERSION attributes (contribution 1..1, commit_audit 1..1 — RM common
    -- version.adoc §Attributes); `signature` is 0..1 and may be absent.
    CONSTRAINT ck_vo_version_wrapped_original CHECK (
        wrapped_original IS NULL
        OR (jsonb_typeof(wrapped_original) = 'object'
            AND wrapped_original ? 'contribution'
            AND wrapped_original ? 'commit_audit')
    ),
    -- NOTE: two GiST EXCLUDE (temporal non-overlap) constraints were
    -- REMOVED here after measurement: GiST exclusion inserts serialize under
    -- concurrency (PostgreSQL 18 docs, "Exclusion Constraints") and were a
    -- prime "everything slows together" contributor on the write path (the
    -- reference implementation's version table pays a plain btree PK). The
    -- non-overlap INVARIANT (master06: one valid version per lineage at any
    -- instant) is unchanged and enforced by construction instead:
    --   * the partial unique btrees below admit at most ONE open row per
    --     lineage (trunk / each branch);
    --   * every regular write closes the open row and inserts the successor
    --     in one transaction at the same `now()` (half-open ranges meet
    --     exactly — no overlap possible), serialized per vo by the advisory
    --     lock;
    --   * the admin archive load — the only path writing explicit historical
    --     periods — runs a per-EHR overlap audit after loading and fails the
    --     record on a violation.
    -- No openEHR spec governs the enforcement mechanism — our own design;
    -- the semantics stay master06.
    CONSTRAINT fk_vo_version_contribution FOREIGN KEY (contribution_id) REFERENCES contribution (id),
    CONSTRAINT fk_vo_version_audit FOREIGN KEY (audit_id) REFERENCES audit (id),
    CONSTRAINT fk_vo_version_template FOREIGN KEY (template_id) REFERENCES template_ref (template_id),
    CONSTRAINT fk_vo_version_ehr FOREIGN KEY (ehr_id) REFERENCES ehr (id) ON DELETE CASCADE
) WITH (fillfactor = 90);
-- LATEST_VERSION (= the current trunk tip, RM common master06
-- latest_trunk_version) = this partial index; one current trunk version per
-- vo_id. Each open branch additionally has its own current tip.
CREATE UNIQUE INDEX uq_vo_version_current ON vo_version (vo_id)
    WHERE upper_inf(sys_period) AND branch_number = 0;
CREATE UNIQUE INDEX uq_vo_version_branch_current ON vo_version
    (vo_id, creating_system_id, trunk_version, branch_number)
    WHERE upper_inf(sys_period) AND branch_number > 0;
-- A TRUNK POSITION is unique across creating systems, while a BRANCH id is
-- not. RM common master06 §Distributed Versioning globally identifies a
-- version by the tuple {object_id, creating_system_id, version_tree_id} —
-- which is uq_vo_version_tree above — and that tuple alone would admit
-- (vo, sysA, 2, 0, 0) AND (vo, sysB, 2, 0, 0): one container with two versions
-- claiming trunk position 2. The model forbids exactly that. §Copying
-- §Subsequent Local Modifications: "When new versions are added locally to a
-- copied object, the local system id is recorded in the uid.creating_system_id()
-- attribute, while branching numbering is used in the uid.version_tree_id()" —
-- so a second system never extends the trunk of a copied container, it
-- branches; and §Moving Version Containers has the trunk CONTINUE its
-- increment under the new system's id, never restart. The trunk line is one
-- global sequence whatever system minted each node. Branch ids legitimately
-- collide across systems (each system allocates branch numbers locally,
-- "Two places are indicated on the diagram where identification clashes could
-- have occurred, but are prevented due to the use of the 3-part unique Version
-- identifier scheme"), which is why creating_system_id stays in the tuple
-- constraint and out of this one.
CREATE UNIQUE INDEX uq_vo_version_trunk_position ON vo_version (vo_id, trunk_version)
    WHERE branch_number = 0;
CREATE INDEX idx_vo_version_ehr ON vo_version (ehr_id, kind);
-- The most-executed predicate family: "the current trunk row(s) of this EHR's
-- objects of one kind" (EHR_STATUS/FOLDER current reads, EHR summaries, the
-- directory join) and the persistent-duplicate probe. Partial over open trunk
-- rows only, so those lookups never touch an EHR's version history and the
-- index carries exactly one entry per live object.
CREATE INDEX idx_vo_version_current_ehr ON vo_version (ehr_id, kind)
    WHERE upper_inf(sys_period) AND branch_number = 0;
CREATE INDEX idx_vo_version_current_template ON vo_version (ehr_id, template_id)
    WHERE upper_inf(sys_period) AND branch_number = 0 AND template_id IS NOT NULL;
CREATE INDEX idx_vo_version_contribution ON vo_version (contribution_id);
CREATE INDEX idx_vo_version_audit ON vo_version (audit_id);
CREATE INDEX idx_vo_version_template ON vo_version (template_id) WHERE template_id IS NOT NULL;

COMMENT ON TABLE vo_version IS 'One temporal row per version of a versioned object (RM common master06 version tree). Non-overlap holds per lineage by construction (one open row per lineage via the partial unique indexes; close-then-insert at one now() per write; load-path overlap audit); ALL_VERSIONS = unfiltered, LATEST_VERSION (latest trunk) = uq_vo_version_current.';
COMMENT ON COLUMN vo_version.sys_version IS 'Opaque per-vo commit ordinal (1..n across trunk AND branch commits) — the node/vo_attestation FK key and AQL join key. NOT the wire version number: the VERSION_TREE_ID lives in trunk_version/branch_number/branch_version.';
COMMENT ON COLUMN vo_version.trunk_version IS 'VERSION_TREE_ID first part. For a trunk row this is the wire version number; for a branch row the trunk version the branch forks from.';
COMMENT ON COLUMN vo_version.branch_number IS 'VERSION_TREE_ID second part; 0 = trunk row, >= 1 = branch (numbered per fork point, RM common master06 §The ''Virtual Version Tree'').';
COMMENT ON COLUMN vo_version.branch_version IS 'VERSION_TREE_ID third part; 0 = trunk row, >= 1 = position on the branch.';
COMMENT ON COLUMN vo_version.sys_period IS 'Validity interval [committed, superseded); the current trunk version has upper_inf(sys_period) AND branch_number = 0. Committal time is the only server-managed temporal axis (RM common master06 §Committal and Audits). The interval itself is our own storage design — no openEHR spec governs it.';
COMMENT ON COLUMN vo_version.creating_system_id IS 'Immutable per-version creating-system id — the OBJECT_VERSION_ID middle segment (RM common master06 §Distributed versioning). Reconstructed from storage, never live config; never an empty-string sentinel.';
COMMENT ON COLUMN vo_version.preceding_version_uid IS 'ORIGINAL_VERSION.preceding_version_uid (0..1, full OBJECT_VERSION_ID) — stored at commit from the actual preceding row; preserved verbatim on import; NULL for a first version.';
COMMENT ON COLUMN vo_version.lifecycle_state IS 'ORIGINAL_VERSION.lifecycle_state, coded from the openEHR `version lifecycle state` group (RM common master06 §Version Lifecycle: 532 complete, 553 incomplete, 523 deleted, 800 inactive, 801 abandoned). 523 = logical delete (content-less version, §Logical Deletion). 553 relaxes content validity (§Incomplete Content).';
COMMENT ON COLUMN vo_version.signature IS 'VERSION.signature (0..1), opaque radix-64 — on an imported row the IMPORTED_VERSION wrapper''s own signature, which "signifies the act of importing" (RM common master06 §Digital Signature). Canonicalisation is spec-TBD (master06 marks the exact serialisation "To Be Determined").';
COMMENT ON COLUMN vo_version.signature_client_supplied IS 'True when signature was supplied verbatim by the committing client (foreign — never re-verified at read); false for a server-generated signature (RM common master06 §Digital Signature). Our own design (verify-on-read hardening).';
COMMENT ON COLUMN vo_version.wrapped_original IS 'IMPORTED_VERSION discriminator: NULL = a locally created ORIGINAL_VERSION; NOT NULL = the wrapped ORIGINAL_VERSION''s own {contribution, commit_audit, signature?} as verbatim canonical openEHR JSON, while the row''s contribution_id/audit_id/signature carry the LOCAL act of committal (RM common master06 §Committal and Audits, §Copying).';
COMMENT ON COLUMN vo_version.other_input_version_uids IS 'ORIGINAL_VERSION merge provenance (master06 §Version Merging); NULL when not a merge; is_merged = derived (Is_merged_validity).';

-- ── ehr_folder ───────────────────────────────────────────────────────────────
-- One row per folder hierarchy of an EHR (RM ehr master04 §Folders: "at any
-- time, an entirely new Folder hierarchy may be added, which will be referenced
-- by a new member of the `EHR._folders_` attribute"). `rank` order is the
-- `EHR.folders` list order (1-based); `EHR.directory` = the first LIVE hierarchy
-- (RM ehr §EHR Class `Directory_in_folders`: `folders /= Void implies
-- folders.item(1) = directory`). Ranks are APPEND-ONLY and never reused — a
-- deleted hierarchy keeps its rank slot. Each referenced hierarchy is its own
-- versioned object (rows in `vo_version`/`node`); this table only records
-- membership + order. No openEHR spec governs the storage mechanism itself (our
-- own storage design). `vo_id` carries no FK (vo_version is keyed per version,
-- not per object) — a service-wide UNIQUE stands in. The `ehr_id` FK cascades
-- like every other ehr-scoped table so a `DELETE FROM ehr` (admin purge) removes
-- the membership rows too.
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
-- The decomposed content: one row per RM structure node, per version. The
-- nested-set interval (num..=num_cap) makes AQL CONTAINS an integer range join
-- (never a JSON walk). ehr_id is nullable (demographic content has none).
--
-- DECOMPOSING the RM containment into rows rather than storing each versioned
-- object as one physical document is EXPLICITLY sanctioned by the RM, not
-- merely unaddressed by it: RM common master06 §Overview, of the containment
-- its own figure draws — "Although the figure implies physical containment of
-- Versions by a Versioned object, this is only one possible implementation.
-- Other implementations (e.g. using orthodox relational structures) might use
-- references, separate compressed copies, or any other mechanism." The column
-- set, the nested-set index and the promoted predicate columns below are our
-- own storage design — no openEHR spec governs those.
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
    -- Archetype-subsumption columns, parsed from a full archetype HRID and
    -- comparison-normalized (lowercased); NULL on at-code/id-code nodes. Parts
    -- per BASE base_types master05 §Archetype Identifiers:
    --   archetype_id   = qualified_rm_entity '.' domain_concept '.v' version_id,
    --   domain_concept = concept_name { '-' specialisation }.
    -- arch_entity = qualified_rm_entity (e.g. openehr-ehr-observation),
    -- arch_concept = the full domain_concept incl. specialisation segments
    -- (e.g. laboratory-glucose), arch_major = the .v major version.
    -- These drive query subsumption (BASE architecture_overview master10
    -- §Design-time Relationships: "data created with any specialised archetype
    -- will always be matched by queries based on the parent archetype"): a query
    -- naming a parent matches a specialisation child via a `concept-%` prefix
    -- scan within the same entity + major, the major boundary being hard (AM
    -- master07 §Querying). Stored lowercased for case-insensitive comparison
    -- (master05 §"Composite Identifiers and Case"), as is `archetype` itself
    -- (case-folded at write by storage::codec); `data` stays canonical.
    arch_entity  text,
    arch_concept text,
    arch_major   integer,
    name        text,
    -- Materialized path from the root; byte-order under COLLATE "C" equals tree
    -- order (used only for reassembly, not as an AQL predicate).
    path        text COLLATE "C" NOT NULL,
    -- The node's canonical openEHR JSON fragment verbatim, structure children
    -- pruned (no aliasing, no synthetic fields — the stored fragment IS the
    -- canonical ITS-JSON encoding, so storage == API). lz4-compressed
    -- (storage choice; COMPRESSION precedes constraints).
    data        jsonb COMPRESSION lz4 NOT NULL,
    -- Promoted EVENT_CONTEXT.start_time.value on the COMPOSITION root row
    -- (num = 0); NULL elsewhere and for context-less persistent compositions.
    -- Serves the AQL dashboard ORDER BY through the partial index below
    -- instead of a per-candidate-row jsonb extraction (a measured hot path).
    -- Populated via
    -- ext.openehr_timestamp; the (rm_type, path)→column registry is
    -- app/ferroehr/src/storage/promoted.rs. Our own storage design — no
    -- openEHR spec governs storage columns.
    context_start timestamptz,
    CONSTRAINT pk_node PRIMARY KEY (vo_id, sys_version, num),
    -- num_cap >= num and parent above (pre-order) — the nested-set invariant
    -- (nested-set integrity — our own storage design). The root (num = 0,
    -- parent_num = 0) is exempt from the parent
    -- ordering check.
    CONSTRAINT ck_node_num_cap CHECK (num_cap >= num),
    CONSTRAINT ck_node_parent CHECK (num = 0 OR parent_num < num),
    -- DEFERRABLE so a multi-row version commit (vo_version + its many node rows)
    -- can be reordered within one transaction; INITIALLY IMMEDIATE
    -- keeps the default check-at-statement-end behaviour.
    CONSTRAINT fk_node_vo_version FOREIGN KEY (vo_id, sys_version)
        REFERENCES vo_version (vo_id, sys_version) ON DELETE CASCADE
        DEFERRABLE INITIALLY IMMEDIATE
);
-- The rm_type-only CONTAINS anchor path (a class filter with no archetype).
-- Deliberately WITHOUT the archetype column (measured 2026-08-25, #2698):
-- 83% of node rows carry at-code archetype text, so the composite's key
-- bytes dominated the write path's index maintenance while every measured
-- anchor plan either used the subsumption index below (full-HRID anchors)
-- or filtered archetype after this rm_type probe at identical latency. The
-- archetype column stays case-folded at write (storage::codec; BASE
-- base_types master05 §"Composite Identifiers and Case") so the residual
-- filter is plain equality with honest statistics.
CREATE INDEX idx_node_rm_type ON node (rm_type);
-- Archetype-subsumption scan (BASE architecture_overview master10 §Design-time
-- Relationships; AM master07 §Querying): a parent-archetype predicate resolves
-- to arch_entity = $entity AND arch_major = $major AND (arch_concept = $concept
-- OR arch_concept LIKE $concept || '-%'). text_pattern_ops on arch_concept makes
-- the specialisation-child prefix scan (`LIKE 'concept-%'`) index-usable under
-- the pool's non-C collation. rm_type LEADS: the AQL engine emits the
-- CONTAINS class filter (rm_type) alongside every archetype predicate, and
-- the single full-boundary index serves the anchor scan in ONE probe — the
-- measured cross-EHR profile showed the old (entity, concept, major) order
-- degrading to a BitmapAnd with a second whole-type bitmap, whose build
-- materializes every matching row before the first result and defeats
-- LIMIT early termination at corpus scale.
CREATE INDEX idx_node_arch_subsume ON node (rm_type, arch_entity, arch_major, arch_concept text_pattern_ops)
    WHERE arch_entity IS NOT NULL;

-- NOTE: two speculative jsonb indexes were REMOVED here after the measured
-- repricing: a gin(data
-- jsonb_ops) index (the AQL engine emits no GIN-servable operator — CONTAINS
-- is the nested-set interval join) and an ext.openehr_magnitude(data->'value')
-- expression index (the generator never emits that expression verbatim). Both
-- were per-node-row write amplification inside the held commit transaction —
-- ~34 rows for a populated vital-signs composition, hundreds for an IPS.
-- Measured ordering hot paths are served by promoted columns instead:
-- The dashboard ORDER-BY partial index over the promoted column: the COMPOSITION
-- roots of one EHR ordered by context start-time (the AQL generator emits
-- rm_type + ehr_id filters, never num = 0 — COMPOSITION occurs only at the
-- root, so the predicate is exactly what the query proves).
CREATE INDEX idx_node_context_start ON node (ehr_id, context_start)
    WHERE rm_type = 'COMPOSITION';

COMMENT ON TABLE node IS 'Decomposed versioned-object content: one row per RM structure node, per version (our own storage design — openEHR defines no SQL schema). Nested-set interval num..=num_cap makes CONTAINS an integer range join.';
COMMENT ON COLUMN node.num IS 'Pre-order number within the versioned object (root = 0).';
COMMENT ON COLUMN node.num_cap IS 'Max num in this node''s subtree: the subtree is num..=num_cap (AQL CONTAINS).';
COMMENT ON COLUMN node.parent_num IS 'num of the parent structure node (root points at itself/0).';
COMMENT ON COLUMN node.citem_num IS 'num of the nearest ancestor carrying an archetype id.';
COMMENT ON COLUMN node.context_start IS 'Promoted EVENT_CONTEXT.start_time.value (timestamptz) on the COMPOSITION root (num = 0); NULL elsewhere and for context-less persistent compositions. Serves the AQL dashboard ORDER BY (our own storage design — no openEHR spec governs it; mapping in storage/promoted.rs).';
COMMENT ON COLUMN node.arch_entity IS 'qualified_rm_entity of a full archetype HRID, lowercased for comparison (BASE base_types master05 §Archetype Identifiers); NULL on at/id-code nodes. Drives archetype-subsumption querying (master10 §Design-time Relationships).';
COMMENT ON COLUMN node.arch_concept IS 'Full domain_concept (incl. specialisation segments, e.g. laboratory-glucose) of a full archetype HRID, lowercased (BASE base_types master05); a parent query matches a child via a `concept-%` prefix (master10 §Design-time Relationships).';
COMMENT ON COLUMN node.arch_major IS 'Major version (.v major) of a full archetype HRID; the interface-reference major boundary is hard (AM master07 §Querying). NULL on at/id-code nodes.';
COMMENT ON COLUMN node.path IS 'Materialized path from the root; byte-order under COLLATE "C" equals tree order. Reassembly only — never an AQL predicate.';
COMMENT ON COLUMN node.data IS 'The node''s canonical openEHR JSON fragment verbatim (ITS-JSON encoding), structure children pruned — storage == API, no synthetic fields.';

-- ── vo_attestation ───────────────────────────────────────────────────────────
-- ATTESTATION storage (RM common master06 §Attestation; the class is defined in
-- master04-generic_package.adoc §Attestation): a new ATTESTATION is appended to
-- an existing ORIGINAL_VERSION's attestations list WITHOUT a new version
-- ("Attestations can be added at any time after committal of the content being
-- attested"; master06 §Contributions lists attestation of an item as a change
-- achieved by "new Attestation objects to existing Versions"), committed via a
-- contribution. One row per attestation, canonical RM ATTESTATION verbatim in
-- data (no synthetic fields — the stored value is the canonical ITS-JSON
-- encoding).
CREATE TABLE vo_attestation (
    id              uuid NOT NULL DEFAULT uuidv7(),
    vo_id           uuid NOT NULL,
    sys_version     integer NOT NULL,
    contribution_id uuid NOT NULL,
    time_committed  timestamptz NOT NULL DEFAULT now(),
    -- Whether this ATTESTATION was present ON THE VERSION at the act of
    -- committal (RM common master06 §Attestation, "Signing content at
    -- committal"; SM UML/classes/update_version.adoc §Attributes carries
    -- `attestations` on the client-supplied UPDATE_VERSION) rather than added
    -- later ("Attestations can be added at any time after committal of the
    -- content being attested"). The signed canonical form of a VERSION is "the
    -- entire Version object" minus `signature` (master06 §Digital Signature),
    -- so an at-committal attestation is INSIDE it and an after-committal one is
    -- outside; this column is what keeps the two apart at read time.
    at_committal    boolean NOT NULL,
    data            jsonb NOT NULL,
    CONSTRAINT pk_vo_attestation PRIMARY KEY (id),
    CONSTRAINT fk_vo_attestation_vo_version FOREIGN KEY (vo_id, sys_version)
        REFERENCES vo_version (vo_id, sys_version) ON DELETE CASCADE,
    CONSTRAINT fk_vo_attestation_contribution FOREIGN KEY (contribution_id) REFERENCES contribution (id)
);
CREATE INDEX idx_vo_attestation_version ON vo_attestation (vo_id, sys_version);
CREATE INDEX idx_vo_attestation_contribution ON vo_attestation (contribution_id);

COMMENT ON TABLE vo_attestation IS 'ATTESTATION storage (RM common master06 §Attestation; class in master04-generic_package.adoc §Attestation): appended to an ORIGINAL_VERSION''s attestations list without a new version; canonical ATTESTATION verbatim in data.';
COMMENT ON COLUMN vo_attestation.at_committal IS 'True when the ATTESTATION was on the VERSION at the act of committal (RM common master06 §Attestation "Signing content at committal"; SM update_version.adoc UPDATE_VERSION.attestations) and is therefore INSIDE the version''s signed canonical form (master06 §Digital Signature: "serialising the entire Version object", signature Void); false for one added afterwards ("Attestations can be added at any time after committal"), which stays outside it.';

-- ── stored_query ─────────────────────────────────────────────────────────────
-- Stored AQL queries, addressed by qualified name + SEMVER version (ITS-REST
-- specifications/docs/query/Qualified_query_name.md: "Stored queries are
-- identified by their name, used as `qualified_query_name`, and an optional
-- `version` number"; "`[{namespace}::]{query-name}`"; "The `version` identifier
-- is in the format specified by SEMVER"). Columns: reverse-domain namespace
-- (default "misc" — the namespace is optional on the wire, so an unqualified
-- name needs a stored stand-in; no openEHR spec governs the substitute value,
-- our own design), semantic id, semver, formalism.
CREATE TABLE stored_query (
    reverse_domain_name text NOT NULL,
    semantic_id         text NOT NULL,
    semver              text NOT NULL DEFAULT '0.0.0',
    query_type          text NOT NULL DEFAULT 'AQL',
    query_text          text NOT NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_stored_query PRIMARY KEY (reverse_domain_name, semantic_id, semver)
);

COMMENT ON TABLE stored_query IS 'Stored AQL queries, addressed by qualified name + SEMVER version (ITS-REST specifications/docs/query/Qualified_query_name.md).';

-- ── item_tag ─────────────────────────────────────────────────────────────────
-- Item tags (RM common master07-tags.adoc ITEM_TAG; the ITS-REST tags routes).
-- Loose coupling: tags are mutable, EHR-scoped, outside the version chain,
-- require no contribution. ehr_id is nullable (a party may be tagged too).
-- No openEHR spec governs the SQL itself — our own design.
CREATE TABLE item_tag (
    id           uuid NOT NULL DEFAULT uuidv7(),
    ehr_id       uuid,
    -- The tagged target's uid. INTENTIONALLY FK-LESS (RM common master07 tags:
    -- ITEM_TAG.target "may be a VERSIONED_OBJECT<T> or a VERSION<T>"), i.e. it
    -- is deliberately outside the version chain, so a FK into vo_version (which
    -- is keyed per version) would be wrong. Referential looseness is by design.
    target_vo_id uuid NOT NULL,
    -- The '{creating_system_id}::{version_tree_id}' tail of an addressed
    -- VERSION target, verbatim as addressed; NULL = the tag targets the
    -- VERSIONED_OBJECT container. Container and per-version tag sets are
    -- disjoint collections of the same target_vo_id.
    target_version text,
    target_type  text NOT NULL,
    key          text NOT NULL,
    value        text,
    target_path  text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_item_tag PRIMARY KEY (id),
    -- ITEM_TAG identity: ITS-REST overview Requests_and_responses.md
    -- ("uniquely identified by their key and target_path pair", within one
    -- target — container or specific VERSION). NULLS NOT DISTINCT so a NULL
    -- target_version / target_path participates in the identity.
    CONSTRAINT uq_item_tag_identity UNIQUE NULLS NOT DISTINCT
        (ehr_id, target_vo_id, target_version, key, target_path),
    -- The change-controlled resource types a tag may target. FOLDER is one of
    -- them by name: the ITS-REST wrapper headers are defined "for associating
    -- tags with change-controlled resources (e.g. COMPOSITION, EHR_STATUS,
    -- FOLDER, etc.)" (specifications/docs/overview/Requests_and_responses.md
    -- §openehr-item-tag and openehr-version-item-tag), and the DIRECTORY write
    -- routes carry those headers. The release defines no dedicated
    -- /directory/.../tags operations, so FOLDER tags are reachable through the
    -- wrapper headers and the EHR-wide listing (ehr_tags_get) only.
    CONSTRAINT ck_item_tag_target_type CHECK (target_type IN (
        'COMPOSITION', 'EHR_STATUS', 'FOLDER',
        'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE'
    )),
    CONSTRAINT fk_item_tag_ehr FOREIGN KEY (ehr_id) REFERENCES ehr (id) ON DELETE CASCADE
);
-- NOTE: no dedicated index on `key` alone. RM ehr master04-ehr_package.adoc
-- §Tags states the indexing OBLIGATION ("in a typical implementation, tags
-- would be indexed") but names no index and no access path, so the choice is
-- ours. uq_item_tag_identity is a btree over
-- (ehr_id, target_vo_id, target_version, key, target_path) and already serves
-- every addressed read the released wire defines — all of them name a target,
-- so the leading columns are bound. The one key-leading access is the EHR-wide
-- listing's optional `tag_key` filter (ehr_tags_get), which is already scoped
-- to one EHR by the same index's leading column; PG 18 B-tree skip scan covers
-- the remainder. A key-leading index is a measured-need addition, not a
-- speculative one.

COMMENT ON TABLE item_tag IS 'Item tags (RM common master07-tags.adoc ITEM_TAG; identity per ITS-REST Requests_and_responses.md: key + target_path within one target). Mutable, EHR-scoped, outside the version chain.';
COMMENT ON COLUMN item_tag.target_vo_id IS 'NOTE: intentionally FK-less (RM common master07: ITEM_TAG.target may reference a container OR a specific VERSION), so it is deliberately outside the version chain.';
COMMENT ON COLUMN item_tag.target_version IS 'The {creating_system_id}::{version_tree_id} tail of a VERSION-addressed target (RM ITEM_TAG.target: UID_BASED_ID "may be a VERSIONED_OBJECT<T> or a VERSION<T>"); NULL = container target.';

-- ── archetype_store (SM-2, I_DEFINITION_ADL14) ───────────────────────────────
-- ADL 1.4 source archetypes, keyed by their human-readable ARCHETYPE_ID (not a
-- UUID — reserved for OPTs/ADL2). Source ADL text stored verbatim; upload
-- replaces an existing id (ON CONFLICT DO UPDATE). Separate identity scheme per
-- formalism (SM I_DEFINITION_QUERY: stored queries are addressed by qualified
-- name + version).
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
    -- The artefact's declared `specialize` parent (AUTHORED_ARCHETYPE
    -- .parent_archetype_id), extracted from the validated source at upload;
    -- NULL for a non-specialised artefact. This is the ONLY lineage source for
    -- an AOM2-era identifier: AM Identification master03 §Legacy ADL 1.4
    -- Semantics strips the '-' concept separator of all meaning, and master07
    -- §Supporting Archetype-based Querying rules that "for specialised
    -- archetypes, the specialisation lineage can only be obtained from the
    -- operational form of the archetype". The AQL archetype predicate reads
    -- the (hrid, parent_hrid) edge set to widen a parent query to its stored
    -- specialisation children.
    parent_hrid text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_adl2_artefact PRIMARY KEY (hrid),
    CONSTRAINT ck_adl2_artefact_kind CHECK (kind IN ('archetype', 'template', 'operational_template'))
);

COMMENT ON TABLE adl2_artefact IS 'SM-2 ADL2 artefacts (I_DEFINITION_ADL2), keyed by ARCHETYPE_HRID; kind ∈ archetype/template/operational_template; verbatim ADL2 text.';
COMMENT ON COLUMN adl2_artefact.parent_hrid IS 'Declared `specialize` parent HRID (NULL when unspecialised); the archetype-lineage edge the AQL archetype predicate resolves an AOM2-era parent query through (AM Identification master07 §Supporting Archetype-based Querying).';

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
    -- LOCATION_DESC {system_id, uri?, description?}, canonical JSON (NOTE).
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
-- Versioned-object archive MARKERS realizing SM I_ADMIN_ARCHIVE
-- (`archive_ehrs` / `archive_parties`: "Move selected EHRs to archival
-- storage" — SM openehr_platform master15-admin_service.adoc via
-- UML/classes/i_admin_archive.adoc). The SM says only "move ... to archival
-- storage" and defines no storage form; no openEHR spec governs one, so the
-- marker IS our archival storage — our own design, and the MARKER IS THE FINAL
-- ARCHIVAL SEMANTICS (owner adjudication 2026-08-03): there is no second tier
-- to move rows to and none is planned. A marker keeps the archived object
-- readable and its version chain intact, which the SM neither requires nor
-- forbids.
-- Serving reads never join this table (zero wire drift). Plain marker keyed by
-- vo_id (not a per-version key), so INTENTIONALLY FK-LESS to the
-- composite-keyed vo_version.
CREATE TABLE vo_archive (
    vo_id       uuid NOT NULL,
    archived_at timestamptz NOT NULL DEFAULT now(),
    reason      text,
    CONSTRAINT pk_vo_archive PRIMARY KEY (vo_id)
);

COMMENT ON TABLE vo_archive IS 'Archive markers realizing SM I_ADMIN_ARCHIVE (SM openehr_platform master15-admin_service.adoc); the SM defines no storage form — our own design, and the marker IS the final archival semantics (owner adjudication 2026-08-03), not a step toward a second storage tier. Serving reads never join it (zero wire drift). Intentionally FK-less (vo_id is not a per-version key).';

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

-- ── Grants (no openEHR spec governs DB grants — operational design) ──────────
-- Guarded like the role block: applied when the roles exist (production
-- migrator), skipped with a NOTICE otherwise (dev/compose without CREATEROLE).
DO $$
BEGIN
    -- Lock down the public schema: no PUBLIC CREATE (PostgreSQL 18 docs,
    -- "Privileges" / CREATE SCHEMA notes on the public schema).
    BEGIN
        REVOKE CREATE ON SCHEMA public FROM PUBLIC;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'skipping public-schema lockdown (not schema owner)';
    END;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        -- The runtime writer (DML) and the read-only role. The migrator owns
        -- the objects. No sequences — all generated keys use uuidv7().
        GRANT USAGE ON SCHEMA ehr TO ferroehr_app, ferroehr_reader;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA ehr TO ferroehr_app;
        GRANT SELECT ON ALL TABLES IN SCHEMA ehr TO ferroehr_reader;
        -- Future ehr tables reachable without a manual grant (PostgreSQL 18
        -- docs, ALTER DEFAULT PRIVILEGES) — the deploy-outage classic.
        ALTER DEFAULT PRIVILEGES IN SCHEMA ehr
            GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ferroehr_app;
        ALTER DEFAULT PRIVILEGES IN SCHEMA ehr
            GRANT SELECT ON TABLES TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping ehr grants (roles absent — see the role block NOTICE)';
    END IF;
END $$;
