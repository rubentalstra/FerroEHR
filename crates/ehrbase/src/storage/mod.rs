//! Greenfield node storage (ADR-008, P10): the codec between canonical
//! openEHR JSON and the decomposed `node` rows the schema stores.
//!
//! `decompose` turns a versioned object's canonical JSON into nested-set
//! numbered rows (structure children pruned out of their parents' fragments,
//! stored verbatim otherwise); `reassemble` is its lossless inverse. Storage
//! context (`vo_id`, `sys_version`, `ehr_id`) is the repository's concern
//! (P12) — the codec is a pure content transform.

mod codec;
mod error;

pub use codec::{NodeRow, decompose, is_structure_type, reassemble};
pub use error::StorageError;
