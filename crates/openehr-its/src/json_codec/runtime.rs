//! Hand-written runtime for canonical-JSON serialization (ITS-JSON).
//!
//! The generated code (`emit-json`, in `generated/`) implements [`ToJson`] for
//! the RM/BASE/AM/TERM/LANG spec types; this module is the trait definition, the
//! [`JsonWriter`], and the primitive/leaf impls those generated impls call into.
//! It mirrors the shape of the canonical-XML runtime ([`crate::xml::runtime`]): a
//! writer plus per-type impls, generated code on top.
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
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
}
