//! Hand-written runtime for canonical-XML (de)serialization (ITS-XML, ADR-005).
//!
//! The generated code (`emit-xml`, in `generated/`) implements [`ToXml`] /
//! `FromXml` for the RM/BASE spec types; this module is the trait definitions,
//! the `quick-xml` reader/writer helpers, and the primitive/leaf impls those
//! generated impls call into. openEHR XML is order-sensitive and uses
//! `xsi:type` attribute dispatch for polymorphic slots, which serde + quick-xml
//! cannot express — hence explicit generated impls over this runtime rather than
//! a serde derive.

use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

pub use quick_xml::events::BytesStart as XmlStart;

/// The `xsi` namespace, declared on every serialized root element.
pub const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// The two openEHR ITS-XML wire lineages (see `schemas/xml/PROVENANCE.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    /// `http://schemas.openehr.org/v1` — what stock EHRbase emits (parity target).
    V1,
    /// `http://schemas.openehr.org/v2` — latest openEHR ITS-XML.
    V2,
}

impl Namespace {
    #[must_use]
    pub const fn uri(self) -> &'static str {
        match self {
            Namespace::V1 => "http://schemas.openehr.org/v1",
            Namespace::V2 => "http://schemas.openehr.org/v2",
        }
    }
}

/// Errors from canonical-XML (de)serialization.
#[derive(Debug, thiserror::Error)]
pub enum XmlError {
    #[error("xml write error: {0}")]
    Write(#[from] quick_xml::Error),
    #[error("xml io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("xml output was not valid utf-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("xml parse error: {0}")]
    Parse(String),
}

/// Writes canonical openEHR XML. A thin wrapper over `quick_xml::Writer` that
/// injects the root namespace declarations on the first start tag.
pub struct XmlWriter {
    w: Writer<Vec<u8>>,
    /// Set before writing the root element; pushed as `xmlns`/`xmlns:xsi` onto
    /// the next start tag, then cleared.
    pending_root_ns: Option<Namespace>,
}

impl std::fmt::Debug for XmlWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XmlWriter")
            .field("pending_root_ns", &self.pending_root_ns)
            .finish_non_exhaustive()
    }
}

impl XmlWriter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            w: Writer::new(Vec::new()),
            pending_root_ns: None,
        }
    }

    /// Declare the default + `xsi` namespaces on the next (root) start tag.
    pub fn set_root_namespace(&mut self, ns: Namespace) {
        self.pending_root_ns = Some(ns);
    }

    /// Write a start tag built by generated code (attributes already pushed).
    ///
    /// # Errors
    /// Propagates the underlying writer error.
    pub fn write_start(&mut self, mut e: BytesStart<'_>) -> Result<(), XmlError> {
        if let Some(ns) = self.pending_root_ns.take() {
            e.push_attribute(("xmlns", ns.uri()));
            e.push_attribute(("xmlns:xsi", XSI_NS));
        }
        self.w.write_event(Event::Start(e))?;
        Ok(())
    }

    /// Write an end tag.
    ///
    /// # Errors
    /// Propagates the underlying writer error.
    pub fn write_end(&mut self, tag: &str) -> Result<(), XmlError> {
        self.w.write_event(Event::End(BytesEnd::new(tag)))?;
        Ok(())
    }

    /// Write `<tag>text</tag>` (text is XML-escaped by quick-xml).
    ///
    /// # Errors
    /// Propagates the underlying writer error.
    pub fn write_text_element(&mut self, tag: &str, text: &str) -> Result<(), XmlError> {
        // A leaf text element can still be the root (e.g. serializing a bare
        // String), so honour a pending namespace here too.
        let mut start = BytesStart::new(tag);
        if let Some(ns) = self.pending_root_ns.take() {
            start.push_attribute(("xmlns", ns.uri()));
            start.push_attribute(("xmlns:xsi", XSI_NS));
        }
        self.w.write_event(Event::Start(start))?;
        self.w.write_event(Event::Text(BytesText::new(text)))?;
        self.write_end(tag)
    }

    /// Consume and return the serialized XML.
    ///
    /// # Errors
    /// Fails only if the emitted bytes are not valid UTF-8 (they always are).
    pub fn into_string(self) -> Result<String, XmlError> {
        Ok(String::from_utf8(self.w.into_inner())?)
    }
}

impl Default for XmlWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize a value as an openEHR canonical-XML document.
///
/// `root_tag` is the root element name (e.g. `"composition"`); `ns` selects the
/// wire lineage. The root carries no `xsi:type`.
///
/// # Errors
/// Propagates serialization errors.
pub fn to_xml<T: ToXml + ?Sized>(
    value: &T,
    root_tag: &str,
    ns: Namespace,
) -> Result<String, XmlError> {
    let mut w = XmlWriter::new();
    w.set_root_namespace(ns);
    value.write_xml(&mut w, root_tag, None)?;
    w.into_string()
}

/// A value that serializes to canonical openEHR XML.
///
/// `write_xml` writes the complete `<tag …>…</tag>` element. `declared` is the
/// statically-declared spec type of the slot; a value whose concrete type
/// differs emits an `xsi:type` attribute (the polymorphic-dispatch mechanism).
pub trait ToXml {
    /// The concrete openEHR type name, or `""` for primitives (which never
    /// carry `xsi:type`).
    fn xml_type_name(&self) -> &'static str {
        ""
    }

    /// # Errors
    /// Propagates serialization errors.
    fn write_xml(
        &self,
        w: &mut XmlWriter,
        tag: &str,
        declared: Option<&str>,
    ) -> Result<(), XmlError>;
}

// ── primitive / leaf impls (so every generated field uniformly calls write_xml) ──

impl ToXml for String {
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        w.write_text_element(tag, self)
    }
}

impl ToXml for str {
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        w.write_text_element(tag, self)
    }
}

macro_rules! impl_to_xml_display {
    ($($t:ty),*) => {$(
        impl ToXml for $t {
            fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>)
                -> Result<(), XmlError>
            {
                w.write_text_element(tag, &self.to_string())
            }
        }
    )*};
}
impl_to_xml_display!(bool, i32, i64, u8, char, f32);

impl ToXml for f64 {
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        // openEHR emits a decimal point on whole reals (`120.0`, not `120`);
        // Rust's default `f64` Display drops it.
        // PERF(port): revisit against the fidelity corpus for exact number parity.
        let s = if self.fract() == 0.0 && self.is_finite() {
            format!("{self:.1}")
        } else {
            self.to_string()
        };
        w.write_text_element(tag, &s)
    }
}

impl<T: ToXml> ToXml for Box<T> {
    fn xml_type_name(&self) -> &'static str {
        (**self).xml_type_name()
    }
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, d: Option<&str>) -> Result<(), XmlError> {
        (**self).write_xml(w, tag, d)
    }
}

impl ToXml for uuid::Uuid {
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        w.write_text_element(tag, &self.to_string())
    }
}

impl ToXml for serde_json::Value {
    // TODO(port): the monomorphized version-family payloads (`X_VERSIONED_*.data`)
    // and BMM-`Any` slots carry an untyped JSON value; their canonical-XML shape
    // is not yet defined and they are off the RM composition parity path. Emit the
    // JSON text as a placeholder so the crate compiles.
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        w.write_text_element(tag, &self.to_string())
    }
}
