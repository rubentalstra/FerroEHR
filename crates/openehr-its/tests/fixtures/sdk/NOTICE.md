# Vendored EHRbase `openEHR_SDK` operational templates

Every `*.opt` in this directory is an OPT 1.4 operational template vendored
**verbatim** from EHRbase's open-source `openEHR_SDK` repository — the subset of
`test-data/src/main/resources/operationaltemplate/*.opt` whose template id has a
matching canonical-JSON composition in `openehr-its`'s vendored corpus
(`crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/`). The
pairing gives the FLAT converter real `(canonical composition, operational
template)` inputs for its round-trip and golden tests.

- Source: <https://github.com/ehrbase/openEHR_SDK> (`test-data/src/main/resources/operationaltemplate/`)
- Copyright: vitagroup AG / EHRbase contributors
- License: **Apache License, Version 2.0** — see <http://www.apache.org/licenses/LICENSE-2.0>

They are used here **only as test inputs**. No modification has been made to the
files. Per the Apache-2.0 terms, this NOTICE preserves the attribution and
license of the vendored material.
