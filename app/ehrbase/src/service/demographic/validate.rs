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

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::service::error::ServiceError;
use crate::service::demographic::types::PartyKind;
use crate::versioning::Kind;

/// `OBJECT_REF.namespace` legality: `"local"`, `"unknown"`, or a value matching
/// the standard regex `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*` (the two specials are
/// themselves matched by the regex) — BASE `base_types`
/// `object_ref.adoc §namespace`.
static NAMESPACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    // The pattern is a fixed literal, valid by construction — a build-time
    // invariant, not a runtime condition.
    #[allow(clippy::expect_used)]
    Regex::new(r"^[a-zA-Z][a-zA-Z0-9_.:/&?=+-]*$").expect("static namespace regex")
});

/// The legal `PARTY_REF.type` set — BASE `base_types` `party_ref.adoc`
/// §`Type_validity`: `type ∈ {PERSON, ORGANISATION, GROUP, AGENT, ROLE, PARTY,
/// ACTOR}` (abstract supertypes are allowed so a valid ref can still be built
/// to a subtype not known by the current implementation).
const PARTY_REF_TYPES: [&str; 7] = [
    "PERSON",
    "ORGANISATION",
    "GROUP",
    "AGENT",
    "ROLE",
    "PARTY",
    "ACTOR",
];

/// The RM `_type` a `PARTY_RELATIONSHIP` versioned object stores.
const RELATIONSHIP_RM_TYPE: &str = "PARTY_RELATIONSHIP";

/// Validate one inbound `PARTY_REF` value (G-17 + G-18): enforce
/// `PARTY_REF.Type_validity` (`type ∈` [`PARTY_REF_TYPES`], BASE `party_ref.adoc`)
/// and `OBJECT_REF.namespace` legality (BASE `object_ref.adoc`). `context`
/// names the referencing attribute for the `422` message. An absent value is
/// the caller's concern (mandatory refs fail deserialization first).
fn validate_party_ref(reference: &Value, context: &str) -> Result<(), ServiceError> {
    let ref_type = reference
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ServiceError::Unprocessable(format!(
                "{context}: PARTY_REF requires a type (OBJECT_REF.type)"
            ))
        })?;
    if !PARTY_REF_TYPES.contains(&ref_type) {
        return Err(ServiceError::Unprocessable(format!(
            "{context}: PARTY_REF.type {ref_type:?} is not one of \
             {{PERSON, ORGANISATION, GROUP, AGENT, ROLE, PARTY, ACTOR}} \
             (PARTY_REF.Type_validity)"
        )));
    }
    // OBJECT_REF.namespace is mandatory (1..1) and must be legal.
    let namespace = reference
        .get("namespace")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !NAMESPACE_RE.is_match(namespace) {
        return Err(ServiceError::Unprocessable(format!(
            "{context}: OBJECT_REF.namespace {namespace:?} is not \"local\", \"unknown\", \
             or a value matching [a-zA-Z][a-zA-Z0-9_.:/&?=+-]* (OBJECT_REF.namespace)"
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
        "AGENT" => serde_json::from_value::<Agent>(data.clone()).map(drop),
        "GROUP" => serde_json::from_value::<Group>(data.clone()).map(drop),
        "ORGANISATION" => serde_json::from_value::<Organisation>(data.clone()).map(drop),
        "PERSON" => serde_json::from_value::<Person>(data.clone()).map(drop),
        "ROLE" => serde_json::from_value::<Role>(data.clone()).map(drop),
        other => {
            return Err(ServiceError::Unprocessable(format!(
                "not a demographic party type: {other:?}"
            )));
        }
    };
    typed.map_err(|e| {
        ServiceError::Unprocessable(format!("body does not validate as {rm_type}: {e}"))
    })?;

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

    // G-17: enforce PARTY_REF.Type_validity on the demographic refs a party
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
    Ok(())
}

/// Structurally validate a candidate `PARTY_RELATIONSHIP` body: deserialize into
/// the `openehr_rm` type (a type mismatch → `422`), enforce that both `source`
/// and `target` `PARTY_REF`s are present continuant refs, and enforce their
/// `PARTY_REF.Type_validity` (G-17). `uid` need not be supplied — the server
/// injects it on read, mirroring the PARTY / COMPOSITION services.
fn relationship_check(data: &Value) -> Result<(), ServiceError> {
    use openehr_rm::prelude::PartyRelationship;
    // `source`/`target` are mandatory `PARTY_REF`s on the RM type, so a missing
    // one already fails deserialization; the explicit checks below give a
    // relationship-specific `422` message (and guard against a future optionality
    // change in the generated type).
    serde_json::from_value::<PartyRelationship>(data.clone()).map_err(|e| {
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
        // G-17: PARTY_REF.Type_validity + OBJECT_REF.namespace (BASE
        // `party_ref.adoc` / `object_ref.adoc`).
        validate_party_ref(reference, &format!("PARTY_RELATIONSHIP.{field}"))?;
    }
    Ok(())
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
/// implementation on [`EhrbaseService`](crate::service::EhrbaseService)
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
/// check remains). `EhrbaseService::validate_for_commit` dispatches a
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
    #![allow(clippy::expect_used)]

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
        json!({
            "_type": "PERSON",
            "name": { "_type": "DV_TEXT", "value": "person" },
            "archetype_node_id": "openEHR-DEMOGRAPHIC-PERSON.person.v1",
            "identities": identities
        })
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

    /// G-17: a `PARTY_REF` whose `type` is outside the legal set is a `422`
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

    /// G-18: an `OBJECT_REF.namespace` that is empty or violates the standard
    /// regex is a `422` (`OBJECT_REF.namespace`).
    #[test]
    fn object_ref_namespace_legality_is_enforced() {
        let bad_ns = json!({ "_type": "PARTY_REF", "namespace": "1nope",
            "type": "PERSON", "id": { "_type": "HIER_OBJECT_ID", "value": "x" } });
        match validate_party_ref(&bad_ns, "ctx") {
            Err(ServiceError::Unprocessable(m)) => assert!(m.contains("namespace"), "got {m}"),
            other => panic!("expected namespace 422, got {other:?}"),
        }
    }

    /// A ROLE.performer / ACTOR.roles ref is checked for `Type_validity` (G-17).
    #[test]
    fn role_performer_ref_is_validated() {
        let role = |performer_type: &str| {
            json!({
                "_type": "ROLE",
                "name": { "_type": "DV_TEXT", "value": "role" },
                "archetype_node_id": "openEHR-DEMOGRAPHIC-ROLE.role.v1",
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
}
