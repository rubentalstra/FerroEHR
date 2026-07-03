//! `ITEM_TABLE` — logical relational-database-style table data structure.
//!
//! openEHR class: `ITEM_TABLE`, package `rm.data_structures.item_structure`.
//!
//! Logical relational database style table data structure, in which
//! columns are named and ordered with respect to each other. Implemented
//! using Cluster-per-row encoding. Each row Cluster must have an identical
//! number of Elements, each of which in turn must have identical names and
//! value types in the corresponding positions in each row.
//!
//! Some columns may be designated 'key' columns, containing key data for
//! each row, in the manner of relational tables. This allows row-naming,
//! where each row represents a body site, a blood antigen etc. All values
//! in a column have the same data type.
//!
//! Used for representing any data which is logically a table of values,
//! such as blood pressure, most protocols, many blood tests etc.
//!
//! Misuse: not to be used for time-based data, which should be represented
//! with the temporal class `HISTORY`. The table may be empty.

use super::data_structure::DataStructureBehaviour;
use super::item_structure::{ItemStructureApi, ItemStructureData};
use crate::data_structures::representation::cluster::Cluster;
use crate::data_structures::representation::element::Element;
use crate::data_structures::representation::item::{Item, ItemApi, ItemData};
// PORT NOTE: `DV_TEXT` belongs to `rm.data_types.text` (now landed); its
// `DvTextApi::value` accessor is used to compare row/column names by string.
use crate::data_types::text::dv_text::{DvText, DvTextApi};
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `ITEM_TABLE` class.
///
/// Embeds the shared `ITEM_STRUCTURE` state (per ADR-001 §3) plus its own
/// `rows` attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemTable {
    /// Canonical `_type` discriminator (`"ITEM_TABLE"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ITEM_STRUCTURE` (and transitively `DATA_STRUCTURE`,
    /// `LOCATABLE`) state.
    #[serde(flatten)]
    pub item_structure: ItemStructureData,

    /// `rows`: physical representation of the table as a list of
    /// `CLUSTER`s, each containing the data of one row of the table.
    ///
    /// Cardinality `0..1` per the spec table; modelled as
    /// `Option<Vec<Cluster>>` for the same "attribute absent vs. empty
    /// list" reasoning as `ItemList.items` (see `item_list.rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Cluster>>,
}

impl TypeName for ItemTable {
    const NAME: &'static str = TYPE_NAME;
}

impl ItemStructureApi for ItemTable {
    fn item_structure_data(&self) -> &ItemStructureData {
        &self.item_structure
    }
}

impl ItemTable {
    /// `row_count`: number of rows in the table.
    pub fn row_count(&self) -> i32 {
        self.rows.as_ref().map_or(0, |v| v.len() as i32)
    }

    /// `column_count`: return number of columns in the table.
    ///
    /// The element-count of any one row (the class description states every
    /// row CLUSTER must have an identical number of `ELEMENT`s). PORT NOTE:
    /// the spec gives no explicit answer for the zero-rows case; the
    /// coherent reading is 0 columns (no row to draw a column structure
    /// from).
    pub fn column_count(&self) -> i32 {
        self.rows
            .as_ref()
            .and_then(|rows| rows.first())
            .map_or(0, |row| row.items.len() as i32)
    }

    /// `row_names`: return set of row names.
    ///
    /// The inherited `LOCATABLE.name` of each row `CLUSTER` (per the class
    /// description and the package's ISO 13606 encoding rules: "The names of
    /// the containing CLUSTER of each row..."). `None` when the `rows`
    /// attribute itself is absent, matching the `0..1` return cardinality.
    pub fn row_names(&self) -> Option<Vec<DvText>> {
        self.rows
            .as_ref()
            .map(|rows| rows.iter().map(|row| row.name().clone()).collect())
    }

    /// `column_names`: return set of column names.
    ///
    /// The `LOCATABLE.name` of each `ITEM` in a row (per the package's ISO
    /// 13606 encoding rules: "The names of the ELEMENT in a row are the
    /// column names"). Derived from the first row, since every row has an
    /// identical column structure. `None` when there are no rows.
    pub fn column_names(&self) -> Option<Vec<DvText>> {
        self.rows
            .as_ref()
            .and_then(|rows| rows.first())
            .map(|row| row.items.iter().map(|item| item.name().clone()).collect())
    }

    /// `ith_row`: return i-th row.
    ///
    /// PORT NOTE: `i` is 1-based (Eiffel-derived openEHR convention, as in
    /// `ItemList.ith_item`). The spec signature returns `CLUSTER` (`1..1`)
    /// with no out-of-range path; widened to `Option<Cluster>` per the
    /// precondition-widening precedent used across this port.
    pub fn ith_row(&self, i: i32) -> Option<Cluster> {
        let idx = usize::try_from(i - 1).ok()?;
        self.rows.as_ref().and_then(|rows| rows.get(idx).cloned())
    }

    /// `has_row_with_name`: return `True` if there is a row with name =
    /// `a_key`.
    ///
    /// PORT NOTE (spec defect 1/6): the published spec's own function
    /// description reads "Return `True` if there is a **column** with name =
    /// `a_key`" for `has_row_with_name`, textually identical to the
    /// description immediately below it for `has_column_with_name`. Given the
    /// function is literally named `has_row_with_name`, this is transcribed
    /// with name-implied semantics (checks **rows**, not columns) — flagged
    /// as an apparent copy-paste artifact in the published spec table rather
    /// than followed literally, per the precedent of transcribing the
    /// name-implied reading and documenting the discrepancy loudly.
    pub fn has_row_with_name(&self, a_key: &str) -> bool {
        self.rows
            .as_ref()
            .is_some_and(|rows| rows.iter().any(|row| row.name().value() == a_key))
    }

    /// `has_column_with_name`: return `True` if there is a column with
    /// name = `a_key`.
    pub fn has_column_with_name(&self, a_key: &str) -> bool {
        self.column_names()
            .is_some_and(|names| names.iter().any(|n| n.value() == a_key))
    }

    /// `named_row`: return row with name = `a_key`.
    ///
    /// PORT NOTE: spec signature returns `CLUSTER` (`1..1`) with no declared
    /// not-found path; widened to `Option<Cluster>` per the same
    /// precondition-widening precedent as `ith_row`.
    pub fn named_row(&self, a_key: &str) -> Option<Cluster> {
        self.rows
            .as_ref()
            .and_then(|rows| rows.iter().find(|row| row.name().value() == a_key).cloned())
    }

    /// `has_row_with_key`: return `True` if there is a row with key
    /// `keys`.
    pub fn has_row_with_key(&self, keys: Option<&[String]>) -> bool {
        // TODO(port) (spec defect 2/6): "key" columns are a
        // class-description-level concept ("Some columns may be designated
        // 'key' columns") without any dedicated attribute recording which
        // columns are keys anywhere in this class's own Attributes table —
        // the mechanism for identifying key columns is unspecified. This
        // function is therefore genuinely not implementable from the spec;
        // left as `todo!()`, per the invoking task's instruction to NOT
        // invent a key-column mechanism the spec omits. Likely resolved once
        // archetype/template binding (P11) supplies the key-column metadata.
        let _ = keys;
        todo!(
            "has_row_with_key(keys): key-column identification mechanism not specified by this class's own spec table (P11 archetype binding)"
        )
    }

    /// `row_with_key`: return rows with particular keys.
    pub fn row_with_key(&self, keys: Option<&[String]>) -> Cluster {
        // TODO(port) (spec defects 3/6 and 4/6): same key-column mechanism
        // gap as `has_row_with_key` (defect 3). Additionally (defect 4) the
        // spec's own description disagrees with its signature — "Return rows
        // [plural] with particular keys" but the declared return type is a
        // single `CLUSTER`, not a `List<CLUSTER>`. Transcribed literally from
        // the signature (single `Cluster`); both defects flagged rather than
        // silently resolved. Not implementable until the key-column mechanism
        // lands (P11).
        let _ = keys;
        todo!(
            "row_with_key(keys): key-column identification mechanism not specified; spec description/signature also disagree on singular vs. plural return (P11 archetype binding)"
        )
    }

    /// `element_at_cell_ij`: return cell at a particular location.
    ///
    /// PORT NOTE: `i` (row) and `j` (column) are 1-based (Eiffel-derived
    /// openEHR convention). The spec signature returns `ELEMENT` (`1..1`)
    /// with no out-of-range path; widened to `Option<Element>` per the
    /// precondition-widening precedent. A cell that is not an `ELEMENT` (a
    /// `Valid_structure` violation — see `invariant_valid_structure`) also
    /// yields `None`.
    pub fn element_at_cell_ij(&self, i: i32, j: i32) -> Option<Element> {
        let row_idx = usize::try_from(i - 1).ok()?;
        let col_idx = usize::try_from(j - 1).ok()?;
        let row = self.rows.as_ref()?.get(row_idx)?;
        match row.items.get(col_idx)? {
            Item::Element(e) => Some(e.clone()),
            Item::Cluster(_) => None,
        }
    }

    /// `as_hierarchy` (redefined): generate a CEN EN13606-compatible
    /// hierarchy consisting of a single `CLUSTER` containing the `CLUSTER`s
    /// (one per row) of this table.
    ///
    /// Covariant redefinition (ADR-001 §6): narrows
    /// `DATA_STRUCTURE.as_hierarchy(): ITEM` to
    /// `ITEM_TABLE.as_hierarchy(): CLUSTER`. See `data_structure.rs` for the
    /// shape rationale.
    ///
    /// PORT NOTE (spec defect 5/6): the function's own description says the
    /// generated wrapper `CLUSTER` contains `CLUSTER`s "representing the
    /// **columns** of this table" — but `ITEM_TABLE`'s physical
    /// representation (`rows: List<CLUSTER>`) is **row-major** (Cluster-per-
    /// row encoding, per the class description and the `item_structure`
    /// package's own "ISO 13606 Encoding Rules > ITEM_TABLE" section: "Each
    /// row is encoded as a Cluster..."). A column-major `as_hierarchy()`
    /// would require transposing the table, which is described nowhere in
    /// this package. Implemented per the **coherent** reading — the
    /// package's own row-major encoding rules — producing a wrapper `CLUSTER`
    /// (carrying this table's own `LOCATABLE` identity, as in
    /// `ItemList::as_hierarchy`) whose `items` are the row `CLUSTER`s. The
    /// "columns" wording in the function description is flagged as a probable
    /// copy-paste/wording defect, not followed.
    pub fn as_hierarchy(&self) -> Cluster {
        Cluster {
            type_tag: TypeTag::new(),
            item: ItemData {
                locatable: self.item_structure.data_structure.locatable.clone(),
            },
            items: self.rows.as_ref().map_or_else(Vec::new, |rows| {
                rows.iter().cloned().map(Item::Cluster).collect()
            }),
        }
    }

    /// `Valid_structure`: `rows.for_all (items.for_all (instance_of
    /// ("ELEMENT")))`.
    ///
    /// PORT NOTE (spec defect 6/6): this is a **genuine runtime invariant**,
    /// unlike `ITEM_LIST`'s statically-guaranteed equivalent. A row is a
    /// `CLUSTER` whose `items` are `Vec<Item>` (per the literal
    /// `CLUSTER.items: List<ITEM>` spec shape), so the Rust type system does
    /// **not** prevent a row from holding a nested `Item::Cluster`. The
    /// invariant is therefore checked at runtime here (working `invariant_*`
    /// method per ADR-003 §8) rather than narrowed away by changing
    /// `Cluster`'s own field type (which would deviate from the spec).
    pub fn invariant_valid_structure(&self) -> bool {
        self.rows.as_ref().is_none_or(|rows| {
            rows.iter().all(|row| {
                row.items
                    .iter()
                    .all(|item| matches!(item, Item::Element(_)))
            })
        })
    }
}

impl DataStructureBehaviour for ItemTable {
    fn as_hierarchy(&self) -> Item {
        Item::Cluster(self.as_hierarchy())
    }
}

pub const TYPE_NAME: &str = "ITEM_TABLE";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::data_structures::item_structure::data_structure::DataStructureData;

    fn locatable(name: &str, node_id: &str) -> LocatableData {
        serde_json::from_value(serde_json::json!({
            "name": { "_type": "DV_TEXT", "value": name },
            "archetype_node_id": node_id,
        }))
        .unwrap()
    }

    fn element(name: &str) -> Element {
        Element {
            type_tag: TypeTag::new(),
            item: ItemData {
                locatable: locatable(name, "at0010"),
            },
            null_flavour: None,
            value: None,
            null_reason: None,
        }
    }

    /// A row `CLUSTER` named `row_name`, with two columns "left" and "right".
    fn row(row_name: &str) -> Cluster {
        Cluster {
            type_tag: TypeTag::new(),
            item: ItemData {
                locatable: locatable(row_name, "at0002"),
            },
            items: vec![
                Item::Element(element("left")),
                Item::Element(element("right")),
            ],
        }
    }

    /// Two-row visual-acuity-style table.
    fn table() -> ItemTable {
        ItemTable {
            type_tag: TypeTag::new(),
            item_structure: ItemStructureData {
                data_structure: DataStructureData {
                    locatable: locatable("visual acuity", "at0001"),
                },
            },
            rows: Some(vec![row("1"), row("2")]),
        }
    }

    /// Spec `row_count` / `column_count`: rows and per-row element count.
    #[test]
    fn row_and_column_counts() {
        let t = table();
        assert_eq!(t.row_count(), 2);
        assert_eq!(t.column_count(), 2);

        let empty = ItemTable {
            type_tag: TypeTag::new(),
            item_structure: ItemStructureData {
                data_structure: DataStructureData {
                    locatable: locatable("empty", "at0001"),
                },
            },
            rows: None,
        };
        assert_eq!(empty.row_count(), 0);
        assert_eq!(empty.column_count(), 0);
    }

    /// Spec `row_names`/`column_names`: names of row CLUSTERs / of the ELEMENTs
    /// in a row (per the ISO 13606 encoding rules).
    #[test]
    fn row_and_column_names() {
        let t = table();
        let row_names = t.row_names().unwrap();
        let rows: Vec<&str> = row_names.iter().map(DvTextApi::value).collect();
        assert_eq!(rows, ["1", "2"]);
        let column_names = t.column_names().unwrap();
        let cols: Vec<&str> = column_names.iter().map(DvTextApi::value).collect();
        assert_eq!(cols, ["left", "right"]);
    }

    /// Spec `ith_row`: 1-based, widened to Option.
    #[test]
    fn ith_row_is_one_based() {
        let t = table();
        assert_eq!(t.ith_row(1).unwrap().name().value(), "1");
        assert_eq!(t.ith_row(2).unwrap().name().value(), "2");
        assert!(t.ith_row(0).is_none());
        assert!(t.ith_row(3).is_none());
    }

    /// Spec `has_row_with_name` (defect 1/6: description copy-paste — we
    /// implement the name-implied *row* semantics) / `has_column_with_name`.
    #[test]
    fn has_row_and_column_with_name() {
        let t = table();
        assert!(t.has_row_with_name("1"));
        assert!(!t.has_row_with_name("99"));
        assert!(t.has_column_with_name("left"));
        assert!(!t.has_column_with_name("center"));
    }

    /// Spec `named_row`: return row with matching name, widened to Option.
    #[test]
    fn named_row_finds_by_name() {
        let t = table();
        assert_eq!(t.named_row("2").unwrap().name().value(), "2");
        assert!(t.named_row("3").is_none());
    }

    /// Spec `element_at_cell_ij`: 1-based (row i, column j).
    #[test]
    fn element_at_cell_is_one_based() {
        let t = table();
        assert_eq!(
            t.element_at_cell_ij(1, 1)
                .unwrap()
                .item
                .locatable
                .name
                .value(),
            "left"
        );
        assert_eq!(
            t.element_at_cell_ij(2, 2)
                .unwrap()
                .item
                .locatable
                .name
                .value(),
            "right"
        );
        assert!(t.element_at_cell_ij(0, 1).is_none());
        assert!(t.element_at_cell_ij(1, 3).is_none());
        assert!(t.element_at_cell_ij(3, 1).is_none());
    }

    /// Spec `as_hierarchy` (defect 5/6): implemented row-major per the
    /// package's own ISO 13606 encoding rules; the wrapper CLUSTER carries
    /// the table's own identity and contains the row CLUSTERs.
    #[test]
    fn as_hierarchy_is_row_major() {
        let t = table();
        let cluster = t.as_hierarchy();
        assert_eq!(cluster.name().value(), "visual acuity");
        assert_eq!(cluster.items.len(), 2);
        assert!(
            cluster
                .items
                .iter()
                .all(|it| matches!(it, Item::Cluster(_)))
        );
    }

    /// Spec `Valid_structure` (defect 6/6): a genuine runtime check. A
    /// well-formed table holds; a row containing a nested CLUSTER violates it.
    #[test]
    fn valid_structure_is_a_runtime_check() {
        assert!(table().invariant_valid_structure());

        let mut bad = table();
        // Inject a nested CLUSTER into a row's items — allowed by the type
        // system (Cluster.items: Vec<Item>), rejected by the invariant.
        if let Some(rows) = bad.rows.as_mut() {
            rows[0].items.push(Item::Cluster(row("nested")));
        }
        assert!(!bad.invariant_valid_structure());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_TABLE — docs/research/spec-cache/RM-1.1.0/uml_classes/item_table.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_table.adoc §ITEM_TABLE Class
//   confidence: low
//   todos: 2
//   note: row_count/column_count/row_names/column_names/ith_row/has_row_with_name/has_column_with_name/named_row/element_at_cell_ij/as_hierarchy implemented (common landed) with spec-derived tests; index functions 1-based and widened to Option. SIX published-spec defects preserved and flagged inline: (1) has_row_with_name description copy-pasted from has_column_with_name (implemented name-implied as rows); (2) has_row_with_key key-column mechanism unspecified → todo!() (P11); (3) row_with_key same key-column gap → todo!() (P11); (4) row_with_key singular/plural return mismatch; (5) as_hierarchy column-major description vs the package's row-major encoding (implemented row-major); (6) Valid_structure is a genuine runtime invariant (rows' items are Vec<Item>) now enforced by a working method, distinct from ITEM_LIST's static one. Remaining 2 todo!()s are the key-column pair. P4/ADR-002: self-tag added.
// ─────────────────────────────────────────────
