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
| `BASIC_DEFINITIONS` (BASE base_types.definitions) | `openehr_base::definitions::basic_definitions::BasicDefinitions` | struct (zero-field, assoc consts) | Constant-holder class pattern; CR/LF decoded from Eiffel octal escapes. |
| `OPENEHR_DEFINITIONS` (BASE base_types.definitions) | `openehr_base::definitions::openehr_definitions::OpenehrDefinitions` | struct (zero-field, assoc consts) | Inherits BASIC_DEFINITIONS in spec; documentation-only in Rust (no parent state). |
| `VALIDITY_KIND` (BASE base_types.definitions, enumeration) | `openehr_base::definitions::validity_kind::ValidityKind` | enum | 3 variants; `symbol()` carries the spec's lower-case name; serde derive deferred until the dep lands. |
| `VERSION_STATUS` (BASE base_types.definitions, enumeration) | `openehr_base::definitions::version_status::VersionStatus` | enum | 5 variants; `symbol()` carries the spec's snake_case name; serde derive deferred. |
| constant-holder class (attributes are only `Constants`) | zero-field struct + associated consts | struct | General pattern beyond BASIC_DEFINITIONS; no `Default`, no fields. |
| `Env` / `Locale` / `Math` / `Quantity_converter` / `Statistical_evaluator` (BASE base_types.builtins) | `openehr_base::builtins::{env,locale,math,quantity_converter,statistical_evaluator}` | trait | No-attribute interface classes → traits (ADR-001 §1); no concrete impl — spec names no service accessor. |
| forward-reference to a not-yet-transcribed sibling type | `use` of the type's eventual module path + `// TODO(port):` | pattern | Primitives narrow to the std type with a TODO; structural types get a real forward-reference `use`. |
| `Interval<T>` (BASE foundation_types.interval) | `openehr_foundation::interval::interval::Interval<T>` | struct | Abstract-with-attributes; embedded by value in PointInterval/ProperInterval, never constructed bare; `T: Ordered` bound per ADR-001 §5. |
| `Point_interval<T>` | `openehr_foundation::interval::point_interval::PointInterval<T>` | struct | Embeds `Interval<T>`; sole constructor `new(value: T)` enforces `Inv_point` structurally. |
| `Proper_interval<T>` | `openehr_foundation::interval::proper_interval::ProperInterval<T>` | struct | Embeds `Interval<T>`; `Inv_not_point` deferred to a fallible Validate constructor (TODO). |
| `Multiplicity_interval` | `openehr_foundation::interval::multiplicity_interval::MultiplicityInterval` | struct | Embeds `ProperInterval<Integer>` per the literal spec `Inherit` row; the prose "Interval<Integer>" reading is documented as equivalent. |
| `Multiplicity_range_marker` / `Multiplicity_unbounded_marker` | consts on `MultiplicityInterval` | const | Spec "Constants" table section → `pub const` on the impl block (`".."` / `'*'`). |
| `Cardinality` | `openehr_foundation::interval::cardinality::Cardinality` | struct | Spec table has no `Inherit` row; implements `Any` by crate-wide convention (flagged inference). |
| `AUTHORED_RESOURCE` (BASE resource, abstract) | `openehr_base::resource::authored_resource::AuthoredResource` + `AuthoredResourceBehaviour` | struct+trait | ADR-001 §3; `revision_history` modelled despite the published Attributes table omitting the row (present in prose/invariants — flagged). |
| `RESOURCE_DESCRIPTION` (BASE resource) | `openehr_base::resource::resource_description::ResourceDescription` | struct | `parent_resource` back-reference modelled as `Weak<AuthoredResource>`, never owning. |
| `RESOURCE_DESCRIPTION_ITEM` (BASE resource) | `openehr_base::resource::resource_description_item::ResourceDescriptionItem` | struct | Spec attribute `use` → field `use_` with `#[serde(rename = "use")]` (reserved keyword). |
| `TRANSLATION_DETAILS` (BASE resource) | `openehr_base::resource::translation_details::TranslationDetails` | struct | Plain struct, no ancestors. |
| `RESOURCE_ANNOTATIONS` (BASE resource) | `openehr_base::resource::resource_annotations::ResourceAnnotations` | struct | Triple-nested Hash → `HashMap<String, HashMap<String, HashMap<String, String>>>`; class table exists but the chapter omits its include (flagged). |
| cross-package placeholder pattern | `type TerminologyCode = String;` + TODO(port) per consuming file | alias | For any class referenced before its own transcription phase (e.g. REVISION_HISTORY is P3). |
| `Container<T>` (BASE foundation_types.structures) | `openehr_foundation::structure_types::container::Container<T>` | trait | `Any` supertrait; `there_exists`/`for_all`/`matching`/`select` default `todo!()` — spec gives no iteration primitive. |
| `List<T>` (BASE foundation_types.structures) | `openehr_foundation::structure_types::list::List<T>` | struct (newtype over `Vec<T>`) | `first`/`last` widened to `Option<&T>` (spec's own "or Void"). |
| `Set<T>` (BASE foundation_types.structures) | `openehr_foundation::structure_types::set::Set<T>` | struct (newtype over `HashSet<T>`) | `T: Eq + Hash` structural bound. |
| `Array<T>` (BASE foundation_types.structures) | `openehr_foundation::structure_types::array::Array<T>` | struct (newtype over `Vec<T>`) | Distinct newtype from List — separate classes/function sets. |
| `Hash<K,V>` (BASE foundation_types.structures) | `openehr_foundation::structure_types::hash::OpenEhrHash<K,V>` | struct (newtype over `HashMap<K,V>`) | Renamed to dodge `std::hash::Hash`, mirrors OpenEhrString; `K: Ordered + Eq + Hash`. |
| `Terminology_code` (BASE foundation_types.terminology) | `openehr_foundation::terminology_types::terminology_code::TerminologyCode` | struct | Leaf; `terminology_version`/`uri` optional. |
| `Terminology_term` (BASE foundation_types.terminology) | `openehr_foundation::terminology_types::terminology_term::TerminologyTerm` | struct | Leaf; concept + text. |
| `TUPLE`/`TUPLE1<A>`/`TUPLE2<A,B>` (BASE foundation_types.functional) | `functional::tuple::Tuple` trait / `Tuple1<A> = (A,)` / `Tuple2<A,B> = (A,B)` | trait + alias | Native tuples are the faithful shape; Tuple impl'd on the underlying tuple type. |
| `ROUTINE<ARGS>`/`FUNCTION<ARGS,RESULT>`/`PROCEDURE<ARGS>` (BASE foundation_types.functional) | `functional::routine::Routine<Args: Tuple>` trait / `Function = dyn Fn(Args)->Result` / `Procedure = dyn Fn(Args)` | trait + alias | ROUTINE description text contradicts its own signature (flagged, confidence low) — transcribed from signature. |
| `Bag<T>` | (not transcribed) | — | BASE 1.2.0 declares no Bag class; phase task wording was loose. Do not invent. |
| `UID` (abstract, BASE identification) | `openehr_base::identification::uid::{UidData, Uid, UidApi}` | struct+enum+trait | Abstract-with-attributes-used-polymorphically: Data embeds, enum closes the subtype set, Api trait shares accessors. |
| `ISO_OID` / `UUID` / `INTERNET_ID` | `identification::{iso_oid::IsoOid, uuid::Uuid, internet_id::InternetId}` | struct | Pure UID subtypes; `Uuid` deliberately not backed by the uuid crate this pass. |
| `OBJECT_ID` (abstract) | `identification::object_id::{ObjectIdData, ObjectId, ObjectIdApi}` | struct+enum+trait | `ObjectId` enum nests `UidBasedId` (not flattened) so covariant narrowing stays type-direct. |
| `UID_BASED_ID` (abstract) | `identification::uid_based_id::{UidBasedIdData, UidBasedId, UidBasedIdApi}` | struct+enum+trait | root/extension/has_extension as default trait methods over `value`. |
| `HIER_OBJECT_ID` / `OBJECT_VERSION_ID` / `VERSION_TREE_ID` | `identification::{hier_object_id, object_version_id, version_tree_id}` | struct | ObjectVersionId's object_id()/creating_system_id() deferred (UID format-sniffing parser todo). |
| `ARCHETYPE_ID` / `TEMPLATE_ID` / `TERMINOLOGY_ID` / `GENERIC_ID` | `identification::{archetype_id, template_id, terminology_id, generic_id}` | struct | ArchetypeId multi-axis via EBNF string-splitting; TemplateId lexical form spec-undetermined. |
| `OBJECT_REF` / `PARTY_REF` / `LOCATABLE_REF` | `identification::{object_ref, party_ref, locatable_ref}` | struct | `type` → `r#type`; PartyRef Type_validity as VALID_TYPES const; LocatableRef is the ADR-001 §6 worked example (`id: UidBasedId`). |
| `_type` discriminator, pre-P4 | `pub const TYPE_NAME: &str` on each concrete class | const | serde not yet a dep of base/foundation; replace with `#[serde(rename)]` at P4. |
| `Temporal` (BASE foundation_types.time) | `openehr_foundation::time::temporal::Temporal` | trait | `Ordered` supertrait, no members; NOT blanket-implemented (contrast OrderedNumeric) — names a semantic category, concrete types write an explicit empty impl. |
| `Time_Definitions` (BASE foundation_types.time) | `openehr_foundation::time::time_definitions::TimeDefinitions` | struct (zero-sized, assoc consts+fns) | Template for inheriting a constants-only class: not a supertrait; descendants call `TimeDefinitions::*` directly. |
| `Iso8601_type` (BASE foundation_types.time, MI) | `openehr_foundation::time::iso8601_type::{Iso8601Type, Iso8601TypeCore}` | trait + struct | ADR-001 §2 worked example: `Iso8601Type: Temporal`; `value: String` embedded via `Iso8601TypeCore` `core` field (future serde flatten at P4/P5). |
| `Iso8601_date`/`_time`/`_date_time`/`_duration`/`_timezone` | `openehr_foundation::time::iso8601_*::Iso8601{Date,Time,DateTime,Duration,Timezone}` | struct | Partial-precision ISO 8601 string wrappers, not resolved instants; parsing/arithmetic `todo!()` pending jiff-backed engine at P17 — do not add jiff to openehr-foundation before then. |
