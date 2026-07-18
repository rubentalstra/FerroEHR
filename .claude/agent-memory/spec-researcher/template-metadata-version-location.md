---
name: template-metadata-version-location
description: Where TemplateMetadata.version (ITS-REST definition/template list) is defined and where an OPT 1.4 version comes from
metadata:
  type: reference
---

# TemplateMetadata.version (ADL1.4 template list) — spec locations

**ITS-REST TemplateMetadata schema:**
- Spec doc form: `docs/specs/openehr/ITS-REST/specifications/schemas/definition/TemplateMetadata.yaml` — properties {template_id, version(deprecated:true), concept, archetype_id, created_timestamp}; `required` = template_id/concept/archetype_id/created_timestamp (version NOT required).
- Vendored OAS (codegen input): `crates/openehr-its/vendor/rest-oas/definition-codegen.openapi.yaml` §`components.schemas.TemplateMetadata` (~L503) — identical; `version: {type: string, deprecated: true}`.
- The `filter_version` query param (OAS ~L3271) is the load-bearing meaning: "Filter by version (e.g. `1.2.*` ...), **taken from `template_id`**; if missing, then only the latest version will be returned."
- No prose Resources.md/Description.md elaborates the field (Description.md is boilerplate only).

**Where an OPT 1.4 version comes from (definitive):** CNF schedule `docs/specs/openehr/CNF/docs/platform_test_schedule/master04-func_tc_definition_adl.adoc` L161-163: "there is no formal versioning mechanism for templates 1.4 ... An alternative solution for the version parameter is to add the version number to the `other_details` of the OPT, or directly into the `template_id`." So version source is server-choice: template_id suffix OR OPT other_details. NOT a spec-mandated OPT field.

**ADL2 vs ADL1.4 versioning:** the `.vN`/`.vMAJOR.MINOR.PATCH` suffix rule is an ADL2/AOM2 archetype-HRID mechanism (`docs/specs/openehr/AM/docs/AOM2/master10-templates.adoc` §Template Identifiers L69+ — templates use "normal ADL multi-axial identifiers"; examples carry `.v1.0.0`). ADL1.4 OPT template_ids are typically a plain concept name with NO version suffix (OAS example L162: `<template_id><value>Vital Signs</value>` — no `.vN`).

**CNF coverage:** get_opts list robot test asserts required=[concept, template_id, archetype_id, created_timestamp] ONLY — version is NOT asserted. `CNF/tests/platform/robot/I_DEFINITION_ADL14/get_opts/I_DEFINITION_ADL14.get_opts-retrieve_all.robot` L107-123; L38 NOTE "versioning is not applicable for ADL 1.4". No CNF case covers the `version` field content.
