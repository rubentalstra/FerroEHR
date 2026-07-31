//! Generic ODIN rendering shared by every ODIN-shaped section: optional string
//! attributes, keyed string maps and lists, the `_default` pseudo-attribute
//! carrying a canonical-JSON value, and the ODIN leaf literals (quoted strings,
//! scalars, `TERM_CODE_REF`) — `LANG/docs/odin/master03` +
//! `master07-leaf_data`.

use std::collections::BTreeMap;

use openehr_base::prelude::TerminologyCode;

use crate::print::Printer;

impl Printer {
    // ── ODIN helpers ────────────────────────────────────────────────────────
    pub(super) fn opt_string(&mut self, depth: usize, key: &str, v: Option<&str>) {
        if let Some(v) = v {
            self.line(depth, &format!("{key} = <{}>", quoted(v)));
        }
    }

    /// Emit a `C_DEFINED_OBJECT.default_value` as the `_default` pseudo-attribute
    /// (`master06-default_values.adoc` §Syntax): `_default = (RM_TYPE) < … >`
    /// with the canonical-JSON intermediate rendered as ODIN — the inverse of
    /// the cADL parser's `_default` handling (`odin_to_json`). Scalar and
    /// object shapes round-trip through print → parse exactly; a JSON array
    /// of objects re-parses as a keyed object (see the [`Self::odin_json_entry`]
    /// NOTE) — the ODIN text is the durable form either way.
    pub(super) fn default_value(&mut self, v: &serde_json::Value, depth: usize) {
        match v {
            serde_json::Value::Object(m) => {
                let head = match m.get("_type").and_then(serde_json::Value::as_str) {
                    Some(t) => format!("_default = ({t}) <"),
                    None => "_default = <".to_owned(),
                };
                self.line(depth, &head);
                for (k, val) in m {
                    if k == "_type" {
                        continue;
                    }
                    self.odin_json_entry(k, val, depth + 1);
                }
                self.line(depth, ">");
            }
            other => self.line(depth, &format!("_default = <{}>", odin_scalar(other))),
        }
    }

    /// One ODIN attribute line (or block) for a canonical-JSON member.
    ///
    /// NOTE: a JSON array of objects has no positional ODIN form — ODIN
    /// containers are keyed lists — so it renders as `["1"] = <…>` entries;
    /// re-parsing yields a `"1"`-keyed object rather than an array. The ODIN
    /// text is the durable ADL2 form; the JSON intermediate carries no
    /// spec-mandated shape (no openEHR spec governs it — our own design,
    /// matching the parser's `odin_to_json`).
    fn odin_json_entry(&mut self, key: &str, v: &serde_json::Value, depth: usize) {
        match v {
            serde_json::Value::Null => {}
            serde_json::Value::Object(m) => {
                let head = match m.get("_type").and_then(serde_json::Value::as_str) {
                    Some(t) => format!("{key} = ({t}) <"),
                    None => format!("{key} = <"),
                };
                self.line(depth, &head);
                for (k, val) in m {
                    if k == "_type" {
                        continue;
                    }
                    self.odin_json_entry(k, val, depth + 1);
                }
                self.line(depth, ">");
            }
            serde_json::Value::Array(items) if items.iter().all(is_json_scalar) => {
                let joined = items.iter().map(odin_scalar).collect::<Vec<_>>().join(", ");
                self.line(depth, &format!("{key} = <{joined}>"));
            }
            serde_json::Value::Array(items) => {
                self.line(depth, &format!("{key} = <"));
                for (i, item) in items.iter().enumerate() {
                    self.odin_json_entry(
                        &format!("[{}]", quoted(&(i + 1).to_string())),
                        item,
                        depth + 1,
                    );
                }
                self.line(depth, ">");
            }
            scalar => self.line(depth, &format!("{key} = <{}>", odin_scalar(scalar))),
        }
    }

    pub(super) fn odin_string_map(
        &mut self,
        depth: usize,
        key: &str,
        m: &BTreeMap<String, String>,
    ) {
        if m.is_empty() {
            return;
        }
        self.line(depth, &format!("{key} = <"));
        for (k, v) in m {
            self.line(depth + 1, &format!("[{}] = <{}>", quoted(k), quoted(v)));
        }
        self.line(depth, ">");
    }

    pub(super) fn odin_string_list(&mut self, depth: usize, key: &str, list: &[String]) {
        if list.is_empty() {
            return;
        }
        let joined = list
            .iter()
            .map(|s| quoted(s))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(depth, &format!("{key} = <{joined}>"));
    }
}

/// Whether a canonical-JSON value renders as one ODIN scalar token.
fn is_json_scalar(v: &serde_json::Value) -> bool {
    matches!(
        v,
        serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Bool(_)
    )
}

/// Render a canonical-JSON scalar as its ODIN literal (the inverse of the
/// cADL parser's `odin_to_json` scalar arms).
fn odin_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => quoted(s),
        serde_json::Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        serde_json::Value::Number(n) => n.to_string(),
        // Null/containers never reach here (`is_json_scalar` gates the list
        // form; containers take the block form).
        other => other.to_string(),
    }
}

/// A double-quoted, `master03`-escaped string literal.
pub(super) fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// `[terminology::code]` (with optional `(version)`), the ODIN `TERM_CODE_REF`
/// form (`master07-leaf_data`).
pub(super) fn term_code_str(t: &TerminologyCode) -> String {
    let id = match &t.terminology_version {
        Some(v) => format!("{}({v})", t.terminology_id),
        None => t.terminology_id.clone(),
    };
    format!("[{id}::{}]", t.code_string)
}
