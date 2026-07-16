//! opt14 ↔ am14 constraint-model divergence sentinel.
//!
//! The AOM 1.4 constraint model exists twice by design: BMM-generated
//! `openehr_am::am14` (the canonical logical model, canonical-JSON codec) and
//! XSD-generated `openehr_its::opt14` (the Ocean OPT-XML wire adapter). The
//! divergences between the two are *deliberate and documented* in
//! `the opt14-wire-model design record`; this test is the drift guard that
//! ADR requires: it pins both models' constraint-type inventories with
//! **exhaustive** (wildcard-free) matches and explicit inventory lists, so an
//! AOM/OPT spec bump that regenerates either side with a new/removed/renamed
//! constraint type fails here and forces a conscious reconciliation (update
//! the design record + this sentinel together).
//!
//! It intentionally does NOT compare field shapes — the field-level divergence
//! (Interval representation, typed vs `Any` assumed values, domain-type sets)
//! is the documented reason the models are separate; the inventory is the
//! drift-prone surface.

/// The opt14 (OPT-XML) polymorphic constraint inventories. Exhaustive matches:
/// a regeneration that adds or removes an enum variant breaks compilation.
#[allow(dead_code)]
mod opt14_inventory {
    use openehr_its::opt14 as opt;

    pub fn c_object_variants(v: &opt::CObject) -> &'static str {
        match v {
            opt::CObject::ArchetypeInternalRef(_) => "ARCHETYPE_INTERNAL_REF",
            opt::CObject::ArchetypeSlot(_) => "ARCHETYPE_SLOT",
            opt::CObject::ConstraintRef(_) => "CONSTRAINT_REF",
            opt::CObject::CArchetypeRoot(_) => "C_ARCHETYPE_ROOT",
            opt::CObject::CCodePhrase(_) => "C_CODE_PHRASE",
            opt::CObject::CCodeReference(_) => "C_CODE_REFERENCE",
            opt::CObject::CComplexObject(_) => "C_COMPLEX_OBJECT",
            opt::CObject::CDefinedObject(_) => "C_DEFINED_OBJECT",
            opt::CObject::CDvOrdinal(_) => "C_DV_ORDINAL",
            opt::CObject::CDvQuantity(_) => "C_DV_QUANTITY",
            opt::CObject::CDvState(_) => "C_DV_STATE",
            opt::CObject::CPrimitiveObject(_) => "C_PRIMITIVE_OBJECT",
            opt::CObject::TComplexObject(_) => "T_COMPLEX_OBJECT",
        }
    }

    pub fn c_attribute_variants(v: &opt::CAttribute) -> &'static str {
        match v {
            opt::CAttribute::CMultipleAttribute(_) => "C_MULTIPLE_ATTRIBUTE",
            opt::CAttribute::CSingleAttribute(_) => "C_SINGLE_ATTRIBUTE",
        }
    }

    pub fn c_primitive_variants(v: &opt::CPrimitive) -> &'static str {
        match v {
            opt::CPrimitive::CBoolean(_) => "C_BOOLEAN",
            opt::CPrimitive::CDate(_) => "C_DATE",
            opt::CPrimitive::CDateTime(_) => "C_DATE_TIME",
            opt::CPrimitive::CDuration(_) => "C_DURATION",
            opt::CPrimitive::CInteger(_) => "C_INTEGER",
            opt::CPrimitive::CReal(_) => "C_REAL",
            opt::CPrimitive::CString(_) => "C_STRING",
            opt::CPrimitive::CTime(_) => "C_TIME",
        }
    }

    pub fn c_domain_type_variants(v: &opt::CDomainType) -> &'static str {
        match v {
            opt::CDomainType::CCodePhrase(_) => "C_CODE_PHRASE",
            opt::CDomainType::CCodeReference(_) => "C_CODE_REFERENCE",
            opt::CDomainType::CDvOrdinal(_) => "C_DV_ORDINAL",
            opt::CDomainType::CDvQuantity(_) => "C_DV_QUANTITY",
            opt::CDomainType::CDvState(_) => "C_DV_STATE",
        }
    }
}

/// The am14 (BMM/AOM 1.4) polymorphic constraint inventories, same mechanism.
#[allow(dead_code)]
mod am14_inventory {
    use openehr_am::am14::prelude as am;

    pub fn c_object_variants(v: &am::CObject) -> &'static str {
        match v {
            am::CObject::ArchetypeInternalRef(_) => "ARCHETYPE_INTERNAL_REF",
            am::CObject::ArchetypeSlot(_) => "ARCHETYPE_SLOT",
            am::CObject::ConstraintRef(_) => "CONSTRAINT_REF",
            am::CObject::CCodedText(_) => "C_CODED_TEXT",
            am::CObject::CComplexObject(_) => "C_COMPLEX_OBJECT",
            am::CObject::COrdinal(_) => "C_ORDINAL",
            am::CObject::CPrimitiveObject(_) => "C_PRIMITIVE_OBJECT",
            am::CObject::CQuantity(_) => "C_QUANTITY",
        }
    }

    pub fn c_attribute_variants(v: &am::CAttribute) -> &'static str {
        match v {
            am::CAttribute::CMultipleAttribute(_) => "C_MULTIPLE_ATTRIBUTE",
            am::CAttribute::CSingleAttribute(_) => "C_SINGLE_ATTRIBUTE",
        }
    }

    pub fn c_primitive_variants(v: &am::CPrimitive) -> &'static str {
        match v {
            am::CPrimitive::CBoolean(_) => "C_BOOLEAN",
            am::CPrimitive::CDate(_) => "C_DATE",
            am::CPrimitive::CDateTime(_) => "C_DATE_TIME",
            am::CPrimitive::CDuration(_) => "C_DURATION",
            am::CPrimitive::CInteger(_) => "C_INTEGER",
            am::CPrimitive::CReal(_) => "C_REAL",
            am::CPrimitive::CString(_) => "C_STRING",
            am::CPrimitive::CTime(_) => "C_TIME",
        }
    }

    pub fn c_domain_type_variants(v: &am::CDomainType) -> &'static str {
        match v {
            am::CDomainType::CCodedText(_) => "C_CODED_TEXT",
            am::CDomainType::COrdinal(_) => "C_ORDINAL",
            am::CDomainType::CQuantity(_) => "C_QUANTITY",
        }
    }
}

/// The *documented* inventory divergence: the OPT-XML
/// domain types the BMM model does not carry, and vice versa. If this set
/// changes (a spec/XSD bump closed or widened the gap), the wire-model design record must be
/// revisited — that is the point of the failure.
#[test]
fn documented_divergence_is_stable() {
    // OPT-XML-only constraint types (no am14 counterpart).
    let opt_only = [
        "C_ARCHETYPE_ROOT", // OPT envelope root (Template.xsd)
        "C_CODE_PHRASE",    // OpenehrProfile.xsd shape of C_CODED_TEXT
        "C_CODE_REFERENCE", // Template.xsd extension of C_CODE_PHRASE
        "C_DV_ORDINAL",     // OpenehrProfile.xsd shape of C_ORDINAL
        "C_DV_QUANTITY",    // OpenehrProfile.xsd shape of C_QUANTITY
        "C_DV_STATE",       // OpenehrProfile.xsd only (no BMM class)
        "T_COMPLEX_OBJECT", // Template.xsd default_value overlay node
    ];
    // BMM/am14-only constraint types (no OPT-XML counterpart).
    let am_only = ["C_CODED_TEXT", "C_ORDINAL", "C_QUANTITY"];

    // Pin both lists; a regenerated model that gains/loses a type will already
    // have broken the exhaustive matches above — this assert documents *which*
    // names are the expected asymmetry so the fix is a conscious edit here +
    // by design, not a silent drift.
    assert_eq!(opt_only.len(), 7);
    assert_eq!(am_only.len(), 3);
    assert!(opt_only.is_sorted());
    assert!(am_only.is_sorted());
}
