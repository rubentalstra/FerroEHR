# Provenance

- Upstream: https://github.com/citation-file-format/citation-file-format
- Ref: tag `1.2.0` (commit `396f738fb025b1d8acdb02a56ffc923f95dc8999`)
- Files: `schema.json` (the CFF 1.2.0 JSON Schema, draft-07), `LICENSE`
  (CC-BY-4.0)
- Fetched by: `scripts/vendor/cff-schema.sh` — never hand-edit; re-run the
  script to update (.claude/rules/vendored-corpora.md)
- Consumer: the `citation-guard` CI job (`.github/workflows/ci.yml`)
  validates `CITATION.cff` against it with a pinned `jsonschema-cli` over
  the `yq -o=json` conversion (#2791)
