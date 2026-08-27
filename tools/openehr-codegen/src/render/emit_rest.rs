// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! REST emitter: OAS → the Rust *contract* for one API group, into
//! `openehr-its/src/rest/generated/`.
//!
//! Spec-first: the vendored `-codegen` OAS is the source of truth. For each API
//! group this emits the transport DTOs (the non-RM component schemas), a param
//! struct per operation, an `#[async_trait]` server trait (one typed method per
//! operation), and a route table `(method, path, operationId)`. RM payload
//! schemas resolve to the generated `openehr_rm`/`openehr_base` crates rather
//! than being re-emitted. `ferroehr-rest` implements the trait and wires axum
//! (the handler logic is hand-written application code, not generatable).

#![expect(
    clippy::disallowed_types,
    reason = "dev tooling over JSON artifacts (vendored BMM/OAS bundles, emitter reports) — not the \
              application (#1694)"
)]
use crate::load::oas::{Oas, Operation};
use crate::plan::overrides::oas_monomorphization;
use crate::render::naming;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// Map non-`[A-Za-z0-9_]` chars to `_` (a valid-ident base).
fn clean(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// A proper `snake_case` Rust field ident for an OAS property/param name, then
/// keyword-escaped. Handles camelCase (`minOp` → `min_op`), acronym runs
/// (`SNOMED-CT` → `snomed_ct`), and separators (`Content-Type` → `content_type`,
/// `view:pass_through` → `view_pass_through`). A leading `_` (metadata keys like
/// `_type`) is preserved. Pair with a `#[serde(rename)]` to keep the wire name.
fn field_id(raw: &str) -> String {
    let leading_us = raw.starts_with('_');
    let mut out = String::new();
    let mut prev_alnum_lower = false;
    for c in raw.chars() {
        if c.is_ascii_uppercase() {
            if prev_alnum_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_alnum_lower = false;
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_alnum_lower = true;
        } else {
            if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }
            prev_alnum_lower = false;
        }
    }
    let mut s = out.trim_matches('_').to_string();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    if leading_us {
        s.insert(0, '_');
    }
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    naming::field_ident(&s)
}

/// A Rust type ident for an OAS operation name (`operationId`s are snake, so
/// `type_name` gives `PascalCase`). Non-ident chars (`adl1.4`) map to `_` first.
fn type_id(raw: &str) -> String {
    naming::type_name(&clean(raw))
}

/// The Rust type name for an OAS DTO schema key. Keys are authored in
/// `PascalCase` (`ResultSet`, `OperationalTemplateV2`) and kept verbatim; an
/// all-uppercase acronym (`AQL`, `SNOMEDCT`) is title-cased (`Aql`, `Snomedct`)
/// so it is idiomatic Rust.
fn dto_type(raw: &str) -> String {
    let c = clean(raw);
    let alpha: String = c.chars().filter(char::is_ascii_alphabetic).collect();
    if !alpha.is_empty() && alpha.chars().all(|ch| ch.is_ascii_uppercase()) {
        naming::type_name(&c)
    } else {
        c
    }
}

/// The serde attribute every `Option` field of a generated REST DTO or param
/// struct carries.
///
/// An OAS property that is not in the schema's `required` list and does not set
/// `nullable: true` admits its declared type and nothing else — `null` is not a
/// member of `type: string` (or of a `$ref` to a string alias) under OpenAPI
/// 3.0 (<https://spec.openapis.org/oas/v3.0.3#schema-object>: "nullable …
/// Default value is false", and the Schema Object is a JSON Schema subset). No
/// component schema in the vendored ITS-REST bundles
/// (`crates/openehr-its/vendor/rest-oas/`) sets `nullable`, so an absent
/// optional property is **omitted** on the wire, never serialized as `null`.
const SKIP_NONE_ATTR: &str = "    #[serde(skip_serializing_if = \"Option::is_none\")]\n";

/// The generated field name for an OAS `additionalProperties` extension map.
const ADDITIONAL_PROPERTIES_FIELD: &str = "additional_properties";

/// Emit the flattened extension map for a DTO whose OAS schema declares
/// `additionalProperties` (the designated extension point), or nothing when it
/// declares `additionalProperties: false`/omits the keyword.
///
/// `additionalProperties: true` carries arbitrary JSON values; an
/// `additionalProperties: <schema>` form carries that schema's Rust type. A
/// `BTreeMap` keeps the emitted order deterministic, and `#[serde(flatten)]`
/// puts the entries at the object's own level — which is what "additional
/// properties" means in JSON Schema — while collecting every undeclared key on
/// the way in. An empty map serializes to nothing, so a DTO with no extensions
/// is byte-identical to one emitted before this field existed.
fn emit_additional_properties(b: &mut String, name: &str, schema: &Value, ctx: &Ctx) {
    let value_ty = match schema.get("additionalProperties") {
        Some(Value::Bool(true)) => "serde_json::Value".to_string(),
        Some(v @ Value::Object(_)) => ctx.rust_type(v),
        // `false`, a non-schema value, or the keyword's absence: the object is
        // closed and gets no extension slot.
        _ => return,
    };
    let _ = write!(
        b,
        "    /// The undeclared (`additionalProperties`) members of `{name}`, which\n\
         \x20   /// its ITS-REST OAS component schema declares as an extension point.\n\
         \x20   #[serde(flatten)]\n\
         \x20   pub {ADDITIONAL_PROPERTIES_FIELD}: std::collections::BTreeMap<String, {value_ty}>,\n"
    );
}

/// Types emitted by the RM and BASE crates — `PascalCase` Rust name → the full
/// generation-module type path an OAS `$ref` resolves to (never a prelude).
pub(crate) struct RmNames {
    pub rm: BTreeMap<String, String>,
    pub base: BTreeMap<String, String>,
}

/// Component schemas the OAS declares GENERIC through a discriminator-typed
/// field, per the SM's own class definition — the one live case is
/// `UPDATE_VERSION<T>` (SM `update_version.adoc`: "An object representing an
/// update to an existing `VERSION` … The back-end will construct a full
/// `VERSION<T>` object"), whose OAS rendering flattens `T` into the per-group
/// `data: Versionable` ref. The hoisted shared module emits the struct with
/// the real generic parameter; each group aliases it at its own `Versionable`.
const GENERIC_OVER: &[(&str, &str)] = &[("UpdateVersion", "data")];

/// The `$ref` names a schema reaches, skipping a genericized field's subtree.
fn ref_names_of(name: &str, schema: &Value, out: &mut BTreeSet<String>) {
    let skip = GENERIC_OVER
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, f)| *f);
    walk_refs(schema, skip, out);
}

/// Walks a schema value collecting every `$ref` target name.
///
/// `skip_field` is dropped only at the `properties` level, so the marker is
/// passed exactly one level below `properties` and nowhere else.
fn walk_refs(v: &Value, skip_field: Option<&str>, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(m) => {
            if let Some(r) = m.get("$ref").and_then(Value::as_str)
                && let Some(n) = r.rsplit('/').next()
            {
                out.insert(n.to_string());
            }
            for (k, val) in m {
                if skip_field == Some(k.as_str()) {
                    continue;
                }
                match val {
                    Value::Object(props) if k == "properties" => {
                        walk_property_refs(props, skip_field, out);
                    }
                    _ => walk_refs(val, None, out),
                }
            }
        }
        Value::Array(a) => {
            for item in a {
                walk_refs(item, skip_field, out);
            }
        }
        _ => {}
    }
}

/// Walks a `properties` map, skipping the genericized field's subtree.
fn walk_property_refs(
    props: &serde_json::Map<String, Value>,
    skip_field: Option<&str>,
    out: &mut BTreeSet<String>,
) {
    for (pk, pv) in props {
        if skip_field != Some(pk.as_str()) {
            walk_refs(pv, None, out);
        }
    }
}

/// A canonical (key-sorted) representation for schema-identity comparison.
fn stable_repr(v: &Value) -> String {
    fn sort(v: &Value) -> Value {
        match v {
            Value::Object(m) => Value::Object(
                m.iter()
                    .map(|(k, val)| (k.clone(), sort(val)))
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            Value::Array(a) => Value::Array(a.iter().map(sort).collect()),
            other => other.clone(),
        }
    }
    sort(v).to_string()
}

/// The cross-group HOIST set: component schemas that appear in more than one
/// group bundle with byte-identical definitions (key-order-independent), are
/// not RM/BASE-resolved, and whose transitive `$ref` closure (minus a
/// genericized field) stays inside {RM, BASE, the hoist set} — so the shared
/// module is self-contained. Everything else keeps per-group emission (a
/// schema like `Versionable` is textually shared but semantically per-group:
/// its discriminator mapping differs).
pub(crate) fn hoist_set(bundles: &[(&str, Oas)], names: &RmNames) -> BTreeSet<String> {
    use std::collections::BTreeMap;
    let mut reprs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_, oas) in bundles {
        for (name, schema) in oas.schemas() {
            *counts.entry(name.clone()).or_default() += 1;
            reprs
                .entry(name.clone())
                .or_default()
                .insert(stable_repr(schema));
            let mut r = BTreeSet::new();
            ref_names_of(&name, schema, &mut r);
            refs.entry(name).or_default().extend(r);
        }
    }
    let mut hoisted: BTreeSet<String> = counts
        .iter()
        .filter(|(n, c)| {
            **c > 1
                && reprs.get(*n).is_some_and(|r| r.len() == 1)
                && !names.rm.contains_key(*n)
                && !names.base.contains_key(*n)
                && oas_monomorphization(n).is_none()
        })
        .map(|(n, _)| n.clone())
        .collect();
    // Fixpoint: drop any candidate whose refs leave {RM, BASE, hoisted}.
    loop {
        let snapshot = hoisted.clone();
        hoisted.retain(|n| {
            refs.get(n).is_some_and(|rs| {
                rs.iter().all(|r| {
                    names.rm.contains_key(r)
                        || names.base.contains_key(r)
                        || oas_monomorphization(r).is_some()
                        || snapshot.contains(r)
                })
            })
        });
        if hoisted.len() == snapshot.len() {
            break;
        }
    }
    hoisted
}

/// Emit the shared `common` module: the cross-group hoisted DTOs (byte-identical
/// component schemas whose ref closure is self-contained — [`hoist_set`]), each
/// emitted exactly once. The one genericized schema ([`GENERIC_OVER`]) emits
/// with its real SM generic parameter; the groups alias it at their own type
/// argument.
#[must_use]
pub(crate) fn emit_common(oas: &Oas, names: &RmNames, hoisted: &BTreeSet<String>) -> String {
    let ctx = Ctx {
        oas,
        names,
        dtos: hoisted,
        hoisted,
        in_common: true,
    };
    let mut b = String::new();
    let _ = write!(
        b,
        "// @generated by openehr-codegen (emit-rest) — DO NOT EDIT.\n\
         //! ITS-REST contract, shared component schemas: DTOs that appear in more\n\
         //! than one API group's OAS bundle with identical definitions, hoisted so\n\
         //! one Rust type serves every group (the per-group bundles duplicate\n\
         //! shared schemas verbatim).\n\n\
         #![allow(\n    \
         clippy::all,\n    \
         clippy::pedantic,\n    \
         clippy::nursery,\n    \
         dead_code,\n    \
         unused_variables,\n    \
         reason = \"mechanically generated contract text: the OAS is emitted in \
         full (every DTO, param struct and route, whether or not this workspace \
         consumes it yet), so style and dead-code lints do not apply — the \
         hand-written runtime and the implementing adapter carry the lint bar\"\n\
         )]\n\
         use serde::{{Deserialize, Serialize}};\n\n"
    );
    for (name, schema) in oas.schemas() {
        if hoisted.contains(&name) {
            emit_dto(&mut b, &name, schema, &ctx);
        }
    }
    b
}

/// Emit the generated module for one API group. `dtos` is the set of component
/// schema names this group defines that are *not* RM types (i.e. real DTOs).
#[must_use]
pub(crate) fn emit_group(
    oas: &Oas,
    group: &str,
    names: &RmNames,
    hoisted: &BTreeSet<String>,
) -> String {
    let trait_name = format!("{}Api", naming::type_name(group));
    // Component schemas split into RM-resolved vs local DTOs.
    let dtos: BTreeSet<String> = oas
        .schemas()
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| {
            !names.rm.contains_key(n)
                && !names.base.contains_key(n)
                && oas_monomorphization(n).is_none()
        })
        .collect();
    let ctx = Ctx {
        oas,
        names,
        dtos: &dtos,
        hoisted,
        in_common: false,
    };

    let mut b = String::new();
    let _ = write!(
        b,
        "// @generated by openehr-codegen (emit-rest) — DO NOT EDIT.\n\
         //! ITS-REST contract for the `{group}` API group: DTOs, per-operation\n\
         //! param structs, the `{trait_name}` server trait, and the route table.\n\n\
         #![allow(\n    \
         clippy::all,\n    \
         clippy::pedantic,\n    \
         clippy::nursery,\n    \
         dead_code,\n    \
         unused_variables,\n    \
         reason = \"mechanically generated contract text: the OAS is emitted in \
         full (every DTO, param struct and route, whether or not this workspace \
         consumes it yet), so style and dead-code lints do not apply — the \
         hand-written runtime and the implementing adapter carry the lint bar\"\n\
         )]\n\
         use serde::{{Deserialize, Serialize}};\n\n"
    );

    // ── DTOs ──
    for (name, schema) in oas.schemas() {
        if !ctx.dtos.contains(&name) {
            continue;
        }
        if ctx.hoisted.contains(&name) {
            // Hoisted into `common`; a GENERIC-over schema gets a group-local
            // alias binding the group's own type argument (the flattened
            // `data` ref — e.g. this group's `Versionable`), so group refs
            // keep using the bare name.
            if let Some((_, field)) = GENERIC_OVER.iter().find(|(n, _)| *n == name) {
                let arg = ctx
                    .oas
                    .resolve(schema)
                    .pointer(&format!("/properties/{field}"))
                    .map_or_else(|| "serde_json::Value".to_string(), |s| ctx.rust_type(s));
                let ty = dto_type(&name);
                let _ = writeln!(
                    b,
                    "/// This group's instantiation of the shared generic `{name}` envelope\n\
                     /// (`super::common::{ty}`), bound at this group's own content union.\n\
                     pub type {ty} = super::common::{ty}<{arg}>;\n"
                );
            }
            continue;
        }
        emit_dto(&mut b, &name, schema, &ctx);
    }

    // ── per-operation param structs ──
    let ops = oas.operations();
    for op in &ops {
        emit_params_struct(&mut b, op, &ctx);
    }

    // ── server trait ──
    let _ = write!(
        b,
        "/// Server contract for the `{group}` API group (ITS-REST). Every method\n\
         /// defaults to returning `ApiError::NotImplemented`, so an implementor\n\
         /// (the application service, or a test stub) overrides only the\n\
         /// operations it supports.\n\
         #[async_trait::async_trait]\n\
         pub trait {trait_name} {{\n"
    );
    for op in &ops {
        emit_trait_method(&mut b, op, &ctx);
    }
    b.push_str("}\n\n");

    // ── route table ──
    let _ = write!(
        b,
        "/// The operations of this group as `(method, path, operation_id)`, for\n\
         /// wiring an axum router in `ferroehr-rest`.\n\
         pub const ROUTES: &[(&str, &str, &str)] = &[\n"
    );
    for op in &ops {
        let _ = writeln!(
            b,
            "    (\"{}\", \"{}\", \"{}\"),",
            op.method.to_uppercase(),
            op.path,
            op.operation_id
        );
    }
    b.push_str("];\n");
    b
}

struct Ctx<'a> {
    oas: &'a Oas,
    names: &'a RmNames,
    dtos: &'a BTreeSet<String>,
    /// The cross-group hoisted schema names ([`hoist_set`]).
    hoisted: &'a BTreeSet<String>,
    /// Whether we are emitting the shared `common` module itself (hoisted
    /// names render bare) or a group module (hoisted names render
    /// `super::common::…`, except a [`GENERIC_OVER`] name, which the group
    /// aliases locally).
    in_common: bool,
}

impl Ctx<'_> {
    /// Map a `$ref` schema to its Rust type.
    ///
    /// A schema whose KEY does not match its class — the released bundles
    /// rename `CLUSTER` to `Clstr` and give every generic INSTANTIATION its own
    /// flat key — is resolved through the monomorphization map read from each
    /// schema's own `title` (see `plan::overrides::OAS_MONOMORPHIZATIONS`);
    /// without it these emit as `allOf`-truncated DTOs that drop their
    /// inherited members and the spec type's strict reader.
    ///
    /// RM/BASE spec types resolve to the TYPED spec structs: since the
    /// foundation rewrite (#1702) every spec type carries emitted manual
    /// `serde::Serialize`/`Deserialize` impls (its crate's `json_serde.rs`),
    /// and those impls ARE the strict canonical-JSON reader — so a typed field
    /// is strict by construction where an untyped `Value` silently accepted
    /// anything (#1712). A GENERIC-over hoisted schema has a group-local alias
    /// (bare name); every other hoisted schema lives in `super::common`. A ref
    /// to something emitted nowhere is resolved and mapped structurally.
    fn ref_type(&self, name: &str, schema: &Value) -> String {
        if let Some(spec) = oas_monomorphization(name) {
            return spec.to_string();
        }
        if let Some(path) = self.names.rm.get(name) {
            return path.clone();
        }
        if let Some(path) = self.names.base.get(name) {
            return path.clone();
        }
        if self.hoisted.contains(name) && !self.in_common {
            if GENERIC_OVER.iter().any(|(n, _)| *n == name) {
                return dto_type(name);
            }
            return format!("super::common::{}", dto_type(name));
        }
        if self.dtos.contains(name) {
            return dto_type(name);
        }
        self.rust_type(self.oas.resolve(schema))
    }

    /// Map an OAS schema to a Rust type. RM `$ref`s resolve to the spec crate
    /// preludes; local DTO refs to the bare name; unknown/complex shapes degrade
    /// to `serde_json::Value` (the same honest fallback the BMM emitter uses).
    fn rust_type(&self, schema: &Value) -> String {
        if let Some(name) = Oas::ref_name(schema) {
            return self.ref_type(&name, schema);
        }
        // `allOf` COMPOSITION. A schema whose only structural content is a
        // single-`$ref` `allOf` is a pure alias for its referent — OAS 3.0
        // composes independently-validated definitions
        // (<https://spec.openapis.org/oas/v3.0.3#composition-and-inheritance-polymorphism>),
        // so an empty own-contribution leaves exactly the referent's shape. The
        // released OAS uses it to give one `ITEM_TAG` schema a per-resource name.
        // A schema that ALSO declares `properties` is a genuine extension and
        // reaches the object branch instead.
        if schema.get("properties").is_none()
            && let Some(members) = schema.get("allOf").and_then(Value::as_array)
        {
            match members.as_slice() {
                [only] => return self.rust_type(only),
                // A multi-member `allOf` composes several schemas into one
                // object, which needs a MERGED struct this emitter does not
                // build. The released OAS contains none (every `allOf` in the
                // vendored bundles has exactly one member), so this arm is
                // unreachable today and carries the payload untyped rather
                // than picking one member and silently losing the others.
                _ => return "serde_json::Value".to_string(),
            }
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("string") => "String".to_string(),
            Some("integer") => "i64".to_string(),
            Some("number") => "f64".to_string(),
            Some("boolean") => "bool".to_string(),
            Some("array") => {
                let item = schema.get("items").map_or_else(
                    || "serde_json::Value".to_string(),
                    |i| {
                        if i.as_object().is_some_and(serde_json::Map::is_empty) {
                            "serde_json::Value".to_string()
                        } else {
                            // The item schema goes through `rust_type` VERBATIM
                            // (never pre-resolved): a `$ref` item must keep its
                            // name so `items: {$ref: ResultSetColumn}` emits
                            // `Vec<ResultSetColumn>`. Resolving first discards
                            // the name and the item degrades to an untyped
                            // `serde_json::Value`. The `$ref` arm above still
                            // resolves structurally when the name is not one
                            // this emitter binds.
                            self.rust_type(i)
                        }
                    },
                );
                format!("Vec<{item}>")
            }
            Some("object") => {
                if schema.get("properties").is_some() {
                    // An inline (anonymous) object — not named; carry as JSON.
                    "serde_json::Value".to_string()
                } else {
                    // A free map (`additionalProperties`).
                    "std::collections::BTreeMap<String, serde_json::Value>".to_string()
                }
            }
            // oneOf/anyOf and untyped → free-form JSON (single-`$ref` `allOf`
            // composition is resolved above).
            _ => "serde_json::Value".to_string(),
        }
    }

    /// The `allOf`-flattened object shape of a component schema: the members it
    /// accepts (ancestors first, then its own, in document order) and the union
    /// of every `required` list on the chain.
    ///
    /// OAS 3.0 models inheritance as `allOf` composition — a derived schema is
    /// "validated against all the schemas" it composes plus its own definition,
    /// and `discriminator` "can be used to aid in serialization,
    /// deserialization, and validation" of exactly that construct
    /// (<https://spec.openapis.org/oas/v3.0.3#composition-and-inheritance-polymorphism>).
    /// The released ITS-REST bundles use it for the whole RM subtype chain and
    /// for `UpdateAttestation` extending `UpdateAudit`, so a DTO emitted from
    /// its OWN `properties` alone silently loses every inherited member (an
    /// `UPDATE_ATTESTATION` without `change_type`/`committer`).
    ///
    /// A member redeclared by a descendant (the `_type` enum narrowing) keeps
    /// the ancestor's position and takes the descendant's schema, so the
    /// emitted field order stays deterministic.
    fn merged_object(&self, schema: &Value) -> (Vec<(String, Value)>, BTreeSet<String>) {
        let mut props: Vec<(String, Value)> = Vec::new();
        let mut required: BTreeSet<String> = BTreeSet::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        self.collect_object(schema, &mut props, &mut required, &mut seen);
        (props, required)
    }

    /// Accumulate one link of the [`Ctx::merged_object`] chain: ancestors first
    /// (depth-first through `allOf`), then this schema's own contribution.
    /// `seen` breaks a cyclic or diamond `$ref` chain.
    fn collect_object(
        &self,
        schema: &Value,
        props: &mut Vec<(String, Value)>,
        required: &mut BTreeSet<String>,
        seen: &mut BTreeSet<String>,
    ) {
        if let Some(members) = schema.get("allOf").and_then(Value::as_array) {
            for member in members {
                if let Some(name) = Oas::ref_name(member)
                    && !seen.insert(name)
                {
                    continue;
                }
                self.collect_object(self.oas.resolve(member), props, required, seen);
            }
        }
        if let Some(own) = schema.get("properties").and_then(Value::as_object) {
            for (pname, pschema) in own {
                if let Some(slot) = props.iter_mut().find(|(n, _)| n == pname) {
                    slot.1 = pschema.clone();
                } else {
                    props.push((pname.clone(), pschema.clone()));
                }
            }
        }
        required.extend(
            schema
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
    }

    /// Map an OAS **parameter** schema to a Rust type.
    ///
    /// A parameter is transported as TEXT, not as JSON: its wire form is fixed
    /// by the parameter's `style`/`explode` serialization rules
    /// (<https://spec.openapis.org/oas/v3.0.3#style-values>) — e.g. the
    /// `openehr-item-tag` header is `style: simple, explode: true`, whose
    /// values read `key="flag",value="follow-up"`, not JSON objects. So an
    /// array parameter whose items are a structured schema carries the raw
    /// parameter values (one `String` per occurrence) and the handler decodes
    /// the style-encoded content; only primitive items keep their mapped type,
    /// which is exactly what a text value can be coerced to.
    fn param_rust_type(&self, schema: &Value) -> String {
        if schema.get("type").and_then(Value::as_str) == Some("array") {
            let item = schema.get("items").map(|i| self.rust_type(i));
            let inner = item
                .as_deref()
                .filter(|mapped| matches!(*mapped, "String" | "i64" | "f64" | "bool"))
                .unwrap_or("String");
            return format!("Vec<{inner}>");
        }
        self.rust_type(schema)
    }
}

fn emit_dto(b: &mut String, name: &str, schema: &Value, ctx: &Ctx) {
    let schema = ctx.oas.resolve(schema);
    // A `discriminator.mapping` schema is OAS polymorphism over the canonical
    // `_type` (every released mapping uses `propertyName: _type`): it emits a
    // real enum over the mapping's targets, dispatched by the same strict
    // tag-anywhere machinery the spec crates' own emitted impls use — never an
    // untyped alias (issue #1712; `Versionable` was `pub type … =
    // serde_json::Value`).
    if let Some(mapping) = schema
        .get("discriminator")
        .and_then(|d| d.get("mapping"))
        .and_then(Value::as_object)
    {
        // A base carrying `x-discriminator-value` is INSTANTIABLE in its own
        // right — the extension names the `_type` an instance of the base itself
        // sends — so its members survive: it emits as a `…Data` struct and joins
        // the enum as one more variant. A base WITHOUT it is abstract and stays
        // a pure union over its mapping.
        let base_tag = schema
            .get("x-discriminator-value")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let (props, required) = ctx.merged_object(schema);
        let base = match (base_tag, props.is_empty()) {
            (Some(tag), false) => {
                let data_ty = format!("{}Data", dto_type(name));
                emit_struct(b, name, &data_ty, None, &props, &required, schema, ctx);
                Some((tag, data_ty))
            }
            _ => None,
        };
        emit_discriminator_enum(b, name, mapping, base.as_ref(), ctx);
        return;
    }
    // object with named properties → struct; everything else → an alias.
    if schema.get("type").and_then(Value::as_str) == Some("object")
        && schema.get("properties").is_some()
    {
        let ty_name = dto_type(name);
        let (props, required) = ctx.merged_object(schema);
        // The SM-generic hoisted schema ([`GENERIC_OVER`]) emits with its real
        // type parameter in the shared module; the flattened field types as
        // `T` and each group binds it via a local alias.
        let generic_field = if ctx.in_common {
            GENERIC_OVER
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, f)| *f)
        } else {
            None
        };
        emit_struct(
            b,
            name,
            &ty_name,
            generic_field,
            &props,
            &required,
            schema,
            ctx,
        );
    } else {
        // string/array/map/ref alias.
        let _ = writeln!(
            b,
            "/// The `{name}` ITS-REST OAS component schema (a non-object shape, so\n\
             /// it is an alias rather than a struct).\n\
             pub type {} = {};\n",
            dto_type(name),
            ctx.rust_type(schema)
        );
    }
}

/// Emit one transport-DTO struct: `props`/`required` are the `allOf`-flattened
/// shape ([`Ctx::merged_object`]), `generic_field` names the property carried as
/// the type parameter `T` (the [`GENERIC_OVER`] envelope), and `schema` is the
/// declaring schema, read for its `additionalProperties` policy.
#[expect(
    clippy::too_many_arguments,
    reason = "one emission site each for the schema's identity, its Rust name, the generic binding, the flattened shape and the declaring schema — bundling them into a struct would only rename the same arguments"
)]
fn emit_struct(
    b: &mut String,
    name: &str,
    ty_name: &str,
    generic_field: Option<&str>,
    props: &[(String, Value)],
    all_required: &BTreeSet<String>,
    schema: &Value,
    ctx: &Ctx,
) {
    // The vendored OAS `required` list, minus the docs-text-wins
    // corrections (`plan::overrides::REST_OPTIONAL_OVERRIDES` — the
    // ITS-REST docs text wins every conflict with the released OAS; where
    // it contradicts the OAS shape, the field is emitted optional).
    let required: BTreeSet<&str> = all_required
        .iter()
        .map(String::as_str)
        .filter(|f| crate::plan::overrides::rest_optional_override(ty_name, f).is_none())
        .collect();
    // A schema declaring `additionalProperties: false` is CLOSED by the released
    // OAS, so the DTO must refuse an undeclared member rather than accept what
    // the specification's own computable artifact rejects. serde's
    // `deny_unknown_fields` is the exact realization, and it is mutually
    // exclusive with the flatten extension map by construction.
    let closed = schema.get("additionalProperties") == Some(&Value::Bool(false));
    let (deny_doc, deny_attr) = if closed {
        (
            "///\n\
             /// The OAS declares this schema `additionalProperties: false`, so an\n\
             /// undeclared member is refused rather than silently ignored.\n",
            "#[serde(deny_unknown_fields)]\n",
        )
    } else {
        ("", "")
    };
    let generics = if generic_field.is_some() { "<T>" } else { "" };
    let _ = write!(
        b,
        "/// The `{name}` transport DTO of this API group (an ITS-REST OAS\n\
         /// component schema).\n\
         {deny_doc}#[derive(Debug, Clone, Serialize, Deserialize)]\n\
         {deny_attr}pub struct {ty_name}{generics} {{\n"
    );
    for (pname, pschema) in props {
        emit_struct_field(
            b,
            StructField {
                owner: name,
                ty_name,
                pname,
                pschema,
                is_generic: generic_field == Some(pname.as_str()),
                is_required: required.contains(pname.as_str()),
            },
            ctx,
        );
    }
    emit_additional_properties(b, name, schema, ctx);
    b.push_str("}\n\n");
}

/// One DTO field's emission inputs.
#[derive(Clone, Copy)]
struct StructField<'a> {
    /// The OAS component schema name the field belongs to.
    owner: &'a str,
    /// The Rust type name of the emitted DTO.
    ty_name: &'a str,
    /// The OAS property name.
    pname: &'a str,
    /// The OAS property schema.
    pschema: &'a Value,
    /// Whether the field carries the DTO's generic parameter.
    is_generic: bool,
    /// Whether the field is required after the docs-text-wins corrections.
    is_required: bool,
}

/// Emits one DTO field: its doc line, serde attributes and typed declaration.
fn emit_struct_field(b: &mut String, f: StructField<'_>, ctx: &Ctx) {
    let ident = field_id(f.pname);
    let mut ty = if f.is_generic {
        "T".to_string()
    } else {
        ctx.rust_type(f.pschema)
    };
    if !f.is_required {
        ty = format!("Option<{ty}>");
    }
    // A struct field is a public item `missing_docs` checks; the OAS property
    // name is the honest, deterministic summary.
    let _ = writeln!(b, "    /// The `{}` property of `{}`.", f.pname, f.owner);
    // A docs-text-wins correction carries its citation into the generated code
    // (the OAS lists the field as required; the ITS-REST docs text wins).
    if let Some(ov) = crate::plan::overrides::rest_optional_override(f.ty_name, f.pname) {
        let _ = writeln!(b, "    /// OPTIONAL by the docs text — {}", ov.citation);
        let _ = writeln!(b, "    /// ({})", ov.reason);
    }
    if let Some(rename) = naming::serde_rename(f.pname, &ident) {
        let _ = writeln!(b, "    #[serde(rename = \"{rename}\")]");
    }
    if !f.is_required {
        b.push_str(SKIP_NONE_ATTR);
    }
    let _ = writeln!(b, "    pub {ident}: {ty},");
}

/// Emit an OAS `discriminator.mapping` schema as a `_type`-dispatched enum.
///
/// One variant per mapping entry, in document order; each carries the mapped
/// target's Rust type (an RM/BASE spec type or a local DTO, through
/// [`Ctx::rust_type`]). Serialization delegates to the inner value (whose
/// emitted impl writes its own `_type`); deserialization dispatches on the
/// mapping keys with the shared strict tag-anywhere runtime
/// (`openehr_base::serde_support`) — an unknown `_type` is refused naming the
/// legal set, exactly like the spec crates' own closed-set enums.
///
/// `base` is `Some((tag, data_type))` when the schema is INSTANTIABLE ITSELF
/// (it carries `x-discriminator-value`): the base joins the union as a final
/// variant over its own `…Data` struct, and — because the OAS leaves `_type`
/// out of such a base's `required` list and gives it a `default` — an object
/// with NO discriminator reads as that base variant instead of being refused.
/// For a purely abstract base (no `x-discriminator-value`) a missing `_type` is
/// still an error: nothing could be constructed from it.
fn emit_discriminator_enum(
    b: &mut String,
    name: &str,
    mapping: &serde_json::Map<String, Value>,
    base: Option<&(String, String)>,
    ctx: &Ctx,
) {
    let ty_name = dto_type(name);
    let mut variants: Vec<(String, String, String)> = mapping
        .iter()
        .filter_map(|(tag, target)| {
            let target = target.as_str()?;
            let ref_name = target.rsplit('/').next()?.to_string();
            let rust_ty = ctx.rust_type(&serde_json::json!({ "$ref": target }));
            Some((tag.clone(), ref_name, rust_ty))
        })
        .collect();
    if let Some((tag, data_ty)) = base {
        variants.push((tag.clone(), ty_name.clone(), data_ty.clone()));
    }
    let variants = variants;
    let tag_list = variants
        .iter()
        .map(|(t, _, _)| t.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let _ = write!(
        b,
        "/// The `{name}` ITS-REST OAS component schema: `_type`-discriminated\n\
         /// polymorphism over its OAS `discriminator.mapping` targets.\n\
         #[derive(Debug, Clone)]\npub enum {ty_name} {{\n"
    );
    for (tag, ref_name, rust_ty) in &variants {
        let _ = writeln!(b, "    /// `_type: \"{tag}\"`\n    {ref_name}({rust_ty}),");
    }
    b.push_str("}\n\n");
    // Serialize: delegate to the inner value (its impl writes `_type`).
    let _ = write!(
        b,
        "impl ::serde::Serialize for {ty_name} {{\n    \
         fn serialize<__S: ::serde::Serializer>(&self, __serializer: __S) \
         -> ::core::result::Result<__S::Ok, __S::Error> {{\n        match self {{\n"
    );
    for (_, ref_name, _) in &variants {
        let _ = writeln!(
            b,
            "            Self::{ref_name}(__x) => ::serde::Serialize::serialize(__x, __serializer),"
        );
    }
    b.push_str("        }\n    }\n}\n\n");
    // Deserialize: strict tag-anywhere dispatch on the mapping keys.
    let _ = write!(
        b,
        "impl<'de> ::serde::Deserialize<'de> for {ty_name} {{\n    \
         fn deserialize<__D: ::serde::Deserializer<'de>>(__deserializer: __D) \
         -> ::core::result::Result<Self, __D::Error> {{\n        \
         const __TAGS: &[&str] = &[{tags}];\n        \
         struct __Visitor;\n        \
         impl<'de> ::serde::de::Visitor<'de> for __Visitor {{\n            \
         type Value = {ty_name};\n            \
         fn expecting(&self, __f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{\n                \
         __f.write_str(\"an ITS-REST `{name}` object\")\n            \
         }}\n            \
         fn visit_map<__A: ::serde::de::MapAccess<'de>>(\n                \
         self,\n                mut __map: __A,\n            \
         ) -> ::core::result::Result<Self::Value, __A::Error> {{\n                \
         let (__tag, __buffered) =\n                    \
         ::openehr_base::serde_support::read_slot_tag(&mut __map, __TAGS)?;\n                \
         match __tag {{\n                    \
         Some(::openehr_base::serde_support::TagMatch::Known(__t)) => {{\n                        \
         let __rest = ::openehr_base::serde_support::TaggedRest::new(\n                            \
         Some(__t),\n                            __buffered,\n                            __map,\n                        \
         );\n                        match __t {{\n",
        tags = variants
            .iter()
            .map(|(t, _, _)| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for (tag, ref_name, _) in &variants {
        let _ = write!(
            b,
            "                            \"{tag}\" => ::core::result::Result::Ok({ty_name}::{ref_name}(\n                                \
             ::serde::Deserialize::deserialize(__rest)?,\n                            )),\n"
        );
    }
    // An object with no `_type` at all: the concrete base's own form when the
    // schema declares one, otherwise a refusal (an abstract slot cannot pick a
    // variant without its discriminator).
    let none_arm = base.map_or_else(
        || {
            format!(
                "                        \
                 ::core::result::Result::Err(::openehr_base::serde_support::missing_type(\n                            \
                 \"{name}\",\n                            \"{tag_list}\",\n                        ))\n"
            )
        },
        |(_, _)| {
            format!(
                "                        \
                 let __rest =\n                            \
                 ::openehr_base::serde_support::TaggedRest::new(None, __buffered, __map);\n                        \
                 ::core::result::Result::Ok({ty_name}::{ty_name}(\n                            \
                 ::serde::Deserialize::deserialize(__rest)?,\n                        ))\n"
            )
        },
    );
    let _ = write!(
        b,
        "                            __other => ::core::result::Result::Err(\n                                \
         ::openehr_base::serde_support::unexpected_type(\n                                    \
         \"{name}\",\n                                    __other,\n                                    \
         \"{tag_list}\",\n                                ),\n                            ),\n                        \
         }}\n                    }}\n                    \
         Some(::openehr_base::serde_support::TagMatch::Unknown(__other)) => {{\n                        \
         ::core::result::Result::Err(::openehr_base::serde_support::unexpected_type(\n                            \
         \"{name}\",\n                            &__other,\n                            \"{tag_list}\",\n                        \
         ))\n                    }}\n                    \
         None => {{\n{none_arm}                    \
         }}\n                }}\n            }}\n        }}\n        \
         ::serde::Deserializer::deserialize_map(__deserializer, __Visitor)\n    }}\n}}\n\n"
    );
}

fn param_struct_name(op: &Operation) -> String {
    format!("{}Params", type_id(&op.operation_id))
}

fn emit_params_struct(b: &mut String, op: &Operation, ctx: &Ctx) {
    if op.parameters.is_empty() {
        return;
    }
    let sname = param_struct_name(op);
    let _ = write!(
        b,
        "/// Parameters for `{}` (path/query/header).\n\
         #[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {sname} {{\n",
        op.operation_id
    );
    for p in &op.parameters {
        let ident = field_id(&p.name);
        let mut ty = ctx.param_rust_type(&p.schema);
        if !p.required {
            ty = format!("Option<{ty}>");
        }
        if let Some(rename) = naming::serde_rename(&p.name, &ident) {
            let _ = writeln!(b, "    #[serde(rename = \"{rename}\")]");
        }
        if !p.required {
            b.push_str(SKIP_NONE_ATTR);
        }
        let _ = writeln!(b, "    /// `{}` ({})", p.name, p.location);
        let _ = writeln!(b, "    pub {ident}: {ty},");
    }
    b.push_str("}\n\n");
}

fn emit_trait_method(b: &mut String, op: &Operation, ctx: &Ctx) {
    let method = field_id(&op.operation_id);
    let mut args = String::from("&self");
    if !op.parameters.is_empty() {
        let _ = write!(args, ", params: {}", param_struct_name(op));
    }
    if let Some((schema, required)) = &op.request_body {
        let ty = ctx.rust_type(schema);
        if *required {
            let _ = write!(args, ", body: {ty}");
        } else {
            let _ = write!(args, ", body: Option<{ty}>");
        }
    }
    let ret = op
        .success_body
        .as_ref()
        .map_or_else(|| "()".to_string(), |s| ctx.rust_type(s));
    // Emit a default body returning `NotImplemented` so an implementor overrides
    // only the operations it supports (no per-implementor stub boilerplate).
    let _ = writeln!(
        b,
        "    /// `{} {}`\n    async fn {method}({args}) -> Result<{ret}, crate::rest::runtime::ApiError> {{\n        Err(crate::rest::runtime::ApiError::NotImplemented)\n    }}",
        op.method.to_uppercase(),
        op.path
    );
}
