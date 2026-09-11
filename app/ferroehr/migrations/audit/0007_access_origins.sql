-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The origins of the data an access served, beside the person, the
-- organisation and the roles (#3212).
--
-- EHDS Annex II 3.2(e): "the origin or origins of data" (Regulation (EU)
-- 2025/327, https://eur-lex.europa.eu/eli/reg/2025/327/oj). The origin of the
-- DATA, not of the request: the record already knows the client address it
-- was asked from; this is where the content it served came from, read from
-- the FEEDER_AUDIT provenance openEHR stamps on content (RM common
-- FEEDER_AUDIT_DETAILS.system_id) as the commit path derived it onto
-- vo_version.origins. No openEHR spec governs the read-side model — our own
-- design/extension.
--
-- Two columns, because a set may be capped: `origins` holds up to the cap
-- the server records, `origin_count` the true number of distinct origins the
-- access served, so a truncated record says it is truncated instead of
-- claiming to be the whole answer. A JSON array rather than text[]: the
-- batched write path binds one array per column and unnests it, which an
-- array of jsonb scalars can express row by row and a text[][] cannot.
--
-- Append-only is covered by the BEFORE UPDATE trigger, which compares the
-- whole row minus the delivery stamps, the moment these columns exist.

ALTER TABLE audit_event
    ADD COLUMN origins      jsonb,
    ADD COLUMN origin_count bigint;

COMMENT ON COLUMN audit_event.origins IS
    'The distinct originating systems of the data this access served, as a JSON array of system ids, capped; NULL when the operation served no version body or the record predates the column.';
COMMENT ON COLUMN audit_event.origin_count IS
    'The number of distinct origins the access served; greater than the array length when the array was capped.';
