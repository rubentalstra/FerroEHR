//! OPT 1.4 ingestion: canonical XML → [`OperationalTemplate`] parse.
//!
//! # Spec basis
//!
//! An operational template is the **compiled, inheritance-flattened, standalone
//! top-level artefact** (`docs/specs/openehr/AM/docs/OPT2/master02-overview.adoc`
//! §Purpose of the OPT, §Types of OPT; `master03-opt_raw.adoc` §Flattening) and,
//! being a descendant of `AUTHORED_RESOURCE`
//! (`docs/specs/openehr/BASE/docs/resource/master02-resource_package.adoc`
//! §Meta-data), carries the meta-data (original language +
//! translations, `RESOURCE_DESCRIPTION`, revision history).
//!
//! NOTE (OPT 1.4 has no prose master): there is **no normative
//! prose chapter** for the OPT 1.4 wire structure (the OPT2 masters describe the
//! ADL2 successor). The OPT
//! 1.4 canonical XML this module ingests is governed by the **ITS-XML v1
//! Template XSD** plus AOM 1.4 — cite those, never the OPT2 masters, for
//! structure conformance. The tolerant [`openehr_its::opt14`] codec decodes it;
//! the structural well-formedness gate closing the leniency the codec would
//! otherwise accept is owned by the artefact-validity area
//! (`crate::validation::structure::validate_opt_structure`) — the store calls
//! it before every ingest (see [`crate::templates::store`]).
//!
//! NOTE (meta-data parsed, not surfaced): the
//! meta-data (`language` / `description` / `translations` / `revision_history`)
//! is parsed by the codec but we index only `template_id` / `concept` / root
//! archetype for lookup and listing (see [`crate::templates::store`]); the spec
//! permits an optional `_description_` (BASE resource master02 §Meta-data).
//! Surfacing/querying the full `AUTHORED_RESOURCE` meta-data is not required by
//! the provisioning surface.

use openehr_its::opt14::OperationalTemplate;

use crate::service::error::ServiceError;

/// Parse OPT 1.4 canonical XML into an [`OperationalTemplate`].
///
/// Failure classification follows the ITS-REST status split: a payload that is
/// not well-formed XML at all is *syntactically invalid content* — the released
/// `400` branch (`docs/specs/openehr/ITS-REST/specifications/responses/400.yaml`:
/// "the request could not be parsed or is invalid (e.g. … syntactically
/// invalid … content)") — while well-formed XML that does not decode as an OPT
/// is a semantic error on the artefact (the overview status table's `422` row,
/// `docs/specs/openehr/ITS-REST/specifications/docs/overview/
/// Requests_and_responses.md` §HTTP status codes; no template operation
/// declares `422`, so the semantic branch is the adjudicated handling).
///
/// # Errors
///
/// - [`ServiceError::BadRequest`] (→ ITS-REST `400`) when the payload is not
///   well-formed XML (including an empty document).
/// - [`ServiceError::Unprocessable`] (→ ITS-REST `422`) when well-formed XML
///   does not decode as an OPT 1.4 `OPERATIONAL_TEMPLATE` document.
pub(crate) fn parse_opt(xml: &str) -> Result<OperationalTemplate, ServiceError> {
    require_well_formed_xml(xml)?;
    openehr_its::opt14::from_xml(xml).map_err(|e| {
        ServiceError::content_invalid(crate::service::error::Violation::new(format!(
            "invalid OPT 1.4 XML: {e}"
        )))
    })
}

/// The well-formedness gate ahead of the OPT decode: scan the document with a
/// bare `quick_xml::Reader` and reject anything that is not one well-formed
/// XML document with a root element. This isolates the released `400` branch
/// (syntactically invalid content) from the semantic `422` branch the tolerant
/// codec reports past it.
fn require_well_formed_xml(xml: &str) -> Result<(), ServiceError> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut saw_root = false;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::Start(_) | quick_xml::events::Event::Empty(_)) => {
                saw_root = true;
            }
            Ok(_) => {}
            Err(e) => {
                return Err(ServiceError::precondition(format!(
                    "syntactically invalid XML content: {e}"
                )));
            }
        }
    }
    if saw_root {
        Ok(())
    } else {
        Err(ServiceError::precondition(
            "syntactically invalid XML content: the document has no root element".to_owned(),
        ))
    }
}
