# Catalogue audit — DEFINITION_ADL2 chapter

Issue #231 · audited 2026-07-24 · 24 cases · verdicts: 24 ok / 0 defects / 0 ambiguities

Chapter context: `master04-func_tc_definition_adl.adoc` defines NO official
ADL2 test cases (verified: every Test Case heading in the chapter is
I_DEFINITION_ADL14.*), so this chapter is a full catalogue authoring over the
SM interface — all 14 `I_DEFINITION_ADL2` operations verified present in
`docs/specs/openehr/SM/docs/UML/classes/i_definition_adl2.adoc`
(has/valid/upload/get/list_artefacts/list_archetypes/list_templates/list_opts/
list_matching_artefacts/delete_artefact + the four counts), each covered by at
least one case, nothing silent. AMB-37 (read in full) is the chapter's
realization law: ITS-REST 1.1.0 surfaces ONLY the OPT wire under
`/definition/template/adl2*` — no archetype provisioning, no generic listing,
no DELETE, no counts, no validation-only endpoint, no regex matching — and
adds an example-generation surface the SM does not define. The 15 committed
bindings (one per operation + the example/version variants) declare each
unwired operation unrealized.

| case family | verdict | evidence | resolution |
|---|---|---|---|
| upload_artefact (valid / invalid_artefacts / duplicate_conflict) | ok | SM upload_artefact; ADL2 syntax ground for the invalid rows; per-case fresh logical artefacts (the minimal_b..f HRID-renamed fixture family) prevent shared-SUT collisions; conflict on re-upload | none |
| valid_artefact (valid / invalid) | ok | SM valid_artefact; no validation endpoint → realized via upload (AMB-37), mirroring the AMB-16/AMB-18 pattern | none |
| get_artefact (retrieve / unknown / version_get / example / example_unknown) | ok | OPT retrieval + the deprecated versioned GET + the ITS-added example surface, each with its own binding; unknown → not_found | none |
| has_artefact (existing / non_existing) | ok | Boolean realized via the list-filter (an empty match set IS the false return — binding NOTE); consistent with the AMB-19/48 boolean-by-outcome family | none |
| list_opts (empty / non_empty), list_templates-non_empty, list_matching_artefacts-filter | ok | The realized listing surface; wildcard-glob filter per AMB-37 (no regex, no invalid-pattern error path) | none |
| list_artefacts / list_archetypes / the four counts / delete_artefact (2) | ok | Unrealized per AMB-37, N/A-with-citation, SM-derived draft flows | none |

Checks common to the chapter:
- **Ground (dim 1):** no official rows exist; every case carries the authoring posture and (where divergent/unrealized) the AMB-37 tag.
- **Expectations (dim 2):** recomputed against the SM signatures and the vendored adl2 OAS surface; the outcome kinds map consistently.
- **Fixtures (dim 4):** the `cnf.adl2.opt.minimal[_b..f]` family verified in `corpus/MANIFEST.yaml`; the HRID-renaming provenance gives each upload-flow case its own logical artefact.
- **Captures (dim 5):** the duplicate-conflict capture chain (template_id) consistent.
- **Ambiguity tags (dim 6):** AMB-37 on every case whose ground the ITS gap or the ITS-added surface touches.

Sequencing observation (no action now): `delete_artefact-existing` targets the
shared `openEHR-EHR-COMPOSITION.cnf_minimal.v1.0.0` artefact that the read
cases also provision. Today the case is never driven (AMB-37); when a future
ITS realizes DELETE, the 409-tolerant re-provisioning of later cases makes the
ordering safe, but giving the delete case its own HRID-renamed fixture (the
minimal_b..f pattern) would remove the coupling outright.
