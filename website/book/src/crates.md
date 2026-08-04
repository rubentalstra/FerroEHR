# Rust crates

FerroEHR's openEHR specification layer is published on crates.io as eight
standalone Rust crates, usable without the CDR. They are the same crates the
server itself is built on: the spec types are generated deterministically from
the official openEHR machine-readable artifacts (BMM/XSD/OpenAPI), and the
engines (ADL, AQL, serialization runtimes) are hand-written against the
vendored specification text.

| Crate | Implements | What it gives you |
|---|---|---|
| [`openehr-base`](https://crates.io/crates/openehr-base) | BASE 1.3.0 | Foundation + base types (identification, intervals, ISO-8601 types) |
| [`openehr-rm`](https://crates.io/crates/openehr-rm) | RM 1.2.0 | The Reference Model: `COMPOSITION`, data structures/types, change control, generated invariant validation, the static RM attribute model |
| [`openehr-am`](https://crates.io/crates/openehr-am) | AM 1.4 + 2.4 | The Archetype Object Model, both generations (`am14`/`am24`) |
| [`openehr-adl`](https://crates.io/crates/openehr-adl) | ADL 2.4 | ADL2/cADL/ODIN parser, AOM2 validation, flattener, OPT2, ADL 1.4→2 conversion |
| [`openehr-term`](https://crates.io/crates/openehr-term) | TERM 3.1.0 | Terminology model + the embedded official openEHR terminology (five languages) |
| [`openehr-lang`](https://crates.io/crates/openehr-lang) | LANG | The BMM meta-model (both generations) + an ODIN parser |
| [`openehr-query`](https://crates.io/crates/openehr-query) | QUERY 1.1 | AQL lexer, parser, and typed AST |
| [`openehr-its`](https://crates.io/crates/openehr-its) | ITS-REST 1.1.0 | Canonical JSON + XML codecs, the generated ITS-REST contract, OPT 1.4, Simplified Data Formats (FLAT/STRUCTURED/Web Template) |

```toml
[dependencies]
openehr-rm = "0.0.1"
openehr-its = "0.0.1"
```

## Versioning

The packages are on a **pre-stabilisation `0.0.x` line** while the public API
settles — expect breaking changes between `0.0.x` releases, which always ship
in lockstep across all eight crates. Once stable, each crate adopts the
version of the openEHR specification it implements.

The implemented specification version is independent of the package version
and always available at runtime:

```rust,no_run
assert_eq!(openehr_rm::SPEC_VERSION, "1.2.0");
```

## Licensing

The Rust code is MIT. Crates that embed material derived from the official
openEHR machine-readable artifacts (generated types with spec doc text, the
terminology XML, the ITS-JSON schema) are `MIT AND Apache-2.0`, with both
license texts shipped in the package. The openEHR specifications themselves
are © the openEHR Foundation.
