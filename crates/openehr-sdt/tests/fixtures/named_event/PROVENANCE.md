# `named_event.en.v1` — a repo-authored regression fixture

Derived from the openEHR CNF Robot fixture
`docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/valid_templates/minimal_persistent/persistent_minimal.opt`
(specifications-CNF, commit `33251d2abe5a75c042e11c9385d2e9a79aa15904`,
CC-BY-SA 3.0 Unported), and licensed under the same terms as a derivative
work. `LICENSE-CC-BY-SA-3.0` at the repository root is the licence text; the
declaration is in `REUSE.toml`.

## What was changed and why

The carrier issue is #3142: an operational template may fix `LOCATABLE.name`
on a node the Simplified Formats collapse away, and no template in any
vendored corpus does that for a COLLAPSED event. Three edits to the skeleton
produce one:

- the event is narrowed from the abstract `EVENT` to `POINT_EVENT` and its
  occurrences from `0..1` to `1..1`, so `master04 §Conditionally Collapsed
  Wrapper Types` collapses it;
- a `name` attribute is constrained to the `C_STRING` list `Point in time`,
  so the collapsed node's name survives only inside the leaf's `aqlPath`;
- the identifiers, term definitions and the composition category (`433`,
  event) are renamed to describe the fixture rather than the skeleton.

Everything else, including the template envelope and the `content` /
`ITEM_TREE` / `ELEMENT` chain, is the upstream skeleton unchanged.
