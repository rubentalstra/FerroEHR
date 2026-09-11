-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The roles the accessing person held when the access happened, beside the
-- person (`principal`) and the organisation they acted for (`organisation`).
--
-- NEN 7513 lists, among what a logged event has to contain, the role or
-- authority under which the person accessed the record; the RBAC layer holds
-- the caller's roles at emission time (the RFC 9068 §2.2.3.1 claim carriers
-- for a bearer, the user definition for Basic) and until now the record
-- dropped them (#3239). No openEHR spec governs the read-side model — our own
-- design/extension.
--
-- A JSON array of role names rather than text[]: the batched write path binds
-- one array per column and unnests it, which a two-dimensional text array
-- cannot express row by row, while an array of jsonb scalars can.
--
-- Append-only needs no new machinery: the BEFORE UPDATE trigger compares the
-- whole row as jsonb minus the two delivery stamps, so this column is covered
-- by it the moment it exists.

ALTER TABLE audit_event
    ADD COLUMN roles jsonb;

-- NULL means "not recorded": an unauthenticated refusal has no principal and
-- no roles, and records written before this migration carry none. An
-- authenticated caller with no roles is recorded as the empty array, which is
-- a different fact from an unknown one.
COMMENT ON COLUMN audit_event.roles IS
    'The roles the caller held at access time, as a JSON array of role names; NULL when no principal was authenticated and on records written before the column existed.';
