//! The routed screens (§7A screen catalog in the design doc). One module
//! per screen; components stay thin — data flows through the `#[server]`
//! fns each module co-locates.

pub mod login;
pub mod shell;
pub mod system;
