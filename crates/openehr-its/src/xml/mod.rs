//! **ITS-XML** — canonical XML serialization (openEHR ITS-XML), namespaces
//! `http://schemas.openehr.org/v1` (parity target) and `…/v2` (latest).
//!
//! The `ToXml`/`FromXml` impls for the RM/BASE spec types are **generated** by
//! `openehr-codegen`'s `emit-xml` target into [`generated`], driven by
//! the vendored XSDs (`schemas/xml/`) + the BMM field model. This module is the
//! hand-written [`runtime`] (traits + `quick-xml` writer/reader) and the public
//! entry points. Regenerate with `cargo run -p openehr-codegen -- emit-xml`.

pub mod runtime;

// Trait impls for the spec types — compiled for their effect; nothing to export.
mod generated;

pub use runtime::{
    FromXml, Namespace, StartTag, ToXml, XmlError, XmlEvent, XmlReader, XmlWriter, from_xml, to_xml,
};

/// Serialize an RM value to canonical openEHR XML using the **v1** namespace —
/// the Stage-1 parity target (what stock EHRbase emits). `root_tag` is the root
/// element name (e.g. `"composition"`).
///
/// # Errors
/// Propagates serialization errors.
pub fn to_canonical_xml<T: ToXml + ?Sized>(value: &T, root_tag: &str) -> Result<String, XmlError> {
    to_xml(value, root_tag, Namespace::V1)
}

/// Deserialize an RM value from a canonical openEHR XML document (either
/// namespace lineage).
///
/// # Errors
/// Propagates parse errors.
pub fn from_canonical_xml<T: FromXml>(xml: &str) -> Result<T, XmlError> {
    from_xml(xml)
}
