// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The **construction-door** decision map: which generated spec classes hide
//! their fields behind a validating constructor, and which stay plain records.
//!
//! # The scheme
//!
//! The generator emits a spec class as a plain data record by default: every
//! field `pub`, construction by struct literal. That is correct wherever the
//! released spec states no constraint over the field values — privacy there
//! would guarantee nothing and only add ceremony.
//!
//! It is *wrong* wherever the released spec states a **constraint over the
//! field values that is decidable from those fields alone**. Two shapes qualify:
//!
//! * a **lexical form** — BASE `base_types/master05-identification_package.adoc`
//!   §Syntaxes gives an EBNF grammar for every identifier class, so a
//!   `HIER_OBJECT_ID` whose `value` is `"1234-5678"` is not a valid instance of
//!   the spec type at all;
//! * a **class invariant over its own fields** — RM
//!   `UML/classes/org.openehr.rm.common.item_tag.adoc` §Invariants states
//!   `Inv_key_valid` and `Inv_value_valid` over `ITEM_TAG.key`/`.value`, so an
//!   `ITEM_TAG` with an empty key is likewise not an instance of the class.
//!
//! With a public field the type can hold the invalid value anyway, and the
//! validating constructor beside it is merely *a* door rather than *the* door.
//!
//! The boundary is decidability *from the fields*: an invariant that needs
//! anything the instance does not carry — terminology lookup, the existence of
//! the `target` an `ITEM_TAG` points at, a parent's node id — cannot move to the
//! door and stays at the `Validate` tier or in the service layer.
//!
//! An entry with [`Door::Validated`] closes that: the emitter renders the
//! class's fields `pub(crate)` and emits read accessors, so **outside the
//! defining crate construction is only possible through the hand-written
//! `*_impl.rs` constructor**, which runs the §Syntaxes grammar. Every generated
//! codec (`emit-json`, `emit-xml`) is routed through the same constructor, so a
//! malformed identifier in any document position refuses at parse rather than
//! entering the model.
//!
//! # Why `pub(crate)` and not fully private
//!
//! The grammar itself is hand-written spec behaviour living in sibling modules
//! of the same crate (`identification/lexical.rs`, `identification/*_impl.rs`) —
//! a fully-private field would be invisible to the very code that validates it,
//! and the generator cannot place hand-written code inside a `// @generated`
//! file. The defining crate is therefore the trust boundary: inside
//! `openehr-base` the identification package owns the grammar; outside it, every
//! consumer (`openehr-rm`, `openehr-its`, `openehr-adl`, the `ferroehr-*`
//! application) can only obtain a value that the grammar already accepted.
//!
//! # Scope discipline
//!
//! This map is declarative and exhaustive over the classes it speaks about: a
//! class named here with [`Door::PlainRecord`] or [`Door::TierEnforced`] is a
//! *recorded decision*, not an omission. Extending the validated set is a
//! spec adjudication per class, never a sweep — see [`Door::TierEnforced`].

/// How a generated spec class is constructed.
pub(crate) enum Door {
    /// Fields are emitted `pub(crate)`; the sibling `*_impl.rs` owns a
    /// validating `new(..) -> Result<Self, _>` (plus `FromStr`/`TryFrom`), and
    /// the generated codecs construct through it.
    Validated {
        /// The hand-written constructor's parameters as `(field, type)` pairs
        /// — the contract between the generated struct and the `*_impl.rs`
        /// sibling, keyed by FIELD NAME so it is independent of any
        /// generation's BMM declaration order (RM 1.1.0 and 1.2.0 declare
        /// `ITEM_TAG`'s fields in different orders; a positional contract
        /// mis-binds one of them). The emitters bind each decoded field to a
        /// local of the declared type — so an `impl Into<String>` parameter
        /// has no inference ambiguity — and call `new` in THIS declared
        /// order, one canonical signature for every generation. A BMM change
        /// that adds or renames a field fails loudly (a name/arity mismatch)
        /// instead of silently emitting a call the constructor cannot
        /// answer.
        ///
        /// A parameter that is itself a spec type is written as the marker
        /// `@<SPEC_CLASS>` (`@UID_BASED_ID`), never a literal Rust path: the
        /// emitter resolves it per GENERATION (the declaring generation's own
        /// module, or the paired dependency generation's), so one table row
        /// serves every generation of the declaring crate.
        params: &'static [(&'static str, &'static str)],
        /// `true` when `new` returns `Result` (the normal case: the constructor
        /// runs a grammar or an invariant over the incoming values). `false`
        /// when the field types already make an invalid value unrepresentable,
        /// so the door exists to be the ONLY door, not to reject anything — a
        /// `Result` there would be a lie the `unnecessary_wraps` lint is right
        /// to flag.
        fallible: bool,
        /// The released grammar or invariant the constructor runs.
        citation: &'static str,
    },
    /// Fields stay `pub`: the released spec states **no** constraint over this
    /// class's field values, so a construction door would validate nothing.
    PlainRecord {
        /// The released text that says so.
        citation: &'static str,
    },
    /// The class HAS a released lexical form, but it is enforced at the
    /// `openehr_base::validate::Validate` tier rather than at construction.
    ///
    /// Moving such a grammar to the construction door converts a *validation*
    /// verdict into a *parse* refusal — an accept/reject boundary change that
    /// needs its own spec adjudication, not a side effect of the privacy
    /// scheme. It is deliberately not taken here.
    TierEnforced {
        /// The released text that both defines the grammar and motivates the
        /// tier split.
        citation: &'static str,
    },
}

/// One class's construction decision.
pub(crate) struct Construction {
    /// The BMM class name.
    pub class: &'static str,
    /// The decision.
    pub door: Door,
}

/// Every adjudicated construction decision.
///
/// The identification family is the lexical-form half: it is the package where
/// the released spec gives a complete machine-checkable grammar for the field
/// value. `ITEM_TAG` is the invariant half — the RM class whose §Invariants are
/// stated purely over its OWN fields, so the same door closes them.
///
/// **The set is adjudicated per class, never swept.** The emit-validate
/// classification knows which invariants are mechanically evaluable, but
/// mechanical evaluability is not sufficient: an invariant may only move to the
/// door when it is decidable from the class's own fields AND refusing at
/// construction is the right accept/reject boundary (see [`Door::TierEnforced`]
/// for a case where it is not). The invariant families still at the `Validate`
/// tier are recorded in `openehr-rm`'s generated realization register.
pub(crate) static CONSTRUCTION: &[Construction] = &[
    // ── the UID hierarchy (uid = iso_oid | uuid | internet_id) ──────────────
    Construction {
        class: "UUID",
        door: Door::Validated {
            params: &[("value", "uuid::Uuid")],
            fallible: false,
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes: `uuid = hex-number, \
                       '-', hex-number, '-', hex-number, '-', hex-number, '-', hex-number`. The \
                       field carries the pinned `uuid` crate's RFC-4122 type (the settled \
                       strong-typing override), so parsing IS validation and the constructor is \
                       total \u{2014} but the door is still the only way in, so the value can \
                       never be replaced by an unparsed one after the fact.",
        },
    },
    Construction {
        class: "ISO_OID",
        door: Door::Validated {
            params: &[("value", "String")],
            fallible: true,
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes: \
                       `iso_oid = number, { '.', number }`.",
        },
    },
    Construction {
        class: "INTERNET_ID",
        door: Door::Validated {
            params: &[("value", "String")],
            fallible: true,
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes: \
                       `internet_id = subdomain`, `subdomain = label | subdomain, '.', label`.",
        },
    },
    // ── the UID_BASED_ID family (root, [ '::', extension ]) ─────────────────
    Construction {
        class: "HIER_OBJECT_ID",
        door: Door::Validated {
            params: &[("value", "String")],
            fallible: true,
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes: \
                       `hier_object_id = uid_based_id`, \
                       `uid_based_id = root, [ '::', extension ]`, `root = uid`.",
        },
    },
    Construction {
        class: "OBJECT_VERSION_ID",
        door: Door::Validated {
            params: &[("value", "String")],
            fallible: true,
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes: \
                       `object_version_id = object_id, '::', creating_system_id, '::', \
                       version_tree_id` with `object_id = uid` and `creating_system_id = uid`.",
        },
    },
    Construction {
        class: "VERSION_TREE_ID",
        door: Door::Validated {
            params: &[("value", "String")],
            fallible: true,
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes: \
                       `version_tree_id = trunk_version, [ '.', branch_number, '.', \
                       branch_version ]`, each part a `number`; RM common \
                       master06-change_control_package.adoc \u{a7}\u{201c}The 'Virtual Version \
                       Tree'\u{201d} starts every part at 1 (`VERSION_TREE_ID.\
                       Trunk_version_valid` / `.Branch_validity`).",
        },
    },
    // ── the invariant half: a class constrained by its OWN §Invariants ──────
    Construction {
        class: "ITEM_TAG",
        door: Door::Validated {
            params: &[
                ("key", "String"),
                ("value", "Option<String>"),
                ("target", "@UID_BASED_ID"),
                ("target_path", "Option<String>"),
                ("owner_id", "@OBJECT_REF"),
            ],
            fallible: true,
            citation: "docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc \
                       \u{a7}Invariants: `Inv_key_valid: not key.is_empty and key.is_justified` \
                       and `Inv_value_valid: value /= Void implies not value.is_empty`. Both are \
                       stated over ITEM_TAG's own fields and decidable from them alone, so an \
                       instance violating either is not an instance of the class \u{2014} the \
                       same standing the \u{a7}Syntaxes grammar has for an identifier. The \
                       remaining checks an ITEM_TAG needs (does `target` exist, does \
                       `target_path` resolve within it) read state the instance does NOT carry \
                       and stay in the service layer.",
        },
    },
    // ── recorded NON-entries: identifier classes that stay plain records ────
    Construction {
        class: "TEMPLATE_ID",
        door: Door::PlainRecord {
            citation: "docs/specs/openehr/BASE/docs/UML/classes/\
                       org.openehr.base.base_types.template_id.adoc: \u{201c}Identifier for \
                       templates. Lexical form to be determined.\u{201d} \u{2014} the release \
                       states no grammar, and \u{a7}Syntaxes has no `template_id` production, so \
                       there is nothing a construction door could check.",
        },
    },
    Construction {
        class: "GENERIC_ID",
        door: Door::PlainRecord {
            citation: "docs/specs/openehr/BASE/docs/UML/classes/\
                       org.openehr.base.base_types.generic_id.adoc: \u{201c}Generic identifier \
                       type for identifiers whose format is otherwise unknown to \
                       openEHR\u{201d}; master05 \u{a7}\u{201c}Generic and External \
                       Identifiers\u{201d}: \u{201c}The names of schemes are not currently \
                       controlled.\u{201d} Both the value and the scheme are unconstrained by \
                       the release.",
        },
    },
    Construction {
        class: "ARCHETYPE_ID",
        door: Door::TierEnforced {
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes gives \
                       `archetype_id = qualified_rm_entity, '.', domain_concept, '.v', \
                       version_id`, but \u{a7}\u{201c}Archetype Identifiers\u{201d} carries a \
                       WARNING that \u{201c}some archetype authoring tools have historically \
                       allowed a nonconforming version part \u{2026} of the form `.v1draft` or \
                       similar\u{201d} \u{2014} i.e. the release itself states that \
                       nonconforming instances are in circulation. Refusing them at PARSE \
                       rather than at validation is an accept/reject boundary change requiring \
                       its own adjudication; the grammar is enforced today by \
                       `impl Validate for ArchetypeId`.",
        },
    },
    Construction {
        class: "TERMINOLOGY_ID",
        door: Door::TierEnforced {
            citation: "docs/specs/openehr/BASE/docs/base_types/\
                       master05-identification_package.adoc \u{a7}Syntaxes gives \
                       `terminology_id = name-str, [ '(', name-str, ')' ]` with \
                       `name-str = letter, { letter | digit | '_' | '-' | '/' | '+' }`, which \
                       the same chapter's own examples contradict \
                       (\u{a7}\u{201c}Terminology Identifiers\u{201d}: `\"ICD10AM(3rd_ed)\"` \
                       \u{2014} the version part starts with a digit, which `name-str` forbids). \
                       A parse-time refusal would reject the release's own examples, so the \
                       grammar stays at the `Validate` tier pending adjudication.",
        },
    },
];

/// The construction decision recorded for `class`, or `None` (the default:
/// a plain record with public fields).
pub(crate) fn door(class: &str) -> Option<&'static Door> {
    CONSTRUCTION
        .iter()
        .find(|c| c.class == class)
        .map(|c| &c.door)
}

/// Does `class` construct through a validating door (fields `pub(crate)`,
/// codecs routed through `new`)?
#[must_use]
pub(crate) fn is_validated(class: &str) -> bool {
    matches!(door(class), Some(Door::Validated { .. }))
}

/// The declared `(parameter types, fallible)` of a validated `class`'s
/// constructor, or `None` when the class has no validating door.
#[must_use]
pub(crate) fn validated_ctor(
    class: &str,
) -> Option<(&'static [(&'static str, &'static str)], bool)> {
    match door(class) {
        Some(Door::Validated {
            params, fallible, ..
        }) => Some((*params, *fallible)),
        _ => None,
    }
}

/// The spec citation the emitter writes above a validated class's fields, so
/// the reason the fields are not `pub` is readable where they are.
#[must_use]
pub(crate) fn validated_citation(class: &str) -> Option<&'static str> {
    match door(class) {
        Some(Door::Validated { citation, .. }) => Some(citation),
        _ => None,
    }
}

/// The adjudication the emitter writes into a class that stays a PLAIN RECORD
/// although a reader might expect a construction door — so the decision is
/// readable at the public field rather than only in this map.
///
/// `None` for a validated class (its citation is [`validated_citation`]) and for
/// a class the map says nothing about (the unremarkable default).
#[must_use]
pub(crate) fn plain_record_note(class: &str) -> Option<String> {
    match door(class)? {
        Door::Validated { .. } => None,
        Door::PlainRecord { citation } => Some(format!(
            "the fields stay public deliberately: the release states no constraint over \
             this class's field values, so a validating construction door would check \
             nothing \u{2014} {citation}"
        )),
        Door::TierEnforced { citation } => Some(format!(
            "the fields stay public deliberately: this class HAS a released lexical form, \
             but it is enforced at the `openehr_base::validate::Validate` tier rather than \
             at construction \u{2014} moving it to the door would turn a validation verdict \
             into a parse refusal, which needs its own adjudication \u{2014} {citation}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{CONSTRUCTION, Door, door, is_validated, validated_ctor};

    /// Every entry names a distinct class (a duplicate would make `door`'s
    /// first-match silently authoritative).
    #[test]
    fn entries_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for c in CONSTRUCTION {
            assert!(seen.insert(c.class), "duplicate entry for {}", c.class);
        }
    }

    /// Every citation names a vendored spec path — the map is the decision
    /// record, and a decision without its released source is not one.
    #[test]
    fn every_decision_cites_the_release() {
        for c in CONSTRUCTION {
            let citation = match &c.door {
                Door::Validated { citation, .. }
                | Door::PlainRecord { citation }
                | Door::TierEnforced { citation } => *citation,
            };
            assert!(
                citation.contains("docs/specs/openehr/"),
                "{} cites no vendored spec text",
                c.class
            );
        }
    }

    /// The identification family's validated set is exactly the classes whose
    /// §Syntaxes production is total and uncontradicted.
    #[test]
    fn validated_set_is_the_scoped_family() {
        for class in [
            "UUID",
            "ISO_OID",
            "INTERNET_ID",
            "HIER_OBJECT_ID",
            "OBJECT_VERSION_ID",
            "VERSION_TREE_ID",
        ] {
            assert!(is_validated(class), "{class} must construct through a door");
            let (params, _) = validated_ctor(class).expect("a validated class has a constructor");
            assert_eq!(params.len(), 1, "{class}");
        }
        for class in [
            "TEMPLATE_ID",
            "GENERIC_ID",
            "ARCHETYPE_ID",
            "TERMINOLOGY_ID",
        ] {
            assert!(!is_validated(class), "{class} stays a plain record");
            assert!(door(class).is_some(), "{class} must be a RECORDED decision");
        }
        // A class the map says nothing about defaults to a plain record.
        assert!(door("DV_TEXT").is_none());
        assert!(!is_validated("DV_TEXT"));
    }
}
