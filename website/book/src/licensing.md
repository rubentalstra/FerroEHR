# Licensing & legal

This page is the complete licensing picture for FerroEHR: what the project's
own code is licensed under, which third-party material ships inside the
repository and container images, and the trademark and lineage
acknowledgments. It is a summary for evaluators and deployers, not legal
advice.

<!-- toc -->

## FerroEHR's own code — MIT

Everything written for this project — the server and application crates, the
code generator and tooling, the admin console, and the hand-written
specification engines (`openehr-query`, `openehr-adl`) — is licensed under the
[MIT License](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSE).
You can use, modify, and redistribute it freely, including commercially,
provided the copyright and permission notice are preserved.

The six published spec crates that **embed** material derived from the official
openEHR machine-readable artifacts — `openehr-base`, `openehr-rm`,
`openehr-am`, `openehr-lang`, `openehr-term`, `openehr-its` — declare
`MIT AND Apache-2.0` and ship both license texts in the package: the emitted
Rust is this project's, the specification documentation text, terminology XML,
and JSON Schema carried inside it are openEHR's. See
[Rust crates](crates.md#licensing).

## Vendored third-party material

The repository vendors external material verbatim — machine-readable
specification artifacts the code generator consumes, the openEHR
specification text used as the conformance oracle, and real-world clinical
models and fixtures used as test corpora. Each family keeps its upstream
license:

| Material | Source | License |
|---|---|---|
| openEHR machine-readable artifacts (BMM meta-models, XML Schemas, OpenAPI, JSON Schemas, ANTLR grammars) | the openEHR `specifications-ITS-*` repositories | [Apache-2.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/Apache-2.0.txt) |
| openEHR specification text (the conformance reference) | the openEHR `specifications-*` repositories | [CC-BY-SA 3.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/CC-BY-SA-3.0.txt) |
| Clinical models (archetypes and templates) from the openEHR Clinical Knowledge Manager and the openEHR ADL archetype library | ckm.openehr.org, `openEHR/adl-archetypes` | per-file `licence` metadata — a **mix** of [CC-BY-SA 4.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/CC-BY-SA-4.0.txt) and [CC-BY-SA 3.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/CC-BY-SA-3.0.txt) |
| Test corpora (archie fixtures, Better `web-template-tests`, EHRbase SDK canonical-JSON data) | Nedap, Better Ltd, vitasystems | Apache-2.0 |
| Three ISO 13606 / rejected-extract BMM reference models inside the archie corpus | offered by their authors under MPL 1.1 / GPL 2.0 / LGPL 2.1 | taken under [MPL 1.1](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/MPL-1.1.txt) — see the election below |
| One terminology schema file, `PropertyUnitData.xsd` | ADL Designer / ADL2-tools, via the openEHR TERM assets | [AGPL 3.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/AGPL-3.0-only.txt) — see the contradiction below |

Every vendored tree in the repository carries a `PROVENANCE.md` naming its
exact upstream source, pinned revision, and license, with the upstream
`LICENSE` file vendored alongside where the source publishes one.

**The clinical-model corpora are mixed, and the table says so on purpose.** A
first-hand count over the vendored CKM material finds both CC-BY-SA 4.0 (the
majority) and CC-BY-SA 3.0 (several hundred files), so no single version is a
true statement about the tree. Each archetype carries its own `licence` field
inside its `description` block, and that per-file metadata is the authority for
any individual file — which also means licensing for this material already
survives being copied out of the repository.

**Two positions worth stating explicitly**, because both are the kind of thing
a compliance review finds and a summary table hides:

- **The MPL election.** Three BMM reference models in the archie corpus
  (`cen_EN13606_0.95.bmm`, `cen_ts14796_0.90.bmm`,
  `openehr_ehr_extract_999.bmm`) are offered by their authors under a
  tri-license: MPL 1.1, GPL 2.0, or LGPL 2.1. This project **takes them under
  MPL 1.1**, a file-scoped weak copyleft. The election is recorded in that
  corpus's `PROVENANCE.md`, so **no GPL or LGPL obligation attaches to
  anything here**.
- **One upstream contradiction, not resolved by us.**
  `crates/openehr-term/assets/schema/PropertyUnitData.xsd` carries an ADL
  Designer / ADL2-tools header offering it under the GNU Affero General Public
  License, inside an upstream repository whose own `LICENSE` is CC-BY-SA 3.0.
  Both cannot be right, the contradiction is upstream's, and re-licensing
  someone else's file is not ours to do — so it is declared at the **more
  restrictive** of the two readings, the one the file's own text asserts. No
  obligation reaches a consumer: the file is excluded from the published crate
  by that crate's `include`, so it ships in nothing.

## Machine-readable licensing (REUSE 3.3)

The `PROVENANCE.md` arrangement above is accurate, and it stays. What it does
not do is survive a file leaving this repository — someone who lifts a single
archetype out of a test corpus takes a CC-BY-SA file bearing no marking they
copied. Since the stated premise of this project is that people build on it,
ship it and sell it, downstream file-level redistribution is the expected case.

So licensing is **also** published in the machine-readable form the
[REUSE Specification 3.3](https://reuse.software/spec-3.3/) defines:

- **[`LICENSES/`](https://github.com/rubentalstra/FerroEHR/tree/develop/LICENSES)**
  holds the full text of every license any file in the tree is offered under,
  named by SPDX identifier: `MIT`, `Apache-2.0`, `CC-BY-SA-3.0`,
  `CC-BY-SA-4.0`, `MPL-1.1`, `AGPL-3.0-only`.
- **[`REUSE.toml`](https://github.com/rubentalstra/FerroEHR/blob/develop/REUSE.toml)**
  declares, by glob, which files are offered under which — including the two
  positions above, represented rather than flattened.

It is expressed by glob rather than by per-file headers for the vendored trees
for a reason that is not convenience: **no vendored file may be edited**, so a
header sweep over third-party material was never available. `REUSE.toml` is the
mechanism that makes the declaration complete without touching one.

Both halves are gated in CI: `reuse lint` must report the project compliant,
and a second check fails the build if the set of licenses declared in
`REUSE.toml`, the texts present in `LICENSES/`, and the licenses named on this
page ever stop agreeing. This page cannot drift away from the declarations
silently.

The CC-BY-SA specification text and clinical models are redistributed
**verbatim, with attribution** — they are reference and test material, not
part of the compiled server. The FerroEHR binary you deploy is built from
MIT code plus Apache-2.0-licensed machine-readable inputs; the crates that
carry those inputs are the `MIT AND Apache-2.0` ones named above.

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
