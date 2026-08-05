# Licensing & legal

This page is the complete licensing picture for FerroEHR: what the project's
own code is licensed under, which third-party material ships inside the
repository and container images, and the trademark and lineage
acknowledgments. It is a summary for evaluators and deployers, not legal
advice.

<!-- toc -->

## FerroEHR's own code — MIT

Everything written for this project — the server and application crates, the
code generator and tooling, **and the generated openEHR specification
crates** (they are this project's own emitted Rust) — is licensed under the
[MIT License](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSE).
You can use, modify, and redistribute it freely, including commercially,
provided the copyright and permission notice are preserved.

## Vendored third-party material

The repository vendors external material verbatim — machine-readable
specification artifacts the code generator consumes, the openEHR
specification text used as the conformance oracle, and real-world clinical
models and fixtures used as test corpora. Each family keeps its upstream
license:

| Material | Source | License |
|---|---|---|
| openEHR machine-readable artifacts (BMM meta-models, XML Schemas, OpenAPI, JSON Schemas) | the openEHR `specifications-ITS-*` repositories | [Apache-2.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSE-APACHE-2.0) |
| openEHR specification text (the conformance reference) | the openEHR `specifications-*` repositories | [CC-BY-SA 3.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSE-CC-BY-SA-3.0) |
| Clinical models (archetypes and templates) from the openEHR Clinical Knowledge Manager and the openEHR ADL archetype library | ckm.openehr.org, `openEHR/adl-archetypes` | per-file `licence` metadata, predominantly CC-BY-SA 3.0 |
| Test corpora (archie fixtures, Better `web-template-tests`, EHRbase SDK canonical-JSON data) | Nedap, Better Ltd, vitasystems | Apache-2.0 |

Every vendored tree in the repository carries a `PROVENANCE.md` naming its
exact upstream source, pinned revision, and license, with the upstream
`LICENSE` file vendored alongside where the source publishes one.

The CC-BY-SA specification text and clinical models are redistributed
**verbatim, with attribution** — they are reference and test material, not
part of the compiled server. The FerroEHR binary you deploy is built from
MIT code plus Apache-2.0-licensed machine-readable inputs.

## Rust dependencies

All third-party Rust crates are pinned in the workspace manifest and
license-gated in CI with `cargo deny` (licenses, security advisories, bans,
and sources are all checked on every change). No copyleft crate licenses are
admitted by that gate.

## Trademarks and lineage

- **openEHR®** is the registered trademark of the
  [openEHR Foundation](https://www.openehr.org/). FerroEHR is an independent
  implementation of the openEHR specifications and is **not affiliated with
  or endorsed by** the openEHR Foundation.
- FerroEHR began as a fork of **EHRbase**, developed by
  [vitasystems GmbH](https://www.vitagroup.ag/) and the
  [Peter L. Reichertz Institute](https://www.plri.de/), and keeps that
  lineage in its git history. EHRbase itself remains Apache-2.0; no code
  from it is present in this tree. FerroEHR is not affiliated with or
  endorsed by the EHRbase project.

## Questions

If you need a clarification for a compliance review, open a
[GitHub discussion or issue](https://github.com/rubentalstra/FerroEHR/issues)
— provenance questions can usually be answered by pointing at the exact
`PROVENANCE.md` and upstream pin.
