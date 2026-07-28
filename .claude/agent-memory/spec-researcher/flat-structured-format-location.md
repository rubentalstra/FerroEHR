---
name: flat-structured-format-location
description: Where the FLAT / STRUCTURED (Simplified Formats / SDT) spec lives, the carrying-operation matrix, the released-text defect list, and the implementation/test map
metadata:
  type: reference
---

# FLAT / STRUCTURED (Simplified Formats) — location map

**Authoritative wire format = ITS-REST `docs/simplified_formats/` (`spec_status: STABLE`
in `manifest_vars.adoc`).** 6 chapters, master.adoc includes 01–06:
- `master01-preface.adoc` — §Conformance ("same calls as other serialisation formats,
  different `Content-Type`") · `master02-overview.adoc` — §MIME Types, §Scope,
  §Relationship to Other Specifications (template-derived ids)
- `master03-design_rationale.adoc` — NARRATIVE (history TDS/ECISFLAT/WT) + §Requirements
  (5 numbered) + full FLAT/STRUCTURED worked examples
- `master04-basic_concepts.adoc` — **THE normative syntax chapter**: §Web Template
  Metadata (WT example), §Field Identifiers (6 parts), §Node ID Generation Rules
  (7 steps + examples table), §Path Construction, §Instance Indexing, §Attribute
  Suffixes, §RM Attributes prefix (`_`), §Raw canonical JSON (`|raw`), §Context,
  §Format variants (Flat 6 MUST rules / Structured 6 MUST+SHOULD rules),
  §Conversion Between Formats (2 algorithms), §Level Removal (3 sub-§: container
  attribute elision list, always-collapsed wrappers, conditionally-collapsed EVENT),
  §Open Value-Sets and the `|other` Suffix, §Validation (8 SHOULDs)
- `master05-rm_mapping.adoc` (3215 L) — **43 `[cols=5*]` tables** (Flat Path / Flat type /
  RM Path / Required / Note), 45 sections; PARTY_PROXY + "When a DV_CODED_TEXT becomes a
  DV_TEXT" carry prose only. NO COMPOSITION-external root: no EHR_STATUS, no FOLDER, no
  demographic PARTY root table.
- `master06-context_information.adoc` — 17 `ctx/` sub-sections + defaults

**SDT** (`ITS-REST/docs/simplified_data_template/`) = `spec_status: RETIRED`, 4 files,
ZERO normative content (master.adoc includes only preface+amendment). SPECITS-61.

**Amendment 1.1.1 (SPECITS-94, 28 Apr 2026)** is the current master05/04 delta:
Level Removal split, `|other`, `|preferred_term`, LINK `|target`, DV_MULTIMEDIA
`|mediatype`, PARTICIPATION §, `|accuracy`/`|accuracy_is_percent`/`|normal_status` sweep.

## Carrying-operation matrix (grep `Accept_LOCATABLE` / `ContentType_LOCATABLE`)

Media-type enums live ONLY in `specifications/parameters/header/{Accept,ContentType}_LOCATABLE.yaml`
(json|xml|wt.flat+json|wt.structured+json) + `specifications/headers/ContentType_LOCATABLE.yaml`
(response). `Accept_Template.yaml` = json|xml|**wt+json** (template resource only, NOT
flat/structured). `Accept_canonical` = json|xml.

Accept_LOCATABLE (28 ops): composition_{create,get,update}; contribution_{create,get};
definition_template_{adl1.4,adl2}_example_get; directory_{create,update,get_at_time,get_by_version_id};
ehr_status_{update,get_at_time,get_by_version_id}; {agent,group,organisation,person,role}_{create,get,update}.
ContentType_LOCATABLE adds **ehr_create / ehr_create_with_id** (whose Accept is `Accept_canonical` —
an asymmetry). Do NOT admit: all tag ops, all versioned_* ops, ehr_get_by_*, query ops,
demographic_contribution_*, ADL2 template ops, stored-query ops, definition_template_adl1.4_get
(that one uses `Accept_Template` → `wt+json`).

Prose rules: `overview/Resources.md` §Simplified Formats (MIME list incl. wt+json + the
deprecated `.schema+json` NOTE + the 415/406 MUSTs) and §Data representation
("canonical MUST, Simplified SHOULD, other MAY") and §Alternative data formats
(legacy `nc.flat+json`, `tds2+xml`). `overview/Requests_and_responses.md`
§openehr-template-id = the ONE uppercase MUST, scoped to "committing COMPOSITION".
`operations/contribution_create.yaml` §Simplified Formats = envelope-canonical /
`versions[i].data`-simplified rule (SPECITS-84).
**There is NO `responses/415*.yaml` at all** (415 is prose-only); `406.yaml` is
referenced by only 3 ops (the two example_gets + adl1.4 template get).

## Confirmed released-text defects in master05 (verified first-hand)

- `/territory` row on ADMIN_ENTRY/INSTRUCTION/ACTION/EVALUATION/OBSERVATION ×5 —
  RM ENTRY has NO territory (`RM/docs/UML/classes/org.openehr.rm.composition.entry.adoc`)
- `/encoding` (RM 1..1 mandatory) MISSING from all 5 ENTRY tables though the chapter's
  own COMPOSITION example emits `.../encoding|code`
- DV_QUANTITY: Flat-type column SWAPPED (`|magnitude`=String, `|unit`=Real)
- `|id_scheme` typed `Integer` in PARTY_SELF/PARTY_IDENTIFIED/PARTY_RELATED (×3)
- DV_MULTIMEDIA `|data` → RM Path typo `dta`
- REFERENCE_RANGE meaning row spelled `\meaning` (malformed cell)
- `other_reference_ranges` RM-Path column written `` `_other_reference_ranges` `` ×8
- FEEDER_AUDIT "one one of …" typo ×2
- INSTRUCTION `/_expiry_time` Required=Yes on an `_`-prefixed (optional) attribute
- PARTY_RELATED `/_relationship` vs PARTICIPATION-performer `/relationship` — two
  spellings for one attribute in one chapter
- master06 §Workflow ID says "if `ctx/namespace` is set"; the vocabulary defines
  `ctx/id_namespace`
- master04 §Structured rule 2 + §Structured-to-Flat step 6 ("indices remain in property
  names") CONTRADICT rule 5 + every worked example (index = array position)

## Implementation + tests

`crates/openehr-its/src/flat/` (hand-written, 23.8k L): `path.rs` FlatKey parser ·
`sim/{flat,structured}.rs` wire codecs over `sim::SimNode` · `flatten.rs` RM→sim ·
`build.rs` sim→RM · `map/{data_values,parties,structures}.rs` = the master05 tables ·
`ctx.rs` = master06 · `webtemplate/id.rs` = the 7-step node-id algorithm ·
`validation/` · `example.rs` · `tdd.rs`. Crate CLAUDE.md §"The `flat` module" is accurate.
**There is NO Better-quirks feature flag** — `docs/architecture.md:131` claims one;
`.claude/rules/serialization.md` correctly says "there is no quirks feature flag".

Wire seam: `app/ehrbase-rest/src/formats/dispatch.rs` (+ `crate::overview::negotiate`);
`guard_non_templated()` rejects simplified on EHR/EHR_STATUS/FOLDER/parties (415 in / 406 out),
called from `api/{ehr/ehr_resource,ehr/ehr_status,ehr/directory,demographic/party,demographic/relationship}.rs`.

Tests: `crates/openehr-its/tests/spec_vectors.rs` (1971 L, ~64 tests, one per chapter
section incl. all 43 master05 tables) — but `assert_flat_vector` only checks FlatKey
round-trip + FLAT⇄STRUCTURED fixed point, **NOT** the table's RM-Path/type/required
semantics, so mapping bugs pass. Also `flat.rs`/`structured.rs` (corpus round-trips),
`webtemplate.rs`, `tdd.rs`. Corpora: `tests/fixtures/better` (64 Better web-template-tests
OPTs), `tests/fixtures/sdk` (21), `tests/vendor/openehr_sdk/composition/canonical_json`.

CNF: 58 cases in `tools/cnf-runner/artifacts/schedule/simplified_formats/` (SF-*).
Register entries touching Simplified: AMB-39 (deprecated `.schema+json`, option_select),
AMB-57 (contribution READ side), AMB-58 (our wt+json extension), AMB-61 (legacy
nc.flat/tds2, option_select), AMB-128 (party canonical-only), AMB-134 (contribution
envelope), AMB-109 (WebTemplate `semVer` not in schema). **No register entry exists for
the directory / EHR_STATUS simplified refusal** although SF-SCOPE-{directory,ehr_status}_no_simplified
assert it.
