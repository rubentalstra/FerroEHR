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
use crate::data_structures::representation::item::Item;
// PORT NOTE: `DV_TEXT` belongs to `rm.data_types.text`, transcribed
// concurrently by a sibling agent; see `representation/element.rs` for the
// identical forward-reference rationale and assumed module path.
use crate::data_types::text::dv_text::DvText;
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
    pub fn column_count(&self) -> i32 {
        // TODO(port): column count is the element-count of any one row
        // (the class description states every row CLUSTER must have an
        // identical number of Elements); requires inspecting `rows[0].items`
        // once a row exists. Left as `todo!()` pending a decision on the
        // zero-rows case (0 columns, vs. undefined per the spec, which
        // gives no explicit answer for an empty table).
        todo!(
            "column_count(): needs a policy for the zero-rows case, not specified by the spec table"
        )
    }

    /// `row_names`: return set of row names.
    pub fn row_names(&self) -> Option<Vec<DvText>> {
        // TODO(port): row names are derived from each row CLUSTER's
        // inherited `LOCATABLE.name` (per the class description: "The
        // names of the containing CLUSTER of each row..."), which depends
        // on the not-yet-landed `common::archetyped::locatable` module —
        // see `representation/item.rs`.
        todo!("row_names(): needs LOCATABLE.name accessor on Cluster via common package")
    }

    /// `column_names`: return set of column names.
    pub fn column_names(&self) -> Option<Vec<DvText>> {
        // TODO(port): column names are the `LOCATABLE.name` of each
        // ELEMENT within a row CLUSTER (per the class description: "The
        // names of the ELEMENT in a row are the column names"), same
        // `common` package dependency as `row_names()`.
        todo!("column_names(): needs LOCATABLE.name accessor on Element via common package")
    }

    /// `ith_row`: return i-th row.
    pub fn ith_row(&self, i: i32) -> Cluster {
        // TODO(port): spec signature declares this returning `CLUSTER`
        // (not `Option<CLUSTER>`); out-of-range `i` has no declared
        // Void/error path in the table, same open question as
        // `ItemList.ith_item` (see `item_list.rs`).
        let _ = i;
        todo!("ith_row(i): out-of-range behaviour not specified by the spec table")
    }

    /// `has_row_with_name`: return `True` if there is a column with name =
    /// `a_key`.
    ///
    /// PORT NOTE: the spec's own function description reads "Return `True`
    /// if there is a **column** with name = `a_key`" for
    /// `has_row_with_name`, textually identical to the description
    /// immediately below it for `has_column_with_name`. Given the function
    /// is literally named `has_row_with_name`, this is transcribed with
    /// name-implied semantics (checks **rows**, not columns) — flagged as
    /// an apparent copy-paste artifact in the published spec table rather
    /// than followed literally, per the precedent established in the
    /// `time_types` transcription (`docs/ADRs/ADR-001-spec-transcription-shapes.md`
    /// pattern of transcribing the name-implied reading and documenting the
    /// discrepancy loudly).
    pub fn has_row_with_name(&self, a_key: &str) -> bool {
        // TODO(port): needs LOCATABLE.name accessor on Cluster; same
        // `common` package dependency as `row_names()`.
        let _ = a_key;
        todo!(
            "has_row_with_name(a_key): needs LOCATABLE.name accessor on Cluster via common package"
        )
    }

    /// `has_column_with_name`: return `True` if there is a column with
    /// name = `a_key`.
    pub fn has_column_with_name(&self, a_key: &str) -> bool {
        // TODO(port): needs LOCATABLE.name accessor on Element within a
        // row Cluster; same `common` package dependency as
        // `column_names()`.
        let _ = a_key;
        todo!(
            "has_column_with_name(a_key): needs LOCATABLE.name accessor on Element via common package"
        )
    }

    /// `named_row`: return row with name = `a_key`.
    pub fn named_row(&self, a_key: &str) -> Cluster {
        // TODO(port): needs LOCATABLE.name accessor on Cluster; spec
        // signature declares this returning `CLUSTER` with no declared
        // not-found path, same open question as `ith_row`.
        let _ = a_key;
        todo!("named_row(a_key): needs LOCATABLE.name accessor on Cluster via common package")
    }

    /// `has_row_with_key`: return `True` if there is a row with key
    /// `keys`.
    pub fn has_row_with_key(&self, keys: Option<&[String]>) -> bool {
        // TODO(port): "key" columns are a class-description-level concept
        // ("Some columns may be designated 'key' columns") without a
        // dedicated attribute recording which columns are keys anywhere in
        // this class's own Attributes table — the mechanism for
        // identifying key columns is not specified. Left as `todo!()`
        // pending that determination, likely resolved once archetype/
        // template binding (a later phase) supplies the key-column
        // metadata.
        let _ = keys;
        todo!(
            "has_row_with_key(keys): key-column identification mechanism not specified by this class's own spec table"
        )
    }

    /// `row_with_key`: return rows with particular keys.
    pub fn row_with_key(&self, keys: Option<&[String]>) -> Cluster {
        // TODO(port): same key-column mechanism gap as
        // `has_row_with_key`. Note the spec's own function name/return
        // type disagree with its own description ("Return rows [plural]
        // with particular keys" but returns a single `CLUSTER`, not a
        // `List<CLUSTER>`) — transcribed literally from the signature
        // (single `Cluster`), description mismatch flagged rather than
        // silently resolved.
        let _ = keys;
        todo!(
            "row_with_key(keys): key-column identification mechanism not specified; spec description/signature also disagree on singular vs. plural return"
        )
    }

    /// `element_at_cell_ij`: return cell at a particular location.
    pub fn element_at_cell_ij(&self, i: i32, j: i32) -> Element {
        // TODO(port): `i`/`j` addressing (row index, column index) into
        // `rows[i].items[j]`; out-of-range behaviour not specified, same
        // open question as `ith_row`/`ith_item`.
        let _ = (i, j);
        todo!("element_at_cell_ij(i, j): out-of-range behaviour not specified by the spec table")
    }

    /// `as_hierarchy` (redefined): generate a CEN EN13606-compatible
    /// hierarchy consisting of a single `CLUSTER` containing the
    /// `CLUSTER`s representing the columns of this table.
    ///
    /// Covariant redefinition (ADR-001 §6): narrows
    /// `DATA_STRUCTURE.as_hierarchy(): ITEM` to
    /// `ITEM_TABLE.as_hierarchy(): CLUSTER`. See `data_structure.rs` for
    /// the shape rationale.
    ///
    /// PORT NOTE: the function's own description says the generated
    /// wrapper `CLUSTER` contains `CLUSTER`s "representing the **columns**
    /// of this table" — but `ITEM_TABLE`'s physical representation
    /// (`rows: List<CLUSTER>`) is row-major (per-row Cluster-per-row
    /// encoding, per the class description and the `item_structure`
    /// package's own "ISO 13606 Encoding Rules > ITEM_TABLE" section:
    /// "Each row is encoded as a Cluster..."). A column-major
    /// `as_hierarchy()` would require transposing the table, which is not
    /// otherwise described anywhere in this package. This reads as a
    /// probable copy-paste/wording defect in the published table
    /// (mismatched with the package's own encoding-rules section), flagged
    /// here rather than silently resolved; left as `todo!()` pending
    /// clarification of whether row-major or column-major is actually
    /// intended.
    pub fn as_hierarchy(&self) -> Cluster {
        todo!(
            "as_hierarchy(): spec description says column-major, but the package's ISO 13606 encoding rules for ITEM_TABLE describe row-major Cluster-per-row; ambiguity not resolved"
        )
    }
}

impl DataStructureBehaviour for ItemTable {
    fn as_hierarchy(&self) -> Item {
        Item::Cluster(self.as_hierarchy())
    }
}

// TODO(port): invariant `Valid_structure`:
// `rows.for_all (items.for_all (instance_of ("ELEMENT")))` — trivially true
// by construction here, since `rows: Option<Vec<Cluster>>` and
// `Cluster.items: Vec<Item>` (not literally `Vec<Element>`) — note the
// spec's own invariant text implicitly assumes each row Cluster's `items`
// are all ELEMENTs (never nested CLUSTERs), which the Rust `Cluster` type
// does not statically enforce (a `Cluster.items` can hold `Item::Cluster`
// variants too). This is a genuine, not-yet-enforced runtime invariant
// (unlike `ItemList`'s equivalent, which is statically guaranteed) —
// recorded here as a `Validate`-framework TODO rather than silently
// dropped or narrowed away by changing `Cluster`'s own field type (which
// would deviate from the literal `CLUSTER.items: List<ITEM>` spec shape).

pub const TYPE_NAME: &str = "ITEM_TABLE";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_TABLE — docs/research/spec-cache/RM-1.1.0/uml_classes/item_table.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_table.adoc §ITEM_TABLE Class
//   confidence: low
//   todos: 11
//   note: the row/column function battery is stubbed per signature per the invoking task's instruction; three genuine published-spec ambiguities flagged inline (has_row_with_name's description copy-pasted from has_column_with_name; row_with_key singular/plural mismatch; as_hierarchy column-major description contradicting the package's own row-major encoding rules) plus a real (not statically enforced) Valid_structure invariant gap distinct from ITEM_LIST's. P4/ADR-002: self-tag (TypeName + first-field TypeTag) added.
// ─────────────────────────────────────────────
