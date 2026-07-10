//! TDD (Template Data Document) import — SM `I_TDD_SERVICE.import_tdd` /
//! `import_tdds` (`docs/specs/openehr/SM/docs/UML/classes/i_tdd_service.adoc`).
//! Design digest: `docs/design/sm-platform/10-message-integration.md` §2 (the
//! `ehrbase` `message`/`tdd` module — "TDD XML → OPT-guided content model →
//! COMPOSITION → the normal validated commit path").
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
//! ## Scope this wave (envelope, not body)
//!
//! This module implements the TDD **envelope**: parse the XML, require the
//! templates namespace + a `template_id`, verify the target EHR exists
//! (`has_ehr`), and resolve the referenced operational template (an unknown
//! template is `template_does_not_exist` — which also correctly rejects the
//! corpus `..__invalid_opt_doesnt_exist` case; a stored-but-unbuildable template
//! surfaces as a `content_invalid`/exception through
//! [`web_template_for`](EhrbaseService::web_template_for)).
//!
//! PORT NOTE (deferred: the OPT-guided body walk). Turning the TDD *content*
//! into a canonical COMPOSITION requires an OPT-guided content model — a walk of
//! the operational template's node tree in parallel with the TDD element tree
//! that (a) maps each template node name to its RM type + `archetype_node_id` +
//! structural attribute, (b) re-inserts the wrapper structures (`HISTORY` /
//! `EVENT` / `ITEM_TREE`) the template node names collapse over, and (c) parses
//! each `rm:`-namespaced leaf into its RM datatype (`DV_TEXT`, `DV_CODED_TEXT`,
//! `DV_QUANTITY`, `DV_DATE_TIME`, …). That is a subsystem in its own right (the
//! prior art is archie's `TemplateDataDocument` reader; the design sequences TDD
//! as step 4 of the SM-5 build) and is **not** implemented in this closing wave.
//! A well-formed TDD for a provisioned template is therefore rejected with a
//! typed `precondition_violation` naming the missing capability — never a
//! silent partial COMPOSITION (which would violate composition validity anyway,
//! since `archetype_node_id`/structure would be absent). Once the reader lands,
//! [`import_one_tdd`](EhrbaseService::import_one_tdd) resumes into the existing
//! validated [`create_composition`](EhrbaseService::create_composition) path
//! (and `import_tdds` gains its single-transaction, all-or-nothing commit).

use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::Event;
use uuid::Uuid;

use ehrbase_sm::{CallStatusType, SmError, TddService};

use super::{EhrbaseService, ServiceError};

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

impl EhrbaseService {
    /// Parse the TDD XML envelope: locate the root element, require the Ocean
    /// templates namespace, and read its `template_id`. A document that is not
    /// well-formed XML, is not in the templates namespace, or carries no
    /// `template_id` is a typed `precondition_violation`.
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

    /// Validate a single TDD against a target EHR and its referenced template,
    /// then convert + commit it. Returns the created COMPOSITION's
    /// `OBJECT_VERSION_ID`.
    ///
    /// See the module PORT NOTE: the OPT-guided body walk is not yet
    /// implemented, so a well-formed TDD for a provisioned template is rejected
    /// with a typed `precondition_violation` rather than committing a partial
    /// COMPOSITION.
    async fn import_one_tdd(&self, ehr_id: Uuid, tdd: &str) -> Result<String, SmError> {
        let envelope = Self::parse_tdd_envelope(tdd)?;

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
        // unknown template_id is `template_does_not_exist` (this rejects the
        // corpus `..__invalid_opt_doesnt_exist` TDD). A stored-but-unbuildable
        // template surfaces through `web_template_for` as content_invalid.
        match self.get_template_xml(&envelope.template_id).await {
            Ok(_) => {}
            Err(ServiceError::NotFound(_)) => {
                return Err(SmError::new(
                    CallStatusType::TemplateDoesNotExist,
                    format!(
                        "TDD references operational template {:?}, which is not provisioned",
                        envelope.template_id
                    ),
                ));
            }
            Err(e) => return Err(e.into()),
        }
        // Prove the template is usable (builds a WebTemplate) — the OPT-guided
        // body walk will consume it. Surfaces a stored-but-broken template.
        let _web_template = self.web_template_for(&envelope.template_id).await?;

        // PORT NOTE (deferred): the OPT-guided TDD body → COMPOSITION conversion
        // is not implemented this wave (module doc). Reject rather than commit a
        // partial/invalid COMPOSITION.
        Err(SmError::precondition(format!(
            "TDD body conversion for template {:?} is not yet supported: turning the \
             template-namespaced content into a canonical COMPOSITION requires the OPT-guided \
             content walk (archie's TemplateDataDocument reader; design 10-message-integration \
             §2), which is deferred. The TDD envelope (namespace, template_id, target EHR, \
             template provisioning) validated successfully.",
            envelope.template_id
        )))
    }
}

#[async_trait]
impl TddService for EhrbaseService {
    async fn import_tdd(&self, an_ehr_id: Uuid, tdd: String) -> Result<String, SmError> {
        self.import_one_tdd(an_ehr_id, &tdd).await
    }

    async fn import_tdds(
        &self,
        an_ehr_id: Uuid,
        tdds: Vec<String>,
    ) -> Result<Vec<String>, SmError> {
        // Fail-fast, all-or-nothing (trait PORT NOTE): every TDD is validated
        // (and, once the body reader lands, converted) before any is committed,
        // so a bad TDD rejects the whole batch with nothing committed.
        let mut ids = Vec::with_capacity(tdds.len());
        for tdd in &tdds {
            ids.push(self.import_one_tdd(an_ehr_id, tdd).await?);
        }
        Ok(ids)
    }
}
