//! The declarative flow API (design v4 §2.4): multi-step cases as readable
//! given/when/expect sequences with step-numbered failure context.
//!
//! A [`Flow`] wraps the [`RunContext`], numbers every request it sends, and
//! stamps assertion failures with `step N/N (name)` — so a failing five-step
//! case reports *which* step diverged and what came before it, instead of a
//! bare assertion string. Suites migrate to this incrementally; the plain
//! [`crate::assert`] helpers remain the assertion vocabulary underneath.
//!
//! ```ignore
//! let mut flow = Flow::new(ctx);
//! let ehr = flow
//!     .send("create EHR", post("/ehr").json_body(json!({})))
//!     .await?
//!     .expect_status(201)?;
//! let ehr_id = ehr.json_pointer_str("/ehr_id/value")?;
//! flow.send("read it back", get(&format!("/ehr/{ehr_id}")))
//!     .await?
//!     .expect_status(200)?;
//! ```

use serde_json::Value;

use crate::engine::assert;
use crate::engine::harness::{CaseError, HttpRequest, HttpResponse, RunContext};

/// A numbered, named multi-step execution over the SUT transport.
#[derive(Debug)]
pub struct Flow<'a> {
    ctx: &'a RunContext<'a>,
    /// Names of the steps executed so far (1-based history).
    executed: Vec<String>,
}

impl<'a> Flow<'a> {
    /// Start a flow over `ctx`.
    #[must_use]
    pub fn new(ctx: &'a RunContext<'a>) -> Self {
        Flow {
            ctx,
            executed: Vec::new(),
        }
    }

    /// The wire format this flow runs under.
    #[must_use]
    pub fn format(&self) -> crate::case::Format {
        self.ctx.format
    }

    /// Execute one named step: send `request` and return the [`Step`] for
    /// assertions. Transport failures are stamped with the step context.
    ///
    /// # Errors
    /// [`CaseError::Transport`] (step-stamped) if the request cannot be sent.
    pub async fn send(&mut self, name: &str, request: HttpRequest) -> Result<Step, CaseError> {
        self.executed.push(name.to_owned());
        let number = self.executed.len();
        let response = self
            .ctx
            .send(request)
            .await
            .map_err(|e| stamp(e, number, name))?;
        Ok(Step {
            number,
            name: name.to_owned(),
            response,
        })
    }

    /// The executed step names, in order (diagnostic).
    #[must_use]
    pub fn executed(&self) -> &[String] {
        &self.executed
    }
}

/// One executed step: the response plus its step identity, with
/// `expect_*` assertions that stamp failures with the step context.
#[derive(Debug)]
pub struct Step {
    number: usize,
    name: String,
    /// The raw response (escape hatch for bespoke assertions).
    pub response: HttpResponse,
}

impl Step {
    /// Assert the exact status code.
    ///
    /// # Errors
    /// A step-stamped [`CaseError::Assertion`] on mismatch.
    pub fn expect_status(self, expected: u16) -> Result<Self, CaseError> {
        assert::status(&self.response, expected).map_err(|e| self.stamped(e))?;
        Ok(self)
    }

    /// Assert the status is one of `expected`.
    ///
    /// # Errors
    /// A step-stamped [`CaseError::Assertion`] on mismatch.
    pub fn expect_status_in(self, expected: &[u16]) -> Result<Self, CaseError> {
        assert::status_in(&self.response, expected).map_err(|e| self.stamped(e))?;
        Ok(self)
    }

    /// Assert a header is present (case-insensitive) and return its value.
    ///
    /// # Errors
    /// A step-stamped [`CaseError::Assertion`] if absent.
    pub fn expect_header(&self, name: &str) -> Result<String, CaseError> {
        assert::header_present(&self.response, name).map_err(|e| self.stamp_ref(e))?;
        Ok(self.response.header(name).unwrap_or_default().to_owned())
    }

    /// Parse the body as JSON.
    ///
    /// # Errors
    /// A step-stamped [`CaseError::Codec`] if the body is not JSON.
    pub fn json(&self) -> Result<Value, CaseError> {
        self.response.json().map_err(|e| self.stamp_ref(e))
    }

    /// Read a string at a JSON pointer (e.g. `"/ehr_id/value"`).
    ///
    /// # Errors
    /// A step-stamped [`CaseError::Assertion`] if the pointer is absent or
    /// not a string.
    pub fn json_pointer_str(&self, pointer: &str) -> Result<String, CaseError> {
        let body = self.json()?;
        body.pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                self.stamp_ref(CaseError::Assertion(format!(
                    "expected a string at {pointer}, body: {body}"
                )))
            })
    }

    /// The step's 1-based number.
    #[must_use]
    pub fn number(&self) -> usize {
        self.number
    }

    fn stamped(&self, e: CaseError) -> CaseError {
        stamp(e, self.number, &self.name)
    }

    fn stamp_ref(&self, e: CaseError) -> CaseError {
        stamp(e, self.number, &self.name)
    }
}

/// Prefix an error's message with the step context. A transport error keeps
/// its typed source (it is a runner/SUT fault, not an assertion) — the step
/// context wraps its message as an assertion-free codec-style rewrap would
/// lose the taxonomy, so it is left untouched apart from the step name being
/// recorded in [`Flow::executed`].
fn stamp(e: CaseError, step_no: usize, name: &str) -> CaseError {
    let prefix = format!("step {step_no} ({name}): ");
    match e {
        CaseError::Assertion(m) => CaseError::Assertion(format!("{prefix}{m}")),
        CaseError::Codec(m) => CaseError::Codec(format!("{prefix}{m}")),
        e @ (CaseError::Transport(_) | CaseError::Skipped(_)) => e,
    }
}
