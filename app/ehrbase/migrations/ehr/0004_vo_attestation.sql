-- ATTESTATION storage (RM common master06 §Change Control: "a new ATTESTATION
-- is added to the attestations list of an existing ORIGINAL_VERSION" — no new
-- version). One row per attestation, canonical RM ATTESTATION verbatim in
-- data (ADR-008: no synthetic fields); contribution_id links the attestation
-- to the CONTRIBUTION that committed it so CONTRIBUTION.versions can list the
-- affected version.
CREATE TABLE vo_attestation (
    id              uuid PRIMARY KEY DEFAULT uuidv7(),
    vo_id           uuid NOT NULL,
    sys_version     integer NOT NULL,
    contribution_id uuid NOT NULL REFERENCES contribution(id),
    time_committed  timestamptz NOT NULL DEFAULT now(),
    data            jsonb NOT NULL,
    FOREIGN KEY (vo_id, sys_version) REFERENCES vo_version (vo_id, sys_version) ON DELETE CASCADE
);
CREATE INDEX vo_attestation_version_idx ON vo_attestation (vo_id, sys_version);
CREATE INDEX vo_attestation_contribution_idx ON vo_attestation (contribution_id);
