---
name: rm-common-master05-directory-overview
description: Verified findings for RM common master05 §5.1 (directory Overview + Paths) — the FOLDER.details slot is unchecked, FOLDER.items has zero CNF coverage, and the RM directory path grammar is unregistered (AMB-81 covers only the REST param)
metadata:
  type: feedback
---

Verified 2026-08-02 against RM 1.2.0
`docs/specs/openehr/RM/docs/common/master05-directory_package.adoc`.
Section boundary: `== Overview` = lines 3–28 INCLUDING its `=== Paths`
subsection (§5.1.1); `== Class Descriptions` (line 30) is #989. Unlike the
master04 generic chapter, the directory diagram IS vendored
(`RM/docs/UML/diagrams/RM-common.directory.svg`, 148 KB) — but its text is in
`<g>`-nested glyphs, so a naive `>text<` regex extracts nothing.

**SUPERSEDED 2026-08-19 (see [[rm-common-ch567-fix-verification]]): the
`FOLDER.details` and `FOLDER.items` defects below are FIXED — the JSON write path
is typed (`negotiate.rs:594 rm_value::<Folder>`), the model-driven
`check_declared_slot_type` covers every slot, and the items battery
(with_items / items_by_value / multiply_classified_items / versioned_id_items /
wide_id_items) exists. The archie citation is purged.**

**VERIFIED DEFECTS (as of 2026-08-02, most now closed):**
- `FOLDER.details` has NO slot-type check anywhere: `validate_folder`
  (`app/ferroehr/src/service/ehr/validation.rs:684`) walks `_type`/`name`/
  `archetype_node_id`/`links`/`items`/`folders` and never inspects `details`,
  while the sibling `EHR_STATUS.other_details` IS checked (`:540`). The
  whole-instance pass cannot cover it: `rm_invariant_pass`
  (`crates/openehr-its/src/flat/validation/mod.rs:686`) trusts the node's own
  `_type` tag (`declared_concrete_type` is only a fallback for UNTAGGED nodes)
  and there is no slot conformance outside the WebTemplate pass. JSON write
  bodies are NOT typed-decoded (`negotiate.rs:505` → `parse_json`, untyped),
  so a JSON `details: {"_type":"DV_TEXT"}` commits 201 — then an XML read
  re-types through `Folder` and 500s (`negotiate.rs:679`). XML writes reject
  it (typed `from_canonical_xml`) → JSON/XML asymmetry.
- `FOLDER.items` has ZERO CNF coverage: no `corpus/fixtures/directory/*`
  fixture carries `items` (v2.json's "items" is the ITEM_TREE's), so §5.1's
  defining semantic (references, and MULTIPLE references to the same object
  for multiple classification) is never exercised by the acceptance
  instrument; the by-value refusal (`validation.rs:739`) has no invalid-twin
  case. `app/ferroehr-rest/tests/it/directory_http.rs:139 folder_with_items`
  is the only items coverage in the repo, used once with a single ref —
  nothing anywhere asserts the same OBJECT_REF twice.
- `crates/openehr-rm/src/common/directory/folder_impl.rs:1-4` cites **archie**
  as authority ("archie's own `Folder.Folders_valid` is `ignored`") and names
  an invariant RM 1.2.0 does not have (BMM `FOLDER` = ancestors [LOCATABLE],
  properties [items, folders, details], zero invariants; the class table
  agrees). No RM citation at all in that file.
- ~~`VERSIONED_FOLDER` container bodies are hand-built `serde_json::json!`~~
  **FIXED in #1658** — `versioning/wire.rs:217` now builds the generated
  `VersionedObject::VersionedFolder` and serializes through the codec.

**Register status (do NOT re-report as new):** AMB-81 already fixes the
handling of the ITS-REST `path` query param (root-implicit, folders-only,
first-sibling-wins, leading slash tolerated) AND explicitly notes the RM
bracket uniqueness-modifier convention is never adopted by the param. What is
NOT registered: the RM path layer itself — §5.1.1's `/folders[hospital
episodes]` / `[hospital episodes(car accident Aug 1998)]` is unexpressible in
BASE `master11-paths` (a bare bracket token is the archetype_node_id
shortcut), and `crates/openehr-rm/src/paths.rs:612-618` silently binds any
such token, parentheses and all, to `archetype_node_id` instead of failing
loud. Latent only: the `ehr:` URI resolver (`service/ehr/uri.rs:106`) is the
sole consumer and has NO REST binding (tests only).

**Conformant, verified (don't re-check):** FOLDER is `is_structure_root: true`
(`crates/openehr-rm/src/model/data.rs:5303`) so a whole tree decomposes to one
node row per FOLDER and each version is a full Folder structure; the RM model's
FOLDER attribute types are exact; no duplicate-item rejection exists anywhere
(multiple refs allowed); duplicate sibling folder NAMES accepted
(`directory_http.rs:1096`); `select_subfolder` (`service/ehr/directory.rs:432`)
matches the ITS-REST param definition verbatim
(`ITS-REST specifications/parameters/query/path.yaml`: slash-separated
FOLDER name values); AQL treats FOLDER as a VO root
(`app/ferroehr/src/aql/sql/from.rs:888`); `get_versioned_directory` has no
ITS-REST endpoint and is honestly registered as AMB-24 unrealized.

**Cross-refs for #989 (class tables):** the FOLDER class NOTE is a THIRD
instance of the AMB-65 self-contradiction (says copy `object_id()`, then the
worked example copies the full 3-part `…::uk.nhs.ehr1::2`); AMB-65's `source`
cites only master03 + EHR_STATUS, not FOLDER.
