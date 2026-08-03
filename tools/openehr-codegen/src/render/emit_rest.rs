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

use crate::load::oas::{Oas, Operation};
use crate::render::naming;
use serde_json::Value;
use std::collections::BTreeSet;
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

/// Names emitted by the RM and BASE crates, to resolve OAS `$ref`s to preludes.
pub(crate) struct RmNames {
    pub rm: BTreeSet<String>,
    pub base: BTreeSet<String>,
}

/// Emit the generated module for one API group. `dtos` is the set of component
/// schema names this group defines that are *not* RM types (i.e. real DTOs).
#[must_use]
pub(crate) fn emit_group(oas: &Oas, group: &str, names: &RmNames) -> String {
    let trait_name = format!("{}Api", naming::type_name(group));
    // Component schemas split into RM-resolved vs local DTOs.
    let dtos: BTreeSet<String> = oas
        .schemas()
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| !names.rm.contains(n) && !names.base.contains(n))
        .collect();
    let ctx = Ctx {
        oas,
        names,
        dtos: &dtos,
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
        if ctx.dtos.contains(&name) {
            emit_dto(&mut b, &name, schema, &ctx);
        }
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
}

impl Ctx<'_> {
    /// Map an OAS schema to a Rust type. RM `$ref`s resolve to the spec crate
    /// preludes; local DTO refs to the bare name; unknown/complex shapes degrade
    /// to `serde_json::Value` (the same honest fallback the BMM emitter uses).
    fn rust_type(&self, schema: &Value) -> String {
        // A `$ref` (possibly to a name we resolve without following it).
        if let Some(name) = Oas::ref_name(schema) {
            // RM/BASE spec types are opaque canonical-JSON payloads at the REST
            // boundary: the application exchanges `serde_json::Value` bodies and
            // validates them via the native codec + `openehr_its::wire_validate`.
            // The spec types carry no serde derive (the codec owns the wire), so
            // the contract DTOs carry any RM/BASE payload as an untyped `Value`
            // rather than the typed spec struct.
            if self.names.rm.contains(&name) || self.names.base.contains(&name) {
                return "serde_json::Value".to_string();
            }
            if self.dtos.contains(&name) {
                return dto_type(&name);
            }
            // A ref to something not emitted anywhere → resolve + map structurally.
            return self.rust_type(self.oas.resolve(schema));
        }
        // `allOf` COMPOSITION. A schema whose only structural content is a
        // single-`$ref` `allOf` — no `properties` of its own — is a pure alias
        // for its referent: OAS 3.0 defines `allOf` as "inline or referenced
        // schema MUST be of a Schema Object and not a standard JSON Schema …
        // allOf takes an array of object definitions that are validated
        // *independently* but together compose a single object"
        // (<https://spec.openapis.org/oas/v3.0.3#composition-and-inheritance-polymorphism>),
        // so an empty own-contribution leaves exactly the referent's shape.
        // The released ITS-REST OAS uses precisely this form to give one
        // `ITEM_TAG` schema a per-resource NAME (`ItemTagOfComposition`,
        // `ItemTagOfEhrStatus`, `ItemTagOfPerson`, …). Dropping the
        // composition degraded all seven to an untyped map.
        //
        // A schema that ALSO declares its own `properties` is a genuine
        // extension of its referent and keeps its own struct (the RM/BASE
        // subtype chain); it never reaches here, because the caller emits it
        // through the object branch.
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
    // object with named properties → struct; everything else → an alias.
    if schema.get("type").and_then(Value::as_str) == Some("object")
        && let Some(props) = schema.get("properties").and_then(Value::as_object)
    {
        let ty_name = dto_type(name);
        // The vendored OAS `required` list, minus the docs-text-wins
        // corrections (`plan::overrides::REST_OPTIONAL_OVERRIDES` — the
        // ITS-REST docs text wins every conflict with the released OAS; where
        // it contradicts the OAS shape, the field is emitted optional).
        let required: BTreeSet<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|f| crate::plan::overrides::rest_optional_override(&ty_name, f).is_none())
            .collect();
        // A schema that declares `additionalProperties: false` is CLOSED by the
        // released OAS, and a closed object must refuse an undeclared member —
        // otherwise the generated DTO silently accepts payloads the
        // specification's own computable artifact rejects. serde's
        // `deny_unknown_fields` is the exact realization
        // (<https://serde.rs/container-attrs.html#deny_unknown_fields>), and it
        // is mutually exclusive with the `#[serde(flatten)]` extension map by
        // construction: that map is emitted only when `additionalProperties` is
        // present and NOT `false`.
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
        let _ = write!(
            b,
            "/// The `{name}` transport DTO of this API group (an ITS-REST OAS\n\
             /// component schema).\n\
             {deny_doc}#[derive(Debug, Clone, Serialize, Deserialize)]\n\
             {deny_attr}pub struct {ty_name} {{\n"
        );
        for (pname, pschema) in props {
            let ident = field_id(pname);
            let mut ty = ctx.rust_type(pschema);
            if !required.contains(pname.as_str()) {
                ty = format!("Option<{ty}>");
            }
            // A struct field is a public item `missing_docs` checks; the OAS
            // property name is the honest, deterministic summary.
            let _ = writeln!(b, "    /// The `{pname}` property of `{name}`.");
            // A docs-text-wins correction carries its citation into the
            // generated code (the OAS lists the field as required; the
            // ITS-REST docs text wins).
            if let Some(ov) = crate::plan::overrides::rest_optional_override(&ty_name, pname) {
                let _ = writeln!(b, "    /// OPTIONAL by the docs text — {}", ov.citation);
                let _ = writeln!(b, "    /// ({})", ov.reason);
            }
            if let Some(rename) = naming::serde_rename(pname, &ident) {
                let _ = writeln!(b, "    #[serde(rename = \"{rename}\")]");
            }
            if !required.contains(pname.as_str()) {
                b.push_str(SKIP_NONE_ATTR);
            }
            let _ = writeln!(b, "    pub {ident}: {ty},");
        }
        emit_additional_properties(b, name, schema, ctx);
        b.push_str("}\n\n");
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
