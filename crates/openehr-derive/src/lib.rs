//! Proc-macros for the generated openEHR spec crates.
//!
//! [`OpenEhrType`] gives a struct canonical-JSON (de)serialization with the
//! openEHR `_type` discriminator, **without** a per-struct tag field:
//!
//! - **Serialize**: emits `"_type": "<CLASS>"` as the first entry, then each
//!   field. `Option` fields are omitted when `None`; `Vec` fields are omitted
//!   when empty. (This matches serde's `skip_serializing_if` conventions, done
//!   here so the generated struct itself carries no serde attributes.)
//! - **Deserialize**: accepts input with or without `_type`; if present, it
//!   must equal the class name (mismatch is an error). This tag check is what
//!   lets the abstract-slot enums (emitted by `openehr-codegen`) dispatch on
//!   `_type` — see their hand-rolled `Deserialize`, which *requires* `_type` on
//!   an abstract polymorphic slot and rejects a `_type`-less value rather
//!   than guessing structurally. Unknown wire keys are ignored; this
//!   deliberate tolerance (a superset of the ITS-JSON schema's
//!   `additionalProperties: false`) is documented as a `PORT NOTE` on the
//!   shadow struct below.
//!
//! Usage (emitted by `openehr-codegen`):
//! ```ignore
//! #[derive(Debug, Clone, PartialEq, OpenEhrType)]
//! #[openehr(type_name = "DV_QUANTITY")]
//! pub struct DvQuantity {
//!     pub magnitude: f64,
//!     pub precision: Option<i32>,          // omitted when None
//!     #[openehr(rename = "use")]
//!     pub use_: String,                     // serialized as "use"
//!     pub other_reference_ranges: Vec<ReferenceRange<DvQuantity>>, // omitted when empty
//! }
//! ```

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Type, parse_macro_input};

/// Derive canonical openEHR `_type` (de)serialization. See the crate docs.
#[proc_macro_derive(OpenEhrType, attributes(openehr))]
pub fn derive_openehr_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct FieldInfo {
    ident: Ident,
    /// Wire name (`_type`-level JSON key).
    wire: String,
    kind: FieldKind,
    ty: Type,
    /// A literal default (`"true"` / `"false"` / …) for a `Plain` field the wire
    /// may omit — e.g. archie omits the `Interval` `*_included`/`*_unbounded`
    /// flags. When set, the field deserializes to this default if absent instead
    /// of erroring, and is always re-emitted on serialize.
    default: Option<String>,
}

enum FieldKind {
    /// `Option<T>` — omit when `None`.
    Optional,
    /// `Vec<T>` — omit when empty.
    Container,
    /// Anything else — always present.
    Plain,
}

#[allow(clippy::too_many_lines)]
fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let type_name = openehr_type_name(input)?;

    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            name,
            "OpenEhrType can only be derived for structs",
        ));
    };
    let Fields::Named(named) = &data.fields else {
        return Err(syn::Error::new_spanned(
            name,
            "OpenEhrType requires named fields",
        ));
    };

    let mut fields = Vec::new();
    for f in &named.named {
        let Some(ident) = f.ident.clone() else {
            return Err(syn::Error::new_spanned(f, "expected a named field"));
        };
        // Default wire name = the field ident with any raw-identifier prefix
        // stripped (`r#type` → `type`), matching serde's own derive. An explicit
        // `#[openehr(rename = "…")]` overrides this.
        let wire = field_rename(f)?
            .unwrap_or_else(|| ident.to_string().trim_start_matches("r#").to_string());
        let kind = classify(&f.ty);
        let default = field_default(f)?;
        fields.push(FieldInfo {
            ident,
            wire,
            kind,
            ty: f.ty.clone(),
            default,
        });
    }

    // Original type generics for `Name<T>` references.
    let (_orig_impl_g, ty_g, _orig_where_g) = input.generics.split_for_impl();

    // Serialize impl generics: original params + a `Serialize` bound each.
    let mut ser_generics = input.generics.clone();
    for tp in ser_generics.type_params_mut() {
        tp.bounds.push(syn::parse_quote!(::serde::Serialize));
    }
    let (ser_impl_g, _, ser_where_g) = ser_generics.split_for_impl();

    // Deserialize impl generics: prepend `'de` + a `DeserializeOwned` bound each.
    // openEHR RM types are pure owned data (no borrowed fields), and the
    // abstract-slot enums dispatch on `_type` by buffering the value into an owned
    // `serde_json::Value` and re-deserializing with `serde_json::from_value`, which
    // requires the payload — hence any generic parameter — to be `DeserializeOwned`
    // (`for<'a> Deserialize<'a>`). Bounding the parameters here keeps a generic
    // container (`History<T>` ⊇ `Vec<Event<T>>`) composable with those enums.
    let mut de_generics = input.generics.clone();
    for tp in de_generics.type_params_mut() {
        tp.bounds
            .push(syn::parse_quote!(::serde::de::DeserializeOwned));
    }
    de_generics.params.insert(0, syn::parse_quote!('de));
    let (de_impl_g, _, de_where_g) = de_generics.split_for_impl();

    // Shadow struct declaration uses the original generics (serde's derive adds
    // its own `'de` + bounds when it expands on the shadow).
    let (shadow_decl_g, _, shadow_where_g) = input.generics.split_for_impl();

    // ── Serialize: serialize_map, `_type` first, skipping None/empty ─────────
    let ser_entries = fields.iter().map(|f| {
        let id = &f.ident;
        let wire = &f.wire;
        match f.kind {
            FieldKind::Optional => quote! {
                if self.#id.is_some() {
                    map.serialize_entry(#wire, &self.#id)?;
                }
            },
            FieldKind::Container => quote! {
                if !self.#id.is_empty() {
                    map.serialize_entry(#wire, &self.#id)?;
                }
            },
            FieldKind::Plain => quote! {
                map.serialize_entry(#wire, &self.#id)?;
            },
        }
    });

    // ── Deserialize: module-level shadow struct (handles generics uniformly) ──
    let shadow = format_ident!("__OpenEhrShadow_{}", name);
    // Pin the shadow's serde bounds explicitly so serde does not infer a
    // spurious `T: Default` from the field-level `#[serde(default)]`s.
    let de_bounds: Vec<String> = input
        .generics
        .type_params()
        .map(|tp| format!("{}: ::serde::de::DeserializeOwned", tp.ident))
        .collect();
    let shadow_bound_attr = if de_bounds.is_empty() {
        quote! {}
    } else {
        let s = de_bounds.join(", ");
        quote! { #[serde(bound(deserialize = #s))] }
    };
    // Default-value helper fns for `Plain` fields the wire may omit (e.g. the
    // `Interval` `*_included`/`*_unbounded` flags), injected into the shadow's
    // const block so `#[serde(default = "…")]` can name them.
    let default_fn_ident =
        |f: &FieldInfo| format_ident!("__default_{}", f.ident.to_string().trim_start_matches("r#"));
    let default_fns: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .filter_map(|f| {
            let lit = f.default.as_ref()?;
            let fname = default_fn_ident(f);
            let ty = &f.ty;
            let val: proc_macro2::TokenStream = lit.parse().ok()?;
            Some(quote! { fn #fname() -> #ty { #val } })
        })
        .collect();

    let shadow_fields = fields.iter().map(|f| {
        let id = &f.ident;
        let ty = &f.ty;
        let wire = &f.wire;
        // Every field is optional-with-default on the shadow so missing keys are
        // tolerated; plain fields are then required back in the conversion,
        // unless the field carries an explicit default.
        if let FieldKind::Plain = f.kind
            && f.default.is_some()
        {
            let fname = default_fn_ident(f).to_string();
            return quote! {
                #[serde(rename = #wire, default = #fname)]
                #id: #ty,
            };
        }
        match f.kind {
            FieldKind::Optional | FieldKind::Container => quote! {
                #[serde(rename = #wire, default)]
                #id: #ty,
            },
            FieldKind::Plain => quote! {
                #[serde(rename = #wire, default)]
                #id: ::core::option::Option<#ty>,
            },
        }
    });
    let convert_fields = fields.iter().map(|f| {
        let id = &f.ident;
        let wire = &f.wire;
        if let FieldKind::Plain = f.kind
            && f.default.is_some()
        {
            return quote! { #id: shadow.#id, };
        }
        match f.kind {
            FieldKind::Optional | FieldKind::Container => quote! {
                #id: shadow.#id,
            },
            FieldKind::Plain => quote! {
                #id: shadow.#id.ok_or_else(|| ::serde::de::Error::missing_field(#wire))?,
            },
        }
    });

    let expanded = quote! {
        impl #ser_impl_g ::serde::Serialize for #name #ty_g #ser_where_g {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                use ::serde::ser::SerializeMap as _;
                let mut map = serializer.serialize_map(::core::option::Option::None)?;
                map.serialize_entry("_type", #type_name)?;
                #(#ser_entries)*
                map.end()
            }
        }

        const _: () = {
            #(#default_fns)*

            // PORT NOTE: unknown wire keys are deliberately *ignored*
            // (no `#[serde(deny_unknown_fields)]`), a documented superset of the
            // ITS-JSON schema's `additionalProperties: false`. Two reasons make
            // strict rejection the wrong default at the deserialize layer:
            //  1. RM-version skew — the generated types are RM 1.2.0 but the
            //     vendored ITS-JSON schema + SDK corpus are RM 1.1.0-era, so a
            //     conformant-for-its-version payload can legitimately carry keys
            //     this pinned model does not place identically.
            //  2. The vendored SDK corpus itself ships fixtures with stray keys
            //     (e.g. `feeder_system_audit` on an INSTRUCTION), and the
            //     openehr-its corpus-read fidelity gate requires them to load.
            // The strict wire-shape contract (`_type` present + no unknown keys)
            // is available separately via `openehr_its::json::validate_canonical`
            // (the ITS-JSON schema), to be run at the ingestion edge where strict
            // 400/422 rejection is desired. The *polymorphic-slot*
            // `_type` requirement — the one that caused silent type corruption —
            // is enforced unconditionally by the enums' hand-rolled `_type`
            // dispatch (F-04-01/03), independent of this leniency.
            #[derive(::serde::Deserialize)]
            #[allow(non_camel_case_types, non_snake_case)]
            #shadow_bound_attr
            struct #shadow #shadow_decl_g #shadow_where_g {
                #[serde(rename = "_type", default)]
                __type: ::core::option::Option<::std::string::String>,
                #(#shadow_fields)*
            }

            impl #de_impl_g ::serde::Deserialize<'de> for #name #ty_g #de_where_g {
                fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    let shadow = <#shadow #ty_g>::deserialize(deserializer)?;
                    if let ::core::option::Option::Some(t) = &shadow.__type {
                        if t != #type_name {
                            return ::core::result::Result::Err(::serde::de::Error::custom(
                                ::std::format!(
                                    "expected _type \"{}\", found \"{}\"",
                                    #type_name, t
                                ),
                            ));
                        }
                    }
                    ::core::result::Result::Ok(Self {
                        #(#convert_fields)*
                    })
                }
            }
        };
    };

    Ok(expanded)
}

/// Read the required `#[openehr(type_name = "...")]` from the struct.
fn openehr_type_name(input: &DeriveInput) -> syn::Result<LitStr> {
    let mut found = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("openehr") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("type_name") {
                let value = meta.value()?;
                found = Some(value.parse::<LitStr>()?);
                Ok(())
            } else {
                Err(meta.error("unknown openehr attribute (expected `type_name`)"))
            }
        })?;
    }
    found.ok_or_else(|| {
        syn::Error::new_spanned(
            &input.ident,
            "missing #[openehr(type_name = \"...\")] on OpenEhrType derive",
        )
    })
}

/// Read an optional `#[openehr(rename = "...")]` on a field.
fn field_rename(field: &syn::Field) -> syn::Result<Option<String>> {
    field_attr(field, "rename")
}

/// Read an optional `#[openehr(default = "<expr>")]` on a field — a literal Rust
/// expression (`"true"`, `"false"`) used as the field's default when the wire
/// omits it (kept as a string here; the derive parses it as tokens).
fn field_default(field: &syn::Field) -> syn::Result<Option<String>> {
    field_attr(field, "default")
}

/// Read a single string-valued `#[openehr(<key> = "...")]` field attribute,
/// ignoring the other recognized keys (`rename`, `default`).
fn field_attr(field: &syn::Field, key: &str) -> syn::Result<Option<String>> {
    const KNOWN: &[&str] = &["rename", "default"];
    let mut found = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("openehr") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                let value = meta.value()?;
                found = Some(value.parse::<LitStr>()?.value());
                Ok(())
            } else if KNOWN.iter().any(|k| meta.path.is_ident(k)) {
                // Recognized elsewhere; consume its value and skip.
                drop(meta.value()?.parse::<LitStr>()?);
                Ok(())
            } else {
                Err(meta.error("unknown openehr field attribute (expected `rename`/`default`)"))
            }
        })?;
    }
    Ok(found)
}

/// Classify a field type as `Option<_>`, `Vec<_>`, or plain by its head path.
fn classify(ty: &Type) -> FieldKind {
    if let Type::Path(p) = ty
        && let Some(seg) = p.path.segments.last()
    {
        match seg.ident.to_string().as_str() {
            "Option" => return FieldKind::Optional,
            "Vec" => return FieldKind::Container,
            _ => {}
        }
    }
    FieldKind::Plain
}
