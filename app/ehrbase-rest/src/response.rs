//! The typed response envelope carried out of the service seam.
//!
//! Moved to `ehrbase-sm` (SM-1, ADR-010): [`ServiceResponse`] and
//! [`ResourceMeta`] now live in the `ehrbase-sm` crate's shared service types;
//! this module re-exports them for existing paths.

pub use ehrbase_sm::types::{ResourceMeta, ServiceResponse};
