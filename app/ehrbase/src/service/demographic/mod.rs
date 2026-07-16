//! DEMOGRAPHIC (PARTY + `PARTY_RELATIONSHIP`) service module — the platform-crate
//! realization of the SM DEMOGRAPHIC group over the shared
//! [`crate::versioning`] change-control machinery, with **no EHR scope**
//! (`ehr_id = None` — our own design: a party has no owning EHR). Parties
//! (PERSON / ORGANISATION / GROUP / AGENT / ROLE) and `PARTY_RELATIONSHIP`s are
//! versioned objects in the demographics repository.
//!
//! Internal split mirrors the SM interface boundaries
//! (`app/ehrbase-sm/src/services/demographic/`):
//! [`party`] = `I_PARTY` CRUD (+ `I_DEMOGRAPHIC_SERVICE.create_party`),
//! [`relationship`] = `I_PARTY_RELATIONSHIP` (+ `create_party_relationship`),
//! [`versioned`] = the `VERSIONED_PARTY` read surface (our extension),
//! [`contribution`] = the demographic (ehr-less) CONTRIBUTION (our extension),
//! [`tags`] = the demographic `ITEM_TAG` surface (our extension), and [`api`] =
//! the `DemographicService` + `PartyRelationshipService` trait impls.
//!
//! ITS-REST 1.0.3 defines no demographic wire contract (the SM demographic
//! service is abstract; the CNF demographic schedule — master10 — is all TBD;
//! CNF profiles list demographic as OPTIONS-profile only). This behaviour is
//! therefore our own extension **by analogy with the EHR group**: identical
//! status/`ETag`/`Location`/`Prefer`/`If-Match`/deleted-read semantics.
//! (Design register: `docs/design/platform/04-service-demographic-ehr-index.md`.)
//!
//! Standing PORT NOTEs (deliberate divergences, register §6):
//! - The `UV_PARTY`/`UV_PARTY_RELATIONSHIP` envelope (`uv_party.adoc`) is
//!   realized server-side; the wire seam carries a **bare RM party** and
//!   `lifecycle_state` defaults to `532|complete|` (a documented
//!   ITS-REST-style adaptation of `create_party`/`update_party`).
//! - The `definitions_valid` precondition + `definition_unknown` error
//!   (`i_demographic_service.adoc §create_party`) are deliberately
//!   unimplemented: there is no demographic archetype/OPT store (demographic
//!   is OPTIONS-profile only); only `valid_content` → `422` is enforced.
//! - `PARTY.reverse_relationships` (`party.adoc
//!   §Reverse_relationships_validity`) is a derived `0..1` attribute the
//!   server leaves unpopulated.
//!
//! Spec oracles for the RM-level rules enforced here:
//! - `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.demographic.party.adoc`
//!   (PARTY invariants `Identities_valid`, `Contacts_valid`,
//!   `Relationships_validity`, `Uid_mandatory`),
//!   `…demographic.actor.adoc` (`Roles_valid`), `…demographic.role.adoc`
//!   (`Capabilities_valid`, `performer`),
//!   `…demographic.party_relationship.adoc` (`source`/`target`),
//! - `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.party_ref.adoc`
//!   (`PARTY_REF.Type_validity`) + `…object_ref.adoc` (`OBJECT_REF.namespace`),
//! - `docs/specs/openehr/SM/docs/UML/classes/i_party.adoc` /
//!   `i_demographic_service.adoc` / `i_party_relationship.adoc`.

use std::sync::LazyLock;

use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::demographic::types::PartyKind;
use regex::Regex;
use serde_json::Value;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    AuditInput, CommitEnv, Committed, Kind, TreeId, VersionRead, object_kind, object_version_id,
    read_current, read_version, version_at,
};

pub(crate) mod api;
pub(crate) mod contribution;
pub(crate) mod party;
pub(crate) mod relationship;
pub(crate) mod tags;
pub(crate) mod versioned;

// The commit-path validators the CONTRIBUTION engine (`validate_for_commit`)
// dispatches to once the [`Kind`] is known from the payload `_type`.
pub(crate) use relationship::validate_relationship_for_commit;

/// The versioned-object [`Kind`] for a REST [`PartyKind`].
fn kind_of(kind: PartyKind) -> Kind {
    match kind {
        PartyKind::Agent => Kind::Agent,
        PartyKind::Group => Kind::Group,
        PartyKind::Organisation => Kind::Organisation,
        PartyKind::Person => Kind::Person,
        PartyKind::Role => Kind::Role,
    }
}

/// The REST [`PartyKind`] for a versioned-object [`Kind`], or `None` for a
/// non-party kind (COMPOSITION / `EHR_STATUS` / … / `PARTY_RELATIONSHIP`).
fn party_kind_of(kind: Kind) -> Option<PartyKind> {
    match kind {
        Kind::Agent => Some(PartyKind::Agent),
        Kind::Group => Some(PartyKind::Group),
        Kind::Organisation => Some(PartyKind::Organisation),
        Kind::Person => Some(PartyKind::Person),
        Kind::Role => Some(PartyKind::Role),
        _ => None,
    }
}

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
fn typed_check(rm_type: &str, data: &Value) -> Result<(), ServiceError> {
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

/// Validate a party body for a create/update: its root `_type` must equal the
/// routed [`PartyKind`]'s RM type (mismatch → `422` naming both), then the
/// structural + invariant checks of [`typed_check`].
fn validate_party_body(kind: PartyKind, body: &Value) -> Result<(), ServiceError> {
    let declared = body.get("_type").and_then(Value::as_str);
    if declared != Some(kind.rm_type()) {
        return Err(ServiceError::Unprocessable(format!(
            "party _type mismatch: the {} endpoint requires _type {:?}, got {:?}",
            kind.segment(),
            kind.rm_type(),
            declared.unwrap_or("<none>"),
        )));
    }
    typed_check(kind.rm_type(), body)
}

/// Validate a party version reached through the CONTRIBUTION path, where the
/// [`Kind`] was already derived from the payload `_type` (so only the structural +
/// invariant checks remain). The `CommitEnv::validate_for_commit`
/// implementation on [`EhrbaseService`] (`ehr/composition_validate.rs`)
/// dispatches a demographic-party `Kind` here.
pub(crate) fn validate_party_kind_for_commit(kind: Kind, data: &Value) -> Result<(), ServiceError> {
    typed_check(kind.as_str(), data)
}

/// Inject the `uid` (`OBJECT_VERSION_ID`) into a versioned object's JSON on read
/// — PARTY `Uid_mandatory` (`demographic.party.adoc`), the party's identity
/// copied from its enclosing VERSION.
fn inject_uid(mut canonical: Value, vo_id: Uuid, creating_system_id: &str, tree: TreeId) -> Value {
    if let Value::Object(map) = &mut canonical {
        map.insert(
            "uid".to_owned(),
            serde_json::json!({
                "_type": "OBJECT_VERSION_ID",
                "value": object_version_id(vo_id, creating_system_id, tree)
            }),
        );
    }
    canonical
}

impl EhrbaseService {
    /// Build the version `commit_audit` for a direct (non-CONTRIBUTION)
    /// demographic write: the effective `system_id`, the numeric
    /// `audit_change_type` group code, and the request's default committer
    /// (RM common master04 §Audit Details). The committer comes from the
    /// [`CommitEnv::default_committer`] implementation on [`EhrbaseService`]
    /// (the authenticated principal's `PARTY_PROXY`).
    fn demographic_audit(&self, change_type: &str, description: &str) -> AuditInput {
        AuditInput {
            system_id: self.effective_system_id(),
            change_type: change_type.to_owned(),
            description: Some(description.to_owned()),
            committer: CommitEnv::default_committer(self),
        }
    }

    /// Load a version of a party, verifying it is of the expected [`PartyKind`]
    /// and ehr-less. A wrong-kind or unknown id is `404`.
    async fn load_party_version(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        version: Option<TreeId>,
        at: Option<jiff::Timestamp>,
    ) -> Result<VersionRead, ServiceError> {
        // The stored kind (constant per versioned object) must match the route.
        let stored = object_kind(&self.pool, vo_id).await?;
        if stored != Some(kind_of(kind)) {
            return Err(ServiceError::NotFound(format!(
                "{} {vo_id}",
                kind.rm_type()
            )));
        }
        let read = match (version, at) {
            (Some(v), _) => read_version(&self.pool, vo_id, v).await?,
            (None, Some(at)) => version_at(&self.pool, vo_id, at).await?,
            (None, None) => read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.rm_type())))?;
        Ok(read)
    }

    /// Confirm a live party of the expected kind exists (not deleted).
    async fn ensure_party(&self, kind: PartyKind, vo_id: Uuid) -> Result<(), ServiceError> {
        let read = self.load_party_version(kind, vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::NotFound(format!(
                "{} {vo_id} is deleted",
                kind.rm_type()
            )));
        }
        Ok(())
    }

    /// The stored [`PartyKind`] of a versioned object, for the kind-agnostic SM
    /// `I_PARTY` calls (which address parties by versioned-object id only). A
    /// non-party id (COMPOSITION, `PARTY_RELATIONSHIP`, …) or unknown id is `404`
    /// (`versioned_object_does_not_exist`).
    pub(crate) async fn party_kind_at(&self, vo_id: Uuid) -> Result<PartyKind, ServiceError> {
        object_kind(&self.pool, vo_id)
            .await?
            .and_then(party_kind_of)
            .ok_or_else(|| ServiceError::NotFound(format!("versioned party {vo_id}")))
    }

    /// Confirm `vo_id` is some party (any of the five kinds) — the check for the
    /// kind-agnostic `versioned_party` reads. A non-party id (COMPOSITION, …) or
    /// unknown id is `404` (`versioned_object_does_not_exist`).
    async fn ensure_any_party(&self, vo_id: Uuid) -> Result<(), ServiceError> {
        match object_kind(&self.pool, vo_id).await? {
            Some(k) if k.is_party() => Ok(()),
            _ => Err(ServiceError::NotFound(format!("versioned party {vo_id}"))),
        }
    }

    /// A [`ServiceResponse`] for a loaded party: its canonical body with the
    /// `uid` injected (PARTY `Uid_mandatory`) plus the resource metadata (an
    /// empty `ehr_id` — parties are not EHR-scoped).
    fn party_version_response(vo_id: Uuid, read: VersionRead) -> ServiceResponse {
        let meta = ResourceMeta::new(
            String::new(),
            object_version_id(vo_id, &read.creating_system_id, read.tree),
        )
        .with_last_modified(read.time_committed);
        ServiceResponse::new(
            inject_uid(read.canonical, vo_id, &read.creating_system_id, read.tree),
            meta,
        )
    }

    /// The party create/update representation built **from the commit result**,
    /// never a post-commit re-read: the served body is the just-written
    /// `canonical` with the `uid` injected, and the identity + commit instant
    /// come straight from [`Committed`](crate::versioning::Committed) (RM common
    /// master06 §Committal — the written version identity). Byte-identical to a
    /// fresh [`read_party`](EhrbaseService::read_party): the served form is
    /// `inject_uid(reassemble(decompose(canonical)))`, and the node codec
    /// round-trips `canonical` losslessly (pinned by a test). The caller passes
    /// the pre-write `canonical`; the multimedia-externalization fallback (where
    /// the stored form is offloaded and the in-memory body would diverge) stays
    /// in [`EhrbaseService::create_party`] / `commit_party_update`.
    fn party_committed_response(canonical: Value, committed: &Committed) -> ServiceResponse {
        let vo_id = committed.vo_id;
        let meta = ResourceMeta::new(
            String::new(),
            object_version_id(vo_id, &committed.creating_system_id, committed.tree),
        )
        .with_last_modified(committed.time_committed);
        ServiceResponse::new(
            inject_uid(
                canonical,
                vo_id,
                &committed.creating_system_id,
                committed.tree,
            ),
            meta,
        )
    }
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
        assert!(typed_check("PERSON", &person(&json!([]))).is_err());
        assert!(typed_check("PERSON", &json!({ "_type": "PERSON" })).is_err());
        typed_check("PERSON", &person(&json!([identity()])))
            .expect("a party with one identity is valid");
    }

    #[test]
    fn present_but_empty_lists_are_rejected() {
        let mut body = person(&json!([identity()]));
        body["contacts"] = json!([]);
        let msg = match typed_check("PERSON", &body) {
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
        typed_check("ROLE", &role("PERSON")).expect("a legal performer ref is accepted");
        match typed_check("ROLE", &role("COMPOSITION")) {
            Err(ServiceError::Unprocessable(m)) => assert!(m.contains("Type_validity"), "got {m}"),
            other => panic!("expected performer Type_validity 422, got {other:?}"),
        }
    }
}

pub mod types;
