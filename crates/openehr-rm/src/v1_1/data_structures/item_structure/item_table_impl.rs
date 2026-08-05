// @generated-from-template templates/openehr-rm/data_structures/item_structure/item_table_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM class invariants for `ITEM_TABLE`.
//!
//! Spec:
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_structures.item_table.adoc`.
//! - `Valid_structure` (`rows.for_all (items.for_all (instance_of
//!   ("ELEMENT")))`, §Invariants): every item in every row `CLUSTER` is an
//!   `ELEMENT` — no nested clusters.
//! - `Archetype_node_id_valid` — the inherited LOCATABLE invariant
//!   (`…org.openehr.rm.common.locatable.adoc` §Invariants).
//!
//! Two further checks come from that page's §Description ("Each row Cluster must
//! have an identical number of Elements, each of which in turn must have
//! identical names and value types in the corresponding positions in each row"),
//! which §Invariants does not restate. The rules are the page's; the two names
//! are not.
//!
//! NOTE: no openEHR spec names these two checks — `Valid_number_of_rows`
//! (equal item counts) and `Row_regularity` (matching names + value types per
//! column) are our own labels for §Description rules.

use crate::v1_1::data_structures::item_structure::item_table::ItemTable;
use crate::v1_1::data_structures::representation::item::Item;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for ItemTable {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        // Valid_structure: rows contain only ELEMENTs.
        if self.rows.iter().flatten().any(|row| {
            row.items
                .iter()
                .any(|item| !matches!(item, Item::Element(_)))
        }) {
            out.push(InvariantViolation::here(
                "Invariant Valid_structure failed on type ITEM_TABLE",
            ));
        }
        // Valid_number_of_rows (item_table.adoc §Description): every row has the
        // same number of items.
        if let Some(first) = self.rows.iter().flatten().next()
            && self
                .rows
                .iter()
                .flatten()
                .any(|row| row.items.len() != first.items.len())
        {
            out.push(InvariantViolation::here(
                "Invariant Valid_number_of_rows failed on type ITEM_TABLE",
            ));
        }
        // Row_regularity (item_table.adoc §Description): corresponding ELEMENTs
        // across rows must have identical names and value types — "each of which
        // in turn must have identical names and value types in the corresponding
        // positions in each row".
        if let Some(first) = self.rows.iter().flatten().next() {
            use crate::v1_1::data_types::text::dv_text::DvText;
            let name_of = |name: &DvText| match name {
                DvText::DvText(t) => t.value.clone(),
                DvText::DvCodedText(t) => t.value.clone(),
            };
            let signature =
                |row: &crate::v1_1::data_structures::representation::cluster::Cluster| {
                    row.items
                        .iter()
                        .map(|item| match item {
                            Item::Element(e) => (
                                name_of(&e.name),
                                e.value.as_ref().map(std::mem::discriminant),
                            ),
                            Item::Cluster(c) => (name_of(&c.name), None),
                        })
                        .collect::<Vec<_>>()
                };
            let first_sig = signature(first);
            if self
                .rows
                .iter()
                .flatten()
                .skip(1)
                .any(|row| signature(row) != first_sig)
            {
                out.push(InvariantViolation::here(
                    "Invariant Row_regularity failed on type ITEM_TABLE \
                     (corresponding row ELEMENTs must share names and value types)",
                ));
            }
        }
        crate::v1_1::validate::generated::archetype_node_id_core(
            "ITEM_TABLE",
            &self.archetype_node_id,
            out,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_1::data_structures::representation::cluster::Cluster;
    use crate::v1_1::data_structures::representation::element::Element;
    use crate::v1_1::data_types::basic::data_value::DataValue;
    use crate::v1_1::data_types::basic::dv_boolean::DvBoolean;
    use crate::v1_1::data_types::text::dv_text::{DvText, DvTextData};

    fn text(value: &str) -> DvText {
        DvText::DvText(DvTextData {
            value: value.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: openehr_base::containers::present_nonempty(Vec::new()),
            language: None,
            encoding: None,
        })
    }

    fn element(id: &str) -> Element {
        Element {
            name: text("e"),
            archetype_node_id: id.to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            null_flavour: None,
            value: Some(DataValue::DvBoolean(DvBoolean { value: true })),
            null_reason: None,
        }
    }

    fn row(items: Vec<Item>) -> Cluster {
        Cluster {
            name: text("row"),
            archetype_node_id: "at0002".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            items: openehr_base::containers::NonEmptyVec::new(items)
                .expect("a fixture container declared 1..* must have members"),
        }
    }

    fn table(rows: Vec<Cluster>) -> ItemTable {
        ItemTable {
            name: text("table"),
            archetype_node_id: "at0001".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            rows: openehr_base::containers::present(rows),
        }
    }

    #[test]
    fn valid_table() {
        let t = table(vec![
            row(vec![
                Item::Element(element("id5")),
                Item::Element(element("id6")),
            ]),
            row(vec![
                Item::Element(element("id7")),
                Item::Element(element("id8")),
            ]),
        ]);
        assert!(t.invariants().is_empty());
        assert!(table(vec![]).invariants().is_empty()); // empty table is valid
    }

    #[test]
    fn cluster_in_row_invalid() {
        let nested = row(vec![Item::Element(element("id7"))]);
        let t = table(vec![row(vec![
            Item::Element(element("id5")),
            Item::Cluster(nested),
        ])]);
        let v = t.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Valid_structure failed on type ITEM_TABLE"),
            "got {v:?}"
        );
    }

    #[test]
    fn ragged_rows_invalid() {
        let t = table(vec![
            row(vec![
                Item::Element(element("id5")),
                Item::Element(element("id6")),
            ]),
            row(vec![Item::Element(element("id7"))]),
        ]);
        let v = t.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Valid_number_of_rows failed on type ITEM_TABLE"),
            "got {v:?}"
        );
    }

    /// Row regularity (`item_table.adoc` §description): corresponding row
    /// ELEMENTs must share names and value types across all rows.
    #[test]
    fn row_regularity_names_and_value_types() {
        use crate::v1_1::data_types::quantity::dv_count::DvCount;
        let named = |name: &str, value: DataValue| Element {
            name: text(name),
            archetype_node_id: "at0001".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            null_flavour: None,
            value: Some(value),
            null_reason: None,
        };
        let row = |elems: Vec<Element>| Cluster {
            name: text("row"),
            archetype_node_id: "at0002".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            items: openehr_base::containers::NonEmptyVec::new(
                elems.into_iter().map(Item::Element).collect(),
            )
            .expect("the fixture row carries elements"),
        };
        let bool_val = || DataValue::DvBoolean(DvBoolean { value: true });
        let count_val = || {
            DataValue::DvCount(DvCount {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: openehr_base::containers::present_nonempty(Vec::new()),
                magnitude_status: None,
                accuracy: None,
                accuracy_is_percent: None,
                magnitude: 1,
            })
        };
        let table = |rows: Vec<Cluster>| ItemTable {
            name: text("t"),
            archetype_node_id: "at0000".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            rows: openehr_base::containers::present(rows),
        };
        let regular = table(vec![
            row(vec![named("a", bool_val()), named("b", count_val())]),
            row(vec![named("a", bool_val()), named("b", count_val())]),
        ]);
        let mut out = Vec::new();
        regular.validate_invariants(&mut out);
        assert!(
            !out.iter().any(|m| m.message.contains("Row_regularity")),
            "regular rows must pass, got {out:?}"
        );
        // Same count, different value type in position 2 → Row_regularity.
        let irregular = table(vec![
            row(vec![named("a", bool_val()), named("b", count_val())]),
            row(vec![named("a", bool_val()), named("b", bool_val())]),
        ]);
        let mut out = Vec::new();
        irregular.validate_invariants(&mut out);
        assert!(
            out.iter().any(|m| m.message.contains("Row_regularity")),
            "type-irregular rows must fail, got {out:?}"
        );
        // Different name in position 1 → Row_regularity.
        let renamed = table(vec![
            row(vec![named("a", bool_val())]),
            row(vec![named("x", bool_val())]),
        ]);
        let mut out = Vec::new();
        renamed.validate_invariants(&mut out);
        assert!(
            out.iter().any(|m| m.message.contains("Row_regularity")),
            "name-irregular rows must fail, got {out:?}"
        );
    }
}
