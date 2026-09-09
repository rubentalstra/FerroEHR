-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The pseudonymisation cutover: parties leave the clinical schema.
--
-- `0001_baseline` built the demographic domain and deliberately left it empty,
-- so a server could upgrade through it and notice nothing. This migration is
-- the move itself, and it is NOT inert: after it a party is readable only
-- through the demographic schema. It therefore ships in the same release as
-- the code that reads it there. Landing it alone would delete the parties from
-- under a service still looking in `ehr`.
--
-- Runs with search_path = demographic, ext.

-- ── refuse before touching anything ──────────────────────────────────────────
-- An EHR-less row of a kind this migration does not classify stops the upgrade
-- HERE, before a single row moves, naming the kind and the count. Checking
-- afterwards would be too late in two ways: rows would already have moved, and
-- the failure would surface as a foreign-key or check violation on a table the
-- operator did not ask about, which says nothing about the decision they
-- actually have to make.
DO $$
DECLARE
    stray text;
BEGIN
    SELECT string_agg(format('%s (%s rows)', kind, n), ', ' ORDER BY kind)
    INTO stray
    FROM (SELECT kind, count(*) AS n FROM ehr.vo_version
          WHERE ehr_id IS NULL
            AND kind NOT IN ('AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE',
                             'PARTY_RELATIONSHIP')
          GROUP BY kind) s;
    IF stray IS NOT NULL THEN
        RAISE EXCEPTION
            'ehr.vo_version still holds EHR-less rows this migration does not '
            'classify: %. The pseudonymisation boundary moves the demographic '
            'kinds (AGENT, GROUP, ORGANISATION, PERSON, ROLE, '
            'PARTY_RELATIONSHIP) and refuses to guess about anything else. '
            'Decide which domain these belong to before upgrading.', stray;
    END IF;
END $$;

-- ── data move ────────────────────────────────────────────────────────────────
-- Selected by KIND, never by `ehr_id IS NULL`.
--
-- The two are the same set today: the demographic kinds are exactly the
-- EHR-less ones. They are not the same PREDICATE, and the difference is the
-- whole safety of this migration. Moving "everything with a null ehr_id" would
-- relocate any future or unforeseen EHR-less clinical object into the
-- demographic domain silently, which is the precise failure this boundary
-- exists to prevent, committed by the migration that introduces it. Naming the
-- kinds moves what was meant, and the assertion below turns anything else into
-- a refused upgrade with a readable message.
--
-- In foreign-key order: audits, then contributions, then versions, then the
-- rows that hang off a version. An audit moves only when nothing clinical still
-- references it.

CREATE TEMPORARY TABLE moved_version ON COMMIT DROP AS
    SELECT vo_id, sys_version, audit_id, contribution_id
    FROM ehr.vo_version
    WHERE kind IN ('AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE',
                   'PARTY_RELATIONSHIP');

-- A contribution reached by a moved version, plus any EHR-less contribution
-- that carries no version at all (an empty change set still belongs to the
-- domain that made it).
CREATE TEMPORARY TABLE moved_contribution ON COMMIT DROP AS
    SELECT c.id, c.audit_id
    FROM ehr.contribution c
    WHERE c.id IN (SELECT contribution_id FROM moved_version
                   WHERE contribution_id IS NOT NULL)
       OR c.ehr_id IS NULL;

CREATE TEMPORARY TABLE moved_audit ON COMMIT DROP AS
    SELECT a.id
    FROM ehr.audit a
    WHERE (a.id IN (SELECT audit_id FROM moved_contribution WHERE audit_id IS NOT NULL)
           OR a.id IN (SELECT audit_id FROM moved_version WHERE audit_id IS NOT NULL))
      AND NOT EXISTS (
          SELECT 1 FROM ehr.contribution c
          WHERE c.audit_id = a.id
            AND c.id NOT IN (SELECT id FROM moved_contribution))
      AND NOT EXISTS (
          SELECT 1 FROM ehr.vo_version v
          WHERE v.audit_id = a.id
            AND (v.vo_id, v.sys_version) NOT IN
                (SELECT vo_id, sys_version FROM moved_version));

INSERT INTO demographic.audit
    SELECT a.* FROM ehr.audit a JOIN moved_audit m ON m.id = a.id;
INSERT INTO demographic.contribution
    SELECT c.* FROM ehr.contribution c JOIN moved_contribution m ON m.id = c.id;
INSERT INTO demographic.vo_version
    SELECT v.* FROM ehr.vo_version v
    JOIN moved_version m ON m.vo_id = v.vo_id AND m.sys_version = v.sys_version;
INSERT INTO demographic.node
    SELECT n.* FROM ehr.node n
    JOIN moved_version m ON m.vo_id = n.vo_id AND m.sys_version = n.sys_version;
INSERT INTO demographic.vo_attestation
    SELECT t.* FROM ehr.vo_attestation t
    JOIN moved_version m ON m.vo_id = t.vo_id AND m.sys_version = t.sys_version;
INSERT INTO demographic.item_tag
    SELECT g.* FROM ehr.item_tag g
    WHERE g.target_vo_id IN (SELECT vo_id FROM moved_version);
INSERT INTO demographic.vo_archive
    SELECT r.* FROM ehr.vo_archive r
    WHERE r.vo_id IN (SELECT vo_id FROM moved_version);

-- Reverse order. `node` and `vo_attestation` cascade off `vo_version`; the
-- explicit deletes keep the sequence readable rather than relying on that.
DELETE FROM ehr.item_tag WHERE target_vo_id IN (SELECT vo_id FROM moved_version);
DELETE FROM ehr.vo_archive WHERE vo_id IN (SELECT vo_id FROM moved_version);
DELETE FROM ehr.vo_attestation t
    WHERE EXISTS (SELECT 1 FROM moved_version m
                  WHERE m.vo_id = t.vo_id AND m.sys_version = t.sys_version);
DELETE FROM ehr.node n
    WHERE EXISTS (SELECT 1 FROM moved_version m
                  WHERE m.vo_id = n.vo_id AND m.sys_version = n.sys_version);
DELETE FROM ehr.vo_version v
    WHERE EXISTS (SELECT 1 FROM moved_version m
                  WHERE m.vo_id = v.vo_id AND m.sys_version = v.sys_version);
DELETE FROM ehr.contribution c WHERE c.id IN (SELECT id FROM moved_contribution);
DELETE FROM ehr.audit a WHERE a.id IN (SELECT id FROM moved_audit);

-- ── the structural boundary ──────────────────────────────────────────────────
-- Declared only once the domains hold only their own kind, and only once that
-- has been PROVEN rather than assumed. An EHR-less row of a kind this migration
-- does not know about refuses the upgrade here, naming the kind and the count,
-- instead of surfacing three statements later as an opaque check violation on a
-- table the operator did not ask about.

-- Added AFTER the mirror relations exist, so `LIKE ... INCLUDING CONSTRAINTS`
-- cannot copy one side's rule onto the other, and after the move, because the
-- clinical rows violate the clinical rule until they have left. Together they
-- make the split a property the database enforces in both directions.
ALTER TABLE ehr.vo_version
    ADD CONSTRAINT ck_vo_version_ehr_scoped CHECK (ehr_id IS NOT NULL);
ALTER TABLE ehr.contribution
    ADD CONSTRAINT ck_contribution_ehr_scoped CHECK (ehr_id IS NOT NULL);
ALTER TABLE demographic.vo_version
    ADD CONSTRAINT ck_dem_vo_version_unscoped CHECK (ehr_id IS NULL);
ALTER TABLE demographic.contribution
    ADD CONSTRAINT ck_dem_contribution_unscoped CHECK (ehr_id IS NULL);

COMMENT ON CONSTRAINT ck_vo_version_ehr_scoped ON ehr.vo_version IS 'The clinical half of the pseudonymisation boundary: every versioned object in this schema belongs to an EHR. A party reaching this table is a code path that missed the split, and this makes it a loud write failure rather than a silent leak.';
COMMENT ON CONSTRAINT ck_dem_vo_version_unscoped ON demographic.vo_version IS 'The demographic half of the pseudonymisation boundary: nothing in this schema is EHR-scoped, so a clinical object cannot be written here.';

