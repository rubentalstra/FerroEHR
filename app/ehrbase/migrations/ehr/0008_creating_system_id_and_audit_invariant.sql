-- Storage-semantics audit wave (RM common master06 change_control).
--
-- M2 — persist the per-version creating_system_id (RM common master06
-- §"Distributed Versioning"): the OBJECT_VERSION_ID is the tuple
-- {object_id, creating_system_id, version_tree_id}, and creating_system_id is
-- the *immutable* identity of the system that created a version. It must be
-- stored per version and reconstructed from storage (never re-derived from the
-- live service config), so a later config change cannot mutate a historical
-- version's uid or invalidate its digital signature. Greenfield: an empty
-- string is the legacy sentinel (no rows predate this column), and the read
-- paths fall back to the service system id only for the empty sentinel.
ALTER TABLE vo_version
    ADD COLUMN creating_system_id text NOT NULL DEFAULT '';

-- m6 — AUDIT_DETAILS.System_id_valid (RM common §"AUDIT_DETAILS"): system_id
-- is mandatory (1..1, non-void). Enforce the non-empty invariant at the
-- database as a domain CHECK; every write already supplies a non-empty system
-- id (service config default or a client-supplied value).
ALTER TABLE audit
    ADD CONSTRAINT audit_system_id_nonempty CHECK (system_id <> '');

-- m7 — indelibility design note (RM common master06 §Overview: a versioned
-- repository "is by definition indelible"; §"Logical Deletion": content is
-- "only ever logically deleted"). The ON DELETE CASCADE edges from `ehr`
-- (vo_version → node/vo_attestation, contribution, item_tag; see
-- 0001_schema.sql) exist SOLELY for the administrative physical-delete / GDPR
-- erasure path (SM I_ADMIN_SERVICE.physical_ehr_delete — see
-- app/ehrbase/src/service/admin.rs). Normal openEHR operation NEVER physically
-- deletes a version: a delete is a new content-less `523|deleted|` version.
-- The indelibility guarantee is therefore enforced at the service layer by
-- design (no service path issues a physical DELETE except admin), not by a
-- database prohibition — the cascade is the deliberate escape hatch for the
-- one operation the spec places outside the change-control model.
