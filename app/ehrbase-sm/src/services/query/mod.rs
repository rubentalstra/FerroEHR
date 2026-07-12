//! The SM Query service (`master08-query_service.adoc`): `I_QUERY_SERVICE`
//! plus the execution specifications.
//!
//! "The model of querying here is based on the notion of being able to
//! execute either queries previously stored in the `DEFINITION` service, or
//! else ad hoc queries. … If either type of query executes successfully, the
//! response is a `RESULT_SET`" (master08 §Overview). A stored query is
//! identified by `reverse-domain-name '::' semantic-id [ '/' version ]`.

pub mod request;
pub mod service;

pub use request::{AqlQueryRequest, QueryOutcome};
pub use service::QueryService;
