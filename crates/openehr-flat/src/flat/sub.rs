//! FLAT-key parsing and the per-leaf [`FlatView`] used by the reverse mappers.
//!
//! A FLAT key is `seg0/seg1/…/segN[|suffix]` where each `seg` is `id[:index]`
//! (Better `WebTemplatePathSegment`). The reverse conversion groups entries by
//! their leading segment as it walks the web-template tree; at a leaf, the
//! remaining entries are the `|suffix` (and bare) datum parts collected into a
//! [`FlatView`].

use serde_json::Value;

/// One `id[:index]` segment of a FLAT key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeySeg {
    pub id: String,
    pub index: Option<usize>,
}

/// A parsed FLAT entry: its path segments, an optional `|suffix`, and the value.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub segs: Vec<KeySeg>,
    pub suffix: Option<String>,
    pub value: Value,
}

/// Parse a FLAT key `"a/b:0/c|magnitude"` into `(segments, suffix)`.
pub(crate) fn parse_key(key: &str) -> (Vec<KeySeg>, Option<String>) {
    let (path, suffix) = match key.split_once('|') {
        Some((p, s)) => (p, Some(s.to_owned())),
        None => (key, None),
    };
    let segs = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| match s.split_once(':') {
            Some((id, idx)) => KeySeg {
                id: id.to_owned(),
                index: idx.parse().ok(),
            },
            None => KeySeg {
                id: s.to_owned(),
                index: None,
            },
        })
        .collect();
    (segs, suffix)
}

/// A flattened view of the datum parts at one leaf: the suffixed entries plus
/// the bare (no-suffix) value.
pub(crate) struct FlatView<'a> {
    entries: &'a [Entry],
}

impl<'a> FlatView<'a> {
    pub(crate) fn new(entries: &'a [Entry]) -> Self {
        Self { entries }
    }

    /// The value for `|suffix`, if present.
    ///
    /// A datum part reaches a leaf-relative view either as an explicit `|suffix`
    /// (empty path, `Some(name)`) or — when the caller addresses it by its bare
    /// leaf token with no remaining path — as a single index-less path segment
    /// (`segs == [name]`, no suffix). Both forms name the same datum part; the
    /// only single-segment entries a real leaf view also carries are the
    /// `_`-prefixed RM-attribute entries, which no leaf mapper queries by name.
    pub(crate) fn suffix(&self, name: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|e| {
                (e.suffix.as_deref() == Some(name) && e.segs.is_empty())
                    || (e.suffix.is_none()
                        && matches!(e.segs.as_slice(), [seg] if seg.id == name && seg.index.is_none()))
            })
            .map(|e| &e.value)
    }

    /// The bare (no-suffix) value, if present.
    pub(crate) fn bare(&self) -> Option<&Value> {
        self.entries
            .iter()
            .find(|e| e.suffix.is_none() && e.segs.is_empty())
            .map(|e| &e.value)
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn parses_indexed_key_with_suffix() {
        let (segs, suffix) = parse_key("vitals/blood_pressure:0/any_event:1/systolic|magnitude");
        assert_eq!(segs.len(), 4);
        assert_eq!(
            segs[0],
            KeySeg {
                id: "vitals".into(),
                index: None
            }
        );
        assert_eq!(
            segs[1],
            KeySeg {
                id: "blood_pressure".into(),
                index: Some(0)
            }
        );
        assert_eq!(
            segs[3],
            KeySeg {
                id: "systolic".into(),
                index: None
            }
        );
        assert_eq!(suffix.as_deref(), Some("magnitude"));
    }

    #[test]
    fn parses_bare_key() {
        let (segs, suffix) = parse_key("vitals/note");
        assert_eq!(segs.len(), 2);
        assert!(suffix.is_none());
    }
}
