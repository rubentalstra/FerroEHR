//! `OpenAPI` (OAS) reader for the REST codegen.
//!
//! Loads a vendored `*-codegen.openapi.yaml` bundle into a `serde_json::Value`
//! (via `serde_norway`) and exposes the pieces the emitter needs: the component
//! schemas, and each operation's method/path/params/request/response with local
//! `$ref`s resolved. The OAS is the source of truth; RM payload schemas are
//! recognized by name and resolved to the generated RM/BASE crates rather than
//! re-emitted.

use serde_json::Value;

/// A parsed `OpenAPI` document.
pub(crate) struct Oas {
    root: Value,
}

/// One HTTP operation (a method on a path).
#[allow(clippy::struct_field_names)] // `operation_id` mirrors the OAS field name
pub(crate) struct Operation<'a> {
    pub method: &'a str,
    pub path: &'a str,
    /// `operationId`, e.g. `ehr_create` — the trait method name.
    pub operation_id: String,
    /// Resolved parameters (path/query/header).
    pub parameters: Vec<Param>,
    /// The request body schema (resolved `Value`) + whether it is required.
    pub request_body: Option<(Value, bool)>,
    /// The primary success (2xx) response body schema (resolved), if any.
    pub success_body: Option<Value>,
}

/// One resolved operation parameter.
pub(crate) struct Param {
    pub name: String,
    /// `path` | `query` | `header`.
    pub location: String,
    pub required: bool,
    /// The parameter's schema (resolved `Value`).
    pub schema: Value,
}

impl Oas {
    /// Parse an OAS YAML bundle.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub(crate) fn parse_file(path: &std::path::Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let root: Value =
            serde_norway::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        Ok(Self { root })
    }

    /// The component schemas (`#/components/schemas`), in document order.
    #[must_use]
    pub(crate) fn schemas(&self) -> Vec<(String, &Value)> {
        self.root
            .pointer("/components/schemas")
            .and_then(Value::as_object)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v)).collect())
            .unwrap_or_default()
    }

    /// Resolve a local `$ref` (`#/components/...`) to the pointed-at value.
    #[must_use]
    pub(crate) fn resolve<'a>(&'a self, value: &'a Value) -> &'a Value {
        let mut cur = value;
        // Follow chained single-level `$ref`s (parameters/responses → schema).
        while let Some(r) = cur.get("$ref").and_then(Value::as_str) {
            let ptr = r.trim_start_matches('#');
            match self.root.pointer(ptr) {
                Some(next) => cur = next,
                None => break,
            }
        }
        cur
    }

    /// The base name of a `$ref` (`#/components/schemas/Ehr` → `Ehr`), if this
    /// value is a direct ref.
    #[must_use]
    pub(crate) fn ref_name(value: &Value) -> Option<String> {
        value
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|r| r.rsplit('/').next())
            .map(str::to_string)
    }

    /// All operations across all paths.
    #[must_use]
    pub(crate) fn operations(&self) -> Vec<Operation<'_>> {
        const METHODS: &[&str] = &["get", "put", "post", "delete", "patch"];
        let mut out = Vec::new();
        let Some(paths) = self.root.get("paths").and_then(Value::as_object) else {
            return out;
        };
        for (path, item) in paths {
            for method in METHODS {
                let Some(op) = item.get(method) else { continue };
                let operation_id = op
                    .get("operationId")
                    .and_then(Value::as_str)
                    .map_or_else(|| synth_op_id(method, path), str::to_string);
                let parameters = self.parse_params(op);
                let request_body = self.parse_request_body(op);
                let success_body = self.parse_success(op);
                out.push(Operation {
                    method,
                    path,
                    operation_id,
                    parameters,
                    request_body,
                    success_body,
                });
            }
        }
        out
    }

    fn parse_params(&self, op: &Value) -> Vec<Param> {
        let mut out = Vec::new();
        let Some(params) = op.get("parameters").and_then(Value::as_array) else {
            return out;
        };
        for p in params {
            let p = self.resolve(p);
            let (Some(name), Some(location)) = (
                p.get("name").and_then(Value::as_str),
                p.get("in").and_then(Value::as_str),
            ) else {
                continue;
            };
            out.push(Param {
                name: name.to_string(),
                location: location.to_string(),
                required: p.get("required").and_then(Value::as_bool).unwrap_or(false),
                schema: p
                    .get("schema")
                    .map_or(Value::Null, |s| self.resolve(s).clone()),
            });
        }
        out
    }

    fn parse_request_body(&self, op: &Value) -> Option<(Value, bool)> {
        let rb = self.resolve(op.get("requestBody")?);
        let required = rb.get("required").and_then(Value::as_bool).unwrap_or(false);
        let schema = self.first_json_schema(rb)?;
        Some((schema, required))
    }

    fn parse_success(&self, op: &Value) -> Option<Value> {
        let responses = op.get("responses")?.as_object()?;
        // The first 2xx response, in numeric order.
        let mut codes: Vec<&String> = responses.keys().filter(|c| c.starts_with('2')).collect();
        codes.sort();
        for code in codes {
            let resp = self.resolve(&responses[code]);
            if let Some(schema) = self.first_json_schema(resp) {
                return Some(schema);
            }
        }
        None
    }

    /// The `application/json` (or first) content schema of a requestBody/response.
    fn first_json_schema(&self, container: &Value) -> Option<Value> {
        let content = container.get("content")?.as_object()?;
        let media = content
            .get("application/json")
            .or_else(|| content.values().next())?;
        let schema = media.get("schema")?;
        Some(self.resolve(schema).clone())
    }
}

/// Synthesize an operation id from method + path when the OAS omits one.
fn synth_op_id(method: &str, path: &str) -> String {
    let mut s = method.to_string();
    for seg in path.split('/').filter(|s| !s.is_empty()) {
        s.push('_');
        s.push_str(&seg.replace(['{', '}'], "").replace('-', "_"));
    }
    s
}
