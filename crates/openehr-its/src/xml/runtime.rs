//! Hand-written runtime for canonical-XML (de)serialization (ITS-XML).
//!
//! The generated code (`emit-xml`, in `generated/`) implements [`ToXml`] /
//! `FromXml` for the RM/BASE spec types; this module is the trait definitions,
//! the `quick-xml` reader/writer helpers, and the primitive/leaf impls those
//! generated impls call into. openEHR XML is order-sensitive and uses
//! `xsi:type` attribute dispatch for polymorphic slots, which serde + quick-xml
//! cannot express — hence explicit generated impls over this runtime rather than
//! a serde derive.

use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

pub use quick_xml::events::BytesStart as XmlStart;

/// The `xsi` namespace, declared on every serialized root element.
pub const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// The two openEHR ITS-XML wire lineages. Both bundles are vendored under
/// `schemas/xml/` and merged into one emission closure by `emit-xml`; they
/// differ only in the root namespace a document declares
/// (`docs/specs/openehr/ITS-XML/README.adoc` §"Releases and IM Versions").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    /// `http://schemas.openehr.org/v1` — the `Release-1.0.2v2` bundle, the
    /// RELEASED-STABLE lineage upstream directs stable consumers to, and this
    /// crate's serialization default.
    V1,
    /// `http://schemas.openehr.org/v2` — the `Release-2.0.0v2` bundle, TRIAL
    /// upstream ("These schemas are in *TRIAL* state and subject to change").
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

    /// Write `<tag id="id">text</tag>` — the openEHR `StringDictionaryItem`
    /// shape for one `Hash<String, String>` entry (`id` = key, text = value).
    ///
    /// # Errors
    /// Propagates the underlying writer error.
    pub fn write_kv_element(&mut self, tag: &str, id: &str, text: &str) -> Result<(), XmlError> {
        let mut start = BytesStart::new(tag);
        start.push_attribute(("id", id));
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

/// Serialize a value as an openEHR canonical-XML document whose ROOT element
/// carries a statically-declared type — the `declared`-aware sibling of
/// [`to_xml`], for a published global element whose XSD type is abstract.
///
/// The value emits `xsi:type` through the same polymorphic-dispatch mechanism
/// every nested slot uses, i.e. iff its concrete type differs from `declared`.
///
/// # Errors
/// Propagates serialization errors.
pub fn to_xml_declared<T: ToXml + ?Sized>(
    value: &T,
    root_tag: &str,
    declared: &str,
    ns: Namespace,
) -> Result<String, XmlError> {
    let mut w = XmlWriter::new();
    w.set_root_namespace(ns);
    value.write_xml(&mut w, root_tag, Some(declared))?;
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
        // TODO(perf): revisit against the fidelity corpus for exact number parity.
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
    // SCOPE: a `serde_json::Value` slot is a codegen
    // *monomorphization artifact* — the version-family payloads
    // (`X_VERSIONED_*.data`) and BMM-`Any` fields that the codegen deliberately
    // leaves untyped. These are not concrete openEHR types and have no
    // spec-defined canonical-XML shape, so there is nothing to serialize
    // faithfully; they never occur on the RM composition/EHR wire. The JSON
    // value is emitted as element text as a last resort rather than guessing a
    // shape. (Resolved if/when the codegen monomorphization is made precise.)
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        w.write_text_element(tag, &self.to_string())
    }
}

// ── deserialization (FromXml) ─────────────────────────────────────────────────

/// An owned start tag (element name + attributes), decoupled from the borrowed
/// reader so it can cross recursive `from_xml` calls.
#[derive(Debug, Clone)]
pub struct StartTag {
    pub name: String,
    pub attrs: Vec<(String, String)>,
}

impl StartTag {
    /// The value of attribute `key`, if present.
    #[must_use]
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The `xsi:type` discriminator, if present, with any namespace prefix on
    /// the *value* stripped (`v1:CLUSTER` → `CLUSTER`) so dispatch matches the
    /// bare openEHR type name regardless of the document's prefix convention.
    #[must_use]
    pub fn xsi_type(&self) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == "xsi:type" || (k.ends_with(":type") && k.contains("xsi")))
            .map(|(_, v)| v.rsplit(':').next().unwrap_or(v))
    }
}

/// One decoded XML event (owned).
#[derive(Debug)]
pub enum XmlEvent {
    Start(StartTag),
    End,
    Text(String),
    Eof,
}

/// Reads canonical openEHR XML, yielding owned [`XmlEvent`]s. Empty elements are
/// expanded to Start+End so callers only handle those four cases.
pub struct XmlReader<'a> {
    r: Reader<&'a [u8]>,
}

impl std::fmt::Debug for XmlReader<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XmlReader").finish_non_exhaustive()
    }
}

impl<'a> XmlReader<'a> {
    #[must_use]
    pub fn new(xml: &'a str) -> Self {
        let mut r = Reader::from_str(xml);
        r.config_mut().expand_empty_elements = true;
        Self { r }
    }

    /// Read the next meaningful event.
    ///
    /// # Errors
    /// Propagates parse errors.
    pub fn read(&mut self) -> Result<XmlEvent, XmlError> {
        loop {
            let ev = self
                .r
                .read_event()
                .map_err(|e| XmlError::Parse(e.to_string()))?;
            match ev {
                Event::Start(e) => return Ok(XmlEvent::Start(to_start_tag(&e)?)),
                Event::End(_) => return Ok(XmlEvent::End),
                Event::Text(t) => {
                    let raw = t.decode().map_err(|e| XmlError::Parse(e.to_string()))?;
                    let s = quick_xml::escape::unescape(&raw)
                        .map_err(|e| XmlError::Parse(e.to_string()))?;
                    return Ok(XmlEvent::Text(s.into_owned()));
                }
                Event::Eof => return Ok(XmlEvent::Eof),
                // An entity reference in text (`&apos;`, `&#39;`) arrives as a
                // separate event in quick-xml 0.41 — resolve it to its text so
                // leaf accumulation keeps it.
                Event::GeneralRef(e) => {
                    if let Some(c) = e
                        .resolve_char_ref()
                        .map_err(|e| XmlError::Parse(e.to_string()))?
                    {
                        return Ok(XmlEvent::Text(c.to_string()));
                    }
                    let name = e.decode().map_err(|e| XmlError::Parse(e.to_string()))?;
                    let resolved = match name.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "apos" => "'",
                        "quot" => "\"",
                        other => {
                            return Err(XmlError::Parse(format!("unknown entity &{other};")));
                        }
                    };
                    return Ok(XmlEvent::Text(resolved.to_string()));
                }
                // Decl / Comment / PI / CData / DocType: skip.
                _ => {}
            }
        }
    }

    /// Consume the rest of the current element's subtree (its start already read).
    ///
    /// # Errors
    /// Propagates parse errors; errors on premature EOF.
    pub fn skip_element(&mut self) -> Result<(), XmlError> {
        let mut depth = 1i32;
        while depth > 0 {
            match self.read()? {
                XmlEvent::Start(_) => depth += 1,
                XmlEvent::End => depth -= 1,
                XmlEvent::Eof => return Err(XmlError::Parse("unexpected EOF in element".into())),
                XmlEvent::Text(_) => {}
            }
        }
        Ok(())
    }
}

fn to_start_tag(e: &BytesStart<'_>) -> Result<StartTag, XmlError> {
    // Strip any namespace prefix on the *element* name (`ns2:language` →
    // `language`) so name-based child dispatch matches regardless of a
    // document's prefix convention (some OPT exports qualify every element).
    // Attribute keys are left intact, since `xsi:type` dispatch keys on the
    // `xsi:` prefix. A default-namespace (unprefixed) name is unaffected.
    let qname = e.name();
    let raw = String::from_utf8_lossy(qname.as_ref());
    let name = raw.rsplit(':').next().unwrap_or(&raw).to_string();
    let mut attrs = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|e| XmlError::Parse(e.to_string()))?;
        let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let raw = std::str::from_utf8(&a.value).map_err(|e| XmlError::Parse(e.to_string()))?;
        let v = quick_xml::escape::unescape(raw)
            .map_err(|e| XmlError::Parse(e.to_string()))?
            .into_owned();
        attrs.push((k, v));
    }
    Ok(StartTag { name, attrs })
}

/// Deserialize a value from an openEHR canonical-XML document.
///
/// # Errors
/// Propagates parse errors.
pub fn from_xml<T: FromXml>(xml: &str) -> Result<T, XmlError> {
    let mut reader = XmlReader::new(xml);
    loop {
        match reader.read()? {
            XmlEvent::Start(s) => return T::from_xml(&mut reader, &s),
            XmlEvent::Eof => return Err(XmlError::Parse("no root element".into())),
            _ => {}
        }
    }
}

/// A value that deserializes from canonical openEHR XML. `from_xml` is called
/// after the element's start tag has been read (`start`); it consumes events
/// through the matching end tag.
pub trait FromXml: Sized {
    /// # Errors
    /// Propagates parse errors.
    fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError>;
}

impl FromXml for String {
    fn from_xml(reader: &mut XmlReader, _start: &StartTag) -> Result<Self, XmlError> {
        let mut text = String::new();
        loop {
            match reader.read()? {
                XmlEvent::Text(t) => text.push_str(&t),
                XmlEvent::End => break,
                XmlEvent::Start(_) => reader.skip_element()?, // stray child in a text leaf
                XmlEvent::Eof => return Err(XmlError::Parse("EOF in text element".into())),
            }
        }
        Ok(text)
    }
}

macro_rules! impl_from_xml_parse {
    ($($t:ty),*) => {$(
        impl FromXml for $t {
            fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError> {
                let s = String::from_xml(reader, start)?;
                s.trim().parse::<$t>().map_err(|e| XmlError::Parse(format!("{e}: {s:?}")))
            }
        }
    )*};
}
impl_from_xml_parse!(bool, i32, i64, u8, f32, f64, char);

impl FromXml for uuid::Uuid {
    fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError> {
        let s = String::from_xml(reader, start)?;
        uuid::Uuid::parse_str(s.trim()).map_err(|e| XmlError::Parse(e.to_string()))
    }
}

impl<T: FromXml> FromXml for Box<T> {
    fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError> {
        Ok(Box::new(T::from_xml(reader, start)?))
    }
}

impl FromXml for serde_json::Value {
    // SCOPE: mirror of the `ToXml` impl above — an untyped codegen
    // monomorphization artifact with no spec canonical-XML shape, never on the
    // RM composition/EHR wire. Consume the element and yield Null.
    fn from_xml(reader: &mut XmlReader, _start: &StartTag) -> Result<Self, XmlError> {
        reader.skip_element()?;
        Ok(serde_json::Value::Null)
    }
}
