//! Hand-written RM class invariants (ADR-003) for `ITEM_TABLE`.
//!
//! Mirrors archie `ItemTable` + inherited LOCATABLE:
//! - `Valid_structure`: every item in every row `CLUSTER` is an `ELEMENT`
//!   (no nested clusters).
//! - `Valid_number_of_rows`: all rows have the same number of items.
//! - `Archetype_node_id_valid`: `archetype_node_id` non-empty.

use crate::data_structures::item_structure::item_table::ItemTable;
use crate::data_structures::representation::item::Item;
use crate::validate::{InvariantViolation, Validate, push_archetype_node_id_valid};

impl Validate for ItemTable {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        // Valid_structure: rows contain only ELEMENTs.
        if self.rows.iter().any(|row| {
            row.items
                .iter()
                .any(|item| !matches!(item, Item::Element(_)))
        }) {
            out.push(InvariantViolation::here(
                "Invariant Valid_structure failed on type ITEM_TABLE",
            ));
        }
        // Valid_number_of_rows: every row has the same number of items.
        if let Some(first) = self.rows.first()
            && self
                .rows
                .iter()
                .any(|row| row.items.len() != first.items.len())
        {
            out.push(InvariantViolation::here(
                "Invariant Valid_number_of_rows failed on type ITEM_TABLE",
            ));
        }
        push_archetype_node_id_valid(out, "ITEM_TABLE", &self.archetype_node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_structures::representation::cluster::Cluster;
    use crate::data_structures::representation::element::Element;
    use crate::data_types::basic::data_value::DataValue;
    use crate::data_types::basic::dv_boolean::DvBoolean;
    use crate::data_types::text::dv_text::{DvText, DvTextData};

    fn text(value: &str) -> DvText {
        DvText::DvText(DvTextData {
            value: value.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: Vec::new(),
            language: None,
            encoding: None,
        })
    }

    fn element(id: &str) -> Element {
        Element {
            name: text("e"),
            archetype_node_id: id.to_owned(),
            uid: None,
            links: Vec::new(),
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
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            items,
        }
    }

    fn table(rows: Vec<Cluster>) -> ItemTable {
        ItemTable {
            name: text("table"),
            archetype_node_id: "at0001".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            rows,
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
}
