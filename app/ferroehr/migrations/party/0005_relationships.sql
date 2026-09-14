-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- party: the reverse index of PARTY_RELATIONSHIP.
--
-- RM demographic master02-demographic_package.adoc gives PARTY a
-- `reverse_relationships` attribute whose computation the model expresses as a
-- repository-wide lookup ("repository(\"demographics\").all_party_relationships"):
-- every relationship whose TARGET is this party. Walking the versioned bodies
-- to answer it would read every relationship in the domain, so the target side
-- is indexed here.
--
-- No openEHR spec governs the index relation itself — our own storage design;
-- the attribute it serves is the RM's.
--
-- Runs with search_path = party, ext, public.
CREATE TABLE party_relationship_target (
    -- The party the relationship points AT (PARTY_RELATIONSHIP.target).
    target_party_id uuid NOT NULL,
    -- The PARTY_RELATIONSHIP versioned object that points at it. FK-less for
    -- the reason the change-control relations are keyed per version: this row
    -- names the OBJECT, which outlives any one of its versions.
    source_vo_id    uuid NOT NULL,
    CONSTRAINT pk_party_relationship_target PRIMARY KEY (target_party_id, source_vo_id)
);

-- Maintenance from the source side: a relationship's target changes with a new
-- version, and the old row is removed by source.
CREATE INDEX idx_party_relationship_target_source
    ON party_relationship_target (source_vo_id);

COMMENT ON TABLE party_relationship_target IS 'The target side of PARTY_RELATIONSHIP, indexed so PARTY.reverse_relationships (RM demographic master02-demographic_package.adoc) is a lookup rather than a scan of every relationship in the domain. Our own storage design.';
COMMENT ON COLUMN party_relationship_target.source_vo_id IS 'The PARTY_RELATIONSHIP versioned object pointing at the target; FK-less because it names the object, not one of its versions.';
