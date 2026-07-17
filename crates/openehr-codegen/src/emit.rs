//! The Rust emitter: walks a merged BMM [`Model`] and produces
//! idiomatic, strongly-typed Rust for the openEHR spec crates.
//!
//! Emission rules:
//! - **Flattened concrete structs**: a concrete class inlines all inherited
//!   fields (ancestor-first, `// inherited: X` banners); one hop to any field.
//! - **`Option<T>`** for non-mandatory single properties; **`Vec<T>`** for
//!   containers (optional containers get `default` + `skip_serializing_if`).
//! - **Enums** (`#[serde(untagged)]`) for abstract classes used as a property
//!   type — the closed polymorphic slots (`DATA_VALUE`, `ITEM`, …).
//! - **Transparent newtypes** for enumeration classes that are just a
//!   primitive on the wire (`VALIDITY_KIND` → `String`).
//! - **Generics** only for classes the BMM declares generic (`Interval<T>`);
//!   the actual type argument is emitted at each use site.
//! - `_type` is handled by `#[derive(OpenEhrType)]` (`openehr-derive`), not a
//!   per-struct field.
//! - Foundation **primitives / containers / marker traits** are mapped to Rust
//!   (bool, i32, Vec, …) and never emitted (see [`SKIP`] and [`primitive`]).

use crate::bmm::{BmmClass, BmmPropKind, BmmSchema, BmmType};
use crate::naming;
use std::collections::{BTreeMap, BTreeSet};

/// A merged BMM model (e.g. BASE + RM) used for ancestor flattening and type
/// resolution across schema boundaries.
pub(crate) struct Model {
    classes: BTreeMap<String, BmmClass>,
}

/// Spec types provided by *dependency* crates. When a schema references a type
/// it does not emit itself but a dependency does, the emitter resolves it to
/// that crate's prelude (e.g. `openehr_base::prelude::Uid`) instead of
/// degrading to `serde_json::Value`.
#[derive(Default)]
pub(crate) struct External {
    /// Each entry: the set of spec class names a dependency exports, and the
    /// Rust path to import them from (its prelude).
    deps: Vec<(BTreeSet<String>, String)>,
}

impl External {
    /// Register a dependency crate's exported spec names under a prelude path.
    pub(crate) fn with(mut self, specs: BTreeSet<String>, prelude_path: &str) -> Self {
        self.deps.push((specs, prelude_path.to_string()));
        self
    }

    /// The prelude path a dependency exports `spec` from, if any.
    fn prelude_of(&self, spec: &str) -> Option<&str> {
        self.deps
            .iter()
            .find(|(specs, _)| specs.contains(spec))
            .map(|(_, path)| path.as_str())
    }

    fn contains(&self, spec: &str) -> bool {
        self.prelude_of(spec).is_some()
    }
}

/// The spec class names a schema will actually emit (non-skipped), for building
/// the [`External`] index a downstream crate resolves against.
#[must_use]
pub(crate) fn emittable_specs(model: &Model, schema: &BmmSchema) -> BTreeSet<String> {
    let used = model.used_as_type();
    schema
        .classes
        .iter()
        .filter(|(_, c)| !matches!(decide(model, c, &used), Emission::Skip))
        .map(|(n, _)| n.clone())
        .collect()
}

/// A property resolved onto a concrete class, tracking which class it came from
/// (for the `// inherited: X` banner).
pub(crate) struct ResolvedProp<'a> {
    pub(crate) owner: String,
    pub(crate) prop: &'a crate::bmm::BmmProperty,
}

/// Foundation classes that are **mapped to Rust and never emitted**: the
/// container types, marker/functional/service classes, and constant holders.
/// Scalar primitives are handled by [`primitive`]. `Multiplicity_interval` and
/// `Cardinality` *are* emitted (real structs with fields); their inherited
/// `Interval<Integer>` binding is supplied by [`class_binding`].
const SKIP: &[&str] = &[
    // containers → Vec / handled by container properties
    "Container",
    "List",
    "Set",
    "Array",
    "Hash",
    // abstract marker / algebraic traits (no data)
    "Any",
    "Ordered",
    "Numeric",
    "Ordered_Numeric",
    "Comparable",
    "Temporal",
    // functional types
    "TUPLE",
    "TUPLE1",
    "TUPLE2",
    "ROUTINE",
    "FUNCTION",
    "PROCEDURE",
    // service interfaces (no data)
    "Env",
    "Locale",
    "Math",
    "Quantity_converter",
    "Statistical_evaluator",
    // constant-holder classes (no data; become assoc consts in *_impl.rs)
    "Time_Definitions",
    "BASIC_DEFINITIONS",
    "OPENEHR_DEFINITIONS",
];

impl Model {
    /// Merge several schemas into one class map (later schemas override earlier
    /// on name collision — pass BASE before RM).
    #[must_use]
    pub(crate) fn merged(schemas: &[&BmmSchema]) -> Self {
        let mut classes = BTreeMap::new();
        for s in schemas {
            for (name, class) in &s.classes {
                classes.insert(name.clone(), class.clone());
            }
        }
        Model { classes }
    }

    pub(crate) fn get(&self, name: &str) -> Option<&BmmClass> {
        self.classes.get(name)
    }

    /// Iterate every class in the merged model, in name order.
    pub(crate) fn class_iter(&self) -> impl Iterator<Item = (&String, &BmmClass)> {
        self.classes.iter()
    }

    /// Whether `name` is a generic parameter declared on `class` or any of its
    /// (transitive) ancestors — e.g. `T` for `INTERVAL_EVENT` (declared on the
    /// ancestor `EVENT<T>`). Used to resolve a bare-parameter attribute type to
    /// its bound in the concrete class's scope.
    pub(crate) fn is_generic_param(&self, class: &str, name: &str) -> bool {
        let Some(c) = self.get(class) else {
            return false;
        };
        c.generic_params.iter().any(|g| g.name == name)
            || c.ancestors.iter().any(|a| self.is_generic_param(a, name))
    }

    /// Is `name` mapped to Rust rather than emitted (primitive or [`SKIP`])?
    pub(crate) fn is_mapped(name: &str) -> bool {
        primitive(name).is_some() || SKIP.contains(&name)
    }

    /// Does `class` inherit from `target` (transitively)?
    pub(crate) fn inherits(&self, class: &str, target: &str) -> bool {
        let Some(c) = self.get(class) else {
            return false;
        };
        for a in &c.ancestors {
            if a == target || self.inherits(a, target) {
                return true;
            }
        }
        false
    }

    /// Can `from` transitively reach `target` through **`Single`** (non-`Vec`)
    /// field types? Used to detect struct-sizing cycles that need boxing
    /// (`Vec`/`Box` already break a cycle; `Option<T>` and plain `T` do not).
    fn reaches(&self, from: &str, target: &str, seen: &mut BTreeSet<String>) -> bool {
        if from == target {
            return true;
        }
        if !seen.insert(from.to_string()) {
            return false;
        }
        let Some(class) = self.get(from) else {
            return false;
        };
        // An abstract class is emitted as an untagged enum; a cycle can run
        // through its variants (e.g. ARCHETYPE_CONSTRAINT ↔ ARCHETYPE_SLOT,
        // EXPR_ITEM ↔ EXPR_BINARY_OPERATOR). Traverse them too.
        if class.is_abstract {
            for d in self.enum_variants(from) {
                if d == target || self.reaches(&d, target, seen) {
                    return true;
                }
            }
            return false;
        }
        for rp in self.flattened_props(class) {
            if let BmmPropKind::Single(t) = &rp.prop.kind {
                let root = t.root_name();
                if Self::is_mapped(root) {
                    continue;
                }
                if root == target || self.reaches(root, target, seen) {
                    return true;
                }
            }
        }
        false
    }

    /// The *immediate* concrete, emittable subtypes of `name` — the variants of
    /// its untagged enum. Generic descendants are included (the enum is emitted
    /// generic over the class's own params, `Event<T> { PointEvent(PointEvent<T>) }`).
    ///
    /// A descendant `D` is dropped when another concrete descendant `C` sits
    /// between `name` and `D`, because `C` is itself emitted as an enum that
    /// covers `D` — so the enums nest by the type hierarchy (`DATA_VALUE ⊇ DvText`,
    /// `DvText ⊇ DvCodedText`) instead of `DATA_VALUE` flatly listing both
    /// `DV_TEXT` and `DV_CODED_TEXT` (which would double-match on the wire).
    fn enum_variants(&self, name: &str) -> Vec<String> {
        let all: Vec<String> = self
            .classes
            .values()
            .filter(|c| {
                !c.is_abstract
                    && c.name != name
                    && !Self::is_mapped(&c.name)
                    && self.inherits(&c.name, name)
            })
            .map(|c| c.name.clone())
            .collect();
        all.iter()
            .filter(|d| {
                !all.iter()
                    .any(|c| c.as_str() != d.as_str() && self.inherits(d, c))
            })
            .cloned()
            .collect()
    }

    /// Class names used anywhere as a property type — enum-slot candidates.
    fn used_as_type(&self) -> BTreeSet<String> {
        let mut used = BTreeSet::new();
        for c in self.classes.values() {
            for p in &c.properties {
                match &p.kind {
                    BmmPropKind::Single(t) => collect_roots(t, &mut used),
                    BmmPropKind::Container { item, .. } => collect_roots(item, &mut used),
                }
            }
        }
        used
    }

    /// Flatten a class's properties, ancestor-first, with child redefinitions
    /// overriding the inherited type in place.
    pub(crate) fn flattened_props(&self, class: &BmmClass) -> Vec<ResolvedProp<'_>> {
        let mut order: Vec<String> = Vec::new();
        let mut map: BTreeMap<String, ResolvedProp<'_>> = BTreeMap::new();
        self.gather(&class.name, &mut order, &mut map);
        order.into_iter().filter_map(|n| map.remove(&n)).collect()
    }

    fn gather<'a>(
        &'a self,
        class_name: &str,
        order: &mut Vec<String>,
        map: &mut BTreeMap<String, ResolvedProp<'a>>,
    ) {
        let Some(class) = self.get(class_name) else {
            return;
        };
        for anc in &class.ancestors {
            self.gather(anc, order, map);
        }
        for p in &class.properties {
            if !map.contains_key(&p.name) {
                order.push(p.name.clone());
            }
            map.insert(
                p.name.clone(),
                ResolvedProp {
                    owner: class.name.clone(),
                    prop: p,
                },
            );
        }
    }

    // ── type rendering ──────────────────────────────────────────────────────

    /// Render a BMM type to Rust. `local` is the set of spec class names emitted
    /// in the current crate; `external` maps types provided by dependency
    /// crates. `subst` binds a generic parameter name to a concrete spec type
    /// (an ancestor-generic binding the BMM drops, e.g. `Multiplicity_interval`'s
    /// `T` → `Integer`). A referenced class in neither `local` nor `external`
    /// (or a malformed container) degrades to `serde_json::Value`. Local and
    /// external class types render as the bare ident (the import machinery adds
    /// the right `use`).
    fn render_type(
        &self,
        t: &BmmType,
        generics: &[String],
        subst: &BTreeMap<String, String>,
        local: &BTreeSet<String>,
        external: &External,
    ) -> String {
        match t {
            BmmType::Simple(n) => {
                if let Some(concrete) = subst.get(n) {
                    // A bound ancestor-generic parameter: render the concrete.
                    self.render_type(
                        &BmmType::Simple(concrete.clone()),
                        generics,
                        subst,
                        local,
                        external,
                    )
                } else if let Some(p) = primitive(n) {
                    p.to_string()
                } else if generics.iter().any(|g| g == n) {
                    n.clone()
                } else if n == "Any" {
                    "serde_json::Value".to_string()
                } else if local.contains(n) || external.contains(n) {
                    // A bare reference to a *generic* class (BMM omits the args,
                    // e.g. `normal_range: DV_INTERVAL`) is filled with each type
                    // parameter's bound (`DV_INTERVAL` → `DvInterval<DvOrdered>`).
                    // An *unbounded* parameter (the versioned-content `T` of the
                    // VERSION family) is threaded from the enclosing scope's type
                    // params (`generics`, then bound `subst` values), so it stays
                    // strongly typed instead of degrading to `serde_json::Value`.
                    match self.generic_param_bounds(n) {
                        Some(bounds) => {
                            let mut content = Self::scope_content_types(generics, subst);
                            let args: Vec<String> = bounds
                                .iter()
                                .map(|b| match b {
                                    Some(bound) => self.render_type(
                                        &BmmType::Simple(bound.clone()),
                                        generics,
                                        subst,
                                        local,
                                        external,
                                    ),
                                    None => content
                                        .next()
                                        .unwrap_or_else(|| "serde_json::Value".to_string()),
                                })
                                .collect();
                            format!("{}<{}>", naming::type_name(n), args.join(", "))
                        }
                        None => naming::type_name(n),
                    }
                } else {
                    "serde_json::Value".to_string()
                }
            }
            BmmType::Generic { root, params } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| self.render_type(p, generics, subst, local, external))
                    .collect();
                // Foundation container generics map to Rust collections; a
                // container with the wrong arity (e.g. the deeply-nested
                // free-form `Hash` in RESOURCE_ANNOTATIONS) is free-form JSON.
                match root.as_str() {
                    "Hash" if ps.len() == 2 => {
                        format!("std::collections::BTreeMap<{}>", ps.join(", "))
                    }
                    "List" | "Array" if ps.len() == 1 => format!("Vec<{}>", ps[0]),
                    "Set" if ps.len() == 1 => format!("std::collections::BTreeSet<{}>", ps[0]),
                    // A container of the wrong arity (e.g. the deeply-nested
                    // free-form `Hash` in RESOURCE_ANNOTATIONS) or a type neither
                    // emitted here nor by a dependency → free-form JSON.
                    "Hash" | "List" | "Array" | "Set" => "serde_json::Value".to_string(),
                    _ if !local.contains(root) && !external.contains(root) => {
                        "serde_json::Value".to_string()
                    }
                    // Respect the class's *effective* arity: a class whose only
                    // param was unused is monomorphized (emitted non-generic), so
                    // a reference must drop the explicit args (`REFERENCE_RANGE<X>`
                    // → `ReferenceRange`).
                    _ => match self.generic_param_bounds(root) {
                        None => naming::type_name(root),
                        Some(_) => format!("{}<{}>", naming::type_name(root), ps.join(", ")),
                    },
                }
            }
        }
    }

    /// The generic params a class *actually uses* in its (flattened) fields —
    /// those referenced explicitly as the bare param type. A declared param
    /// whose only occurrences are inside auto-filled bare generic references
    /// (which resolve to bounds, not the param) is dropped, so the emitted
    /// struct is not generic over an unused `T` (which Rust rejects). Returns
    /// `(name, bound)` pairs in declaration order.
    fn used_generic_params(&self, name: &str) -> Vec<(String, Option<String>)> {
        let Some(class) = self.get(name) else {
            return Vec::new();
        };
        if class.generic_params.is_empty() {
            return Vec::new();
        }
        // An abstract class is emitted as an untagged enum generic over the
        // declared params any *concrete variant* uses — a subtype family whose
        // parameter is exposed only by descendants (`VERSION<T>` carries `T`
        // only through `ORIGINAL_VERSION.data: T`) still needs `Version<T>`, and
        // a bare reference to it must resolve to the same arity.
        if class.is_abstract {
            let variants = self.enum_variants(name);
            return class
                .generic_params
                .iter()
                .filter(|g| {
                    variants.iter().any(|d| {
                        self.used_generic_params(d)
                            .iter()
                            .any(|(n, _)| *n == g.name)
                    })
                })
                .map(|g| (g.name.clone(), self.resolved_param_bound(name, &g.name)))
                .collect();
        }
        let declared: BTreeSet<&str> = class
            .generic_params
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        let mut used = BTreeSet::new();
        let mut threads_own_param = false;
        for rp in self.flattened_props(class) {
            match &rp.prop.kind {
                BmmPropKind::Single(t) | BmmPropKind::Container { item: t, .. } => {
                    collect_param_uses(t, &declared, &mut used);
                    // A bare reference to an *unbounded* generic class threads
                    // this class's own parameter into that open slot
                    // (`IMPORTED_VERSION.item: ORIGINAL_VERSION` →
                    // `OriginalVersion<T>`), so the parameter is used even though
                    // it never appears literally in the (BMM-dropped) type args.
                    if let BmmType::Simple(n) = t
                        && self
                            .generic_param_bounds(n)
                            .is_some_and(|b| b.iter().any(Option::is_none))
                    {
                        threads_own_param = true;
                    }
                }
            }
        }
        class
            .generic_params
            .iter()
            .filter(|g| used.contains(g.name.as_str()) || threads_own_param)
            .map(|g| (g.name.clone(), self.resolved_param_bound(name, &g.name)))
            .collect()
    }

    /// The `conforms_to` bound of generic parameter `param` on `class`, resolved
    /// up the ancestor chain. A subclass may redeclare an inherited parameter
    /// without repeating its bound (`INTERVAL_EVENT<T>` re-lists `T` but the
    /// `T: ITEM_STRUCTURE` bound lives on `EVENT<T>`); the bound is inherited by
    /// parameter name, since the family reuses the same name down the hierarchy.
    pub(crate) fn resolved_param_bound(&self, class_name: &str, param: &str) -> Option<String> {
        let class = self.get(class_name)?;
        if let Some(g) = class.generic_params.iter().find(|g| g.name == param)
            && let Some(bound) = &g.conforms_to
        {
            return Some(bound.clone());
        }
        class
            .ancestors
            .iter()
            .find_map(|anc| self.resolved_param_bound(anc, param))
    }

    /// The concrete content types available in the current emission scope, in
    /// priority order, for threading into an *unbounded* generic slot: the free
    /// generic-parameter names first (a generic class threads its own `T`), then
    /// the rendered values of any ancestor-generic `subst` bindings (a
    /// monomorphized class threads its bound content type).
    fn scope_content_types(
        generics: &[String],
        subst: &BTreeMap<String, String>,
    ) -> std::vec::IntoIter<String> {
        let mut v: Vec<String> = generics.to_vec();
        for val in subst.values() {
            v.push(primitive(val).map_or_else(|| naming::type_name(val), str::to_string));
        }
        v.into_iter()
    }

    /// The bounds to auto-fill for a bare reference to a generic class, or
    /// `None` if the class has no *used* generic params (so a bare reference
    /// stays bare). Consistent with [`Self::used_generic_params`].
    fn generic_param_bounds(&self, name: &str) -> Option<Vec<Option<String>>> {
        let used = self.used_generic_params(name);
        if used.is_empty() {
            None
        } else {
            Some(used.into_iter().map(|(_, bound)| bound).collect())
        }
    }

    /// Every spec class name the *rendered* type of `t` embeds by value — its
    /// root, explicit generic args, and (for a bare generic-class reference) the
    /// auto-filled parameter bounds. Used for both imports and cycle detection
    /// so they stay consistent with [`Self::render_type`].
    fn effective_roots(&self, t: &BmmType, out: &mut BTreeSet<String>) {
        match t {
            BmmType::Simple(n) => {
                out.insert(n.clone());
                if let Some(bounds) = self.generic_param_bounds(n) {
                    for b in bounds.into_iter().flatten() {
                        self.effective_roots(&BmmType::Simple(b), out);
                    }
                }
            }
            BmmType::Generic { root, params } => {
                out.insert(root.clone());
                // Only recurse into the args that survive rendering: a container
                // (`List<T>` → `Vec<T>`) keeps them, but a spec class whose sole
                // param was unused is monomorphized (`BMM_LITERAL_VALUE<BMM_TYPE>`
                // → `BmmLiteralValue`), dropping the args — mirror `render_type`
                // so imports/cycle detection do not see a phantom arg.
                let keeps_args = matches!(root.as_str(), "List" | "Array" | "Set" | "Hash")
                    || self.generic_param_bounds(root).is_some();
                if keeps_args {
                    for p in params {
                        self.effective_roots(p, out);
                    }
                }
            }
        }
    }

    /// The emittable spec class names a class refers to in its (flattened)
    /// fields — for computing precise `use` imports. Excludes primitives,
    /// generic params, mapped/skip types, and `Any`.
    fn referenced_specs(
        &self,
        class: &BmmClass,
        generics: &[String],
        subst: &BTreeMap<String, String>,
    ) -> BTreeSet<String> {
        let mut roots = BTreeSet::new();
        for rp in self.flattened_props(class) {
            match &rp.prop.kind {
                BmmPropKind::Single(t) => self.effective_roots(t, &mut roots),
                BmmPropKind::Container { item, .. } => self.effective_roots(item, &mut roots),
            }
        }
        roots
            .into_iter()
            // Substitute a bound generic parameter (`T` → `COMPOSITION`) so the
            // concrete content type is imported, not the parameter name.
            .map(|n| subst.get(&n).cloned().unwrap_or(n))
            .filter(|n| !Self::is_mapped(n) && n != "Any" && !generics.iter().any(|g| g == n))
            .collect()
    }
}

/// Ancestor-generic bindings the BMM drops (it records `ancestors` and some
/// generic-content property types as bare class names, losing the `<Integer>` /
/// `<COMPOSITION>` argument). Maps a class's generic-parameter name to the
/// concrete spec type it is instantiated with, so the emitter can substitute it
/// instead of degrading the field to `serde_json::Value`. Seeded here; slated to
/// move to `codegen.toml` alongside [`type_override`].
fn class_binding(class: &str) -> BTreeMap<String, String> {
    let pairs: &[(&str, &str)] = match class {
        // "An Interval of Integer" — openEHR files it under `primitive_types`
        // without carrying the `Interval<Integer>` binding.
        "Multiplicity_interval" => &[("T", "Integer")],
        // The EHR-Extract version containers bind the versioned-content type
        // that `X_VERSIONED_OBJECT<T>` leaves open.
        "X_VERSIONED_COMPOSITION" => &[("T", "COMPOSITION")],
        "X_VERSIONED_EHR_ACCESS" => &[("T", "EHR_ACCESS")],
        "X_VERSIONED_EHR_STATUS" => &[("T", "EHR_STATUS")],
        "X_VERSIONED_PARTY" => &[("T", "PARTY")],
        "X_VERSIONED_FOLDER" => &[("T", "FOLDER")],
        _ => &[],
    };
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// The set of primitive spec types → Rust types (the codegen type map).
fn primitive(name: &str) -> Option<&'static str> {
    Some(match name {
        "Boolean" => "bool",
        "Integer" => "i32",
        "Integer64" => "i64",
        "Real" | "Double" => "f64",
        // `Uri` is a plain string until the strong-newtype override lands.
        "String" | "Uri" => "String",
        "Octet" => "u8",
        "Character" => "char",
        _ => return None,
    })
}

fn collect_roots(t: &BmmType, out: &mut BTreeSet<String>) {
    match t {
        BmmType::Simple(n) => {
            out.insert(n.clone());
        }
        BmmType::Generic { root, params } => {
            out.insert(root.clone());
            for p in params {
                collect_roots(p, out);
            }
        }
    }
}

/// Record which of `declared` generic-parameter names appear explicitly in `t`
/// (as the bare param type or an explicit generic argument).
fn collect_param_uses(t: &BmmType, declared: &BTreeSet<&str>, out: &mut BTreeSet<String>) {
    match t {
        BmmType::Simple(n) => {
            if declared.contains(n.as_str()) {
                out.insert(n.clone());
            }
        }
        BmmType::Generic { root, params } => {
            if declared.contains(root.as_str()) {
                out.insert(root.clone());
            }
            for p in params {
                collect_param_uses(p, declared, out);
            }
        }
    }
}

/// A generated Rust source file (path relative to the crate `src/`, plus body).
pub(crate) struct GenFile {
    /// Relative path under the crate `src/`, e.g. `data_types/quantity/dv_quantity.rs`.
    pub path: String,
    /// The Rust source.
    pub body: String,
}

/// What to emit for a class.
enum Emission<'a> {
    Struct,
    /// Untagged enum for an *abstract* polymorphic slot: one variant per
    /// immediate concrete subtype (the abstract class itself is not instantiable).
    Enum(Vec<String>),
    /// Untagged enum for a *concrete* class that also has subtypes
    /// (`DV_TEXT` → `DV_CODED_TEXT`): a field typed as the parent accepts either.
    /// Emits a `{Name}Data` struct for the class's own instances plus a `{Name}`
    /// enum over `{Name}Data` and each immediate concrete subtype.
    PolyEnum(Vec<String>),
    /// Transparent newtype over a Rust primitive (an enumeration-of-strings, …).
    Newtype(&'a str),
    Skip,
}

/// Decide how a class is emitted.
fn decide<'a>(model: &Model, class: &'a BmmClass, used: &BTreeSet<String>) -> Emission<'a> {
    if Model::is_mapped(&class.name) {
        return Emission::Skip;
    }
    if class.is_abstract {
        let variants = model.enum_variants(&class.name);
        if !variants.is_empty() {
            // A closed polymorphic slot. Emit the untagged enum whenever the
            // class has concrete descendants — even if *this* schema never uses
            // it as a field type, because a downstream crate may (e.g. `Interval`
            // is a BASE foundation type referenced by AM's `Interval<Integer>`).
            Emission::Enum(variants)
        } else if used.contains(&class.name) {
            // Abstract, referenced as a field type, but no concrete descendants
            // in this schema (e.g. `AUTHORED_RESOURCE` in BASE — its concretes
            // live in AM). Emit its own fields as a struct so the reference
            // resolves; a cross-schema pass can promote it to an enum later.
            Emission::Struct
        } else {
            Emission::Skip
        }
    } else {
        // Concrete but with its own concrete subtypes: a field typed as this
        // class accepts the subtype too (`DV_TEXT` holds a `DV_CODED_TEXT`, a
        // coded name), so emit a polymorphic enum plus the `{Name}Data` struct.
        let variants = model.enum_variants(&class.name);
        if !variants.is_empty() {
            return Emission::PolyEnum(variants);
        }
        // Concrete leaf: a 0-field class whose sole ancestor is a primitive is
        // an enumeration-style newtype (VALIDITY_KIND → String).
        let flattened = model.flattened_props(class);
        if flattened.is_empty()
            && class.ancestors.len() == 1
            && let Some(prim) = primitive(&class.ancestors[0])
        {
            return Emission::Newtype(prim);
        }
        Emission::Struct
    }
}

/// One emitted type and the module chain it lives in (for import + prelude).
struct Emitted {
    /// Module chain under the crate root, e.g. `["base_types","identification","uid"]`.
    chain: Vec<String>,
    /// Rust type identifier, e.g. `Uid`.
    ident: String,
}

/// The generated files for one schema version plus its top-level module names.
struct Version {
    files: Vec<GenFile>,
    /// Top-level module names of this version (under its prefix, if any).
    top: BTreeSet<String>,
}

/// Emit one schema version under `prefix` (empty for a single-version crate).
/// Produces the type files, the `mod.rs` tree, and a `prelude.rs`; the caller
/// assembles `lib.rs`.
fn emit_version(model: &Model, schema: &BmmSchema, prefix: &str, external: &External) -> Version {
    struct Planned<'a> {
        class: &'a BmmClass,
        emission: Emission<'a>,
        chain: Vec<String>,
    }

    let class_pkg = class_paths(schema);
    let used = model.used_as_type();

    // Spec class names emitted in this version; anything referenced outside it
    // degrades to `serde_json::Value` so the crate stays self-contained.
    let mut local: BTreeSet<String> = BTreeSet::new();
    for (name, class) in &schema.classes {
        if !matches!(decide(model, class, &used), Emission::Skip) {
            local.insert(name.clone());
        }
    }

    let mut planned = Vec::new();
    let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new(); // ident → chain
    for (name, class) in &schema.classes {
        let emission = decide(model, class, &used);
        if matches!(emission, Emission::Skip) {
            continue;
        }
        let pkg = class_pkg.get(name).cloned().unwrap_or_default();
        let mut chain: Vec<String> = Vec::new();
        if !prefix.is_empty() {
            chain.push(prefix.to_string());
        }
        chain.extend(pkg.split('/').filter(|s| !s.is_empty()).map(str::to_string));
        chain.push(naming::field_ident(&to_snake(name)));
        index.insert(naming::type_name(name), chain.clone());
        // A polymorphic-concrete class emits a sibling `{Name}Data` struct in the
        // same file (the enum owns `{Name}`); export it from the prelude too so
        // downstream code (e.g. the generated XML impls) can name it.
        if matches!(emission, Emission::PolyEnum(_)) {
            index.insert(format!("{}Data", naming::type_name(name)), chain.clone());
        }
        planned.push(Planned {
            class,
            emission,
            chain,
        });
    }

    let mut files = Vec::new();
    for p in &planned {
        let body = match &p.emission {
            Emission::Struct => emit_struct(model, p.class, &index, &local, external),
            Emission::Enum(variants) => {
                emit_enum(model, p.class, variants, false, &index, &local, external)
            }
            Emission::PolyEnum(variants) => {
                emit_enum(model, p.class, variants, true, &index, &local, external)
            }
            Emission::Newtype(prim) => emit_newtype(p.class, prim),
            Emission::Skip => unreachable!(),
        };
        files.push(GenFile {
            path: format!("{}.rs", p.chain.join("/")),
            body,
        });
    }

    let type_chains: Vec<Vec<String>> = planned.iter().map(|p| p.chain.clone()).collect();
    let emitted: Vec<Emitted> = index
        .into_iter()
        .map(|(ident, chain)| Emitted { chain, ident })
        .collect();

    // Module tree. For a prefixed version, also register `<prefix>/prelude` so
    // the prefix `mod.rs` declares it.
    let mut tree_chains = type_chains.clone();
    let prelude_path = if prefix.is_empty() {
        "prelude.rs".to_string()
    } else {
        tree_chains.push(vec![prefix.to_string(), "prelude".to_string()]);
        format!("{prefix}/prelude.rs")
    };
    files.extend(emit_module_tree(&tree_chains));
    files.push(emit_prelude(&emitted, &prelude_path));

    // Top modules: the prefix itself if prefixed, else the type roots.
    let top = if prefix.is_empty() {
        top_modules(&type_chains)
    } else {
        std::iter::once(prefix.to_string()).collect()
    };
    Version { files, top }
}

/// Emit a single-version crate (`openehr-base`): one schema, top-level modules,
/// crate `prelude`, and `lib.rs`. `external` resolves dependency-crate types.
#[must_use]
pub(crate) fn emit_crate(
    model: &Model,
    schema: &BmmSchema,
    external: &External,
    crate_doc: &str,
) -> Vec<GenFile> {
    let v = emit_version(model, schema, "", external);
    let mut files = v.files;
    files.push(emit_lib(&v.top, true, crate_doc));
    files
}

/// Emit a multi-version crate (`openehr-am`): each `(prefix, model, schema)`
/// becomes a top-level version module (`am14`, `am24`) with its own prelude.
#[must_use]
pub(crate) fn emit_multi_crate(
    versions: &[(&str, &Model, &BmmSchema)],
    external: &External,
    crate_doc: &str,
) -> Vec<GenFile> {
    let mut files = Vec::new();
    let mut top: BTreeSet<String> = BTreeSet::new();
    for (prefix, model, schema) in versions {
        let v = emit_version(model, schema, prefix, external);
        files.extend(v.files);
        top.extend(v.top);
    }
    files.push(emit_lib(&top, false, crate_doc));
    files
}

/// Build every `mod.rs` from the set of emitted module chains.
fn emit_module_tree(chains: &[Vec<String>]) -> Vec<GenFile> {
    // dir path ("" = root is handled by lib.rs) → set of child module idents.
    let mut dirs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for chain in chains {
        for i in 1..chain.len() {
            let dir = chain[..i].join("/");
            dirs.entry(dir).or_default().insert(chain[i].clone());
        }
    }
    dirs.into_iter()
        .map(|(dir, children)| {
            let mut b = String::from("// @generated by openehr-codegen — DO NOT EDIT.\n\n");
            for c in &children {
                b.push_str(&format!("pub mod {c};\n"));
            }
            GenFile {
                path: format!("{dir}/mod.rs"),
                body: b,
            }
        })
        .collect()
}

/// Top-level module names (first chain segment), deduped.
fn top_modules(chains: &[Vec<String>]) -> BTreeSet<String> {
    chains.iter().filter_map(|c| c.first().cloned()).collect()
}

fn emit_lib(top: &BTreeSet<String>, include_prelude: bool, crate_doc: &str) -> GenFile {
    let mut b = String::new();
    for line in crate_doc.lines() {
        b.push_str(&format!("//! {line}\n"));
    }
    b.push_str("//!\n//! @generated module tree by openehr-codegen. The type files\n");
    b.push_str("//! are generated; hand-written spec behaviour lives in sibling `*_impl.rs`.\n\n");
    // Lint exceptions inherent to faithful spec generation:
    //  - doc comments are verbatim openEHR spec text (bare URLs, un-backticked
    //    terms, tabs, quote-style links, loose list continuation);
    //  - some spec classes carry >3 boolean flags (e.g. `Interval` bounds);
    //  - the package tree can nest a module of the same name (module_inception);
    //  - closed-slot enums can have size-disparate variants.
    b.push_str(
        "#![allow(\n    \
         clippy::doc_markdown,\n    \
         clippy::doc_link_with_quotes,\n    \
         clippy::tabs_in_doc_comments,\n    \
         clippy::doc_lazy_continuation,\n    \
         clippy::struct_excessive_bools,\n    \
         clippy::module_inception,\n    \
         clippy::large_enum_variant\n\
         )]\n\n",
    );
    for m in top {
        b.push_str(&format!("pub mod {m};\n"));
    }
    if include_prelude {
        b.push_str("pub mod prelude;\n");
    }
    GenFile {
        path: "lib.rs".to_string(),
        body: b,
    }
}

fn emit_prelude(emitted: &[Emitted], path: &str) -> GenFile {
    let mut b = String::from(
        "//! Prelude: re-exports every generated spec type of this version.\n\
         //!\n//! @generated by openehr-codegen. Per-file imports are precise;\n\
         //! downstream crates and hand-written code may `use <path>::*`.\n\n",
    );
    let mut lines: Vec<String> = emitted
        .iter()
        .map(|e| format!("pub use crate::{}::{};", e.chain.join("::"), e.ident))
        .collect();
    lines.sort();
    for l in lines {
        b.push_str(&l);
        b.push('\n');
    }
    GenFile {
        path: path.to_string(),
        body: b,
    }
}

/// Build a class → nested directory path map from the package tree, e.g.
/// `DV_QUANTITY` → `data_types/quantity`.
fn class_paths(schema: &BmmSchema) -> BTreeMap<String, String> {
    fn walk(p: &crate::bmm::BmmPackage, prefix: &str, out: &mut BTreeMap<String, String>) {
        let seg = p.name.rsplit('.').next().unwrap_or(&p.name);
        let path = if prefix.is_empty() {
            seg.to_string()
        } else {
            format!("{prefix}/{seg}")
        };
        for c in &p.classes {
            out.insert(c.clone(), path.clone());
        }
        for sub in &p.packages {
            walk(sub, &path, out);
        }
    }
    let mut out = BTreeMap::new();
    for p in &schema.packages {
        walk(p, "", &mut out);
    }
    out
}

fn emit_struct(
    model: &Model,
    class: &BmmClass,
    index: &BTreeMap<String, Vec<String>>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    let ty = naming::type_name(&class.name);
    let generics = struct_generics(model, class);
    let subst = class_binding(&class.name);

    let mut b = String::new();
    let imports = import_lines(model, class, &generics, &subst, &ty, index, external);
    struct_header(&mut b, &class.name, &imports);
    b.push_str(&render_struct_def(
        model, class, &ty, &generics, &subst, local, external,
    ));
    b
}

/// The params a struct is generic over (see `used_generic_params`).
fn struct_generics(model: &Model, class: &BmmClass) -> Vec<String> {
    model
        .used_generic_params(&class.name)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The struct definition (doc, derive, fields) under the name `struct_ty`,
/// without the file header. `struct_ty` is normally `type_name(class)`, but a
/// polymorphic-concrete class emits its own instances as `{Name}Data` (the
/// enum owns `{Name}`). The canonical `_type` stays the class name either way.
fn render_struct_def(
    model: &Model,
    class: &BmmClass,
    struct_ty: &str,
    generics: &[String],
    subst: &BTreeMap<String, String>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    let gen_decl = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };
    let mut b = String::new();
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str("#[derive(Debug, Clone, PartialEq, OpenEhrType)]\n");
    b.push_str(&format!("#[openehr(type_name = \"{}\")]\n", class.name));
    b.push_str(&format!("pub struct {struct_ty}{gen_decl} {{\n"));

    let props = model.flattened_props(class);
    let mut prev_owner: Option<&str> = None;
    let mut first = true;
    for rp in &props {
        let p = rp.prop;
        if rp.owner != class.name && prev_owner != Some(rp.owner.as_str()) {
            // Blank line before a new `// inherited:` group, but not as the very
            // first line inside the braces (rustfmt strips a leading blank line).
            let sep = if first { "" } else { "\n" };
            b.push_str(&format!("{sep}    // inherited: {}\n", rp.owner));
        }
        prev_owner = Some(rp.owner.as_str());
        first = false;
        doc_block(&mut b, p.documentation.as_deref(), "    ");

        let ident = naming::field_ident(&p.name);
        if let Some(rename) = naming::serde_rename(&p.name, &ident) {
            b.push_str(&format!("    #[openehr(rename = \"{rename}\")]\n"));
        }
        if let Some(default) = field_default(&rp.owner, &p.name) {
            b.push_str(&format!("    #[openehr(default = \"{default}\")]\n"));
        }
        let rust_ty = field_type(model, class, p, generics, subst, local, external);
        b.push_str(&format!("    pub {ident}: {rust_ty},\n"));
    }

    b.push_str("}\n");
    b
}

/// Field-level type overrides mapping a `(class, field)` to a proven Rust crate
/// type instead of the BMM primitive (the codegen override layer). Seeded here;
/// slated to move to `codegen.toml`. Only unambiguous mappings belong here —
/// where openEHR's semantics are broader than a crate (partial-precision ISO
/// 8601, plain-text URIs) the field stays `String` and the crate is used in the
/// hand-written `*_impl.rs` behavior instead.
fn type_override(class: &str, field: &str) -> Option<&'static str> {
    match (class, field) {
        // A UUID is an RFC-4122 canonical UUID — use the `uuid` crate directly.
        // (ISO_OID / INTERNET_ID / OBJECT_VERSION_ID are *not* plain UUIDs.)
        ("UUID", "value") => Some("uuid::Uuid"),
        _ => None,
    }
}

/// A serde default for a field the canonical wire may omit, keyed by the field's
/// declaring class (`owner`) and name. `Interval`'s inclusivity/boundedness
/// flags are mandatory in the BMM but archie/EHRbase omit them: a bounded limit
/// is *included* by default, and an unstated limit is *bounded* by default.
/// The value is a literal Rust expression consumed by `#[openehr(default = …)]`.
fn field_default(owner: &str, field: &str) -> Option<&'static str> {
    if owner != "Interval" {
        return None;
    }
    match field {
        "lower_included" | "upper_included" => Some("true"),
        "lower_unbounded" | "upper_unbounded" => Some("false"),
        _ => None,
    }
}

/// Compute a field's Rust type (`OpenEhrType` handles skip-if-none/empty, so no
/// serde attributes are needed on the field).
fn field_type(
    model: &Model,
    class: &BmmClass,
    p: &crate::bmm::BmmProperty,
    generics: &[String],
    subst: &BTreeMap<String, String>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    match &p.kind {
        BmmPropKind::Single(t) => {
            let overridden = type_override(&class.name, &p.name);
            let mut inner = match overridden {
                Some(rust) => rust.to_string(),
                None => model.render_type(t, generics, subst, local, external),
            };
            // Box a field that would make the struct infinitely sized: direct
            // self-recursion, mutual recursion (RESOURCE_DESCRIPTION ↔
            // AUTHORED_RESOURCE), and F-bounded recursion through an auto-filled
            // generic arg (DV_QUANTITY → normal_range: DvInterval<DvOrdered>,
            // and DvOrdered's variants include DV_QUANTITY). We check every spec
            // name the rendered type embeds by value, not just its head.
            // A type already behind an indirection (`Vec`, `BTreeMap`,
            // `BTreeSet`) breaks the cycle on its own — boxing it is redundant.
            let already_indirect =
                inner.starts_with("Vec<") || inner.starts_with("std::collections::");
            let cyclic = overridden.is_none() && !already_indirect && {
                let mut roots = BTreeSet::new();
                model.effective_roots(t, &mut roots);
                roots.iter().any(|r| {
                    !Model::is_mapped(r)
                        && (r == &class.name || model.reaches(r, &class.name, &mut BTreeSet::new()))
                })
            };
            if cyclic {
                inner = format!("Box<{inner}>");
            }
            if p.is_mandatory {
                inner
            } else {
                format!("Option<{inner}>")
            }
        }
        BmmPropKind::Container { item, .. } => {
            // A byte buffer (`Array<Octet>` / `List<Octet>`, e.g.
            // `DV_MULTIMEDIA.data`) is inline base64 *text* on the canonical
            // wire, not a JSON array — carry the base64 verbatim as a `String`
            // (decoding is a behaviour-layer concern), like other broader-than-a-
            // crate openEHR types. Optionality follows the property.
            if item.root_name() == "Octet" {
                return if p.is_mandatory {
                    "String".to_string()
                } else {
                    "Option<String>".to_string()
                };
            }
            format!(
                "Vec<{}>",
                model.render_type(item, generics, subst, local, external)
            )
        }
    }
}

fn emit_enum(
    model: &Model,
    class: &BmmClass,
    variants: &[String],
    self_data: bool,
    index: &BTreeMap<String, Vec<String>>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    let ty = naming::type_name(&class.name);
    // The enum is generic over the abstract class's declared params that any
    // concrete variant uses (`VERSION<T>` exposes `T` only through
    // `ORIGINAL_VERSION.data: T`); `used_generic_params` resolves this uniformly
    // so a bare reference elsewhere renders the same arity.
    let enum_generics: Vec<String> = model
        .used_generic_params(&class.name)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    let gen_decl = if enum_generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", enum_generics.join(", "))
    };
    let mut b = String::new();
    let no_subst = BTreeMap::new();

    // Compute payloads first (so imports can be derived from what they touch).
    let payloads: Vec<(String, String)> = variants
        .iter()
        .map(|d| {
            let variant = naming::type_name(d);
            let d_generic = !model.used_generic_params(d).is_empty();
            let payload = if d_generic && !enum_generics.is_empty() {
                // Same subtype family: thread the enum's own params (`Event<T>`
                // → `PointEvent(PointEvent<T>)`).
                format!("{variant}<{}>", enum_generics.join(", "))
            } else {
                // Non-generic enum (e.g. `DataValue`) with a generic variant:
                // bound-fill the variant (`DvInterval(DvInterval<DvOrdered>)`).
                model.render_type(
                    &BmmType::Simple(d.clone()),
                    &enum_generics,
                    &no_subst,
                    local,
                    external,
                )
            };
            // Box a variant that would make the enum infinitely sized: either
            // the payload embeds the enum type by value via a bound-filled arg
            // (`EL_TERMINAL` ⊇ `EL_CASE_TABLE<EL_TERMINAL>`), or the variant's
            // own fields reach back to the enum (`BMM_TYPE` ⊇ `BMM_CONTAINER_TYPE`
            // whose `base_type` is a `BMM_TYPE`). A `Vec`/map payload already
            // breaks the cycle.
            let already_indirect =
                payload.starts_with("Vec<") || payload.starts_with("std::collections::");
            let cyclic = !already_indirect && {
                let mut roots = BTreeSet::new();
                model.effective_roots(&BmmType::Simple(d.clone()), &mut roots);
                roots.contains(&class.name) || model.reaches(d, &class.name, &mut BTreeSet::new())
            };
            let payload = if cyclic {
                format!("Box<{payload}>")
            } else {
                payload
            };
            (variant, payload)
        })
        .collect();

    // A polymorphic *concrete* class also carries its own instances: append a
    // `{Name}({Name}Data)` variant last (least-rich, so richer subtypes match
    // first on the untagged wire), and emit the `{Name}Data` struct in-file.
    let data_ty = format!("{ty}Data");
    let data_generics = struct_generics(model, class);
    let data_subst = class_binding(&class.name);
    let mut payloads = payloads;
    if self_data {
        let data_payload = if data_generics.is_empty() {
            data_ty.clone()
        } else {
            format!("{data_ty}<{}>", data_generics.join(", "))
        };
        payloads.push((ty.clone(), data_payload));
    }

    // Imports: every emittable spec type each payload embeds. For a variant
    // threaded over the enum's own params (`IntervalEvent<T>`) that is just the
    // variant type; for a bound-filled variant (`DvInterval<DvOrdered>`) it also
    // includes the auto-filled bound args. Mirror the payload decision so we do
    // not import a bound type the payload never names.
    let mut imports: BTreeSet<String> = BTreeSet::new();
    for d in variants {
        let mut roots = BTreeSet::new();
        let d_generic = !model.used_generic_params(d).is_empty();
        if d_generic && !enum_generics.is_empty() {
            roots.insert(d.clone());
        } else {
            model.effective_roots(&BmmType::Simple(d.clone()), &mut roots);
        }
        for r in roots {
            add_import(&mut imports, &r, &ty, index, external);
        }
    }
    // The in-file `{Name}Data` struct pulls in imports for the class's own fields.
    if self_data {
        imports.extend(
            model
                .referenced_specs(class, &data_generics, &data_subst)
                .iter()
                .filter_map(|spec| {
                    let ident = naming::type_name(spec);
                    if ident == ty {
                        return None;
                    }
                    if let Some(chain) = index.get(&ident) {
                        Some(format!("use crate::{}::{};", chain.join("::"), ident))
                    } else {
                        external
                            .prelude_of(spec)
                            .map(|path| format!("use {path}::{ident};"))
                    }
                }),
        );
    }

    // ── `_type` dispatch (mirrors the XML xsi:type runtime) ─────────────────
    // The ITS-JSON schema requires `_type` on an *abstract* polymorphic slot
    // (`DATA_VALUE`, `UID`, `VERSION`, …) and rejects a `_type`-less value, while
    // a *concrete* polymorphic slot (`DV_TEXT`, holding a plain DV_TEXT or a
    // DV_CODED_TEXT) makes `_type` optional and defaults a `_type`-less value to
    // the base concrete type. We emit a hand-rolled `Deserialize` that dispatches
    // on `_type` (deep descendants routed to their direct variant, which
    // recurses) instead of `#[serde(untagged)]`, whose structural guessing
    // silently mis-types a `_type`-less value. Serialize keeps
    // `#[serde(untagged)]` — its output is byte-identical (variant payload only).
    let dispatch = model.xsi_dispatch(&class.name, variants);
    // `_type` dispatch is valid only when every concrete target actually carries
    // a `_type` on the wire (a Struct or PolyEnum, not a transparent enumeration
    // Newtype); otherwise keep the structural `#[serde(untagged)]` reader.
    let type_dispatch = !dispatch.is_empty()
        && dispatch
            .iter()
            .all(|(spec, _)| model.concrete_carries_type(spec));
    // The variant a `_type`-less value defaults to: `Some` for a concrete
    // polymorphic slot (its own `{Name}Data`), `None` for an abstract slot (a
    // `_type`-less value is rejected, per the schema).
    let self_ident = dispatch
        .iter()
        .find(|(spec, _)| *spec == class.name)
        .map(|(_, id)| id.clone());

    // Header: an untagged enum uses serde derives; a polymorphic-concrete file
    // also emits an `OpenEhrType` struct, so it needs that import too.
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{}`) — DO NOT EDIT.\n",
        class.name
    ));
    if self_data {
        b.push_str("// Hand-written spec functions/invariants live in the sibling `*_impl.rs`.\n");
    }
    b.push('\n');
    // When we hand-roll `Deserialize`, only `Serialize` is derived; `Deserialize`
    // is referenced by full path in the emitted impl, so drop its import.
    let fixed: &[&str] = match (self_data, type_dispatch) {
        (true, true) => &["use serde::Serialize;", "use openehr_derive::OpenEhrType;"],
        (true, false) => &[
            "use serde::{Deserialize, Serialize};",
            "use openehr_derive::OpenEhrType;",
        ],
        (false, true) => &["use serde::Serialize;"],
        (false, false) => &["use serde::{Deserialize, Serialize};"],
    };
    write_uses(&mut b, fixed, &imports);

    // The `{Name}Data` struct (the class's own instances) precedes the enum.
    if self_data {
        b.push_str(&render_struct_def(
            model,
            class,
            &data_ty,
            &data_generics,
            &data_subst,
            local,
            external,
        ));
        b.push('\n');
    }

    doc_block(&mut b, class.documentation.as_deref(), "");
    let slot = if self_data {
        "Polymorphic slot"
    } else {
        "Closed subtype set"
    };
    b.push_str(&format!(
        "/// {slot} of `{}`: a closed subtype set dispatched on each payload's `_type`.\n",
        class.name
    ));
    if type_dispatch {
        b.push_str("#[derive(Debug, Clone, PartialEq, Serialize)]\n");
    } else {
        b.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
    }
    b.push_str("#[serde(untagged)]\n");
    b.push_str(&format!("pub enum {ty}{gen_decl} {{\n"));
    for (variant, payload) in &payloads {
        b.push_str(&format!("    {variant}({payload}),\n"));
    }
    b.push_str("}\n");

    if type_dispatch {
        b.push('\n');
        b.push_str(&emit_type_dispatch_deser(
            &ty,
            &enum_generics,
            &class.name,
            &dispatch,
            self_ident.as_deref(),
        ));
    }
    b
}

/// Emit a hand-rolled `Deserialize` for an abstract/polymorphic enum that
/// dispatches on the canonical-JSON `_type` discriminator instead of
/// `#[serde(untagged)]`'s structural fallback.
///
/// The value is buffered into a `serde_json::Value` (these types are
/// canonical-JSON-only for serde; XML has its own `FromXml` path), its `_type`
/// read, and the whole value re-deserialized into the one matching variant via
/// `serde_json::from_value` — which preserves that variant's precise inner error
/// and re-checks `_type` + unknown keys in the inner `OpenEhrType`
/// reader. A deep descendant (`DV_CODED_TEXT` in a `DATA_VALUE` slot) routes to
/// its direct variant (`DvText`), whose own dispatcher recurses.
///
/// `self_ident` is `Some(variant)` for a concrete polymorphic slot — a
/// `_type`-less value defaults to the base concrete type, matching the schema's
/// `if not required _type then <base>` construction — and `None` for an abstract
/// slot, where a `_type`-less value is rejected (schema `required: [_type]`).
fn emit_type_dispatch_deser(
    ty: &str,
    generics: &[String],
    spec_name: &str,
    dispatch: &[(String, String)],
    self_ident: Option<&str>,
) -> String {
    let expected = dispatch
        .iter()
        .map(|(s, _)| s.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let (impl_hdr, ty_ref, where_cl) = if generics.is_empty() {
        ("impl<'de>".to_string(), ty.to_string(), String::new())
    } else {
        let ps = generics.join(", ");
        // `from_value` deserializes an owned `Value`, so each parameter must be
        // `DeserializeOwned` (satisfied at every call site — the RM types are all
        // owned, and canonical JSON is parsed from owned input).
        let wc = generics
            .iter()
            .map(|p| format!("{p}: ::serde::de::DeserializeOwned"))
            .collect::<Vec<_>>()
            .join(", ");
        (
            format!("impl<'de, {ps}>"),
            format!("{ty}<{ps}>"),
            format!("\nwhere\n{wc},"),
        )
    };

    let mut b = String::new();
    b.push_str(&format!(
        "{impl_hdr} ::serde::Deserialize<'de> for {ty_ref}{where_cl} {{\n"
    ));
    // `too_many_lines`: enums with many concrete descendants generate a long
    // match. `match_same_arms`: several `_type`s can route to one direct variant
    // (deep descendants collapse), yielding intentionally-identical arms.
    b.push_str("    #[allow(clippy::too_many_lines, clippy::match_same_arms)]\n");
    b.push_str(
        "    fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>\n",
    );
    b.push_str("    where\n        D: ::serde::Deserializer<'de>,\n    {\n");
    b.push_str(
        "        let __value = <::serde_json::Value as ::serde::Deserialize>::deserialize(deserializer)?;\n",
    );
    b.push_str("        match __value.get(\"_type\").and_then(::serde_json::Value::as_str) {\n");
    for (spec, ident) in dispatch {
        b.push_str(&format!(
            "            ::core::option::Option::Some({spec:?}) => ::core::result::Result::Ok(\n                Self::{ident}(::serde_json::from_value(__value).map_err(::serde::de::Error::custom)?),\n            ),\n"
        ));
    }
    if let Some(ident) = self_ident {
        b.push_str(&format!(
            "            ::core::option::Option::None => ::core::result::Result::Ok(\n                Self::{ident}(::serde_json::from_value(__value).map_err(::serde::de::Error::custom)?),\n            ),\n"
        ));
    } else {
        let msg = format!(
            "{spec_name}: missing required `_type` on polymorphic slot (expected one of: {expected})"
        );
        b.push_str(&format!(
            "            ::core::option::Option::None => ::core::result::Result::Err(::serde::de::Error::custom(\n                {msg:?},\n            )),\n"
        ));
    }
    // Inline the binding (`{__other:?}`) so the generated `format!` is
    // clippy-clean (`uninlined_format_args`).
    let fmt =
        format!("{spec_name}: unexpected `_type` {{__other:?}} (expected one of: {expected})");
    b.push_str(&format!(
        "            ::core::option::Option::Some(__other) => ::core::result::Result::Err(::serde::de::Error::custom(\n                ::std::format!({fmt:?}),\n            )),\n"
    ));
    b.push_str("        }\n    }\n}\n");
    b
}

fn emit_newtype(class: &BmmClass, prim: &str) -> String {
    let ty = naming::type_name(&class.name);
    let mut b = String::new();
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{}`) — DO NOT EDIT.\n\n\
         use serde::{{Deserialize, Serialize}};\n\n",
        class.name
    ));
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str("#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n");
    b.push_str("#[serde(transparent)]\n");
    b.push_str(&format!("pub struct {ty}(pub {prim});\n"));
    b
}

// ── import + header helpers ──────────────────────────────────────────────────

/// Precise `use` lines for a struct's referenced spec types: `crate::…` for
/// types emitted in this crate, `<dep>::prelude::…` for dependency types.
fn import_lines(
    model: &Model,
    class: &BmmClass,
    generics: &[String],
    subst: &BTreeMap<String, String>,
    self_ident: &str,
    index: &BTreeMap<String, Vec<String>>,
    external: &External,
) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for spec in model.referenced_specs(class, generics, subst) {
        add_import(&mut imports, &spec, self_ident, index, external);
    }
    imports
}

/// Resolve a referenced spec type to a `use` line: local (`crate::…`) wins,
/// then a dependency prelude; an unresolved type needs no import (it rendered as
/// `serde_json::Value`).
fn add_import(
    imports: &mut BTreeSet<String>,
    spec: &str,
    self_ident: &str,
    index: &BTreeMap<String, Vec<String>>,
    external: &External,
) {
    let ident = naming::type_name(spec);
    if ident == self_ident {
        return;
    }
    if let Some(chain) = index.get(&ident) {
        imports.insert(format!("use crate::{}::{};", chain.join("::"), ident));
    } else if let Some(path) = external.prelude_of(spec) {
        imports.insert(format!("use {path}::{ident};"));
    }
}

fn struct_header(b: &mut String, class: &str, imports: &BTreeSet<String>) {
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{class}`) — DO NOT EDIT.\n\
         // Hand-written spec functions/invariants live in the sibling `*_impl.rs`.\n\n"
    ));
    write_uses(b, &["use openehr_derive::OpenEhrType;"], imports);
}

/// Emit a crate's `use` block as a single lexicographically-sorted list (so the
/// output matches `rustfmt`'s default import ordering — `crate::…` before
/// `openehr_base::…` before `openehr_derive::…`/`serde::…`), followed by a blank
/// line. `fixed` holds always-present uses (the derive / serde); `imports` holds
/// the per-file resolved spec imports.
fn write_uses(b: &mut String, fixed: &[&str], imports: &BTreeSet<String>) {
    let mut all: BTreeSet<String> = imports.clone();
    for f in fixed {
        all.insert((*f).to_string());
    }
    for u in &all {
        b.push_str(u);
        b.push('\n');
    }
    b.push('\n');
}

fn doc_block(b: &mut String, doc: Option<&str>, indent: &str) {
    let Some(doc) = doc else { return };
    // Spec prose carries example blocks (ODIN snippets, `YYYY-MM-DDTHH:MM:SS`
    // date formats) that rustdoc would compile as Rust doctests and choke on.
    // Neutralize both forms it recognizes so the docs render as text, never run:
    //   - a bare ``` fence → tag the opening as ```text (closing stays bare);
    //   - a run of 4-space-indented lines → wrap it in a ```text fence.
    let mut push = |line: &str| {
        if line.is_empty() {
            b.push_str(&format!("{indent}///\n"));
        } else {
            b.push_str(&format!("{indent}/// {line}\n"));
        }
    };
    let mut in_fence = false; // inside an explicit ``` fence
    let mut in_indent = false; // inside an auto-wrapped indented block
    for line in doc.lines() {
        let line = line.trim_end();
        let stripped = line.trim_start();
        let lead = line.len() - stripped.len();

        if stripped.starts_with("```") && !in_indent {
            if in_fence {
                in_fence = false;
                push(line);
            } else {
                in_fence = true;
                push(&if stripped == "```" {
                    line.replacen("```", "```text", 1)
                } else {
                    line.to_string()
                });
            }
            continue;
        }
        if in_fence {
            push(line);
            continue;
        }

        let is_indent_line = lead >= 4 && !stripped.is_empty();
        if is_indent_line && !in_indent {
            push("```text");
            in_indent = true;
        } else if in_indent && !is_indent_line && !stripped.is_empty() {
            push("```");
            in_indent = false;
        }
        push(line);
    }
    if in_indent {
        push("```");
    }
    if in_fence {
        push("```");
    }
}

/// `DV_QUANTITY` → `dv_quantity`, `Iso8601_date` → `iso8601_date`.
fn to_snake(spec: &str) -> String {
    spec.to_lowercase()
}

// ── XML codegen support ─────────────────────────────────────────────
// A thin, semantic view of the generated types for the XML emitter (`emit_xml`).
// The XML wire *shape* (element order, attribute-vs-element, xsi:type) comes from
// the XSD reader; this supplies the matching Rust facts (field idents, Option/Vec,
// enum variants, generics) so the generated impls compile against the emitted
// structs. Boxing is transparent to `.write_xml()`, so it is deliberately ignored.

/// One field of an instantiable type. The XML element/attribute name is the
/// openEHR property name (`wire_name`); the Rust accessor is `rust_name`.
/// `target` is the spec type of the value (item type for containers), passed as
/// the declared type so a polymorphic value emits `xsi:type`.
pub(crate) struct XmlField {
    pub wire_name: String,
    pub rust_name: String,
    pub optional: bool,
    pub multiple: bool,
    pub target: String,
    /// For a `Hash<String, V>` field (`target == "Hash"`), the value type's spec
    /// name (`V`); `None` otherwise. `Some("String")` is serialized inline as the
    /// openEHR `StringDictionaryItem` shape.
    pub map_value: Option<String>,
    /// A mandatory field archie omits at its default (the `Interval` inclusivity/
    /// boundedness flags): the Rust default expression (`true`/`false`) to use on
    /// deserialization when the element is absent. `None` = genuinely required.
    pub default: Option<String>,
}

/// One variant of an untagged enum, for the forwarding `ToXml`/`FromXml` impl.
#[allow(dead_code)] // `spec` consumed by the FromXml pass (landing next)
pub(crate) struct XmlVariant {
    /// Rust variant identifier (`DvCodedText`, or the enum's own name for the
    /// polymorphic-concrete self-data variant).
    pub ident: String,
    /// The concrete spec type this variant carries (`DV_CODED_TEXT`), i.e. its
    /// `xsi:type` value on the wire.
    pub spec: String,
}

/// An instantiable type needing a `ToXml`/`FromXml` impl.
// `spec` is consumed by the `FromXml` pass (xsi:type → variant dispatch), landing
// next; keep it now so the type is stable across both directions.
#[allow(dead_code)]
pub(crate) enum XmlType {
    /// A struct: a plain `Struct` class, or a `PolyEnum`'s `{Name}Data`.
    Struct {
        spec: String,
        rust: String,
        generics: Vec<String>,
        fields: Vec<XmlField>,
    },
    /// An untagged enum (abstract slot or polymorphic-concrete) — forwards to
    /// the active variant's payload.
    Enum {
        spec: String,
        rust: String,
        generics: Vec<String>,
        variants: Vec<XmlVariant>,
        /// xsi:type deserialization map: every concrete descendant spec (and the
        /// enum's own spec, if concrete) → the direct variant ident it routes
        /// into. A deep type (`DV_CODED_TEXT` in a `DATA_VALUE` slot) routes into
        /// the intermediate variant (`DvText`), which recurses.
        dispatch: Vec<(String, String)>,
    },
    /// A transparent newtype over a primitive (`VALIDITY_KIND(String)`) — writes
    /// its inner value as element text.
    Newtype { spec: String, rust: String },
}

impl Model {
    /// The flattened fields of a concrete class for XML emission (same order and
    /// flattening as struct emission).
    #[must_use]
    pub(crate) fn xml_fields(&self, class_name: &str) -> Vec<XmlField> {
        let Some(class) = self.get(class_name) else {
            return Vec::new();
        };
        self.flattened_props(class)
            .iter()
            .map(|rp| {
                let p = rp.prop;
                let octet = matches!(&p.kind,
                    BmmPropKind::Container { item, .. } if item.root_name() == "Octet");
                let (multiple, target) = match &p.kind {
                    BmmPropKind::Single(t) => (false, t.root_name().to_string()),
                    BmmPropKind::Container { item, .. } => (!octet, item.root_name().to_string()),
                };
                // The value type of a `Hash<K, V>` field (second generic arg).
                let map_value = match &p.kind {
                    BmmPropKind::Single(BmmType::Generic { root, params }) if root == "Hash" => {
                        params.get(1).map(|v| v.root_name().to_string())
                    }
                    _ => None,
                };
                XmlField {
                    wire_name: p.name.clone(),
                    rust_name: naming::field_ident(&p.name),
                    optional: !p.is_mandatory && !multiple,
                    multiple,
                    target,
                    map_value,
                    default: field_default(&rp.owner, &p.name).map(str::to_string),
                }
            })
            .collect()
    }

    /// The xsi:type deserialization map for an enum: every concrete descendant
    /// spec (and the enum's own spec, if concrete) → the direct variant ident it
    /// routes into. `direct` is the enum's immediate variant specs. A deep type
    /// routes into its intermediate direct variant, which recurses.
    fn xsi_dispatch(&self, enum_spec: &str, direct: &[String]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (name, class) in &self.classes {
            if class.is_abstract || Self::is_mapped(name) {
                continue;
            }
            if name != enum_spec && !self.inherits(name, enum_spec) {
                continue;
            }
            let ident = if name == enum_spec {
                naming::type_name(enum_spec) // polymorphic-concrete self-data variant
            } else if let Some(v) = direct
                .iter()
                .find(|v| v.as_str() == name || self.inherits(name, v))
            {
                naming::type_name(v)
            } else {
                continue;
            };
            out.push((name.clone(), ident));
        }
        out
    }

    /// Does a *concrete* class carry a `_type` discriminator on the
    /// canonical-JSON wire? A `Struct` or `PolyEnum` does (it derives
    /// `OpenEhrType`, which emits `_type` first); a transparent enumeration
    /// `Newtype` (`VALIDITY_KIND` → a bare JSON string) does not. Used to decide
    /// whether an enum's variants can be dispatched on `_type`:
    /// `_type` dispatch is only valid when every concrete target carries one.
    fn concrete_carries_type(&self, name: &str) -> bool {
        let Some(class) = self.get(name) else {
            return false;
        };
        if !self.enum_variants(name).is_empty() {
            return true; // PolyEnum — the `{Name}` enum + `{Name}Data` both tag.
        }
        // Mirror `decide`'s concrete newtype rule: a 0-field concrete leaf whose
        // sole ancestor is a primitive is a transparent newtype (no `_type`).
        let flattened = self.flattened_props(class);
        !(flattened.is_empty()
            && class.ancestors.len() == 1
            && primitive(&class.ancestors[0]).is_some())
    }

    /// Generic parameter names a type exposes (`Version<T>` → `["T"]`).
    #[must_use]
    pub(crate) fn xml_generics(&self, class_name: &str) -> Vec<String> {
        self.used_generic_params(class_name)
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    }

    /// The instantiable XML types of a schema, in class order.
    #[must_use]
    pub(crate) fn xml_types(&self, schema: &BmmSchema) -> Vec<XmlType> {
        let used = self.used_as_type();
        let mut out = Vec::new();
        for (name, class) in &schema.classes {
            let generics = self.xml_generics(name);
            let rust = naming::type_name(name);
            match decide(self, class, &used) {
                Emission::Struct => out.push(XmlType::Struct {
                    spec: name.clone(),
                    rust,
                    generics,
                    fields: self.xml_fields(name),
                }),
                Emission::PolyEnum(variants) => {
                    out.push(XmlType::Struct {
                        spec: name.clone(),
                        rust: format!("{rust}Data"),
                        generics: generics.clone(),
                        fields: self.xml_fields(name),
                    });
                    let mut vs: Vec<XmlVariant> = variants
                        .iter()
                        .map(|v| XmlVariant {
                            ident: naming::type_name(v),
                            spec: v.clone(),
                        })
                        .collect();
                    // The polymorphic-concrete self-data variant is emitted last,
                    // its identifier is the enum's own name (`DvText(DvTextData)`).
                    vs.push(XmlVariant {
                        ident: rust.clone(),
                        spec: name.clone(),
                    });
                    let dispatch = self.xsi_dispatch(name, &variants);
                    out.push(XmlType::Enum {
                        spec: name.clone(),
                        rust,
                        generics,
                        variants: vs,
                        dispatch,
                    });
                }
                Emission::Enum(variants) => {
                    let dispatch = self.xsi_dispatch(name, &variants);
                    out.push(XmlType::Enum {
                        spec: name.clone(),
                        rust,
                        generics,
                        variants: variants
                            .iter()
                            .map(|v| XmlVariant {
                                ident: naming::type_name(v),
                                spec: v.clone(),
                            })
                            .collect(),
                        dispatch,
                    });
                }
                Emission::Newtype(_) => out.push(XmlType::Newtype {
                    spec: name.clone(),
                    rust,
                }),
                Emission::Skip => {}
            }
        }
        out
    }
}
