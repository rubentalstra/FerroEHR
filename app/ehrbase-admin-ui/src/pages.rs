//! The routed screens. One module
//! per screen; components stay thin — data flows through the `#[server]`
//! fns each module co-locates.

pub mod composition;
pub mod ehr_detail;
pub mod ehrs;
pub mod login;
pub mod shell;
pub mod system;
pub mod template_detail;
pub mod templates;
