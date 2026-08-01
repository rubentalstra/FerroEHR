#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test assertions/diagnostics/fixtures"
)]
//! Model-derived instance generation drives the validation walk over EVERY
//! concrete RM class (Lane 3 of the hardening program).
//!
//! The generators are derived from the BMM-generated static RM model
//! (`openehr_rm::model`) — never hand-written shapes — and the enumeration is
//! deterministic and EXHAUSTIVE over the finite class set (strictly more
//! systematic than random sampling for the same domain; there is nothing for
//! a fuzzer to discover that full enumeration does not visit):
//!
//! 1. **Structural acceptance** — for every concrete class, a minimal
//!    (mandatory-only) and a rich (optionals populated) generated instance
//!    must produce NO structural `does not conform to RM type` violation:
//!    what the model declares structurally valid, the codec-backed walk must
//!    decode. Semantic violations (invariants, terminology) are legitimate on
//!    synthetic values and are not the property.
//! 2. **Mutation refusal** — for every concrete class and every mandatory
//!    attribute, dropping that attribute from the minimal instance must
//!    surface a structural violation naming the missing field. A class where
//!    the drop is NOT caught is a reach gap; the honest register of those is
//!    pinned exactly, so it can only shrink deliberately.

use std::collections::BTreeSet;

use openehr_its::rm_validate::validate_rm_value;
use serde_json::{Map, Value, json};

/// Deterministic scalar for a primitive/foundation type name the static model
/// does not carry as a class. `None` for an unknown name (recorded by the
/// caller — never silently defaulted).
fn primitive(name: &str) -> Option<Value> {
    Some(match name {
        "String" | "Character" | "Terminology_code" | "Any" => json!("x"),
        "Boolean" => json!(true),
        "Integer" | "Integer64" | "Ordered" | "Ordered_numeric" | "Numeric" => json!(1),
        "Real" | "Double" => json!(1.5),
        "Octet" => json!(0),
        "Uri" | "URI" => json!("local://x"),
        // ISO 8601 value strings: several RM classes type their `value`
        // attribute directly as the ISO string form.
        "Iso8601_date" => json!("2020-01-01"),
        "Iso8601_time" => json!("10:00:00"),
        "Iso8601_date_time" => json!("2020-01-01T10:00:00"),
        "Iso8601_duration" => json!("PT1H"),
        "Hash" => Value::Object(Map::new()),
        _ => return None,
    })
}

/// A generated instance of concrete class `class`, derived from the model:
/// `_type`-tagged, every mandatory attribute present; optionals included when
/// `rich`. Abstract attribute types instantiate their first concrete
/// descendant (deterministic); when the class is generic and the caller
/// supplied type arguments, an attribute declared as the bare parameter
/// (which the static model resolves to its BOUND) substitutes the argument —
/// `DV_INTERVAL<DV_QUANTITY>.lower` generates a `DV_QUANTITY`, never an
/// arbitrary `Ordered` descendant. `unknown` collects type names neither the
/// model nor the primitive table covers.
fn generate(
    class: &str,
    args: &[&openehr_rm::model::RmTypeRef],
    rich: bool,
    depth: usize,
    unknown: &mut BTreeSet<String>,
) -> Value {
    let mut obj = Map::new();
    obj.insert("_type".into(), json!(class));
    if depth > 12 {
        return Value::Object(obj);
    }
    let generic_params = openehr_rm::model::class(class).map_or(&[][..], |c| c.generic_params);
    for attr in openehr_rm::model::attributes(class) {
        if !attr.is_mandatory && !rich {
            continue;
        }
        // Rich mode still skips optional RECURSIVE containment (folders in
        // folders, links, feeder audit) to keep instances finite and small.
        if !attr.is_mandatory && depth > 2 {
            continue;
        }
        // Canonical JSON carries `List<Octet>` (DV_MULTIMEDIA.data etc.) as
        // an inline base64 STRING, not a JSON array (ITS-JSON) — the one
        // container shape the codec re-forms.
        if attr.declared_type == "Octet"
            && matches!(attr.container, openehr_rm::model::Container::List)
        {
            obj.insert(attr.name.to_owned(), json!("AA=="));
            continue;
        }
        // Bare-generic-parameter substitution: an attribute whose declared
        // type equals a parameter's bound takes the caller's argument.
        let (ty, ty_args): (&str, &[openehr_rm::model::RmTypeRef]) = generic_params
            .iter()
            .zip(args.iter())
            .find(|(p, _)| p.conforms_to.unwrap_or("Any") == attr.declared_type)
            .map_or((attr.declared_type, attr.type_params), |(_, a)| {
                (a.name, a.params)
            });
        let element = value_for(ty, ty_args, rich, depth, unknown);
        let Some(element) = element else { continue };
        let v = match attr.container {
            openehr_rm::model::Container::None => element,
            openehr_rm::model::Container::List | openehr_rm::model::Container::Set => {
                json!([element])
            }
            openehr_rm::model::Container::Hash => json!({ "x": element }),
        };
        obj.insert(attr.name.to_owned(), v);
    }
    Value::Object(obj)
}

/// A value of declared type `ty` (with its generic arguments): a model class
/// recurses (abstract → first concrete descendant), a primitive comes from
/// the table, anything else is recorded as unknown.
fn value_for(
    ty: &str,
    ty_args: &[openehr_rm::model::RmTypeRef],
    rich: bool,
    depth: usize,
    unknown: &mut BTreeSet<String>,
) -> Option<Value> {
    if let Some(class) = openehr_rm::model::class(ty) {
        let concrete = if class.is_abstract {
            // Deterministic non-recursive preference: the concrete descendant
            // with the FEWEST mandatory class-typed attributes (ELEMENT over
            // CLUSTER for an ITEM slot), so mandatory recursion terminates.
            let mut best: Option<(&str, usize)> = None;
            for d in class.descendants {
                let cost = openehr_rm::model::attributes(d)
                    .filter(|a| {
                        a.is_mandatory && openehr_rm::model::class(a.declared_type).is_some()
                    })
                    .count();
                if best.is_none_or(|(_, c)| cost < c) {
                    best = Some((d, cost));
                }
            }
            best?.0.to_owned()
        } else {
            class.name.to_owned()
        };
        let args: Vec<&openehr_rm::model::RmTypeRef> = ty_args.iter().collect();
        return Some(generate(&concrete, &args, rich, depth + 1, unknown));
    }
    if let Some(e) = openehr_rm::model::enumeration(ty) {
        return e.literals.first().map(|l| match l.value {
            openehr_rm::model::EnumValue::Int(i) => json!(i),
            openehr_rm::model::EnumValue::Str(s) => json!(s),
        });
    }
    let p = primitive(ty);
    if p.is_none() {
        unknown.insert(ty.to_owned());
    }
    p
}

/// Every concrete class of the static model, in declaration order.
fn concrete_classes() -> Vec<&'static str> {
    openehr_rm::model::classes()
        .filter(|c| !c.is_abstract)
        .map(|c| c.name)
        .collect()
}

fn structural_violations(v: &Value) -> Vec<String> {
    let mut out = Vec::new();
    validate_rm_value(v, &mut out);
    out.into_iter()
        .map(|iv| format!("{}: {}", iv.path, iv.message))
        .filter(|m| m.contains("does not conform to RM type"))
        .collect()
}

/// Property 1: what the model declares structurally valid, the walk decodes.
#[test]
fn generated_instances_are_structurally_accepted() {
    let mut unknown = BTreeSet::new();
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for class in concrete_classes() {
        for rich in [false, true] {
            let inst = generate(class, &[], rich, 0, &mut unknown);
            let bad = structural_violations(&inst);
            if !bad.is_empty() {
                failures.push(format!("{class} (rich={rich}): {bad:?}"));
            }
            checked += 1;
        }
    }
    assert!(
        checked > 200,
        "expected the full concrete class set, saw {checked}"
    );
    assert!(
        failures.is_empty(),
        "model-valid instances were structurally refused ({} classes):\n{}",
        failures.len(),
        failures.join("\n")
    );
    // Type names outside both the model and the primitive table: pin the
    // honest register (growth = a model/table gap to close, never silence).
    assert_eq!(
        unknown,
        BTreeSet::new(),
        "attribute types neither the model nor the primitive table cover"
    );
}

/// Property 2: dropping any mandatory attribute is caught structurally —
/// classes where it is NOT are the pinned reach register.
#[test]
fn dropped_mandatory_attributes_are_refused() {
    let mut unknown = BTreeSet::new();
    let mut unreached: BTreeSet<String> = BTreeSet::new();
    let mut mutations = 0usize;
    for class in concrete_classes() {
        let base = generate(class, &[], false, 0, &mut unknown);
        for attr in openehr_rm::model::attributes(class) {
            if !attr.is_mandatory {
                continue;
            }
            let Some(obj) = base.as_object() else {
                continue;
            };
            if !obj.contains_key(attr.name) {
                continue;
            }
            let mut mutated = base.clone();
            mutated
                .as_object_mut()
                .expect("generated instances are objects")
                .remove(attr.name);
            mutations += 1;
            let caught = structural_violations(&mutated)
                .iter()
                .any(|m| m.contains(attr.name));
            if !caught {
                unreached.insert(format!("{class}.{}", attr.name));
            }
        }
    }
    assert!(
        mutations > 300,
        "expected the full mandatory set, saw {mutations}"
    );
    // The honest reach register: every entry is a class whose
    // missing-mandatory defect the dispatcher does not yet surface (the
    // per-node typed dispatch covers only invariant-bearing classes and the
    // fast path vouches without rejecting — the generated structural
    // dispatch that empties this register is #1458). It may only SHRINK (a
    // fix) or grow with an adjudicated reason — never grow silently (the
    // assert names each newcomer).
    let expected: BTreeSet<String> = include_str!("model_walkgen_register.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        unreached, expected,
        "mandatory-drop reach register drifted (fix issue: #1458)"
    );
}
