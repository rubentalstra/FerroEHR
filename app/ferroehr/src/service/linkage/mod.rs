// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The LINKAGE service module: which demographic party is the subject of
//! which EHR.
//!
//! **No openEHR spec governs this — our own design/extension.** It realizes no
//! SM chapter: the SM has no linkage component, and nothing on the openEHR
//! wire depends on this map. `ehr_get_by_subject` obliges a match against the
//! EHR's own `EHR_STATUS.subject.external_ref.id.value` and `.namespace`
//! (ITS-REST `specifications/operations/ehr_get_by_subject.yaml`), which the
//! clinical schema serves from its own promoted columns — and which
//! [`crate::privacy::PrivacyPolicy`] constrains to an opaque pseudonym. That
//! operation therefore resolves a PSEUDONYM, and routing it through this
//! module would move no map.
//!
//! What this module holds is the resolution the system can perform nowhere
//! else: from a person to a record. GDPR Art. 4(5) defines pseudonymisation as
//! processing where attribution to a person needs "additional information"
//! that is "kept separately and subject to technical and organisational
//! measures" (<https://eur-lex.europa.eu/eli/reg/2016/679/oj>); this map IS
//! that additional information, so it lives in its own schema under its own
//! role.
//!
//! ## Where the two domains meet, and why that is safe
//!
//! [`FerroEhrService::resolve_ehr_for_identity`] is the only path that crosses
//! from an external identity to an EHR, and it crosses at the APPLICATION
//! layer over TWO pools, never inside one statement. It asks
//! [`FerroEhrService::resolve_party_by_identifier`] — the demographic pool,
//! through `demographic.resolve_national_identifier`, which matches a keyed
//! digest and never decrypts — for the party, and then asks the linkage pool
//! for that party's EHR. Two connections, two search paths, and under
//! `[db].demographic_url` + `[db].linkage_url` two database roles, each
//! revoked from the other's schema in both directions by
//! `linkage/0001_baseline`. No database credential spans the join, none must
//! ever be given one, and `crate::db::verify_domain_isolation` refuses to boot
//! a database where one does.
//!
//! ## Temporal, never destructive
//!
//! A merge or a split CLOSES the mapping in force and opens its successor in
//! one transaction; nothing here deletes a row, and the linkage role holds no
//! `DELETE` privilege to do it with. "Which party was the subject of this EHR
//! when that composition was written" therefore stays answerable after the
//! two person records have been merged. The temporal primary key
//! (`PRIMARY KEY (tenant_id, party_id, sys_period WITHOUT OVERLAPS)`) is what
//! enforces one mapping in force per party — the database, not whichever code
//! path happens to write.
//!
//! ## Every crossing is recorded
//!
//! Each operation emits one `linkage`-domain access event
//! ([`crate::system_log::event::AccessDomain::Linkage`]), naming the actor,
//! the declared purpose of use and how many mappings it resolved. A boundary
//! crossing nobody can reconstruct afterwards is exactly what the access log
//! exists to prevent, and a MISS is recorded for the same reason a hit is: it
//! says someone asked.

pub mod cohort;
mod store;

use crate::ids::{EhrId, VoId};
use crate::service::FerroEhrService;
use crate::service::status::SmError;
use crate::system_log::event::{
    AccessDomain, AuditEvent, EventActionCode, EventOutcome, ObjectClass,
};

/// The subject reference a party is known by on the clinical side: an opaque
/// pseudonym in a declared namespace, minted by the server (#3232).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectPseudonym {
    /// The declared pseudonym namespace the value belongs to.
    pub namespace: String,
    /// The opaque, tenant-bound pseudonym.
    pub id: uuid::Uuid,
}

/// What went wrong resolving or rewriting the party-to-EHR map.
///
/// Typed rather than stringly: a caller distinguishing "this party is already
/// linked" from "the store is unavailable" reaches two different outcomes, and
/// a `23P01` read out of an error message is not a decision anyone should have
/// to make twice.
#[derive(Debug, thiserror::Error)]
pub enum LinkageError {
    /// The party already holds a mapping in force, so a second one would
    /// overlap it.
    ///
    /// Raised by the temporal primary key, not by a pre-read: a check-then-act
    /// would race, and the constraint is the enforcement either way.
    #[error("party {party_id} already holds a mapping to an EHR")]
    AlreadyLinked {
        /// The party that already holds one.
        party_id: VoId,
    },
    /// The party holds no mapping in force, so there is nothing to close.
    ///
    /// A merge or a split of a party that was never linked is a caller
    /// mistake, not an empty success: reporting it as success would leave the
    /// caller believing a mapping moved when none did.
    #[error("party {party_id} holds no mapping to an EHR")]
    NotLinked {
        /// The party that holds none.
        party_id: VoId,
    },
    /// Resolving the external identity to a party failed.
    ///
    /// The demographic half of the crossing, kept its own variant because it
    /// fails for reasons this domain has no say over — a deployment that
    /// configures no identifier protection has no identity to resolve at all.
    #[error("resolving the external identity to a party failed")]
    Identity(#[source] SmError),
    /// The linkage store refused or failed.
    #[error("the linkage store is unavailable")]
    Database(#[source] sqlx::Error),
    /// No pseudonym namespace is declared, so there is nothing to mint a
    /// subject reference in.
    ///
    /// The namespace is a deployment fact (`[privacy] subject_namespaces`),
    /// never something the server invents: a minted pseudonym must be one the
    /// subject rule admits, and that rule is defined by the declaration.
    #[error(
        "no pseudonym namespace is declared ([privacy] subject_namespaces), so no subject \
         reference can be minted"
    )]
    NoPseudonymNamespace,
    /// No identifier-protection key is configured, so a pseudonym cannot be
    /// derived.
    ///
    /// The pseudonym is a keyed derivation over the same root key that seals
    /// national identifiers (`[demographic.identifier_protection]`), one
    /// scheme rather than two.
    #[error(
        "no identifier-protection key is configured ([demographic.identifier_protection]), so \
         no subject pseudonym can be derived"
    )]
    NoMintingKey,
    /// Writing the minted subject reference onto the EHR's `EHR_STATUS`
    /// failed.
    #[error("writing the EHR_STATUS subject reference failed")]
    Status(#[source] SmError),
    /// The operation ran but its access record could not be taken, and the
    /// deployment fails closed, so the result is withheld.
    ///
    /// A crossing of the pseudonymisation boundary that nobody can reconstruct
    /// afterwards is what the access log exists to prevent; under
    /// `[audit] fail_mode = "closed"` an unrecorded crossing is refused rather
    /// than served.
    #[error("the access record could not be taken and the deployment fails closed")]
    Unrecorded(#[source] SmError),
}

impl FerroEhrService {
    /// Open a mapping: `party_id` is the subject of `ehr_id`, from now on.
    ///
    /// Who calls this is the consumer's question, not this module's: nothing
    /// populates the map automatically, because a mapping written by a path
    /// that guessed at it would be the one fact in the deployment nobody
    /// could audit.
    ///
    /// # Errors
    /// [`LinkageError::AlreadyLinked`] when the party already holds a mapping
    /// in force — close it with [`Self::merge`] or [`Self::split`] first —
    /// and [`LinkageError::Database`] when the write fails.
    pub async fn link(&self, party_id: VoId, ehr_id: EhrId) -> Result<(), LinkageError> {
        let mut conn = self.linkage_pool.acquire().await.map_err(classify)?;
        let outcome = store::open(&mut conn, party_id, ehr_id)
            .await
            .map_err(|error| classify_for(error, party_id));
        self.emit_linkage_access(
            EventActionCode::Create,
            format!("party-ehr:{party_id}"),
            outcome.is_ok().then_some(ehr_id),
            outcome.is_ok(),
        )?;
        outcome
    }

    /// The EHR `party_id` is the subject of right now, or `None`.
    ///
    /// The direct half of the map, for a caller that already holds a party id
    /// — an identity resolution that got there through the demographic domain,
    /// or a demographic read that is already entitled to the party.
    ///
    /// # Errors
    /// [`LinkageError::Database`] when the read fails.
    pub async fn resolve_ehr_for_party(
        &self,
        party_id: VoId,
    ) -> Result<Option<EhrId>, LinkageError> {
        let resolved = store::open_mapping(&self.linkage_pool, party_id)
            .await
            .map_err(classify)?;
        self.emit_linkage_access(
            EventActionCode::Read,
            format!("party-ehr:{party_id}"),
            resolved,
            true,
        )?;
        Ok(resolved)
    }

    /// The EHR whose subject holds `value` in `scheme`, or `None`.
    ///
    /// The whole crossing, and the only one: external identity → party →
    /// EHR. The first hop runs on the demographic pool through the keyed
    /// digest (the plaintext never reaches the database), the second on the
    /// linkage pool; the two never meet in one statement, and under separated
    /// roles no credential holds both. See the module documentation.
    ///
    /// Two access events are recorded, one per hop, because two boundaries
    /// were crossed and a reviewer asking "who learned that this person has a
    /// record here" needs both answers.
    ///
    /// # Errors
    /// [`LinkageError::Identity`] when the identity cannot be resolved —
    /// including a deployment that configures no identifier protection, which
    /// has no sealed identifier to resolve — and [`LinkageError::Database`]
    /// when the linkage read fails.
    pub async fn resolve_ehr_for_identity(
        &self,
        scheme: &str,
        value: &str,
    ) -> Result<Option<EhrId>, LinkageError> {
        let Some(party_id) = self
            .resolve_party_by_identifier(scheme, value)
            .await
            .map_err(LinkageError::Identity)?
        else {
            return Ok(None);
        };
        self.resolve_ehr_for_party(party_id).await
    }

    /// The opaque subject pseudonym `party` is known by on the clinical side,
    /// minted by the server (#3232).
    ///
    /// A keyed, tenant-bound derivation over the party id under the linkage
    /// domain ([`crate::service::demographic::identifier::crypto::RootKey::subject_pseudonym`]),
    /// in the first declared pseudonym namespace. The same party always yields
    /// the same pseudonym within a tenant, and no caller-supplied value enters
    /// it, so a national identifier cannot become a subject reference on this
    /// path by construction. The subject rule
    /// ([`crate::privacy::PrivacyPolicy::subject_rule_in_force`]) still governs every
    /// value that arrives from elsewhere: a client writing `EHR_STATUS`
    /// directly, an EHR-Extract, an archive load.
    ///
    /// # Errors
    /// [`LinkageError::NoPseudonymNamespace`] when the deployment declares no
    /// namespace, [`LinkageError::NoMintingKey`] when it configures no
    /// identifier-protection key.
    pub fn mint_subject_pseudonym(&self, party: VoId) -> Result<SubjectPseudonym, LinkageError> {
        let namespace = self
            .privacy
            .subject_namespaces()
            .first()
            .cloned()
            .ok_or(LinkageError::NoPseudonymNamespace)?;
        let engine = self
            .identifier_protection()
            .ok_or(LinkageError::NoMintingKey)?;
        Ok(SubjectPseudonym {
            namespace,
            id: engine.subject_pseudonym(party.0),
        })
    }

    /// Make `party` the subject of `ehr`: mint its pseudonym, write it as the
    /// EHR's `EHR_STATUS.subject.external_ref`, and open the mapping.
    ///
    /// The one path on which FerroEHR itself resolves a party into a subject
    /// reference, and the value written is the server's, never a caller's
    /// (#3232). The status write is an ordinary versioned `EHR_STATUS` commit
    /// (RM ehr master04 §EHR Status; the `PARTY_SELF.external_ref` slot of
    /// §`PARTY_SELF`), so the subject rule, the one-EHR-per-subject index and the
    /// database guard all see it; the mapping opens after it, so a party that
    /// is already linked leaves the EHR carrying the pseudonym it would carry
    /// anyway.
    ///
    /// # Errors
    /// The minting refusals above; [`LinkageError::Status`] when the EHR has no
    /// `EHR_STATUS`, the pseudonym already names another EHR, or the commit
    /// fails; and every [`Self::link`] error.
    pub async fn link_as_subject(
        &self,
        party: VoId,
        ehr: EhrId,
    ) -> Result<SubjectPseudonym, LinkageError> {
        let minted = self.mint_subject_pseudonym(party)?;
        let mut status = self
            .get_ehr_status_at_time(ehr, None)
            .await
            .map_err(LinkageError::Status)?;
        let preceding = status
            .pointer("/uid/value")
            .and_then(serde_json::Value::as_str)
            .map(str::parse::<openehr_base::prelude::ObjectVersionId>)
            .transpose()
            .map_err(|error| {
                LinkageError::Status(
                    SmError::exception("the current EHR_STATUS carries no readable version uid")
                        .with_source(error),
                )
            })?;
        // Built from the generated types, never a literal: the canonical shape
        // (attribute order, mandatory attributes) is correct by construction.
        let id =
            openehr_base::prelude::HierObjectId::new(minted.id.to_string()).map_err(|error| {
                LinkageError::Status(
                    SmError::exception("the minted pseudonym is not a valid HIER_OBJECT_ID")
                        .with_source(error),
                )
            })?;
        let subject = openehr_its::json::to_canonical_value(
            &openehr_rm::prelude::PartyProxy::PartySelf(openehr_rm::prelude::PartySelf {
                external_ref: Some(openehr_base::prelude::PartyRef {
                    namespace: minted.namespace.clone(),
                    r#type: "PERSON".to_owned(),
                    id: openehr_base::prelude::ObjectId::HierObjectId(id),
                }),
            }),
        );
        if let Some(object) = status.as_object_mut() {
            object.remove("uid");
            object.insert("subject".to_owned(), subject);
        }
        let data: openehr_rm::prelude::EhrStatus = openehr_its::json::from_canonical_value(&status)
            .map_err(|error| {
                LinkageError::Status(
                    SmError::exception("the current EHR_STATUS does not decode as EHR_STATUS")
                        .with_source(error),
                )
            })?;
        let mut envelope = crate::service::version_update::direct_envelope(data);
        envelope.preceding_version_uid = preceding;
        if let openehr_its::rest::generated::common::UpdateAudit::UpdateAudit(audit) =
            &mut envelope.commit_audit
        {
            audit.change_type = crate::service::version_update::change_type_coded(
                crate::versioning::audit::change_type::MODIFICATION,
            );
        }
        self.replace_ehr_status(ehr, envelope)
            .await
            .map_err(LinkageError::Status)?;
        self.link(party, ehr).await?;
        Ok(minted)
    }

    /// Merge `from_party` into `into_party`: the EHR the first was the subject
    /// of becomes the second's, in one transaction.
    ///
    /// Two person records turning out to be one person. The absorbed party's
    /// mapping is CLOSED rather than removed and the survivor's is opened at
    /// the same instant, so the half-open periods meet without overlapping and
    /// the history reads as what it is — a correction made on a date, not a
    /// past that was rewritten.
    ///
    /// # Errors
    /// [`LinkageError::NotLinked`] when `from_party` holds no mapping to
    /// move, [`LinkageError::AlreadyLinked`] when `into_party` already holds
    /// one — two parties each with their own EHR need those EHRs merged
    /// first, which this map cannot decide — and [`LinkageError::Database`]
    /// when the transaction fails.
    pub async fn merge(&self, from_party: VoId, into_party: VoId) -> Result<(), LinkageError> {
        let outcome = self.move_mapping(from_party, into_party, None).await;
        self.emit_linkage_access(
            EventActionCode::Update,
            format!("party-ehr:{from_party}->{into_party}"),
            outcome.as_ref().ok().copied(),
            outcome.is_ok(),
        )?;
        outcome.map(|_| ())
    }

    /// Split `party_id` onto `new_ehr_id`: the mapping in force is closed and
    /// a mapping to the new EHR opened, in one transaction.
    ///
    /// One person record turning out to be two people. The closed row keeps
    /// the previous EHR, so a reader asking which record this party was the
    /// subject of before the split still gets an answer.
    ///
    /// # Errors
    /// [`LinkageError::NotLinked`] when the party holds no mapping to split,
    /// and [`LinkageError::Database`] when the transaction fails.
    pub async fn split(&self, party_id: VoId, new_ehr_id: EhrId) -> Result<(), LinkageError> {
        let outcome = self
            .move_mapping(party_id, party_id, Some(new_ehr_id))
            .await;
        self.emit_linkage_access(
            EventActionCode::Update,
            format!("party-ehr:{party_id}"),
            outcome.as_ref().ok().copied(),
            outcome.is_ok(),
        )?;
        outcome.map(|_| ())
    }

    /// Close `from_party`'s mapping and open `to_party`'s successor in ONE
    /// transaction, returning the EHR the successor names.
    ///
    /// The shared body of [`Self::merge`] (a new party, the same EHR) and
    /// [`Self::split`] (the same party, a new EHR). One transaction is
    /// load-bearing: a close that committed without its successor would leave
    /// a record with no subject and no error to say so.
    async fn move_mapping(
        &self,
        from_party: VoId,
        to_party: VoId,
        to_ehr: Option<EhrId>,
    ) -> Result<EhrId, LinkageError> {
        let mut tx = self.linkage_pool.begin().await.map_err(classify)?;
        let closed = store::close(&mut tx, from_party)
            .await
            .map_err(classify)?
            .ok_or(LinkageError::NotLinked {
                party_id: from_party,
            })?;
        let successor = to_ehr.unwrap_or(closed);
        store::open(&mut tx, to_party, successor)
            .await
            .map_err(|error| classify_for(error, to_party))?;
        tx.commit().await.map_err(classify)?;
        Ok(successor)
    }

    /// Record one linkage-domain access.
    ///
    /// The record names the mapping (`party-ehr:<party>`, and both parties on
    /// a merge), the EHR it resolved to when it resolved to one, and the count, never an identifier value: a trail carrying the
    /// identity would hold the very data the sealing keeps out of readable
    /// storage. The actor is the request's authenticated committer and the
    /// purpose is the code the deployment's own vocabulary accepted
    /// ([`crate::system_log::access_context`]); outside a request scope both
    /// are absent, which the record states rather than guesses.
    ///
    /// # Errors
    /// [`LinkageError::Unrecorded`] when the sender rejected the record under
    /// `fail_mode = "closed"`; the caller withholds its result.
    fn emit_linkage_access(
        &self,
        action: EventActionCode,
        object_id: String,
        ehr_id: Option<EhrId>,
        succeeded: bool,
    ) -> Result<(), LinkageError> {
        if !self.audit_enabled() {
            return Ok(());
        }
        let outcome = if succeeded {
            EventOutcome::Success
        } else {
            EventOutcome::MinorFailure
        };
        let mut event = AuditEvent::new(action, ObjectClass::Demographic, outcome);
        event.domain = AccessDomain::Linkage;
        event.object_id = Some(object_id);
        event.ehr_id = ehr_id.map(|id| id.to_string());
        event.result_count = Some(u64::from(ehr_id.is_some()));
        stamp_requester(&mut event);
        event.legal_basis = self.audit_legal_basis().map(str::to_owned);
        self.record_access(event).map_err(LinkageError::Unrecorded)
    }
}

/// Put the request's actor and declared purpose on a service-emitted record.
///
/// The protocol adapter's own records get these from the request it still
/// holds; a record built down here gets them from the two task-locals the
/// adapter published for the request's scope
/// ([`crate::service::committer`], [`crate::system_log::access_context`]).
fn stamp_requester(event: &mut AuditEvent) {
    if let Some(committer) = crate::service::committer::current_committer() {
        event.user_id = committer.subject;
    }
    event.purpose = crate::system_log::access_context::current_purpose();
    if let Some(tenant) = crate::extensions::tenant_context::current() {
        event.tenant_id = Some(tenant.tenant_id);
    }
}

/// A driver error that carries no domain meaning beyond "the store failed".
fn classify(error: sqlx::Error) -> LinkageError {
    LinkageError::Database(error)
}

/// A driver error from a write that could be the temporal key refusing an
/// overlap, classified against the party the write named.
fn classify_for(error: sqlx::Error, party_id: VoId) -> LinkageError {
    if store::is_overlap(&error) {
        return LinkageError::AlreadyLinked { party_id };
    }
    LinkageError::Database(error)
}
