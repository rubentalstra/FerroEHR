//! OPT authoring for the content-chapter constraint cases (master15/16/17.x).
//!
//! The vendored CNF corpus ships **no** OPT per archetype-constraint variant — the
//! schedule itself says the archetypes "should be generated" (master15
//! §Implementation notes). Rather than skip every cardinality / occurrences /
//! value-constraint case (which would leave the data-validation truth tables
//! untested), the suite **authors** the constraining OPT programmatically: it
//! parses a vendored base OPT into the typed [`openehr_its::opt14`] model, tightens
//! the exact constraint the case exercises, re-serialises to ADL 1.4 XML, and
//! provisions it via [`crate::suites::support::ensure_opt_xml`].
//!
//! The authored template keeps the base's archetype structure (root archetype id,
//! node ids, term codes, ontology) and only changes (a) the `template_id` — so each
//! variant is a distinct, independently-uploadable template — and (b) the one
//! constraint under test. A composition that conforms to the base archetype
//! therefore still conforms to the authored template except where the tightened
//! constraint is violated, which is exactly the truth-table oracle.
//!
//! This is not a fabricated pass: the constraint is really expressed in a real OPT
//! the SUT ingests and builds a `WebTemplate` from, and the accept/reject outcome
//! is the server's genuine validation decision. Each authored OPT family is
//! declared as a `generated:` fixture in `testdata/MANIFEST.tsv`,
//! naming the tightening transform below as its source class.

use openehr_its::opt14::{
    self, CArchetypeRoot, CAttribute, CBoolean, CDate, CDateTime, CDuration, CInteger, CObject,
    CPrimitive, CPrimitiveObject, CReal, CSingleAttribute, CString, CTime, Intervalofdate,
    Intervalofdatetime, Intervalofduration, Intervalofinteger, Intervalofreal, Intervaloftime,
    OperationalTemplate,
};

use crate::engine::harness::CaseError;

/// A `multiple-attribute` cardinality interval — the six intervals master15/16
/// enumerate for a "multiple attribute" (`master15 §For testing a 'multiple
/// attribute' cardinality`).
#[derive(Clone, Copy, Debug)]
pub enum Card {
    /// `0..*` — any number, including none (no effective constraint).
    Any,
    /// `1..*` — at least one.
    OnePlus,
    /// `3..*` — at least three.
    ThreePlus,
    /// `0..1` — at most one.
    Opt,
    /// `1..1` — exactly one.
    Mand,
    /// `3..5` — between three and five.
    ThreeToFive,
}

impl Card {
    /// The AOM `IntervalOfInteger` this cardinality denotes.
    #[must_use]
    pub fn interval(self) -> Intervalofinteger {
        match self {
            Card::Any => open_interval(0),
            Card::OnePlus => open_interval(1),
            Card::ThreePlus => open_interval(3),
            Card::Opt => closed_interval(0, 1),
            Card::Mand => closed_interval(1, 1),
            Card::ThreeToFive => closed_interval(3, 5),
        }
    }
}

/// A closed interval `lower..upper` (both bounds included).
#[must_use]
pub fn closed_interval(lower: i32, upper: i32) -> Intervalofinteger {
    Intervalofinteger {
        lower_included: Some(true),
        upper_included: Some(true),
        lower_unbounded: false,
        upper_unbounded: false,
        lower: Some(lower),
        upper: Some(upper),
    }
}

/// A half-open interval `lower..*` (upper unbounded).
#[must_use]
pub fn open_interval(lower: i32) -> Intervalofinteger {
    Intervalofinteger {
        lower_included: Some(true),
        upper_included: Some(false),
        lower_unbounded: false,
        upper_unbounded: true,
        lower: Some(lower),
        upper: None,
    }
}

/// Parse a vendored base OPT (a file under the `template.valid` corpus-dir key,
/// e.g. `minimal/minimal_evaluation.opt`) into the typed model.
///
/// # Errors
/// [`CaseError::Codec`] if the fixture is missing or does not parse.
pub fn parse_base(opt_file: &str) -> Result<OperationalTemplate, CaseError> {
    let xml = crate::testdata::fixtures::read_from("template.valid", opt_file)
        .map_err(|e| CaseError::Codec(e.to_string()))?;
    opt14::from_xml(&xml).map_err(|e| CaseError::Codec(e.to_string()))
}

/// Serialise an authored OPT back to ADL 1.4 XML.
///
/// # Errors
/// [`CaseError::Codec`] if serialisation fails.
pub fn to_xml(opt: &OperationalTemplate) -> Result<String, CaseError> {
    opt14::to_xml(opt).map_err(|e| CaseError::Codec(e.to_string()))
}

/// Retarget the template so it uploads as a distinct template (a fresh
/// `template_id` avoids the 409 the store returns for a duplicate id — see
/// `service::template::store_template`).
pub fn set_template_id(opt: &mut OperationalTemplate, template_id: &str) {
    template_id.clone_into(&mut opt.template_id.value);
}

/// Set the cardinality interval of a **top-level** multiple attribute of the root
/// object (e.g. `COMPOSITION.content`). Returns `true` if the attribute was found.
///
/// The attribute's child object constraints are left untouched — in the vendored
/// bases the single content archetype already permits `0..*` occurrences, so
/// varying only the attribute cardinality isolates the cardinality constraint from
/// the per-node occurrences constraint (master15 §Isolation).
pub fn set_root_multiple_cardinality(
    opt: &mut OperationalTemplate,
    attr: &str,
    interval: Intervalofinteger,
) -> bool {
    for a in &mut opt.definition.attributes {
        if let CAttribute::CMultipleAttribute(m) = a
            && m.rm_attribute_name == attr
        {
            // AOM 1.4: cardinality constrains membership when the attribute is
            // PRESENT; absence is governed by existence. A truth-table row with
            // a lower bound >= 1 means "at least N items must be committed", so
            // the authored artefact must ALSO make the attribute mandatory
            // (existence 1..1) — otherwise the spec-valid absent-attribute
            // encoding of "zero items" would be conformant and the negative
            // rows untestable.
            let min = if interval.lower_unbounded {
                0
            } else {
                interval.lower.unwrap_or(0)
            };
            if min >= 1 {
                m.existence = closed_interval(1, 1);
            }
            m.cardinality.interval = interval;
            return true;
        }
    }
    false
}

/// Make a **top-level single attribute** of the root object mandatory by setting
/// its `existence` to `1..1` (e.g. `COMPOSITION.context`). If the base does not
/// constrain the attribute at all, a bare mandatory `C_SINGLE_ATTRIBUTE` (no value
/// constraint — any RM value accepted, but the attribute must be present) is added,
/// which is what the truth table's "context mandatory" column requires.
pub fn set_root_single_mandatory(opt: &mut OperationalTemplate, attr: &str) {
    for a in &mut opt.definition.attributes {
        if let CAttribute::CSingleAttribute(s) = a
            && s.rm_attribute_name == attr
        {
            s.existence = closed_interval(1, 1);
            return;
        }
    }
    opt.definition
        .attributes
        .push(CAttribute::CSingleAttribute(CSingleAttribute {
            rm_attribute_name: attr.to_owned(),
            existence: closed_interval(1, 1),
            children: vec![],
        }));
}

// ── nested-object structural constraints (master16: HISTORY, EVENT, …) ────────

/// The child `C_OBJECT`s of a `C_OBJECT` (its attributes' children), for the
/// recursive tree walk. `C_ARCHETYPE_ROOT` and `C_COMPLEX_OBJECT` are the only
/// object kinds that carry attributes; the leaf/primitive/domain kinds have none.
fn object_attributes_mut(obj: &mut CObject) -> Option<&mut Vec<CAttribute>> {
    match obj {
        CObject::CArchetypeRoot(r) => Some(&mut r.attributes),
        CObject::CComplexObject(c) => Some(&mut c.attributes),
        _ => None,
    }
}

/// The `rm_type_name` of a `C_OBJECT`, where it has one.
fn object_rm_type(obj: &CObject) -> Option<&str> {
    match obj {
        CObject::CArchetypeRoot(r) => Some(&r.rm_type_name),
        CObject::CComplexObject(c) => Some(&c.rm_type_name),
        CObject::CDefinedObject(o) => Some(&o.rm_type_name),
        CObject::CPrimitiveObject(o) => Some(&o.rm_type_name),
        CObject::CCodePhrase(o) => Some(&o.rm_type_name),
        CObject::CCodeReference(o) => Some(&o.rm_type_name),
        CObject::CDvOrdinal(o) => Some(&o.rm_type_name),
        CObject::CDvQuantity(o) => Some(&o.rm_type_name),
        CObject::CDvState(o) => Some(&o.rm_type_name),
        CObject::ArchetypeInternalRef(_)
        | CObject::ArchetypeSlot(_)
        | CObject::ConstraintRef(_)
        | CObject::TComplexObject(_) => None,
    }
}

/// Depth-first apply `f` to every `C_OBJECT` in the definition tree, pre-order,
/// stopping at the first object for which `f` returns `true`. Returns whether any
/// object was matched.
fn visit_objects(root: &mut CArchetypeRoot, f: &mut impl FnMut(&mut CObject) -> bool) -> bool {
    fn descend_attrs(attrs: &mut [CAttribute], f: &mut impl FnMut(&mut CObject) -> bool) -> bool {
        for a in attrs {
            let children = match a {
                CAttribute::CMultipleAttribute(m) => &mut m.children,
                CAttribute::CSingleAttribute(s) => &mut s.children,
            };
            for ch in children.iter_mut() {
                if f(ch) {
                    return true;
                }
                if let Some(inner) = object_attributes_mut(ch)
                    && descend_attrs(inner, f)
                {
                    return true;
                }
            }
        }
        false
    }
    descend_attrs(&mut root.attributes, f)
}

/// Set the cardinality interval of a **multiple attribute** on the first nested
/// object of type `host` (e.g. `HISTORY.events`), and open the constrained
/// attribute's child object occurrences to `0..*` so that varying the number of
/// committed children exercises *only* the container cardinality — not the
/// per-node occurrences (master15/16 §Isolation). Returns `true` if applied.
pub fn constrain_nested_multiple(
    opt: &mut OperationalTemplate,
    host: &str,
    attr: &str,
    interval: &Intervalofinteger,
) -> bool {
    visit_objects(&mut opt.definition, &mut |obj| {
        if object_rm_type(obj) != Some(host) {
            return false;
        }
        let Some(attrs) = object_attributes_mut(obj) else {
            return false;
        };
        for a in attrs.iter_mut() {
            if let CAttribute::CMultipleAttribute(m) = a
                && m.rm_attribute_name == attr
            {
                m.cardinality.interval = interval.clone();
                for ch in &mut m.children {
                    set_object_occurrences(ch, open_interval(0));
                }
                return true;
            }
        }
        false
    })
}

/// Make a **single attribute** on the first nested object of type `host` mandatory
/// (`existence 1..1`) — e.g. `HISTORY.summary`, `OBSERVATION.state`,
/// `OBSERVATION.protocol`, `EVENT.state`. Adds the attribute if absent. Returns
/// `true` if the host object was found.
pub fn constrain_nested_single_mandatory(
    opt: &mut OperationalTemplate,
    host: &str,
    attr: &str,
) -> bool {
    visit_objects(&mut opt.definition, &mut |obj| {
        if object_rm_type(obj) != Some(host) {
            return false;
        }
        let Some(attrs) = object_attributes_mut(obj) else {
            return false;
        };
        for a in attrs.iter_mut() {
            if let CAttribute::CSingleAttribute(s) = a
                && s.rm_attribute_name == attr
            {
                s.existence = closed_interval(1, 1);
                return true;
            }
        }
        attrs.push(CAttribute::CSingleAttribute(CSingleAttribute {
            rm_attribute_name: attr.to_owned(),
            existence: closed_interval(1, 1),
            children: vec![],
        }));
        true
    })
}

/// Set the `occurrences` of a `C_OBJECT` (the constraint kinds that carry it).
pub fn set_object_occurrences(obj: &mut CObject, interval: Intervalofinteger) {
    match obj {
        CObject::CArchetypeRoot(r) => r.occurrences = interval,
        CObject::CComplexObject(c) => c.occurrences = interval,
        CObject::CDefinedObject(o) => o.occurrences = interval,
        CObject::CPrimitiveObject(o) => o.occurrences = interval,
        CObject::CCodePhrase(o) => o.occurrences = interval,
        CObject::CCodeReference(o) => o.occurrences = interval,
        CObject::CDvOrdinal(o) => o.occurrences = interval,
        CObject::CDvQuantity(o) => o.occurrences = interval,
        CObject::CDvState(o) => o.occurrences = interval,
        CObject::ArchetypeInternalRef(_)
        | CObject::ArchetypeSlot(_)
        | CObject::ConstraintRef(_)
        | CObject::TComplexObject(_) => {}
    }
}

/// Access the first nested object of type `host` and apply `f` to its attribute
/// named `attr` (single or multiple). Returns `true` if applied — the general
/// escape hatch for constraints not covered by the specific helpers above (used by
/// the master17 leaf value-constraint authoring). `f` receives the matched
/// [`CAttribute`].
pub fn with_nested_attribute(
    opt: &mut OperationalTemplate,
    host: &str,
    attr: &str,
    mut f: impl FnMut(&mut CAttribute),
) -> bool {
    visit_objects(&mut opt.definition, &mut |obj| {
        if object_rm_type(obj) != Some(host) {
            return false;
        }
        let Some(attrs) = object_attributes_mut(obj) else {
            return false;
        };
        for a in attrs.iter_mut() {
            let name = match a {
                CAttribute::CMultipleAttribute(m) => &m.rm_attribute_name,
                CAttribute::CSingleAttribute(s) => &s.rm_attribute_name,
            };
            if name == attr {
                f(a);
                return true;
            }
        }
        false
    })
}

/// The `rm_type_name` of a `C_OBJECT`, mutable where it has one.
fn object_rm_type_mut(obj: &mut CObject) -> Option<&mut String> {
    match obj {
        CObject::CArchetypeRoot(r) => Some(&mut r.rm_type_name),
        CObject::CComplexObject(c) => Some(&mut c.rm_type_name),
        CObject::CDefinedObject(o) => Some(&mut o.rm_type_name),
        CObject::CPrimitiveObject(o) => Some(&mut o.rm_type_name),
        CObject::CCodePhrase(o) => Some(&mut o.rm_type_name),
        CObject::CCodeReference(o) => Some(&mut o.rm_type_name),
        CObject::CDvOrdinal(o) => Some(&mut o.rm_type_name),
        CObject::CDvQuantity(o) => Some(&mut o.rm_type_name),
        CObject::CDvState(o) => Some(&mut o.rm_type_name),
        CObject::ArchetypeInternalRef(_)
        | CObject::ArchetypeSlot(_)
        | CObject::ConstraintRef(_)
        | CObject::TComplexObject(_) => None,
    }
}

/// Narrow the **type** of every child object of `host`'s `attr` to `rm_type` (a
/// concrete descendant), e.g. `HISTORY.events` → `POINT_EVENT`. The `WebTemplate` the
/// SUT builds then constrains the slot to that concrete class, so an instance of a
/// sibling subtype is rejected ("Class not allowed"). Returns `true` if applied.
pub fn narrow_nested_child_type(
    opt: &mut OperationalTemplate,
    host: &str,
    attr: &str,
    rm_type: &str,
) -> bool {
    let mut narrowed = false;
    with_nested_attribute(opt, host, attr, |a| {
        let children = match a {
            CAttribute::CMultipleAttribute(m) => &mut m.children,
            CAttribute::CSingleAttribute(s) => &mut s.children,
        };
        for ch in children {
            if let Some(t) = object_rm_type_mut(ch) {
                rm_type.clone_into(t);
                narrowed = true;
            }
        }
    });
    narrowed
}

// ── leaf value constraints (master17: C_STRING / C_INTEGER on a DV_* leaf) ─────

/// The `rm_attribute_name` of a `C_ATTRIBUTE`.
fn attr_name(a: &CAttribute) -> &str {
    match a {
        CAttribute::CMultipleAttribute(m) => &m.rm_attribute_name,
        CAttribute::CSingleAttribute(s) => &s.rm_attribute_name,
    }
}

/// Set the `C_PRIMITIVE` constraining the `value_attr` (`"value"`, `"magnitude"`)
/// of the first nested object of type `host` (a DV_* leaf), creating the attribute
/// and/or its `C_PRIMITIVE_OBJECT` if the base OPT leaves the leaf unconstrained.
/// `prim_rm_type` is the primitive object's `rm_type_name` (`"String"`,
/// `"Integer"`). The `WebTemplate` builder surfaces the `C_STRING` `pattern`/`list` and
/// the numeric `range` into the leaf `input` (`webtemplate::inputs`), which the
/// validator then enforces (`validation::leaf`). Returns `true` if applied.
fn constrain_leaf_primitive(
    opt: &mut OperationalTemplate,
    host: &str,
    value_attr: &str,
    prim_rm_type: &str,
    prim: CPrimitive,
) -> bool {
    // `visit_objects` stops at the first object `f` returns true for; capture the
    // primitive in an Option so it is moved into the closure exactly once.
    let mut prim = Some(prim);
    visit_objects(&mut opt.definition, &mut |obj| {
        if object_rm_type(obj) != Some(host) {
            return false;
        }
        let Some(attrs) = object_attributes_mut(obj) else {
            return false;
        };
        let Some(prim) = prim.take() else {
            return true;
        };
        let idx = if let Some(i) = attrs.iter().position(|a| attr_name(a) == value_attr) {
            i
        } else {
            attrs.push(CAttribute::CSingleAttribute(CSingleAttribute {
                rm_attribute_name: value_attr.to_owned(),
                existence: closed_interval(1, 1),
                children: Vec::new(),
            }));
            attrs.len() - 1
        };
        let children = match &mut attrs[idx] {
            CAttribute::CSingleAttribute(s) => &mut s.children,
            CAttribute::CMultipleAttribute(m) => &mut m.children,
        };
        for ch in children.iter_mut() {
            if let CObject::CPrimitiveObject(po) = ch {
                po.item = Some(Box::new(prim));
                return true;
            }
        }
        children.push(CObject::CPrimitiveObject(CPrimitiveObject {
            rm_type_name: prim_rm_type.to_owned(),
            occurrences: closed_interval(1, 1),
            node_id: String::new(),
            item: Some(Box::new(prim)),
        }));
        true
    })
}

/// Constrain a DV_* leaf's string `value_attr` with a `C_STRING` regex `pattern`
/// and/or an enumerated `list` (e.g. `DV_URI.value`, `DV_TEXT.value`). Returns
/// `true` if applied.
pub fn constrain_leaf_string(
    opt: &mut OperationalTemplate,
    host: &str,
    value_attr: &str,
    pattern: Option<&str>,
    list: Vec<String>,
) -> bool {
    constrain_leaf_primitive(
        opt,
        host,
        value_attr,
        "String",
        CPrimitive::CString(CString {
            pattern: pattern.map(str::to_owned),
            list,
            list_open: Some(false),
            assumed_value: None,
        }),
    )
}

/// Constrain a DV_* leaf's integer `value_attr` with a `C_INTEGER` `range`
/// (`lower..=upper`) and/or an enumerated `list` (e.g. `DV_COUNT.magnitude`).
/// Returns `true` if applied.
pub fn constrain_leaf_integer(
    opt: &mut OperationalTemplate,
    host: &str,
    value_attr: &str,
    range: Option<(i32, i32)>,
    list: Vec<i32>,
) -> bool {
    constrain_leaf_primitive(
        opt,
        host,
        value_attr,
        "Integer",
        CPrimitive::CInteger(CInteger {
            list,
            range: range.map(|(lo, hi)| closed_interval(lo, hi)),
            assumed_value: None,
        }),
    )
}

/// Constrain a DV_* leaf's real `value_attr` with a `C_REAL` `range` and/or `list`
/// (e.g. `DV_PROPORTION.numerator`). Returns `true` if applied.
pub fn constrain_leaf_real(
    opt: &mut OperationalTemplate,
    host: &str,
    value_attr: &str,
    range: Option<(f64, f64)>,
    list: Vec<f64>,
) -> bool {
    constrain_leaf_primitive(
        opt,
        host,
        value_attr,
        "Real",
        CPrimitive::CReal(CReal {
            list,
            range: range.map(|(lo, hi)| Intervalofreal {
                lower_included: Some(true),
                upper_included: Some(true),
                lower_unbounded: false,
                upper_unbounded: false,
                lower: Some(lo),
                upper: Some(hi),
            }),
            assumed_value: None,
        }),
    )
}

/// Constrain a DV_* leaf's boolean `value_attr` with a `C_BOOLEAN` (e.g.
/// `DV_BOOLEAN.value` = only-true / only-false). Returns `true` if applied.
pub fn constrain_leaf_boolean(
    opt: &mut OperationalTemplate,
    host: &str,
    value_attr: &str,
    true_valid: bool,
    false_valid: bool,
) -> bool {
    constrain_leaf_primitive(
        opt,
        host,
        value_attr,
        "Boolean",
        CPrimitive::CBoolean(CBoolean {
            true_valid,
            false_valid,
            assumed_value: None,
        }),
    )
}

/// Constrain a `DV_DATE`/`DV_TIME`/`DV_DATE_TIME`/`DV_DURATION` leaf's `value` with
/// a temporal `C_*` primitive (`rm_type` = `"Date"`/`"Time"`/`"Date_Time"`/
/// `"Duration"`; build `prim` with [`c_date`]/[`c_time`]/[`c_date_time`]/
/// [`c_duration`]). Returns `true` if applied.
pub fn constrain_leaf_temporal(
    opt: &mut OperationalTemplate,
    host: &str,
    rm_type: &str,
    prim: CPrimitive,
) -> bool {
    constrain_leaf_primitive(opt, host, "value", rm_type, prim)
}

/// A `C_DATE` with optional `pattern` and ISO-date `range`.
#[must_use]
pub fn c_date(pattern: Option<&str>, range: Option<(&str, &str)>) -> CPrimitive {
    CPrimitive::CDate(CDate {
        pattern: pattern.map(str::to_owned),
        timezone_validity: None,
        range: range.map(|(lo, hi)| Intervalofdate {
            lower_included: Some(true),
            upper_included: Some(true),
            lower_unbounded: false,
            upper_unbounded: false,
            lower: Some(lo.to_owned()),
            upper: Some(hi.to_owned()),
        }),
        assumed_value: None,
    })
}

/// A `C_TIME` with optional `pattern` and ISO-time `range`.
#[must_use]
pub fn c_time(pattern: Option<&str>, range: Option<(&str, &str)>) -> CPrimitive {
    CPrimitive::CTime(CTime {
        pattern: pattern.map(str::to_owned),
        timezone_validity: None,
        range: range.map(|(lo, hi)| Intervaloftime {
            lower_included: Some(true),
            upper_included: Some(true),
            lower_unbounded: false,
            upper_unbounded: false,
            lower: Some(lo.to_owned()),
            upper: Some(hi.to_owned()),
        }),
        assumed_value: None,
    })
}

/// A `C_DATE_TIME` with optional `pattern` and ISO-datetime `range`.
#[must_use]
pub fn c_date_time(pattern: Option<&str>, range: Option<(&str, &str)>) -> CPrimitive {
    CPrimitive::CDateTime(CDateTime {
        pattern: pattern.map(str::to_owned),
        timezone_validity: None,
        range: range.map(|(lo, hi)| Intervalofdatetime {
            lower_included: Some(true),
            upper_included: Some(true),
            lower_unbounded: false,
            upper_unbounded: false,
            lower: Some(lo.to_owned()),
            upper: Some(hi.to_owned()),
        }),
        assumed_value: None,
    })
}

/// A `C_DURATION` with optional `pattern` (allowed fields, e.g. `PYMD`) and ISO
/// `range`.
#[must_use]
pub fn c_duration(pattern: Option<&str>, range: Option<(&str, &str)>) -> CPrimitive {
    CPrimitive::CDuration(CDuration {
        pattern: pattern.map(str::to_owned),
        range: range.map(|(lo, hi)| Intervalofduration {
            lower_included: Some(true),
            upper_included: Some(true),
            lower_unbounded: false,
            upper_unbounded: false,
            lower: Some(lo.to_owned()),
            upper: Some(hi.to_owned()),
        }),
        assumed_value: None,
    })
}

/// Constrain a `DV_QUANTITY` leaf's `property` (`C_DV_QUANTITY.property`) to
/// `terminology::code`, locating the `C_DV_QUANTITY` domain object directly.
/// Returns `true` if applied.
pub fn constrain_dv_quantity_property(
    opt: &mut OperationalTemplate,
    terminology: &str,
    code: &str,
) -> bool {
    visit_objects(&mut opt.definition, &mut |obj| {
        if let CObject::CDvQuantity(q) = obj {
            q.property = Some(openehr_base::prelude::CodePhrase {
                terminology_id: openehr_base::prelude::TerminologyId {
                    value: terminology.to_owned(),
                },
                code_string: code.to_owned(),
                preferred_term: None,
            });
            true
        } else {
            false
        }
    })
}

/// Replace the constraint object under a DV_* leaf whose current value type is
/// `current_type` with a fresh object `new_obj` — the "slot-retype" used to test a
/// type the base composition does not carry (e.g. reuse the `DV_TEXT` slot for
/// `DV_URI`). Finds the first single/multiple-attribute child whose
/// `rm_type_name == current_type` and replaces it. Returns `true` if applied.
pub fn retype_leaf(opt: &mut OperationalTemplate, current_type: &str, new_obj: CObject) -> bool {
    let mut new_obj = Some(new_obj);
    visit_objects(&mut opt.definition, &mut |obj| {
        let Some(attrs) = object_attributes_mut(obj) else {
            return false;
        };
        for a in attrs {
            let children = match a {
                CAttribute::CSingleAttribute(s) => &mut s.children,
                CAttribute::CMultipleAttribute(m) => &mut m.children,
            };
            for ch in children.iter_mut() {
                if object_rm_type(ch) == Some(current_type)
                    && let Some(replacement) = new_obj.take()
                {
                    *ch = replacement;
                    return true;
                }
            }
        }
        false
    })
}

/// Re-type the child of `host`'s `attr` whose current `rm_type` is
/// `current_type` — the first such host/attr/child triple in document order.
/// Unlike [`retype_leaf`] (first `current_type` anywhere), this pins the
/// replacement to the intended slot when the same RM type occurs at several
/// unrelated leaves of the template.
pub fn retype_attr_child(
    opt: &mut OperationalTemplate,
    host: &str,
    attr: &str,
    current_type: &str,
    new_obj: CObject,
) -> bool {
    let mut new_obj = Some(new_obj);
    visit_objects(&mut opt.definition, &mut |obj| {
        if object_rm_type(obj) != Some(host) {
            return false;
        }
        let Some(attrs) = object_attributes_mut(obj) else {
            return false;
        };
        for a in attrs {
            let (name, children) = match a {
                CAttribute::CSingleAttribute(s) => (&s.rm_attribute_name, &mut s.children),
                CAttribute::CMultipleAttribute(m) => (&m.rm_attribute_name, &mut m.children),
            };
            if name != attr {
                continue;
            }
            for ch in children.iter_mut() {
                if object_rm_type(ch) == Some(current_type)
                    && let Some(replacement) = new_obj.take()
                {
                    *ch = replacement;
                    return true;
                }
            }
        }
        false
    })
}

/// A minimal `C_COMPLEX_OBJECT` for a leaf of `rm_type` with no inner attribute
/// constraints (any RM-valid instance accepted) — the base for a slot-retype to a
/// type absent from the base composition.
#[must_use]
pub fn open_complex(rm_type: &str) -> CObject {
    CObject::CComplexObject(openehr_its::opt14::CComplexObject {
        rm_type_name: rm_type.to_owned(),
        occurrences: closed_interval(1, 1),
        node_id: String::new(),
        attributes: Vec::new(),
    })
}

/// Constrain a code-phrase leaf attribute (`DV_CODED_TEXT.defining_code`,
/// `DV_MULTIMEDIA.media_type`) with a `C_CODE_PHRASE`: `terminology` + `codes`.
/// Replaces the attribute's child object with a `C_CODE_PHRASE`. Returns `true` if
/// applied.
pub fn constrain_codephrase(
    opt: &mut OperationalTemplate,
    host: &str,
    attr: &str,
    terminology: &str,
    codes: Vec<String>,
) -> bool {
    let cp = openehr_its::opt14::CCodePhrase {
        rm_type_name: "CODE_PHRASE".to_owned(),
        occurrences: closed_interval(1, 1),
        node_id: String::new(),
        assumed_value: None,
        terminology_id: Some(openehr_base::prelude::TerminologyId {
            value: terminology.to_owned(),
        }),
        code_list: codes,
    };
    let mut cp = Some(cp);
    with_nested_attribute(opt, host, attr, |a| {
        let children = match a {
            CAttribute::CSingleAttribute(s) => &mut s.children,
            CAttribute::CMultipleAttribute(m) => &mut m.children,
        };
        if let Some(cp) = cp.take() {
            if let Some(first) = children.first_mut() {
                *first = CObject::CCodePhrase(cp);
            } else {
                children.push(CObject::CCodePhrase(cp));
            }
        }
    })
}

fn object_node_id(obj: &CObject) -> Option<&str> {
    match obj {
        CObject::CComplexObject(c) => Some(&c.node_id),
        CObject::CArchetypeRoot(r) => Some(&r.node_id),
        _ => None,
    }
}

/// Like [`constrain_codephrase`], but pinned to the `host` object found under
/// the `ELEMENT` with `element_node_id` — the blanket first-match variant hits
/// the first `DV_CODED_TEXT` in document order (the COMPOSITION `category`, in
/// the `all_types` OPT), never the intended leaf.
pub fn constrain_codephrase_under(
    opt: &mut OperationalTemplate,
    archetype: &str,
    element_node_id: &str,
    attr: &str,
    terminology: &str,
    codes: Vec<String>,
) -> bool {
    let cp = openehr_its::opt14::CCodePhrase {
        rm_type_name: "CODE_PHRASE".to_owned(),
        occurrences: closed_interval(1, 1),
        node_id: String::new(),
        assumed_value: None,
        terminology_id: Some(openehr_base::prelude::TerminologyId {
            value: terminology.to_owned(),
        }),
        code_list: codes,
    };
    let mut cp = Some(cp);
    // Node ids are archetype-local (`at0005` recurs in every archetype of the
    // template, and the EVENT_CONTEXT precedes the content in document order),
    // so the ELEMENT search is scoped to the named archetype root.
    let mut in_scope = opt.definition.archetype_id.value.starts_with(archetype);
    visit_objects(&mut opt.definition, &mut |obj| {
        if let CObject::CArchetypeRoot(r) = obj {
            in_scope = r.archetype_id.value.starts_with(archetype);
        }
        if !in_scope || object_node_id(obj) != Some(element_node_id) {
            return false;
        }
        let Some(attrs) = object_attributes_mut(obj) else {
            return false;
        };
        // Descend: ELEMENT.value → the coded host object → `attr` (created
        // when the host is open — an unconstrained coded text carries no
        // `defining_code` C_ATTRIBUTE at all).
        for a in attrs {
            let children = match a {
                CAttribute::CSingleAttribute(s) => &mut s.children,
                CAttribute::CMultipleAttribute(m) => &mut m.children,
            };
            for host in children.iter_mut() {
                let Some(host_attrs) = object_attributes_mut(host) else {
                    continue;
                };
                let slot = host_attrs.iter_mut().find_map(|ha| {
                    let (name, hc) = match ha {
                        CAttribute::CSingleAttribute(s) => (&s.rm_attribute_name, &mut s.children),
                        CAttribute::CMultipleAttribute(m) => {
                            (&m.rm_attribute_name, &mut m.children)
                        }
                    };
                    (name == attr).then_some(hc)
                });
                if let Some(cp_obj) = cp.take() {
                    match slot {
                        Some(hc) => {
                            if let Some(first) = hc.first_mut() {
                                *first = CObject::CCodePhrase(cp_obj);
                            } else {
                                hc.push(CObject::CCodePhrase(cp_obj));
                            }
                        }
                        None => host_attrs.push(CAttribute::CSingleAttribute(CSingleAttribute {
                            rm_attribute_name: attr.to_owned(),
                            existence: closed_interval(1, 1),
                            children: vec![CObject::CCodePhrase(cp_obj)],
                        })),
                    }
                    return true;
                }
            }
        }
        false
    })
}
