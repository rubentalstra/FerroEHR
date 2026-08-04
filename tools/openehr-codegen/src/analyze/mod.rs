//! Stage 2 — ANALYZE. Model analysis over the loaded BMM meta-model.
//!
//! Consumes the loaded [`crate::load::bmm`] schemas and produces the semantic
//! facts the later stages need: the merged include-closure ([`Model`]), the
//! polymorphic seams (descendant/variant sets), the ownership graph and the
//! back-reference edges that break its cycles, the constructibility proof, and
//! the cross-schema re-emission closure. These are analysis results computed
//! from the model — the text producers live in [`crate::render`], the shape
//! decisions in [`crate::plan`].

use crate::load::bmm::{BmmClass, BmmPropKind, BmmSchema, BmmType};
use crate::plan::overrides::{
    back_reference, is_mapped_class, primitive, subtype_extension_parents,
};
use crate::plan::{Emission, decide};
use crate::render::naming;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod invariants;

/// A merged BMM model (e.g. BASE + RM) used for ancestor flattening and type
/// resolution across schema boundaries.
pub(crate) struct Model {
    pub(crate) classes: BTreeMap<String, BmmClass>,
    /// Lazy memo of [`nonempty_optional_lists`] (consulted per field).
    nonempty_memo: std::sync::OnceLock<BTreeSet<(String, String)>>,
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
    /// Path to the hand-written container-shape module
    /// (`openehr_base::containers`, or `crate::containers` inside
    /// `openehr-base` itself). `NonEmptyVec` — the emission shape of a `1..*`
    /// container — is named from here.
    containers: String,
}

impl External {
    /// Point the container-shape path at the crate being emitted: within
    /// `openehr-base` the module is `crate::containers`, elsewhere it is
    /// reached through the `openehr_base` dependency.
    pub(crate) fn in_crate(mut self, crate_name: &str) -> Self {
        self.containers = if crate_name == "openehr-base" {
            "crate::containers".to_string()
        } else {
            "openehr_base::containers".to_string()
        };
        self
    }

    /// The path the `NonEmptyVec` container shape is named from.
    pub(crate) fn containers_path(&self) -> &str {
        &self.containers
    }

    /// Register a dependency crate's exported spec names under a prelude path.
    pub(crate) fn with(mut self, specs: BTreeSet<String>, prelude_path: &str) -> Self {
        self.deps.push((specs, prelude_path.to_string()));
        self
    }

    /// The prelude path a dependency exports `spec` from, if any.
    pub(crate) fn prelude_of(&self, spec: &str) -> Option<&str> {
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
    pub(crate) prop: &'a crate::load::bmm::BmmProperty,
}

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
        Model {
            classes,
            nonempty_memo: std::sync::OnceLock::new(),
        }
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

    /// Is `name` mapped to Rust rather than emitted (a [`primitive`] or a
    /// [`crate::plan::overrides::MAPPED_CLASSES`] entry)?
    pub(crate) fn is_mapped(name: &str) -> bool {
        primitive(name).is_some() || is_mapped_class(name)
    }

    /// Does `class` inherit from `target` (transitively)?
    ///
    /// The declared `ancestors` plus the additional edges
    /// [`crate::plan::overrides::SUBTYPE_EXTENSIONS`] declares, so a subtype
    /// seam the vendored BMM under-declares participates in the
    /// descendant/variant computation exactly like a declared ancestor. Property
    /// flattening ([`Model::flattened_props`]) reads `ancestors` directly and is
    /// therefore untouched by an extension edge.
    pub(crate) fn inherits(&self, class: &str, target: &str) -> bool {
        let Some(c) = self.get(class) else {
            return false;
        };
        for a in c.ancestors.iter().map(String::as_str) {
            if a == target || self.inherits(a, target) {
                return true;
            }
        }
        // The declared edges are walked first (the common case); the extension
        // edges are a separate loop because their `&'static str` items would
        // otherwise force the borrow of `self` above to `'static`.
        for a in subtype_extension_parents(class) {
            if a == target || self.inherits(a, target) {
                return true;
            }
        }
        false
    }

    /// Can `from` transitively reach `target` through **`Single`** (non-`Vec`)
    /// field types? Used to detect struct-sizing cycles that need boxing
    /// (`Vec`/`Box` already break a cycle; `Option<T>` and plain `T` do not).
    pub(crate) fn reaches(&self, from: &str, target: &str, seen: &mut BTreeSet<String>) -> bool {
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
    pub(crate) fn enum_variants(&self, name: &str) -> Vec<String> {
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

    /// The class-name roots this class references through its own (flattened)
    /// **field** types — i.e. what its emitted struct/enum *contains* (not the
    /// subtype payloads of a polymorphic parent). Used to grow the cross-schema
    /// re-emit closure ([`cross_schema_reemit`]).
    fn field_roots(&self, name: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        if let Some(class) = self.get(name) {
            for rp in self.flattened_props(class) {
                match &rp.prop.kind {
                    BmmPropKind::Single(t) => collect_roots(t, &mut out),
                    BmmPropKind::Container { item, .. } => collect_roots(item, &mut out),
                }
            }
        }
        out
    }

    /// Class names used anywhere as a property type — enum-slot candidates.
    pub(crate) fn used_as_type(&self) -> BTreeSet<String> {
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
    // TODO(#1452): `render_type` (and the type-resolution cluster it anchors —
    // `effective_roots`, `referenced_specs`, `generic_param_bounds`,
    // `scope_content_types`) produces a Rust type *string*, which is a stage-4
    // RENDER concern; it is kept here in ANALYZE because it is fused with
    // Model's type-graph analysis (cycle detection + import edges) and splitting
    // the single `impl Model` across stages risked behaviour change. Move the
    // pure string production to `crate::render` once the graph facts it computes
    // are separated from the text it emits.

    /// Render a BMM type to Rust. `local` is the set of spec class names emitted
    /// in the current crate; `external` maps types provided by dependency
    /// crates. `subst` binds a generic parameter name to a concrete spec type
    /// (an ancestor-generic binding the BMM drops, e.g. `Multiplicity_interval`'s
    /// `T` → `Integer`). A referenced class in neither `local` nor `external`
    /// (or a malformed container) degrades to `serde_json::Value`. Local and
    /// external class types render as the bare ident (the import machinery adds
    /// the right `use`).
    pub(crate) fn render_type(
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
                // Arity is matched structurally (slice patterns), never by index.
                match (root.as_str(), ps.as_slice()) {
                    ("Hash", [key, value]) => {
                        format!("std::collections::BTreeMap<{key}, {value}>")
                    }
                    ("List" | "Array", [item]) => format!("Vec<{item}>"),
                    ("Set", [item]) => format!("std::collections::BTreeSet<{item}>"),
                    // A container of the wrong arity (e.g. the deeply-nested
                    // free-form `Hash` in RESOURCE_ANNOTATIONS) or a type neither
                    // emitted here nor by a dependency → free-form JSON.
                    ("Hash" | "List" | "Array" | "Set", _) => "serde_json::Value".to_string(),
                    (r, _) if !local.contains(r) && !external.contains(r) => {
                        "serde_json::Value".to_string()
                    }
                    // Respect the class's *effective* arity: a class whose only
                    // param was unused is monomorphized (emitted non-generic), so
                    // a reference must drop the explicit args (`REFERENCE_RANGE<X>`
                    // → `ReferenceRange`).
                    (r, _) => match self.generic_param_bounds(r) {
                        None => naming::type_name(r),
                        Some(_) => format!("{}<{}>", naming::type_name(r), ps.join(", ")),
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
    pub(crate) fn used_generic_params(&self, name: &str) -> Vec<(String, Option<String>)> {
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
    pub(crate) fn effective_roots(&self, t: &BmmType, out: &mut BTreeSet<String>) {
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
    pub(crate) fn referenced_specs(
        &self,
        class: &BmmClass,
        generics: &[String],
        subst: &BTreeMap<String, String>,
    ) -> BTreeSet<String> {
        let mut roots = BTreeSet::new();
        for rp in self.flattened_props(class) {
            // A back-reference field is omitted from the emitted struct
            // (see `back_reference` / `render_struct_def`), so it contributes no
            // import — including its target here would emit an unused `use`.
            if back_reference(&rp.owner, &rp.prop.name).is_some() {
                continue;
            }
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

    /// The set of class names that are **constructible** — a finite value
    /// exists. Computed as a least fixpoint: a mapped/primitive type is
    /// constructible; an abstract (untagged-enum) class is constructible if any
    /// variant is; a concrete class is constructible iff every *mandatory
    /// single-valued* field's type is constructible. Container (`Vec`/map) and
    /// optional (`Option`) fields never block construction (they can be
    /// empty/`None`), and a designated owner/parent [`back_reference`] is omitted
    /// from the struct entirely, so it never blocks either.
    fn constructible_classes(&self) -> BTreeSet<String> {
        let mut ok: BTreeSet<String> = BTreeSet::new();
        loop {
            let mut changed = false;
            for (name, class) in &self.classes {
                if ok.contains(name) || Self::is_mapped(name) {
                    continue;
                }
                let constructible = if class.is_abstract {
                    self.enum_variants(name)
                        .iter()
                        .any(|v| ok.contains(v) || Self::is_mapped(v))
                } else {
                    self.flattened_props(class).iter().all(|rp| {
                        // Only a mandatory, single-valued, non-back-reference
                        // field can force construction of another value.
                        if !rp.prop.is_mandatory {
                            return true;
                        }
                        if back_reference(&rp.owner, &rp.prop.name).is_some() {
                            return true;
                        }
                        let BmmPropKind::Single(t) = &rp.prop.kind else {
                            return true;
                        };
                        // A single-valued property whose type is a container
                        // generic (`List`/`Array`/`Set`/`Hash` → `Vec`/`BTreeMap`)
                        // renders as an indirection that can be empty, so it never
                        // blocks construction (mirrors `field_type`'s
                        // `already_indirect`).
                        if let BmmType::Generic { root, .. } = t
                            && matches!(root.as_str(), "List" | "Array" | "Set" | "Hash")
                        {
                            return true;
                        }
                        let mut roots = BTreeSet::new();
                        self.effective_roots(t, &mut roots);
                        // A root blocks construction only if it is a *defined
                        // model class* not yet known constructible. A generic
                        // parameter (`EVENT.data: T`), `Any`, or a cross-schema
                        // type (rendered as `serde_json::Value`) is not a model
                        // class here — it is caller-filled/mapped and never blocks.
                        roots
                            .iter()
                            .all(|r| Self::is_mapped(r) || self.get(r).is_none() || ok.contains(r))
                    })
                };
                if constructible {
                    ok.insert(name.clone());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        ok
    }

    /// The concrete emittable classes of `schema` that are **not**
    /// constructible — an unbroken mandatory single-valued construction cycle
    /// (sorted). Empty is the required state. This is the general ownership-cycle
    /// analysis (orchestrator ruling 2026-07-19): a mandatory single-valued
    /// construction cycle yields a non-constructible infinite-value type in Rust,
    /// so it MUST be broken — and only ever at a designated owner/parent
    /// [`back_reference`] edge, never by relaxing a forward composition. The fix
    /// for any offender is a spec-cited `back_reference` entry naming the
    /// offending back-reference property, never making a real data field
    /// `Option`.
    #[must_use]
    pub(crate) fn constructibility_violations(&self, schema: &BmmSchema) -> Vec<String> {
        let ok = self.constructible_classes();
        let mut bad: Vec<String> = schema
            .classes
            .values()
            .filter(|c| !c.is_abstract && !Self::is_mapped(&c.name))
            .map(|c| c.name.clone())
            .filter(|n| !ok.contains(n))
            .collect();
        bad.sort_unstable();
        bad
    }

    /// Prove every concrete emittable class in `schema` is constructible; panic
    /// otherwise (the loud safeguard on the emit path — see
    /// [`Self::constructibility_violations`] for the pure analysis the invariant
    /// test consumes).
    pub(crate) fn assert_constructible(&self, schema: &BmmSchema) {
        let bad = self.constructibility_violations(schema);
        assert!(
            bad.is_empty(),
            "openehr-codegen: non-constructible type(s) would be emitted — an unbroken \
             mandatory single-valued construction cycle: {bad:?}. Break each cycle at its \
             owner/parent back-reference edge by adding a spec-cited `back_reference` entry; \
             never relax a forward/mandatory data field to `Option`."
        );
    }
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

/// Compute the closure of *upstream* (dependency-schema) classes a downstream
/// schema must **re-emit locally**, because the downstream schema declares
/// subtypes of upstream polymorphic classes across an `includes` boundary.
///
/// This realizes the owner ruling (2026-07-19):
/// cross-component subtype extension (e.g. the AM 2.4 `rules` leaves
/// `EXPR_ARCHETYPE_REF`/`EXPR_CONSTRAINT` extending LANG's `EXPR_VALUE_REF`/
/// `EXPR_LEAF`) is re-opened at the **downstream** crate boundary — an
/// extender-level enum composing the upstream variants + the downstream leaves —
/// while the upstream crate stays byte-identical (dependency arrows point one
/// way, so an upstream enum never gains a downstream variant).
///
/// `model` is the downstream crate's merged include-closure (BASE + LANG + AM);
/// `schema` is the downstream schema (its own classes are "downstream"). A class
/// `C` defined only upstream is re-emitted iff its emitted Rust form **differs**
/// in the downstream view (a least fixpoint): (a) `C` is a polymorphic parent
/// whose concrete-descendant (variant) set gains a downstream class or an
/// already-in-closure class, or (b) `C` **contains**, through one of its own
/// (flattened) field types, a downstream or already-in-closure class. This is
/// deliberately **complete, not minimal** (owner ruling 2026-07-19): code
/// generation exists to emit *every* cross-`includes` extension the vendored
/// BMM implies — e.g. both the beom expression/statement subtree (extended by
/// the AM `rules` leaves) and `AUTHORED_RESOURCE`/`RESOURCE_DESCRIPTION` (the
/// resource metatype `AUTHORED_ARCHETYPE` etc. extend) are re-emitted. Upstream
/// classes whose form is unchanged (they gain no downstream descendant and touch
/// no widened type — e.g. `EXPR_LITERAL`, the whole EL/BMM3 tree) stay external.
/// Returns the empty set when the downstream schema adds no cross-boundary
/// subtypes (e.g. AM 1.4).
#[must_use]
pub(crate) fn cross_schema_reemit(model: &Model, schema: &BmmSchema) -> BTreeSet<String> {
    let downstream: BTreeSet<&str> = schema.classes.keys().map(String::as_str).collect();
    let is_upstream =
        |n: &str| model.get(n).is_some() && !downstream.contains(n) && !Model::is_mapped(n);

    let mut reemit: BTreeSet<String> = BTreeSet::new();
    loop {
        let mut added = false;
        let names: Vec<String> = model.class_iter().map(|(n, _)| n.clone()).collect();
        let is_target = |n: &str, r: &BTreeSet<String>| downstream.contains(n) || r.contains(n);
        for name in names {
            if reemit.contains(&name) || !is_upstream(&name) {
                continue;
            }
            // (a) polymorphic parent whose variant set widens downstream …
            let widens = model
                .enum_variants(&name)
                .iter()
                .any(|v| is_target(v, &reemit));
            // … or (b) container whose own field types touch a widened/downstream
            // type (so the field must resolve to the re-emitted crate-local type).
            let contains = model
                .field_roots(&name)
                .iter()
                .any(|r| is_target(r, &reemit));
            if widens || contains {
                reemit.insert(name);
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    reemit
}

/// Produce an augmented copy of `schema` in which the [`cross_schema_reemit`]
/// closure classes (pulled from the merged `model`) are added to the schema's
/// own class set and grafted into its package tree at the **same package path
/// they occupy in their source schema** (`sources`) — so a re-emitted
/// `EXPRESSION` lands under `<prefix>/beom/core` mirroring LANG, an
/// `AUTHORED_RESOURCE` under its BASE package, etc. — emitting them as
/// crate-local types the downstream fields resolve against.
#[must_use]
pub(crate) fn augment_with_reemit(
    schema: &BmmSchema,
    model: &Model,
    reemit: &BTreeSet<String>,
    sources: &[&BmmSchema],
) -> BmmSchema {
    // Source package path (slash-joined) per class, from the include sources.
    let mut src_path: BTreeMap<String, String> = BTreeMap::new();
    for src in sources {
        for (cls, path) in class_paths(src) {
            src_path.entry(cls).or_insert(path);
        }
    }
    let mut s = schema.clone();
    for name in reemit {
        if let Some(c) = model.get(name) {
            s.classes.insert(name.clone(), c.clone());
        }
        let path = src_path.get(name).cloned().unwrap_or_default();
        let segments: Vec<&str> = path.split('/').filter(|seg| !seg.is_empty()).collect();
        insert_class_into_packages(&mut s.packages, &segments, name);
    }
    s
}

/// Descend/create the nested [`BmmPackage`](crate::load::bmm::BmmPackage) chain named
/// by `segments` and add `class` to the leaf package's class list (idempotent).
/// An empty `segments` drops `class` at the schema root (no package prefix).
fn insert_class_into_packages(
    packages: &mut Vec<crate::load::bmm::BmmPackage>,
    segments: &[&str],
    class: &str,
) {
    let Some((head, rest)) = segments.split_first() else {
        return;
    };
    if !packages.iter().any(|p| p.name.as_str() == *head) {
        packages.push(crate::load::bmm::BmmPackage {
            name: (*head).to_string(),
            classes: Vec::new(),
            packages: Vec::new(),
        });
    }
    // Found or just pushed, so the lookup succeeds; `find` keeps it index-free.
    let Some(pkg) = packages.iter_mut().find(|p| p.name.as_str() == *head) else {
        return;
    };
    if rest.is_empty() {
        if !pkg.classes.iter().any(|c| c == class) {
            pkg.classes.push(class.to_string());
        }
    } else {
        insert_class_into_packages(&mut pkg.packages, rest, class);
    }
}

/// Build a class → nested directory path map from the package tree, e.g.
/// `DV_QUANTITY` → `data_types/quantity`.
pub(crate) fn class_paths(schema: &BmmSchema) -> BTreeMap<String, String> {
    fn walk(p: &crate::load::bmm::BmmPackage, prefix: &str, out: &mut BTreeMap<String, String>) {
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

/// The attribute named by a `x /= Void implies not x.is_empty` assertion, when
/// the expression has exactly that shape (both operands the same attribute).
/// `Void` is matched case-insensitively — the RM BMM spells it both ways
/// (`DV_TEXT.Mappings_valid` uses lowercase `void`).
pub(crate) fn nonempty_list_attribute(expression: &str) -> Option<String> {
    let normalized = expression.split_whitespace().collect::<Vec<_>>().join(" ");
    let (lhs, rhs) = normalized.split_once(" implies ")?;
    let attribute = lhs
        .strip_suffix(" /= Void")
        .or_else(|| lhs.strip_suffix(" /= void"))?;
    if rhs != format!("not {attribute}.is_empty") {
        return None;
    }
    if attribute.is_empty()
        || !attribute
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_')
    {
        return None;
    }
    Some(attribute.to_owned())
}

/// The `(declaring class, attribute)` pairs of every OPTIONAL container
/// attribute carrying a present-implies-non-empty class invariant — emitted
/// `Option<NonEmptyVec<T>>` so present-but-empty is unrepresentable (#1730).
pub(crate) fn nonempty_optional_lists_cached(model: &Model) -> &BTreeSet<(String, String)> {
    model
        .nonempty_memo
        .get_or_init(|| nonempty_optional_lists(model))
}

fn nonempty_optional_lists(model: &Model) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    for (class_name, class) in &model.classes {
        for expression in class.invariants.values() {
            let Some(attribute) = nonempty_list_attribute(expression) else {
                continue;
            };
            let Some(p) = class.properties.iter().find(|p| p.name == attribute) else {
                continue;
            };
            if !p.is_mandatory && matches!(p.kind, BmmPropKind::Container { .. }) {
                out.insert((class_name.clone(), attribute));
            }
        }
    }
    out
}
