-- Demographic (PARTY) storage on the existing versioned-object machinery.
--
-- Parties (PERSON/ORGANISATION/GROUP/AGENT/ROLE) are versioned objects with NO
-- EHR scope: they live in the same `vo_version` + `node` tables as clinical
-- content, but their `ehr_id` is NULL (the demographics repository is not owned
-- by any EHR). ITS-REST 1.0.3 defines no demographic wire contract (SM is
-- abstract; CNF master10 is all TBD), so this storage is our own design by
-- analogy with the EHR group (ADR-008).
--
-- Constraint names below are the PostgreSQL auto-generated defaults
-- (`<table>_<column>_check`), verified against 0001_schema.sql.

-- 1. `vo_version.kind` may now discriminate a concrete party RM type.
ALTER TABLE vo_version DROP CONSTRAINT vo_version_kind_check;
ALTER TABLE vo_version ADD CONSTRAINT vo_version_kind_check
    CHECK (kind IN (
        'COMPOSITION', 'EHR_STATUS', 'EHR_ACCESS', 'FOLDER',
        'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE'
    ));

-- 2. A party has no owning EHR: `ehr_id` becomes nullable on every table that
--    scopes versioned content by EHR. The foreign keys stay (a NULL passes an
--    FK trivially); NULL = the demographics repository.
ALTER TABLE vo_version   ALTER COLUMN ehr_id DROP NOT NULL;
ALTER TABLE contribution ALTER COLUMN ehr_id DROP NOT NULL;
ALTER TABLE node         ALTER COLUMN ehr_id DROP NOT NULL;
ALTER TABLE item_tag     ALTER COLUMN ehr_id DROP NOT NULL;

-- 3. Item tags may target a party as well as a COMPOSITION / EHR_STATUS.
ALTER TABLE item_tag DROP CONSTRAINT item_tag_target_type_check;
ALTER TABLE item_tag ADD CONSTRAINT item_tag_target_type_check
    CHECK (target_type IN (
        'COMPOSITION', 'EHR_STATUS',
        'AGENT', 'GROUP', 'ORGANISATION', 'PERSON', 'ROLE'
    ));
