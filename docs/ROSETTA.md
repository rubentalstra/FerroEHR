# ROSETTA — Living Mapping Registry

This is the living Java↔Rust and openEHR-spec↔Rust mapping registry for the
port. Unlike `docs/PORTING.md` (the general rule set, fixed once written),
this file accumulates one row per concrete symbol as it gets ported or
transcribed, so a later session or subagent can look up "where did `X` go"
without re-deriving it.

Maintained by the `rosetta-curator` agent and the `rosetta-mapping` skill.
Append rows as files are ported or transcribed; do not delete a row once a
symbol has landed, even if the symbol is later renamed — update the row in
place instead. Kind values are free text but should stay consistent within a
table (e.g. `struct`, `enum`, `trait`, `fn`, `module`).

## Java → Rust

| Java symbol | Rust path | kind | notes |
|---|---|---|---|

## openEHR spec → Rust

| Spec construct | Rust path | kind | notes |
|---|---|---|---|
| abstract class, no attributes (`Any`, `Ordered`, `Numeric`) | `openehr-foundation::primitive_types::*` | trait | ADR-001 §1; symbolic operators become named methods |
| multiple inheritance (`Ordered_Numeric`, `Iso8601_type`) | supertrait composition + field embedding | trait | ADR-001 §2; `trait OrderedNumeric: Ordered + Numeric` |
| abstract class with attributes | embedded struct + marker trait, `#[serde(flatten)]` | struct+trait | ADR-001 §3 |
| closed subtype set (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>`) | one `enum`, variant per concrete class | enum | ADR-001 §4 |
| constrained generic (`Interval<T: Ordered>`, `DV_INTERVAL<T: DV_ORDERED>`) | generic with trait bound | struct | ADR-001 §5 |
| covariant redefinition (`LOCATABLE_REF.id`, `DV_COUNT.magnitude`) | narrowed type on the concrete struct + doc note | field | ADR-001 §6; no generics gymnastics |
| primitives (`Boolean`, `Character`, `Octet`, `String`, `Integer`, `Integer64`, `Real`, `Double`, `Uri`) | std mappings behind spec-named aliases/newtypes | alias/newtype | ADR-001 §7; Octet=`u8`, Real/Double both `f64` (PORT NOTE) |
| spec class file layout | one class per snake_case file, dir per spec package, unwired until P17 | module | ADR-001 §9 |
| `Any` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::any::Any` | trait | Root marker trait; `is_equal`/`equal`/`not_equal`/`type_of` as named methods, `instance_of` left undefined by default (no reflection-by-name in Rust without a registry). |
| `Ordered` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::ordered::Ordered` | trait | `Any` supertrait; `less_than` abstract, `less_than_or_equal`/`greater_than`/`greater_than_or_equal` are default methods encoding the spec's `Post_result` postconditions. |
| `Numeric` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::numeric::Numeric` | trait | `Any` supertrait; same-type `&Self -> Self` shape only. Heterogeneous-result effectors (`Integer.divide -> Double`, etc.) live as inherent methods on the concrete type instead — the trait cannot express an open/self-widening result type. |
| `Ordered_Numeric` (BASE foundation_types.primitive_types, multiple inheritance) | `openehr_foundation::primitive_types::ordered_numeric::OrderedNumeric` | trait | Supertrait composition `Ordered + Numeric`, blanket-impl'd for any `T: Ordered + Numeric` — do not write an explicit `impl OrderedNumeric for ConcreteType {}`, it conflicts with the blanket impl (E0119). |
| `Boolean` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::boolean::Boolean` | struct (newtype over `bool`) | Logical connectives (`conjunction`, `semistrict_conjunction`, `disjunction`, etc.) as inherent methods; semi-strict variants take `impl FnOnce() -> Boolean` for short-circuit evaluation. |
| `Character` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::character::Character` | struct (newtype over `char`) | Chose Rust `char` (Unicode scalar) over `u8`, since the spec chapter states `String`/`Character` assume UTF-8/Unicode, distinct from the explicitly-8-bit `Octet`. |
| `Octet` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::octet::Octet` | struct (newtype over `u8`) | Named `Octet`, not "Byte" — settled hazard, do not relitigate. |
| `String` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::string::OpenEhrString` | struct (newtype over `std::string::String`) | Renamed to `OpenEhrString` to avoid colliding with `std::string::String`. This is distinct from an ordinary RM attribute of spec type `String`, which still maps directly to `std::string::String` per `docs/PORTING.md` §14.2 — `OpenEhrString` is specifically the foundation-types class with its own operations (`is_empty`, `is_integer`, `as_integer`, `append`, `contains`). |
| `Integer` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::integer::Integer` | struct (newtype over `i32`) | `divide`/`exponent` narrow to `Double`-involving signatures per the spec table; transcribed as inherent methods, with the `Numeric` trait's same-type `divide`/`exponent` stubbed `todo!()` on this type. |
| `Integer64` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::integer64::Integer64` | struct (newtype over `i64`) | Spec's `add`/`subtract`/`multiply`/`modulo` take a 32-bit `Integer` operand (asymmetric widening), transcribed as inherent methods; `Numeric` trait impl provides a same-type `Integer64+Integer64` specialization instead. |
| `Real` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::real::Real` | struct (newtype over `f64`) | **Directed deviation**: spec text literally describes `Real` as single-precision/32-bit; backed by `f64` (matching `Double`) per explicit instruction for the primitive_types transcription pass, not spec literalism. Revisit if float-precision parity is ever needed. |
| `Double` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::double::Double` | struct (newtype over `f64`) | Matches the spec's "double-precision"/64-bit description exactly; the only concrete `Numeric` effector in the cluster whose `divide`/`exponent` match the trait's same-type shape with no stub needed. |
| `Uri` (BASE foundation_types.primitive_types) | `openehr_foundation::primitive_types::uri::Uri` | struct (newtype over `OpenEhrString`) | Wraps `OpenEhrString` (not `std::string::String` directly) to reflect the spec's actual `Uri`-inherits-`String` relationship. RFC 3986 syntax invariant not yet enforced (`new_unchecked` only). |
