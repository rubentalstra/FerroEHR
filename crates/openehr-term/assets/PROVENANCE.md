# Vendored openEHR terminology assets

- Source: https://github.com/openEHR/specifications-TERM
- Ref: master (TERM 3.1.0)
- Commit: `007d0dddcdd77648711681878b54ace021b2fbd5`
- Upstream paths:
  - `computable/XML/{en,es,ja,pt,zh}/openehr_terminology.xml` → `assets/<lang>/openehr_terminology.xml`
  - `computable/XML/openehr_external_terminologies.xml` → `assets/openehr_external_terminologies.xml`
  - `computable/XML/PropertyUnitData.xml` → `assets/PropertyUnitData.xml`
  - `computable/XML/schema/*.xsd` → `assets/schema/`

The computable form is the definitive expression of the openEHR Support
Terminology (`docs/specs/openehr/TERM/docs/SupportTerminology/master02-overview.adoc`),
so every asset here must stay **byte-identical** to the upstream file at the
pinned commit — never "clean up", reformat, or re-indent them; known upstream
defects (e.g. SPECPR-51) are handled in access logic with a citation, never by
editing the asset.

The same commit is vendored as spec text + computable XML at
`docs/specs/openehr/TERM/` (see its `PROVENANCE.md`); the `asset_identity`
test in `tests/it/` byte-compares this directory against that copy, so a
re-vendor of either side fails loudly until both move together. The XSDs are
excluded from the spec-text vendoring (text-formats-only script), so their
byte-identity to upstream was verified directly at the pinned commit
(2026-08-01).
