# Porting Rules (the Rosetta core)

> **⚠️ Scope (ADR-004):** the **openEHR spec → Rust** mapping domain below is
> now realized by the **code generator** (`openehr-codegen`), not by hand — do
> not hand-transcribe spec classes. Those rules survive here as documentation of
> what the emitter produces (and as the contract for the `codegen.toml` override
> layer), not as a per-file hand-authoring checklist. The **Java → Rust** domain
> is still hand-applied for the `ehrbase-*` application port. See
> `docs/ADRs/ADR-004-spec-driven-codegen.md`.
>
> **⚠️ Scope (ADR-006):** the **Java → Rust** domain below is now a *reference*,
> not a literal-port checklist. The `ehrbase-*` application is built as a
> **modern idiomatic Rust service** consuming the generated `openehr-*` crates —
> we do **not** mirror EHRbase's Java class structure (rule 1), and the
> application phases are built as **compiling, tested increments** (rule 2's
> "need not compile" no longer applies). The type/idiom maps stay useful; the
> "mirror the source" and PORT-STATUS-trailer conventions do not apply to
> idiomatic app code. See `docs/ADRs/ADR-006-application-port-philosophy.md`.

This file is a lookup table, not prose. Two mapping domains are covered:
**Java → Rust** (for the EHRbase application code in `ehrbase-rest`,
`ehrbase-compat`, `ehrbase`) — hand-applied during the port — and **openEHR
spec → Rust** (for the generated `openehr-base`, `openehr-rm`, `openehr-am`, and
the hand-written `openehr-term`, `openehr-its`, `openehr-flat`, `openehr-lang`,
`openehr-query`) — now encoded in the generator.

For the full narrative rationale behind these rules, see `PORT_MASTER_PLAN.md`
Sections 4 and 14, and ADR-004 for how the spec domain is now generated.

---

## 1. Ground rules

| # | Rule |
|---|---|
| 1 | Mirror the source: same file/module names, same type names, same method names, same field order, same control flow. |
| 2 | Phases P1–P16 need not compile. Capture intent. Leave `todo!()` and `// TODO(port):` freely rather than getting stuck on a type error. |
| 3 | Every ported or transcribed file ends with the PORT STATUS trailer (Section 6 below). |
| 4 | Prefer closed Rust `enum`s for closed source hierarchies (Java sealed-ish class families, openEHR closed subtype sets). Trait objects only for genuinely open, archetype-driven runtime polymorphism. |

## 2. Two standing exceptions (allowed without special justification)

These two reshapes are pre-approved and do not need a `// PORT NOTE:` beyond
the ones described here — they are the routine Java-to-Rust idiom shift, not a
borrowck workaround.

| Java shape | Rust shape |
|---|---|
| A constructor that throws (validation, required-field checks) | `fn new(...) -> Result<Self, E>` |
| `AutoCloseable` / an explicit `close()` method | `impl Drop` |

Any *other* reshape forced by the borrow checker (e.g. an owning reference
becoming an index, a back-reference becoming `Weak`) is allowed only when
marked `// PORT NOTE: reshaped for borrowck` at the point of deviation.

## 3. Annotation vocabulary (mandatory, grep-able)

Every ported file carries these where relevant:

| Annotation | Meaning |
|---|---|
| `// TODO(port):` | Unfinished translation — the real logic is not yet in place. |
| `// PERF(port):` | A place to optimize after parity is reached; not a correctness gap. |
| `// PORT NOTE:` | A deliberate structural deviation from the source (e.g. a borrowck reshape, or a JVM-plumbing substitution per Section 5). |
| `// SAFETY:` | Justification for any `unsafe` block. Expected to be rare to nonexistent — this is a web service, not a runtime. |

## 4. Java → Rust type map

| Java | Rust |
|---|---|
| `String` | `String` (owned) / `&str` (borrowed) |
| `boolean` / `int` / `long` / `double` | `bool` / `i32` / `i64` / `f64` |
| `Integer` / `Long` / `Double` (nullable) | `Option<i32>` / `Option<i64>` / `Option<f64>` |
| `BigDecimal` | `rust_decimal::Decimal` |
| `byte[]` | `Vec<u8>` |
| `List<T>` | `Vec<T>` |
| `Set<T>` | `HashSet<T>` / `BTreeSet<T>` |
| `Map<K,V>` | `HashMap<K,V>` / `BTreeMap<K,V>` |
| `Optional<T>` | `Option<T>` |
| `UUID` | `uuid::Uuid` |
| `Instant` / `OffsetDateTime` / `LocalDate` | `jiff::Timestamp` / `jiff::Zoned` / `jiff::civil::Date` |
| `enum` | `enum` |
| interface with impls | `trait` + impls, or a closed `enum` if the impl set is closed |
| `Stream<T>` pipeline | iterator chain |

## 5. Java → Rust idiom map

| Java idiom | Rust idiom |
|---|---|
| checked/unchecked exception | `Result<T, E>` with `thiserror` error enums |
| constructor that throws | `fn new(...) -> Result<Self, E>` (standing exception, Section 2) |
| `AutoCloseable` / `close()` | `impl Drop` (standing exception, Section 2) |
| `null` | `Option<T>` |
| inheritance (`extends`) | composition + trait, or an enum variant |
| abstract class with fields | a shared struct embedded by concrete types |
| generics `<T extends X>` | `<T: XTrait>` |
| builder pattern | builder struct, or `#[derive(bon::Builder)]` / typed-builder |
| `equals` / `hashCode` | `#[derive(PartialEq, Eq, Hash)]` |
| `toString` | `impl Display` |
| Jackson `@JsonProperty` | serde `#[serde(rename = "...")]` |
| Spring `@RestController` | axum handler + router |
| Spring DI (`@Autowired`) | explicit constructor injection / axum `State` |
| jOOQ DSL | sea-query builder + sqlx execution |

## 6. openEHR spec → Rust mappings (literal transcription rules)

| Spec construct | Rust mapping |
|---|---|
| One RM class | One Rust struct or enum, named identically in case-converted form (e.g. `DV_TEXT` → `DvText`). Keep the openEHR name in a doc comment and in a serde rename so the canonical `_type` string round-trips. |
| Abstract RM class with attributes | A struct the concrete types embed (composition), plus a marker trait if behaviour is shared. |
| Closed subtype set (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>`) | A closed `enum`. |
| Constrained generic (`DV_INTERVAL<T: DV_ORDERED>`) | A Rust generic with a trait bound. |
| Covariant redefinition (e.g. `LOCATABLE_REF.id`, `DV_COUNT.magnitude`) | Encode the narrowed type directly on the concrete struct; document the override in a doc comment. |
| Multiple inheritance (`Ordered_Numeric`, `Iso8601_type`, `DV_DURATION`, `EXTERNAL_ENVIRONMENT_ACCESS`) | Compose fields from all parents; implement each parent's behaviour as a separate trait. |
| `PATHABLE.parent()` reverse pointer | `Weak<..>` or a path-index lookup. Never an owning back-reference. |
| Recursive containment (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`, `DV_MULTIMEDIA.thumbnail`) | `Box<..>` / `Vec<Box<..>>`. |
| Symbolic operators (`++`, `and then`, `∀`) | Named methods. |
| serde conventions | snake_case attribute names; `_type` discriminator; omit nulls; UIDs serialize as `{_type, value}`; validate output against `openehr_rm_1.1.0_all.json`. |

## 7. Do-not-translate list

| Do not | Instead |
|---|---|
| Port JVM-specific plumbing literally (classloaders, Spring context internals, PF4J internals) | Record a `// PORT NOTE:` at the point of removal and design the Rust equivalent in the relevant phase, or defer the whole subsystem to a Stage 2 ADR (e.g. the plugin system). |
| Port `archie` / JVM openEHR-SDK internals | Transcribe from the published specifications instead (Section 6). `archie` is not a dependency and its code is not in this repo. |
| Port Maven build tooling literally (`pom.xml`, `mvnw`, `.mvn/`) | Use Cargo. Maven files are also protected from edits by the `protect_java.sh` hook until their owning module is fully cut over. |

## 8. PORT STATUS trailer (mandatory, verbatim format)

Every ported or transcribed `.rs` file ends with this block:

```rust
// ─────────────────────────────────────────────
// PORT STATUS
//   source: <java file this replaces, e.g. crates/openehr-server/src/aql/AqlSqlLayer.java>
//   source_loc: <line count of the Java file>
//   confidence: high | medium | low
//   todos: <count of TODO(port) in this file>
//   note: <one line for Phase B triage>
// ─────────────────────────────────────────────
```

For files transcribed from an openEHR specification rather than ported from
Java, set `source` to the specification document and section instead of a
Java path (e.g. `source: RM 1.1.0 rm.data_types.quantity, DV_QUANTITY`), and
set `source_loc` to `n/a`.
