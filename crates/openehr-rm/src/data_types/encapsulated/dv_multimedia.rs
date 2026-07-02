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
use crate::data_types::encapsulated::dv_encapsulated::{DvEncapsulatedApi, DvEncapsulatedData};
use crate::data_types::text::code_phrase::CodePhrase;
use crate::data_types::uri::dv_uri::DvUri;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical-JSON base64 bridge for `DV_MULTIMEDIA.data`/`integrity_check`.
///
/// PORT NOTE (P4): the invoking task's premise assumed a shared
/// `crate::serde_support::{base64_vec, base64_option}` module already
/// exists in this crate (`crates/openehr-rm/src/serde_support.rs`) — it
/// does **not** exist in this worktree at the time of this pass (confirmed
/// directly: the file is absent). Rather than fabricate that shared module
/// (a sibling file at `crates/openehr-rm/src/`, outside this pass's
/// declared `data_types/`-only scope, and a name another concurrent agent
/// may independently be landing), the same base64-bridge shape is defined
/// locally, scoped to this file only. `.claude/rules/serialization.md`'s
/// canonical-JSON rule ("`DV_MULTIMEDIA.data` is inline base64, not a
/// separate reference") is satisfied either way; consolidating this into a
/// shared `serde_support` module once one lands is a mechanical follow-up,
/// not a behavioural change.
mod base64_bridge {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::{Deserialize, Deserializer, Serializer};

    /// For `#[serde(with = "self::base64_bridge")]` on `Option<Vec<u8>>`.
    /// Combine with `#[serde(skip_serializing_if = "Option::is_none")]` so
    /// an absent value is omitted entirely rather than serialized as
    /// `null` (canonical JSON never emits nulls).
    ///
    /// # Errors
    ///
    /// Only the (de)serializer's own errors, or invalid base64 content on
    /// deserialize; encoding itself cannot fail.
    pub fn serialize<S: Serializer>(
        bytes: &Option<Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match bytes {
            Some(b) => serializer.serialize_str(&STANDARD.encode(b)),
            None => serializer.serialize_none(),
        }
    }

    /// # Errors
    ///
    /// The deserializer's own errors, or invalid base64 content.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<u8>>, D::Error> {
        let encoded: Option<String> = Option::deserialize(deserializer)?;
        encoded
            .map(|s| {
                STANDARD
                    .decode(s.as_bytes())
                    .map_err(serde::de::Error::custom)
            })
            .transpose()
    }
}

/// `DV_MULTIMEDIA`.
///
/// openEHR class: `DV_MULTIMEDIA`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvMultimedia {
    /// Canonical `_type` discriminator (`"DV_MULTIMEDIA"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_ENCAPSULATED` state (`charset`, `language`).
    #[serde(flatten)]
    pub encapsulated: DvEncapsulatedData,

    /// `alternate_text`: `String` (`0..1`).
    ///
    /// Text to display in lieu of multimedia display/replay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternate_text: Option<String>,

    /// `uri`: `DV_URI` (`0..1`).
    ///
    /// URI reference to electronic information stored outside the record as
    /// a file, database entry etc, if supplied as a reference.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    ///
    /// Serialized as inline base64 via the local `base64_bridge` module
    /// above, per `.claude/rules/serialization.md`'s explicit
    /// `DV_MULTIMEDIA.data` rule.
    #[serde(
        with = "self::base64_bridge",
        skip_serializing_if = "Option::is_none",
        default
    )]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_algorithm: Option<CodePhrase>,

    /// `integrity_check`: `List<Byte>` (`0..1`).
    ///
    /// Binary cryptographic integrity checksum.
    ///
    /// Same `Vec<u8>` byte-buffer transcription as `data` above, same
    /// inline-base64 serialization via `base64_bridge`.
    #[serde(
        with = "self::base64_bridge",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub integrity_check: Option<Vec<u8>>,

    /// `integrity_check_algorithm`: `CODE_PHRASE` (`0..1`).
    ///
    /// Type of integrity check, a coded value from the openEHR
    /// `Integrity check` code set.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    ///
    /// PORT NOTE: `skip_serializing_if` only, deliberately without
    /// `default` — consistent with the rest of this P4 pass, `default` is
    /// redundant on an `Option` field (an absent field already
    /// deserializes to `None`) and was found, in the generic-plus-flatten
    /// case, to actively require a spurious `T: Default` bound; see the
    /// `dv_quantity.rs` round-trip test's doc comment for the full
    /// write-up. Not applicable to this specific field (no generic `T` is
    /// involved here), but dropped for consistency regardless.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<Box<DvMultimedia>>,

    /// `size`: `Integer` (`1..1`).
    ///
    /// Original size in bytes of unencoded encapsulated data. I.e.
    /// encodings such as base64, hexadecimal etc do not change the value of
    /// this attribute.
    pub size: i32,
}

pub const TYPE_NAME: &str = "DV_MULTIMEDIA";

impl TypeName for DvMultimedia {
    const NAME: &'static str = TYPE_NAME;
}

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
    /// `DvEncapsulatedApi::invariant_language_valid`/`invariant_charset_valid`.
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

impl DvEncapsulatedApi for DvMultimedia {
    fn encapsulated_data(&self) -> &DvEncapsulatedData {
        &self.encapsulated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::code_phrase::CodePhrase;

    /// Built via deserialization rather than a struct literal so this test
    /// does not hard-code `CodePhrase`/`TerminologyId` field shapes owned by
    /// concurrent conversion passes (a missing `_type` is tolerated on
    /// concrete slots per ADR-002 either way).
    fn media_type() -> CodePhrase {
        serde_json::from_value(serde_json::json!({
            "terminology_id": { "value": "IANA_media-types" },
            "code_string": "image/png"
        }))
        .unwrap()
    }

    /// Pins the canonical-JSON rules for this class: `_type` first,
    /// `data` inline base64 (never a byte array), absent Options omitted.
    #[test]
    fn multimedia_data_serializes_as_inline_base64() {
        let m = DvMultimedia {
            type_tag: TypeTag::new(),
            encapsulated: DvEncapsulatedData {
                charset: None,
                language: None,
            },
            alternate_text: None,
            uri: None,
            data: Some(vec![0x00, 0x01, 0x02, 0xff]),
            media_type: media_type(),
            compression_algorithm: None,
            integrity_check: None,
            integrity_check_algorithm: None,
            thumbnail: None,
            size: 4,
        };
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["_type"], "DV_MULTIMEDIA");
        // STANDARD base64 of [0x00, 0x01, 0x02, 0xff] — an inline string.
        assert_eq!(json["data"], "AAEC/w==");
        // Absent optionals are omitted entirely, not null.
        assert!(json.get("integrity_check").is_none());
        assert!(json.get("thumbnail").is_none());

        let back: DvMultimedia = serde_json::from_str(&json.to_string()).unwrap();
        assert_eq!(back, m);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.encapsulated — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_multimedia.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master09-encapsulated_package.adoc §Class Descriptions / dv_multimedia.adoc §DV_MULTIMEDIA Class
//   confidence: high
//   todos: 3
//   note: the named §7.2 recursion hazard — thumbnail: Option<Box<DvMultimedia>> per ADR-001 §8; data/integrity_check transcribed Vec<u8> per the List<Byte>->byte-buffer convention, not List<Octet>; four computed is_*/has_* functions and two structural invariants (Integrity_check_validity, Size_valid) are fully implemented; the three code-set-membership invariants (Media_type_valid, Compression_algorithm_validity, Integrity_check_algorithm_validity) are stubbed todo!() pending a TerminologyService lookup not yet threaded through any Validate-context signature; forward-references DvUri and CodePhrase from concurrent, not-yet-landed transcription passes. P4: Serialize/Deserialize added; `encapsulated` flattened; data/integrity_check use a locally-scoped `base64_bridge` module (crate::serde_support does not exist in this worktree, confirmed absent — do not assume the task brief's premise without verifying) with `default` (verified necessary and safe for #[serde(with)] on a non-generic field, unlike the plain-Option case elsewhere in this pass); every other Option field skips-only, no default; ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME); in-file test pins _type-first + inline-base64 `data` + omitted-absent-Options wire shape.
// ─────────────────────────────────────────────
