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
