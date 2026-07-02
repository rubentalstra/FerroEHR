//! `PARTY_PROXY` — abstract proxy description of a party.
//!
//! openEHR class: `PARTY_PROXY` (abstract), package `common.generic`.
//!
//! Abstract concept of a proxy description of a party, including an
//! optional link to data for this party in a demographic or other
//! identity management system. Subtyped into `PARTY_IDENTIFIED` and
//! `PARTY_SELF` (and, via `PARTY_IDENTIFIED`, `PARTY_RELATED`).
//!
//! There are two ways to refer to a demographic identity in the openEHR
//! EHR: using `PARTY_REF` directly, which records an identifier of the
//! party in some external system, and using `PARTY_PROXY`, consisting of a
//! small amount of descriptive data, depending on the subtype, and an
//! optional `PARTY_REF`. The approach taken in openEHR for representing
//! demographic and user entities in the EHR data is based on the following
//! assumptions:
//!
//! * there is at least one human readable name or official identifier of
//!   the party, such as `"Julius Marlowe, MD"`, `"NHS provider number
//!   1039385"`, or a system user id such as `"Rahil.Azam"`;
//! * there might be data in a service external to the EHR for the party
//!   in question, such as a demographic, identity management or patient
//!   index service; if there is, it should be referenceable;
//! * the subject of the record is never to be identified in any direct
//!   way (i.e. via the use of her name or other human-readable details),
//!   but may include a meaningless identifier in some external system.
//!
//! The `PARTY_PROXY` class and subtypes model references to parties based
//! on these assumptions. The semantics of `PARTY_PROXY` enable a flexible
//! approach: in stricter environments that have identity management and
//! demographic services, and where there is an entry in such a service for
//! the party in question, `PARTY_PROXY.external_ref` will be non-Void,
//! while in other environments, it will be empty.
//!
//! The two subtypes correspond to the mutually distinct categories of the
//! "subject of the record", known as the "self" party in openEHR (modelled
//! by `PARTY_SELF`), and any other party (modelled by `PARTY_IDENTIFIED`,
//! and its own subtype `PARTY_RELATED` for parties whose relationship to
//! the subject of the record is known).
use openehr_base::identification::party_ref::PartyRef;

use super::party_identified::PartyIdentified;
use super::party_related::PartyRelated;
use super::party_self::PartySelf;

/// Shared attribute state of `PARTY_PROXY` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait). `PARTY_PROXY` declares one attribute,
/// `external_ref: PARTY_REF` (cardinality `0..1`); every concrete subtype
/// embeds this struct.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyProxyData {
    /// `external_ref`: `PARTY_REF`, cardinality `0..1`.
    ///
    /// Optional reference to more detailed demographic or identification
    /// information for this party, in an external system.
    pub external_ref: Option<PartyRef>,
}

/// `PARTY_PROXY` is abstract in the spec and is used polymorphically
/// wherever an attribute is declared of that type — e.g.
/// `AUDIT_DETAILS.committer`, `PARTICIPATION.performer`,
/// `FEEDER_AUDIT_DETAILS.subject`. Per ADR-001 §4 (closed subtype set →
/// enum) — this enum is the ADR's own named example of the pattern — the
/// three concrete descendants `PARTY_SELF`, `PARTY_IDENTIFIED`, and
/// `PARTY_RELATED` are collected into this closed `enum` so a field or
/// return type can be declared `PartyProxy` exactly where the spec
/// declares it `PARTY_PROXY`.
///
/// `PartyRelated` is included as a direct variant here (not nested inside
/// a `PartyIdentified` variant) even though `PARTY_RELATED inherits
/// PARTY_IDENTIFIED` in the spec, because `PARTY_PROXY`'s own subtype set,
/// as drawn in the `common.generic` package diagram and described in the
/// package overview ("The two subtypes correspond to... `PARTY_SELF`...
/// and... `PARTY_IDENTIFIED`"), is really three siblings from the
/// `PARTY_PROXY` perspective: `PARTY_SELF`, `PARTY_IDENTIFIED`, and
/// `PARTY_RELATED`. This mirrors the `ObjectId`/`UidBasedId` nesting
/// precedent in `openehr_base::identification::object_id` in spirit
/// (narrower enums nest inside wider ones so covariant narrowing stays
/// type-direct) but is not identical to it: there, `UID_BASED_ID` is
/// itself a field's declared type elsewhere in the spec (`LOCATABLE_REF.id`),
/// which justified keeping `UidBasedId` as its own nested enum. No RM/BASE
/// attribute anywhere is declared with type `PARTY_IDENTIFIED` requiring a
/// narrower `PartyIdentified`-only enum in the same way, so flattening
/// `PartyRelated` directly into `PartyProxy` (rather than nesting a
/// `PartyIdentified(PartyIdentifiedOrRelated)`-shaped indirection) is the
/// simpler, equally faithful choice here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PartyProxy {
    /// `PARTY_SELF`.
    PartySelf(PartySelf),
    /// `PARTY_IDENTIFIED`.
    PartyIdentified(PartyIdentified),
    /// `PARTY_RELATED`.
    PartyRelated(PartyRelated),
}

/// Marker/accessor trait shared by every `PARTY_PROXY` descendant,
/// exposing the abstract class's sole attribute uniformly whether the
/// caller holds a concrete type or a `PartyProxy` enum value.
pub trait PartyProxyApi {
    /// `external_ref`: `PARTY_REF`, cardinality `0..1`. See
    /// [`PartyProxyData::external_ref`].
    fn external_ref(&self) -> Option<&PartyRef>;
}

impl PartyProxyApi for PartyProxy {
    fn external_ref(&self) -> Option<&PartyRef> {
        match self {
            PartyProxy::PartySelf(v) => v.external_ref(),
            PartyProxy::PartyIdentified(v) => v.external_ref(),
            PartyProxy::PartyRelated(v) => v.external_ref(),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/party_proxy.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Referring to Demographic Entities / uml_classes/party_proxy.adoc §PARTY_PROXY Class
//   confidence: high
//   todos: 0
//   note: PartyProxy is the ADR-001 §4 named closed-enum example for this transcription pass; PartyRelated flattened as a direct sibling variant rather than nested inside PartyIdentified, since no spec attribute anywhere is typed narrowly as PARTY_IDENTIFIED requiring the narrower nesting (contrast ObjectId/UidBasedId) — documented as a considered, not arbitrary, choice.
// ─────────────────────────────────────────────
