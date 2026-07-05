//! The generated ITS-REST server traits, implemented on [`EhrbaseService`].
//!
//! [`ehr`] holds the real EHR / `EHR_STATUS` logic (it overrides only the methods
//! it implements; the rest inherit the generated `NotImplemented` defaults). The
//! other four groups are not yet implemented, so their `impl`s are empty and
//! inherit the defaults wholesale (demographic, definition/templates P13, query
//! P16, admin later).

mod definition;
mod ehr;

use openehr_its::rest::generated::admin::AdminApi;
use openehr_its::rest::generated::demographic::DemographicApi;
use openehr_its::rest::generated::query::QueryApi;

use super::EhrbaseService;

// Not yet implemented — empty impls inherit the generated `NotImplemented`
// defaults (demographic: RM phase; query/AQL: P16; admin: later).
impl DemographicApi for EhrbaseService {}
impl QueryApi for EhrbaseService {}
impl AdminApi for EhrbaseService {}
