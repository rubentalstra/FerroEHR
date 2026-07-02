//! Fixtures for rm.data_structures classes.

use openehr_foundation::serde_support::TypeTag;
use openehr_foundation::time::iso8601_duration::Iso8601Duration;
use openehr_foundation::time::iso8601_type::Iso8601TypeCore;
use openehr_rm::data_structures::history::event::EventData;
use openehr_rm::data_structures::history::history::History;
use openehr_rm::data_structures::history::interval_event::IntervalEvent;
use openehr_rm::data_structures::history::point_event::PointEvent;
use openehr_rm::data_structures::item_structure::data_structure::DataStructureData;
use openehr_rm::data_structures::item_structure::item_list::ItemList;
use openehr_rm::data_structures::item_structure::item_single::ItemSingle;
use openehr_rm::data_structures::item_structure::item_structure::{
    ItemStructure, ItemStructureData,
};
use openehr_rm::data_structures::item_structure::item_table::ItemTable;
use openehr_rm::data_structures::representation::cluster::Cluster;
use openehr_rm::data_structures::representation::element::Element;
use openehr_rm::data_structures::representation::item::{Item, ItemData};
use openehr_rm::data_types::data_value::DataValue;
use openehr_rm::data_types::date_time::dv_duration::DvDuration;

use super::helpers::{coded, date_time, item_structure, item_tree, locatable, text};
use super::{Vector, vector};

fn element(name: &str, node: &str, value: &str) -> Element {
    Element {
        type_tag: TypeTag::new(),
        item: ItemData {
            locatable: locatable(name, node),
        },
        null_flavour: None,
        value: Some(DataValue::Text(text(value))),
        null_reason: None,
    }
}

fn cluster(name: &str, node: &str) -> Cluster {
    Cluster {
        type_tag: TypeTag::new(),
        item: ItemData {
            locatable: locatable(name, node),
        },
        items: vec![Item::Element(element("inner", "at0002", "v"))],
    }
}

fn structure_data(name: &str, node: &str) -> ItemStructureData {
    ItemStructureData {
        data_structure: DataStructureData {
            locatable: locatable(name, node),
        },
    }
}

fn event_data(name: &str, node: &str) -> EventData<ItemStructure> {
    EventData {
        locatable: locatable(name, node),
        time: date_time("2026-07-02T10:00:00Z"),
        state: None,
        data: item_structure("data", "at0003"),
    }
}

fn duration(value: &str) -> DvDuration {
    DvDuration {
        type_tag: TypeTag::new(),
        accuracy_is_percent: None,
        accuracy: None,
        iso8601: Iso8601Duration {
            core: Iso8601TypeCore {
                value: value.to_string(),
            },
        },
    }
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        vector("ELEMENT", &element("systolic", "at0004", "120")),
        vector("CLUSTER", &cluster("blood pressure", "at0001")),
        vector("ITEM_TREE", &item_tree("tree", "at0001")),
        vector(
            "ITEM_LIST",
            &ItemList {
                type_tag: TypeTag::new(),
                item_structure: structure_data("list", "at0001"),
                items: Some(vec![element("first", "at0002", "one")]),
            },
        ),
        vector(
            "ITEM_SINGLE",
            &ItemSingle {
                type_tag: TypeTag::new(),
                item_structure: structure_data("single", "at0001"),
                item: element("only", "at0002", "one"),
            },
        ),
        vector(
            "ITEM_TABLE",
            &ItemTable {
                type_tag: TypeTag::new(),
                item_structure: structure_data("table", "at0001"),
                rows: Some(vec![cluster("row 1", "at0002")]),
            },
        ),
        vector(
            "POINT_EVENT",
            &PointEvent::<ItemStructure> {
                type_tag: TypeTag::new(),
                event: event_data("point", "at0005"),
            },
        ),
        vector(
            "INTERVAL_EVENT",
            &IntervalEvent::<ItemStructure> {
                type_tag: TypeTag::new(),
                event: event_data("interval", "at0006"),
                width: duration("PT5M"),
                sample_count: None,
                math_function: coded("mean", "openehr", "146"),
            },
        ),
        vector(
            "HISTORY",
            &History::<ItemStructure> {
                type_tag: TypeTag::new(),
                data_structure: DataStructureData {
                    locatable: locatable("history", "at0001"),
                },
                origin: date_time("2026-07-02T10:00:00Z"),
                period: None,
                duration: None,
                summary: None,
                events: None,
            },
        ),
    ]
}
