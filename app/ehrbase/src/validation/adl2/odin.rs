//! A minimal ODIN reader for the ADL2 `language`/`terminology` subset.
//!
//! NOTE (why a local reader — G-09-04 re-verified 2026-07-12): the openEHR
//! ODIN object syntax has a normative grammar (`AM/docs/ADL2/master08-adl.adoc`
//! §ODIN), but there is no shared runtime ODIN parser to consume. `openehr-lang`
//! holds only the BMM / `P_BMM` object model generated from the meta-model; its
//! own module doc records that "the runtime ODIN and EL parsers are future
//! hand-written work" (`crates/openehr-lang/src/lib.rs`), and no
//! `openehr_lang::odin` item exists. The ADL2 registration surface therefore
//! carries this deliberately small lexical reader for exactly the shapes the
//! `language`/`terminology` sections use — keyed blocks (`["k"] = <…>`),
//! attribute blocks (`name = <…>`), string leaves/lists, and code leaves
//! (`[ISO_639-1::en]`). Anything else is kept as [`OdinValue::Other`] and
//! tolerated, never a parse error: a registry must not reject a source over
//! ODIN features it does not model (`AOM2/master08-validation.adoc` §Phase 1
//! is about mandatory-part presence, not full ODIN coverage).

use std::collections::BTreeMap;

/// The ODIN subset ADL2 `language`/`terminology` sections use.
pub(super) enum OdinValue {
    /// `attr = <…>` pairs at one level.
    Attrs(BTreeMap<String, OdinValue>),
    /// `["key"] = <…>` pairs at one level.
    Keyed(BTreeMap<String, OdinValue>),
    /// `<"a", "b">` or `<"a">`.
    Strings(Vec<String>),
    /// `<[ISO_639-1::en]>`.
    Code(String),
    /// Unmodelled leaf content (numbers, uris, intervals, …) — tolerated and
    /// discarded: the registry must never reject a source over ODIN features
    /// it does not model.
    Other,
}

impl OdinValue {
    /// Parse a section body as a top-level attribute list.
    pub(super) fn parse(src: &str) -> Option<Self> {
        let mut p = OdinParser {
            src: src.as_bytes(),
            pos: 0,
        };
        p.attrs_block()
    }

    pub(super) fn attr(&self, name: &str) -> Option<&OdinValue> {
        match self {
            OdinValue::Attrs(map) => map.get(name),
            _ => None,
        }
    }

    pub(super) fn keyed_entries(&self) -> impl Iterator<Item = (&str, &OdinValue)> {
        let map = match self {
            OdinValue::Keyed(map) => Some(map),
            _ => None,
        };
        map.into_iter().flatten().map(|(k, v)| (k.as_str(), v))
    }

    pub(super) fn keys(&self) -> std::collections::HashSet<String> {
        match self {
            OdinValue::Keyed(map) => map.keys().cloned().collect(),
            _ => std::collections::HashSet::new(),
        }
    }

    pub(super) fn code_string(&self) -> Option<String> {
        match self {
            OdinValue::Code(c) => Some(c.clone()),
            _ => None,
        }
    }

    pub(super) fn string_items(&self) -> Vec<&String> {
        match self {
            OdinValue::Strings(items) => items.iter().collect(),
            _ => Vec::new(),
        }
    }
}

struct OdinParser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl OdinParser<'_> {
    fn skip_ws(&mut self) {
        while self.pos < self.src.len() {
            match self.src[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' | b';' | b',' => self.pos += 1,
                b'-' if self.src.get(self.pos + 1) == Some(&b'-') => {
                    while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    /// A sequence of `name = <…>` and/or `["key"] = <…>` entries, ending at
    /// end-of-input or a closing `>`.
    fn attrs_block(&mut self) -> Option<OdinValue> {
        let mut attrs = BTreeMap::new();
        let mut keyed = BTreeMap::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None | Some(b'>') => break,
                Some(b'[') => {
                    let key = self.bracket_key()?;
                    self.expect_eq()?;
                    let value = self.angle_value()?;
                    keyed.insert(key, value);
                }
                _ => {
                    let name = self.identifier()?;
                    self.expect_eq()?;
                    let value = self.angle_value()?;
                    attrs.insert(name, value);
                }
            }
        }
        if !keyed.is_empty() && attrs.is_empty() {
            Some(OdinValue::Keyed(keyed))
        } else {
            Some(OdinValue::Attrs(attrs))
        }
    }

    /// `["key"]` or `[key]` → the key text.
    fn bracket_key(&mut self) -> Option<String> {
        debug_assert_eq!(self.peek(), Some(b'['));
        self.pos += 1;
        self.skip_ws();
        let key = if self.peek() == Some(b'"') {
            self.quoted_string()?
        } else {
            let start = self.pos;
            while self.pos < self.src.len() && self.src[self.pos] != b']' {
                self.pos += 1;
            }
            String::from_utf8_lossy(&self.src[start..self.pos])
                .trim()
                .to_owned()
        };
        self.skip_ws();
        if self.peek() != Some(b']') {
            return None;
        }
        self.pos += 1;
        Some(key)
    }

    fn identifier(&mut self) -> Option<String> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.pos += 1;
        }
        (self.pos > start).then(|| String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
    }

    fn expect_eq(&mut self) -> Option<()> {
        self.skip_ws();
        if self.peek() != Some(b'=') {
            return None;
        }
        self.pos += 1;
        Some(())
    }

    /// `<…>` — a nested block, a string (list), a code, or an unmodelled leaf.
    fn angle_value(&mut self) -> Option<OdinValue> {
        self.skip_ws();
        if self.peek() != Some(b'<') {
            return None;
        }
        self.pos += 1;
        self.skip_ws();
        let value = match self.peek() {
            // Nested keyed/attr block.
            Some(b'[') if self.looks_like_keyed_entry() => self.attrs_block()?,
            Some(b'"') => {
                let mut items = vec![self.quoted_string()?];
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b'"') {
                        items.push(self.quoted_string()?);
                    } else {
                        break;
                    }
                }
                OdinValue::Strings(items)
            }
            Some(b'[') => {
                // `<[ISO_639-1::en]>` — a code leaf.
                OdinValue::Code(self.bracket_key()?)
            }
            Some(b'>') => OdinValue::Attrs(BTreeMap::new()),
            Some(b) if b.is_ascii_alphabetic() && self.looks_like_attr_entry() => {
                self.attrs_block()?
            }
            _ => {
                // Unmodelled leaf (number, uri, interval, …): consume to the
                // matching `>` at this nesting level.
                let mut depth = 0usize;
                while let Some(b) = self.peek() {
                    match b {
                        b'<' => depth += 1,
                        b'>' if depth == 0 => break,
                        b'>' => depth -= 1,
                        _ => {}
                    }
                    self.pos += 1;
                }
                OdinValue::Other
            }
        };
        self.skip_ws();
        if self.peek() != Some(b'>') {
            return None;
        }
        self.pos += 1;
        Some(value)
    }

    /// `["…"] =` lookahead (vs a code leaf `[…]>`).
    fn looks_like_keyed_entry(&self) -> bool {
        let rest = &self.src[self.pos..];
        let Some(close) = rest.iter().position(|b| *b == b']') else {
            return false;
        };
        rest[close + 1..]
            .iter()
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|b| *b == b'=')
    }

    /// `name =` lookahead (vs a bare-word leaf like `true`).
    fn looks_like_attr_entry(&self) -> bool {
        let rest = &self.src[self.pos..];
        let end = rest
            .iter()
            .position(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
            .unwrap_or(rest.len());
        rest[end..]
            .iter()
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|b| *b == b'=')
    }

    fn quoted_string(&mut self) -> Option<String> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.src.len() && self.src[self.pos] != b'"' {
            if self.src[self.pos] == b'\\' {
                self.pos += 1;
            }
            self.pos += 1;
        }
        if self.pos >= self.src.len() {
            return None;
        }
        let s = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
        self.pos += 1;
        Some(s)
    }
}
