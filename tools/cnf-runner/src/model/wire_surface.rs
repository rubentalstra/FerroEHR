//! The wire-surface coverage register (`vocab/wire_surface.yaml`) — the
//! authored, spec-cited record of the wire surface the CNF catalogue is
//! measured against for TOTAL coverage (issue #271; `.claude/rules/testing.md`
//! §CNF coverage). Three sections, one per axis the `surface-coverage` gate
//! (`crate::validate`) enforces:
//!
//! - `sm_operations` — Axis 1: SM operations of the platform interfaces that
//!   carry no `its-rest` binding, each an honest cited boundary (off the
//!   ITS-REST 1.1.0 wire, realized as another operation's variant, a
//!   navigation accessor, or a tracked coverage gap). Silence is not
//!   coverage: an SM operation with neither a binding nor an entry here is a
//!   finding.
//! - `branches` — Axis 2: per-binding outcome/format branches that no case
//!   exercises, each a cited exception.
//! - `elements` — Axis 3: the cross-cutting wire behaviours (conditional
//!   headers, content negotiation, the negotiation/error status families)
//!   mapped to the cases that exercise them, or an adjudicated exception.
//!
//! Every enumeration source is a RELEASED spec component or the ITS-REST docs
//! text — NEVER the vendored OAS (owner ruling 2026-07-24,
//! `.claude/rules/spec-adherence.md`). This artifact and its gate never weaken
//! an expectation; a behaviour genuinely off the wire is recorded here with
//! its citation, never silently omitted.

use serde::{Deserialize, Serialize};

use crate::ids::{AmbiguityId, CaseId, SmOperationRef};
use crate::vocab::{FormatName, OutcomeKind};

/// Why a spec-defined wire behaviour has no exercising case — the closed
/// disposition vocabulary of the coverage register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceReason {
    /// ITS-REST 1.1.0 surfaces no wire for this SM operation / branch (an
    /// off-wire boundary, cited to the released ITS-REST docs text / SM).
    OffWire,
    /// Realized on the wire as a variant or fold of another binding (the
    /// `note` names the realizing operation/variant).
    VariantOf,
    /// A sub-interface navigation accessor (return type is an interface), not
    /// a service operation with its own wire.
    Accessor,
    /// On the wire but not yet exercised by a case — a tracked NEW-CASE
    /// candidate. Coverage ratchets up: the entry is removed when a case lands.
    CoverageGap,
    /// The behaviour is party-statement-declared (an off-wire capability),
    /// not a normative case (links to a `statement_declared` register entry
    /// via `register`).
    StatementDeclared,
}

impl SurfaceReason {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [SurfaceReason] = &[
        SurfaceReason::OffWire,
        SurfaceReason::VariantOf,
        SurfaceReason::Accessor,
        SurfaceReason::CoverageGap,
        SurfaceReason::StatementDeclared,
    ];

    /// The stable token (`off_wire`, …).
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            SurfaceReason::OffWire => "off_wire",
            SurfaceReason::VariantOf => "variant_of",
            SurfaceReason::Accessor => "accessor",
            SurfaceReason::CoverageGap => "coverage_gap",
            SurfaceReason::StatementDeclared => "statement_declared",
        }
    }
}

/// Axis 1 exception: an SM operation of a pinned platform interface that
/// carries no `its-rest` binding, recorded with its citation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmOperationException {
    /// The SM operation with no binding.
    pub operation: SmOperationRef,
    /// Why it has no binding.
    pub reason: SurfaceReason,
    /// The released-spec / ITS-REST-docs citation for the boundary.
    pub source: String,
    /// Optional detail (e.g. the realizing operation for `variant_of`).
    #[serde(default)]
    pub note: Option<String>,
}

/// Axis 2 exception: a realized binding's outcome or format branch that no
/// case exercises, recorded with its citation. Exactly one of `outcome` /
/// `format` is set.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchException {
    /// The binding's SM operation.
    pub binding: SmOperationRef,
    /// The binding variant this exception scopes to (absent = the
    /// variant-less binding, or every variant of the operation).
    #[serde(default)]
    pub variant: Option<String>,
    /// The un-exercised outcome kind (xor `format`).
    #[serde(default)]
    pub outcome: Option<OutcomeKind>,
    /// The un-exercised format (xor `outcome`).
    #[serde(default)]
    pub format: Option<FormatName>,
    /// Why the branch is not exercised.
    pub reason: SurfaceReason,
    /// The released-spec / ITS-REST-docs citation.
    pub source: String,
    /// Optional detail.
    #[serde(default)]
    pub note: Option<String>,
}

impl BranchException {
    /// Shape invariant: exactly one of `outcome` / `format`.
    ///
    /// # Errors
    /// Returns a message naming the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        match (self.outcome, self.format) {
            (Some(_), None) | (None, Some(_)) => Ok(()),
            _ => Err(format!(
                "branch exception for {} must carry exactly one of outcome | format",
                self.binding
            )),
        }
    }

    /// Whether this exception matches a binding's `(operation, variant)`.
    #[must_use]
    pub fn scopes(&self, operation: &SmOperationRef, variant: Option<&str>) -> bool {
        &self.binding == operation && (self.variant.is_none() || self.variant.as_deref() == variant)
    }
}

/// The adjudicated exception on an Axis 3 wire-surface element (no exercising
/// case).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementException {
    /// Why no case exercises the element.
    pub reason: SurfaceReason,
    /// The ambiguity-register entry adjudicating the boundary, when the
    /// exception is grounded there (`statement_declared` / `off_wire` from a
    /// spec silence).
    #[serde(default)]
    pub register: Option<AmbiguityId>,
    /// Optional detail.
    #[serde(default)]
    pub note: Option<String>,
}

/// Axis 3: one cross-cutting wire-surface behaviour, mapped to the cases that
/// exercise it or an adjudicated exception.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireElement {
    /// A stable identifier for the behaviour.
    pub id: String,
    /// What the behaviour is.
    pub description: String,
    /// The ITS-REST-docs / released-spec citation defining the behaviour.
    pub source: String,
    /// The catalogue cases that exercise it (xor `exception`).
    #[serde(default)]
    pub covered_by: Vec<CaseId>,
    /// The adjudicated exception when no case exercises it (xor `covered_by`).
    #[serde(default)]
    pub exception: Option<ElementException>,
}

impl WireElement {
    /// Shape invariant: exactly one of a non-empty `covered_by` / `exception`.
    ///
    /// # Errors
    /// Returns a message naming the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        match (self.covered_by.is_empty(), &self.exception) {
            (false, None) | (true, Some(_)) => Ok(()),
            (false, Some(_)) => Err(format!(
                "wire-surface element {} carries both covered_by and an exception",
                self.id
            )),
            (true, None) => Err(format!(
                "wire-surface element {} has neither a covering case nor an exception",
                self.id
            )),
        }
    }
}

/// `vocab/wire_surface.yaml` — the whole coverage register.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireSurface {
    /// Axis 1 exceptions (SM operations with no binding).
    #[serde(default)]
    pub sm_operations: Vec<SmOperationException>,
    /// Axis 2 exceptions (per-binding outcome/format branches with no case).
    #[serde(default)]
    pub branches: Vec<BranchException>,
    /// Axis 3 cross-cutting wire-surface elements.
    #[serde(default)]
    pub elements: Vec<WireElement>,
}

impl WireSurface {
    /// The Axis 1 exception for an SM operation, if any.
    #[must_use]
    pub fn sm_exception(&self, operation: &SmOperationRef) -> Option<&SmOperationException> {
        self.sm_operations
            .iter()
            .find(|e| &e.operation == operation)
    }

    /// The Axis 2 exception for a binding's outcome branch, if any.
    #[must_use]
    pub fn outcome_exception(
        &self,
        operation: &SmOperationRef,
        variant: Option<&str>,
        outcome: OutcomeKind,
    ) -> Option<&BranchException> {
        self.branches
            .iter()
            .find(|b| b.outcome == Some(outcome) && b.scopes(operation, variant))
    }

    /// The Axis 2 exception for a binding's format branch, if any.
    #[must_use]
    pub fn format_exception(
        &self,
        operation: &SmOperationRef,
        variant: Option<&str>,
        format: FormatName,
    ) -> Option<&BranchException> {
        self.branches
            .iter()
            .find(|b| b.format == Some(format) && b.scopes(operation, variant))
    }

    /// Structural invariants (per-entry shapes + id uniqueness).
    ///
    /// # Errors
    /// Returns every violation.
    pub fn check_invariants(&self) -> Result<(), Vec<String>> {
        let mut findings = Vec::new();
        for branch in &self.branches {
            if let Err(message) = branch.check_invariants() {
                findings.push(message);
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for element in &self.elements {
            if let Err(message) = element.check_invariants() {
                findings.push(message);
            }
            if !seen.insert(element.id.as_str()) {
                findings.push(format!(
                    "wire-surface element id {} is not unique",
                    element.id
                ));
            }
        }
        if findings.is_empty() {
            Ok(())
        } else {
            Err(findings)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    #[test]
    fn branch_exactly_one_of_outcome_format() {
        let both: BranchException = serde_json::from_value(serde_json::json!({
            "binding": "I_EHR_SERVICE.create_ehr",
            "outcome": "created", "format": "canonical-xml",
            "reason": "coverage_gap", "source": "s"
        }))
        .unwrap();
        assert!(both.check_invariants().is_err());
        let one: BranchException = serde_json::from_value(serde_json::json!({
            "binding": "I_EHR_SERVICE.create_ehr",
            "format": "canonical-xml", "reason": "coverage_gap", "source": "s"
        }))
        .unwrap();
        assert!(one.check_invariants().is_ok());
    }

    #[test]
    fn element_xor_covered_by_exception() {
        let neither: WireElement = serde_json::from_value(serde_json::json!({
            "id": "x", "description": "d", "source": "s"
        }))
        .unwrap();
        assert!(neither.check_invariants().is_err());
        let covered: WireElement = serde_json::from_value(serde_json::json!({
            "id": "x", "description": "d", "source": "s", "covered_by": ["CASE-1"]
        }))
        .unwrap();
        assert!(covered.check_invariants().is_ok());
    }

    #[test]
    fn branch_scopes_variant() {
        let ex: BranchException = serde_json::from_value(serde_json::json!({
            "binding": "I_EHR_SERVICE.create_ehr", "variant": "with_ehr_id",
            "outcome": "already_exists", "reason": "coverage_gap", "source": "s"
        }))
        .unwrap();
        let op = SmOperationRef::parse("I_EHR_SERVICE.create_ehr").unwrap();
        assert!(ex.scopes(&op, Some("with_ehr_id")));
        assert!(!ex.scopes(&op, None));
        assert!(!ex.scopes(&op, Some("other")));
    }
}
