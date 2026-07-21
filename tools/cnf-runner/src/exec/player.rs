//! The transcript player — the runner-verification pack's part 1: replay a
//! fixed transcript and reproduce the adjudicated verdicts. The transcript
//! is itself a specified artifact (`transcript.schema.json`): an ordered
//! sequence per case × format × row of recorded exchanges with adjudicated
//! expected verdicts; the player answers the Nth matching request with the
//! Nth recorded response (matching = method + path suffix), so a fixture
//! file fully determines what any conformant runner must conclude.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::exec::state::{Captured, VarStore};
use crate::exec::{StepDriver, StepObservation, outcome};
use crate::ids::{CaptureName, CaseId, SmOperationRef};
use crate::model::binding::WireFrom;
use crate::model::case::{CaseCore, FlowStep};
use crate::vocab::{FormatName, OutcomeKind};

/// One recorded response in a transcript entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<Value>,
}

/// One recorded request key (matching = method + path suffix + step).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub body_digest: Option<String>,
}

/// The adjudicated expectation for one step exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptStep {
    pub step: u32,
    pub request: RecordedRequest,
    pub response: RecordedResponse,
}

/// One case×format×row sequence with its adjudicated verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptEntry {
    pub case: CaseId,
    #[serde(default)]
    pub format: Option<FormatName>,
    pub row: usize,
    pub steps: Vec<TranscriptStep>,
    /// The adjudicated per-row verdict the runner MUST reproduce.
    pub expected_verdict: ExpectedVerdict,
    /// The adjudication citation (spec text, register entry).
    pub adjudication_ref: String,
}

/// The adjudicated verdict vocabulary of the pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedVerdict {
    Passed,
    Failed,
    Errored,
    NotApplicable,
    Skipped,
}

/// A whole transcript document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transcript {
    /// The schedule release the adjudications were made against.
    pub schedule_release: String,
    pub entries: Vec<TranscriptEntry>,
}

/// The player: a [`StepDriver`] answering from recorded exchanges.
#[derive(Debug)]
pub struct TranscriptPlayer<'a> {
    set: &'a crate::artifacts::ArtifactSet,
    entry: &'a TranscriptEntry,
    cursor: usize,
}

impl<'a> TranscriptPlayer<'a> {
    /// A player over one transcript entry.
    #[must_use]
    pub fn new(set: &'a crate::artifacts::ArtifactSet, entry: &'a TranscriptEntry) -> Self {
        Self {
            set,
            entry,
            cursor: 0,
        }
    }

    fn binding_for(
        &self,
        case: &CaseCore,
        call: &str,
    ) -> Option<&'a crate::model::binding::OperationBinding> {
        let op = if call.contains('.') {
            SmOperationRef::parse(call).ok()?
        } else {
            case.sm_operation.as_ref()?.sibling(call)
        };
        self.set
            .bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| b.sm_operation == op)
    }
}

impl StepDriver for TranscriptPlayer<'_> {
    fn perform(
        &mut self,
        case: &CaseCore,
        step: &FlowStep,
        _expected: OutcomeKind,
        _row: usize,
        vars: &mut VarStore,
    ) -> Result<StepObservation, String> {
        let Some(recorded) = self.entry.steps.get(self.cursor) else {
            return Ok(StepObservation {
                observation: outcome::Observation::Transport(
                    "transcript exhausted before the flow ended".to_owned(),
                ),
                assertion_failures: Vec::new(),
            });
        };
        self.cursor += 1;

        let Some(binding) = self.binding_for(case, &step.call) else {
            return Err(format!("no binding declares operation {}", step.call));
        };
        let selectors = self.set.selectors.as_ref().map(|(_, s)| s);
        let observation = outcome::classify_status(binding, selectors, recorded.response.status);

        // Bind captures from the recorded response exactly like the live
        // driver (same closed grammar).
        if let outcome::Observation::Kind(kind) = observation {
            for (name, source) in step.captures() {
                if source.outcome != kind {
                    continue;
                }
                match &source.field {
                    crate::refgrammar::CaptureField::Body => {
                        if let Some(b) = &recorded.response.body {
                            vars.set(name.clone(), Captured::Body(b.clone()));
                        }
                    }
                    crate::refgrammar::CaptureField::CommitTime => {
                        let ms = i64::try_from(self.cursor).unwrap_or(i64::MAX) * 1_000;
                        vars.set(name.clone(), Captured::InstantMs(ms));
                    }
                    crate::refgrammar::CaptureField::Field { name: field, list } => {
                        let Some(spec) = binding
                            .captures
                            .as_deref()
                            .unwrap_or_default()
                            .iter()
                            .find(|(n, _)| n == field)
                            .map(|(_, s)| s)
                        else {
                            continue;
                        };
                        if *list {
                            if let (Some(body), WireFrom::Body { path }) =
                                (&recorded.response.body, &spec.from)
                            {
                                let items = extract_list(body, path);
                                vars.set(name.clone(), Captured::List(items));
                            }
                        } else if let Some(value) = extract_scalar(&recorded.response, spec, vars) {
                            vars.set(name.clone(), Captured::Scalar(value));
                        }
                    }
                }
            }
        }
        Ok(StepObservation {
            observation,
            assertion_failures: Vec::new(),
        })
    }

    fn provision(
        &mut self,
        _case: &CaseCore,
        _row: usize,
        vars: &mut VarStore,
    ) -> Result<(), String> {
        // Provisioning is pre-recorded state in the pack; the requires
        // handles bind to fixed adjudicated values.
        if let Ok(handle) = CaptureName::parse("ehr_id") {
            vars.set(
                handle,
                Captured::Scalar("7d44b88c-4199-4bad-97dc-d78268e01398".to_owned()),
            );
        }
        Ok(())
    }

    fn postconditions(
        &mut self,
        _case: &CaseCore,
        _row: usize,
        _vars: &mut VarStore,
    ) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    fn aggregates(
        &mut self,
        case: &CaseCore,
        all_rows: &[VarStore],
    ) -> Result<Vec<String>, String> {
        let mut failures = Vec::new();
        for assertion in &case.postconditions {
            if let crate::model::assertion::Assertion::Unique { over, .. } = assertion
                && let crate::refgrammar::ValueRef::Capture { name, .. } = &over.0
                && let Err(crate::exec::assertions::AssertionFailure(m)) =
                    crate::exec::assertions::eval_unique(name, all_rows)
            {
                failures.push(m);
            }
        }
        Ok(failures)
    }
}

fn extract_scalar(
    response: &RecordedResponse,
    spec: &crate::model::binding::WireCapture,
    vars: &VarStore,
) -> Option<String> {
    let from_source = |source: &WireFrom| -> Option<String> {
        match source {
            WireFrom::Header { name, last_segment } => {
                let value = response
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(name))
                    .map(|(_, v)| v.clone())?;
                if *last_segment {
                    value.rsplit('/').next().map(ToOwned::to_owned)
                } else {
                    Some(value)
                }
            }
            WireFrom::Body { path } => {
                let mut current = response.body.as_ref()?;
                for seg in path.split('.') {
                    current = current.get(seg)?;
                }
                match current {
                    Value::String(s) => Some(s.clone()),
                    other => Some(other.to_string()),
                }
            }
            WireFrom::Capture(name) => vars.scalar(name).map(ToOwned::to_owned),
        }
    };
    let mut value =
        from_source(&spec.from).or_else(|| spec.fallback.as_ref().and_then(from_source))?;
    if matches!(
        spec.strip,
        Some(crate::model::binding::StripRule::WeakQuotes)
    ) {
        value = value.trim_start_matches("W/").trim_matches('"').to_owned();
    }
    if matches!(
        spec.transform,
        Some(crate::model::binding::TransformRule::RootUid)
    ) {
        value = value.split("::").next().unwrap_or(&value).to_owned();
    }
    Some(value)
}

fn extract_list(body: &Value, path: &str) -> Vec<String> {
    let mut current = vec![body];
    for seg in path.split('.') {
        let (attr, star) = match seg.strip_suffix("[*]") {
            Some(attr) => (attr, true),
            None => (seg, false),
        };
        let mut next = Vec::new();
        for v in current {
            let v = if attr.is_empty() {
                Some(v)
            } else {
                v.get(attr)
            };
            if let Some(v) = v {
                if star {
                    if let Some(items) = v.as_array() {
                        next.extend(items.iter());
                    }
                } else {
                    next.push(v);
                }
            }
        }
        current = next;
    }
    current
        .into_iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            other => other.as_str().map(ToOwned::to_owned),
        })
        .collect()
}

/// Replay one transcript entry against its case and judge whether the
/// runner reproduces the adjudicated verdict.
///
/// # Errors
/// Interpreter defects only (unknown case, unresolvable binding).
pub fn replay_entry(
    set: &crate::artifacts::ArtifactSet,
    entry: &TranscriptEntry,
) -> Result<(ExpectedVerdict, crate::exec::RowOutcome), String> {
    let case = set
        .cases
        .iter()
        .map(|(_, c)| c)
        .find(|c| c.id == entry.case)
        .ok_or_else(|| format!("transcript case {} is not in the catalogue", entry.case))?;
    // A transcript entry records ONE case×format×row sequence: slice the
    // case to that row so the recorded steps line up 1:1 with the flow.
    let mut sliced = case.clone();
    if let Some(parameters) = &mut sliced.parameters {
        if let Some(matrix) = &mut parameters.matrix {
            let row =
                matrix.rows.get(entry.row).cloned().ok_or_else(|| {
                    format!("case {} has no matrix row {}", entry.case, entry.row)
                })?;
            matrix.rows = vec![row];
        }
        if let Some(fixtures) = &mut parameters.fixture_set {
            let fixture = fixtures
                .get(entry.row)
                .cloned()
                .ok_or_else(|| format!("case {} has no fixture row {}", entry.case, entry.row))?;
            *fixtures = vec![fixture];
        }
    }
    let mut player = TranscriptPlayer::new(set, entry);
    let record = crate::exec::run_case(&sliced, entry.format, &mut player)?;
    let row = record
        .rows
        .first()
        .cloned()
        .ok_or_else(|| format!("case {} produced no row", entry.case))?;
    Ok((entry.expected_verdict, row))
}

/// Whether a produced row outcome reproduces the adjudicated verdict.
#[must_use]
pub fn verdict_matches(expected: ExpectedVerdict, produced: &crate::exec::RowOutcome) -> bool {
    matches!(
        (expected, produced),
        (ExpectedVerdict::Passed, crate::exec::RowOutcome::Passed)
            | (
                ExpectedVerdict::Failed,
                crate::exec::RowOutcome::Failed { .. }
            )
            | (
                ExpectedVerdict::Errored,
                crate::exec::RowOutcome::Errored { .. }
            )
            | (
                ExpectedVerdict::NotApplicable,
                crate::exec::RowOutcome::NotApplicable { .. }
            )
            | (
                ExpectedVerdict::Skipped,
                crate::exec::RowOutcome::Skipped { .. }
            )
    )
}
