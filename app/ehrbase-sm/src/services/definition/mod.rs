//! The SM Definitions service (`master04-definition_package.adoc`): "The
//! interfaces provided in this service are designed to enable any model-like
//! or reference artefacts, other than terminology, to be stored for use by
//! the rest of the system. This includes archetypes, templates, queries, and
//! query sets."
//!
//! One Rust trait per SM interface: [`DefinitionAdl14Service`]
//! (`i_definition_adl14.adoc`), [`DefinitionAdl2Service`]
//! (`i_definition_adl2.adoc`), [`DefinitionQueryService`]
//! (`i_definition_query.adoc`); [`QueryDescriptor`] (`query_descriptor.adoc`).
//! The `DEFINITION_CALL_STATUS_TYPE` members live in the flat
//! [`CallStatusType`](crate::CallStatusType) enum (chapter 03).
//!
//! PORT NOTE (interchange form). The SM signatures exchange AOM `ARCHETYPE` /
//! AOM2 `AUTHORED_ARCHETYPE` objects; openEHR publishes no BMM meta-model for
//! AOM instances, so the native API exchanges the **interchange
//! serializations** the platform actually ingests — ADL 1.4 source text /
//! OPT 1.4 canonical XML / ADL2 source text — and parsing happens inside the
//! service, exactly as the ITS-REST wire does.

pub mod adl14;
pub mod adl2;
pub mod query;

pub use adl14::DefinitionAdl14Service;
pub use adl2::DefinitionAdl2Service;
pub use query::{DefinitionQueryService, QueryDescriptor};
