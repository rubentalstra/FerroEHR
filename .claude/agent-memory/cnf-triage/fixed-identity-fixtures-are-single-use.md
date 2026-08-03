---
name: fixed-identity-fixtures-are-single-use
description: An EHR-Extract fixture with a fixed container uid can serve only ONE case per run; the runner's 409 tolerance hides the collision
metadata:
  type: project
---

A `requires.import` fixture carries a FIXED `X_VERSIONED_*.uid`, and a
versioned-object identity is global in the store — `master06 §Copying`: "If
some version of the item had already been received, this step will have already
occurred, and the requisite `VERSIONED_OBJECT` would already exist", with
version identifiers "globally unique" (`master06` L33). So the SECOND case that
imports the same fixture (into its own freshly minted EHR) gets a spec-correct
409 "versioned object … already exists in another EHR", and its read then 404s
in an EHR that never held the container.

Reproduced 2026-08-03 with `cnf.messaging.ehr_extract.imported_composition`
shared by `get_versioned_composition-imported_version` (ran first, path-sorted
by `artifacts.rs::yaml_files_under` + a stable `sort_by_key` in `run.rs`) and
`…-imported_version_xml_root` (ran second, red).

The masking half: `driver.rs::provisioning_refusal` returns None for **409** —
a tolerance written for template re-upload idempotence ("a re-run row
re-uploads the same deterministic OPT"), where 409 means the ground DOES hold.
For `provision_import` a 409 means the opposite, so an unestablished
precondition is driven anyway and surfaces as a false SUT failure instead of an
inconclusive row.

**How to apply:** every `requires.import` case needs its OWN identity-disjoint
fixture (the corpus MANIFEST provenance strings already use "identity-disjoint"
as the convention). When a `requires.import` case reads `not_found`, check
whether a sibling case imports the same fixture before suspecting addressing or
negotiation.
