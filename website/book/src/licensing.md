# Licensing & legal

This page is the complete licensing picture for FerroEHR: what the project's own
code is licensed under, which third-party material ships inside the repository
and the container images, and the trademark and lineage acknowledgments. It is a
summary for evaluators and deployers, not legal advice.

<!-- toc -->

## FerroEHR's own code — MIT

Everything written for this project — the server and application crates, the
code generator and tooling, the admin console, and the hand-written
specification engines (`openehr-query`, `openehr-adl`) — is licensed under the
[MIT License](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSE).
You can use, modify, and redistribute it freely, including commercially,
provided the copyright and permission notice are preserved. The copyright holder
is stated as *FerroEHR contributors*, identically in `LICENSE`, in `REUSE.toml`,
and in every first-party file header — a CI gate compares the three so they
cannot drift apart.

The six published spec crates that **embed** openEHR-derived material —
`openehr-base`, `openehr-rm`, `openehr-am`, `openehr-lang`, `openehr-term`,
`openehr-its` — declare `MIT AND Apache-2.0` and ship both license texts in the
package: the emitted Rust is this project's, while the specification
documentation text carried in the generated doc comments and the vendored JSON
Schema are openEHR's. `openehr-term` additionally embeds the official openEHR
terminology XML, which is CC-BY-SA 3.0 (see the table below) and is
redistributed verbatim with attribution. See [Rust crates](crates.md#licensing).

## Vendored third-party material

The repository vendors external material verbatim — machine-readable
specification artifacts the code generator consumes, the openEHR specification
text used as the conformance oracle, and real-world clinical models and fixtures
used as test corpora. Each family keeps its upstream license:

| Material | Source | License |
|---|---|---|
| openEHR machine-readable artifacts (BMM meta-models, XML Schemas, OpenAPI documents, JSON Schemas) | the openEHR `specifications-ITS-*` repositories | [Apache-2.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/Apache-2.0.txt) |
| The normative ADL, cADL, ODIN, BEL and Expression-Language ANTLR grammars | `openEHR/adl-antlr`, `openEHR/openEHR-antlr4` | [Apache-2.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/Apache-2.0.txt) |
| openEHR specification text (the conformance reference) | the openEHR `specifications-*` repositories | [CC-BY-SA 3.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/CC-BY-SA-3.0.txt) |
| The AQL grammar and the computable terminology assets (the terminology XML the server embeds, and its schemas) | `specifications-QUERY`, `specifications-TERM` | [CC-BY-SA 3.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/CC-BY-SA-3.0.txt) |
| Clinical models (archetypes and templates) from the openEHR Clinical Knowledge Manager and the openEHR ADL archetype library | ckm.openehr.org, `openEHR/adl-archetypes` | per-file `licence` metadata — a **mix** of [CC-BY-SA 4.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/CC-BY-SA-4.0.txt) and [CC-BY-SA 3.0](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/CC-BY-SA-3.0.txt) |
| Test corpora (archie fixtures and reference models, Better `web-template-tests`, EHRbase SDK canonical-JSON data) | Nedap, Better Ltd, vitasystems | Apache-2.0 |
| Three ISO 13606 / rejected-extract BMM reference models inside the archie corpus | offered by their authors under MPL 1.1 / GPL 2.0 / LGPL 2.1 | taken under [MPL 1.1](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/MPL-1.1.txt) — see the election below |
| One terminology schema file, `PropertyUnitData.xsd` | ADL Designer / ADL2-tools, via the openEHR TERM assets | [AGPL-3.0-only](https://github.com/rubentalstra/FerroEHR/blob/develop/LICENSES/AGPL-3.0-only.txt) — see the contradiction below |
| The self-hosted KaTeX stylesheet and fonts this documentation site renders maths with | KaTeX contributors | MIT |

Every vendored tree in the repository carries a provenance note naming its exact
upstream source and pinned revision, with the upstream `LICENSE` file vendored
alongside where the source publishes one; the specification and corpus trees
name their license there as well, and `REUSE.toml` below is the authority for
all of them. The fuzzing seed corpus is a copy of several of those trees, so it
is declared under the union of their licenses rather than guessing each seed's
origin from its filename.

**The clinical-model corpora are mixed, and the table says so on purpose.** A
first-hand count over the vendored CKM material finds both CC-BY-SA 4.0 (the
majority) and CC-BY-SA 3.0 (several hundred files), so no single version is a
true statement about the tree. Each archetype carries its own `licence` field
inside its `description` block, and that per-file metadata is the authority for
any individual file — which also means licensing for this material already
survives being copied out of the repository.

**Two positions worth stating explicitly**, because both are the kind of thing a
compliance review finds and a summary table hides:

- **The MPL election.** Three BMM reference models in the archie corpus
  (`cen_EN13606_0.95.bmm`, `cen_ts14796_0.90.bmm`,
  `openehr_ehr_extract_999.bmm`) are offered by their authors under a
  tri-license: MPL 1.1, GPL 2.0, or LGPL 2.1. This project **takes them under
  MPL 1.1**, a file-scoped weak copyleft. The election is recorded in that
  corpus's `PROVENANCE.md`, so **no GPL or LGPL obligation attaches to anything
  here**.
- **One upstream contradiction, not resolved by us.**
  `crates/openehr-term/assets/schema/PropertyUnitData.xsd` carries an ADL
  Designer / ADL2-tools header offering it under the GNU Affero General Public
  License, inside an upstream repository whose own `LICENSE` is CC-BY-SA 3.0.
  Both cannot be right, the contradiction is upstream's, and re-licensing
  someone else's file is not ours to do — so it is declared at the **more
  restrictive** of the two readings, the one the file's own text asserts. No
  obligation reaches a consumer: the terminology schemas are excluded from the
  published crate by that crate's `include` list, so the file ships in nothing.

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
- **Every first-party source file carries the header inside itself** — an
  `SPDX-FileCopyrightText` line and an `SPDX-License-Identifier` line stating the
  same position `REUSE.toml` declares for it, so a file copied out of this
  repository takes its licensing along. Rust files of the six published spec
  crates state `MIT AND Apache-2.0`; every other first-party Rust, shell, SQL and
  YAML file states `MIT`. A copied migration or script arrives licensed, which is
  the whole point.

The vendored trees are glob-declared rather than headered for a reason that is
not convenience: **no vendored file may be edited**, so a header sweep over
third-party material was never available. `REUSE.toml` is the mechanism that
makes the declaration complete without touching one. The generated spec-crate
sources are the mirror case — a hand-written header there would be erased by the
next code-generation run, so they receive theirs from the code generator, which
stamps every file it writes.

All of it is gated in CI, and by more than one check, because each one can only
see part of the picture:

- `reuse lint` proves the declarations are **complete** — every file carries
  licensing information and no license text is orphaned.
- A second check fails the build if the set of licenses declared in `REUSE.toml`,
  the texts present in `LICENSES/`, and the licenses named **on this page** ever
  stop agreeing. A license cannot enter the tree without this chapter acquiring
  it.
- Two header checks fail the build if a first-party file loses its header or
  states a license other than the one declared for its path — one for Rust, one
  for shell, SQL and YAML.
- One check fails if the copyright holder is stated differently in `LICENSE`,
  `REUSE.toml`, and the file headers.
- One check refuses copyleft license **text** inside this project's own source,
  which is a conflict no reviewer reliably catches by eye.

What none of them can check is whether the *prose* on this page is correct. No
tool can judge that, which is why any change to what the repository
redistributes updates this chapter in the same pull request.

The CC-BY-SA specification text and clinical models are redistributed
**verbatim, with attribution** — they are reference and test material, not part
of the compiled server. The FerroEHR binary you deploy is built from MIT code,
plus the Apache-2.0 machine-readable inputs the generated crates carry, plus the
CC-BY-SA 3.0 openEHR terminology bundle `openehr-term` compiles in (five
languages, the external-terminology index, and the property/unit data). No
copyleft obligation beyond those attribution-and-share-alike terms on the
verbatim data attaches to anything shipped.

## Rust dependencies

All third-party Rust crates are pinned in the workspace manifest and
license-gated in CI with `cargo deny`, which checks licenses, security
advisories, bans, and sources on every change. The allow-list is permissive
(MIT, Apache-2.0, the BSD family, ISC, Zlib, BSL-1.0, Unicode-3.0, CC0-1.0,
MIT-0, CDLA-Permissive-2.0) and deliberately admits two **file-scoped weak
copyleft** licenses: MPL 2.0, and CDDL 1.0 as a single crate-scoped exception for
the flamegraph renderer the profiling instruments use. Obligations under both
attach to those crates' own files, which are consumed unmodified. No strong
copyleft — GPL, LGPL, AGPL, SSPL — is admitted, and a new dependency carrying one
fails the build.

A separate FOSSA lane publishes dependency and license analysis from the
committed CLI configuration for review. It is analysis-only by design and gates
no merge; `cargo deny` is the gate.

## Trademarks and lineage

- **openEHR®** is the registered trademark of the
  [openEHR Foundation](https://www.openehr.org/). FerroEHR is an independent
  implementation of the openEHR specifications and is **not affiliated with or
  endorsed by** the openEHR Foundation.
- FerroEHR began as a fork of **EHRbase**, developed by
  [vitasystems GmbH](https://www.vitagroup.ag/) and the
  [Peter L. Reichertz Institute](https://www.plri.de/), and keeps that lineage in
  its git history. EHRbase itself remains Apache-2.0; no code from it is present
  in this tree, and it is consulted as prior art only. FerroEHR is not affiliated
  with or endorsed by the EHRbase project. The measured
  [comparison](comparison.md) between the two is published in both directions.

## Questions

If you need a clarification for a compliance review, open a
[GitHub discussion or issue](https://github.com/rubentalstra/FerroEHR/issues) —
provenance questions can usually be answered by pointing at the exact
`PROVENANCE.md` and upstream pin.
