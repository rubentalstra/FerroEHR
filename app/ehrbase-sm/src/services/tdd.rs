//! The SM `I_TDD_SERVICE` interface — importing **Template Data Documents**
//! (TDD: template-namespaced XML instances of a COMPOSITION) into an EHR
//! (`docs/specs/openehr/SM/docs/UML/classes/i_tdd_service.adoc`, included by
//! `SM/docs/openehr_platform/master09-message_service.adoc`).
//! Design digest: `docs/design/sm-platform/10-message-integration.md` §2
//! (the `ehrbase` component's `message`/`tdd` module — "TDD XML → OPT-guided
//! content model → COMPOSITION → the normal validated commit path").
//!
//! A TDD is **not** canonical openEHR XML: its root element is named after the
//! operational template (carrying a `template_id` attribute) in the Ocean/Marand
//! templates namespace (`http://schemas.oceanehr.com/templates`), its structural
//! nodes use the template node names, and its leaves are `rm:`-namespaced
//! canonical RM value fragments — the archetype-node-ids, `xsi:type`s and RM
//! structural attribute names are supplied by the OPT, not the document. Turning
//! a TDD into a COMPOSITION therefore requires an OPT-guided content walk
//! (archie's `TemplateDataDocument` reader is the prior art).
//!
//! PORT NOTE (spec surface). The vendored `i_tdd_service.adoc` gives only the
//! two call names and, for `import_tdd`, the `(an_ehr_id: UUID, tdd: String)`
//! parameter list — **no** return type, and **no** parameters, return, pre/post,
//! or errors for `import_tdds` ("Bulk import numerous TDDs"). The `master09`
//! narrative adds nothing. The signatures below therefore fill the gaps by
//! design (each choice flagged), and there are no ITS-REST endpoints for TDD
//! (Messaging is an OPTIONS-profile capability — no CORE/STANDARD conformance
//! impact), so like [`EhrExtractService`](super::EhrExtractService) this is a
//! native-API-only interface, **not** part of the [`Platform`](crate::Platform)
//! union.
//!
//! PORT NOTE (design-filled preconditions). The SM declares none; we fill
//! `has_ehr(an_ehr_id)` (an unknown EHR is `ehr_id_does_not_exist`) and
//! template resolution (an unknown `template_id` is `template_does_not_exist`),
//! surfaced as [`SmError`] over `CALL_STATUS_TYPE`.

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::SmError;

/// `I_TDD_SERVICE` — import of Template Data Documents, one Rust method per SM
/// call (`i_tdd_service.adoc`).
///
/// No default method bodies (compile-time completeness by design): a backend that
/// does not implement a call is a build error, not a silent `501`.
#[async_trait]
pub trait TddService: Send + Sync {
    /// `import_tdd (an_ehr_id: UUID, tdd: String)` — "Import a TDD." Converts the
    /// TDD XML instance to a COMPOSITION (guided by the referenced operational
    /// template) and commits it to `an_ehr_id` through the normal validated
    /// composition-commit path.
    ///
    /// PORT NOTE (return): the SM declares no return. We return the created
    /// COMPOSITION's `OBJECT_VERSION_ID` as a `String` — the same convention as
    /// [`EhrCompositionService::create_composition`](super::EhrCompositionService::create_composition)
    /// — so the caller can address the committed COMPOSITION (a TDD import that
    /// produced an unaddressable COMPOSITION would be useless).
    ///
    /// Preconditions (design-filled): `has_ehr(an_ehr_id)`
    /// (`ehr_id_does_not_exist`); the TDD's `template_id` names a provisioned
    /// operational template (`template_does_not_exist`). A malformed / structurally
    /// non-conformant TDD is `precondition_violation`.
    async fn import_tdd(&self, an_ehr_id: Uuid, tdd: String) -> Result<String, SmError>;

    /// `import_tdds` — "Bulk import numerous TDDs."
    ///
    /// PORT NOTE (signature + semantics): the SM gives this call **no** signature
    /// at all. By design it takes the target EHR id and the TDD instances, and
    /// returns the created COMPOSITIONs' `OBJECT_VERSION_ID`s in input order. It
    /// is **fail-fast, all-or-nothing**: every TDD is parsed and converted before
    /// any is committed, so a single bad TDD rejects the whole batch and commits
    /// nothing (no silent partial import) — consistent with this server's
    /// one-CONTRIBUTION-per-change-set discipline. (The design's alternative,
    /// per-item `DUMP_LOAD_FAIL_REPORT`-style results, is deferred until a real
    /// bulk caller needs partial success.)
    async fn import_tdds(&self, an_ehr_id: Uuid, tdds: Vec<String>)
    -> Result<Vec<String>, SmError>;
}
