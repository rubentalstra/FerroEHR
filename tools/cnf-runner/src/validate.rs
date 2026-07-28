//! Cross-artifact validation — the schedule's machine gates, generalized
//! from ECC's coverage-guard discipline: id uniqueness, SM-operation
//! resolution, spec-ref link checks, binding completeness, `verified_by`
//! resolution, corpus integrity, ambiguity/option resolution,
//! capability-vs-tier consistency, reference/sentinel grammar,
//! decision-table literals, and vocabulary drift.
//!
//! Every check is pure over the loaded [`ArtifactSet`] (+ the vendored spec
//! tree for the two resolution checks); every violation is one typed
//! [`Finding`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::artifacts::ArtifactSet;
use crate::ids::{CapabilityName, CaseId, CorpusKey, SmOperationRef, ViewName};
use crate::literal::{Literal, ViolationRef};
use crate::load::LoadError;
use crate::model::assertion::{Assertion, EquivalentTarget, assertion_refs};
use crate::model::binding::OperationBinding;
use crate::model::capability::Realization;
use crate::model::case::{CaseCore, ExpectSpec, FlowStep, MatrixCell, Parameters};
use crate::model::wire_surface::{ServedExtension, WireSurface};
use crate::refgrammar::{CaptureField, TimeExpr, ValueRef};
use crate::vocab::{CaseKind, CaseStatus, Disposition, FormatName, Iteration, OutcomeKind};

/// The check taxonomy (one id per machine gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckId {
    /// File-level load failure (YAML, schema, or typed parse).
    Load,
    /// Case-id uniqueness.
    IdUniqueness,
    /// Kind-conditional structure + assertion/parameter invariants.
    KindShape,
    /// `${…}` reference resolution + sentinel discipline.
    ReferenceGrammar,
    /// Decision-table literals + violation categories.
    LiteralGrammar,
    /// `sm_operation` (and flow `call:`) resolution against the vendored SM.
    SmOperation,
    /// `spec_refs` resolution against the vendored spec tree.
    SpecRef,
    /// Every used outcome kind mapped, every used capture wired, per binding.
    BindingCompleteness,
    /// A binding file's stem states the binding it holds:
    /// `<sm_operation>[-<variant>]`.
    BindingFilename,
    /// `verified_by` targets exist.
    VerifiedBy,
    /// Corpus keys/views/sources exist; entry invariants hold.
    CorpusIntegrity,
    /// `ambiguities` ids resolve in the register.
    AmbiguityLink,
    /// `option:` tags resolve to an `option_select` register entry.
    OptionTag,
    /// Capabilities exist in the matrix; profile tiers consistent.
    CapabilityTier,
    /// Published vocabulary files drift from the compiled enums.
    VocabDrift,
    /// Journey-catalogue invariants + the population-anchored envelope
    /// reconciliation of every performance workload (write share inside the
    /// 10:1..50:1 derivation band; stage templates resolve in the corpus).
    JourneyEnvelope,
    /// Claim completeness (issue #622): a capability a committed party
    /// statement claims has ≥ 1 verdict-bearing catalogue case, and a
    /// capability whose every case resolves excused/deselected names the
    /// register entry that adjudicated that. Declaring a capability IS the
    /// obligation to run the framework against it, so a hollow claim cannot
    /// even enter a run — the gate is at validate time, before any SUT is
    /// composed.
    ClaimCompleteness,
    /// Per-capability case-count floors (issue #622): one token case never
    /// certifies a capability. The capability matrix records each row's
    /// `min_cases`; a battery below its floor is a finding naming the
    /// shortfall. Floors ratchet UP only.
    CapabilityDepth,
    /// Measured-workload coverage (issue #622): every claimed capability is
    /// either exercised by the hospital-simulation journeys the performance
    /// workloads name, or carries a register-linked `workload_exclusion` the
    /// certificate renders. A bare `NO — catalogue gap` row is an undecided
    /// hole, never a publishable one.
    WorkloadCoverage,
    /// Realization scoping (issue #623): an `extension` binding drives a
    /// route no openEHR specification governs, so it is fenced off from every
    /// released-wire judgement — its family and path must resolve in the
    /// `served_extensions` axis, its adjudication must resolve in the
    /// register, and every capability its cases carry must be
    /// `realization: extension` in the matrix (never `required`). The reverse
    /// bites too: an `extension` matrix row whose cases drive released wire
    /// is mislabelled, and understates the conformance the product earned.
    RealizationScope,
    /// Total wire-surface coverage (issue #271): every spec-defined wire
    /// behaviour — SM operations (Axis 1), per-binding outcome/format branches
    /// (Axis 2), cross-cutting behaviours (Axis 3) — is exercised by ≥ 1 case
    /// or carries an adjudicated `vocab/wire_surface.yaml` exception. Silence
    /// is not coverage.
    SurfaceCoverage,
}

impl CheckId {
    /// Stable token for reports/tests.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::IdUniqueness => "id-uniqueness",
            Self::KindShape => "kind-shape",
            Self::ReferenceGrammar => "reference-grammar",
            Self::LiteralGrammar => "literal-grammar",
            Self::SmOperation => "sm-operation",
            Self::SpecRef => "spec-ref",
            Self::BindingCompleteness => "binding-completeness",
            Self::BindingFilename => "binding-filename",
            Self::VerifiedBy => "verified-by",
            Self::CorpusIntegrity => "corpus-integrity",
            Self::AmbiguityLink => "ambiguity-link",
            Self::OptionTag => "option-tag",
            Self::CapabilityTier => "capability-tier",
            Self::VocabDrift => "vocab-drift",
            Self::JourneyEnvelope => "journey-envelope",
            Self::ClaimCompleteness => "claim-completeness",
            Self::CapabilityDepth => "capability-depth",
            Self::WorkloadCoverage => "workload-coverage",
            Self::RealizationScope => "realization-scope",
            Self::SurfaceCoverage => "surface-coverage",
        }
    }
}

/// One violation.
#[derive(Debug, Clone)]
pub struct Finding {
    pub check: CheckId,
    /// The offending artifact (file path or case id).
    pub artifact: String,
    pub message: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.check.token(),
            self.artifact,
            self.message
        )
    }
}

/// Validation context: the artifact set + optional vendored-spec root
/// (`docs/specs/openehr`) enabling the two resolution checks.
#[derive(Debug)]
pub struct Context<'a> {
    pub set: &'a ArtifactSet,
    pub load_errors: &'a [LoadError],
    pub spec_root: Option<&'a Path>,
}

/// Run every gate; findings in check order.
#[must_use]
pub fn validate(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for e in ctx.load_errors {
        findings.push(Finding {
            check: CheckId::Load,
            artifact: e.path().display().to_string(),
            message: e.detail(),
        });
    }

    check_id_uniqueness(ctx.set, &mut findings);
    for (_path, case) in &ctx.set.cases {
        let who = case.id.to_string();
        check_kind_shape(case, &who, &mut findings);
        check_references(case, &who, ctx.set, &mut findings);
        check_literals(case, &who, &mut findings);
        check_capability_tier(case, &who, ctx.set, &mut findings);
        check_links(case, &who, ctx.set, &mut findings);
        if let Some(spec_root) = ctx.spec_root {
            check_sm_operations(case, &who, spec_root, &mut findings);
            check_spec_refs(case, &who, spec_root, &mut findings);
        }
    }
    check_binding_completeness(ctx.set, &mut findings);
    for (path, binding) in &ctx.set.bindings {
        let who = path.display().to_string();
        if let Err(message) = binding.check_invariants() {
            push(&mut findings, CheckId::KindShape, &who, message);
        }
        check_binding_filename(path, binding, &mut findings);
        if let (Some(decl), Some((_, register))) = (&binding.unrealized, &ctx.set.register)
            && register.get(&decl.ambiguity).is_none()
        {
            push(
                &mut findings,
                CheckId::AmbiguityLink,
                &who,
                format!(
                    "unrealized declaration cites {} which is not in the register",
                    decl.ambiguity
                ),
            );
        }
        if let Some(spec_root) = ctx.spec_root {
            resolve_sm_operation(&binding.sm_operation, &who, spec_root, &mut findings);
        }
    }
    check_corpus_integrity(ctx.set, &mut findings);
    check_vocab_drift(ctx.set, &mut findings);
    check_journey_envelope(ctx.set, &mut findings);
    check_claim_completeness(ctx.set, &mut findings);
    check_capability_depth(ctx.set, &mut findings);
    check_workload_coverage(ctx.set, &mut findings);
    check_realization_scope(ctx.set, &mut findings);
    check_surface_coverage(ctx.set, ctx.spec_root, &mut findings);

    findings
}

// ── claim completeness, depth floors, workload coverage (issue #622) ────────

/// How a catalogue case will resolve for verdict purposes, as far as the
/// CATALOGUE alone can say — the static twin of [`crate::verdict::Evidence`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    /// The case can carry executed evidence.
    Gating,
    /// Every operation the flow calls is `unrealized` on this ITS, so the
    /// runner records it not-applicable with the binding's citation.
    ExcusedUnrealized,
    /// The case realizes one branch of an `option_select` ambiguity: it
    /// carries evidence only for a party whose ICS declares that branch.
    OptionGated,
}

/// The catalogue-side resolution of one case (see [`Resolution`]).
fn resolution(set: &ArtifactSet, case: &CaseCore) -> Resolution {
    if crate::run::fully_unrealized(set, case).is_some() {
        Resolution::ExcusedUnrealized
    } else if case.option.is_some() {
        Resolution::OptionGated
    } else {
        Resolution::Gating
    }
}

/// Whether a `report_only` register entry suspends the case's gating (such a
/// case reports but never contributes to a verdict — [`crate::verdict`]).
fn suspended_report_only(set: &ArtifactSet, case: &CaseCore) -> bool {
    let Some((_, register)) = &set.register else {
        return false;
    };
    case.ambiguities.iter().any(|id| {
        register
            .get(id)
            .is_some_and(|e| e.disposition == Disposition::ReportOnly)
    })
}

/// The verdict-bearing cases of one capability: active cases naming it whose
/// gating is not suspended by a `report_only` register entry. This is the
/// count the depth floor measures and the set the claim gate requires to be
/// non-empty.
#[must_use]
pub fn verdict_bearing<'a>(set: &'a ArtifactSet, cap: &CapabilityName) -> Vec<&'a CaseCore> {
    set.cases
        .iter()
        .map(|(_, c)| c)
        .filter(|c| {
            c.status == CaseStatus::Active
                && c.capabilities.contains(cap)
                && !suspended_report_only(set, c)
        })
        .collect()
}

/// A claim without cases is a certification hole, and a capability whose
/// every case resolves excused/deselected is one too unless a register entry
/// says otherwise.
///
/// ISO/IEC 9646 test selection legitimizes "not applicable" only for a
/// capability the party does NOT claim; a claimed-but-unevidenced row is not
/// a selection outcome (owner directive 2026-07-28).
fn check_claim_completeness(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    let matrix_who = matrix_path.display().to_string();

    for (party_path, statement) in &set.parties {
        let who = party_path.display().to_string();
        for cap in &statement.claims.capabilities {
            // An unknown capability is the static review's / capability-tier
            // gate's finding, not this one.
            if matrix.get(cap).is_none() {
                continue;
            }
            if verdict_bearing(set, cap).is_empty() {
                push(
                    findings,
                    CheckId::ClaimCompleteness,
                    &who,
                    format!(
                        "claimed capability {cap} has zero verdict-bearing catalogue cases — \
                         declaring a capability is the obligation to run the CNF framework \
                         against it; author its battery or withdraw the claim"
                    ),
                );
            }
        }
    }

    for (name, entry) in matrix.entries() {
        let cases = verdict_bearing(set, name);
        let all_excused = !cases.is_empty()
            && cases
                .iter()
                .all(|c| resolution(set, c) != Resolution::Gating);
        match (&entry.evidence_exception, all_excused) {
            (None, true) => push(
                findings,
                CheckId::ClaimCompleteness,
                &matrix_who,
                format!(
                    "{name}: every one of its {} verdict-bearing case(s) resolves excused or \
                     deselected, so the capability can never carry executed evidence — name the \
                     adjudicating register entry in `evidence_exception`, realize the wire, or \
                     move the capability to the extension surface",
                    cases.len()
                ),
            ),
            (Some(adjudication), true) => {
                if set
                    .register
                    .as_ref()
                    .is_some_and(|(_, r)| r.get(&adjudication.register).is_none())
                {
                    push(
                        findings,
                        CheckId::ClaimCompleteness,
                        &matrix_who,
                        format!(
                            "{name}: evidence_exception cites {} which is not in the register",
                            adjudication.register
                        ),
                    );
                }
            }
            (Some(adjudication), false) => push(
                findings,
                CheckId::ClaimCompleteness,
                &matrix_who,
                format!(
                    "{name}: evidence_exception ({}) is stale — the capability now has cases \
                     that can carry executed evidence ({} of {}); delete the exception",
                    adjudication.register,
                    cases
                        .iter()
                        .filter(|c| resolution(set, c) == Resolution::Gating)
                        .count(),
                    cases.len()
                ),
            ),
            (None, false) => {}
        }
    }
}

/// One token case does not certify a capability: every matrix row records the
/// verdict-bearing case count its battery must keep (`min_cases`), and the
/// floors ratchet UP only.
fn check_capability_depth(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    let who = matrix_path.display().to_string();
    for (name, entry) in matrix.entries() {
        let count = verdict_bearing(set, name).len();
        if count < entry.min_cases {
            push(
                findings,
                CheckId::CapabilityDepth,
                &who,
                format!(
                    "{name}: {count} verdict-bearing case(s) against a floor of {} — short by \
                     {}; coverage only ratchets up, so restore the battery (never lower the \
                     floor)",
                    entry.min_cases,
                    entry.min_cases.saturating_sub(count)
                ),
            );
        }
    }
}

// ── realization scoping (issue #623) ────────────────────────────────────────

/// A route path with every `{parameter}` segment collapsed to `{}` — the shape
/// two artifacts can be compared on when each names its parameters locally.
fn path_shape(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether every operation the case's flow calls that resolves to a binding
/// at all is an EXTENSION realization — the extension twin of
/// [`crate::run::fully_unrealized`]. A case that touches even one released
/// operation is NOT an extension-only case: it earns released-wire evidence
/// too, so its capabilities may keep the released-wire marker.
fn drives_only_extension_bindings(set: &ArtifactSet, case: &CaseCore) -> bool {
    let mut saw_binding = false;
    for step in &case.flow {
        // A step with no resolvable binding is the binding-completeness gate's
        // finding, not this one — skip it rather than double-report.
        let Some(binding) = select_binding_for_step(set, case, step) else {
            continue;
        };
        saw_binding = true;
        if !binding.is_extension() {
            return false;
        }
    }
    saw_binding
}

/// An extension realization is fenced off from every released-wire judgement,
/// and the fence is structural rather than conventional:
///
/// 1. the binding's `family` + request path resolve in the `served_extensions`
///    axis, so a binding can only drive a route the SUT actually DECLARES;
/// 2. its adjudication resolves in the ambiguity register;
/// 3. a capability whose cases drive only extension bindings is
///    `realization: extension` in the matrix — and, since
///    `check_realization_scoping` forbids `required` there, no openEHR
///    profile tier can ever rest on it (owner ruling 2026-07-28, #610);
/// 4. the reverse: an `extension` row whose cases drive released wire is
///    mislabelled and understates what the product earned.
fn check_realization_scope(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    // (1) + (2): every extension binding declares a real, declared route.
    for (path, binding) in &set.bindings {
        let Some(decl) = &binding.extension else {
            continue;
        };
        let who = path.display().to_string();
        if set
            .register
            .as_ref()
            .is_some_and(|(_, r)| r.get(&decl.ambiguity).is_none())
        {
            push(
                findings,
                CheckId::RealizationScope,
                &who,
                format!(
                    "extension declaration cites {} which is not in the register",
                    decl.ambiguity
                ),
            );
        }
        let Some((_, wire_surface)) = &set.wire_surface else {
            continue;
        };
        let Some(family) = wire_surface
            .served_extensions
            .iter()
            .find(|e| e.family == decl.family)
        else {
            push(
                findings,
                CheckId::RealizationScope,
                &who,
                format!(
                    "extension family {:?} is not declared in the served_extensions axis of \
                     vocab/wire_surface.yaml — an extension binding may only drive a route the \
                     SUT declares outwardly",
                    decl.family
                ),
            );
            continue;
        };
        let Some(request) = binding.request.as_ref() else {
            continue; // the shape invariant already reported it
        };
        // Path-parameter NAMES are local to each artifact (the axis writes the
        // served `{uid_based_id}`, a binding writes the capture it fills the
        // segment from), so the comparison is on path SHAPE: every `{…}`
        // segment collapses to `{}`. The axis writes absolute
        // default-deployment paths and a binding path is base-relative, hence
        // the suffix match.
        let wanted = path_shape(request.path.raw());
        let declared = family.routes.iter().any(|route| {
            ServedExtension::route_path(route)
                .is_some_and(|declared| path_shape(declared).ends_with(wanted.as_str()))
        });
        if !declared {
            push(
                findings,
                CheckId::RealizationScope,
                &who,
                format!(
                    "request path {} is not one of the routes the {:?} served_extensions family \
                     declares — declare the route or bind a declared one",
                    request.path.raw(),
                    decl.family
                ),
            );
        }
    }

    // (3) + (4): the matrix marker matches what the capability's cases drive.
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    let matrix_who = matrix_path.display().to_string();
    for (name, entry) in matrix.entries() {
        let cases = verdict_bearing(set, name);
        if cases.is_empty() {
            continue; // claim-completeness / capability-depth own the empty row
        }
        let all_extension = cases
            .iter()
            .all(|case| drives_only_extension_bindings(set, case));
        match (entry.realization, all_extension) {
            (Realization::ReleasedWire, true) => push(
                findings,
                CheckId::RealizationScope,
                &matrix_who,
                format!(
                    "{name}: every one of its {} verdict-bearing case(s) drives EXTENSION \
                     routes only, so the row must carry `realization: extension` — a \
                     released-wire marker would claim openEHR wire conformance the release \
                     does not define",
                    cases.len()
                ),
            ),
            (Realization::Extension, false) => push(
                findings,
                CheckId::RealizationScope,
                &matrix_who,
                format!(
                    "{name}: `realization: extension` is stale — at least one of its \
                     verdict-bearing cases drives RELEASED ITS-REST operations; delete the \
                     marker so the row claims the conformance it earns"
                ),
            ),
            _ => {}
        }
    }
}

/// The capabilities the measured hospital simulation exercises: the union of
/// the capability sets of every operation of every journey a performance
/// workload names. The certificate's Workload Coverage table computes the
/// same union from the measurement records that actually ran; this is its
/// catalogue-side twin, so a gap is caught before a run, not after one.
fn workload_exercised(set: &ArtifactSet) -> BTreeSet<&'static str> {
    let mut exercised = BTreeSet::new();
    let Some((_, catalogue)) = &set.journeys else {
        return exercised;
    };
    for (_, case) in &set.performance {
        for (name, _) in &case.workload.journeys {
            let Some(journey) = catalogue.get(name) else {
                continue; // the journey-envelope gate reports the dangling name
            };
            for stage in &journey.stages {
                if let Ok(op) = crate::perf::PerfOp::parse(&stage.op) {
                    exercised.extend(op.capabilities().iter().copied());
                }
            }
        }
    }
    exercised
}

/// A claimed capability the measured workload never touches is either a
/// journey-catalogue gap to close or an adjudicated exclusion — never a bare
/// `NO — catalogue gap` row on a published certificate.
fn check_workload_coverage(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    if set.performance.is_empty() {
        return; // nothing is measured, so nothing can be excluded from it
    }
    let matrix_who = matrix_path.display().to_string();
    let exercised = workload_exercised(set);

    for (party_path, statement) in &set.parties {
        let who = party_path.display().to_string();
        for cap in &statement.claims.capabilities {
            let Some(entry) = matrix.get(cap) else {
                continue;
            };
            if exercised.contains(cap.as_str()) || entry.workload_exclusion.is_some() {
                continue;
            }
            push(
                findings,
                CheckId::WorkloadCoverage,
                &who,
                format!(
                    "claimed capability {cap} is neither exercised by the measured \
                     hospital-simulation workload nor carries a `workload_exclusion` — extend \
                     the journey catalogue or adjudicate the exclusion in the capability matrix"
                ),
            );
        }
    }

    for (name, entry) in matrix.entries() {
        let Some(adjudication) = &entry.workload_exclusion else {
            continue;
        };
        if set
            .register
            .as_ref()
            .is_some_and(|(_, r)| r.get(&adjudication.register).is_none())
        {
            push(
                findings,
                CheckId::WorkloadCoverage,
                &matrix_who,
                format!(
                    "{name}: workload_exclusion cites {} which is not in the register",
                    adjudication.register
                ),
            );
        }
        if exercised.contains(name.as_str()) {
            push(
                findings,
                CheckId::WorkloadCoverage,
                &matrix_who,
                format!(
                    "{name}: workload_exclusion ({}) is stale — the hospital simulation now \
                     exercises the capability; delete the exclusion",
                    adjudication.register
                ),
            );
        }
    }
}

/// The journey catalogue's own invariants, every performance workload's
/// envelope reconciliation through it, and the resolution of every stage
/// template (OPT + example payload) in the corpus manifest.
fn check_journey_envelope(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((path, catalogue)) = &set.journeys else {
        if !set.performance.is_empty() {
            push(
                findings,
                CheckId::JourneyEnvelope,
                "vocab/journey_catalogue.yaml",
                "performance cases exist but the journey catalogue is missing".to_owned(),
            );
        }
        return;
    };
    let who = path.display().to_string();
    if let Err(message) = catalogue.check_invariants() {
        push(findings, CheckId::JourneyEnvelope, &who, message);
        return;
    }
    // Every stage template resolves in the corpus manifest: the OPT entry
    // (the constraint carrier the seeder uploads) and its `.example`
    // sibling (the committed payload skeleton the driver commits).
    if let Some((_, manifest)) = &set.corpus {
        for (name, journey) in &catalogue.0 {
            for stage in &journey.stages {
                let Some(template) = &stage.template else {
                    continue;
                };
                for (key, role) in [
                    (template.clone(), "OPT"),
                    (format!("{template}.example"), "example payload"),
                ] {
                    match CorpusKey::parse(&key) {
                        Ok(parsed) if manifest.get(&parsed).is_some() => {}
                        _ => push(
                            findings,
                            CheckId::JourneyEnvelope,
                            &who,
                            format!(
                                "journey {name} template {template}: corpus manifest has no \
                                 {role} entry {key}"
                            ),
                        ),
                    }
                }
            }
        }
    }
    for (case_path, case) in &set.performance {
        if let Err(message) = catalogue.expansion(&case.workload.journeys) {
            push(
                findings,
                CheckId::JourneyEnvelope,
                &case_path.display().to_string(),
                message,
            );
        }
    }
}

fn push(findings: &mut Vec<Finding>, check: CheckId, artifact: &str, message: String) {
    findings.push(Finding {
        check,
        artifact: artifact.to_owned(),
        message,
    });
}

// ── id uniqueness ───────────────────────────────────────────────────────────

fn check_id_uniqueness(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (path, case) in &set.cases {
        if !seen.insert(case.id.as_str()) {
            push(
                findings,
                CheckId::IdUniqueness,
                &path.display().to_string(),
                format!("case id {} is declared more than once", case.id),
            );
        }
    }
}

// ── kind shape + structural invariants ──────────────────────────────────────

fn check_kind_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    check_kind_blocks(case, who, findings);
    check_parameters_shape(case, who, findings);
    check_assertion_shape(case, who, findings);
    check_flow_shape(case, who, findings);
    for guard in &case.guards {
        if !guard.contains('—') && !guard.to_lowercase().contains("master") {
            push(
                findings,
                CheckId::KindShape,
                who,
                format!("guard {guard:?} carries no spec citation"),
            );
        }
    }
}

fn check_kind_blocks(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    match case.kind {
        CaseKind::Functional => {
            if case.sm_operation.is_none() || case.flow.is_empty() {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "functional case must carry sm_operation and a non-empty flow".to_owned(),
                );
            }
            if case.decision_table.is_some() || case.constraint_context.is_some() {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "functional case must not carry content blocks".to_owned(),
                );
            }
        }
        CaseKind::Content => {
            if case.rm_class.is_none()
                || case.constraint_context.is_none()
                || case.decision_table.is_none()
            {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "content case must carry rm_class, constraint_context and decision_table"
                        .to_owned(),
                );
            }
        }
    }
}

fn check_parameters_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    if let Some(parameters) = &case.parameters {
        match (&parameters.matrix, &parameters.fixture_set) {
            (Some(_), Some(_)) | (None, None) => push(
                findings,
                CheckId::KindShape,
                who,
                "parameters must carry exactly one of matrix | fixture_set".to_owned(),
            ),
            _ => {}
        }
        if let Some(matrix) = &parameters.matrix {
            for (i, row) in matrix.rows.iter().enumerate() {
                if row.len() != matrix.columns.len() {
                    push(
                        findings,
                        CheckId::KindShape,
                        who,
                        format!(
                            "matrix row {i} has {} cells for {} columns",
                            row.len(),
                            matrix.columns.len()
                        ),
                    );
                }
            }
            let expected_col = matrix.columns.iter().position(|c| c == "expected");
            if let Some(col) = expected_col {
                for (i, row) in matrix.rows.iter().enumerate() {
                    match row.get(col) {
                        Some(MatrixCell::Literal(serde_json::Value::String(s)))
                            if OutcomeKind::from_token(s).is_some() => {}
                        _ => push(
                            findings,
                            CheckId::KindShape,
                            who,
                            format!("matrix row {i}: `expected` cell must be an outcome kind"),
                        ),
                    }
                }
            }
        }
    }
}

fn check_assertion_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    let aggregate_needs_single_pass = case
        .postconditions
        .iter()
        .chain(case.flow.iter().flat_map(|s| s.assertions.iter()))
        .any(Assertion::is_aggregate);
    if aggregate_needs_single_pass {
        let single_pass = matches!(
            case.parameters,
            Some(Parameters {
                iteration: Iteration::SinglePass,
                ..
            })
        );
        if !single_pass {
            push(
                findings,
                CheckId::KindShape,
                who,
                "aggregate assertions require parameters.iteration: single_pass".to_owned(),
            );
        }
    }

    for assertion in case
        .postconditions
        .iter()
        .chain(case.flow.iter().flat_map(|s| s.assertions.iter()))
    {
        if let Err(message) = assertion.check_invariants() {
            push(findings, CheckId::KindShape, who, message);
        }
        if let Assertion::State { verified_by, .. } = assertion {
            let verified = verified_by.is_some()
                || !case.verified_by.is_empty()
                || case.flow.len() > 1
                || case.flow.iter().any(|s| !s.assertions.is_empty());
            if !verified {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "state assertion needs verified_by or an in-case verification step".to_owned(),
                );
            }
        }
    }
}

fn check_flow_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    let mut last_step = 0_u32;
    for step in &case.flow {
        if step.step <= last_step {
            push(
                findings,
                CheckId::KindShape,
                who,
                format!(
                    "flow step numbers must strictly increase (step {})",
                    step.step
                ),
            );
        }
        last_step = step.step;
        if matches!(step.expect, ExpectSpec::FixtureExpected)
            && case
                .parameters
                .as_ref()
                .is_none_or(|p| p.fixture_set.is_none())
        {
            push(
                findings,
                CheckId::KindShape,
                who,
                "expect: ${fixture.expected} requires parameters.fixture_set".to_owned(),
            );
        }
        if let ExpectSpec::Kind(kind) = step.expect {
            for (name, source) in step.captures() {
                if source.outcome != kind {
                    push(
                        findings,
                        CheckId::KindShape,
                        who,
                        format!(
                            "capture {name} reads outcome `{}` but the step expects `{}`",
                            source.outcome.token(),
                            kind.token()
                        ),
                    );
                }
            }
        }
    }
}

// ── reference resolution ────────────────────────────────────────────────────

fn matrix_columns(case: &CaseCore) -> Vec<&str> {
    case.parameters
        .as_ref()
        .and_then(|p| p.matrix.as_ref())
        .map(|m| m.columns.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

struct RefCtx<'a> {
    columns: Vec<&'a str>,
    has_fixtures: bool,
    who: &'a str,
    set: &'a ArtifactSet,
}

fn check_one_ref(
    r: &ValueRef,
    ctx: &RefCtx<'_>,
    defined: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
) {
    let who = ctx.who;
    match r {
        ValueRef::Row(column) => {
            if !ctx.columns.contains(&column.as_str()) {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{row.{column}}} names no matrix column"),
                );
            }
        }
        ValueRef::Fixture(_) | ValueRef::FixtureDataSet => {
            if !ctx.has_fixtures {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    "${fixture.*} reference without parameters.fixture_set".to_owned(),
                );
            }
        }
        ValueRef::Capture { name, optional } => {
            if *optional {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{{name}?}} optional form is binding-template-only"),
                );
            } else if !defined.contains(name.as_str()) {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{{name}}} references no earlier capture or requires handle"),
                );
            }
        }
        ValueRef::DataSet { key, view } => {
            check_ds_ref(key, view.as_ref(), who, ctx.set, findings);
        }
        ValueRef::Recipe(name) => {
            let declared = ctx.set.corpus.as_ref().is_some_and(|(_, corpus)| {
                corpus.entries().iter().any(|(_, e)| {
                    e.recipes
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .any(|(n, _)| n == name)
                        || e.generated_by.as_ref().is_some_and(|g| g.recipe == *name)
                })
            });
            if !declared {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{recipe:{name}(row)}} is not declared in the corpus manifest"),
                );
            }
        }
        // A `${ixit:…}` fact is declared by the PARTY, not by the catalogue,
        // so there is nothing in the artifact tree to resolve it against —
        // the closed field set is enforced at parse. A party that declares
        // the fact runs the case; one that does not records it
        // not-applicable with that citation (crate::run).
        ValueRef::Ixit(_) => {}
        ValueRef::Time(expr) => {
            let (a, b) = match expr {
                TimeExpr::Before(t) | TimeExpr::After(t) => (t, None),
                TimeExpr::Between(t1, t2) => (t1, Some(t2)),
            };
            for t in std::iter::once(a).chain(b) {
                if !defined.contains(t.as_str()) {
                    push(
                        findings,
                        CheckId::ReferenceGrammar,
                        who,
                        format!("${{time:…({t})}} references no earlier capture"),
                    );
                }
            }
        }
    }
}

fn check_references(case: &CaseCore, who: &str, set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let ctx = RefCtx {
        columns: matrix_columns(case),
        has_fixtures: case
            .parameters
            .as_ref()
            .is_some_and(|p| p.fixture_set.is_some()),
        who,
        set,
    };
    let mut defined: BTreeSet<String> = case
        .requires
        .minted_handles()
        .iter()
        .map(ToString::to_string)
        .collect();

    for step in &case.flow {
        // `with` executes before the step's captures exist ...
        for (_, value) in step.with_entries() {
            for r in value.refs() {
                check_one_ref(r, &ctx, &defined, findings);
            }
        }
        // The SMART `scope` claim resolves on the same pre-step footing as
        // `with` (it is minted into the request's own Authorization header).
        for value in step.scope_templates() {
            for r in value.refs() {
                check_one_ref(r, &ctx, &defined, findings);
            }
        }
        for (name, _source) in step.captures() {
            defined.insert(name.to_string());
        }
        // ... while post-step assertions run after capture and may use them.
        for assertion in &step.assertions {
            for r in assertion_refs(assertion) {
                check_one_ref(&r, &ctx, &defined, findings);
            }
        }
    }
    for assertion in &case.postconditions {
        for r in assertion_refs(assertion) {
            check_one_ref(&r, &ctx, &defined, findings);
        }
        if let Assertion::Equivalent {
            to: EquivalentTarget::Committed,
            ..
        } = assertion
        {
            // `to: committed` needs something committed in-flow — a step that
            // carried a payload.
            if !case.flow.iter().any(|s| !s.with_entries().is_empty()) {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    "equivalent to: committed, but no flow step commits a payload".to_owned(),
                );
            }
        }
    }

    for key in case
        .data_sets
        .iter()
        .chain(case.requires.templates.iter())
        .chain(case.requires.commit.iter())
        .chain(case.constraint_context.as_ref().map(|c| &c.template))
    {
        check_ds_ref(key, None, who, set, findings);
    }
}

fn check_ds_ref(
    key: &CorpusKey,
    view: Option<&ViewName>,
    who: &str,
    set: &ArtifactSet,
    findings: &mut Vec<Finding>,
) {
    let Some((_, corpus)) = &set.corpus else {
        return;
    };
    match corpus.get(key) {
        None => push(
            findings,
            CheckId::CorpusIntegrity,
            who,
            format!("corpus key {key} is not in the manifest"),
        ),
        Some(entry) => {
            if let Some(view) = view
                && entry.view(view).is_none()
            {
                push(
                    findings,
                    CheckId::CorpusIntegrity,
                    who,
                    format!("view {view} is not declared on corpus entry {key}"),
                );
            }
        }
    }
}

// ── literals ────────────────────────────────────────────────────────────────

fn check_literals(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    let Some(table) = &case.decision_table else {
        return;
    };
    let violates_col = table.columns.iter().position(|c| c == "violates");
    let expected_col = table.columns.iter().position(|c| c == "expected");
    for (i, row) in table.rows.iter().enumerate() {
        for (j, cell) in row.iter().enumerate() {
            if Some(j) == violates_col {
                match cell {
                    serde_json::Value::Array(items) => {
                        for item in items {
                            match item.as_str().map(ViolationRef::parse) {
                                Some(Ok(_)) => {}
                                Some(Err(e)) => push(
                                    findings,
                                    CheckId::LiteralGrammar,
                                    who,
                                    format!("row {i}: {e}"),
                                ),
                                None => push(
                                    findings,
                                    CheckId::LiteralGrammar,
                                    who,
                                    format!("row {i}: violates entries must be strings"),
                                ),
                            }
                        }
                    }
                    _ => push(
                        findings,
                        CheckId::LiteralGrammar,
                        who,
                        format!("row {i}: violates cell must be a list"),
                    ),
                }
            } else if Some(j) == expected_col {
                if !matches!(cell.as_str(), Some("accepted" | "rejected")) {
                    push(
                        findings,
                        CheckId::LiteralGrammar,
                        who,
                        format!("row {i}: expected cell must be accepted | rejected"),
                    );
                }
            } else if let Err(e) = Literal::from_cell(cell) {
                push(
                    findings,
                    CheckId::LiteralGrammar,
                    who,
                    format!("row {i}: {e}"),
                );
            }
        }
    }
}

// ── capability / tier ───────────────────────────────────────────────────────

fn check_capability_tier(
    case: &CaseCore,
    who: &str,
    set: &ArtifactSet,
    findings: &mut Vec<Finding>,
) {
    let Some((_, matrix)) = &set.matrix else {
        return;
    };
    let mut tiers = BTreeSet::new();
    for capability in case.capabilities.iter().chain(case.exercises.iter()) {
        match matrix.get(capability) {
            None => push(
                findings,
                CheckId::CapabilityTier,
                who,
                format!("capability {capability} is not in the capability matrix"),
            ),
            Some(entry) => {
                if case.capabilities.contains(capability) {
                    tiers.insert(entry.tier);
                }
            }
        }
    }
    for tier in &case.profiles {
        if !tiers.contains(tier) {
            push(
                findings,
                CheckId::CapabilityTier,
                who,
                format!(
                    "profiles lists {tier:?} but no verdict-bearing capability carries that tier"
                ),
            );
        }
    }
    for tier in &tiers {
        if !case.profiles.contains(tier) {
            push(
                findings,
                CheckId::CapabilityTier,
                who,
                format!("capability tier {tier:?} is missing from profiles"),
            );
        }
    }
}

// ── register links ──────────────────────────────────────────────────────────

fn check_links(case: &CaseCore, who: &str, set: &ArtifactSet, findings: &mut Vec<Finding>) {
    if let Some((_, register)) = &set.register {
        for id in &case.ambiguities {
            if register.get(id).is_none() {
                push(
                    findings,
                    CheckId::AmbiguityLink,
                    who,
                    format!("{id} is not in the ambiguity register"),
                );
            }
        }
        if let Some(option) = &case.option
            && !register.declares_option(option)
        {
            push(
                findings,
                CheckId::OptionTag,
                who,
                format!("option tag {option} is not declared by any option_select register entry"),
            );
        }
    }
    let ids: BTreeSet<&CaseId> = set.cases.iter().map(|(_, c)| &c.id).collect();
    let assertion_targets = case
        .postconditions
        .iter()
        .chain(case.flow.iter().flat_map(|s| s.assertions.iter()))
        .filter_map(|a| match a {
            Assertion::State {
                verified_by: Some(target),
                ..
            } => Some(target),
            _ => None,
        });
    for target in case.verified_by.iter().chain(assertion_targets) {
        if !ids.contains(target) {
            push(
                findings,
                CheckId::VerifiedBy,
                who,
                format!("verified_by target {target} does not exist"),
            );
        }
    }
}

// ── SM + spec-ref resolution ────────────────────────────────────────────────

fn sm_class_file(spec_root: &Path, interface: &str) -> PathBuf {
    spec_root
        .join("SM/docs/UML/classes")
        .join(format!("{}.adoc", interface.to_lowercase()))
}

/// Resolve an operation reference against the vendored SM, or — for a
/// RELEASED ITS-REST operation the SM does not model — against the pinned
/// [`NON_SM_REST_OPERATIONS`] table. The pseudo-interface prefix is reserved:
/// an `I_ITS_REST_*` reference that is NOT pinned is a finding, so nobody can
/// invent an anchor to dodge SM resolution.
fn resolve_sm_operation(
    op: &SmOperationRef,
    who: &str,
    spec_root: &Path,
    findings: &mut Vec<Finding>,
) {
    if non_sm_operation_source(op).is_some() {
        return;
    }
    if op.interface().starts_with(PSEUDO_INTERFACE_PREFIX) {
        push(
            findings,
            CheckId::SmOperation,
            who,
            format!(
                "{op} uses the reserved {PSEUDO_INTERFACE_PREFIX}* pseudo-interface but is not \
                 pinned in the NON_SM_REST_OPERATIONS table (tools/cnf-runner/src/validate.rs) — \
                 a non-SM anchor exists only for a RELEASED ITS-REST operation the SM defines no \
                 interface for, and that table is the only place one is declared"
            ),
        );
        return;
    }
    let file = sm_class_file(spec_root, op.interface());
    match std::fs::read_to_string(&file) {
        Err(_) => push(
            findings,
            CheckId::SmOperation,
            who,
            format!(
                "interface {} has no vendored SM class export ({})",
                op.interface(),
                file.display()
            ),
        ),
        Ok(text) => {
            if !text.contains(&format!("|*{}*", op.operation())) {
                push(
                    findings,
                    CheckId::SmOperation,
                    who,
                    format!(
                        "operation {op} is not defined by the vendored SM interface ({})",
                        file.display()
                    ),
                );
            }
        }
    }
}

fn check_sm_operations(case: &CaseCore, who: &str, spec_root: &Path, findings: &mut Vec<Finding>) {
    let Some(anchor) = &case.sm_operation else {
        return;
    };
    resolve_sm_operation(anchor, who, spec_root, findings);
    for step in &case.flow {
        let op = if step.call.contains('.') {
            match SmOperationRef::parse(&step.call) {
                Ok(op) => op,
                Err(e) => {
                    push(findings, CheckId::SmOperation, who, e.to_string());
                    continue;
                }
            }
        } else {
            anchor.sibling(&step.call)
        };
        resolve_sm_operation(&op, who, spec_root, findings);
    }
}

/// Component token → vendored directory.
fn component_dir(token: &str) -> Option<&'static str> {
    Some(match token {
        "SM" => "SM",
        "CNF" => "CNF",
        "RM" => "RM",
        "BASE" => "BASE",
        "AM" => "AM",
        "QUERY" => "QUERY",
        "TERM" => "TERM",
        "LANG" => "LANG",
        "ITS-REST" => "ITS-REST",
        "ITS-JSON" => "ITS-JSON",
        "ITS-XML" => "ITS-XML",
        _ => return None,
    })
}

fn check_spec_refs(case: &CaseCore, who: &str, spec_root: &Path, findings: &mut Vec<Finding>) {
    for citation in &case.spec_refs {
        let mut tokens = citation.split_whitespace();
        let Some(component) = tokens.next() else {
            push(findings, CheckId::SpecRef, who, "empty spec_ref".to_owned());
            continue;
        };
        let Some(dir) = component_dir(component) else {
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!("{citation:?}: unknown component {component:?}"),
            );
            continue;
        };
        let root = spec_root.join(dir);
        if !root.is_dir() {
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!(
                    "{citation:?}: vendored component dir {} missing",
                    root.display()
                ),
            );
            continue;
        }
        // The document token: the first token before the § section marker.
        let doc_token = tokens.next().map(|t| t.trim_end_matches(',').to_owned());
        let Some(doc_token) = doc_token.filter(|t| !t.starts_with('§')) else {
            continue; // component-only citation: dir existence was the check
        };
        if !path_contains_token(&root, &doc_token.to_lowercase()) {
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!("{citation:?}: no vendored path under {dir} matches {doc_token:?}"),
            );
        }
    }
}

/// Case-insensitive substring match of `token` against any path under `root`.
fn path_contains_token(root: &Path, token: &str) -> bool {
    let mut stack = vec![root.to_owned()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let matches = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.to_lowercase().contains(token));
            if matches {
                return true;
            }
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    false
}

// ── binding completeness ────────────────────────────────────────────────────

/// Kinds a step may observe: the fixed expectation, every fixture-set
/// `expected` kind when per-fixture, plus any matrix `expected` column kinds.
fn step_observable_kinds(case: &CaseCore, step: &crate::model::case::FlowStep) -> Vec<OutcomeKind> {
    let mut kinds: Vec<OutcomeKind> = Vec::new();
    match step.expect {
        ExpectSpec::Kind(kind) => kinds.push(kind),
        ExpectSpec::FixtureExpected => {
            if let Some(fixtures) = case
                .parameters
                .as_ref()
                .and_then(|p| p.fixture_set.as_ref())
            {
                kinds.extend(fixtures.iter().map(|f| f.expected));
            }
        }
    }
    if let Some(matrix) = case.parameters.as_ref().and_then(|p| p.matrix.as_ref())
        && let Some(col) = matrix.columns.iter().position(|c| c == "expected")
    {
        kinds.extend(matrix.rows.iter().filter_map(|row| match row.get(col) {
            Some(MatrixCell::Literal(serde_json::Value::String(s))) => OutcomeKind::from_token(s),
            _ => None,
        }));
    }
    kinds
}

fn check_binding_completeness(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    for (_, case) in &set.cases {
        let Some(anchor) = &case.sm_operation else {
            continue;
        };
        let who = case.id.to_string();
        for step in &case.flow {
            let op = if step.call.contains('.') {
                match SmOperationRef::parse(&step.call) {
                    Ok(op) => op,
                    Err(_) => continue, // reported by the SM check
                }
            } else {
                anchor.sibling(&step.call)
            };
            let mut bindings: Vec<_> = set
                .bindings
                .iter()
                .filter(|(_, b)| b.sm_operation == op)
                .collect();
            if bindings.is_empty() {
                push(
                    findings,
                    CheckId::BindingCompleteness,
                    &who,
                    format!("no binding declares operation {op}"),
                );
                continue;
            }
            // Mirror the interpreter's binding selection (`binding_for_variant`):
            // a step's `variant` selects the binding declaring that variant;
            // a variant-less step (or a variant with no dedicated binding)
            // falls back to the variant-less binding. Completeness is judged
            // against the binding the interpreter would actually drive, not
            // against every binding of the operation.
            if let Some(v) = &step.variant
                && bindings
                    .iter()
                    .any(|(_, b)| b.variant.as_deref() == Some(v.as_str()))
            {
                bindings.retain(|(_, b)| b.variant.as_deref() == Some(v.as_str()));
            } else {
                let has_variantless = bindings.iter().any(|(_, b)| b.variant.is_none());
                if has_variantless {
                    bindings.retain(|(_, b)| b.variant.is_none());
                }
            }
            // An explicit `unrealized` declaration satisfies completeness:
            // the gap is machine-readable and the interpreter yields
            // not-applicable with its citation on that ITS.
            if bindings.iter().all(|(_, b)| b.is_unrealized()) {
                continue;
            }
            let kinds = step_observable_kinds(case, step);

            let universal: Vec<&str> = set
                .selectors
                .as_ref()
                .and_then(|(_, s)| s.universal_outcomes.as_deref())
                .unwrap_or_default()
                .iter()
                .map(|(k, _)| k.as_str())
                .collect();
            for (path, binding) in bindings {
                for kind in &kinds {
                    if universal.contains(&kind.token()) {
                        continue;
                    }
                    if binding.outcome(*kind).is_none() {
                        push(
                            findings,
                            CheckId::BindingCompleteness,
                            &who,
                            format!(
                                "outcome kind `{}` on {op} is not mapped by {}",
                                kind.token(),
                                path.display()
                            ),
                        );
                    }
                }
                for (name, source) in step.captures() {
                    if let CaptureField::Field { name: field, .. } = &source.field
                        && !binding.maps_capture(field)
                    {
                        push(
                            findings,
                            CheckId::BindingCompleteness,
                            &who,
                            format!(
                                "capture {name} needs wire source `{field}` on {op}, not mapped by {}",
                                path.display()
                            ),
                        );
                    }
                }
            }
        }
    }
}

// ── binding filename ↔ declared identity ────────────────────────────────────

/// The file name a binding's declared identity requires:
/// `<sm_operation>[-<variant>].yaml`.
///
/// Selection is by the DECLARED `sm_operation` + `variant`, so a disagreeing
/// file name misleads nobody at run time — and exactly for that reason it
/// drifts silently: a grep for `-<variant>` misses the file that declares it,
/// and a reader reasons about the wrong realization. The name is therefore
/// gated, not merely conventional.
fn expected_binding_stem(binding: &OperationBinding) -> String {
    match &binding.variant {
        Some(variant) => format!("{}-{variant}", binding.sm_operation),
        None => binding.sm_operation.to_string(),
    }
}

fn check_binding_filename(path: &Path, binding: &OperationBinding, findings: &mut Vec<Finding>) {
    let who = path.display().to_string();
    let expected = expected_binding_stem(binding);
    match path.file_stem().and_then(std::ffi::OsStr::to_str) {
        None => push(
            findings,
            CheckId::BindingFilename,
            &who,
            format!("binding file has no readable stem; expected {expected}.yaml"),
        ),
        Some(stem) if stem != expected => push(
            findings,
            CheckId::BindingFilename,
            &who,
            format!(
                "file stem {stem:?} disagrees with the declared identity \
                 (sm_operation {}{}) — rename the file to {expected}.yaml",
                binding.sm_operation,
                binding
                    .variant
                    .as_deref()
                    .map_or(String::new(), |v| format!(", variant {v:?}")),
            ),
        ),
        Some(_) => {}
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod binding_filename_tests {
    use super::*;

    fn binding(variant: Option<&str>) -> OperationBinding {
        let mut doc = serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr" },
            "outcomes": { "created": { "status": 201 } }
        });
        if let (Some(variant), Some(map)) = (variant, doc.as_object_mut()) {
            map.insert("variant".to_owned(), serde_json::json!(variant));
        }
        serde_json::from_value(doc).unwrap()
    }

    fn findings_for(file: &str, variant: Option<&str>) -> Vec<Finding> {
        let mut findings = Vec::new();
        check_binding_filename(
            &PathBuf::from("bindings/its-rest").join(file),
            &binding(variant),
            &mut findings,
        );
        findings
    }

    #[test]
    fn variant_less_binding_is_named_after_its_operation() {
        assert!(findings_for("I_EHR_SERVICE.create_ehr.yaml", None).is_empty());
        let findings = findings_for("create_ehr.yaml", None);
        assert_eq!(findings.len(), 1);
        let finding = findings.first().unwrap();
        assert_eq!(finding.check, CheckId::BindingFilename);
        assert!(
            finding
                .message
                .contains("rename the file to I_EHR_SERVICE.create_ehr.yaml"),
            "{}",
            finding.message
        );
    }

    #[test]
    fn variant_binding_carries_its_variant_in_the_stem() {
        assert!(
            findings_for(
                "I_EHR_SERVICE.create_ehr-with_ehr_id.yaml",
                Some("with_ehr_id")
            )
            .is_empty()
        );
        // The exact drift #558 caught: an abbreviated stem for a longer
        // declared variant.
        let findings = findings_for("I_EHR_SERVICE.create_ehr-with_id.yaml", Some("with_ehr_id"));
        assert_eq!(findings.len(), 1);
        let finding = findings.first().unwrap();
        assert!(finding.message.contains("with_id"), "{}", finding.message);
        assert!(
            finding
                .message
                .contains("rename the file to I_EHR_SERVICE.create_ehr-with_ehr_id.yaml"),
            "{}",
            finding.message
        );
        // A variant-less file for a variant-declaring binding is caught too.
        assert_eq!(
            findings_for("I_EHR_SERVICE.create_ehr.yaml", Some("with_ehr_id")).len(),
            1
        );
    }

    #[test]
    fn expected_stem_is_the_declared_identity() {
        assert_eq!(
            expected_binding_stem(&binding(None)),
            "I_EHR_SERVICE.create_ehr"
        );
        assert_eq!(
            expected_binding_stem(&binding(Some("wrong_media_type"))),
            "I_EHR_SERVICE.create_ehr-wrong_media_type"
        );
    }
}

// ── corpus integrity ────────────────────────────────────────────────────────

fn check_corpus_integrity(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((path, corpus)) = &set.corpus else {
        return;
    };
    let who = path.display().to_string();
    for (key, entry) in corpus.entries() {
        if let Err(message) = entry.check_invariants() {
            push(
                findings,
                CheckId::CorpusIntegrity,
                &who,
                format!("{key}: {message}"),
            );
        }
        if let (Some(source), Some(dir)) = (&entry.source, &set.corpus_dir) {
            let file = dir.join(source);
            if !file.is_file() {
                push(
                    findings,
                    CheckId::CorpusIntegrity,
                    &who,
                    format!("{key}: source {source} does not exist"),
                );
            }
        }
    }
}

// ── vocabulary drift ────────────────────────────────────────────────────────

fn check_vocab_drift(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    if let Some((path, outcomes)) = &set.outcomes
        && let Err(drift) = outcomes.check_against_enum()
    {
        for message in drift {
            push(
                findings,
                CheckId::VocabDrift,
                &path.display().to_string(),
                message,
            );
        }
    }
    if let Some((path, selectors)) = &set.selectors
        && let Err(drift) = selectors.check_against_enum()
    {
        for message in drift {
            push(
                findings,
                CheckId::VocabDrift,
                &path.display().to_string(),
                message,
            );
        }
    }
    if let Some((path, matrix)) = &set.matrix {
        let who = path.display().to_string();
        for message in matrix
            .check_tier_scoping()
            .err()
            .into_iter()
            .chain(matrix.check_realization_scoping().err())
            .flatten()
        {
            push(findings, CheckId::VocabDrift, &who, message);
        }
    }
    if let Some((path, register)) = &set.register {
        for (id, entry) in register.entries() {
            if let Err(message) = entry.check_invariants() {
                push(
                    findings,
                    CheckId::VocabDrift,
                    &path.display().to_string(),
                    format!("{id}: {message}"),
                );
            }
        }
    }
}

// ── surface coverage (issue #271) ───────────────────────────────────────────

/// The platform interfaces the CNF catalogue speaks — the Axis-1 SM-operation
/// enumeration domain. The set is the openEHR SM Platform Service Model's
/// platform interfaces (`docs/specs/openehr/SM/docs/UML/classes/`), which
/// anchor the operation identities the case cores use; it is NOT derived from
/// the vendored OAS (owner ruling 2026-07-24, `.claude/rules/spec-adherence.md`
/// — the OAS is `emit-rest` codegen input, never a surface source). Every
/// listed interface has a vendored `i_*.adoc` class export (a missing file is
/// itself a `surface-coverage` finding).
///
/// Interfaces are pinned even when the catalogue binds none of their operations
/// (`I_EHR_INDEX`, `I_TERMINOLOGY_SERVICE`, `I_VALIDITY_CHECKER`,
/// `I_SUBJECT_PROXY_SERVICE`, `I_DATA_BINDING`, `I_MESSAGE_SERVICE`,
/// `I_SYSTEM_LOG`): ITS-REST 1.1.0 surfaces no wire for those SM interfaces, so
/// their operations become explicit, individually cited `off_wire` entries in
/// `vocab/wire_surface.yaml` — a visible, ratchetable boundary, never a silent
/// omission. `I_ADMIN_ARCHIVE` / `I_ADMIN_DUMP_LOAD` are pinned alongside
/// `I_ADMIN_SERVICE` because the catalogue binds them (unrealized) as the SM
/// Admin surface. Sub-interface navigation accessors (return type an
/// interface — `i_ehr`, `i_party`, `i_party_relationship`) are not service
/// operations and are excluded by [`sm_interface_operations`].
///
/// This table is the SM half of the Axis-1 domain only. A RELEASED ITS-REST
/// operation the SM defines no interface for is invisible to it by
/// construction, so it is enumerated by the second, ITS-side table
/// [`NON_SM_REST_OPERATIONS`].
const PLATFORM_INTERFACES: &[&str] = &[
    "I_EHR_SERVICE",
    "I_EHR_STATUS",
    "I_EHR_COMPOSITION",
    "I_EHR_DIRECTORY",
    "I_EHR_CONTRIBUTION",
    "I_DEFINITION_ADL14",
    "I_DEFINITION_ADL2",
    "I_DEFINITION_QUERY",
    "I_QUERY_SERVICE",
    "I_DEMOGRAPHIC_SERVICE",
    "I_PARTY",
    "I_PARTY_RELATIONSHIP",
    "I_VALIDITY_CHECKER",
    "I_ADMIN_SERVICE",
    "I_ADMIN_ARCHIVE",
    "I_ADMIN_DUMP_LOAD",
    "I_EHR_INDEX",
    "I_TERMINOLOGY_SERVICE",
    "I_MESSAGE_SERVICE",
    "I_EHR_EXTRACT_SERVICE",
    "I_TDD_SERVICE",
    "I_SUBJECT_PROXY_SERVICE",
    "I_DATA_BINDING",
    "I_SYSTEM_LOG",
];

/// The reserved interface prefix a non-SM ITS-REST anchor uses. It is a
/// CATALOGUE naming convention and never a claim that the SM defines the
/// operation — the same distinction `AMB-127` draws for variant anchoring.
const PSEUDO_INTERFACE_PREFIX: &str = "I_ITS_REST_";

/// RELEASED ITS-REST operations with NO SM interface — pinned from the
/// released ITS-REST sources (docs text first; the released operation files
/// where the docs text is silent, per the oracle order). The catalogue anchors
/// each under a reserved `I_ITS_REST_*` pseudo-interface, ONE per released
/// resource family (a catalogue naming convention, never an SM claim — the
/// same convention AMB-127 pins for variant anchoring, adjudicated for this
/// table in AMB-161).
///
/// Four families are pinned today, and the operation name of every row is the
/// RELEASED `operationId` verbatim, so a row is traceable to exactly one
/// vendored `specifications/operations/<operationId>.yaml`. A family is the
/// SERVED RESOURCE, not the OAS tag — which is why the demographic
/// revision-history route sits with its EHR-side twins rather than with the
/// container read that shares its tag:
///
/// - `I_ITS_REST_SYSTEM` — the System API (ITS-REST
///   `docs/system/Description.md`, STABLE) defines `OPTIONS {base_path}`
///   (overview `Requests_and_responses.md` §HTTP Methods).
/// - `I_ITS_REST_ITEM_TAGS` — the 23 released ITEM_TAG routes: the two
///   space-wide lists, the EHR-side COMPOSITION/EHR_STATUS triples, and the
///   five demographic party triples. The SM models no tag concept at all —
///   `docs/specs/openehr/SM/docs/` contains zero occurrences of "tag"
///   (grep-verified) — while the released ITS-REST calls the
///   `openehr-item-tag` / `openehr-version-item-tag` headers "convenient
///   wrappers around the dedicated ITEM_TAG operations" (overview
///   `Requests_and_responses.md` §openehr-item-tag and
///   openehr-version-item-tag), so the operations are unambiguously part of
///   the released wire with no service-model anchor to name them by.
/// - `I_ITS_REST_REVISION_HISTORY` — the three released revision-history
///   reads (COMPOSITION, EHR_STATUS, PARTY). The SM declares no
///   revision-history operation on any interface —
///   `docs/specs/openehr/SM/docs/` contains zero occurrences of
///   "revision_history" (grep-verified); the abstract counterpart lives in
///   the RM (`common` `versioned_object.adoc` §Functions
///   `revision_history`), which is a model, not a service interface.
/// - `I_ITS_REST_VERSIONED_PARTY` — the VERSIONED_PARTY container read. Its
///   two EHR-side twins DO have SM anchors
///   (`I_EHR_COMPOSITION.get_versioned_composition`,
///   `I_EHR_STATUS.get_versioned_ehr_status`); `I_PARTY` declares no
///   container read at all, and SM
///   `docs/openehr_platform/master06-demographic_service.adoc` includes no
///   versioned-party interface (register AMB-136).
///
/// Without this table the Axis-1 enumeration is structurally blind to such an
/// operation: [`check_surface_sm_operations`] walks SM class exports, so a
/// wire behaviour the SM never models could never be reported missing.
const NON_SM_REST_OPERATIONS: &[(&str, &str)] = &[
    (
        "I_ITS_REST_SYSTEM.options",
        "ITS-REST docs/system/Description.md (STABLE System API) + overview \
         Requests_and_responses.md §HTTP Methods (OPTIONS)",
    ),
    // ── SMART on openEHR: the Platform's service-discovery document ──
    (
        "I_ITS_REST_SMART.discovery",
        "ITS-REST docs/smart_app_launch/master04-service_discovery.adoc §Service Discovery — \
         \"The configuration endpoint should be always available relative to the _Platform_ base \
         URL\", served as `application/json`, its `services` map carrying at minimum \
         `org.openehr.rest` with an absolute `baseUrl` (§Services). The SM models no Platform \
         interface (the Platform is \"a software ecosystem comprising at minimum an Authorization \
         Server, an openEHR Clinical Data Repository (CDR), and a FHIR Server\" — \
         master02-overview.adoc §Glossary — not an SM service), and SMART is the one API area of \
         the release with no OpenAPI group, so the operation is enumerable only from this table",
    ),
    // ── ITEM_TAG: the two space-wide lists ──
    (
        "I_ITS_REST_ITEM_TAGS.ehr_tags_get",
        "ITS-REST specifications/operations/ehr_tags_get.yaml — GET \
         /ehr/{ehr_id}/tags, the EHR-scoped list of \"the ITEM_TAG resources \
         associated with any target VERSION or VERSIONED_OBJECT within the EHR \
         identified by ehr_id\"",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.demographic_tags_get",
        "ITS-REST specifications/operations/demographic_tags_get.yaml — GET \
         /demographic/tags, the space-wide list of \"the ITEM_TAG resources \
         associated with any target VERSION or VERSIONED_PARTY within the \
         Demographic space\" (its unbounded scope is adjudicated in AMB-138)",
    ),
    // ── ITEM_TAG: the EHR-side typed families ──
    (
        "I_ITS_REST_ITEM_TAGS.composition_tags_get",
        "ITS-REST specifications/operations/composition_tags_get.yaml — GET \
         /ehr/{ehr_id}/composition/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.composition_tags_update",
        "ITS-REST specifications/operations/composition_tags_update.yaml — PUT \
         /ehr/{ehr_id}/composition/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.composition_tags_delete",
        "ITS-REST specifications/operations/composition_tags_delete.yaml — \
         DELETE /ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.ehr_status_tags_get",
        "ITS-REST specifications/operations/ehr_status_tags_get.yaml — GET \
         /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.ehr_status_tags_update",
        "ITS-REST specifications/operations/ehr_status_tags_update.yaml — PUT \
         /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.ehr_status_tags_delete",
        "ITS-REST specifications/operations/ehr_status_tags_delete.yaml — \
         DELETE /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}",
    ),
    // ── ITEM_TAG: the five demographic party families ──
    (
        "I_ITS_REST_ITEM_TAGS.person_tags_get",
        "ITS-REST specifications/operations/person_tags_get.yaml — GET \
         /demographic/person/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.person_tags_update",
        "ITS-REST specifications/operations/person_tags_update.yaml — PUT \
         /demographic/person/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.person_tags_delete",
        "ITS-REST specifications/operations/person_tags_delete.yaml — DELETE \
         /demographic/person/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.agent_tags_get",
        "ITS-REST specifications/operations/agent_tags_get.yaml — GET \
         /demographic/agent/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.agent_tags_update",
        "ITS-REST specifications/operations/agent_tags_update.yaml — PUT \
         /demographic/agent/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.agent_tags_delete",
        "ITS-REST specifications/operations/agent_tags_delete.yaml — DELETE \
         /demographic/agent/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.group_tags_get",
        "ITS-REST specifications/operations/group_tags_get.yaml — GET \
         /demographic/group/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.group_tags_update",
        "ITS-REST specifications/operations/group_tags_update.yaml — PUT \
         /demographic/group/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.group_tags_delete",
        "ITS-REST specifications/operations/group_tags_delete.yaml — DELETE \
         /demographic/group/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.organisation_tags_get",
        "ITS-REST specifications/operations/organisation_tags_get.yaml — GET \
         /demographic/organisation/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.organisation_tags_update",
        "ITS-REST specifications/operations/organisation_tags_update.yaml — PUT \
         /demographic/organisation/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.organisation_tags_delete",
        "ITS-REST specifications/operations/organisation_tags_delete.yaml — \
         DELETE /demographic/organisation/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.role_tags_get",
        "ITS-REST specifications/operations/role_tags_get.yaml — GET \
         /demographic/role/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.role_tags_update",
        "ITS-REST specifications/operations/role_tags_update.yaml — PUT \
         /demographic/role/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.role_tags_delete",
        "ITS-REST specifications/operations/role_tags_delete.yaml — DELETE \
         /demographic/role/{uid_based_id}/tags/{key}",
    ),
    // ── REVISION_HISTORY: the three released revision-history reads ──
    (
        "I_ITS_REST_REVISION_HISTORY.versioned_composition_revision_history",
        "ITS-REST specifications/operations/versioned_composition_revision_history.yaml \
         — GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history, \
         \"Retrieves revision history of the VERSIONED_COMPOSITION identified by \
         `versioned_object_uid` and associated with the EHR identified by `ehr_id`\"",
    ),
    (
        "I_ITS_REST_REVISION_HISTORY.versioned_ehr_status_revision_history",
        "ITS-REST specifications/operations/versioned_ehr_status_revision_history.yaml \
         — GET /ehr/{ehr_id}/versioned_ehr_status/revision_history, \"Retrieves \
         revision history of the VERSIONED_EHR_STATUS associated with the EHR \
         identified by `ehr_id`\"",
    ),
    (
        "I_ITS_REST_REVISION_HISTORY.versioned_party_revision_history",
        "ITS-REST specifications/operations/versioned_party_revision_history.yaml — \
         GET /demographic/versioned_party/{versioned_object_uid}/revision_history, \
         \"Retrieves revision history of the VERSIONED_PARTY identified by \
         `versioned_object_uid`\"",
    ),
    // ── VERSIONED_PARTY: the container read the SM models no interface for ──
    (
        "I_ITS_REST_VERSIONED_PARTY.versioned_party_get",
        "ITS-REST specifications/operations/versioned_party_get.yaml — GET \
         /demographic/versioned_party/{versioned_object_uid}, \"Retrieves a \
         VERSIONED_PARTY identified by `versioned_object_uid`\" (register AMB-136: \
         I_PARTY declares no container read)",
    ),
];

/// The pinned citation of a non-SM ITS-REST operation, or `None` when the
/// reference is not in [`NON_SM_REST_OPERATIONS`].
fn non_sm_operation_source(op: &SmOperationRef) -> Option<&'static str> {
    let reference = op.to_string();
    NON_SM_REST_OPERATIONS
        .iter()
        .find(|(name, _)| *name == reference)
        .map(|(_, source)| *source)
}

/// Parse the service operations of an SM interface from its vendored UML class
/// export — the same table shape [`resolve_sm_operation`] resolves against.
/// Operation rows are `|*<name>* (` (a lower-snake signature name followed by
/// its parameter list); sub-interface navigation accessors (`i_*`) are
/// excluded (they return an interface, not a service result).
///
/// This reads the SM only. An ITS-REST operation with no SM interface has no
/// class export to parse and is enumerated from [`NON_SM_REST_OPERATIONS`]
/// instead — never through this function.
///
/// # Errors
/// Returns a message when the interface has no vendored class export.
fn sm_interface_operations(spec_root: &Path, interface: &str) -> Result<Vec<String>, String> {
    let file = sm_class_file(spec_root, interface);
    let text = std::fs::read_to_string(&file).map_err(|_| {
        format!(
            "interface {interface} has no vendored SM class export ({})",
            file.display()
        )
    })?;
    let mut ops: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("|*") else {
            continue;
        };
        let Some((name, tail)) = rest.split_once("* ") else {
            continue;
        };
        if !tail.starts_with('(') || name.starts_with("i_") {
            continue;
        }
        let lower_snake = name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if lower_snake && !ops.iter().any(|o| o == name) {
            ops.push(name.to_owned());
        }
    }
    Ok(ops)
}

/// The three coverage axes (Axis 1 and the Axis-3 section derivation need the
/// vendored spec tree; the rest is pure over the artifact set). An absent
/// `wire_surface.yaml` is treated as an empty register — so every gap surfaces
/// as a finding rather than passing silently.
fn check_surface_coverage(
    set: &ArtifactSet,
    spec_root: Option<&Path>,
    findings: &mut Vec<Finding>,
) {
    let empty = WireSurface::default();
    let wire_surface = set.wire_surface.as_ref().map_or(&empty, |(_, w)| w);
    if let Some(spec_root) = spec_root {
        check_surface_sm_operations(set, spec_root, wire_surface, findings);
        check_axis3_section_derivation(wire_surface, spec_root, AXIS3_SECTION_EXCLUSIONS, findings);
    }
    check_binding_branch_coverage(set, wire_surface, findings);
    check_wire_surface_elements(set, wire_surface, findings);
    check_served_extensions(set, wire_surface, findings);
}

/// Axis 4 — the outward declaration is well-formed and does not claim a route
/// the RELEASED wire already defines.
///
/// This check reads the axis; it never derives an obligation FROM it. Nothing
/// here can require a case, expect a branch, or move a verdict — the axis is a
/// declaration, and `never_gates: true` on every entry (shape-checked by
/// [`WireSurface::check_invariants`], which
/// [`check_wire_surface_elements`] reports) states that in the artifact
/// itself.
///
/// The one cross-artifact check that IS meaningful: a family must not declare
/// a route whose path is a realized ITS-REST binding's path, which would
/// mislabel a released operation as our own extension. The axis writes
/// absolute default-deployment paths while a binding path is relative to the
/// API base, so the comparison strips a leading MOUNT prefix — and a prefix
/// only counts as a mount when its last segment is not itself a released
/// first segment. That distinction is what separates
/// `/…/v1` + `/ehr` (the same released operation, re-declared) from
/// `/…/v1/admin` + `/query/{q}/{v}` (a different resource that merely ends in
/// the same tail).
fn check_served_extensions(
    set: &ArtifactSet,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    // An `extension` binding realizes its operation over one of THESE routes
    // (the `realization-scope` gate proves the family/path pairing), so its
    // path is by construction not a released one — including it here would
    // make every extension family collide with its own bindings.
    let released: Vec<&str> = set
        .bindings
        .iter()
        .filter(|(_, b)| !b.is_unrealized() && !b.is_extension())
        .filter_map(|(_, b)| b.request.as_ref())
        .map(|r| r.path.raw())
        .collect();
    let released_roots: BTreeSet<&str> = released
        .iter()
        .filter_map(|p| p.trim_start_matches('/').split('/').next())
        .collect();
    for extension in &wire_surface.served_extensions {
        for route in &extension.routes {
            let Some(path) = ServedExtension::route_path(route) else {
                continue; // shape finding already raised by check_invariants
            };
            for binding_path in &released {
                let claims = path == *binding_path
                    || path.strip_suffix(*binding_path).is_some_and(|mount| {
                        !mount.is_empty()
                            && !mount.ends_with('/')
                            && mount
                                .rsplit('/')
                                .next()
                                .is_some_and(|last| !released_roots.contains(last))
                    });
                if claims {
                    push(
                        findings,
                        CheckId::SurfaceCoverage,
                        &extension.family,
                        format!(
                            "served_extensions route {route:?} claims the released ITS-REST path \
                             {binding_path} — an extension family may not declare an operation \
                             the release defines"
                        ),
                    );
                }
            }
        }
    }
}

/// Axis 1 — every SM operation of a pinned platform interface, PLUS every
/// pinned non-SM RELEASED ITS-REST operation ([`NON_SM_REST_OPERATIONS`]), has
/// an `its-rest` binding (realized or unrealized) or a cited `sm_operations`
/// exception.
fn check_surface_sm_operations(
    set: &ArtifactSet,
    spec_root: &Path,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    for interface in PLATFORM_INTERFACES {
        let ops = match sm_interface_operations(spec_root, interface) {
            Ok(ops) => ops,
            Err(message) => {
                push(findings, CheckId::SurfaceCoverage, interface, message);
                continue;
            }
        };
        for name in ops {
            let Ok(op) = SmOperationRef::parse(&format!("{interface}.{name}")) else {
                continue;
            };
            let bound = set.bindings.iter().any(|(_, b)| b.sm_operation == op);
            if bound || wire_surface.sm_exception(&op).is_some() {
                continue;
            }
            push(
                findings,
                CheckId::SurfaceCoverage,
                &op.to_string(),
                "SM operation has no its-rest binding and no wire_surface.yaml sm_operations \
                 exception — add a binding (realized or unrealized) or a cited \
                 off_wire/variant_of/coverage_gap entry"
                    .to_owned(),
            );
        }
    }
    // The ITS-side half of the domain: a RELEASED ITS-REST operation the SM
    // models no interface for is enumerated from the pinned table, so it is
    // held to exactly the same obligation as an SM operation.
    for (name, source) in NON_SM_REST_OPERATIONS {
        let Ok(op) = SmOperationRef::parse(name) else {
            push(
                findings,
                CheckId::SurfaceCoverage,
                name,
                "NON_SM_REST_OPERATIONS entry is not a parsable operation reference".to_owned(),
            );
            continue;
        };
        let bound = set.bindings.iter().any(|(_, b)| b.sm_operation == op);
        if bound || wire_surface.sm_exception(&op).is_some() {
            continue;
        }
        push(
            findings,
            CheckId::SurfaceCoverage,
            &op.to_string(),
            format!(
                "non-SM ITS-REST operation ({source}) has no its-rest binding and no \
                 wire_surface.yaml sm_operations exception — add a binding (realized or \
                 unrealized) or a cited off_wire/variant_of/coverage_gap entry"
            ),
        );
    }
    // Ratchet: an sm_operations exception for an operation that now HAS a
    // binding is stale and must be removed (coverage only ratchets up).
    for ex in &wire_surface.sm_operations {
        if set
            .bindings
            .iter()
            .any(|(_, b)| b.sm_operation == ex.operation)
        {
            push(
                findings,
                CheckId::SurfaceCoverage,
                &ex.operation.to_string(),
                "wire_surface.yaml sm_operations exception is redundant — the operation now has \
                 an its-rest binding; remove the exception"
                    .to_owned(),
            );
        }
    }
}

/// The `(operation, variant)` key identifying a binding realization.
type BranchKey = (SmOperationRef, Option<String>);

/// Mirror the interpreter's binding selection (`exec::driver::binding_for_variant`):
/// a step's `variant` selects the binding declaring it, else the variant-less
/// binding for the operation.
fn select_binding_for_step<'a>(
    set: &'a ArtifactSet,
    case: &CaseCore,
    step: &FlowStep,
) -> Option<&'a OperationBinding> {
    let op = if step.call.contains('.') {
        SmOperationRef::parse(&step.call).ok()?
    } else {
        case.sm_operation.as_ref()?.sibling(&step.call)
    };
    if let Some(v) = &step.variant
        && let Some((_, b)) = set
            .bindings
            .iter()
            .find(|(_, b)| b.sm_operation == op && b.variant.as_deref() == Some(v.as_str()))
    {
        return Some(b);
    }
    set.bindings
        .iter()
        .find(|(_, b)| b.sm_operation == op && b.variant.is_none())
        .map(|(_, b)| b)
}

/// The effective wire format a step exercises: its explicit `format`, else the
/// case's format axis, else the canonical-JSON default the driver falls back to
/// when neither is set (`exec::driver` sets no `Content-Type`/`Accept`).
fn step_format(case: &CaseCore, step: &FlowStep) -> FormatName {
    step.format
        .or_else(|| case.formats.first().copied())
        .unwrap_or(FormatName::CanonicalJson)
}

/// Compute, per realized binding, the outcome kinds and formats the catalogue
/// exercises — the inverse of `check_binding_completeness`.
fn exercised_branches(
    set: &ArtifactSet,
) -> BTreeMap<BranchKey, (BTreeSet<OutcomeKind>, BTreeSet<FormatName>)> {
    let mut map: BTreeMap<BranchKey, (BTreeSet<OutcomeKind>, BTreeSet<FormatName>)> =
        BTreeMap::new();
    for (_, case) in &set.cases {
        for step in &case.flow {
            let Some(binding) = select_binding_for_step(set, case, step) else {
                continue;
            };
            if binding.is_unrealized() {
                continue;
            }
            let key = (binding.sm_operation.clone(), binding.variant.clone());
            let entry = map.entry(key).or_default();
            for kind in step_observable_kinds(case, step) {
                entry.0.insert(kind);
            }
            entry.1.insert(step_format(case, step));
        }
    }
    map
}

/// A binding's label for findings/report (`I_X.op` or `I_X.op#variant`).
fn binding_label(binding: &OperationBinding) -> String {
    match &binding.variant {
        Some(v) => format!("{}#{v}", binding.sm_operation),
        None => binding.sm_operation.to_string(),
    }
}

/// The published token of a format (matches `vocab/wire_surface.yaml`).
fn format_token(format: FormatName) -> &'static str {
    match format {
        FormatName::CanonicalJson => "canonical-json",
        FormatName::CanonicalXml => "canonical-xml",
        FormatName::WtFlat => "wt-flat",
        FormatName::WtStructured => "wt-structured",
        FormatName::Wt => "wt",
    }
}

/// The route-table-wide outcome tokens (mapped once in `vocab/selectors.yaml`,
/// exempt from per-binding coverage).
fn universal_outcome_tokens(set: &ArtifactSet) -> Vec<&str> {
    set.selectors
        .as_ref()
        .and_then(|(_, s)| s.universal_outcomes.as_deref())
        .unwrap_or_default()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect()
}

/// Axis 2 — every realized binding's declared outcome key and format is
/// exercised by ≥ 1 case step or carries a cited `branches` exception (universal
/// outcomes exempt).
fn check_binding_branch_coverage(
    set: &ArtifactSet,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    let exercised = exercised_branches(set);
    let universal = universal_outcome_tokens(set);
    for (_, binding) in &set.bindings {
        if binding.is_unrealized() {
            continue;
        }
        let variant = binding.variant.as_deref();
        let key = (binding.sm_operation.clone(), binding.variant.clone());
        let done = exercised.get(&key);
        let outcomes_done = done.map(|d| &d.0);
        let formats_done = done.map(|d| &d.1);
        let who = binding_label(binding);
        for (okey, _) in binding.outcomes.as_deref().unwrap_or_default() {
            let kind = okey.0;
            if universal.contains(&kind.token())
                || outcomes_done.is_some_and(|s| s.contains(&kind))
                || wire_surface
                    .outcome_exception(&binding.sm_operation, variant, kind)
                    .is_some()
            {
                continue;
            }
            push(
                findings,
                CheckId::SurfaceCoverage,
                &who,
                format!(
                    "outcome `{}` is declared by the binding but no case exercises it and no \
                     wire_surface.yaml branch exception covers it",
                    kind.token()
                ),
            );
        }
        for format in &binding.formats {
            if formats_done.is_some_and(|s| s.contains(format))
                || wire_surface
                    .format_exception(&binding.sm_operation, variant, *format)
                    .is_some()
            {
                continue;
            }
            push(
                findings,
                CheckId::SurfaceCoverage,
                &who,
                format!(
                    "format `{}` is declared by the binding but no case exercises it and no \
                     wire_surface.yaml branch exception covers it",
                    format_token(*format)
                ),
            );
        }
    }
}

/// Axis 3 — every cross-cutting wire-surface element resolves (its `covered_by`
/// cases exist; its exception cites a real register entry), plus register
/// self-consistency (element shapes, stale branch exceptions).
fn check_wire_surface_elements(
    set: &ArtifactSet,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    if let Err(messages) = wire_surface.check_invariants() {
        for message in messages {
            push(
                findings,
                CheckId::SurfaceCoverage,
                "vocab/wire_surface.yaml",
                message,
            );
        }
    }
    let case_ids: BTreeSet<&str> = set.cases.iter().map(|(_, c)| c.id.as_str()).collect();
    for element in &wire_surface.elements {
        for cid in &element.covered_by {
            if !case_ids.contains(cid.as_str()) {
                push(
                    findings,
                    CheckId::SurfaceCoverage,
                    &element.id,
                    format!("covered_by case {cid} does not exist"),
                );
            }
        }
        if let Some(ex) = &element.exception
            && let Some(reg) = &ex.register
            && set
                .register
                .as_ref()
                .is_none_or(|(_, r)| r.get(reg).is_none())
        {
            push(
                findings,
                CheckId::SurfaceCoverage,
                &element.id,
                format!("exception cites {reg} which is not in the ambiguity register"),
            );
        }
    }
    // Ratchet: a branch exception matching no realized binding is stale.
    for branch in &wire_surface.branches {
        let matches = set.bindings.iter().any(|(_, b)| {
            !b.is_unrealized()
                && b.sm_operation == branch.binding
                && (branch.variant.is_none() || b.variant.as_deref() == branch.variant.as_deref())
        });
        if !matches {
            push(
                findings,
                CheckId::SurfaceCoverage,
                &branch.binding.to_string(),
                "wire_surface.yaml branch exception matches no realized binding (stale)".to_owned(),
            );
        }
    }
}

// ── Axis 3 section derivation ───────────────────────────────────────────────

/// The RELEASED overview documents whose section headings ARE the Axis-3
/// enumeration domain, relative to the vendored spec root. Both are ITS-REST
/// docs text (never the OAS): the cross-cutting wire behaviours the API defines
/// outside any single operation live in these two chapters and nowhere else.
const AXIS3_OVERVIEW_DOCS: &[&str] = &[
    "ITS-REST/specifications/docs/overview/Requests_and_responses.md",
    "ITS-REST/specifications/docs/overview/Resources.md",
];

/// Sections of [`AXIS3_OVERVIEW_DOCS`] that define no distinct testable
/// cross-cutting wire behaviour, each with the citation saying why. A heading
/// listed here is exempt from the derivation; every other heading must appear
/// in at least one authored `elements`/`branches` source string.
///
/// Empty today: every heading of both released overview chapters is named by
/// an authored source. The table is the pinned escape hatch for a heading that
/// is genuinely un-testable (a pure framing paragraph), never a way to retire
/// an inconvenient behaviour — an entry whose heading IS named by a source, or
/// which names no heading at all, is itself a finding.
const AXIS3_SECTION_EXCLUSIONS: &[(&str, &str)] = &[];

/// Whitespace-normalized, lower-cased form used for heading↔source matching
/// (source strings are YAML-folded, so they carry newlines and runs of spaces;
/// one vendored heading — "Prefer only identifier " — carries a trailing space
/// the vendored file must keep).
fn normalize_heading(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The `#`/`##` section headings of a released markdown chapter, in document
/// order, de-duplicated, with fenced code blocks skipped (a `# …` line inside
/// a fence is code, not a heading).
fn markdown_section_headings(text: &str) -> Vec<String> {
    let mut headings: Vec<String> = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some(rest) = trimmed
            .strip_prefix("# ")
            .or_else(|| trimmed.strip_prefix("## "))
        else {
            continue;
        };
        let heading = rest.trim().to_owned();
        if heading.is_empty() || headings.iter().any(|h| h == &heading) {
            continue;
        }
        headings.push(heading);
    }
    headings
}

/// Every `source` string the Axis-3 register authors (elements + branches) —
/// the corpus a derived heading must be named by.
fn wire_surface_source_texts(wire_surface: &WireSurface) -> Vec<String> {
    let mut sources: Vec<String> = Vec::new();
    for element in &wire_surface.elements {
        sources.push(normalize_heading(&element.source));
    }
    for branch in &wire_surface.branches {
        sources.push(normalize_heading(&branch.source));
    }
    sources
}

/// One released overview chapter's derivation result.
struct DocDerivation {
    /// The chapter's path relative to the spec root.
    doc: &'static str,
    /// The chapter could not be read (its own finding).
    unreadable: bool,
    /// Headings named by ≥ 1 authored source.
    covered: Vec<String>,
    /// Headings exempted by [`AXIS3_SECTION_EXCLUSIONS`].
    excluded: Vec<String>,
    /// Headings with neither (each one a finding).
    uncovered: Vec<String>,
}

/// Derive the Axis-3 enumeration domain from the released overview chapters and
/// classify every heading. Pure over (`wire_surface`, the vendored files,
/// `exclusions`) — the exclusion table is a parameter so the mechanism is
/// testable without touching the pinned const.
fn axis3_derivation(
    wire_surface: &WireSurface,
    spec_root: &Path,
    exclusions: &[(&str, &str)],
) -> Vec<DocDerivation> {
    let sources = wire_surface_source_texts(wire_surface);
    let mut out = Vec::new();
    for doc in AXIS3_OVERVIEW_DOCS.iter().copied() {
        let Ok(text) = std::fs::read_to_string(spec_root.join(doc)) else {
            out.push(DocDerivation {
                doc,
                unreadable: true,
                covered: Vec::new(),
                excluded: Vec::new(),
                uncovered: Vec::new(),
            });
            continue;
        };
        let mut derivation = DocDerivation {
            doc,
            unreadable: false,
            covered: Vec::new(),
            excluded: Vec::new(),
            uncovered: Vec::new(),
        };
        for heading in markdown_section_headings(&text) {
            let needle = normalize_heading(&heading);
            if sources.iter().any(|source| source.contains(&needle)) {
                derivation.covered.push(heading);
            } else if exclusions
                .iter()
                .any(|(excluded, _)| normalize_heading(excluded) == needle)
            {
                derivation.excluded.push(heading);
            } else {
                derivation.uncovered.push(heading);
            }
        }
        out.push(derivation);
    }
    out
}

/// Axis 3, derivation half — the register's element list is AUTHORED, so a
/// cross-cutting behaviour nobody thought of is invisible to
/// [`check_wire_surface_elements`]. This derives the domain instead: every
/// `#`/`##` section of the two RELEASED overview chapters must be named by an
/// authored `elements`/`branches` source, or be pinned in
/// [`AXIS3_SECTION_EXCLUSIONS`] with its citation. Stale exclusions (covered,
/// or naming no heading at all) are findings too — the table only ratchets.
fn check_axis3_section_derivation(
    wire_surface: &WireSurface,
    spec_root: &Path,
    exclusions: &[(&str, &str)],
    findings: &mut Vec<Finding>,
) {
    let derivations = axis3_derivation(wire_surface, spec_root, exclusions);
    let mut every_heading: BTreeSet<String> = BTreeSet::new();
    let mut covered_headings: BTreeSet<String> = BTreeSet::new();
    for derivation in &derivations {
        if derivation.unreadable {
            push(
                findings,
                CheckId::SurfaceCoverage,
                derivation.doc,
                "released overview chapter is not readable under the vendored spec root — the \
                 Axis-3 section derivation cannot run"
                    .to_owned(),
            );
            continue;
        }
        for heading in derivation
            .covered
            .iter()
            .chain(&derivation.excluded)
            .chain(&derivation.uncovered)
        {
            every_heading.insert(normalize_heading(heading));
        }
        for heading in &derivation.covered {
            covered_headings.insert(normalize_heading(heading));
        }
        for heading in &derivation.uncovered {
            push(
                findings,
                CheckId::SurfaceCoverage,
                derivation.doc,
                format!(
                    "§{heading} is a section of a RELEASED overview chapter that no \
                     wire_surface.yaml elements/branches source names — add a cross-cutting \
                     element for the behaviour (covered_by a case, or a cited exception), or pin \
                     the heading in AXIS3_SECTION_EXCLUSIONS \
                     (tools/cnf-runner/src/validate.rs) with the citation saying why it defines \
                     no distinct testable wire behaviour"
                ),
            );
        }
    }
    if derivations.iter().any(|d| d.unreadable) {
        return; // an incomplete domain cannot judge exclusion staleness
    }
    for (heading, _) in exclusions {
        let needle = normalize_heading(heading);
        if !every_heading.contains(&needle) {
            push(
                findings,
                CheckId::SurfaceCoverage,
                "AXIS3_SECTION_EXCLUSIONS",
                format!("exclusion {heading:?} names no section of the released overview chapters"),
            );
        } else if covered_headings.contains(&needle) {
            push(
                findings,
                CheckId::SurfaceCoverage,
                "AXIS3_SECTION_EXCLUSIONS",
                format!(
                    "exclusion {heading:?} is stale — an authored wire_surface.yaml source now \
                     names that section; remove the exclusion"
                ),
            );
        }
    }
}

/// Render the deterministic coverage report (`docs/conformance/coverage-report.md`):
/// per-interface SM-operation status, per-binding outcome/format coverage, and
/// the cross-cutting wire-surface table. Stable ordering, no timestamps — the
/// same inputs always render byte-identical output.
///
/// Axis 1 (the per-interface section) and the Axis-3 section derivation render
/// only when `spec_root` is supplied (they read the vendored spec tree).
#[must_use]
#[allow(clippy::too_many_lines)] // one deterministic report-rendering seam
pub fn render_coverage_report(set: &ArtifactSet, spec_root: Option<&Path>) -> String {
    use std::fmt::Write;

    let empty = WireSurface::default();
    let wire_surface = set.wire_surface.as_ref().map_or(&empty, |(_, w)| w);

    let mut out = String::new();
    out.push_str(
        "# CNF wire-surface coverage report\n\n\
         Generated by `cnf-runner validate --specs …` (the `surface-coverage` gate, issue #271). \
         Deterministic — regenerated in place, never hand-edited. The wire surface is enumerated \
         from the RELEASED spec components (the SM platform interfaces + the ITS-REST docs text), \
         never the vendored OAS. Every un-exercised behaviour is either a covering case or a \
         cited `vocab/wire_surface.yaml` exception; silence is not coverage.\n\n",
    );

    // ── Axis 1 ──
    if let Some(spec_root) = spec_root {
        out.push_str("## Axis 1 — SM-operation coverage (per platform interface)\n\n");
        out.push_str("| Interface | Operations | Realized | Unrealized | Off-wire / exception |\n");
        out.push_str("|---|--:|--:|--:|--:|\n");
        for interface in PLATFORM_INTERFACES {
            let Ok(ops) = sm_interface_operations(spec_root, interface) else {
                let _ = writeln!(out, "| {interface} | (no vendored SM class export) | | | |");
                continue;
            };
            let (mut realized, mut unrealized, mut excepted) = (0_usize, 0_usize, 0_usize);
            for name in &ops {
                let Ok(op) = SmOperationRef::parse(&format!("{interface}.{name}")) else {
                    continue;
                };
                let binding = set.bindings.iter().find(|(_, b)| b.sm_operation == op);
                match binding {
                    Some((_, b)) if b.is_unrealized() => unrealized += 1,
                    Some(_) => realized += 1,
                    None if wire_surface.sm_exception(&op).is_some() => excepted += 1,
                    None => {}
                }
            }
            let _ = writeln!(
                out,
                "| {interface} | {} | {realized} | {unrealized} | {excepted} |",
                ops.len()
            );
        }
        // The ITS-side half of Axis 1: the pinned non-SM ITS-REST operations,
        // grouped by their reserved pseudo-interface. Computed exactly like an
        // SM row — the anchor differs, the obligation does not.
        let mut pseudo: BTreeMap<String, Vec<SmOperationRef>> = BTreeMap::new();
        for (name, _) in NON_SM_REST_OPERATIONS {
            let Ok(op) = SmOperationRef::parse(name) else {
                continue;
            };
            pseudo
                .entry(op.interface().to_owned())
                .or_default()
                .push(op);
        }
        for (interface, ops) in pseudo {
            let (mut realized, mut unrealized, mut excepted) = (0_usize, 0_usize, 0_usize);
            for op in &ops {
                match set.bindings.iter().find(|(_, b)| b.sm_operation == *op) {
                    Some((_, b)) if b.is_unrealized() => unrealized += 1,
                    Some(_) => realized += 1,
                    None if wire_surface.sm_exception(op).is_some() => excepted += 1,
                    None => {}
                }
            }
            let _ = writeln!(
                out,
                "| {interface} (docs-text pinned, non-SM) | {} | {realized} | {unrealized} | \
                 {excepted} |",
                ops.len()
            );
        }
        out.push('\n');
    }

    // ── Axis 2 ──
    out.push_str("## Axis 2 — per-binding outcome/format coverage\n\n");
    out.push_str("| Binding | Outcomes covered | Formats covered |\n");
    out.push_str("|---|---|---|\n");
    let exercised = exercised_branches(set);
    let universal = universal_outcome_tokens(set);
    let mut realized: Vec<&OperationBinding> = set
        .bindings
        .iter()
        .map(|(_, b)| b)
        .filter(|b| !b.is_unrealized())
        .collect();
    realized.sort_by_key(|b| binding_label(b));
    for binding in realized {
        let variant = binding.variant.as_deref();
        let key = (binding.sm_operation.clone(), binding.variant.clone());
        let done = exercised.get(&key);
        let (mut ocov, mut oexc, mut ogap) = (0_usize, 0_usize, 0_usize);
        for (okey, _) in binding.outcomes.as_deref().unwrap_or_default() {
            let kind = okey.0;
            if universal.contains(&kind.token()) {
                continue;
            }
            if done.is_some_and(|d| d.0.contains(&kind)) {
                ocov += 1;
            } else if wire_surface
                .outcome_exception(&binding.sm_operation, variant, kind)
                .is_some()
            {
                oexc += 1;
            } else {
                ogap += 1;
            }
        }
        let (mut fcov, mut fexc, mut fgap) = (0_usize, 0_usize, 0_usize);
        for format in &binding.formats {
            if done.is_some_and(|d| d.1.contains(format)) {
                fcov += 1;
            } else if wire_surface
                .format_exception(&binding.sm_operation, variant, *format)
                .is_some()
            {
                fexc += 1;
            } else {
                fgap += 1;
            }
        }
        let _ = writeln!(
            out,
            "| `{}` | {ocov} exercised / {oexc} excepted / {ogap} gap | {fcov} exercised / {fexc} excepted / {fgap} gap |",
            binding_label(binding)
        );
    }
    out.push('\n');

    // ── Axis 3 ──
    out.push_str("## Axis 3 — cross-cutting wire-surface behaviours\n\n");
    out.push_str("| Element | Coverage |\n|---|---|\n");
    for element in &wire_surface.elements {
        let coverage = if let Some(ex) = &element.exception {
            match &ex.register {
                Some(reg) => format!("exception: {} ({reg})", ex.reason.token()),
                None => format!("exception: {}", ex.reason.token()),
            }
        } else {
            format!("{} case(s)", element.covered_by.len())
        };
        let _ = writeln!(out, "| `{}` | {coverage} |", element.id);
    }
    out.push('\n');

    // ── Axis 3, derivation half ──
    if let Some(spec_root) = spec_root {
        out.push_str("### Axis 3 derivation — RELEASED overview sections\n\n");
        out.push_str(
            "The element list above is AUTHORED; this table is DERIVED — every `#`/`##` section \
             of the two released overview chapters must be named by an authored \
             `elements`/`branches` source or pinned in `AXIS3_SECTION_EXCLUSIONS`.\n\n",
        );
        out.push_str(
            "| Chapter | Sections | Named by a source | Excluded (pinned) | Uncovered |\n",
        );
        out.push_str("|---|--:|--:|--:|--:|\n");
        for derivation in axis3_derivation(wire_surface, spec_root, AXIS3_SECTION_EXCLUSIONS) {
            if derivation.unreadable {
                let _ = writeln!(out, "| `{}` | (not readable) | | | |", derivation.doc);
                continue;
            }
            let sections =
                derivation.covered.len() + derivation.excluded.len() + derivation.uncovered.len();
            let _ = writeln!(
                out,
                "| `{}` | {sections} | {} | {} | {} |",
                derivation.doc,
                derivation.covered.len(),
                derivation.excluded.len(),
                derivation.uncovered.len()
            );
        }
        out.push('\n');
    }
    out
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)] // test assertions/fixtures
mod surface_tests {
    use super::*;

    /// The Axis-1 enumeration source is the vendored SM class tree, never the
    /// OAS (exit criterion 5, issue #271; owner ruling 2026-07-24). The path
    /// [`sm_class_file`] builds is under `SM/docs/UML/classes/` and names no
    /// OAS artifact.
    #[test]
    fn sm_operation_source_is_the_sm_tree_never_the_oas() {
        let file = sm_class_file(Path::new("/root"), "I_EHR_SERVICE");
        let text = file.to_string_lossy();
        assert!(
            text.ends_with("SM/docs/UML/classes/i_ehr_service.adoc"),
            "{text}"
        );
        assert!(!text.contains("oas") && !text.contains("rest-oas") && !text.contains("openapi"));
        // Every pinned interface is an SM `I_`-prefixed interface name.
        assert!(PLATFORM_INTERFACES.iter().all(|i| i.starts_with("I_")));
    }

    #[test]
    fn sm_interface_operations_parses_service_ops_only() {
        // A minimal SM class export: operation rows (`|*name* (`), a
        // sub-interface accessor (`|*i_ehr* (`), and non-operation header
        // cells (`h|*…*`, an uppercase interface cell) that must be ignored.
        let adoc = "\
|===\n\
h|*Interface*\n\
2+^h|*I_FIXTURE*\n\
h|*Functions*\n\
^h|*Signature*\n\
h|*1..1*\n\
|*create_thing* ( +\n\
    x: STRING +\n\
): THING\n\
h|*1..1*\n\
|*get_thing* ( +\n\
): THING\n\
h|*1..1*\n\
|*i_ehr* ( +\n\
): I_EHR\n\
|===\n";
        let dir = assert_fs::TempDir::new().unwrap();
        let classes = dir.path().join("SM/docs/UML/classes");
        std::fs::create_dir_all(&classes).unwrap();
        std::fs::write(classes.join("i_fixture.adoc"), adoc).unwrap();

        let ops = sm_interface_operations(dir.path(), "I_FIXTURE").unwrap();
        assert_eq!(ops, vec!["create_thing".to_owned(), "get_thing".to_owned()]);
        // A missing class export is an error, not an empty list.
        assert!(sm_interface_operations(dir.path(), "I_ABSENT").is_err());
    }

    fn build_set(with_exception: bool) -> ArtifactSet {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr" },
            "formats": ["canonical-json"],
            "outcomes": { "created": { "status": 201 }, "already_exists": { "status": 409 } }
        }))
        .unwrap();
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "T-1", "kind": "functional", "component": "EHR",
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "test_purpose": "t", "description": "d", "spec_refs": ["SM i_ehr_service.adoc"],
            "capabilities": [],
            "flow": [ { "step": 1, "call": "create_ehr", "expect": "created" } ]
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings.push((PathBuf::from("b.yaml"), binding));
        set.cases.push((PathBuf::from("c.yaml"), case));
        if with_exception {
            let wire: WireSurface = serde_json::from_value(serde_json::json!({
                "branches": [ {
                    "binding": "I_EHR_SERVICE.create_ehr", "outcome": "already_exists",
                    "reason": "coverage_gap",
                    "source": "ITS-REST Requests_and_responses.md §HTTP status codes"
                } ]
            }))
            .unwrap();
            set.wire_surface = Some((PathBuf::from("wire_surface.yaml"), wire));
        }
        set
    }

    #[test]
    fn axis2_flags_unexercised_outcome_then_exception_suppresses_it() {
        // `created` is exercised by the case; `already_exists` is declared but
        // never exercised → a surface-coverage finding.
        let set = build_set(false);
        let empty = WireSurface::default();
        let mut findings = Vec::new();
        check_binding_branch_coverage(&set, &empty, &mut findings);
        assert!(
            findings.iter().any(
                |f| f.check == CheckId::SurfaceCoverage && f.message.contains("already_exists")
            ),
            "expected an already_exists gap, got: {findings:?}"
        );

        // A branch exception for the same outcome suppresses the finding.
        let set = build_set(true);
        let wire = set.wire_surface.as_ref().map(|(_, w)| w).unwrap();
        let mut findings = Vec::new();
        check_binding_branch_coverage(&set, wire, &mut findings);
        assert!(
            findings.is_empty(),
            "the branch exception should suppress the gap, got: {findings:?}"
        );
    }

    /// Axis 4 is a DECLARATION: adding served extensions changes no coverage
    /// obligation on the other three axes, and a family may not claim a
    /// released path.
    #[test]
    fn axis4_declares_without_gating() {
        let set = build_set(true);
        let wire = set.wire_surface.as_ref().map(|(_, w)| w).unwrap();
        let mut baseline = Vec::new();
        check_binding_branch_coverage(&set, wire, &mut baseline);
        check_wire_surface_elements(&set, wire, &mut baseline);
        assert!(baseline.is_empty(), "{baseline:?}");

        let declared: WireSurface = serde_json::from_value(serde_json::json!({
            "branches": [ {
                "binding": "I_EHR_SERVICE.create_ehr", "outcome": "already_exists",
                "reason": "coverage_gap",
                "source": "ITS-REST Requests_and_responses.md §HTTP status codes"
            } ],
            "served_extensions": [ {
                "family": "management",
                "routes": ["GET /management/info"],
                "config_gate": "management.enabled",
                "spec_silence": "no released clause governs the URI space beyond the resource set",
                "never_gates": true
            } ]
        }))
        .unwrap();
        let mut with_axis = Vec::new();
        check_binding_branch_coverage(&set, &declared, &mut with_axis);
        check_wire_surface_elements(&set, &declared, &mut with_axis);
        check_served_extensions(&set, &declared, &mut with_axis);
        assert!(
            with_axis.is_empty(),
            "the outward axis must never add an obligation, got: {with_axis:?}"
        );

        // Claiming a released path IS a finding (the binding fixture serves
        // POST /ehr; the mount prefix is stripped at the segment boundary).
        let claiming: WireSurface = serde_json::from_value(serde_json::json!({
            "served_extensions": [ {
                "family": "impostor",
                "routes": ["POST /ehrbase/rest/openehr/v1/ehr"],
                "config_gate": "always on",
                "spec_silence": "s",
                "never_gates": true
            } ]
        }))
        .unwrap();
        let mut findings = Vec::new();
        check_served_extensions(&set, &claiming, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("claims the released")),
            "expected a released-path claim finding, got: {findings:?}"
        );
    }

    #[test]
    fn axis3_element_covered_by_must_resolve() {
        let set = build_set(false);
        let wire: WireSurface = serde_json::from_value(serde_json::json!({
            "elements": [ {
                "id": "x", "description": "d",
                "source": "ITS-REST Requests_and_responses.md §Location",
                "covered_by": ["NO-SUCH-CASE"]
            } ]
        }))
        .unwrap();
        let mut findings = Vec::new();
        check_wire_surface_elements(&set, &wire, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SurfaceCoverage && f.message.contains("NO-SUCH-CASE")),
            "expected an unresolved covered_by finding, got: {findings:?}"
        );
    }

    /// A pinned non-SM ITS-REST operation resolves WITHOUT an SM class export
    /// (there is none — that is the whole point), while an unpinned
    /// `I_ITS_REST_*` reference is a finding naming the pinned table, so the
    /// pseudo-interface cannot become a way around SM resolution.
    #[test]
    fn pinned_pseudo_operation_resolves_and_an_unpinned_one_is_a_finding() {
        let empty_root = assert_fs::TempDir::new().unwrap();

        let (pinned_name, _) = *NON_SM_REST_OPERATIONS.first().unwrap();
        let pinned = SmOperationRef::parse(pinned_name).unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&pinned, "b.yaml", empty_root.path(), &mut findings);
        assert!(findings.is_empty(), "{findings:?}");

        let invented = SmOperationRef::parse("I_ITS_REST_SYSTEM.invented").unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&invented, "b.yaml", empty_root.path(), &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SmOperation
                    && f.message.contains("NON_SM_REST_OPERATIONS")),
            "expected an unpinned-pseudo-interface finding, got: {findings:?}"
        );

        // A real SM interface is still resolved against the vendored tree.
        let sm = SmOperationRef::parse("I_EHR_SERVICE.create_ehr").unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&sm, "b.yaml", empty_root.path(), &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("no vendored SM class export")),
            "{findings:?}"
        );
    }

    /// Axis 1 holds a pinned non-SM operation to the same obligation as an SM
    /// one: no binding and no `sm_operations` exception is a finding.
    #[test]
    fn axis1_requires_a_binding_for_a_pinned_non_sm_operation() {
        let set = build_set(false);
        let empty_root = assert_fs::TempDir::new().unwrap();
        let empty = WireSurface::default();
        let (pinned, pinned_source) = *NON_SM_REST_OPERATIONS.first().unwrap();

        let mut findings = Vec::new();
        check_surface_sm_operations(&set, empty_root.path(), &empty, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.artifact == pinned && f.message.contains("non-SM ITS-REST operation")),
            "expected an unbound non-SM operation finding, got: {findings:?}"
        );

        // A cited sm_operations exception suppresses it (as for an SM op).
        let excepted: WireSurface = serde_json::from_value(serde_json::json!({
            "sm_operations": [ {
                "operation": pinned,
                "reason": "coverage_gap",
                "source": pinned_source
            } ]
        }))
        .unwrap();
        let mut findings = Vec::new();
        check_surface_sm_operations(&set, empty_root.path(), &excepted, &mut findings);
        assert!(
            !findings.iter().any(|f| f.artifact == pinned),
            "the exception should suppress the finding, got: {findings:?}"
        );
    }

    /// The pinned table is a MULTI-interface domain, not a single-row special
    /// case: every row parses, carries the reserved prefix and a non-empty
    /// citation, no reference is pinned twice, and more than one reserved
    /// pseudo-interface is represented (System + ITEM_TAGS today).
    #[test]
    fn pinned_non_sm_table_is_wellformed_across_several_pseudo_interfaces() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut interfaces: BTreeSet<String> = BTreeSet::new();
        for (name, source) in NON_SM_REST_OPERATIONS {
            let op = SmOperationRef::parse(name)
                .unwrap_or_else(|e| panic!("pinned row {name} does not parse: {e}"));
            assert!(
                op.interface().starts_with(PSEUDO_INTERFACE_PREFIX),
                "pinned row {name} must use the reserved pseudo-interface prefix"
            );
            assert!(!source.trim().is_empty(), "pinned row {name} has no source");
            assert!(seen.insert(*name), "pinned row {name} is listed twice");
            interfaces.insert(op.interface().to_owned());
        }
        assert!(
            interfaces.len() > 1,
            "the table must stay interface-general, got {interfaces:?}"
        );
        assert!(interfaces.contains("I_ITS_REST_ITEM_TAGS"));

        // The reservation holds for EVERY pseudo-interface, not just System:
        // an unpinned ITEM_TAGS reference is still a finding naming the table.
        let empty_root = assert_fs::TempDir::new().unwrap();
        let invented = SmOperationRef::parse("I_ITS_REST_ITEM_TAGS.folder_tags_get").unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&invented, "b.yaml", empty_root.path(), &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SmOperation
                    && f.message.contains("NON_SM_REST_OPERATIONS")),
            "expected an unpinned-pseudo-interface finding, got: {findings:?}"
        );
    }

    /// The Axis-1 report renders ONE row per reserved pseudo-interface, with
    /// per-interface counts — the single-pseudo-interface era is over.
    #[test]
    fn axis1_report_groups_rows_per_pseudo_interface() {
        let mut set = build_set(false);
        let tag_binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_ITS_REST_ITEM_TAGS.composition_tags_get",
            "its": "its-rest",
            "request": { "method": "GET", "path": "/ehr/{ehr_id}/composition/{uid_based_id}/tags" },
            "formats": ["canonical-json"],
            "outcomes": { "ok": { "status": 200 } }
        }))
        .unwrap();
        set.bindings.push((PathBuf::from("tags.yaml"), tag_binding));
        let empty_root = assert_fs::TempDir::new().unwrap();

        let report = render_coverage_report(&set, Some(empty_root.path()));
        let tag_ops = NON_SM_REST_OPERATIONS
            .iter()
            .filter(|(name, _)| name.starts_with("I_ITS_REST_ITEM_TAGS."))
            .count();
        assert!(
            report.contains("| I_ITS_REST_SYSTEM (docs-text pinned, non-SM) | 1 | 0 | 0 | 0 |"),
            "{report}"
        );
        assert!(
            report.contains(&format!(
                "| I_ITS_REST_ITEM_TAGS (docs-text pinned, non-SM) | {tag_ops} | 1 | 0 | 0 |"
            )),
            "{report}"
        );
    }

    #[test]
    fn markdown_headings_take_h1_h2_only_and_skip_fences() {
        let doc = "\
[comment]: # (title: Fixture)\n\
\n\
# HTTP Methods\n\
some prose\n\
## Prefer only identifier \n\
```http\n\
# not a heading\n\
```\n\
### too deep\n\
#nospace\n\
# HTTP Methods\n";
        let headings = markdown_section_headings(doc);
        assert_eq!(
            headings,
            vec![
                "HTTP Methods".to_owned(),
                "Prefer only identifier".to_owned()
            ]
        );
    }

    /// The matcher is whitespace-normalized and case-insensitive over the
    /// YAML-folded source strings, and the exclusion table is the only other
    /// way a heading can be accounted for.
    #[test]
    fn axis3_derivation_matches_sources_and_honours_exclusions() {
        let dir = assert_fs::TempDir::new().unwrap();
        for doc in AXIS3_OVERVIEW_DOCS {
            let path = dir.path().join(doc);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "# Covered Section\n\n## Silent Section\n").unwrap();
        }
        let wire: WireSurface = serde_json::from_value(serde_json::json!({
            "elements": [ {
                "id": "x", "description": "d",
                // folded whitespace + different case: still a match
                "source": "ITS-REST Requests_and_responses.md §covered\n  section (the rule)",
                "exception": { "reason": "coverage_gap" }
            } ]
        }))
        .unwrap();

        let derivations = axis3_derivation(&wire, dir.path(), &[]);
        for derivation in &derivations {
            assert!(!derivation.unreadable);
            assert_eq!(derivation.covered, vec!["Covered Section".to_owned()]);
            assert_eq!(derivation.uncovered, vec!["Silent Section".to_owned()]);
        }

        let excluded = axis3_derivation(&wire, dir.path(), &[("silent section", "why")]);
        for derivation in &excluded {
            assert_eq!(derivation.excluded, vec!["Silent Section".to_owned()]);
            assert!(derivation.uncovered.is_empty());
        }
    }

    /// An unnamed released section is a finding; the exclusion table only
    /// ratchets — an exclusion that is covered, or that names no section at
    /// all, is a finding of its own.
    #[test]
    fn axis3_derivation_findings_and_exclusion_ratchet() {
        let dir = assert_fs::TempDir::new().unwrap();
        for doc in AXIS3_OVERVIEW_DOCS {
            let path = dir.path().join(doc);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "# Covered Section\n\n## Silent Section\n").unwrap();
        }
        let wire: WireSurface = serde_json::from_value(serde_json::json!({
            "elements": [ {
                "id": "x", "description": "d",
                "source": "ITS-REST Requests_and_responses.md §Covered Section",
                "exception": { "reason": "coverage_gap" }
            } ]
        }))
        .unwrap();

        let mut findings = Vec::new();
        check_axis3_section_derivation(&wire, dir.path(), &[], &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SurfaceCoverage
                    && f.message.contains("§Silent Section")),
            "expected an uncovered-section finding, got: {findings:?}"
        );

        let mut findings = Vec::new();
        check_axis3_section_derivation(
            &wire,
            dir.path(),
            &[
                ("Covered Section", "already named by a source"),
                ("No Such Section", "names nothing"),
                ("Silent Section", "no distinct testable wire behaviour"),
            ],
            &mut findings,
        );
        assert!(
            findings.iter().any(|f| f.message.contains("is stale")),
            "expected a stale-exclusion finding, got: {findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("names no section")),
            "expected an unknown-exclusion finding, got: {findings:?}"
        );
        assert!(
            !findings.iter().any(|f| f.message.contains("§Silent")),
            "the honest exclusion must suppress its section finding, got: {findings:?}"
        );
    }

    /// A missing released chapter is itself a finding — the derivation must
    /// never silently pass because it could not read its domain.
    #[test]
    fn axis3_derivation_reports_an_unreadable_chapter() {
        let dir = assert_fs::TempDir::new().unwrap();
        let wire = WireSurface::default();
        let mut findings = Vec::new();
        check_axis3_section_derivation(&wire, dir.path(), &[], &mut findings);
        assert_eq!(findings.len(), AXIS3_OVERVIEW_DOCS.len(), "{findings:?}");
        assert!(
            findings.iter().all(|f| f.message.contains("not readable")),
            "{findings:?}"
        );
    }
}
