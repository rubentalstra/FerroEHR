-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- linkage: which party is the subject of which EHR, temporally.
--
-- Identifiers only — no name, no address, no plaintext identifier of any kind
-- — because a row here is already the additional information that re-joins a
-- record to a person (GDPR Art. 4(5),
-- https://eur-lex.europa.eu/eli/reg/2016/679/oj), and adding an attribute
-- would make it identifying on its own.
--
-- No openEHR spec governs this relation: our own design/extension. The service
-- it belongs to is the SM's EHR Index (SM openehr_platform master03,
-- I_EHR_INDEX), which names itself the EHR id / demographic subject
-- cross-reference.
-- TODO(#3345): absorb the clinical ehr_index and the subject-proxy subject
-- columns into a subject_ehr relation here, and add the definer erase
-- function, once the per-domain pools land.
--
-- Runs with search_path = linkage, ext, public.
CREATE TABLE party_ehr (
    -- The party's versioned-object id in the party domain.
    --
    -- No foreign key, for the reason the whole schema exists: a reference into
    -- the party schema would couple this table to a schema whose role cannot
    -- read it and whose role cannot read this one, and PostgreSQL enforces a
    -- foreign key by reading the referenced row.
    party_id   uuid      NOT NULL,
    -- The EHR the party is the subject of. No foreign key into the clinical
    -- schema either, and for the stronger form of the same reason: that one
    -- would cross the clinical boundary.
    ehr_id     uuid      NOT NULL,
    -- Validity interval of the mapping, half-open [opened, closed). An open
    -- upper bound is the mapping in force now. Committal time is the only
    -- server-managed temporal axis openEHR speaks about (RM common
    -- master06-change_control_package.adoc §Committal and Audits); the
    -- interval itself is our own storage design.
    sys_period tstzrange NOT NULL DEFAULT tstzrange(now(), NULL, '[)'),
    -- The temporal primary key: one party has at most ONE mapping in force at
    -- any instant, enforced by the database rather than by whichever code path
    -- happens to write. PostgreSQL 18 builds a GiST index for a key carrying
    -- WITHOUT OVERLAPS, and the equality key parts need the btree_gist
    -- operator classes (PostgreSQL 18, CREATE TABLE, "PRIMARY KEY",
    -- https://www.postgresql.org/docs/18/sql-createtable.html); the extension
    -- is installed in ext by the bootstrap.
    --
    -- The change-control relations pay a plain btree primary key instead,
    -- because they are the hot write path. This table is written once per EHR
    -- and again only on a merge or a split, so the constraint costs nothing
    -- measurable here and the invariant is worth more.
    CONSTRAINT pk_party_ehr PRIMARY KEY (party_id, sys_period WITHOUT OVERLAPS)
);

CREATE INDEX idx_party_ehr_by_ehr ON party_ehr (ehr_id);

COMMENT ON TABLE party_ehr IS 'Which party is the subject of which EHR, temporally: a merge or split closes a row and opens another rather than deleting. Holds identifiers only, because a row here is already the additional information that re-joins a record to a person (GDPR Art. 4(5)).';
COMMENT ON COLUMN party_ehr.sys_period IS 'Validity interval of the mapping, half-open [opened, closed); an open upper bound is the mapping in force now. Part of the temporal primary key, so one party cannot hold two mappings at one instant.';
