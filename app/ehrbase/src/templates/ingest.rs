//! OPT 1.4 ingestion: canonical XML → [`OperationalTemplate`] parse.
//!
//! # Spec basis
//!
//! An operational template is the **compiled, inheritance-flattened, standalone
//! top-level artefact** (`docs/specs/openehr/AM/docs/OPT2/master02-overview.adoc`
//! §Purpose of the OPT, §Types of OPT; `master03-opt_raw.adoc` §Flattening) and,
//! being a descendant of `AUTHORED_RESOURCE`
//! (`docs/specs/openehr/BASE/docs/resource/master02-resource_package.adoc`
//! §Meta-data), carries the S-01/S-02/S-03 meta-data (original language +
//! translations, `RESOURCE_DESCRIPTION`, revision history).
//!
//! NOTE (G-T11 — OPT 1.4 has no prose master): there is **no normative
//! prose chapter** for the OPT 1.4 wire structure (the OPT2 masters describe the
//! ADL2 successor; blueprint `docs/blueprint/03-am.md` §Spec defects). The OPT
//! 1.4 canonical XML this module ingests is governed by the **ITS-XML v1
//! Template XSD** plus AOM 1.4 — cite those, never the OPT2 masters, for
//! structure conformance. The tolerant [`openehr_its::opt14`] codec decodes it;
//! the structural well-formedness gate closing the leniency the codec would
//! otherwise accept is owned by the artefact-validity area
//! (`crate::validation::structure::validate_opt_structure`) — the store calls
//! it before every ingest (see [`crate::templates::store`]).
//!
//! NOTE (G-T12 — meta-data parsed, not surfaced): the S-01/S-02/S-03
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
/// A codec failure is a semantic error on the artefact, not a transport error:
/// the XML negotiated fine at the REST edge but does not decode as a
/// well-formed OPT — ITS-REST `responses/422.yaml` ("semantic validation
/// errors" on a syntactically convertible payload).
///
/// # Errors
///
/// [`ServiceError::Unprocessable`] (→ ITS-REST `422`) when the XML does not
/// decode as an OPT 1.4 `OPERATIONAL_TEMPLATE` document.
pub(crate) fn parse_opt(xml: &str) -> Result<OperationalTemplate, ServiceError> {
    openehr_its::opt14::from_xml(xml)
        .map_err(|e| ServiceError::Unprocessable(format!("invalid OPT 1.4 XML: {e}")))
}
