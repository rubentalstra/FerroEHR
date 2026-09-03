# Rust crates

FerroEHR's openEHR specification layer is published on crates.io as eight
standalone Rust crates, usable without the CDR. They are the same crates the
server itself is built on: the spec types are generated deterministically from
the official openEHR machine-readable artifacts (BMM/XSD/OpenAPI), and the
engines (ADL, AQL, serialization runtimes) are hand-written against the
vendored specification text.

| Crate | Implements | What it gives you |
|---|---|---|
| [`openehr-base`](https://crates.io/crates/openehr-base) | BASE 1.2.0 + 1.3.0 | Foundation + base types (identification, intervals, ISO-8601 types) |
| [`openehr-rm`](https://crates.io/crates/openehr-rm) | RM 1.1.0 + 1.2.0 | The Reference Model: `COMPOSITION`, data structures/types, change control, generated invariant validation, the static RM attribute model |
| [`openehr-am`](https://crates.io/crates/openehr-am) | AM 1.4.0 + 2.4.0 | The Archetype Object Model, both majors side by side |
| [`openehr-adl`](https://crates.io/crates/openehr-adl) | ADL 2.4.0 | ADL2/cADL/ODIN parser, AOM2 validation, flattener, OPT2, ADL 1.4→2 conversion |
| [`openehr-term`](https://crates.io/crates/openehr-term) | TERM 3.1.0 | Terminology model + the embedded official openEHR terminology (five languages: `en`, `es`, `ja`, `pt`, `zh`) |
| [`openehr-lang`](https://crates.io/crates/openehr-lang) | LANG 1.0.0 + 1.1.0 | The BMM meta-model and its P_BMM schema form, plus hand-written ODIN, BEL and Expression-Language readers |
| [`openehr-query`](https://crates.io/crates/openehr-query) | QUERY 1.1.0 | AQL lexer, parser, typed AST, and canonical printer |
| [`openehr-its`](https://crates.io/crates/openehr-its) | ITS-JSON, ITS-XML, ITS-REST 1.1.0 | Canonical JSON + XML codecs, the generated ITS-REST contract, OPT 1.4, Simplified Formats (FLAT/STRUCTURED/Web Template) |

```toml
[dependencies]
openehr-rm = "0.0.29"
openehr-its = "0.0.29"
```

All eight are **edition 2024** with an MSRV of **Rust 1.96**, and all eight
inherit the workspace lint table, including `unsafe_code = "forbid"`, which no
attribute anywhere in the crate can relax. There is no `unsafe` block in the
published specification layer.

<!-- toc -->

## Generations: reaching more than one specification version

A crate generated from more than one openEHR generation exposes each as a
**version-named module**: `openehr_rm::v1_1` and `openehr_rm::v1_2`,
`openehr_base::v1_2` / `v1_3`, `openehr_am::v1_4` / `v2_4`,
`openehr_lang::v1_0` / `v1_1`, `openehr_term::v3_1`. The crate's `prelude`
re-exports the **current** generation, so ordinary code needs no module path;
an older generation is reached by naming its module in full:

```rust,no_run
// The current generation (RM 1.2.0), via the crate prelude:
use openehr_rm::prelude::Composition;

// The released generation (RM 1.1.0), via its own module path. Import
// renaming is not used anywhere in this project; where both generations
// appear in one file, give one of them a type alias:
type Rm110Composition = openehr_rm::v1_1::composition::composition::Composition;
```

Each generation mirrors its specification's own package structure, so the path
after the generation module reads the same in both. Every generation module
also carries its own `prelude`; no name from one generation is ever mixed into
another.

`openehr-lang` is the one crate whose generations also differ in *what they
contain*, because upstream publishes two BMM meta-models side by side. Its
`v1_1` generation carries them as sibling specification units: the stable,
tool-implemented BMM v2.x model (`bmm`, its persistence form
`bmm_persistence`, and the `beom` expression model) is on the generation's
prelude, while the paused BMM3 model (`bmm3`) is reachable only by full module
path. They cannot be merged (a set of class names, `BmmClass`, `BmmModel` and
`BmmPackage` among them, occurs in both units with materially different
shapes) so the prelude carries the stable units and the choice stays explicit
at the use site. The older `v1_0` generation is what its release actually
defines: the BMM model plus an ODIN reader, with no BEL or Expression-Language
notation.

## Versioning

The package version is the crates' **own independent SemVer line**: it tracks
this implementation's code and moves freely with fixes and improvements, never
with the vendored openEHR specification versions. While the line is `0.0.x`,
expect breaking changes between releases, which always ship in lockstep across
all eight crates.

The implemented specification version is therefore a **separate datum, per
generation**. Each generated crate emits a `Generation` enum that is the only
authority for it: one variant per generation module, `Default` marking the
current one, each variant carrying its specification version as a `const fn`,
and `Display`/`FromStr` round-tripping the module token:

```rust,no_run
assert_eq!(openehr_rm::Generation::default().spec_version(), "1.2.0");
assert_eq!(openehr_rm::Generation::V1_1.spec_version(), "1.1.0");
assert_eq!(openehr_rm::Generation::default().as_str(), "v1_2");
```

There is deliberately **no crate-level `SPEC_VERSION` constant in the generated
crates**: a single constant would contradict a caller using a non-current
generation. Exactly three crates implement one specification each and do expose
one: `openehr_its::SPEC_VERSION`, `openehr_query::SPEC_VERSION`,
`openehr_adl::SPEC_VERSION`.

## Building `openehr-its` without its dependencies

`openehr-its` puts every codec behind one default feature, `full`. Taken with
`default-features = false` it compiles to the SMART App Launch scope grammar
alone (std-only, with no dependency of any kind) so a REST client targeting
`wasm32-unknown-unknown` can parse scope strings with the very grammar the CDR
enforces instead of carrying a second one:

```toml
[dependencies]
openehr-its = { version = "0.0.29", default-features = false }
```

## Releases

The eight crates are published through a manual release lane that authenticates
with **crates.io Trusted Publishing** (OIDC, a protected environment, no
long-lived token anywhere) and publishes them **one at a time in dependency
order**, treating "already exists on the index" as done, so a run interrupted
halfway can simply be re-run to finish the set. The lane then reads the
registry back and refuses to report success unless all eight resolve at the
same version, because while the line is `0.0.x` a straggler makes its siblings'
internal requirements unresolvable for every consumer.

## Licensing

`openehr-query` and `openehr-adl` are plain **`BUSL-1.1`**: their packages
ship only their own Rust sources, the README, and the `LICENSE-BUSL-1.1` text.
Five of the crates that embed material derived from the official openEHR
machine-readable artifacts (generated types carrying specification
documentation text, and the vendored ITS-JSON schema) declare
**`BUSL-1.1 AND Apache-2.0`** and ship both license texts in the package
(`LICENSE-BUSL-1.1` and `LICENSE-APACHE-2.0`). `openehr-its` is the one of
them that packages a third-party file as bytes rather than as generated code,
so its README carries that file's attribution (upstream repository, the exact
vendored commit, and the license) inside the package, where it travels with
any redistribution.

`openehr-term` carries a third term, because it embeds a different kind of
openEHR material: the official terminology XML (the five language bundles, the
external-terminology index, and the property/unit data) is **CC-BY-SA 3.0**,
redistributed verbatim with attribution. Its manifest declares
**`BUSL-1.1 AND Apache-2.0 AND CC-BY-SA-3.0`** and the package ships all three
license texts, so the declaration a consumer reads names every license the
crate's own bytes are under. If you redistribute the crate, the terminology data
travels under CC-BY-SA 3.0.

Each published version becomes available under the Apache License 2.0 four
years after it is published. Versions 0.0.56 and earlier were published under
the MIT terms in force at the time and keep them. The full picture, including
the vendored material that never reaches a published package, is in
[Licensing](licensing.md).

The openEHR specifications themselves are © the openEHR Foundation.
