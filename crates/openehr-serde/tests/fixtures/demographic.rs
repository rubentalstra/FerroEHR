//! Fixtures for rm.demographic classes.

use openehr_foundation::serde_support::TypeTag;
use openehr_rm::demographic::actor::ActorData;
use openehr_rm::demographic::address::Address;
use openehr_rm::demographic::agent::Agent;
use openehr_rm::demographic::capability::Capability;
use openehr_rm::demographic::contact::Contact;
use openehr_rm::demographic::group::Group;
use openehr_rm::demographic::organisation::Organisation;
use openehr_rm::demographic::party::PartyData;
use openehr_rm::demographic::party_identity::PartyIdentity;
use openehr_rm::demographic::party_relationship::PartyRelationship;
use openehr_rm::demographic::person::Person;
use openehr_rm::demographic::role::Role;

use super::helpers::{item_structure, locatable, party_ref};
use super::{Vector, vector};

fn identity(name: &str) -> PartyIdentity {
    PartyIdentity {
        type_tag: TypeTag::new(),
        locatable: locatable(name, "at0002"),
        details: item_structure("details", "at0003"),
    }
}

fn party_data(name: &str) -> PartyData {
    // The demographic schema definitions mark LOCATABLE's optional `uid` as
    // REQUIRED on PARTY subtypes.
    let mut loc = locatable(name, "openEHR-DEMOGRAPHIC-PERSON.person.v1");
    loc.uid = Some(
        openehr_base::identification::uid_based_id::UidBasedId::HierObjectId(super::helpers::hier(
            "0e04d3af-0f8a-4be3-90a0-4a34fc94b21e",
        )),
    );
    PartyData {
        locatable: loc,
        identities: vec![identity("legal name")],
        contacts: None,
        details: None,
        reverse_relationships: None,
        relationships: None,
    }
}

fn actor_data(name: &str) -> ActorData {
    ActorData {
        party: party_data(name),
        languages: None,
        roles: None,
    }
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        vector(
            "PERSON",
            &Person {
                type_tag: TypeTag::new(),
                actor: actor_data("J. Jansen"),
            },
        ),
        vector(
            "ORGANISATION",
            &Organisation {
                type_tag: TypeTag::new(),
                actor: actor_data("Example Hospital"),
            },
        ),
        vector(
            "GROUP",
            &Group {
                type_tag: TypeTag::new(),
                actor: actor_data("Care team A"),
            },
        ),
        vector(
            "AGENT",
            &Agent {
                type_tag: TypeTag::new(),
                actor: actor_data("Lab device"),
            },
        ),
        vector(
            "ROLE",
            &Role {
                type_tag: TypeTag::new(),
                party: party_data("General practitioner"),
                time_validity: None,
                performer: party_ref(
                    "demographic",
                    "PERSON",
                    "0e04d3af-0f8a-4be3-90a0-4a34fc94b21e",
                ),
                capabilities: None,
            },
        ),
        vector(
            "PARTY_RELATIONSHIP",
            &PartyRelationship {
                type_tag: TypeTag::new(),
                locatable: locatable("mother of", "at0001"),
                details: None,
                target: party_ref(
                    "demographic",
                    "PERSON",
                    "0e04d3af-0f8a-4be3-90a0-4a34fc94b21e",
                ),
                time_validity: None,
                source: party_ref(
                    "demographic",
                    "PERSON",
                    "cf6bf7d0-9d1a-4c8b-a0c9-3f7bb31e83c2",
                ),
            },
        ),
        vector("PARTY_IDENTITY", &identity("legal name")),
        vector(
            "CONTACT",
            &Contact {
                type_tag: TypeTag::new(),
                locatable: locatable("home", "at0004"),
                addresses: vec![Address {
                    type_tag: TypeTag::new(),
                    locatable: locatable("home address", "at0005"),
                    details: item_structure("details", "at0006"),
                }],
                time_validity: None,
            },
        ),
        vector(
            "ADDRESS",
            &Address {
                type_tag: TypeTag::new(),
                locatable: locatable("home address", "at0005"),
                details: item_structure("details", "at0006"),
            },
        ),
        vector(
            "CAPABILITY",
            &Capability {
                type_tag: TypeTag::new(),
                locatable: locatable("prescribing", "at0007"),
                credentials: item_structure("credentials", "at0008"),
                time_validity: None,
            },
        ),
    ]
}
