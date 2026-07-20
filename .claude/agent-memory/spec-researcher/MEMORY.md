# Memory index

- [Unconstrained / open attribute validation semantics](unconstrained-attribute-validation.md) — where the "no constraint = any RM-valid value allowed" rule lives (AOM1.4/ADL1.4/CNF)
- [RM class definitions location](rm-class-defs-location.md) — RM class attribute tables (existence) live in docs/UML/classes, included by master chapters
- [Official spec only](feedback-official-spec-only.md) — answer only from docs/specs/openehr; never treat ADRs / docs/design as spec authority (owner, emphatic)
- [VERSION.signature location](version-signature-location.md) — signature 0..1/optional, canonical_form, server-vs-client signing; RM common master06 + UML version.adoc; SM/ITS-REST/CNF silent
- [Persistent COMPOSITION uniqueness](persistent-composition-uniqueness.md) — one-persistent-per-template is NOT spec-mandated (SILENT/under-debate); RM ehr master04 + CNF master07 same_opt_twice (tagged future)
- [FLAT/STRUCTURED format location](flat-structured-format-location.md) — ITS-REST simplified_formats (STABLE) = authoritative wire; SM SIM-B/SDF (DEVELOPMENT) = abstract model+rules; SDT retired; CNF only legacy Robot suite
- [DIRECTORY API location](directory-api-location.md) — EHR FOLDER/directory REST (OAS-split YAML) + FOLDER RM + CNF master09 + the status-code spec gaps (409/404-no-directory undefined; no versioned_directory route)
- [TemplateMetadata.version location](template-metadata-version-location.md) — ITS-REST definition/template list version field (deprecated, not required); source = template_id suffix OR OPT other_details (CNF master04 L161); ADL1.4 has no formal versioning
- [FOLDER / directory model location](folder-directory-model-location.md) — RM FOLDER/VERSIONED_FOLDER/EHR.directory-vs-folders model, invariants, deletion, + generated Rust bindings
- [AOM2 validation catalogue location](aom2-validation-catalogue-location.md) — where every V-code full text lives (master03/04.5/07/06), master08=phase orchestration only, the 14 spec-silent codes (external adl_syntax_errors.txt), am24 gen tree map
- [ADL2 specialisation/flattening/templates/OPT2 location](adl2-specialisation-flattening-opt2-location.md) — where ADL2 09.x specialisation, master10 templates, OPT2 raw/profiled, AOM2 master08 phases + master04.5 rule-code definitions (VS*/VD*/VACMC*/VARX*) live
- [ADL2 REST wire-contract location](adl2-rest-wire-contract-location.md) — paths/params/schemas/responses for the 5 ITS-REST ADL2 template ops + OperationalTemplateV2 is an opaque `type:object` + NO ADL2 CNF Robot suite
- [ADL2/cADL2/ODIN source-parser spec location](adl2-parser-spec-location.md) — file map for encoding/cADL/paths/identification/rules/terminology + the vendored GRAMMAR GAP (ANTLR .g4 files not vendored) + syntax-error catalogue snapshot
