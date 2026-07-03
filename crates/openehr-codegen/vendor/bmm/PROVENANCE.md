# Vendored openEHR BMM meta-model (JSON)

Source: https://github.com/openEHR/specifications-ITS-BMM
Pinned commit: `37b317396eb9b5f03c4c422296a15bdd665028dd` (master, vendored 2026-07-03)

Codegen consumes the **JSON** BMM serialization (`serde_json`), not the ODIN
form: it is a cleaner, structured serialization (real arrays, structured
`cardinality`, explicit `_type` tags) of the identical meta-model, and exists
for our exact pinned versions.

| File                        | Upstream path                                    |
|-----------------------------|--------------------------------------------------|
| openehr_rm_1.2.0.bmm.json   | components/RM/json/openehr_rm_1.2.0.bmm.json     |
| openehr_base_1.3.0.bmm.json | components/BASE/json/openehr_base_1.3.0.bmm.json |
| openehr_term_3.1.0.bmm.json | components/TERM/json/openehr_term_3.1.0.bmm.json |
| openehr_am_1.4.0.bmm.json   | components/AM/json/openehr_am_1.4.0.bmm.json     |
| openehr_am_2.4.0.bmm.json   | components/AM/json/openehr_am_2.4.0.bmm.json     |
| openehr_lang_1.1.0.bmm.json | components/LANG/json/openehr_lang_1.1.0.bmm.json |

These are the deterministic input to `openehr-codegen` (ADR-004). The ODIN
reader in `openehr-lang::odin` is retained for ADL/ODIN *instance* parsing
(P8/P9), not for BMM ingestion.
