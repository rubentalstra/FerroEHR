//! `ARCHETYPE_ID` — identifier for archetypes.
//!
//! openEHR class: `ARCHETYPE_ID`, package `base.base_types.identification`.
//! Inherits: `OBJECT_ID`.
//!
//! Identifier for archetypes. Ideally these would identify globally unique
//! archetypes.
//!
//! Lexical form: `rm_originator '-' rm_name '-' rm_entity '.' concept_name
//! { '-' specialisation }* '.v' number`.
use openehr_foundation::serde_support::{TypeName, TypeTag};

use super::object_id::{ObjectId, ObjectIdApi, ObjectIdData};

/// Canonical `_type` discriminator string for this class in serialized
/// form. P4/ADR-002 update: this const single-sources the string carried by
/// the struct's own self-tagging `type_tag` field below (via the
/// [`TypeName`] impl), so every serialized `ArchetypeId` — bare or reached
/// through the untagged `ObjectId::ArchetypeId` variant — emits
/// `{"_type": "ARCHETYPE_ID", ...}` itself.
pub const TYPE_NAME: &str = "ARCHETYPE_ID";

/// `ARCHETYPE_ID` declares no attribute of its own beyond the inherited
/// `value: String` from `OBJECT_ID`; its six functions
/// (`qualified_rm_entity`, `domain_concept`, `rm_originator`, `rm_name`,
/// `rm_entity`, `specialisation`, `version_id`) all parse substrings of
/// that one attribute, so it embeds `ObjectIdData` verbatim (ADR-001 §3).
///
/// `#[serde(flatten)]` on the embedded `object_id` field folds
/// `ObjectIdData`'s single `value` attribute directly into this struct's
/// JSON object.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ArchetypeId {
    /// Canonical `_type` discriminator (`"ARCHETYPE_ID"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `OBJECT_ID` state (the single `value` attribute), in the
    /// lexical form `rm_originator '-' rm_name '-' rm_entity '.'
    /// concept_name { '-' specialisation }* '.v' number`.
    #[serde(flatten)]
    pub object_id: ObjectIdData,
}

impl TypeName for ArchetypeId {
    const NAME: &'static str = TYPE_NAME;
}

impl ArchetypeId {
    /// `value`: the raw archetype identifier string.
    pub fn value(&self) -> &str {
        &self.object_id.value
    }

    /// `qualified_rm_entity(): String`.
    ///
    /// Globally qualified reference model entity, e.g.
    /// `openehr-EHR-OBSERVATION`. The part of `value` before the first `.`.
    pub fn qualified_rm_entity(&self) -> String {
        self.value()
            .split_once('.')
            .map(|(head, _rest)| head.to_string())
            .unwrap_or_default()
    }

    /// `rm_originator(): String`.
    ///
    /// Organisation originating the reference model on which this
    /// archetype is based, e.g. `openehr`, `cen`, `hl7`. The first
    /// `-`-delimited part of [`ArchetypeId::qualified_rm_entity`].
    pub fn rm_originator(&self) -> String {
        self.qualified_rm_entity()
            .split('-')
            .next()
            .unwrap_or_default()
            .to_string()
    }

    /// `rm_name(): String`.
    ///
    /// Name of the reference model, e.g. `rim`, `ehr_rm`, `en13606`. The
    /// second `-`-delimited part of
    /// [`ArchetypeId::qualified_rm_entity`].
    pub fn rm_name(&self) -> String {
        let qualified = self.qualified_rm_entity();
        let mut parts = qualified.splitn(3, '-').skip(1);
        parts.next().unwrap_or_default().to_string()
    }

    /// `rm_entity(): String`.
    ///
    /// Name of the ontological level within the reference model to which
    /// this archetype is targeted, e.g. for openEHR, `folder`,
    /// `composition`, `section`, `entry`. The third `-`-delimited part of
    /// [`ArchetypeId::qualified_rm_entity`].
    pub fn rm_entity(&self) -> String {
        let qualified = self.qualified_rm_entity();
        let mut parts = qualified.splitn(3, '-').skip(2);
        parts.next().unwrap_or_default().to_string()
    }

    /// `domain_concept(): String`.
    ///
    /// Name of the concept represented by this archetype, including
    /// specialisation, e.g. `Biochemistry_result-cholesterol`. The middle
    /// `.`-delimited part of `value` (between `qualified_rm_entity` and
    /// `version_id`).
    pub fn domain_concept(&self) -> String {
        let mut parts = self.value().splitn(3, '.');
        let _qualified_rm_entity = parts.next();
        parts.next().unwrap_or_default().to_string()
    }

    /// `specialisation(): String`.
    ///
    /// Name of specialisation of concept, if this archetype is a
    /// specialisation of another archetype, e.g. `cholesterol`. The part of
    /// [`ArchetypeId::domain_concept`] after its first `-`, if any.
    pub fn specialisation(&self) -> String {
        self.domain_concept()
            .split_once('-')
            .map(|(_concept_name, specialisation)| specialisation.to_string())
            .unwrap_or_default()
    }

    /// `version_id(): String`.
    ///
    /// Version of this archetype. The final `.`-delimited part of `value`,
    /// of the form `v0` or `v` followed by a non-zero-leading number, per
    /// the `version-id` grammar production.
    pub fn version_id(&self) -> String {
        self.value()
            .rsplit_once('.')
            .map(|(_head, version)| version.to_string())
            .unwrap_or_default()
    }
}

impl ObjectIdApi for ArchetypeId {
    fn value(&self) -> &str {
        &self.object_id.value
    }
}

impl From<ArchetypeId> for ObjectId {
    fn from(value: ArchetypeId) -> Self {
        ObjectId::ArchetypeId(value)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §ARCHETYPE_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/archetype_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / archetype_id.adoc §ARCHETYPE_ID Class
//   confidence: medium
//   todos: 0
//   note: multi-part axis functions implemented as string-splitting against the EBNF grammar in the Syntaxes section rather than a dedicated parser/AST; no invariant table given in the spec for this class beyond the lexical grammar itself. P4/ADR-002: self-tags via TypeTag<Self> first field (NAME single-sourced from TYPE_NAME); inert struct-level #[serde(rename)] deleted; the earlier "no wire path emits _type" TODO is resolved by the self-tag.
// ─────────────────────────────────────────────
