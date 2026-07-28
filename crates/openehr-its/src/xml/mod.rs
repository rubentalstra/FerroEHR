//! **ITS-XML** — canonical XML serialization (openEHR ITS-XML), in either of
//! the two published wire lineages: `http://schemas.openehr.org/v1`
//! (Release-1.0.2v2, the STABLE bundle) and `http://schemas.openehr.org/v2`
//! (Release-2.0.0v2, TRIAL upstream). One generated impl set serves both —
//! they differ only by the root `xmlns`, selected at serialize time.
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

/// Serialize an RM value to canonical openEHR XML in the **default** wire
/// lineage, namespace `http://schemas.openehr.org/v1`. `root_tag` is the root
/// element name (e.g. `"composition"`).
///
/// NOTE: the v1 default is the RELEASED-STABLE ITS-XML lineage. The upstream
/// repository README marks the 2.0.0 schemas *TRIAL* ("These schemas are in
/// *TRIAL* state and subject to change") and directs stable consumers to
/// `Release-1.0.2` — `docs/specs/openehr/ITS-XML/README.adoc` §"Releases and
/// IM Versions" — which is exactly what the single-pin-but-latest-RELEASED
/// policy asks a server to emit by default. Use [`to_canonical_xml_ns`] to
/// serialize the v2 lineage on request.
///
/// # Errors
/// Propagates serialization errors.
pub fn to_canonical_xml<T: ToXml + ?Sized>(value: &T, root_tag: &str) -> Result<String, XmlError> {
    to_canonical_xml_ns(value, root_tag, Namespace::V1)
}

/// Serialize an RM value to canonical openEHR XML in an explicitly chosen wire
/// lineage — the namespace-parameterized sibling of [`to_canonical_xml`].
///
/// Both lineages are vendored and validated (`schemas/xml/`), and the
/// generated codec is shared: `ns` selects only the root `xmlns` the document
/// declares (`runtime::Namespace`). Callers that have no reason to choose stay
/// on [`to_canonical_xml`].
///
/// NOTE: OPT 1.4 operational templates are NOT served through here — they are
/// AM documents with their own `<template>` root and are always emitted in the
/// v1 lineage (`crate::opt14::to_xml`), never negotiated: the lineage split
/// documented in `docs/specs/openehr/ITS-XML/README.adoc` §"Releases and IM
/// Versions" keeps `Release-1.0.2` the STABLE bundle, and an operational
/// template is not an RM canonical document a client reads back in a chosen
/// representation.
///
/// # Errors
/// Propagates serialization errors.
pub fn to_canonical_xml_ns<T: ToXml + ?Sized>(
    value: &T,
    root_tag: &str,
    ns: Namespace,
) -> Result<String, XmlError> {
    to_xml(value, root_tag, ns)
}

/// Deserialize an RM value from a canonical openEHR XML document.
///
/// Reading is namespace-agnostic by construction: the reader dispatches on
/// local element names and `xsi:type`, and never inspects the document's root
/// `xmlns`. Both published lineages therefore parse identically — which is
/// sound because the 2.0.0 restructure changed the namespace and nothing else
/// about the document shape (`docs/specs/openehr/ITS-XML/README.adoc`
/// §"Releases and IM Versions": "Simultaneously, the internal namespace used
/// in the schemas is also changed to `http://schemas.openehr.org/v2`").
///
/// # Errors
/// Propagates parse errors.
pub fn from_canonical_xml<T: FromXml>(xml: &str) -> Result<T, XmlError> {
    from_xml(xml)
}
