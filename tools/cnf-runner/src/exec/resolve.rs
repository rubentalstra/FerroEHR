//! Reference resolution — turning the closed `${…}` grammar into concrete
//! values at execution time: corpus data sets (via the manifest), named
//! views, registered recipes, matrix/fixture row bindings, captures, and
//! the fixed temporal expressions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::exec::recipes::{self, BoundRow};
use crate::exec::state::{Captured, VarStore};
use crate::ids::{CorpusKey, RecipeName, ViewName};
use crate::model::case::{CaseCore, FixtureEntry, MatrixCell};
use crate::model::corpus::CorpusManifest;
use crate::model::value::TemplatedValue;
use crate::refgrammar::{FixtureField, Segment, Template, ValueRef};

/// Resolution error — an interpreter defect (the artifacts referenced
/// something unresolvable), never a conformance outcome.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("corpus key {0} is not in the manifest")]
    UnknownCorpusKey(CorpusKey),
    #[error("corpus key {key}: {message}")]
    Corpus { key: CorpusKey, message: String },
    #[error("view {view} on {key}: {message}")]
    View {
        key: CorpusKey,
        view: ViewName,
        message: String,
    },
    #[error("recipe {0} is not registered")]
    UnknownRecipe(RecipeName),
    #[error("{0}")]
    Recipe(#[from] recipes::RecipeError),
    #[error("row reference {0} outside the current row")]
    Row(String),
    #[error("{0}")]
    Vars(String),
}

/// The resolution context for one case×row.
pub struct Resolver<'a> {
    manifest: &'a CorpusManifest,
    corpus_dir: &'a Path,
    /// Cache of loaded corpus payloads.
    cache: BTreeMap<CorpusKey, Value>,
    /// The current matrix row (when the case has one).
    row: Option<(Vec<String>, Vec<MatrixCell>)>,
    /// The current fixture entry (when the case iterates fixtures).
    fixture: Option<FixtureEntry>,
    row_index: usize,
}

impl std::fmt::Debug for Resolver<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resolver")
            .field("row_index", &self.row_index)
            .finish_non_exhaustive()
    }
}

impl<'a> Resolver<'a> {
    /// A resolver rooted at the corpus manifest + its directory.
    #[must_use]
    pub fn new(manifest: &'a CorpusManifest, corpus_dir: &'a Path) -> Self {
        Self {
            manifest,
            corpus_dir,
            cache: BTreeMap::new(),
            row: None,
            fixture: None,
            row_index: 0,
        }
    }

    /// Bind the resolver to a case row (matrix or fixture iteration).
    pub fn bind_row(&mut self, case: &CaseCore, row: usize) {
        self.row_index = row;
        self.row = case
            .parameters
            .as_ref()
            .and_then(|p| p.matrix.as_ref())
            .and_then(|m| {
                m.rows
                    .get(row)
                    .map(|cells| (m.columns.clone(), cells.clone()))
            });
        self.fixture = case
            .parameters
            .as_ref()
            .and_then(|p| p.fixture_set.as_ref())
            .and_then(|fs| fs.get(row).cloned());
    }

    /// The current row's index.
    #[must_use]
    pub fn row_index(&self) -> usize {
        self.row_index
    }

    /// The current row's cell under a named column (sentinels visible).
    #[must_use]
    pub fn row_cell(&self, column: &str) -> Option<&MatrixCell> {
        let (columns, cells) = self.row.as_ref()?;
        columns
            .iter()
            .position(|c| c == column)
            .and_then(|i| cells.get(i))
    }

    /// Load a corpus payload (JSON parsed for JSON-family formats; text
    /// wrapped as a JSON string for opt-xml/adl2-text/aql-text sources;
    /// generated sets synthesize element 0 unless indexed via views).
    ///
    /// # Errors
    /// [`ResolveError`] when the key is unknown or the source unreadable.
    pub fn data_set(&mut self, key: &CorpusKey) -> Result<Value, ResolveError> {
        if let Some(cached) = self.cache.get(key) {
            return Ok(cached.clone());
        }
        let entry = self
            .manifest
            .get(key)
            .ok_or_else(|| ResolveError::UnknownCorpusKey(key.clone()))?;
        let value = if let Some(source) = &entry.source {
            let path: PathBuf = self.corpus_dir.join(source);
            let text = std::fs::read_to_string(&path).map_err(|e| ResolveError::Corpus {
                key: key.clone(),
                message: format!("{}: {e}", path.display()),
            })?;
            match entry.format {
                crate::vocab::CorpusFormat::CanonicalJson
                | crate::vocab::CorpusFormat::WtFlat
                | crate::vocab::CorpusFormat::WtStructured => {
                    serde_json::from_str(&text).map_err(|e| ResolveError::Corpus {
                        key: key.clone(),
                        message: format!("JSON parse: {e}"),
                    })?
                }
                crate::vocab::CorpusFormat::CanonicalXml
                | crate::vocab::CorpusFormat::OptXml
                | crate::vocab::CorpusFormat::Adl2Text
                | crate::vocab::CorpusFormat::AqlText => Value::String(text),
            }
        } else if let Some(generated) = &entry.generated_by {
            // Generated sets: the whole set as a JSON array via the recipe.
            let name = generated.recipe.as_str();
            let series: Result<Vec<Value>, recipes::RecipeError> = match name {
                "bp_series" => (0..10).map(recipes::bp_series).collect(),
                "query_bp" => (0..10).map(recipes::query_bp).collect(),
                other => {
                    return Err(ResolveError::Corpus {
                        key: key.clone(),
                        message: format!("generated_by recipe {other} has no registered generator"),
                    });
                }
            };
            Value::Array(series?)
        } else {
            return Err(ResolveError::Corpus {
                key: key.clone(),
                message: "entry has neither source nor generated_by".to_owned(),
            });
        };
        self.cache.insert(key.clone(), value.clone());
        Ok(value)
    }

    /// Evaluate a named view over a data set. The committed views are
    /// declarative; the registered evaluators here implement the exact
    /// projections the manifest declares (each is part of the recipe
    /// contract and listed as a registered exception).
    ///
    /// # Errors
    /// [`ResolveError`] when the view is undeclared or its evaluator is
    /// not registered.
    pub fn view(&mut self, key: &CorpusKey, view: &ViewName) -> Result<Value, ResolveError> {
        let entry = self
            .manifest
            .get(key)
            .ok_or_else(|| ResolveError::UnknownCorpusKey(key.clone()))?;
        if entry.view(view).is_none() {
            return Err(ResolveError::View {
                key: key.clone(),
                view: view.clone(),
                message: "view not declared in the manifest".to_owned(),
            });
        }
        let data = self.data_set(key)?;
        match view.as_str() {
            // cnf.flat.vitals.minimal_ctx#temperature_magnitude
            "temperature_magnitude" => data
                .get("vitals/body_temperature:0/any_event:0/temperature|magnitude")
                .cloned()
                .ok_or_else(|| ResolveError::View {
                    key: key.clone(),
                    view: view.clone(),
                    message: "flat key missing".to_owned(),
                }),
            // cnf.security.signed_version#signature
            "signature" => data
                .get("signature")
                .cloned()
                .ok_or_else(|| ResolveError::View {
                    key: key.clone(),
                    view: view.clone(),
                    message: "signature attribute missing".to_owned(),
                }),
            // cnf.set.bp-10#magnitude_ge_140_by_uid and friends resolve at
            // query time against committed uids — the driver substitutes the
            // captured uid list; here we return the SELECTION SPEC the
            // driver evaluates.
            "magnitude_ge_140_by_uid" | "magnitude_ge_140" => {
                Ok(serde_json::json!({ "systolic_min": 140, "order": "uid" }))
            }
            other => Err(ResolveError::View {
                key: key.clone(),
                view: view.clone(),
                message: format!("view {other} has no registered evaluator"),
            }),
        }
    }

    /// Resolve one reference to a JSON value.
    ///
    /// # Errors
    /// [`ResolveError`] on unresolvable references.
    pub fn resolve_ref(&mut self, r: &ValueRef, vars: &VarStore) -> Result<Value, ResolveError> {
        match r {
            ValueRef::Row(column) => {
                let (columns, cells) = self.row.as_ref().ok_or_else(|| {
                    ResolveError::Row(format!("${{row.{column}}} without a bound row"))
                })?;
                let i = columns.iter().position(|c| c == column).ok_or_else(|| {
                    ResolveError::Row(format!("column {column} not in the matrix"))
                })?;
                match cells.get(i) {
                    Some(MatrixCell::Literal(v)) => Ok(v.clone()),
                    // `null` is a FIRST-CLASS cell value (stays in the payload);
                    // `absent` means omit — distinguished by the Map resolver
                    // via `row_cell`, which sees the sentinel itself.
                    Some(MatrixCell::Null | MatrixCell::Absent) | None => Ok(Value::Null),
                    Some(MatrixCell::Provided) => {
                        // `provided` in an id column: deterministic synthesis.
                        Ok(Value::String(recipes::deterministic_ehr_id(self.row_index)))
                    }
                }
            }
            ValueRef::Fixture(field) => {
                let fixture = self.fixture.as_ref().ok_or_else(|| {
                    ResolveError::Row("${fixture.*} without a fixture row".to_owned())
                })?;
                Ok(match field {
                    FixtureField::DataSet => Value::String(fixture.data_set.to_string()),
                    FixtureField::Expected => Value::String(fixture.expected.token().to_owned()),
                    FixtureField::Defect => {
                        fixture.defect.clone().map_or(Value::Null, Value::String)
                    }
                })
            }
            ValueRef::FixtureDataSet => {
                let key = self
                    .fixture
                    .as_ref()
                    .map(|f| f.data_set.clone())
                    .ok_or_else(|| {
                        ResolveError::Row("${ds:fixture} without a fixture row".to_owned())
                    })?;
                self.data_set(&key)
            }
            ValueRef::Capture { name, optional } => match vars.get(name) {
                Some(Captured::Scalar(s)) => Ok(Value::String(s.clone())),
                Some(Captured::List(items)) => Ok(Value::Array(
                    items.iter().cloned().map(Value::String).collect(),
                )),
                Some(Captured::Body(v)) => Ok(v.clone()),
                Some(Captured::InstantMs(ms)) => Ok(Value::Number((*ms).into())),
                None if *optional => Ok(Value::Null),
                None => Err(ResolveError::Vars(format!("capture {name} is not bound"))),
            },
            ValueRef::DataSet { key, view: None } => self.data_set(key),
            ValueRef::DataSet {
                key,
                view: Some(view),
            } => self.view(key, view),
            ValueRef::Recipe(name) => match name.as_str() {
                "ehr_status" => {
                    let (columns, cells) = self.row.as_ref().ok_or_else(|| {
                        ResolveError::Row(
                            "${recipe:ehr_status(row)} without a bound row".to_owned(),
                        )
                    })?;
                    let bound = BoundRow { columns, cells };
                    Ok(recipes::ehr_status(&bound, self.row_index)?.unwrap_or(Value::Null))
                }
                _ => Err(ResolveError::UnknownRecipe(name.clone())),
            },
            ValueRef::Time(expr) => {
                let ms = vars.resolve_time(expr).map_err(ResolveError::Vars)?;
                Ok(Value::String(format_instant_ms(ms)))
            }
        }
    }

    /// Resolve a templated string: a single-reference template yields the
    /// referenced value verbatim; a mixed template renders to a string.
    ///
    /// # Errors
    /// [`ResolveError`] on unresolvable references.
    pub fn resolve_template(
        &mut self,
        template: &Template,
        vars: &VarStore,
    ) -> Result<Value, ResolveError> {
        if let Some(single) = template.as_single_ref() {
            return self.resolve_ref(single, vars);
        }
        let mut out = String::new();
        for segment in template.segments() {
            match segment {
                Segment::Lit(s) => out.push_str(s),
                Segment::Ref(r) => match self.resolve_ref(r, vars)? {
                    Value::String(s) => out.push_str(&s),
                    other => out.push_str(&other.to_string()),
                },
            }
        }
        Ok(Value::String(out))
    }

    /// Whether a payload field is OMITTED for the current row: the value is
    /// a single reference whose resolution is the absent sentinel (row cell
    /// `absent`, an `ehr_status: absent` recipe product, or an unresolved
    /// optional capture).
    fn omits_field(&mut self, value: &TemplatedValue, vars: &VarStore) -> bool {
        let TemplatedValue::Text(t) = value else {
            return false;
        };
        let Some(single) = t.as_single_ref() else {
            return false;
        };
        match single {
            ValueRef::Row(column) => {
                matches!(self.row_cell(column), Some(MatrixCell::Absent) | None)
            }
            ValueRef::Capture {
                name,
                optional: true,
            } => vars.get(name).is_none(),
            ValueRef::Recipe(name) if name.as_str() == "ehr_status" => {
                matches!(self.row_cell("ehr_status"), Some(MatrixCell::Absent))
            }
            ValueRef::Fixture(FixtureField::Defect) => {
                self.fixture.as_ref().is_none_or(|f| f.defect.is_none())
            }
            _ => false,
        }
    }

    /// Resolve a whole `with:` payload tree. `absent`-sentinel row cells and
    /// unresolved-optional references resolve to `Null` and are DROPPED from
    /// object payloads (the normative absent-means-omit rule).
    ///
    /// # Errors
    /// [`ResolveError`] on unresolvable references.
    pub fn resolve_value(
        &mut self,
        value: &TemplatedValue,
        vars: &VarStore,
    ) -> Result<Value, ResolveError> {
        Ok(match value {
            TemplatedValue::Null => Value::Null,
            TemplatedValue::Bool(b) => Value::Bool(*b),
            TemplatedValue::Number(n) => Value::Number(n.clone()),
            TemplatedValue::Text(t) => self.resolve_template(t, vars)?,
            TemplatedValue::Seq(items) => Value::Array(
                items
                    .iter()
                    .map(|v| self.resolve_value(v, vars))
                    .collect::<Result<_, _>>()?,
            ),
            TemplatedValue::Map(entries) => {
                let mut map = serde_json::Map::new();
                for (k, v) in entries {
                    // absent-means-omit: an `absent` row sentinel, an absent
                    // recipe product, or an unresolved optional drops the
                    // field entirely; a `null` cell stays a literal null.
                    if self.omits_field(v, vars) {
                        continue;
                    }
                    let resolved = self.resolve_value(v, vars)?;
                    map.insert(k.clone(), resolved);
                }
                Value::Object(map)
            }
        })
    }
}

/// Milliseconds since the Unix epoch → an ISO 8601 UTC instant with
/// millisecond precision (pure integer arithmetic — no clock, no locale).
#[must_use]
pub fn format_instant_ms(ms: i64) -> String {
    let (secs, millis) = (ms.div_euclid(1000), ms.rem_euclid(1000));
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    // civil-from-days (Howard Hinnant's algorithm, integer-only)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    fn manifest() -> CorpusManifest {
        serde_saphyr::from_str(
            "cnf.set.bp-10:\n  generated_by: { recipe: bp_series, digest: \"sha256:x\" }\n  format: canonical-json\n  validity: { verdict: valid }\n  provenance: p\n  views:\n    magnitude_ge_140_by_uid: { select: s }\n",
        )
        .unwrap()
    }

    #[test]
    fn generated_sets_and_instants() {
        let m = manifest();
        let dir = std::path::PathBuf::from(".");
        let mut r = Resolver::new(&m, &dir);
        let set = r
            .data_set(&CorpusKey::parse("cnf.set.bp-10").unwrap())
            .unwrap();
        assert_eq!(set.as_array().unwrap().len(), 10);

        assert_eq!(format_instant_ms(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format_instant_ms(1_767_225_600_000),
            "2026-01-01T00:00:00.000Z"
        );
        assert_eq!(format_instant_ms(999), "1970-01-01T00:00:00.999Z");
    }

    #[test]
    fn absent_cells_drop_from_payloads() {
        let m = manifest();
        let dir = std::path::PathBuf::from(".");
        let mut r = Resolver::new(&m, &dir);
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "I_EHR_SERVICE.create_ehr-x", "kind": "functional", "component": "EHR",
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "capabilities": ["EhrOperations"], "profiles": ["CORE"],
            "test_purpose": "t", "description": "d",
            "spec_refs": ["CNF platform_test_schedule master06 §create_ehr data sets"],
            "parameters": { "iteration": "reset_per_row",
                "matrix": { "columns": ["ehr_id"], "rows": [["absent"], ["provided"]] } },
            "flow": [ { "step": 1, "call": "create_ehr", "expect": "created" } ]
        }))
        .unwrap();
        let payload: TemplatedValue =
            serde_json::from_value(serde_json::json!({ "ehr_id": "${row.ehr_id}" })).unwrap();
        let vars = VarStore::default();

        r.bind_row(&case, 0); // absent
        let v = r.resolve_value(&payload, &vars).unwrap();
        assert!(v.get("ehr_id").is_none());

        r.bind_row(&case, 1); // provided -> deterministic id
        let v = r.resolve_value(&payload, &vars).unwrap();
        assert!(v.get("ehr_id").unwrap().as_str().unwrap().contains('-'));
    }
}
