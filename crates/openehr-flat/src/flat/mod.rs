//! FLAT (simSDT) `RM ⇄ FLAT` conversion, driven by the [`WebTemplate`].
//!
//! Better's `web-template` (`converter/flat`) is the interop oracle: the flat
//! `path|suffix` keys, the `:i` repeat indexing (`isRepeating`), the per-type
//! suffix names (`|unit` singular, `|scale`/`|ordinal`, `|other`), and the
//! `ctx/…` context shortcuts match it.
//!
//! - [`to_flat`] — canonical-JSON `COMPOSITION` → flat map (`RawToFlat`).
//! - [`from_flat`] — flat map → canonical-JSON `COMPOSITION` (the composition
//!   builder), re-materialising the compacted RM structure so the result
//!   deserialises as an `openehr-rm` `Composition`.
//!
//! [`WebTemplate`]: crate::webtemplate::WebTemplate

mod context;
mod from_flat;
mod graph;
mod mappers;
mod sub;
mod to_flat;

pub use from_flat::from_flat;
pub use to_flat::to_flat;
