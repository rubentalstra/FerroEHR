-- SM-4 (I_ADMIN_ARCHIVE): versioned-object archive markers.
--
-- archive = a MARKER this phase. The SM (`i_admin_archive.adoc`) says
-- "Move selected EHRs/Parties to archival storage" but defines no storage
-- form. Actual movement to a storage tier is P20 optimization; here
-- archive_ehrs/archive_parties simply record which versioned objects are
-- archived. SERVING READS ARE UNCHANGED — nothing on the read path joins
-- vo_archive — so archival introduces ZERO wire drift (design fixed-decision
-- SM-4).
--
-- vo_id is the versioned-object id (not a per-version key), so this is a plain
-- marker table with no FK to the composite-keyed vo_version. physical_ehr_delete
-- / physical_party_delete remove any markers for the VOs they physically delete.
CREATE TABLE vo_archive (
    vo_id       uuid PRIMARY KEY,
    archived_at timestamptz NOT NULL DEFAULT now(),
    reason      text
);
