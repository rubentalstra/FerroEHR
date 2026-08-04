//! **ITS-XML** — canonical XML serialization (openEHR ITS-XML), in either of
//! the two published wire lineages: `http://schemas.openehr.org/v1`
//! (Release-1.0.2v2, the STABLE bundle) and `http://schemas.openehr.org/v2`
//! (Release-2.0.0v2, TRIAL upstream). One generated impl set serves both: the
//! documents it writes differ only by the root `xmlns`, selected at serialize
//! time. The two vendored SCHEMA bundles are not equivalent, though — the v1
//! bundle is frozen at an older RM generation (`schemas/xml/PROVENANCE.md`
//! §"Two lineages"), so a served v1 document can carry RM 1.2.0 members its
//! own schema predates.
//!
//! The `ToXml`/`FromXml` impls for the RM/BASE spec types are **generated** by
//! `openehr-codegen`'s `emit-xml` target into `generated`, driven by
//! the vendored XSDs (`schemas/xml/`) + the BMM field model. This module is the
//! hand-written [`runtime`] (traits + `quick-xml` writer/reader) and the public
//! entry points. Regenerate with `cargo run -p openehr-codegen -- emit-xml`.

pub mod runtime;

// Trait impls for the spec types — compiled for their effect; nothing to export.
mod generated;

pub use runtime::{
    FromXml, Namespace, StartTag, ToXml, XmlError, XmlEvent, XmlReader, XmlWriter, from_xml,
    to_xml, to_xml_declared,
};

/// Serialize an RM value to canonical openEHR XML in the **default** wire
/// lineage, namespace `http://schemas.openehr.org/v1`. `root_tag` is the root
/// element name (e.g. `"composition"`).
///
/// NOTE: this LIBRARY default is the RELEASED-STABLE ITS-XML lineage (the
/// upstream repository README marks the 2.0.0 schemas *TRIAL* and directs
/// stable consumers to `Release-1.0.2` —
/// `docs/specs/openehr/ITS-XML/README.adoc` §"Releases and IM Versions").
/// The REST layer chooses its lineage explicitly per request via
/// [`to_canonical_xml_ns`] and SERVES v2 by default (owner ruling 2026-08-03,
/// #1666: the v1 bundle cannot describe the RM 1.2.0 the server emits —
/// register AMB-185); this function is the fixed-lineage convenience for
/// gates and tools, not the wire default.
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

/// Serialize an RM value under a root element whose **XSD-declared type is
/// abstract**, so the instance must name its concrete type — the
/// declared-type-aware sibling of [`to_canonical_xml_ns`].
///
/// [`to_canonical_xml`] passes no declared type at the root, so its root
/// element never carries `xsi:type`. That is right for a published global
/// element whose type is concrete (`<composition>`, `<ehr_status>`, …) and
/// WRONG for one whose type is abstract: `ALL/Version.xsd` publishes
/// `<xs:element name="version" type="VERSION"/>` over
/// `<xs:complexType name="VERSION" abstract="true">`, and XML Schema Part 1
/// forbids an element instance from using an abstract type directly — the
/// instance must select a non-abstract derived type with `xsi:type`
/// (<https://www.w3.org/TR/xmlschema-1/#xsi_type>, §2.6.1 + §3.4.6). Passing
/// `declared_type = "VERSION"` here makes an `ORIGINAL_VERSION` emit
/// `xsi:type="ORIGINAL_VERSION"` through the very same polymorphic-dispatch
/// mechanism every nested slot uses, so the document validates against the
/// published schema.
///
/// # Errors
/// Propagates serialization errors.
pub fn to_canonical_xml_declared<T: ToXml + ?Sized>(
    value: &T,
    root_tag: &str,
    declared_type: &str,
    ns: Namespace,
) -> Result<String, XmlError> {
    to_xml_declared(value, root_tag, declared_type, ns)
}

/// Deserialize an RM value from a canonical openEHR XML document.
///
/// Reading is namespace-agnostic by construction: the reader dispatches on
/// local element names and `xsi:type`, and never inspects the document's root
/// `xmlns`. Both published lineages therefore parse identically. The 2.0.0
/// restructure changed the namespace the schemas declare
/// (`docs/specs/openehr/ITS-XML/README.adoc` §"Releases and IM Versions":
/// "Simultaneously, the internal namespace used in the schemas is also changed
/// to `http://schemas.openehr.org/v2`") — it did NOT make the two bundles
/// interchangeable: the flat `Release-1.0.2v2` bundle is frozen at an older RM
/// generation and declares neither `FOLDER.details` nor 22 other RM 1.2.0
/// attributes, nor 50 RM 1.2.0 classes at all
/// (`schemas/xml/PROVENANCE.md` §"Two lineages"). Namespace-agnostic READING
/// stays sound regardless, because every element name and `xsi:type` the
/// reader dispatches on is spelled identically in both lineages; only what the
/// two schemas ACCEPT differs.
///
/// # Errors
/// Propagates parse errors.
pub fn from_canonical_xml<T: FromXml>(xml: &str) -> Result<T, XmlError> {
    from_xml(xml)
}
