-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- The organisation an access happened on behalf of, beside the natural person
-- the `principal` column already names.
--
-- EHDS Annex II 3.2 asks for both halves of the accessing side: (a) "the
-- healthcare provider or other individuals having accessed" and (b) "the
-- specific natural person or persons having accessed"
-- (https://eur-lex.europa.eu/eli/reg/2025/327/oj). The principal answers (b)
-- alone, and without (a) the same person acting for two providers leaves two
-- identical records. No openEHR spec governs the read-side model — our own
-- design/extension.
--
-- This is the ACCESSING organisation, resolved from the caller's identity
-- token, never the reporting node's own site: the DICOM AuditEnterpriseSiteID
-- of PS3.15 §A.5 names the system that emitted the record, which answers a
-- different question
-- (https://dicom.nema.org/medical/dicom/current/output/chtml/part15/sect_A.5.html).
--
-- Append-only needs no new machinery here either: the BEFORE UPDATE trigger
-- compares the whole row as jsonb minus the two delivery stamps, so this
-- column is covered by it the moment it exists.

ALTER TABLE audit_event
    ADD COLUMN organisation text;

-- NULL is a real answer and means "not recorded": a Basic-authenticated
-- caller carries no claims, a deployment may configure no organisation claim,
-- and records written before this migration have none — inventing an
-- organisation for any of them would be a guess about who a caller acted for.
COMMENT ON COLUMN audit_event.organisation IS
    'The organisation the caller acted for, from the configured identity-token claim; NULL when the deployment resolves none and on records written before the column existed.';
