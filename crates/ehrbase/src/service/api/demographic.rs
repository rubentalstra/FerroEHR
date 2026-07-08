//! [`DemographicService`] on [`EhrbaseService`] — the DEMOGRAPHIC group's
//! PARTY / `VERSIONED_PARTY` / contribution / tags surface.
//!
//! TODO(port): the demographic service implementation lands with this seam
//! (parties on the shared vobject machinery, no EHR scope); until each method
//! is overridden the trait defaults answer 501.

use async_trait::async_trait;

use ehrbase_rest::DemographicService;

use crate::service::EhrbaseService;

#[async_trait]
impl DemographicService for EhrbaseService {}
