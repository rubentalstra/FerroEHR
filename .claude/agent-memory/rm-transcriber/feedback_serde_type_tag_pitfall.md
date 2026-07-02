---
name: feedback-serde-type-tag-pitfall
description: serde(rename) on a struct/container does NOT inject a literal _type discriminator key into JSON output — verified experimentally against serde_json. The _type tag only comes from #[serde(tag = "_type")] on the containing enum (per-variant #[serde(rename = "...")] on the variant, not the inner struct).
metadata:
  type: feedback
---

`#[serde(rename = "HIER_OBJECT_ID")]` (or any struct-level rename) does
**not** add a `"_type": "HIER_OBJECT_ID"` key to the JSON output. It only
affects how the Rust type's own name surfaces to certain
`Deserializer::deserialize_struct` hints — it has zero effect on the wire
bytes serde_json actually produces.

**Why:** discovered while transcribing P4 canonical-JSON serde for
`openehr-rm`'s `data_structures`/`common` and `openehr-base::identification`
(2026-07-02). I initially wrote `#[derive(Serialize, Deserialize)]
#[serde(rename = "HIER_OBJECT_ID")]` on ~12 structs believing this produced
the canonical `{"_type": "...", "value": "..."}` UID shape
`.claude/rules/serialization.md` requires. Built an isolated probe crate
(outside the real workspace, since `openehr-base`/`openehr-rm`'s own
`lib.rs` files are unwired per ADR-001 §9 and so never actually compile
the edited files) and confirmed experimentally: serializing produced
`{"value": "..."}` with **no `_type` key at all**.

**The actual mechanism, verified working:**
- For a **closed enum** dispatching over concrete variants (`PartyProxy`,
  `DataValue`, `Item`, `ItemStructure`, `Version<T>`, etc.): put
  `#[serde(tag = "_type")]` on the **enum**, and `#[serde(rename =
  "PARTY_SELF")]` on each **variant** (not on the inner struct type). This
  produces exactly `{"_type": "PARTY_SELF", ...flattened fields...}` —
  confirmed in an isolated probe.
- For a **standalone (non-enum) struct** that needs a fixed literal
  discriminator field regardless of enum context (e.g. every `UID_BASED_ID`
  leaf per the unconditional "UIDs serialize as `{_type, value}`" rule):
  there is **no attribute-only shortcut**. Two verified-working options,
  neither shipped yet (this is a real open design decision, not something
  to invent unilaterally mid-file):
  1. A hand-written `Serialize`/`Deserialize` impl per struct using
     `serializer.serialize_struct(...)` +
     `state.serialize_field("_type", "LITERAL")`. O(n) boilerplate but
     zero new infrastructure.
  2. A reusable `TypeTag<T: TypeTagName>(PhantomData<T>)` marker-field
     type, hand-written once, with `Serialize`/`Deserialize`/`Debug`/
     `Clone`/`Copy`/`PartialEq`/`Eq` all **manually implemented** (not
     derived — deriving on a `PhantomData<T>`-backed struct adds a
     spurious `T: Trait` bound that the marker type doesn't actually need,
     since `PhantomData` never stores a `T` value). Each concrete class
     then needs a one-line marker unit struct + trait impl
     (`struct HierObjectIdMarker; impl TypeTagName for HierObjectIdMarker
     { const NAME: &str = "HIER_OBJECT_ID"; }`) plus one field:
     `#[serde(rename = "_type", default)] _type: TypeTag<HierObjectIdMarker>`.

**How to apply:** Before annotating any future struct with
`#[serde(rename = "SOME_TYPE_NAME")]` expecting it to produce a `_type`
key, stop — it won't. Decide (or ask the `openehr-serde` P4 owner /
raise an ADR) which of the two struct-level mechanisms above the project
wants **before** rolling it out across many files, since it's real new
shared infrastructure, not a one-line fix. For enums, `#[serde(tag =
"_type")]` + per-variant `#[serde(rename)]` just works — use that whenever
the closed-enum target already exists.

See also [[project-unwired-lib-rs-masks-bugs]].
