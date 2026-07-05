//! `AdminApi` implementation (Stage-1 `NotImplemented` stubs; P12 fills them).

use openehr_its::rest::generated::admin::{
    AdminApi, AdminEhrDeleteAllParams, AdminEhrDeleteParams,
};

crate::api::stub_api!(AdminApi, {
    admin_ehr_delete(AdminEhrDeleteParams) -> ();
    admin_ehr_delete_all(AdminEhrDeleteAllParams) -> ();
});
