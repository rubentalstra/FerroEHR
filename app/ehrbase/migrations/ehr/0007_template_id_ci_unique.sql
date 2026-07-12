-- Case-insensitive uniqueness of TEMPLATE_ID.
--
-- BASE base_types master05 §Composite Identifiers and Case: identifier equality
-- (and thus uniqueness) is case-insensitive. A TEMPLATE_ID that differs only in
-- case from a stored one is the SAME template id, so the upload endpoint must
-- reject a case variant as a duplicate (ITS-REST 409_template_already_exists).
-- The exact `uq_template_store_template_id` UNIQUE (kept — it backs the
-- vo_version.template_id foreign key) cannot enforce this; this functional
-- unique index over lower(template_id) is the race-free guard for the
-- case-insensitive rule.
CREATE UNIQUE INDEX ux_template_store_template_id_ci
    ON template_store (lower(template_id));
