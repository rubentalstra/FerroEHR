// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: BUSL-1.1 AND Apache-2.0

//! Hand-written runtime for canonical-XML (de)serialization (ITS-XML).
//!
//! The generated code (`emit-xml`, in `generated/`) implements [`ToXml`] /
//! `FromXml` for the RM/BASE spec types; this module is the trait definitions,
//! the `quick-xml` reader/writer helpers, and the primitive/leaf impls those
//! generated impls call into. openEHR XML is order-sensitive and uses
//! `xsi:type` attribute dispatch for polymorphic slots, which serde + quick-xml
//! cannot express — hence explicit generated impls over this runtime rather than
//! a serde derive.

#![expect(
    clippy::disallowed_types,
    reason = "the BMM `Any` monomorphization (`X_VERSIONED_OBJECT<Any>` on \
              `OPENEHR_CONTENT_ITEM.item`) renders as serde_json::Value in the generated RM \
              model, so this runtime must implement the codec traits for it (#1694)"
)]

use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

/// The `xsi` namespace, declared on every serialized root element.
pub const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// The two openEHR ITS-XML wire lineages.
///
/// Both bundles are vendored under `schemas/xml/` and merged into one
/// emission closure by `emit-xml`; they differ only in the root namespace a
/// document declares (`docs/specs/openehr/ITS-XML/README.adoc` §"Releases and
/// IM Versions").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    /// `http://schemas.openehr.org/v1` — the `Release-1.0.2v2` bundle, the
    /// RELEASED-STABLE lineage upstream directs stable consumers to. Every
    /// caller selects a namespace explicitly (no `Default` exists); the
    /// template/archetype codecs pin V1, while the served RM wire defaults to
    /// V2, because only the v2 bundle's schemas model the RM 1.2.0 the server
    /// emits.
    V1,
    /// `http://schemas.openehr.org/v2` — the `Release-2.0.0v2` bundle, TRIAL
    /// upstream ("These schemas are in *TRIAL* state and subject to change").
    V2,
}

impl Namespace {
    /// The namespace URI this lineage declares on the document root.
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
    /// The `quick-xml` writer rejected an event.
    #[error("xml write error: {0}")]
    Write(#[from] quick_xml::Error),
    /// The underlying byte sink or source failed.
    #[error("xml io error: {0}")]
    Io(#[from] std::io::Error),
    /// The written bytes were not valid UTF-8.
    #[error("xml output was not valid utf-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// The input was not well-formed, or did not match the expected canonical
    /// shape. Carries the diagnosis and nothing beneath it: a shape violation
    /// has no underlying failure.
    #[error("xml parse error: {0}")]
    Parse(String),
    /// A reader, decoder, or lexical conversion failed underneath the parse.
    ///
    /// The cause is carried as [`std::error::Error::source`] (RFC 0201) so a
    /// caller can walk or match it. It is BOXED deliberately: the sources are
    /// several unrelated types (`quick-xml`'s reader/escape/attribute errors, a
    /// UTF-8 or encoding failure, a `FromStr` failure, a UUID parse failure),
    /// and naming any of them here would make that dependency's own version
    /// part of this crate's public API.
    #[error("xml parse error: {message}")]
    ParseSource {
        /// What the reader was doing when the cause fired.
        message: String,
        /// The underlying failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl XmlError {
    /// A parse failure that carries its underlying cause.
    ///
    /// `message` says what was being read; the cause stays reachable through
    /// [`std::error::Error::source`] rather than being flattened into the text.
    pub fn parse_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ParseSource {
            message: message.into(),
            source: Box::new(source),
        }
    }
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
    /// A writer over a fresh in-memory buffer, with no namespace pending.
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

    /// Write a bare character-data node (XML-escaped by quick-xml).
    ///
    /// The mixed-content half of [`XmlAny`]: an element that carries both text
    /// and children cannot go through [`XmlWriter::write_text_element`].
    ///
    /// # Errors
    /// Propagates the underlying writer error.
    pub fn write_text(&mut self, text: &str) -> Result<(), XmlError> {
        self.w.write_event(Event::Text(BytesText::new(text)))?;
        Ok(())
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

/// Serializes a value as canonical XML under a statically-declared root type.
///
/// This is the `declared`-aware sibling of [`to_xml`], for a published global
/// element whose XSD type is abstract.
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

/// The `xs:double` lexical form of an openEHR `Real`.
///
/// The vendored XSDs type every `Real`-valued element `xs:double`
/// (`crates/openehr-its/schemas/xml/its-xml-1.0.2-nsv1/ALL/BaseTypes.xsd`
/// — e.g. `DV_QUANTITY/magnitude`), whose lexical space is defined by XML
/// Schema Part 2 §3.2.5 (<https://www.w3.org/TR/xmlschema-2/#double>):
///
/// * finite values are the `decimal`/scientific forms — the mantissa carries a
///   decimal point, which is why a whole Real is written `120.0` and not `120`
///   (Rust's `f64` `Display` drops the point);
/// * the three special values have EXACTLY the spellings `INF`, `-INF` and
///   `NaN` — `inf`/`-inf`/`NAN`, which `f64::to_string` produces, are not in
///   the lexical space at all.
///
/// NOTE: the non-finite spellings are unreachable from canonical JSON
/// (RFC 8259 admits no infinity/NaN literal) but reachable from a Rust-built
/// RM value, where `f64::to_string` would emit a schema-invalid document —
/// corrected here to the XSD spellings, which `f64::from_str` already parses
/// case-insensitively
/// (<https://doc.rust-lang.org/std/primitive.f64.html#method.from_str>).
fn xsd_double_lexical(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v.is_sign_negative() { "-INF" } else { "INF" }.to_owned();
    }
    if v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        v.to_string()
    }
}

impl ToXml for f64 {
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        w.write_text_element(tag, &xsd_double_lexical(*self))
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
    // SCOPE: the BMM-`Any` monomorphization the RM model keeps untyped —
    // `OPENEHR_CONTENT_ITEM.item: X_VERSIONED_OBJECT<Any>`. It is not a
    // concrete openEHR type and has no spec-defined canonical-XML shape, so
    // the JSON value is emitted as element text rather than guessing one. The
    // schema-declared open slots (`xs:anyType`) do NOT come here — they carry
    // their subtree in [`XmlAny`].
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        w.write_text_element(tag, &self.to_string())
    }
}

/// An arbitrary XML element subtree, held verbatim for an `xs:anyType` slot.
///
/// The XSD-driven archetype codecs (`opt14`, `aom2`, `aom2_model`) have slots
/// whose content model the schema leaves open, and the model behind them leaves
/// it open too: `EXPR_LEAF.item` is typed `Any` — "a manifest constant, an
/// attribute path (in the form of a `String`), or for the right-hand side of a
/// 'matches' node, a constraint, often a `C_PRIMITIVE_OBJECT`"
/// (`AM aom14 §EXPR_LEAF Class`). There is no closed set of payload types to
/// dispatch to, so the codec keeps the element as it read it: attributes
/// (`xsi:type` among them), character data, and child elements in document
/// order. `ToXml` writes the subtree back, so an `xs:anyType` payload survives
/// a parse → serialize → parse cycle unchanged.
///
/// Element names arrive with any namespace prefix stripped, as everywhere else
/// in this reader; attribute keys keep theirs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XmlAny {
    attrs: Vec<(String, String)>,
    content: Vec<XmlAnyNode>,
}

/// One node of an [`XmlAny`] element's content, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlAnyNode {
    /// Character data (adjacent runs are coalesced on read).
    Text(String),
    /// A child element: its name, then its own subtree.
    Element(String, XmlAny),
}

impl XmlAny {
    /// The element's attributes as `(name, value)` pairs, in document order.
    #[must_use]
    pub fn attrs(&self) -> &[(String, String)] {
        &self.attrs
    }

    /// The value of attribute `key`, if present.
    #[must_use]
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The `xsi:type` discriminator, if present, with any namespace prefix on
    /// the *value* stripped (`xsd:string` → `string`, `C_STRING` → `C_STRING`).
    ///
    /// The raw spelling stays reachable through [`XmlAny::attr`].
    #[must_use]
    pub fn xsi_type(&self) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == "xsi:type" || (k.ends_with(":type") && k.contains("xsi")))
            .map(|(_, v)| v.rsplit(':').next().unwrap_or(v))
    }

    /// The element's content nodes, in document order.
    #[must_use]
    pub fn content(&self) -> &[XmlAnyNode] {
        &self.content
    }

    /// The child elements, as `(name, subtree)` pairs in document order.
    pub fn children(&self) -> impl Iterator<Item = (&str, &XmlAny)> {
        self.content.iter().filter_map(|n| match n {
            XmlAnyNode::Element(name, child) => Some((name.as_str(), child)),
            XmlAnyNode::Text(_) => None,
        })
    }

    /// The child elements named `name`, in document order.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlAny> {
        self.children()
            .filter_map(move |(n, c)| (n == name).then_some(c))
    }

    /// The first child element named `name`.
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&XmlAny> {
        self.children().find(|(n, _)| *n == name).map(|(_, c)| c)
    }

    /// The element's own character data, with child elements' text excluded.
    #[must_use]
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|n| match n {
                XmlAnyNode::Text(t) => Some(t.as_str()),
                XmlAnyNode::Element(_, _) => None,
            })
            .collect()
    }

    /// Whether the element carries neither attributes nor content.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty() && self.content.is_empty()
    }
}

impl ToXml for openehr_base::serde_support::OpenSubtype {
    // NOTE: canonical XML defines element mappings per published class only;
    // no openEHR spec maps a scheme-defined subtype's members onto XML — a
    // member-carrying instance is refused rather than given an invented shape.
    fn write_xml(
        &self,
        w: &mut XmlWriter,
        tag: &str,
        declared: Option<&str>,
    ) -> Result<(), XmlError> {
        if !self.members().is_empty() {
            return Err(XmlError::Parse(format!(
                "no canonical-XML mapping exists for the scheme-defined `{}` members",
                self.type_name()
            )));
        }
        let mut e = BytesStart::new(tag);
        if declared != Some(self.type_name()) {
            e.push_attribute(("xsi:type", self.type_name()));
        }
        w.write_start(e)?;
        w.write_end(tag)
    }
}

impl FromXml for openehr_base::serde_support::OpenSubtype {
    fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError> {
        let type_name = start
            .attrs
            .iter()
            .find(|(k, _)| k == "xsi:type" || (k.ends_with(":type") && k.contains("xsi")))
            .map_or("ACCESS_CONTROL_SETTINGS", |(_, v)| {
                v.rsplit(':').next().unwrap_or(v)
            })
            .to_owned();
        loop {
            match reader.read()? {
                XmlEvent::Start(_) => {
                    return Err(XmlError::Parse(format!(
                        "no canonical-XML mapping exists for the scheme-defined `{type_name}` members"
                    )));
                }
                XmlEvent::Text(t) if !t.trim().is_empty() => {
                    return Err(XmlError::Parse(format!(
                        "no canonical-XML mapping exists for scheme-defined `{type_name}` content"
                    )));
                }
                XmlEvent::Text(_) => {}
                XmlEvent::End => break,
                XmlEvent::Eof => {
                    return Err(XmlError::Parse(
                        "unexpected EOF in an open-subtype element".into(),
                    ));
                }
            }
        }
        openehr_base::serde_support::OpenSubtype::new(type_name, serde_json::Map::new())
            .map_err(|e| XmlError::parse_source("constructing the open-subtype value", e))
    }
}

impl ToXml for XmlAny {
    fn write_xml(&self, w: &mut XmlWriter, tag: &str, _d: Option<&str>) -> Result<(), XmlError> {
        let mut e = BytesStart::new(tag);
        for (k, v) in &self.attrs {
            e.push_attribute((k.as_str(), v.as_str()));
        }
        w.write_start(e)?;
        for node in &self.content {
            match node {
                XmlAnyNode::Text(t) => w.write_text(t)?,
                XmlAnyNode::Element(name, child) => child.write_xml(w, name, None)?,
            }
        }
        w.write_end(tag)
    }
}

// ── deserialization (FromXml) ─────────────────────────────────────────────────

/// An owned start tag (element name + attributes), decoupled from the borrowed
/// reader so it can cross recursive `from_xml` calls.
#[derive(Debug, Clone)]
pub struct StartTag {
    /// The element name exactly as it appeared, prefix included.
    pub name: String,
    /// The element's attributes as `(name, value)` pairs, in document order.
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
    /// An element start tag (empty elements are expanded to `Start` + `End`).
    Start(StartTag),
    /// An element end tag.
    End,
    /// Character data.
    Text(String),
    /// End of input.
    Eof,
}

/// Reads canonical openEHR XML, yielding owned [`XmlEvent`]s. Empty elements are
/// expanded to Start+End so callers only handle those four cases.
pub struct XmlReader<'a> {
    r: Reader<&'a [u8]>,
    /// Current element nesting depth, so a document cannot recurse the reader's
    /// consumers off the stack. See [`MAX_DEPTH`].
    depth: u32,
}

/// The deepest element nesting a document may reach.
///
/// This is a memory-safety bound, not a style preference. The generated
/// `FromXml` impls descend one Rust stack frame per nesting level, and the RM is
/// genuinely recursive — `CLUSTER.items` holds `Item`, which includes `CLUSTER`;
/// `FOLDER` holds folders; `SECTION` holds sections — so a document of nested
/// `<items xsi:type="CLUSTER">` elements recurses without bound. Depth was
/// otherwise limited only by the accepted body size, which admits hundreds of
/// thousands of levels: far past any thread stack.
///
/// A stack overflow in Rust is a guard-page fault that **aborts the process**.
/// It is not an unwind, so `std::panic::catch_unwind` — and therefore the
/// `tower-http` catch-panic layer this server relies on for its clean `500` —
/// cannot intercept it. One request would take the process down for every
/// caller. That is what makes a bound obligatory rather than defensive.
///
/// 256 is chosen against the model rather than picked: the deepest structure the
/// RM composes — COMPOSITION → CONTENT_ITEM → SECTION → ENTRY → ITEM_STRUCTURE →
/// CLUSTER → ELEMENT → DATA_VALUE, plus nested clusters — is tens of levels in
/// the most elaborate real templates, and the canonical-JSON reader's own
/// equivalent bound (`serde_json`'s 128) sits in the same order of magnitude.
///
/// No openEHR spec bounds document nesting — our own design.
pub const MAX_DEPTH: u32 = 256;

impl std::fmt::Debug for XmlReader<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XmlReader").finish_non_exhaustive()
    }
}

impl<'a> XmlReader<'a> {
    /// A reader over `xml`, configured to expand empty elements.
    #[must_use]
    pub fn new(xml: &'a str) -> Self {
        let mut r = Reader::from_str(xml);
        r.config_mut().expand_empty_elements = true;
        Self { r, depth: 0 }
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
                .map_err(|e| XmlError::parse_source("reading the next XML event", e))?;
            match ev {
                Event::Start(e) => {
                    self.depth = self.depth.saturating_add(1);
                    if self.depth > MAX_DEPTH {
                        return Err(XmlError::Parse(format!(
                            "element nesting exceeds the {MAX_DEPTH}-level limit"
                        )));
                    }
                    return Ok(XmlEvent::Start(to_start_tag(&e)?));
                }
                Event::End(_) => {
                    self.depth = self.depth.saturating_sub(1);
                    return Ok(XmlEvent::End);
                }
                Event::Text(t) => {
                    // quick-xml 0.42 constructs events as validated UTF-8
                    // `&str` (a non-UTF-8 document fails at `read_event`), so
                    // only the entity unescape remains on this side.
                    let s = quick_xml::escape::unescape(t.as_ref())
                        .map_err(|e| XmlError::parse_source("unescaping element text", e))?;
                    return Ok(XmlEvent::Text(s.into_owned()));
                }
                Event::Eof => return Ok(XmlEvent::Eof),
                // An entity reference in text (`&apos;`, `&#39;`) arrives as a
                // separate event in quick-xml 0.41 — resolve it to its text so
                // leaf accumulation keeps it.
                Event::GeneralRef(e) => {
                    if let Some(c) = e
                        .resolve_char_ref()
                        .map_err(|e| XmlError::parse_source("resolving a character reference", e))?
                    {
                        return Ok(XmlEvent::Text(c.to_string()));
                    }
                    let name: &str = e.as_ref();
                    let resolved = match name {
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
                // A DOCTYPE is REFUSED, not skipped. It is inert today only
                // because quick-xml parses no DTDs — a property of a dependency's
                // current behaviour, which is the kind that changes silently.
                // Canonical openEHR XML has no use for one.
                Event::DocType(_) => {
                    return Err(XmlError::Parse(
                        "a DOCTYPE declaration is not accepted".into(),
                    ));
                }
                // Decl / Comment / PI / CData: skip.
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
    let raw = qname.as_ref();
    let name = raw.rsplit(':').next().unwrap_or(raw).to_string();
    let mut attrs = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|e| XmlError::parse_source("reading a start-tag attribute", e))?;
        let k = a.key.as_ref().to_owned();
        let v = quick_xml::escape::unescape(a.value.as_ref())
            .map_err(|e| XmlError::parse_source("unescaping an attribute value", e))?
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
                s.trim().parse::<$t>().map_err(|e| {
                    XmlError::parse_source(
                        format!("{s:?} is not a valid {}", stringify!($t)),
                        e,
                    )
                })
            }
        }
    )*};
}
impl_from_xml_parse!(bool, i32, i64, u8, f32, f64, char);

impl FromXml for uuid::Uuid {
    fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError> {
        let s = String::from_xml(reader, start)?;
        uuid::Uuid::parse_str(s.trim())
            .map_err(|e| XmlError::parse_source(format!("{s:?} is not a valid UUID"), e))
    }
}

impl<T: FromXml> FromXml for Box<T> {
    fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError> {
        Ok(Box::new(T::from_xml(reader, start)?))
    }
}

impl FromXml for serde_json::Value {
    // SCOPE: mirror of the `ToXml` impl above — the RM's untyped BMM-`Any`
    // monomorphization, which has no canonical-XML shape to read back.
    fn from_xml(reader: &mut XmlReader, _start: &StartTag) -> Result<Self, XmlError> {
        reader.skip_element()?;
        Ok(serde_json::Value::Null)
    }
}

impl FromXml for XmlAny {
    fn from_xml(reader: &mut XmlReader, start: &StartTag) -> Result<Self, XmlError> {
        let mut any = XmlAny {
            attrs: start.attrs.clone(),
            content: Vec::new(),
        };
        loop {
            match reader.read()? {
                XmlEvent::Text(t) => match any.content.last_mut() {
                    // quick-xml splits text at an entity reference, so adjacent
                    // runs are joined to keep one node per character-data run.
                    Some(XmlAnyNode::Text(prev)) => prev.push_str(&t),
                    _ => any.content.push(XmlAnyNode::Text(t)),
                },
                XmlEvent::Start(child) => {
                    let sub = XmlAny::from_xml(reader, &child)?;
                    any.content.push(XmlAnyNode::Element(child.name, sub));
                }
                XmlEvent::End => break,
                XmlEvent::Eof => {
                    return Err(XmlError::Parse(
                        "unexpected EOF in xs:anyType element".into(),
                    ));
                }
            }
        }
        Ok(any)
    }
}

#[cfg(test)]
mod real_lexical_tests {
    use super::xsd_double_lexical;

    /// Every `Real` shape against the `xs:double` lexical space (XML Schema
    /// Part 2 §3.2.5), including the three special values a bare
    /// `f64::to_string` spells wrongly.
    #[test]
    fn real_values_take_the_xsd_double_lexical_form() {
        // Whole reals keep the decimal point openEHR writes.
        assert_eq!(xsd_double_lexical(120.0), "120.0");
        assert_eq!(xsd_double_lexical(0.0), "0.0");
        assert_eq!(xsd_double_lexical(-0.0), "-0.0");
        assert_eq!(xsd_double_lexical(-7.0), "-7.0");
        // Fractional reals round-trip through the shortest form.
        assert_eq!(xsd_double_lexical(5.66), "5.66");
        assert_eq!(
            xsd_double_lexical(32.299_869_242_485_19),
            "32.29986924248519"
        );
        // The special values: `INF` / `-INF` / `NaN`, never Rust's spellings.
        assert_eq!(xsd_double_lexical(f64::INFINITY), "INF");
        assert_eq!(xsd_double_lexical(f64::NEG_INFINITY), "-INF");
        assert_eq!(xsd_double_lexical(f64::NAN), "NaN");
        assert_ne!(xsd_double_lexical(f64::INFINITY), f64::INFINITY.to_string());
    }

    /// The emitted lexeme must parse back to the same value — the fidelity
    /// property the round-trip gates rest on.
    #[test]
    fn every_emitted_lexeme_parses_back() {
        for v in [
            120.0_f64,
            -0.0,
            5.66,
            32.299_869_242_485_19,
            1e21,
            1e-7,
            f64::MAX,
            f64::MIN_POSITIVE,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let text = xsd_double_lexical(v);
            let back: f64 = text
                .parse()
                .unwrap_or_else(|e| panic!("{text:?} does not parse back: {e}"));
            assert_eq!(back.to_bits(), v.to_bits(), "{text:?}");
        }
        assert!(
            xsd_double_lexical(f64::NAN)
                .parse::<f64>()
                .is_ok_and(f64::is_nan)
        );
    }
}
