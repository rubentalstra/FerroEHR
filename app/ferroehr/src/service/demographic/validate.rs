//! Inbound-body validation for the demographic chapter — the RM
//! PARTY/ACTOR/ROLE/`PARTY_RELATIONSHIP` invariants and the BASE
//! `PARTY_REF`/`OBJECT_REF` rules enforced at the write boundary
//! (`valid_content` → `422`, `i_demographic_service.adoc §create_party`).
//!
//! Spec oracles:
//! - `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.demographic.party.adoc`
//!   (`Identities_valid`, `Contacts_valid`, `Relationships_validity`,
//!   `Uid_mandatory`), `…demographic.actor.adoc` (`Roles_valid`),
//!   `…demographic.role.adoc` (`Capabilities_valid`, `performer`),
//!   `…demographic.party_relationship.adoc` (`source`/`target`),
//! - `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.party_ref.adoc`
//!   (`PARTY_REF.Type_validity`) +
//!   `…object_ref.adoc` (`OBJECT_REF.namespace`),
//! - `docs/specs/openehr/RM/docs/demographic/master02` (§Modelling of Parties
//!   and Relationships — relationship refs denote the party's version
//!   container).

use openehr_base::prelude::PartyRef;
use openehr_base::validate::Validate;
use serde_json::Value;

use crate::service::demographic::types::PartyKind;
use crate::service::error::ServiceError;
use crate::versioning::Kind;

/// The RM `_type` a `PARTY_RELATIONSHIP` versioned object stores.
const RELATIONSHIP_RM_TYPE: &str = "PARTY_RELATIONSHIP";

/// Validate one inbound `PARTY_REF` value against the BASE class checks —
/// `PARTY_REF.Type_validity` and the inherited `OBJECT_REF.namespace` rule.
///
/// The rules themselves are NOT restated here: the reference is decoded into
/// the generated [`PartyRef`] and judged by that class's single spec-cited
/// realization in `openehr-base`
/// (`base_types/identification/party_ref_impl.rs`), so this service and the
/// whole-instance RM pass can never give two answers about the same ref.
/// `context` names the referencing attribute for the `422` message.
///
/// Decoding also enforces the mandatory `OBJECT_REF` attributes (`id`,
/// `namespace`, `type` are all 1..1 —
/// `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.object_ref.adoc`
/// §Attributes), which a partial hand check cannot see.
fn validate_party_ref(reference: &Value, context: &str) -> Result<(), ServiceError> {
    let typed = openehr_its::json::from_canonical_value::<PartyRef>(reference).map_err(|e| {
        ServiceError::Unprocessable(format!("{context}: not a well-formed PARTY_REF: {e}"))
    })?;
    let violations = typed.invariants();
    if let Some(first) = violations.first() {
        return Err(ServiceError::Unprocessable(format!(
            "{context}: {}",
            first.message
        )));
    }
    Ok(())
}

/// Structurally validate a candidate party body of concrete RM type `rm_type`:
/// deserialize into the corresponding `openehr_rm` demographic type (a type
/// mismatch → `422`) and enforce the enforceable PARTY/ACTOR/ROLE invariants.
/// `Uid_mandatory` is met by the server injecting the `uid` on read (mirroring
/// the COMPOSITION service), so an incoming body need not carry one.
fn party_check(rm_type: &str, data: &Value) -> Result<(), ServiceError> {
    use openehr_rm::prelude::{Agent, Group, Organisation, Person, Role};
    let typed = match rm_type {
        "AGENT" => openehr_its::json::from_canonical_value::<Agent>(data).map(drop),
        "GROUP" => openehr_its::json::from_canonical_value::<Group>(data).map(drop),
        "ORGANISATION" => openehr_its::json::from_canonical_value::<Organisation>(data).map(drop),
        "PERSON" => openehr_its::json::from_canonical_value::<Person>(data).map(drop),
        "ROLE" => openehr_its::json::from_canonical_value::<Role>(data).map(drop),
        other => {
            return Err(ServiceError::Unprocessable(format!(
                "not a demographic party type: {other:?}"
            )));
        }
    };
    typed.map_err(|e| {
        ServiceError::Unprocessable(format!("body does not validate as {rm_type}: {e}"))
    })?;

    // PARTY is unconditionally an archetype root (`demographic.party.adoc`
    // §Invariants `Is_archetype_root: is_archetype_root` — no antecedent),
    // so the same root-LOCATABLE rules the EHR_STATUS/EHR_ACCESS commits
    // enforce apply here: `Archetyped_valid` (archetype_details mandatory at
    // a root), the root `archetype_node_id` = the stringified
    // `archetype_details.archetype_id` (`locatable.adoc`
    // §archetype_node_id), and `Links_valid`.
    if let Some(obj) = data.as_object() {
        crate::service::ehr::validation::validate_root_locatable(obj, rm_type)?;
    }

    // PARTY invariant `Identities_valid`: `not identities.is_empty`
    // (`demographic.party.adoc`).
    let has_identities = data
        .get("identities")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty());
    if !has_identities {
        return Err(ServiceError::Unprocessable(format!(
            "{rm_type} violates PARTY invariant Identities_valid: identities must be non-empty"
        )));
    }

    // "Present implies non-empty" list invariants — only checkable on the raw
    // JSON (post-deserialize an absent and a present-empty list are the same
    // Vec): PARTY.Contacts_valid + Relationships_validity (party.adoc),
    // ACTOR.Roles_valid (actor.adoc), ROLE.Capabilities_valid (role.adoc).
    for (attr, invariant) in [
        ("contacts", "Contacts_valid"),
        ("relationships", "Relationships_validity"),
        ("roles", "Roles_valid"),
        ("capabilities", "Capabilities_valid"),
    ] {
        if data
            .get(attr)
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            return Err(ServiceError::Unprocessable(format!(
                "{rm_type}.{attr} is present but empty — a present list must be \
                 non-empty ({invariant})"
            )));
        }
    }

    // Relationships_validity, second arm (party.adoc): every inline
    // relationship's `source` must reference THIS party. The party's identity
    // is its `uid` (copied from the version container); when the body carries
    // one, an inline relationship pointing at another source is invalid.
    if let (Some(uid), Some(relationships)) = (
        data.pointer("/uid/value").and_then(Value::as_str),
        data.get("relationships").and_then(Value::as_array),
    ) {
        for (i, rel) in relationships.iter().enumerate() {
            let source = rel.pointer("/source/id/value").and_then(Value::as_str);
            if source.is_some_and(|s| !s.eq_ignore_ascii_case(uid)) {
                return Err(ServiceError::Unprocessable(format!(
                    "{rm_type}.relationships[{i}].source must reference this party \
                     (uid {uid}) — relationships are stored under their source \
                     (PARTY.Relationships_validity)"
                )));
            }
        }
    }

    // enforce PARTY_REF.Type_validity on the demographic refs a party
    // body carries. ACTOR.roles is `List<PARTY_REF>` (actor.adoc); ROLE.performer
    // is `PARTY_REF` (role.adoc). BASE `party_ref.adoc §Type_validity`.
    if let Some(roles) = data.get("roles").and_then(Value::as_array) {
        for (i, role_ref) in roles.iter().enumerate() {
            validate_party_ref(role_ref, &format!("{rm_type}.roles[{i}]"))?;
        }
    }
    if rm_type == "ROLE"
        && let Some(performer) = data.get("performer").filter(|v| !v.is_null())
    {
        validate_party_ref(performer, "ROLE.performer")?;
    }

    // The whole-instance RM class-invariant + terminology pass, rooted at the
    // concrete party type. The checks above are root-scoped; the RM class
    // invariants bind every node of the body (`ARCHETYPED.Rm_version_valid`,
    // `LOCATABLE.Links_valid`, the `LINK` 1..1 attributes,
    // `FEEDER_AUDIT_DETAILS.System_id_valid`, …), so an identity, contact or
    // nested CLUSTER below the root is judged by the same rules a COMPOSITION's
    // nodes are.
    crate::service::ehr::validation::validate_rm_invariants_for_commit(data, rm_type)
}

/// Structurally validate a candidate `PARTY_RELATIONSHIP` body: deserialize into
/// the `openehr_rm` type (a type mismatch → `422`), enforce that both `source`
/// and `target` `PARTY_REF`s are present continuant refs, and enforce their
/// `PARTY_REF.Type_validity`. `uid` need not be supplied — the server
/// injects it on read, mirroring the PARTY / COMPOSITION services.
fn relationship_check(data: &Value) -> Result<(), ServiceError> {
    use openehr_rm::prelude::PartyRelationship;
    // `source`/`target` are mandatory `PARTY_REF`s on the RM type, so a missing
    // one already fails deserialization; the explicit checks below give a
    // relationship-specific `422` message (and guard against a future optionality
    // change in the generated type).
    openehr_its::json::from_canonical_value::<PartyRelationship>(data).map_err(|e| {
        ServiceError::Unprocessable(format!("body does not validate as PARTY_RELATIONSHIP: {e}"))
    })?;
    for field in ["source", "target"] {
        let Some(reference) = data.get(field).filter(|v| !v.is_null()) else {
            return Err(ServiceError::Unprocessable(format!(
                "PARTY_RELATIONSHIP requires a {field} PARTY_REF"
            )));
        };
        // The refs denote the Version CONTAINER of a Party — an OBJECT_REF
        // carrying a HIER_OBJECT_ID (the continuant), never an
        // OBJECT_VERSION_ID (one particular version) — RM demographic
        // master02 §Modelling of Parties and Relationships.
        if reference
            .pointer("/id/_type")
            .and_then(Value::as_str)
            .is_some_and(|t| t == "OBJECT_VERSION_ID")
        {
            return Err(ServiceError::Unprocessable(format!(
                "PARTY_RELATIONSHIP.{field}.id must identify the party's version \
                 container (HIER_OBJECT_ID), not one version (OBJECT_VERSION_ID) \
                 — RM demographic master02"
            )));
        }
        // PARTY_REF.Type_validity + OBJECT_REF.namespace (BASE
        // `party_ref.adoc` / `object_ref.adoc`).
        validate_party_ref(reference, &format!("PARTY_RELATIONSHIP.{field}"))?;
    }
    // `PARTY_RELATIONSHIP` is a LOCATABLE too
    // (`RM/docs/UML/classes/org.openehr.rm.demographic.party_relationship.adoc`),
    // so the same whole-instance RM class-invariant pass applies.
    crate::service::ehr::validation::validate_rm_invariants_for_commit(data, RELATIONSHIP_RM_TYPE)
}

/// Validate a party body for a create/update: its root `_type` must equal the
/// routed [`PartyKind`]'s RM type (mismatch → `422` naming both), then the
/// structural + invariant checks of [`party_check`].
pub(super) fn validate_party_body(kind: PartyKind, body: &Value) -> Result<(), ServiceError> {
    let declared = body.get("_type").and_then(Value::as_str);
    if declared != Some(kind.rm_type()) {
        return Err(ServiceError::Unprocessable(format!(
            "party _type mismatch: the {} endpoint requires _type {:?}, got {:?}",
            kind.segment(),
            kind.rm_type(),
            declared.unwrap_or("<none>"),
        )));
    }
    party_check(kind.rm_type(), body)
}

/// Validate a relationship body for a direct create/update: its root `_type`
/// must be `PARTY_RELATIONSHIP` (mismatch → `422`), then [`relationship_check`].
pub(super) fn validate_relationship_body(body: &Value) -> Result<(), ServiceError> {
    let declared = body.get("_type").and_then(Value::as_str);
    if declared != Some(RELATIONSHIP_RM_TYPE) {
        return Err(ServiceError::Unprocessable(format!(
            "party_relationship _type mismatch: requires {RELATIONSHIP_RM_TYPE:?}, got {:?}",
            declared.unwrap_or("<none>"),
        )));
    }
    relationship_check(body)
}

/// Validate a party version reached through the CONTRIBUTION path, where the
/// [`Kind`] was already derived from the payload `_type` (so only the structural +
/// invariant checks remain). The `CommitEnv::validate_for_commit`
/// implementation on [`FerroEhrService`](crate::service::FerroEhrService)
/// (`service/ehr/composition_validate.rs`) dispatches a demographic-party
/// [`Kind`] here.
///
/// # Errors
/// [`ServiceError::Unprocessable`] when the body does not deserialize as the
/// kind's RM type or violates an enforceable PARTY/ACTOR/ROLE invariant
/// ([`party_check`]).
pub(crate) fn validate_party_kind_for_commit(kind: Kind, data: &Value) -> Result<(), ServiceError> {
    party_check(kind.as_str(), data)
}

/// Validate a relationship version reached through the CONTRIBUTION path (the
/// [`Kind`] was already derived from the payload `_type`, so only the structural
/// check remains). `FerroEhrService::validate_for_commit` dispatches a
/// [`Kind::PartyRelationship`] here.
///
/// # Errors
/// [`ServiceError::Unprocessable`] when the body does not deserialize as
/// `PARTY_RELATIONSHIP`, misses a `source`/`target` `PARTY_REF`, carries an
/// `OBJECT_VERSION_ID` ref, or violates `PARTY_REF.Type_validity` /
/// `OBJECT_REF.namespace` ([`relationship_check`]).
pub(crate) fn validate_relationship_for_commit(data: &Value) -> Result<(), ServiceError> {
    relationship_check(data)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A complete, RM-valid `PARTY_IDENTITY` (its LOCATABLE `name` +
    /// `archetype_node_id` and the mandatory `details` are 1..1 on the
    /// generated type).
    fn identity() -> Value {
        json!({
            "_type": "PARTY_IDENTITY",
            "name": { "_type": "DV_TEXT", "value": "legal name" },
            "archetype_node_id": "at0004",
            "details": {
                "_type": "ITEM_TREE",
                "name": { "_type": "DV_TEXT", "value": "details" },
                "archetype_node_id": "at0005",
                "items": []
            }
        })
    }

    fn person(identities: &Value) -> Value {
        // Root-valid per PARTY `Is_archetype_root` (party.adoc): the
        // ARCHETYPED block present, the root archetype_node_id equal to its
        // archetype_id — party_check enforces both before the PARTY rules.
        json!({
            "_type": "PERSON",
            "name": { "_type": "DV_TEXT", "value": "person" },
            "archetype_node_id": "openEHR-DEMOGRAPHIC-PERSON.person.v1",
            "archetype_details": {
                "_type": "ARCHETYPED",
                "archetype_id": { "_type": "ARCHETYPE_ID",
                                   "value": "openEHR-DEMOGRAPHIC-PERSON.person.v1" },
                "rm_version": "1.1.0"
            },
            "identities": identities
        })
    }

    /// PARTY is unconditionally an archetype root (`party.adoc`
    /// `Is_archetype_root`): a commit without `archetype_details`, or with a
    /// root `archetype_node_id` differing from the declared `archetype_id`,
    /// is a 422 (`locatable.adoc` `Archetyped_valid` + `§archetype_node_id`).
    #[test]
    fn party_root_locatable_rules_are_enforced() {
        let mut no_details = person(&json!([identity()]));
        no_details
            .as_object_mut()
            .unwrap()
            .remove("archetype_details");
        let msg = match party_check("PERSON", &no_details) {
            Err(ServiceError::Unprocessable(m)) => m,
            other => panic!("expected Unprocessable, got {other:?}"),
        };
        assert!(msg.contains("archetype_details"), "got {msg}");

        let mut mismatched = person(&json!([identity()]));
        mismatched["archetype_node_id"] = json!("openEHR-DEMOGRAPHIC-PERSON.other.v1");
        let msg = match party_check("PERSON", &mismatched) {
            Err(ServiceError::Unprocessable(m)) => m,
            other => panic!("expected Unprocessable, got {other:?}"),
        };
        assert!(msg.contains("archetype_node_id"), "got {msg}");
    }

    #[test]
    fn identities_valid_is_enforced() {
        // Empty / absent identities violate PARTY.Identities_valid.
        assert!(party_check("PERSON", &person(&json!([]))).is_err());
        assert!(party_check("PERSON", &json!({ "_type": "PERSON" })).is_err());
        party_check("PERSON", &person(&json!([identity()])))
            .expect("a party with one identity is valid");
    }

    #[test]
    fn present_but_empty_lists_are_rejected() {
        let mut body = person(&json!([identity()]));
        body["contacts"] = json!([]);
        let msg = match party_check("PERSON", &body) {
            Err(ServiceError::Unprocessable(m)) => m,
            other => panic!("expected Unprocessable, got {other:?}"),
        };
        assert!(msg.contains("Contacts_valid"), "got {msg}");
    }

    /// a `PARTY_REF` whose `type` is outside the legal set is a `422`
    /// (`PARTY_REF.Type_validity`); a legal supertype (`ACTOR`) is accepted.
    #[test]
    fn party_ref_type_validity_is_enforced() {
        let good = json!({ "_type": "PARTY_REF", "namespace": "demographic",
            "type": "PERSON", "id": { "_type": "HIER_OBJECT_ID", "value": "x" } });
        validate_party_ref(&good, "ctx").expect("PERSON is a legal PARTY_REF type");
        let actor = json!({ "_type": "PARTY_REF", "namespace": "local",
            "type": "ACTOR", "id": { "_type": "HIER_OBJECT_ID", "value": "x" } });
        validate_party_ref(&actor, "ctx").expect("ACTOR supertype is legal");
        let bad_type = json!({ "_type": "PARTY_REF", "namespace": "demographic",
            "type": "COMPOSITION", "id": { "_type": "HIER_OBJECT_ID", "value": "x" } });
        match validate_party_ref(&bad_type, "ctx") {
            Err(ServiceError::Unprocessable(m)) => assert!(m.contains("Type_validity"), "got {m}"),
            other => panic!("expected Type_validity 422, got {other:?}"),
        }
    }

    /// The accepting twin for the universal supertype `ANY`: the demographic
    /// write boundary judges `PARTY_REF.type` by the one realization in
    /// `openehr-base`, whose adjudicated set admits `ANY` (the value the CNF
    /// positive commit corpus writes into `PARTY_IDENTIFIED.external_ref` —
    /// `CNF/tests/platform/robot/_resources/test_data_sets/compositions/TDD/persistent_minimal.en.v1__full.xml`).
    /// Before this crate consumed that realization it kept its own list and
    /// refused `ANY`, so the two disagreed.
    #[test]
    fn party_ref_any_supertype_is_accepted() {
        let any = json!({ "_type": "PARTY_REF", "namespace": "local",
            "type": "ANY", "id": { "_type": "HIER_OBJECT_ID", "value": "x" } });
        validate_party_ref(&any, "ctx").expect("ANY is admitted by PARTY_REF.Type_validity");
    }

    /// an `OBJECT_REF.namespace` that is empty or violates the standard
    /// regex is a `422` (`OBJECT_REF.namespace`).
    #[test]
    fn object_ref_namespace_legality_is_enforced() {
        let bad_ns = json!({ "_type": "PARTY_REF", "namespace": "1nope",
            "type": "PERSON", "id": { "_type": "HIER_OBJECT_ID", "value": "x" } });
        match validate_party_ref(&bad_ns, "ctx") {
            Err(ServiceError::Unprocessable(m)) => {
                assert!(m.contains("Namespace_valid"), "got {m}");
            }
            other => panic!("expected namespace 422, got {other:?}"),
        }
    }

    /// The mandatory `OBJECT_REF` attributes (`id`, `namespace`, `type` all
    /// 1..1 — BASE `object_ref.adoc` §Attributes) are enforced by decoding the
    /// reference into the generated type, so a ref missing one is a `422`
    /// instead of passing a partial hand check.
    #[test]
    fn party_ref_mandatory_attributes_are_enforced() {
        let no_id = json!({ "_type": "PARTY_REF", "namespace": "local", "type": "PERSON" });
        assert!(matches!(
            validate_party_ref(&no_id, "ctx"),
            Err(ServiceError::Unprocessable(_))
        ));
        let no_type = json!({ "_type": "PARTY_REF", "namespace": "local",
            "id": { "_type": "HIER_OBJECT_ID", "value": "x" } });
        assert!(matches!(
            validate_party_ref(&no_type, "ctx"),
            Err(ServiceError::Unprocessable(_))
        ));
    }

    /// A ROLE.performer / ACTOR.roles ref is checked for `Type_validity`.
    #[test]
    fn role_performer_ref_is_validated() {
        let role = |performer_type: &str| {
            json!({
                "_type": "ROLE",
                "name": { "_type": "DV_TEXT", "value": "role" },
                "archetype_node_id": "openEHR-DEMOGRAPHIC-ROLE.role.v1",
                "archetype_details": { "_type": "ARCHETYPED",
                    "archetype_id": { "_type": "ARCHETYPE_ID",
                                       "value": "openEHR-DEMOGRAPHIC-ROLE.role.v1" },
                    "rm_version": "1.1.0" },
                "identities": [identity()],
                "performer": { "_type": "PARTY_REF", "namespace": "demographic",
                    "type": performer_type,
                    "id": { "_type": "HIER_OBJECT_ID", "value": "performer-id" } }
            })
        };
        party_check("ROLE", &role("PERSON")).expect("a legal performer ref is accepted");
        match party_check("ROLE", &role("COMPOSITION")) {
            Err(ServiceError::Unprocessable(m)) => assert!(m.contains("Type_validity"), "got {m}"),
            other => panic!("expected performer Type_validity 422, got {other:?}"),
        }
    }

    /// The whole-instance RM class-invariant pass reaches BELOW a party root:
    /// `ARCHETYPED.Rm_version_valid` (RM common
    /// `org.openehr.rm.common.archetyped.adoc` §Invariants) and
    /// `LOCATABLE.Links_valid` (`…common.locatable.adoc` §Invariants) on a
    /// nested identity are both 422s, and the valid twins are accepted.
    #[test]
    fn party_rm_invariants_below_the_root_are_enforced() {
        party_check("PERSON", &person(&json!([identity()]))).expect("the baseline person is valid");

        let mut empty_rm_version = person(&json!([identity()]));
        empty_rm_version["archetype_details"]["rm_version"] = json!("");
        let err = party_check("PERSON", &empty_rm_version)
            .expect_err("an empty rm_version must be refused");
        assert!(
            format!("{err:?}").contains("Rm_version_valid"),
            "the refusal should name the invariant, got {err:?}"
        );

        let mut nested_links = person(&json!([identity()]));
        nested_links["identities"][0]
            .as_object_mut()
            .unwrap()
            .insert("links".into(), json!([]));
        let err =
            party_check("PERSON", &nested_links).expect_err("an empty links list must be refused");
        assert!(
            format!("{err:?}").contains("Links_valid"),
            "the refusal should name the invariant, got {err:?}"
        );
    }
}
