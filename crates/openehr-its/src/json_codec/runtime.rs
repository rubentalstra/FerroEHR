//! Hand-written runtime for canonical-JSON (ITS-JSON) — both directions.
//!
//! The generated code (`emit-json`, in `generated/`) implements [`ToJson`] and
//! [`FromJson`] for the RM/BASE/AM/TERM/LANG spec types; this module is the two
//! trait definitions, the [`JsonWriter`], the hand-written JSON reader
//! ([`JsonNode`] + a borrowing tokenizer), and the primitive/leaf impls those
//! generated impls call into. It mirrors the shape of the canonical-XML runtime
//! ([`crate::xml::runtime`]): a writer/reader pair plus per-type impls, generated
//! code on top.
//!
//! The reader has two backends behind one [`JsonNode`] trait: a borrowing
//! tokenizer over `&str` (the [`from_json_str`] entry point — no `serde_json`
//! on the parse path) and a `&serde_json::Value` walker (the [`from_json_value`]
//! entry point, for the validation tier that already holds `jsonb`-sourced
//! `Value`s). Generated [`FromJson`] impls are generic over the backend and read
//! each known field by key — so unknown wire keys are ignored, fields may arrive
//! out of order, and a polymorphic slot dispatches on its `_type` member without
//! materializing a second value (the serde-`Value` enum double-pass is gone).
//!
//! Output contract (the R0 canonical-output contract, pinned by
//! `tests/canonical_contract.rs`): `_type` is the first member of every object,
//! followed by the fields in BMM declaration order; `None` and empty-`Vec`
//! fields are omitted; integer-typed RM fields print as JSON integers and
//! Real-typed fields carry a decimal point (whole reals as `x.0`). Integers use
//! `itoa` — the exact integer formatter serde_json uses, so integer output is
//! byte-identical; reals use `ryu`, a self-contained formatter (no serde
//! dependency on the number path). String escaping is serde_json's exact escape
//! set (RFC 8259 §7).
//!
//! NOTE: no openEHR spec governs the REAL lexeme — `ryu`'s shortest round-trip
//! form is our own chosen canonical rule. It matches serde_json byte-for-byte on
//! every real in the vendored corpus (proven by `tests/json_codec_parity.rs`);
//! the two formatters differ only on rarely-seen exponent forms serde_json's
//! vendored dtoa writes with a signed exponent (`1e+21`) where ryu writes `1e21`
//! — a deliberate, documented divergence, not a defect.

/// The serde_json string-escape table, byte-indexed. A non-zero entry is the
/// escape selector for that byte (`b'"'`/`b'\\'`/`b'b'`/`b'f'`/`b'n'`/`b'r'`/
/// `b't'`, or `b'u'` for a `\u00XX` control escape); `0` means the byte is
/// emitted verbatim. Reproduces `serde_json`'s `ESCAPE` table: only the C0
/// control range (`0x00..=0x1F`), `"` (`0x22`), and `\` (`0x5C`) are escaped —
/// `/` and every non-ASCII byte pass through unchanged (RFC 8259 §7).
static ESCAPE: [u8; 256] = build_escape_table();

const fn build_escape_table() -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut i = 0;
    while i < 0x20 {
        t[i] = b'u'; // generic `\u00XX` control escape
        i += 1;
    }
    t[0x08] = b'b'; // backspace
    t[0x09] = b't'; // tab
    t[0x0A] = b'n'; // line feed
    t[0x0C] = b'f'; // form feed
    t[0x0D] = b'r'; // carriage return
    t[b'"' as usize] = b'"';
    t[b'\\' as usize] = b'\\';
    t
}

/// Lowercase hex digits for `\u00XX` control escapes (matching serde_json).
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Writes canonical openEHR JSON into an owned `String`.
///
/// The generated [`ToJson`] impls frame objects/arrays with the `begin_*`/
/// `end_*` pairs and write members with [`JsonWriter::field`]; separators are
/// tracked internally, so callers never write a comma. Compact output (no
/// insignificant whitespace), matching `serde_json::to_string`.
pub struct JsonWriter {
    buf: String,
    /// One flag per open container (object or array), innermost last: whether a
    /// member/element has already been written into it (so the next needs a
    /// leading comma).
    stack: Vec<bool>,
}

impl std::fmt::Debug for JsonWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonWriter")
            .field("len", &self.buf.len())
            .field("depth", &self.stack.len())
            .finish()
    }
}

impl JsonWriter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            stack: Vec::new(),
        }
    }

    /// Insert the separating comma before the next member/element of the current
    /// container, and mark the container as non-empty.
    fn separate(&mut self) {
        if let Some(top) = self.stack.last_mut() {
            if *top {
                self.buf.push(',');
            } else {
                *top = true;
            }
        }
    }

    /// Open a JSON object (`{`).
    pub fn begin_object(&mut self) {
        self.buf.push('{');
        self.stack.push(false);
    }

    /// Close the current JSON object (`}`).
    pub fn end_object(&mut self) {
        self.buf.push('}');
        self.stack.pop();
    }

    /// Open a JSON array (`[`).
    pub fn begin_array(&mut self) {
        self.buf.push('[');
        self.stack.push(false);
    }

    /// Close the current JSON array (`]`).
    pub fn end_array(&mut self) {
        self.buf.push(']');
        self.stack.pop();
    }

    /// Write an object member key (with the preceding comma when needed) followed
    /// by `:`; the caller writes the value next.
    pub fn key(&mut self, key: &str) {
        self.separate();
        self.write_escaped(key);
        self.buf.push(':');
    }

    /// Mark the start of an array element (comma when needed); the caller writes
    /// the element value next.
    pub fn element(&mut self) {
        self.separate();
    }

    /// Write a complete object member: key then the value's JSON.
    pub fn field<T: ToJson + ?Sized>(&mut self, key: &str, value: &T) {
        self.key(key);
        value.write_json(self);
    }

    /// Write an object member whose value is a bare string literal (the `_type`
    /// discriminator tag).
    pub fn field_str(&mut self, key: &str, value: &str) {
        self.key(key);
        self.write_str(value);
    }

    /// Write a JSON string value (quoted, escaped).
    pub fn write_str(&mut self, s: &str) {
        self.write_escaped(s);
    }

    /// Write a JSON boolean.
    pub fn write_bool(&mut self, v: bool) {
        self.buf.push_str(if v { "true" } else { "false" });
    }

    /// Write a JSON `null`.
    pub fn write_null(&mut self) {
        self.buf.push_str("null");
    }

    /// Write a signed 64-bit integer (via `itoa`, as serde_json does).
    pub fn write_i64(&mut self, v: i64) {
        let mut b = itoa::Buffer::new();
        self.buf.push_str(b.format(v));
    }

    /// Write a signed 32-bit integer.
    pub fn write_i32(&mut self, v: i32) {
        let mut b = itoa::Buffer::new();
        self.buf.push_str(b.format(v));
    }

    /// Write any `itoa`-formattable integer.
    pub fn write_int<I: itoa::Integer>(&mut self, v: I) {
        let mut b = itoa::Buffer::new();
        self.buf.push_str(b.format(v));
    }

    /// Write an `f64` (via `ryu::Buffer::format_finite`, our chosen canonical
    /// REAL lexeme; a non-finite value serializes as `null`, as serde_json does).
    pub fn write_f64(&mut self, v: f64) {
        if v.is_finite() {
            let mut b = ryu::Buffer::new();
            self.buf.push_str(b.format_finite(v));
        } else {
            self.write_null();
        }
    }

    /// Write an `f32` (via `ryu`; non-finite → `null`).
    pub fn write_f32(&mut self, v: f32) {
        if v.is_finite() {
            let mut b = ryu::Buffer::new();
            self.buf.push_str(b.format_finite(v));
        } else {
            self.write_null();
        }
    }

    /// Append pre-serialized JSON verbatim (used to delegate an untyped
    /// `serde_json::Value` leaf to serde_json for byte-exact output).
    pub fn write_raw(&mut self, raw: &str) {
        self.buf.push_str(raw);
    }

    /// Consume the writer and return the serialized JSON.
    #[must_use]
    pub fn into_string(self) -> String {
        self.buf
    }

    /// Write a `"`-quoted, escaped JSON string. The algorithm is serde_json's:
    /// emit the longest verbatim run up to each byte that needs escaping, then
    /// the escape, and finally the trailing run. Escape bytes are single-byte
    /// ASCII (`"`, `\`, C0 controls), so every slice boundary lands on a UTF-8
    /// character boundary.
    fn write_escaped(&mut self, s: &str) {
        self.buf.push('"');
        let bytes = s.as_bytes();
        let mut start = 0;
        for (i, &b) in bytes.iter().enumerate() {
            let esc = ESCAPE[b as usize];
            if esc == 0 {
                continue;
            }
            if start < i {
                self.buf.push_str(&s[start..i]);
            }
            match esc {
                b'"' => self.buf.push_str("\\\""),
                b'\\' => self.buf.push_str("\\\\"),
                b'b' => self.buf.push_str("\\b"),
                b'f' => self.buf.push_str("\\f"),
                b'n' => self.buf.push_str("\\n"),
                b'r' => self.buf.push_str("\\r"),
                b't' => self.buf.push_str("\\t"),
                // Any other C0 control: `\u00XX`, lowercase hex.
                _ => {
                    self.buf.push_str("\\u00");
                    self.buf.push(HEX[(b >> 4) as usize] as char);
                    self.buf.push(HEX[(b & 0xF) as usize] as char);
                }
            }
            start = i + 1;
        }
        if start < bytes.len() {
            self.buf.push_str(&s[start..]);
        }
        self.buf.push('"');
    }
}

impl Default for JsonWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize a value to a canonical-JSON `String` through the native codec.
///
/// This is the codec entry point mirroring [`crate::xml::runtime::to_xml`]; the
/// canonical-JSON `json::to_canonical_json` entry point still uses serde_json in
/// this phase, and the two are proven byte-identical by the parity gate.
#[must_use]
pub fn to_json_string<T: ToJson + ?Sized>(value: &T) -> String {
    let mut w = JsonWriter::new();
    value.write_json(&mut w);
    w.into_string()
}

/// A value that serializes to canonical openEHR JSON.
///
/// `write_json` writes the value's complete JSON encoding into the writer. The
/// `None`/empty-`Vec` omission and `_type`-first ordering are applied by the
/// generated per-struct impls at their field call sites, not here.
pub trait ToJson {
    /// The concrete openEHR `_type` name, or `""` for a primitive/leaf that
    /// carries no `_type`. Mirrors [`crate::xml::runtime::ToXml::xml_type_name`];
    /// the serialize side does not consume it (a struct emits its own constant
    /// `_type`), but it keeps the two codecs' trait shapes aligned.
    fn json_type_name(&self) -> &'static str {
        ""
    }

    fn write_json(&self, w: &mut JsonWriter);
}

// ── primitive / leaf impls (so every generated field uniformly calls write_json) ──

impl ToJson for String {
    fn write_json(&self, w: &mut JsonWriter) {
        w.write_str(self);
    }
}

impl ToJson for str {
    fn write_json(&self, w: &mut JsonWriter) {
        w.write_str(self);
    }
}

impl ToJson for bool {
    fn write_json(&self, w: &mut JsonWriter) {
        w.write_bool(*self);
    }
}

impl ToJson for char {
    fn write_json(&self, w: &mut JsonWriter) {
        // serde_json serializes a `char` as a one-character (escaped) string.
        let mut buf = [0u8; 4];
        w.write_str(self.encode_utf8(&mut buf));
    }
}

macro_rules! impl_to_json_int {
    ($($t:ty),*) => {$(
        impl ToJson for $t {
            fn write_json(&self, w: &mut JsonWriter) {
                w.write_int(*self);
            }
        }
    )*};
}
impl_to_json_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl ToJson for f64 {
    fn write_json(&self, w: &mut JsonWriter) {
        w.write_f64(*self);
    }
}

impl ToJson for f32 {
    fn write_json(&self, w: &mut JsonWriter) {
        w.write_f32(*self);
    }
}

impl ToJson for uuid::Uuid {
    fn write_json(&self, w: &mut JsonWriter) {
        // serde_json (human-readable) serializes a Uuid as its lowercase
        // hyphenated string; encode into a stack buffer to avoid allocating.
        let mut buf = uuid::Uuid::encode_buffer();
        w.write_str(self.hyphenated().encode_lower(&mut buf));
    }
}

impl<T: ToJson + ?Sized> ToJson for Box<T> {
    fn json_type_name(&self) -> &'static str {
        (**self).json_type_name()
    }
    fn write_json(&self, w: &mut JsonWriter) {
        (**self).write_json(w);
    }
}

impl<T: ToJson + ?Sized> ToJson for &T {
    fn json_type_name(&self) -> &'static str {
        (**self).json_type_name()
    }
    fn write_json(&self, w: &mut JsonWriter) {
        (**self).write_json(w);
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        w.begin_array();
        for e in self {
            w.element();
            e.write_json(w);
        }
        w.end_array();
    }
}

impl<T: ToJson> ToJson for [T] {
    fn write_json(&self, w: &mut JsonWriter) {
        w.begin_array();
        for e in self {
            w.element();
            e.write_json(w);
        }
        w.end_array();
    }
}

impl<V: ToJson> ToJson for std::collections::BTreeMap<String, V> {
    fn write_json(&self, w: &mut JsonWriter) {
        // serde_json emits a map as a JSON object with string keys, in the map's
        // iteration order (sorted, for a BTreeMap) — matched here.
        w.begin_object();
        for (k, v) in self {
            w.key(k);
            v.write_json(w);
        }
        w.end_object();
    }
}

impl ToJson for serde_json::Value {
    // A `serde_json::Value` field is an untyped codegen monomorphization artifact
    // (the version-family payloads and BMM-`Any` fields the generator leaves
    // untyped). serde_json is the authoritative encoder for such a value, so this
    // leaf delegates to it — byte-exact with the serde path by construction, and
    // honouring the workspace's `preserve_order` map ordering. (`Value`
    // serialization is infallible; the `Err` arm is unreachable.)
    fn write_json(&self, w: &mut JsonWriter) {
        match serde_json::to_string(self) {
            Ok(s) => w.write_raw(&s),
            Err(_) => w.write_null(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Deserialize side: JsonNode (two backends) + a borrowing tokenizer + FromJson.
// ════════════════════════════════════════════════════════════════════════════

/// A canonical-JSON deserialization error, carrying a message, an optional source
/// location (byte offset + 1-based line/column, for a tokenizer/syntax error) and
/// an optional JSON path (built as the reader descends, for a semantic error) —
/// quality comparable to `serde_json`'s `line N column M` diagnostics, which the
/// REST layer surfaces on a 400/422.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonParseError {
    message: String,
    /// `Some((byte_offset, line, column))` for a syntax error located in source.
    location: Option<(usize, usize, usize)>,
    /// JSON path segments from the root to the failing node, outermost first
    /// (e.g. `.content`, `[0]`, `.data`). Empty for a root-level error.
    path: Vec<String>,
}

impl JsonParseError {
    /// A semantic error with a message and no source location (path is filled in
    /// as it propagates up through [`Self::in_field`] / [`Self::in_index`]).
    #[must_use]
    pub fn custom(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            path: Vec::new(),
        }
    }

    /// A syntax error at `offset` bytes into `src` (line/column computed from it).
    #[must_use]
    fn syntax(message: impl Into<String>, src: &str, offset: usize) -> Self {
        let (line, column) = line_column(src, offset);
        Self {
            message: message.into(),
            location: Some((offset, line, column)),
            path: Vec::new(),
        }
    }

    /// A "wrong JSON kind" error (`expected` a String/object/number/…, found the
    /// actual node kind), mirroring serde's `invalid type` messages.
    #[must_use]
    pub fn type_error(expected: &str, found: &str) -> Self {
        Self::custom(format!("invalid type: {found}, expected {expected}"))
    }

    /// A missing mandatory field on a concrete type.
    #[must_use]
    pub fn missing_field(field: &str, ty: &str) -> Self {
        Self::custom(format!("missing field `{field}` on `{ty}`"))
    }

    /// A present-but-wrong `_type` discriminator on a concrete type.
    #[must_use]
    pub fn type_mismatch(expected: &str, found: &str) -> Self {
        Self::custom(format!("expected _type \"{expected}\", found \"{found}\""))
    }

    /// Prepend a `.field` path segment (called as the error propagates out of a
    /// named struct field).
    #[must_use]
    pub fn in_field(mut self, field: &str) -> Self {
        self.path.insert(0, format!(".{field}"));
        self
    }

    /// Prepend an `[index]` path segment (called as the error propagates out of
    /// an array element).
    #[must_use]
    pub fn in_index(mut self, index: usize) -> Self {
        self.path.insert(0, format!("[{index}]"));
        self
    }
}

/// Compute a 1-based `(line, column)` for a byte offset into `src` (UTF-8 aware:
/// column counts characters, matching serde_json).
fn line_column(src: &str, offset: usize) -> (usize, usize) {
    let capped = offset.min(src.len());
    let consumed = src.get(..capped).unwrap_or(src);
    let mut line = 1usize;
    let mut column = 1usize;
    for ch in consumed.chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

impl std::fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some((offset, line, column)) = self.location {
            write!(f, " at line {line} column {column} (byte {offset})")?;
        }
        if !self.path.is_empty() {
            write!(f, " (at ${})", self.path.join(""))?;
        }
        Ok(())
    }
}

impl std::error::Error for JsonParseError {}

/// A read-only JSON node, over either backend (the borrowing tree or a
/// `serde_json::Value`). Generated [`FromJson`] impls are generic over this trait
/// and look each known field up by key — the shape that makes unknown keys,
/// out-of-order members, and `_type` lookahead fall out for free.
///
/// The number accessors reproduce `serde_json::Value`'s coercion exactly
/// (`as_i64`/`as_u64` are `None` for a fractional number; `as_f64` widens an
/// integer), so the RM field's Rust type — not the wire lexeme — decides how a
/// number is read (an integer lexeme in a Real field widens to `x.0`).
pub trait JsonNode {
    /// Is this the JSON `null` literal?
    fn is_null(&self) -> bool;
    /// The boolean value, if this is a JSON boolean.
    fn as_bool(&self) -> Option<bool>;
    /// The value as `i64`, if it is an integer that fits (fractional → `None`).
    fn as_i64(&self) -> Option<i64>;
    /// The value as `u64`, if it is a non-negative integer that fits.
    fn as_u64(&self) -> Option<u64>;
    /// The value widened to `f64`, if it is any JSON number.
    fn as_f64(&self) -> Option<f64>;
    /// The string value, if this is a JSON string.
    fn as_str(&self) -> Option<&str>;
    /// The member under `key`, if this is an object containing it (last wins on
    /// a duplicate key, mirroring `serde_json`'s map).
    fn get(&self, key: &str) -> Option<&Self>;
    /// The elements, if this is a JSON array.
    fn as_array(&self) -> Option<&[Self]>
    where
        Self: Sized;
    /// The `(key, value)` members, if this is a JSON object.
    fn object_entries(&self) -> Option<Vec<(&str, &Self)>>
    where
        Self: Sized;
    /// Reconstruct a `serde_json::Value` (for a field typed as untyped `Value`).
    fn to_value(&self) -> serde_json::Value;
    /// A short human label for the actual JSON kind, for error messages.
    fn kind(&self) -> &'static str;
}

// ── backend 1: a borrowing tree produced by the hand-written tokenizer ────────

/// A parsed JSON value that borrows its strings from the source `&str` where no
/// unescaping was needed. This is the [`from_json_str`] backend — the openEHR
/// spec types never round-trip through `serde_json::Value` on the parse path.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonTree<'a> {
    Null,
    Bool(bool),
    /// A signed integer lexeme that fits `i64`.
    Int(i64),
    /// A non-negative integer lexeme that overflows `i64` but fits `u64`.
    Uint(u64),
    /// A fractional/exponent lexeme, or an integer too large for `u64`.
    Float(f64),
    Str(std::borrow::Cow<'a, str>),
    Array(Vec<JsonTree<'a>>),
    /// Object members in source order (duplicate keys retained; `get` returns the
    /// last, matching `serde_json`).
    Object(Vec<(std::borrow::Cow<'a, str>, JsonTree<'a>)>),
}

impl JsonNode for JsonTree<'_> {
    fn is_null(&self) -> bool {
        matches!(self, JsonTree::Null)
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            JsonTree::Bool(b) => Some(*b),
            _ => None,
        }
    }
    fn as_i64(&self) -> Option<i64> {
        match self {
            JsonTree::Int(v) => Some(*v),
            JsonTree::Uint(v) => i64::try_from(*v).ok(),
            _ => None,
        }
    }
    fn as_u64(&self) -> Option<u64> {
        match self {
            JsonTree::Int(v) => u64::try_from(*v).ok(),
            JsonTree::Uint(v) => Some(*v),
            _ => None,
        }
    }
    #[allow(clippy::cast_precision_loss)] // wire number widening to f64, exactly as serde_json::Value::as_f64
    fn as_f64(&self) -> Option<f64> {
        match self {
            JsonTree::Int(v) => Some(*v as f64),
            JsonTree::Uint(v) => Some(*v as f64),
            JsonTree::Float(v) => Some(*v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            JsonTree::Str(s) => Some(s.as_ref()),
            _ => None,
        }
    }
    fn get(&self, key: &str) -> Option<&Self> {
        match self {
            JsonTree::Object(entries) => {
                entries.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
            }
            _ => None,
        }
    }
    fn as_array(&self) -> Option<&[Self]> {
        match self {
            JsonTree::Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }
    fn object_entries(&self) -> Option<Vec<(&str, &Self)>> {
        match self {
            JsonTree::Object(entries) => {
                Some(entries.iter().map(|(k, v)| (k.as_ref(), v)).collect())
            }
            _ => None,
        }
    }
    fn to_value(&self) -> serde_json::Value {
        match self {
            JsonTree::Null => serde_json::Value::Null,
            JsonTree::Bool(b) => serde_json::Value::Bool(*b),
            JsonTree::Int(v) => serde_json::Value::Number((*v).into()),
            JsonTree::Uint(v) => serde_json::Value::Number((*v).into()),
            JsonTree::Float(v) => serde_json::Number::from_f64(*v)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            JsonTree::Str(s) => serde_json::Value::String(s.as_ref().to_owned()),
            JsonTree::Array(a) => {
                serde_json::Value::Array(a.iter().map(JsonNode::to_value).collect())
            }
            JsonTree::Object(entries) => serde_json::Value::Object(
                entries
                    .iter()
                    .map(|(k, v)| (k.as_ref().to_owned(), v.to_value()))
                    .collect(),
            ),
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            JsonTree::Null => "null",
            JsonTree::Bool(_) => "boolean",
            JsonTree::Int(_) | JsonTree::Uint(_) | JsonTree::Float(_) => "number",
            JsonTree::Str(_) => "string",
            JsonTree::Array(_) => "array",
            JsonTree::Object(_) => "object",
        }
    }
}

// ── backend 2: a serde_json::Value walker (the validation-tier entry point) ────

impl JsonNode for serde_json::Value {
    fn is_null(&self) -> bool {
        self.is_null()
    }
    fn as_bool(&self) -> Option<bool> {
        self.as_bool()
    }
    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }
    fn as_u64(&self) -> Option<u64> {
        self.as_u64()
    }
    fn as_f64(&self) -> Option<f64> {
        self.as_f64()
    }
    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }
    fn get(&self, key: &str) -> Option<&Self> {
        self.as_object().and_then(|m| m.get(key))
    }
    fn as_array(&self) -> Option<&[Self]> {
        self.as_array().map(Vec::as_slice)
    }
    fn object_entries(&self) -> Option<Vec<(&str, &Self)>> {
        self.as_object()
            .map(|m| m.iter().map(|(k, v)| (k.as_str(), v)).collect())
    }
    fn to_value(&self) -> serde_json::Value {
        self.clone()
    }
    fn kind(&self) -> &'static str {
        match self {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }
}

// ── the hand-written tokenizer (borrowing, single pass) ───────────────────────

/// Parse `input` into a borrowing [`JsonTree`]. One pass; strings are borrowed
/// from `input` unless an escape forces an owned copy.
///
/// # Errors
/// Returns a [`JsonParseError`] with a byte offset + line/column on any syntax
/// error (malformed literal, unterminated string, trailing content, …).
fn parse_tree(input: &str) -> Result<JsonTree<'_>, JsonParseError> {
    let mut p = Tokenizer {
        src: input,
        bytes: input.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(p.err("trailing characters after JSON value"));
    }
    Ok(value)
}

/// The recursive-descent tokenizer state (byte cursor over the source).
struct Tokenizer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn err(&self, message: impl Into<String>) -> JsonParseError {
        JsonParseError::syntax(message, self.src, self.pos)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Result<JsonTree<'a>, JsonParseError> {
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(JsonTree::Str(self.parse_string()?)),
            Some(b't' | b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            _ => Err(self.err("expected a JSON value")),
        }
    }

    fn expect(&mut self, byte: u8, what: &str) -> Result<(), JsonParseError> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(format!("expected {what}")))
        }
    }

    fn parse_object(&mut self) -> Result<JsonTree<'a>, JsonParseError> {
        self.expect(b'{', "`{`")?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonTree::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected a string object key"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':', "`:` after object key")?;
            self.skip_ws();
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonTree::Object(entries));
                }
                _ => return Err(self.err("expected `,` or `}` in object")),
            }
        }
    }

    fn parse_array(&mut self) -> Result<JsonTree<'a>, JsonParseError> {
        self.expect(b'[', "`[`")?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonTree::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonTree::Array(items));
                }
                _ => return Err(self.err("expected `,` or `]` in array")),
            }
        }
    }

    fn parse_bool(&mut self) -> Result<JsonTree<'a>, JsonParseError> {
        if self.bytes[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(JsonTree::Bool(true))
        } else if self.bytes[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(JsonTree::Bool(false))
        } else {
            Err(self.err("expected `true` or `false`"))
        }
    }

    fn parse_null(&mut self) -> Result<JsonTree<'a>, JsonParseError> {
        if self.bytes[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(JsonTree::Null)
        } else {
            Err(self.err("expected `null`"))
        }
    }

    fn parse_number(&mut self) -> Result<JsonTree<'a>, JsonParseError> {
        let start = self.pos;
        let mut is_float = false;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    is_float = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let token = self.src.get(start..self.pos).unwrap_or("");
        if token.is_empty() || token == "-" {
            return Err(JsonParseError::syntax("invalid number", self.src, start));
        }
        if is_float {
            return token
                .parse::<f64>()
                .map(JsonTree::Float)
                .map_err(|_| JsonParseError::syntax("invalid number", self.src, start));
        }
        if let Ok(v) = token.parse::<i64>() {
            return Ok(JsonTree::Int(v));
        }
        if let Ok(v) = token.parse::<u64>() {
            return Ok(JsonTree::Uint(v));
        }
        // An integer literal wider than u64 falls back to f64, as serde_json does.
        token
            .parse::<f64>()
            .map(JsonTree::Float)
            .map_err(|_| JsonParseError::syntax("invalid number", self.src, start))
    }

    /// Parse a `"`-delimited string, borrowing from the source when it contains
    /// no escape (the common case) and allocating only when an escape is present.
    fn parse_string(&mut self) -> Result<std::borrow::Cow<'a, str>, JsonParseError> {
        self.expect(b'"', "`\"`")?;
        let content_start = self.pos;
        // Fast path: scan to the closing quote; if no backslash was seen, borrow.
        while let Some(b) = self.peek() {
            match b {
                b'"' => {
                    let slice = self.src.get(content_start..self.pos).unwrap_or("");
                    self.pos += 1;
                    return Ok(std::borrow::Cow::Borrowed(slice));
                }
                b'\\' => break,
                _ => self.pos += 1,
            }
        }
        // Slow path: an escape is present — build an owned String from the start.
        let mut out = String::from(self.src.get(content_start..self.pos).unwrap_or(""));
        loop {
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(std::borrow::Cow::Owned(out));
                }
                Some(b'\\') => {
                    self.pos += 1;
                    self.parse_escape(&mut out)?;
                }
                Some(_) => {
                    // Copy one whole UTF-8 char (peek is a lead byte here).
                    let rest = self.src.get(self.pos..).unwrap_or("");
                    if let Some(ch) = rest.chars().next() {
                        out.push(ch);
                        self.pos += ch.len_utf8();
                    } else {
                        return Err(self.err("unterminated string"));
                    }
                }
                None => return Err(self.err("unterminated string")),
            }
        }
    }

    /// Handle one escape sequence (the `\` already consumed), appending to `out`.
    fn parse_escape(&mut self, out: &mut String) -> Result<(), JsonParseError> {
        match self.peek() {
            Some(b'"') => out.push('"'),
            Some(b'\\') => out.push('\\'),
            Some(b'/') => out.push('/'),
            Some(b'b') => out.push('\u{0008}'),
            Some(b'f') => out.push('\u{000C}'),
            Some(b'n') => out.push('\n'),
            Some(b'r') => out.push('\r'),
            Some(b't') => out.push('\t'),
            Some(b'u') => {
                self.pos += 1; // past `u`
                let hi = self.parse_hex4()?;
                let ch = if (0xD800..=0xDBFF).contains(&hi) {
                    // A high surrogate must be followed by `\uXXXX` low surrogate.
                    if self.peek() == Some(b'\\') && self.bytes.get(self.pos + 1) == Some(&b'u') {
                        self.pos += 2; // past `\u`
                        let lo = self.parse_hex4()?;
                        if (0xDC00..=0xDFFF).contains(&lo) {
                            let c = 0x1_0000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                            char::from_u32(c).ok_or_else(|| self.err("invalid surrogate pair"))?
                        } else {
                            return Err(self.err("invalid low surrogate in \\u escape"));
                        }
                    } else {
                        return Err(self.err("unpaired high surrogate in \\u escape"));
                    }
                } else if (0xDC00..=0xDFFF).contains(&hi) {
                    return Err(self.err("unexpected low surrogate in \\u escape"));
                } else {
                    char::from_u32(hi).ok_or_else(|| self.err("invalid \\u escape"))?
                };
                out.push(ch);
                return Ok(()); // pos already advanced past the escape digits
            }
            Some(_) => return Err(self.err("invalid escape sequence")),
            None => return Err(self.err("unterminated escape sequence")),
        }
        self.pos += 1; // the single escape selector byte
        Ok(())
    }

    /// Read exactly four hex digits (the cursor is on the first digit) → `u32`.
    fn parse_hex4(&mut self) -> Result<u32, JsonParseError> {
        let hex = self
            .src
            .get(self.pos..self.pos + 4)
            .ok_or_else(|| self.err("truncated \\u escape"))?;
        let v = u32::from_str_radix(hex, 16).map_err(|_| self.err("invalid hex in \\u escape"))?;
        self.pos += 4;
        Ok(v)
    }
}

// ── the FromJson trait + primitive/leaf impls ─────────────────────────────────

/// A value that deserializes from a canonical-JSON [`JsonNode`]. Generated impls
/// read each known field by key (unknown keys ignored; members may be out of
/// order) and validate/dispatch on `_type` — mirroring, verbatim, the tolerance
/// rules of the retired `#[derive(OpenEhrType)]` reader.
pub trait FromJson: Sized {
    /// # Errors
    /// Returns a [`JsonParseError`] if `node` is not a valid encoding of `Self`.
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError>;
}

impl FromJson for String {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        node.as_str()
            .map(str::to_owned)
            .ok_or_else(|| JsonParseError::type_error("a string", node.kind()))
    }
}

impl FromJson for bool {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        node.as_bool()
            .ok_or_else(|| JsonParseError::type_error("a boolean", node.kind()))
    }
}

impl FromJson for char {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        let s = node
            .as_str()
            .ok_or_else(|| JsonParseError::type_error("a single-character string", node.kind()))?;
        let mut it = s.chars();
        match (it.next(), it.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(JsonParseError::custom(format!(
                "expected a single character, found {s:?}"
            ))),
        }
    }
}

impl FromJson for f64 {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        node.as_f64()
            .ok_or_else(|| JsonParseError::type_error("a number", node.kind()))
    }
}

impl FromJson for f32 {
    #[allow(clippy::cast_possible_truncation)] // Real→f32 field narrowing, as serde does for an f32 field
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        node.as_f64()
            .map(|v| v as f32)
            .ok_or_else(|| JsonParseError::type_error("a number", node.kind()))
    }
}

/// Integer `FromJson` via `as_i64`/`as_u64` + a checked narrowing, matching
/// serde's behaviour (out-of-range for the field's width is an error).
macro_rules! impl_from_json_signed {
    ($($t:ty),*) => {$(
        impl FromJson for $t {
            fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
                let v = node.as_i64().ok_or_else(|| JsonParseError::type_error("an integer", node.kind()))?;
                <$t>::try_from(v).map_err(|_| JsonParseError::custom(format!("integer {v} out of range for {}", stringify!($t))))
            }
        }
    )*};
}
impl_from_json_signed!(i8, i16, i32, i64, isize);

macro_rules! impl_from_json_unsigned {
    ($($t:ty),*) => {$(
        impl FromJson for $t {
            fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
                let v = node.as_u64().ok_or_else(|| JsonParseError::type_error("a non-negative integer", node.kind()))?;
                <$t>::try_from(v).map_err(|_| JsonParseError::custom(format!("integer {v} out of range for {}", stringify!($t))))
            }
        }
    )*};
}
impl_from_json_unsigned!(u8, u16, u32, u64, usize);

impl FromJson for uuid::Uuid {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        let s = node
            .as_str()
            .ok_or_else(|| JsonParseError::type_error("a UUID string", node.kind()))?;
        uuid::Uuid::parse_str(s).map_err(|e| JsonParseError::custom(e.to_string()))
    }
}

impl<T: FromJson> FromJson for Box<T> {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        Ok(Box::new(T::from_json(node)?))
    }
}

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        let arr = node
            .as_array()
            .ok_or_else(|| JsonParseError::type_error("an array", node.kind()))?;
        let mut out = Vec::with_capacity(arr.len());
        for (i, e) in arr.iter().enumerate() {
            out.push(T::from_json(e).map_err(|err| err.in_index(i))?);
        }
        Ok(out)
    }
}

impl<V: FromJson> FromJson for std::collections::BTreeMap<String, V> {
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        let entries = node
            .object_entries()
            .ok_or_else(|| JsonParseError::type_error("an object", node.kind()))?;
        let mut out = std::collections::BTreeMap::new();
        for (k, v) in entries {
            out.insert(k.to_owned(), V::from_json(v).map_err(|e| e.in_field(k))?);
        }
        Ok(out)
    }
}

impl FromJson for serde_json::Value {
    // The untyped codegen monomorphization artifact (BMM-`Any` / version-family
    // payloads): reconstruct the full value from the node (a clone on the Value
    // backend, a tree walk on the borrowing backend). Mirror of the `ToJson` impl.
    fn from_json<N: JsonNode>(node: &N) -> Result<Self, JsonParseError> {
        Ok(node.to_value())
    }
}

// ── helpers the generated struct/enum impls call (the tolerance rules live here) ──

/// Require `node` to be a JSON object (a struct never deserializes from a scalar).
///
/// # Errors
/// Returns a [`JsonParseError`] if `node` is not an object.
pub fn expect_object<N: JsonNode>(node: &N, ty: &str) -> Result<(), JsonParseError> {
    if node.object_entries().is_some() {
        Ok(())
    } else {
        Err(JsonParseError::custom(format!(
            "invalid type: {}, expected an object for `{ty}`",
            node.kind()
        )))
    }
}

/// Enforce the concrete-type `_type` discipline: a present `_type` must equal
/// `expected`; an absent one is tolerated (per ITS-JSON, a concretely-typed slot
/// may omit `_type`). Verbatim from the retired derive's shadow-struct check.
///
/// # Errors
/// Returns a [`JsonParseError`] on a present-but-wrong `_type`.
pub fn check_type<N: JsonNode>(node: &N, expected: &str) -> Result<(), JsonParseError> {
    if let Some(found) = node.get("_type").and_then(JsonNode::as_str)
        && found != expected
    {
        return Err(JsonParseError::type_mismatch(expected, found));
    }
    Ok(())
}

/// Read a mandatory field (missing → error). Verbatim from the derive: a plain
/// field with no default is required.
///
/// # Errors
/// Returns a [`JsonParseError`] if the field is absent or fails to parse.
pub fn required_field<N: JsonNode, T: FromJson>(
    node: &N,
    key: &str,
    ty: &str,
) -> Result<T, JsonParseError> {
    match node.get(key) {
        Some(v) if v.is_null() => Err(JsonParseError::missing_field(key, ty)),
        Some(v) => T::from_json(v).map_err(|e| e.in_field(key)),
        None => Err(JsonParseError::missing_field(key, ty)),
    }
}

/// Read an optional field (absent or `null` → `None`). Verbatim from the derive:
/// an `Option` field defaults to `None`.
///
/// # Errors
/// Returns a [`JsonParseError`] if a present, non-null value fails to parse.
pub fn optional_field<N: JsonNode, T: FromJson>(
    node: &N,
    key: &str,
) -> Result<Option<T>, JsonParseError> {
    match node.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => Ok(Some(T::from_json(v).map_err(|e| e.in_field(key))?)),
        None => Ok(None),
    }
}

/// Read a container field (absent or `null` → empty `Vec`). Verbatim from the
/// derive: a `Vec` field defaults to empty.
///
/// # Errors
/// Returns a [`JsonParseError`] if a present, non-null value fails to parse.
pub fn container_field<N: JsonNode, T: FromJson>(
    node: &N,
    key: &str,
) -> Result<Vec<T>, JsonParseError> {
    match node.get(key) {
        Some(v) if v.is_null() => Ok(Vec::new()),
        Some(v) => Vec::from_json(v).map_err(|e| e.in_field(key)),
        None => Ok(Vec::new()),
    }
}

/// Read a plain field that carries a literal default (the `Interval`
/// `*_included`/`*_unbounded` flags): absent → `default`. Verbatim from the
/// derive's `#[openehr(default = "…")]`.
///
/// # Errors
/// Returns a [`JsonParseError`] if a present value fails to parse.
pub fn defaulted_field<N: JsonNode, T: FromJson>(
    node: &N,
    key: &str,
    default: T,
) -> Result<T, JsonParseError> {
    match node.get(key) {
        Some(v) if v.is_null() => Ok(default),
        Some(v) => T::from_json(v).map_err(|e| e.in_field(key)),
        None => Ok(default),
    }
}

/// Read the `_type` discriminator of a polymorphic slot, for enum dispatch.
#[must_use]
pub fn slot_type<N: JsonNode>(node: &N) -> Option<&str> {
    node.get("_type").and_then(JsonNode::as_str)
}

// ── entry points ──────────────────────────────────────────────────────────────

/// Deserialize a value from a canonical-JSON `&str` through the native codec (no
/// `serde_json` on the parse path).
///
/// # Errors
/// Returns a [`JsonParseError`] on a syntax error or an invalid encoding.
pub fn from_json_str<T: FromJson>(input: &str) -> Result<T, JsonParseError> {
    let tree = parse_tree(input)?;
    T::from_json(&tree)
}

/// Deserialize a value from an already-parsed `serde_json::Value` through the
/// native codec (the validation tier's entry point — no re-stringifying).
///
/// # Errors
/// Returns a [`JsonParseError`] on an invalid encoding.
pub fn from_json_value<T: FromJson>(value: &serde_json::Value) -> Result<T, JsonParseError> {
    T::from_json(value)
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::panic)]
mod tests {
    use super::{JsonWriter, to_json_string};

    #[test]
    fn escapes_match_serde_json() {
        // Quotes, backslashes, control chars, and non-ASCII, against serde_json.
        for s in [
            "plain",
            "with \"quotes\"",
            "back\\slash",
            "tab\tnewline\nreturn\r",
            "bell\u{07}form\u{0C}",
            "unit\u{1F}sep",
            "slash/kept",
            "café — naïve — 日本語 — 😀",
        ] {
            let mine = to_json_string(&s.to_string());
            let serde = serde_json::to_string(s).unwrap();
            assert_eq!(mine, serde, "escaping diverged for {s:?}");
        }
    }

    #[test]
    fn numbers() {
        // Integers are byte-identical to serde_json (both via itoa).
        assert_eq!(to_json_string(&5i64), serde_json::to_string(&5i64).unwrap());
        assert_eq!(to_json_string(&(-42i32)), "-42");
        assert_eq!(to_json_string(&5_000_000_000usize), "5000000000");
        // Reals via ryu: whole reals keep `x.0`; shortest round-trip otherwise.
        assert_eq!(to_json_string(&5.0f64), "5.0");
        assert_eq!(to_json_string(&0.1f64), "0.1");
        // ryu is our chosen REAL lexeme; it writes an unsigned exponent (`1e21`),
        // deliberately diverging from serde_json's dtoa (`1e+21`) — see the
        // module NOTE. Assert the deterministic ryu form, not serde equality.
        assert_eq!(to_json_string(&1e21f64), "1e21");
        // Non-finite → null (matching serde_json).
        assert_eq!(to_json_string(&f64::NAN), "null");
        assert_eq!(to_json_string(&f64::INFINITY), "null");
    }

    #[test]
    fn container_framing() {
        let v = vec![1i32, 2, 3];
        assert_eq!(to_json_string(&v), "[1,2,3]");
        let empty: Vec<i32> = Vec::new();
        assert_eq!(to_json_string(&empty), "[]");

        let mut m = std::collections::BTreeMap::new();
        m.insert("b".to_string(), "2".to_string());
        m.insert("a".to_string(), "1".to_string());
        assert_eq!(to_json_string(&m), r#"{"a":"1","b":"2"}"#);
        assert_eq!(to_json_string(&m), serde_json::to_string(&m).unwrap());
    }

    #[test]
    fn nested_object_separators() {
        // Exercise the separator stack across nesting.
        let mut w = JsonWriter::new();
        w.begin_object();
        w.field_str("_type", "X");
        w.field("n", &3i32);
        w.key("inner");
        w.begin_object();
        w.field("a", &true);
        w.end_object();
        w.field("arr", &vec![1i32, 2]);
        w.end_object();
        assert_eq!(
            w.into_string(),
            r#"{"_type":"X","n":3,"inner":{"a":true},"arr":[1,2]}"#
        );
    }

    // ── tokenizer / reader ────────────────────────────────────────────────────

    use super::{JsonNode, JsonTree, from_json_str, from_json_value, parse_tree};

    #[test]
    fn parses_scalars_and_number_kinds() {
        // Int vs Uint vs Float distinction (mirrors serde_json::Value coercion).
        assert!(matches!(parse_tree("5").unwrap(), JsonTree::Int(5)));
        assert!(matches!(parse_tree("-42").unwrap(), JsonTree::Int(-42)));
        assert!(matches!(parse_tree("5.0").unwrap(), JsonTree::Float(f) if f == 5.0));
        assert!(matches!(parse_tree("1e3").unwrap(), JsonTree::Float(f) if f == 1000.0));
        assert!(matches!(
            parse_tree("18446744073709551615").unwrap(),
            JsonTree::Uint(u64::MAX)
        ));
        // as_i64 is None for a fractional number; as_f64 widens an integer.
        assert_eq!(parse_tree("5").unwrap().as_i64(), Some(5));
        assert_eq!(parse_tree("5.5").unwrap().as_i64(), None);
        assert_eq!(parse_tree("5").unwrap().as_f64(), Some(5.0));
    }

    #[test]
    fn strings_borrow_when_unescaped_and_own_when_escaped() {
        // No escape → borrowed slice of the source.
        match parse_tree(r#""plain text""#).unwrap() {
            JsonTree::Str(std::borrow::Cow::Borrowed(s)) => assert_eq!(s, "plain text"),
            other => panic!("expected a borrowed string, got {other:?}"),
        }
        // Escape present → owned, correctly decoded.
        match parse_tree(r#""a\tb\nA""#).unwrap() {
            JsonTree::Str(std::borrow::Cow::Owned(s)) => assert_eq!(s, "a\tb\nA"),
            other => panic!("expected an owned string, got {other:?}"),
        }
    }

    #[test]
    fn syntax_errors_carry_a_location() {
        let e = parse_tree(r#"{"a": }"#).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("line 1 column"), "no location in {msg:?}");
        // Trailing content after a complete value is rejected.
        assert!(parse_tree("5 6").is_err());
        // Unterminated string.
        assert!(parse_tree(r#""oops"#).is_err());
        // Lone high surrogate.
        assert!(parse_tree(r#""\uD83D""#).is_err());
    }

    #[test]
    fn value_backend_and_str_backend_agree() {
        // The same document read through both JsonNode backends yields the same
        // typed integer.
        let via_str: i32 = from_json_str("7").unwrap();
        let value: serde_json::Value = serde_json::from_str("7").unwrap();
        let via_value: i32 = from_json_value(&value).unwrap();
        assert_eq!(via_str, via_value);
        assert_eq!(via_str, 7);
    }

    #[test]
    fn get_returns_last_on_duplicate_key() {
        // Mirrors serde_json's last-wins map semantics.
        let tree = parse_tree(r#"{"a":1,"a":2}"#).unwrap();
        assert_eq!(tree.get("a").and_then(JsonNode::as_i64), Some(2));
    }
}
