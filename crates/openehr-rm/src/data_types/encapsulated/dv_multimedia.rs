//! `DV_MULTIMEDIA` — a specialisation of `DV_ENCAPSULATED` for audiovisual
//! and bio-signal data.
//!
//! openEHR class: `DV_MULTIMEDIA`, package `rm.data_types.encapsulated`.
//! Inherits: `DV_ENCAPSULATED`.
//!
//! A specialisation of `DV_ENCAPSULATED` for audiovisual and bio-signal
//! types. Includes further metadata relating to multimedia types which are
//! not applicable to other subtypes of `DV_ENCAPSULATED`.
//!
//! # Recursion hazard: `thumbnail: DV_MULTIMEDIA`
//!
//! PORT_MASTER_PLAN.md §7.2 explicitly names `DV_MULTIMEDIA.thumbnail`
//! among the recursive-containment cases that must be boxed
//! (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`,
//! `DV_MULTIMEDIA.thumbnail`), and ADR-001 §8 restates the same rule
//! (recursive containment → `Box`). `thumbnail` is declared `0..1` in the
//! spec table, so it is `Option<Box<DvMultimedia>>` rather than a bare
//! `Box<DvMultimedia>` — see the field doc below.
//!
//! # Forward references
//!
//! `DV_URI` (`data_types.uri` package) and `CODE_PHRASE`
//! (`data_types.text` package) are both transcribed by concurrent
//! transcription passes not yet landed at the time of this file, and are
//! imported directly by their eventual module paths per the invoking
//! task's instruction.
use crate::data_types::encapsulated::dv_encapsulated::{DvEncapsulated, DvEncapsulatedData};
use crate::data_types::text::code_phrase::CodePhrase;
use crate::data_types::uri::dv_uri::DvUri;

/// `DV_MULTIMEDIA`.
///
/// openEHR class: `DV_MULTIMEDIA`.
#[derive(Debug, Clone, PartialEq)]
pub struct DvMultimedia {
    /// Embedded `DV_ENCAPSULATED` state (`charset`, `language`).
    pub encapsulated: DvEncapsulatedData,

    /// `alternate_text`: `String` (`0..1`).
    ///
    /// Text to display in lieu of multimedia display/replay.
    pub alternate_text: Option<String>,

    /// `uri`: `DV_URI` (`0..1`).
    ///
    /// URI reference to electronic information stored outside the record as
    /// a file, database entry etc, if supplied as a reference.
    pub uri: Option<DvUri>,

    /// `data`: `List<Byte>` (`0..1`).
    ///
    /// The actual data found at `uri`, if supplied inline.
    ///
    /// Per `docs/PORTING.md` §14.2 (`byte[]` → `Vec<u8>`) and the RM
    /// transcription rule's settled `Octet`-not-"Byte" hazard, the spec's
    /// `List<Byte>` is transcribed as `Vec<u8>` — a flat byte buffer, not
    /// `Vec<Octet>`/`openehr_foundation::structure_types::list::List<Octet>`,
    /// since the RM-level attribute here is genuinely raw inline binary
    /// content (the multimedia payload itself), not a foundation-types
    /// `List` value being manipulated with `Container`/`List` operations —
    /// matching this crate's established `Vec<u8>` convention for
    /// byte-buffer-shaped RM attributes.
    pub data: Option<Vec<u8>>,

    /// `media_type`: `CODE_PHRASE` (`1..1`).
    ///
    /// Data media type coded from openEHR code set "media types" (interface
    /// for the IANA MIME types code set).
    pub media_type: CodePhrase,

    /// `compression_algorithm`: `CODE_PHRASE` (`0..1`).
    ///
    /// Compression type, a coded value from the openEHR Integrity check
    /// code set. `Void`/`None` means no compression.
    pub compression_algorithm: Option<CodePhrase>,

    /// `integrity_check`: `List<Byte>` (`0..1`).
    ///
    /// Binary cryptographic integrity checksum.
    ///
    /// Same `Vec<u8>` byte-buffer transcription as `data` above.
    pub integrity_check: Option<Vec<u8>>,

    /// `integrity_check_algorithm`: `CODE_PHRASE` (`0..1`).
    ///
    /// Type of integrity check, a coded value from the openEHR
    /// `Integrity check` code set.
    pub integrity_check_algorithm: Option<CodePhrase>,

    /// `thumbnail`: `DV_MULTIMEDIA` (`0..1`).
    ///
    /// The thumbnail for this item, if one exists; mainly for graphics
    /// formats.
    ///
    /// Boxed per PORT_MASTER_PLAN.md §7.2 and ADR-001 §8's recursive-
    /// containment rule — `DV_MULTIMEDIA` containing an optional
    /// `DV_MULTIMEDIA` field would otherwise be an infinitely-sized type.
    /// `Option` (not a bare `Box`) reflects the spec's `0..1` cardinality:
    /// most instances have no thumbnail at all.
    pub thumbnail: Option<Box<DvMultimedia>>,

    /// `size`: `Integer` (`1..1`).
    ///
    /// Original size in bytes of unencoded encapsulated data. I.e.
    /// encodings such as base64, hexadecimal etc do not change the value of
    /// this attribute.
    pub size: i32,
}

pub const TYPE_NAME: &str = "DV_MULTIMEDIA";

impl DvMultimedia {
    /// `is_external` `(): Boolean`.
    ///
    /// Computed from the value of the `uri` attribute: `true` if the data
    /// is stored externally to the record, as indicated by `uri`. A copy
    /// may also be stored internally, in which case `is_inline` is also
    /// true.
    pub fn is_external(&self) -> bool {
        self.uri.is_some()
    }

    /// `is_inline` `(): Boolean`.
    ///
    /// Computed from the value of the `data` attribute. `true` if the data
    /// is stored in expanded form, i.e. within the EHR itself.
    pub fn is_inline(&self) -> bool {
        self.data.is_some()
    }

    /// `is_compressed` `(): Boolean`.
    ///
    /// Computed from the value of the `compression_algorithm` attribute:
    /// `true` if the data is stored in compressed form.
    pub fn is_compressed(&self) -> bool {
        self.compression_algorithm.is_some()
    }

    /// `has_integrity_check` `(): Boolean`.
    ///
    /// Computed from the value of the `integrity_check_algorithm`
    /// attribute: `true` if an integrity check has been computed.
    pub fn has_integrity_check(&self) -> bool {
        self.integrity_check_algorithm.is_some()
    }

    /// `Not_empty` invariant: `is_inline or is_external`.
    pub fn invariant_not_empty(&self) -> bool {
        self.is_inline() || self.is_external()
    }

    /// `Media_type_valid` invariant:
    /// `media_type /= Void and then code_set(Code_set_id_media_types).has_code(media_type)`.
    ///
    /// PORT NOTE: the published condition's `media_type /= Void` conjunct is
    /// unreachable for this struct, since `media_type` is declared `1..1`
    /// (non-optional, `CodePhrase` not `Option<CodePhrase>`) here — matching
    /// the class table's own `1..1` cardinality for the attribute, which
    /// makes the null-check clause of the invariant dead code by
    /// construction rather than a runtime condition to check. Transcribed
    /// as the remaining, load-bearing `has_code` check only.
    ///
    /// TODO(port): requires a live `TERMINOLOGY_SERVICE`/`code_set` lookup
    /// bound to `OPENEHR_CODE_SET_IDENTIFIERS::MEDIA_TYPES` — same
    /// terminology-service threading gap noted on
    /// `DvEncapsulated::invariant_language_valid`/`invariant_charset_valid`.
    pub fn invariant_media_type_valid(&self) -> bool {
        todo!(
            "DV_MULTIMEDIA.invariant_media_type_valid: requires a TerminologyService code_set(media_types) lookup, not yet threaded through a Validate context"
        )
    }

    /// `Compression_algorithm_validity` invariant:
    /// `compression_algorithm /= Void implies code_set(Code_set_id_compression_algorithms).has_code(compression_algorithm)`.
    ///
    /// TODO(port): same terminology-service threading gap as
    /// `invariant_media_type_valid` above, bound to
    /// `OPENEHR_CODE_SET_IDENTIFIERS::COMPRESSION_ALGORITHMS`.
    pub fn invariant_compression_algorithm_validity(&self) -> bool {
        if self.compression_algorithm.is_none() {
            return true;
        }
        todo!(
            "DV_MULTIMEDIA.invariant_compression_algorithm_validity: requires a TerminologyService code_set(compression_algorithms) lookup, not yet threaded through a Validate context"
        )
    }

    /// `Integrity_check_validity` invariant:
    /// `integrity_check /= Void implies integrity_check_algorithm /= Void`.
    ///
    /// Fully implementable without a terminology-service lookup — a plain
    /// structural presence check.
    pub fn invariant_integrity_check_validity(&self) -> bool {
        if self.integrity_check.is_some() {
            self.integrity_check_algorithm.is_some()
        } else {
            true
        }
    }

    /// `Integrity_check_algorithm_validity` invariant:
    /// `integrity_check_algorithm /= Void implies code_set(Code_set_id_integrity_check_algorithms).has_code(integrity_check_algorithm)`.
    ///
    /// TODO(port): same terminology-service threading gap as
    /// `invariant_media_type_valid` above, bound to
    /// `OPENEHR_CODE_SET_IDENTIFIERS::INTEGRITY_CHECK_ALGORITHMS`.
    pub fn invariant_integrity_check_algorithm_validity(&self) -> bool {
        if self.integrity_check_algorithm.is_none() {
            return true;
        }
        todo!(
            "DV_MULTIMEDIA.invariant_integrity_check_algorithm_validity: requires a TerminologyService code_set(integrity_check_algorithms) lookup, not yet threaded through a Validate context"
        )
    }

    /// `Size_valid` invariant: `size >= 0`.
    ///
    /// Fully implementable without a terminology-service lookup.
    pub fn invariant_size_valid(&self) -> bool {
        self.size >= 0
    }
}

impl DvEncapsulated for DvMultimedia {
    fn encapsulated_data(&self) -> &DvEncapsulatedData {
        &self.encapsulated
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.encapsulated — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_multimedia.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master09-encapsulated_package.adoc §Class Descriptions / dv_multimedia.adoc §DV_MULTIMEDIA Class
//   confidence: high
//   todos: 3
//   note: the named §7.2 recursion hazard — thumbnail: Option<Box<DvMultimedia>> per ADR-001 §8; data/integrity_check transcribed Vec<u8> per the List<Byte>->byte-buffer convention, not List<Octet>; four computed is_*/has_* functions and two structural invariants (Integrity_check_validity, Size_valid) are fully implemented; the three code-set-membership invariants (Media_type_valid, Compression_algorithm_validity, Integrity_check_algorithm_validity) are stubbed todo!() pending a TerminologyService lookup not yet threaded through any Validate-context signature; forward-references DvUri and CodePhrase from concurrent, not-yet-landed transcription passes.
// ─────────────────────────────────────────────
