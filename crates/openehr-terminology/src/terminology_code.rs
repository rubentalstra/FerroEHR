//! Lightweight terminology code value used by the service API surface.

/// A (terminology id, code string) pair as returned by the terminology
/// service lookups.
///
/// PORT NOTE: the RM 1.1.0 support spec types these returns as RM
/// `CODE_PHRASE`, which is transcribed in P3 into `openehr-rm` — a crate that
/// depends on this one, so it cannot be referenced here. BASE 1.2.0's own
/// `Terminology_code` (transcribed, still unwired, in
/// `openehr-foundation/src/terminology_types/terminology_code.rs`) is the
/// spec-native equivalent shape. This local struct mirrors that shape's two
/// load-bearing fields; reconcile the three at P17 module wiring.
/// `// TODO(port):` swap service signatures to the wired spec type at P17.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerminologyCode {
    /// Identifier of the owning terminology or code set (e.g. `openehr`,
    /// `ISO_639-1`).
    pub terminology_id: String,
    /// The code itself (e.g. `249`, `en`, `gzip`).
    pub code_string: String,
}

impl TerminologyCode {
    /// Convenience constructor.
    pub fn new(terminology_id: impl Into<String>, code_string: impl Into<String>) -> Self {
        Self {
            terminology_id: terminology_id.into(),
            code_string: code_string.into(),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §terminology_package (CODE_PHRASE-typed returns) — docs/research/spec-cache/RM-1.1.0/support/ (Release-1.1.0 @ 3cbd85b)
//   source_loc: terminology_access.adoc / code_set_access.adoc signatures
//   confidence: medium
//   todos: 1
//   note: deliberate local stand-in for CODE_PHRASE / BASE Terminology_code until P3/P17; see PORT NOTE above
// ─────────────────────────────────────────────
