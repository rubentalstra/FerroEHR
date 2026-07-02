//! `BASIC_DEFINITIONS` — globally used constant values.
//!
//! openEHR class: `BASIC_DEFINITIONS`, package `base.base_types.definitions`.
//!
//! Defines globally used constant values: the carriage-return and line-feed
//! characters, and a handful of well-known strings used elsewhere in the
//! openEHR specifications (the type name conventionally used for the "any"
//! type and for absent/`None` values, the default text encoding, and the
//! canonical "match anything" regular expression pattern).

/// `BASIC_DEFINITIONS` declares no attributes, only constants, so it is
/// transcribed as a Rust struct with no fields and one associated `const`
/// per spec constant, per the transcription guidance for constant-holder
/// classes (associated consts, not an instantiable value type).
///
/// `OPENEHR_DEFINITIONS` (`openehr_definitions.rs`, same package) inherits
/// this class to gain access to these constants alongside its own; the Rust
/// transcription mirrors that by having `OpenehrDefinitions` re-export these
/// same associated consts (see the PORT NOTE there), since Rust has no
/// struct-level inheritance to fall back on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicDefinitions;

impl BasicDefinitions {
    /// `CR`: `char = '\015'`.
    ///
    /// Carriage return character. The spec's `'\015'` is an Eiffel-style
    /// octal character-code escape (octal 15 = decimal 13), i.e. the
    /// standard ASCII carriage-return control character, `'\r'` in Rust
    /// source notation.
    pub const CR: char = '\r';

    /// `LF`: `char = '\012'`.
    ///
    /// Line feed character. The spec's `'\012'` is octal 12 = decimal 10,
    /// the standard ASCII line-feed control character, `'\n'` in Rust
    /// source notation.
    pub const LF: char = '\n';

    /// `Any_type_name`: `String = "Any"`.
    pub const ANY_TYPE_NAME: &'static str = "Any";

    /// `Regex_any_pattern`: `String = ".*"`.
    pub const REGEX_ANY_PATTERN: &'static str = ".*";

    /// `Default_encoding`: `String = "UTF-8"`.
    pub const DEFAULT_ENCODING: &'static str = "UTF-8";

    /// `None_type_name`: `String = "None"`.
    pub const NONE_TYPE_NAME: &'static str = "None";
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/basic_definitions.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-definitions_package.adoc §Class Definitions / basic_definitions.adoc §BASIC_DEFINITIONS Class
//   confidence: high
//   todos: 0
//   note: constant-holder class transcribed as a zero-field struct with associated consts; CR/LF decoded from the spec's Eiffel octal escapes (\015 = 13 = '\r', \012 = 10 = '\n').
// ─────────────────────────────────────────────
