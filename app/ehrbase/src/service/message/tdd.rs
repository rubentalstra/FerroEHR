//! TDD (Template Data Document) import — SM `I_TDD_SERVICE.import_tdd` /
//! `import_tdds` (`docs/specs/openehr/SM/docs/UML/classes/i_tdd_service.adoc`).
//!
//! A TDD is a template-namespaced XML instance of a COMPOSITION: the root
//! element is named after the operational template and carries a `template_id`
//! attribute in the Ocean/Marand templates namespace
//! (`http://schemas.oceanehr.com/templates`); structural nodes use the template
//! node names and leaves are `rm:`-namespaced canonical RM value fragments (see
//! the corpus at
//! `docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/compositions/TDD/`).
//! The document does **not** carry the archetype-node-ids, `xsi:type`s or RM
//! structural attribute names a COMPOSITION needs — those come from the OPT.
//!
//! ## Pipeline
//!
//! 1. **Envelope** — parse the XML, require the templates namespace + a
//!    `template_id`, verify the target EHR exists (`has_ehr`), and resolve the
//!    referenced operational template (an unknown template is
//!    `template_does_not_exist`, which also rejects the corpus
//!    `..__invalid_opt_doesnt_exist` case; a stored-but-unbuildable template
//!    surfaces through [`EhrbaseService::web_template_for`]).
//! 2. **Body conversion** — the OPT-guided TDD-body → canonical-COMPOSITION walk
//!    ([`openehr_flat::tdd::from_tdd`]): the template node names are matched to the
//!    `WebTemplate` node tree to supply `archetype_node_id`s, re-materialise the
//!    `HISTORY`/`EVENT`/`ITEM_TREE`/`ELEMENT` wrappers the template compacts, and
//!    parse each `rm:`-namespaced leaf into its RM datatype. A body that does not
//!    conform to the template is a typed `precondition_violation`.
//! 3. **Commit** — the produced canonical COMPOSITION goes through the normal
//!    validated [`EhrbaseService::create_composition`] path (`WebTemplate` +
//!    RM-invariant + terminology validation, contribution/audit — RM common
//!    master06 §Contributions), returning its `OBJECT_VERSION_ID`. A validation
//!    failure is `content_invalid` — never a silent partial COMPOSITION.
//!
//! NOTE (keep — `i_tdd_service.adoc` declares no `import_tdds` signature):
//! `import_tdds` is a design-filled `(UUID, Vec<String>) -> Vec<String>`,
//! all-or-nothing — every TDD is parsed and converted before any is committed,
//! so a single unconvertible TDD rejects the whole batch with nothing committed.
//! A flagged extension of the SM interface.
//!
//! This module reaches the templates layer through
//! [`EhrbaseService::web_template_for`] / [`EhrbaseService::get_template_xml`]
//! and the validated commit through [`EhrbaseService::create_composition`].

use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::Value;

use crate::ids::EhrId;
use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::service::status::{CallStatusType, SmError};
use crate::service::version_update::UpdateVersion;

/// The Ocean/Marand operational-template-data XML namespace every TDD root
/// declares (the corpus TDD instances use exactly this default `xmlns`).
const TDD_TEMPLATE_NS: &str = "http://schemas.oceanehr.com/templates";

/// The structural facts read from a TDD document's root element.
struct TddEnvelope {
    /// The operational template the TDD instantiates (root `template_id`
    /// attribute), e.g. `persistent_minimal.en.v1`.
    template_id: String,
}

/// Decode an XML attribute value as UTF-8 (`template_id` / `xmlns` are plain
/// text — no entity unescaping needed).
fn decode_attr(attr: &quick_xml::events::attributes::Attribute) -> Result<String, SmError> {
    std::str::from_utf8(&attr.value)
        .map(str::to_owned)
        .map_err(|e| {
            SmError::new(
                CallStatusType::ContentInvalid,
                format!("TDD attribute value is not valid UTF-8: {e}"),
            )
        })
}

/// Parse the TDD XML envelope: locate the root element, require the Ocean
/// templates namespace, and read its `template_id`. A document that is not
/// well-formed XML, is not in the templates namespace, or carries no
/// `template_id` is a typed rejection (`content_invalid` /
/// `precondition_violation`).
fn parse_tdd_envelope(tdd: &str) -> Result<TddEnvelope, SmError> {
    let mut reader = Reader::from_str(tdd);

    loop {
        match reader.read_event() {
            // The first element start is the TDD root.
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let mut template_id: Option<String> = None;
                let mut namespace: Option<String> = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|err| {
                        SmError::new(
                            CallStatusType::ContentInvalid,
                            format!("TDD root has malformed XML attributes: {err}"),
                        )
                    })?;
                    let key = attr.key.as_ref();
                    // The default-namespace declaration `xmlns="..."` (the
                    // TDD root declares the templates namespace as default).
                    if key == b"xmlns" {
                        namespace = Some(decode_attr(&attr)?);
                    } else if key == b"template_id" {
                        template_id = Some(decode_attr(&attr)?);
                    }
                }

                match namespace.as_deref() {
                    Some(TDD_TEMPLATE_NS) => {}
                    other => {
                        return Err(SmError::precondition(format!(
                            "TDD root is not in the templates namespace {TDD_TEMPLATE_NS:?} \
                             (found {other:?}); the payload is not a Template Data Document"
                        )));
                    }
                }
                let template_id = template_id.filter(|t| !t.is_empty()).ok_or_else(|| {
                    SmError::precondition("TDD root carries no non-empty template_id attribute")
                })?;
                return Ok(TddEnvelope { template_id });
            }
            Ok(Event::Eof) => {
                return Err(SmError::new(
                    CallStatusType::ContentInvalid,
                    "TDD payload has no root element (empty or not XML)",
                ));
            }
            // Skip the XML declaration, comments, leading text, etc.
            Ok(_) => {}
            Err(err) => {
                return Err(SmError::new(
                    CallStatusType::ContentInvalid,
                    format!("TDD payload is not well-formed XML: {err}"),
                ));
            }
        }
    }
}

impl EhrbaseService {
    /// SM `import_tdd`: convert a single TDD and commit it through the
    /// validated [`Self::create_composition`] path. Returns the created
    /// COMPOSITION's `OBJECT_VERSION_ID`.
    ///
    /// # Errors
    /// - `content_invalid` — the payload is not well-formed XML / has no root,
    ///   or the produced COMPOSITION fails WebTemplate/RM/terminology
    ///   validation at commit.
    /// - `precondition_violation` (`400`) — the root is not in the templates
    ///   namespace, carries no `template_id`, or the body does not conform to
    ///   the operational template.
    /// - `ehr_id_does_not_exist` — no EHR with `an_ehr_id` (`has_ehr` false).
    /// - `template_does_not_exist` — the referenced operational template is not
    ///   provisioned.
    /// - `exception` — a database fault while checking/committing.
    pub async fn import_tdd(&self, an_ehr_id: EhrId, tdd: String) -> Result<String, SmError> {
        let composition = self.prepare_one_tdd(an_ehr_id, &tdd).await?;
        // The validated commit path (WebTemplate + RM-invariant + terminology
        // validation, contribution/audit).
        let resp = self
            .create_composition(an_ehr_id, UpdateVersion::direct(composition))
            .await?;
        Ok(resp.version_uid())
    }

    /// The `import_tdds` extension: convert and commit a batch of TDDs,
    /// all-or-nothing — every TDD is parsed and converted before any is
    /// committed, so a single unconvertible TDD rejects the whole batch with
    /// nothing committed. Returns the created `OBJECT_VERSION_ID`s in input
    /// order.
    ///
    /// # Errors
    /// As [`Self::import_tdd`], for any TDD in the batch (a conversion failure
    /// rejects the batch before any commit).
    pub async fn import_tdds(
        &self,
        an_ehr_id: EhrId,
        tdds: Vec<String>,
    ) -> Result<Vec<String>, SmError> {
        let mut prepared = Vec::with_capacity(tdds.len());
        for tdd in &tdds {
            prepared.push(self.prepare_one_tdd(an_ehr_id, tdd).await?);
        }
        let mut ids = Vec::with_capacity(prepared.len());
        for composition in prepared {
            // The validated commit path (as in `import_tdd`).
            let resp = self
                .create_composition(an_ehr_id, UpdateVersion::direct(composition))
                .await?;
            ids.push(resp.version_uid());
        }
        Ok(ids)
    }

    /// Validate a TDD's envelope against a target EHR and its referenced
    /// template, then convert its body to a canonical COMPOSITION (no commit).
    ///
    /// Envelope failures are typed (`ehr_id_does_not_exist`,
    /// `template_does_not_exist`); a body that does not conform to the template
    /// is `precondition_violation`. Splitting prepare from commit is what lets
    /// [`Self::import_tdds`] convert a whole batch before committing any.
    async fn prepare_one_tdd(&self, ehr_id: EhrId, tdd: &str) -> Result<Value, SmError> {
        let envelope = parse_tdd_envelope(tdd)?;

        // Precondition: the target EHR exists (`has_ehr`).
        let ehr_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
            .bind(ehr_id)
            .fetch_one(&self.pool)
            .await
            .map_err(ServiceError::from)?;
        if !ehr_exists {
            return Err(SmError::ehr_not_found(format!("no EHR with id {ehr_id}")));
        }

        // Precondition: the referenced operational template is provisioned. An
        // unknown template_id is `template_does_not_exist` — the template store
        // names that status at construction and `ServiceError` round-trips it
        // losslessly (this rejects the corpus `..__invalid_opt_doesnt_exist`
        // TDD). A stored-but-unbuildable template surfaces through
        // `web_template_for` as content_invalid.
        let _stored_xml = self.get_template_xml(&envelope.template_id).await?;
        let web_template = self.web_template_for(&envelope.template_id).await?;

        // OPT-guided body → canonical COMPOSITION. A body that does not conform
        // to the template is a typed precondition_violation (never a silent
        // partial COMPOSITION).
        openehr_flat::tdd::from_tdd(tdd, &web_template).map_err(|e| {
            SmError::precondition(format!(
                "TDD body does not conform to operational template {:?}: {e}",
                envelope.template_id
            ))
        })
    }
}
