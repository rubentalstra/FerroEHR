//! FLAT (simSDT) `RM ⇄ FLAT` conversion, driven by the [`WebTemplate`].
//!
//! Better's `web-template` (`converter/flat`) is the interop oracle: the flat
//! `path|suffix` keys, the `:i` repeat indexing (`isRepeating`), the per-type
//! suffix names (`|unit` singular, `|scale`/`|ordinal`, `|other`), and the
//! `ctx/…` context shortcuts match it.
//!
//! **Why Better, not the SM spec (F-10-01/05):** the normative openEHR SDT
//! concrete-format text is unfinished — `SM/.../serial_data_formats/master05`
//! is `xxxx` and its `master04` String Parser is `TBD`, and
//! `ITS-REST/.../simplified_data_template/master05-jdt_concrete_formats.adoc` is
//! "under development; currently just notes". The one finished normative table
//! (`SM/.../serial_data_formats/master03-data_values.adoc`) lists the `|suffix`
//! object forms this converter implements as the *"`EhrScape` Variants"* and a
//! different string syntax (`"78.500,kg"`, `"1|[snomed_ct::…]"`, ODIN intervals)
//! as *primary*; we implement only the EhrScape/Better `|suffix` envelope,
//! because the CNF Robot fixtures and the ITS-REST examples all use it and no SM
//! string-parser grammar exists to conform to yet. Re-evaluate on any SM
//! `serial_data_formats` / `simplified_im_b` STABLE release.
//!
//! **FLAT round-trip scope:** the tested contract is `from_flat →
//! to_flat` stability of *FLAT-expressible* data, not full RM fidelity. RM
//! constructs with no web-template node are intentionally **not surfaced** on
//! `RM → FLAT` and are lost on that direction: `LINK`, `FEEDER_AUDIT`, non-root
//! `uid`, `DV_ORDERED.normal_range`/`other_reference_ranges`, `DV_TEXT.mappings`,
//! and inline `DV_MULTIMEDIA.data` (base64). This matches Better and is inherent
//! to the simplified format's "less self-standing" design
//! (`SM/.../master03-conceptual.adoc`).
//!
//! - [`to_flat`] — canonical-JSON `COMPOSITION` → flat map (`RawToFlat`).
//! - [`from_flat`] — flat map → canonical-JSON `COMPOSITION` (the composition
//!   builder), re-materialising the compacted RM structure so the result
//!   deserialises as an `openehr-rm` `Composition`.
//!
//! [`WebTemplate`]: crate::webtemplate::WebTemplate

mod context;
mod defaults;
mod from_flat;
mod graph;
mod mappers;
mod rmattr;
mod sub;
mod to_flat;

pub use from_flat::from_flat;
pub use to_flat::to_flat;
