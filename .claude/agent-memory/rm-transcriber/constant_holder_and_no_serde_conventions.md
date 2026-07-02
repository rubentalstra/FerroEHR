---
name: constant-holder-and-no-serde-conventions
description: Settled Rust shapes for two BASE 1.2.0 class patterns not explicitly covered by ADR-001 — constant-holder classes (BASIC_DEFINITIONS-style) and enumeration/interface classes in a crate that has no serde dependency yet.
metadata:
  type: feedback
---

Two patterns came up transcribing `base_types.definitions` and
`base_types.builtins` (BASE 1.2.0) that ADR-001 does not name explicitly;
recorded here so a later transcriber does not re-derive them, and appended
to `docs/ROSETTA.md` via the `rosetta-mapping` skill at the end of the
session that discovered them (check ROSETTA is actually current before
relying on this from memory alone).

**Constant-holder classes** (a class that declares only `Constants`, no
instance attributes, e.g. `BASIC_DEFINITIONS`; and its inheritor
`OPENEHR_DEFINITIONS`, which adds one attribute but is itself only ever used
as a constant/default-value provider, never instantiated) → a zero-field
Rust struct (`pub struct BasicDefinitions;`) with one associated `const` per
spec constant, not a struct with a `Default` impl and real fields. Char
constants given as Eiffel octal escapes (`'\015'`, `'\012'`) decode as
`char` literals directly (`'\015'` = octal 15 = decimal 13 = `'\r'`;
`'\012'` = octal 12 = decimal 10 = `'\n'`) — do not leave these as opaque
octal-string TODOs, the arithmetic is unambiguous. Where a subclass
"inherits" a constant-holder purely to bring its constants into scope
(`OPENEHR_DEFINITIONS` inherits `BASIC_DEFINITIONS`), Rust has no struct
inheritance to mirror this with, so the relationship is documentation-only
(a PORT NOTE plus, optionally, a `#[allow(dead_code)] type
_InheritsX = X;` marker) rather than composing an empty parent struct into
the child.

**Why:** these classes have no meaningful instances in the RM/AM — every
real use is `ClassName::CONSTANT_NAME`, exactly what a Rust associated
const gives you, with zero indirection. A struct-with-fields-and-Default
shape would invite constructing an instance that nothing in the spec ever
does.

**Enumeration and no-attribute interface classes in a crate with no serde
dependency yet.** `openehr-base` and `openehr-foundation` (as of the P1
session that discovered this) declare `serde` nowhere in their
`Cargo.toml`, even though `serde` is pinned at the workspace level and the
general RM transcription rule (`.claude/rules/rm-transcription.md`) calls
for a serde rename to a canonical discriminator string. Do not add a
`serde` dependency edge yourself to satisfy that rule in a definitions/
builtins-scope task — check whether the crate's `Cargo.toml` already lists
`serde` first (`openehr-foundation::primitive_types::any.rs`,
`double.rs`, `string.rs`, etc. are all serde-free at time of writing). If
it is not there yet, transcribe a spec enumeration (`VALIDITY_KIND`,
`VERSION_STATUS`) as a plain Rust `enum` plus a `const fn symbol(self) ->
&'static str` method that renders the spec's own lower-case/snake_case
identifier, so a later serde impl at the RM/serde layer has a
spec-verified string to key its rename off. Do not derive
`Serialize`/`Deserialize` speculatively.

**Why:** editing `Cargo.toml` is outside a definitions/builtins-scoped
transcription task's remit (and was explicitly out of scope in the
task instructions for this session), and adding a real dependency the rest
of the crate does not use yet would be a scope-creeping structural change a
reviewer would have to notice and question. The `symbol()` method preserves
the spec-exact string so nothing is lost when serde is wired in later.

**How to apply:** any future BASE/RM transcription task that hits a
constant-only class, or an enumeration/trait-shaped class inside
`openehr-foundation`/`openehr-base` before those crates pick up a `serde`
dependency. Re-check the crate's `Cargo.toml` each time rather than trusting
this memory indefinitely — once `serde` is added (likely alongside the
canonical-JSON work in P4/`openehr-serde`, or whenever an RM class first
needs `_type` discriminator serialization), enums transcribed this way
should gain a real `#[serde(rename = "...")]` derive using the exact string
`symbol()` already documents.

See also [[phase-a-forward-references]] for the sibling pattern of
referencing not-yet-transcribed dependency types.
