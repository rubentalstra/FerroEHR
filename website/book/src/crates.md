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
| [`openehr-term`](https://crates.io/crates/openehr-term) | TERM 3.1.0 | Terminology model + the embedded official openEHR terminology (five languages) |
| [`openehr-lang`](https://crates.io/crates/openehr-lang) | LANG 1.0.0 + 1.1.0 | The BMM meta-model + hand-written ODIN/BEL readers |
| [`openehr-query`](https://crates.io/crates/openehr-query) | QUERY 1.1.0 | AQL lexer, parser, typed AST, and canonical printer |
| [`openehr-its`](https://crates.io/crates/openehr-its) | ITS-REST 1.1.0 | Canonical JSON + XML codecs, the generated ITS-REST contract, OPT 1.4, Simplified Formats (FLAT/STRUCTURED/Web Template) |

```toml
[dependencies]
openehr-rm = "0.0.10"
openehr-its = "0.0.10"
```

## Generations: reaching more than one specification version

A crate generated from more than one openEHR generation exposes each as a
**version-named module** — `openehr_rm::v1_1` and `openehr_rm::v1_2`,
`openehr_base::v1_2` / `v1_3`, `openehr_am::v1_4` / `v2_4`,
`openehr_lang::v1_0` / `v1_1`, `openehr_term::v3_1`. The crate's `prelude`
re-exports the **current** generation, so ordinary code needs no module path;
an older generation is reached by naming its module in full:

```rust,no_run
// The current generation (RM 1.2.0), via the crate prelude:
use openehr_rm::prelude::Composition;

// The released generation (RM 1.1.0), via its own module path:
use openehr_rm::v1_1::composition::composition::Composition as Rm110Composition;
```

Each generation mirrors its specification's own package structure, so the path
after the generation module reads the same in both. Every generation module
also carries its own `prelude`; no name from one generation is ever mixed into
another.

## Versioning

The package version is the crates' **own independent SemVer line** — it
tracks this implementation's code and moves freely with fixes and
improvements, never with the vendored openEHR specification versions. While
the line is `0.0.x`, expect breaking changes between releases, which always
ship in lockstep across all eight crates.

The implemented specification version is therefore a **separate datum, per
generation**. Each generated crate emits a `Generation` enum that is the only
authority for it — `Default` is the current generation, and every variant
carries its specification version as a `const fn`:

```rust,no_run
assert_eq!(openehr_rm::Generation::default().spec_version(), "1.2.0");
assert_eq!(openehr_rm::Generation::V1_1.spec_version(), "1.1.0");
assert_eq!(openehr_rm::Generation::default().as_str(), "v1_2");
```

There is deliberately **no crate-level `SPEC_VERSION` constant in the
generated crates**: a single constant would contradict a caller using a
non-current generation. The three hand-written crates implement exactly one
specification each and do expose one — `openehr_its::SPEC_VERSION`,
`openehr_query::SPEC_VERSION`, `openehr_adl::SPEC_VERSION`.

## Licensing

`openehr-query` and `openehr-adl` are plain **MIT**. The six crates that embed
material derived from the official openEHR machine-readable artifacts
(generated types carrying specification documentation text, the terminology
XML, the ITS-JSON schema) are **`MIT AND Apache-2.0`**, with both license
texts shipped in the package. The openEHR specifications themselves are © the
openEHR Foundation.
