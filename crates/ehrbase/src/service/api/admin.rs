//! [`AdminService`] on [`EhrbaseService`] — physical EHR deletion
//! (SM `I_ADMIN_SERVICE.physical_ehr_delete`).
//!
//! TODO(port): the admin service implementation lands with this seam; until
//! each method is overridden the trait defaults answer 501.

use async_trait::async_trait;

use ehrbase_rest::AdminService;

use crate::service::EhrbaseService;

#[async_trait]
impl AdminService for EhrbaseService {}
