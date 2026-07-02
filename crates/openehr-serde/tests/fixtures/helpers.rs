//! Shared deterministic constructors for the fixture set. Everything here
//! is fixed-value — no clocks, no generated UUIDs — so golden vectors stay
//! stable.

use openehr_base::identification::hier_object_id::HierObjectId;
use openehr_base::identification::object_id::ObjectId;
use openehr_base::identification::object_ref::ObjectRef;
use openehr_base::identification::object_version_id::ObjectVersionId;
use openehr_base::identification::terminology_id::TerminologyId;
use openehr_base::identification::uid_based_id::{UidBasedId, UidBasedIdData};
use openehr_base::identification::{object_id::ObjectIdData, party_ref::PartyRef};
use openehr_foundation::serde_support::TypeTag;
use openehr_rm::common::archetyped::locatable::LocatableData;
use openehr_rm::common::generic::audit_details::{AuditDetails, AuditDetailsData};
use openehr_rm::common::generic::party_proxy::{PartyProxy, PartyProxyData};
use openehr_rm::common::generic::party_self::PartySelf;
use openehr_rm::data_structures::item_structure::data_structure::DataStructureData;
use openehr_rm::data_structures::item_structure::item_structure::{
    ItemStructure, ItemStructureData,
};
use openehr_rm::data_structures::item_structure::item_tree::ItemTree;
use openehr_rm::data_types::date_time::dv_date_time::DvDateTime;
use openehr_rm::data_types::quantity::dv_ordered::DvOrderedApi;
use openehr_rm::data_types::text::code_phrase::CodePhrase;
use openehr_rm::data_types::text::dv_coded_text::DvCodedText;
use openehr_rm::data_types::text::dv_text::{DvText, DvTextData};

pub fn text_data(value: &str) -> DvTextData {
    DvTextData {
        value: value.to_string(),
        hyperlink: None,
        formatting: None,
        mappings: None,
        language: None,
        encoding: None,
    }
}

pub fn text(value: &str) -> DvText {
    DvText::Text {
        type_tag: TypeTag::new(),
        data: text_data(value),
    }
}

pub fn code_phrase(terminology: &str, code: &str) -> CodePhrase {
    CodePhrase {
        type_tag: TypeTag::new(),
        terminology_id: TerminologyId {
            type_tag: TypeTag::new(),
            object_id: ObjectIdData {
                value: terminology.to_string(),
            },
        },
        code_string: code.to_string(),
        preferred_term: None,
    }
}

pub fn coded(value: &str, terminology: &str, code: &str) -> DvCodedText {
    DvCodedText {
        type_tag: TypeTag::new(),
        text: text_data(value),
        defining_code: code_phrase(terminology, code),
    }
}

pub fn date_time(value: &str) -> DvDateTime {
    DvDateTime {
        type_tag: TypeTag::new(),
        temporal: temporal_data(),
        value: value.to_string(),
    }
}

pub fn temporal_data<T: DvOrderedApi>()
-> openehr_rm::data_types::date_time::dv_temporal::DvTemporalData<T> {
    openehr_rm::data_types::date_time::dv_temporal::DvTemporalData {
        quantified:
            openehr_rm::data_types::quantity::dv_absolute_quantity::DvAbsoluteQuantityData {
                quantified: quantified_data(),
                accuracy: None,
            },
        accuracy: None,
    }
}

pub fn quantified_data<T: DvOrderedApi>()
-> openehr_rm::data_types::quantity::dv_quantified::DvQuantifiedData<T> {
    openehr_rm::data_types::quantity::dv_quantified::DvQuantifiedData {
        ordered: ordered_data(),
        magnitude_status: None,
        accuracy: None,
    }
}

pub fn ordered_data<T: DvOrderedApi>()
-> openehr_rm::data_types::quantity::dv_ordered::DvOrderedData<T> {
    openehr_rm::data_types::quantity::dv_ordered::DvOrderedData {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
    }
}

pub fn amount_data<T: DvOrderedApi>() -> openehr_rm::data_types::quantity::dv_amount::DvAmountData<T>
{
    openehr_rm::data_types::quantity::dv_amount::DvAmountData {
        quantified: quantified_data(),
        accuracy_is_percent: None,
        accuracy: None,
    }
}

pub fn locatable(name: &str, node_id: &str) -> LocatableData {
    LocatableData {
        name: text(name),
        archetype_node_id: node_id.to_string(),
        uid: None,
        links: None,
        archetype_details: None,
        feeder_audit: None,
        parent: None,
    }
}

pub fn hier(value: &str) -> HierObjectId {
    HierObjectId {
        type_tag: TypeTag::new(),
        uid_based_id: UidBasedIdData {
            value: value.to_string(),
        },
    }
}

pub fn object_version_id(value: &str) -> ObjectVersionId {
    ObjectVersionId {
        type_tag: TypeTag::new(),
        uid_based_id: UidBasedIdData {
            value: value.to_string(),
        },
    }
}

pub fn object_ref(namespace: &str, r#type: &str, id: &str) -> ObjectRef {
    ObjectRef {
        type_tag: TypeTag::new(),
        namespace: namespace.to_string(),
        r#type: r#type.to_string(),
        id: ObjectId::UidBased(UidBasedId::HierObjectId(hier(id))),
    }
}

pub fn party_ref(namespace: &str, r#type: &str, id: &str) -> PartyRef {
    PartyRef {
        type_tag: TypeTag::new(),
        namespace: namespace.to_string(),
        r#type: r#type.to_string(),
        id: ObjectId::UidBased(UidBasedId::HierObjectId(hier(id))),
    }
}

pub fn party_self() -> PartySelf {
    PartySelf {
        type_tag: TypeTag::new(),
        party_proxy: PartyProxyData { external_ref: None },
    }
}

pub fn committer() -> PartyProxy {
    PartyProxy::PartySelf(party_self())
}

pub fn audit_data() -> AuditDetailsData {
    AuditDetailsData {
        system_id: "ehrbase.example.org".to_string(),
        time_committed: date_time("2026-07-02T10:00:00Z"),
        change_type: coded("creation", "openehr", "249"),
        description: None,
        committer: committer(),
    }
}

pub fn audit() -> AuditDetails {
    AuditDetails {
        type_tag: TypeTag::new(),
        data: audit_data(),
    }
}

pub fn item_tree(name: &str, node_id: &str) -> ItemTree {
    ItemTree {
        type_tag: TypeTag::new(),
        item_structure: ItemStructureData {
            data_structure: DataStructureData {
                locatable: locatable(name, node_id),
            },
        },
        items: None,
    }
}

pub fn item_structure(name: &str, node_id: &str) -> ItemStructure {
    ItemStructure::Tree(item_tree(name, node_id))
}
