// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The wire-surface coverage register (`vocab/wire_surface.yaml`) — the
//! authored, spec-cited record of the wire surface the CNF catalogue is
//! measured against for TOTAL coverage.
//!
//! Three sections, one per axis the `surface-coverage` gate
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
//! - `served_extensions` — Axis 4, the OUTWARD axis: the route families a SUT
//!   serves BEYOND the openEHR resource set. The first three axes are all
//!   spec-inward (what the spec defines and whether a case reaches it); this
//!   one runs the other way and is a **declaration, never an obligation** —
//!   every entry carries `never_gates: true` and NO coverage requirement is
//!   ever derived from it. It exists so an `SDoC` reader learns the extension
//!   surface exists instead of discovering it on the wire.
//!
//! Every enumeration source is a RELEASED spec component or the ITS-REST docs
//! text — NEVER the vendored OAS (owner ruling 2026-07-24). This artifact and
//! its gate never weaken
//! an expectation; a behaviour genuinely off the wire is recorded here with
//! its citation, never silently omitted.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use serde::{Deserialize, Serialize};

use crate::ids::{AmbiguityId, CaseId, SmOperationRef};
use crate::vocab::{FormatName, HttpMethod, OutcomeKind};

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
    pub const ALL: &[SurfaceReason] = &[
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

/// Axis 4: one family of routes the SUT serves OUTSIDE the openEHR resource
/// set — a **declaration**, never an obligation.
///
/// The released ITS-REST text defines a resource set
/// (`docs/overview/Resources.md` §Resources: "a **resource** is an instance
/// object of a specific openEHR class (type) that can be identified,
/// addressed, handled or managed by the service") and never constrains the URI
/// space around it: no released clause permits, forbids, or bounds additional
/// routes. So every entry here is spec-silent by construction — our own
/// design/extension — and `spec_silence` records the verified silence rather
/// than a permission that does not exist.
///
/// The axis is inert by design: the `surface-coverage` gate derives NO
/// obligation from it (no case is required, no branch is expected, no verdict
/// depends on it), which [`ServedExtension::never_gates`] states in every
/// entry and the gate's own tests pin.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServedExtension {
    /// A stable family name (the grouping an `SDoC` reader sees).
    pub family: String,
    /// The routes the family serves, each `"<METHOD> <path>"`, written as the
    /// server mounts them under the DEFAULT deployment (absolute from the
    /// server root, base path included — the same convention the served
    /// `OpenAPI` document uses).
    pub routes: Vec<String>,
    /// The configuration that mounts (or answers on) the family, verbatim —
    /// including the always-on case.
    pub config_gate: String,
    /// The verified released-text silence the family rests on: what the
    /// released text does and does not say about this URI space.
    pub spec_silence: String,
    /// Always `true`, and stated per entry rather than assumed: this axis is a
    /// declaration and MUST NOT be read as a coverage obligation.
    pub never_gates: bool,
}

impl ServedExtension {
    /// The path half of a declared route (`"GET /health"` → `"/health"`).
    /// `None` when the route is outside the grammar — which
    /// [`ServedExtension::check_invariants`] reports as its own finding.
    #[must_use]
    pub fn route_path(route: &str) -> Option<&str> {
        let (method, path) = route.split_once(' ')?;
        Self::method(method)?;
        path.starts_with('/').then_some(path)
    }

    /// The closed method vocabulary, resolved through the same serde tokens the
    /// binding layer uses (no second spelling of the method list).
    fn method(token: &str) -> Option<HttpMethod> {
        serde_json::from_value::<HttpMethod>(serde_json::Value::String(token.to_owned())).ok()
    }

    /// Shape invariants: a named family with at least one well-formed route, a
    /// stated gate and silence, and `never_gates` actually set. Returns one
    /// message per violated invariant (empty = sound).
    #[must_use]
    pub fn check_invariants(&self) -> Vec<String> {
        let mut findings = Vec::new();
        let label = if self.family.trim().is_empty() {
            findings.push("served_extensions entry has an empty family name".to_owned());
            "<unnamed>"
        } else {
            self.family.as_str()
        };
        if !self.never_gates {
            findings.push(format!(
                "served extension {label} sets never_gates: false — this axis is a declaration \
                 and can never carry a coverage obligation"
            ));
        }
        if self.routes.is_empty() {
            findings.push(format!("served extension {label} declares no routes"));
        }
        for route in &self.routes {
            if Self::route_path(route).is_none() {
                findings.push(format!(
                    "served extension {label} route {route:?} is outside the grammar \
                     (\"<METHOD> /<path>\", method from the closed HTTP-method vocabulary)"
                ));
            }
        }
        if self.config_gate.trim().is_empty() {
            findings.push(format!("served extension {label} states no config gate"));
        }
        if self.spec_silence.trim().is_empty() {
            findings.push(format!(
                "served extension {label} states no spec-silence citation"
            ));
        }
        findings
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
    /// Axis 4 — the outward declaration of the non-openEHR surface this SUT
    /// serves. Never a source of coverage obligations.
    #[serde(default)]
    pub served_extensions: Vec<ServedExtension>,
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
        let mut families = std::collections::BTreeSet::new();
        let mut routes = std::collections::BTreeSet::new();
        for extension in &self.served_extensions {
            findings.extend(extension.check_invariants());
            if !families.insert(extension.family.as_str()) {
                findings.push(format!(
                    "served_extensions family {} is declared twice",
                    extension.family
                ));
            }
            for route in &extension.routes {
                if !routes.insert(route.as_str()) {
                    findings.push(format!(
                        "served_extensions route {route:?} is declared by more than one family"
                    ));
                }
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

    fn served(never_gates: bool, routes: &[&str]) -> ServedExtension {
        serde_json::from_value(serde_json::json!({
            "family": "management",
            "routes": routes,
            "config_gate": "management.enabled",
            "spec_silence": "no released clause governs the URI space beyond the resource set",
            "never_gates": never_gates
        }))
        .unwrap()
    }

    #[test]
    fn served_extension_requires_never_gates_and_a_route_grammar() {
        let ok = served(true, &["GET /management/info"]);
        assert!(ok.check_invariants().is_empty());

        // never_gates: false is the one value the axis cannot carry — a
        // declaration may never become an obligation.
        let gating = served(false, &["GET /management/info"]);
        assert!(
            gating
                .check_invariants()
                .iter()
                .any(|m| m.contains("never_gates")),
            "{:?}",
            gating.check_invariants()
        );

        // Route grammar: "<METHOD> /<path>", method from the closed vocabulary.
        for bad in ["/management/info", "FETCH /management/info", "GET info"] {
            let e = served(true, &[bad]);
            assert!(
                e.check_invariants().iter().any(|m| m.contains("grammar")),
                "{bad} should be rejected"
            );
        }
        assert_eq!(
            ServedExtension::route_path("DELETE /admin/tenant/{tenant_id}"),
            Some("/admin/tenant/{tenant_id}")
        );
    }

    #[test]
    fn served_extension_families_and_routes_are_unique() {
        let surface: WireSurface = serde_json::from_value(serde_json::json!({
            "served_extensions": [
                { "family": "health", "routes": ["GET /health"], "config_gate": "always on",
                  "spec_silence": "s", "never_gates": true },
                { "family": "health", "routes": ["GET /health"], "config_gate": "always on",
                  "spec_silence": "s", "never_gates": true }
            ]
        }))
        .unwrap();
        let findings = surface.check_invariants().unwrap_err();
        assert!(findings.iter().any(|m| m.contains("declared twice")));
        assert!(findings.iter().any(|m| m.contains("more than one family")));
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
