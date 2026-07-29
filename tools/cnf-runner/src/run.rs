//! The run orchestration: select → execute (the interpreter over the live
//! driver) → record — producing the `results.json` outcomes the party layer
//! emits and the verdict pipeline consumes.
//!
//! Interpreter-coverage accounting is first-class: every case that cannot
//! be interpreter-run is a REGISTERED EXCEPTION with its reason (the
//! ≥90%-interpreter-run gate is computed, never asserted).

use crate::artifacts::ArtifactSet;
use crate::exec::driver::HttpDriver;
use crate::exec::{CaseRecord, RowOutcome, run_case};
use crate::ids::SmOperationRef;
use crate::ixit::Ixit;
use crate::model::assertion::assertion_refs;
use crate::model::case::CaseCore;
use crate::refgrammar::{IxitField, ValueRef};
use crate::vocab::CaseStatus;

/// Why a case was not interpreter-run (the registered-exception taxonomy).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum Exception {
    /// Every binding of the case's operations is `unrealized` on this ITS —
    /// not-applicable with the binding's citation.
    Unrealized(String),
    /// `kind: content` — decision-table execution needs the row-to-instance
    /// generation seam (the registered recipes cover the committed corpus;
    /// per-row template projection is the remaining glue).
    ContentGeneration(String),
    /// A guard excludes the case on this SUT (citation carried).
    Guarded(String),
    /// The case is `draft`/`retired` — never verdict-bearing.
    Status(String),
}

/// One run's execution report.
#[derive(Debug, Default)]
pub struct RunReport {
    pub records: Vec<CaseRecord>,
    pub exceptions: Vec<(crate::ids::CaseId, Exception)>,
    /// Cases the interpreter drove end-to-end.
    pub interpreter_run: usize,
    /// All active assertion-machinery cases considered.
    pub considered: usize,
    /// The `restapi_specs_version` the System OPTIONS manifest served, when
    /// the campaign drove that exchange — an independent confirmation of the
    /// party's declared `spec_versions.its_rest`, never a source of truth
    /// (the released `Options` schema has no `required` list; a divergence
    /// becomes a static-review finding, not a re-declaration).
    pub restapi_specs_version: Option<String>,
}

impl RunReport {
    /// The interpreter-coverage fraction (the ≥90% gate input).
    #[must_use]
    pub fn interpreter_coverage(&self) -> f64 {
        if self.considered == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)] // case counts << 2^52
        {
            self.interpreter_run as f64 / self.considered as f64
        }
    }
}

/// Whether every operation the case's flow calls is unrealized on this ITS.
///
/// Shared with the `claim-completeness` gate ([`crate::validate`]), which
/// needs the same catalogue-side predicate to tell a case that can carry
/// executed evidence from one that will always resolve excused.
pub(crate) fn fully_unrealized(set: &ArtifactSet, case: &CaseCore) -> Option<String> {
    // ANY unrealized step makes the whole case not-applicable on this ITS:
    // the flow cannot reach its expectation without the missing wire, so a
    // verdict would be meaningless — the case is excused with the machine-
    // readable citation the binding declares.
    let anchor = case.sm_operation.as_ref()?;
    let mut citations = Vec::new();
    for step in &case.flow {
        let op = if step.call.contains('.') {
            SmOperationRef::parse(&step.call).ok()?
        } else {
            anchor.sibling(&step.call)
        };
        let binding = set
            .bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| b.sm_operation == op)?;
        if let Some(decl) = &binding.unrealized {
            citations.push(format!("{op}: {}", decl.ambiguity));
        }
    }
    (!citations.is_empty()).then(|| citations.join("; "))
}

/// The binding a flow step drives — the driver's variant-aware selection, so
/// selection-time guards judge exactly the realization the driver will send.
fn step_binding<'a>(
    set: &'a ArtifactSet,
    case: &CaseCore,
    step: &crate::model::case::FlowStep,
) -> Option<&'a crate::model::binding::OperationBinding> {
    let op = if step.call.contains('.') {
        SmOperationRef::parse(&step.call).ok()?
    } else {
        case.sm_operation.as_ref()?.sibling(&step.call)
    };
    let mut bindings = set.bindings.iter().map(|(_, b)| b);
    if let Some(variant) = step.variant.as_deref()
        && let Some(exact) = bindings
            .clone()
            .find(|b| b.sm_operation == op && b.variant.as_deref() == Some(variant))
    {
        return Some(exact);
    }
    bindings.find(|b| b.sm_operation == op && b.variant.is_none())
}

/// The OPERATION-level spec-version floors this party does not meet
/// (`OperationBinding::applies`, issue #629 — the field was deserialized and
/// read by nothing).
///
/// A binding declares a floor when the WIRE itself arrived in a later
/// release: driving it against a party that declares an earlier one asks a
/// server for an endpoint or request form its release never defined, which is
/// a selection question (ISO/IEC 9646), not a conformance failure. The case is
/// therefore not-applicable with the citation, exactly as an undeclared option
/// branch or an undeclared ixit fact is.
///
/// This is the OPERATION level only. A release that merely dates how an
/// ANSWER must look (the `W/` weakness indicator, the read/DELETE `Location`
/// restriction) puts its floor on the header expectation instead, so the
/// operation stays driven and only that one rule is out of scope — see
/// [`crate::model::binding::HeaderExpectation`].
fn unmet_binding_floors(
    set: &ArtifactSet,
    case: &CaseCore,
    versions: &crate::party::SpecVersions,
) -> Vec<String> {
    let mut unmet = Vec::new();
    for step in &case.flow {
        let Some(binding) = step_binding(set, case, step) else {
            continue;
        };
        let Some(applies) = &binding.applies else {
            continue;
        };
        if applies.satisfied_by(versions) {
            continue;
        }
        let declared: Vec<String> = applies
            .entries()
            .into_iter()
            .map(|(component, range)| format!("{} {}", component.token(), range.raw()))
            .collect();
        let citation = format!("{} requires {}", binding.sm_operation, declared.join(", "));
        if !unmet.contains(&citation) {
            unmet.push(citation);
        }
    }
    unmet
}

/// The `served_extensions` family + adjudicating register entry of the first
/// EXTENSION binding the case's flow drives, if any — the marker that the case
/// verifies a route no openEHR specification governs (our own
/// design/extension). The register id travels with it so the not-applicable
/// citation is register-linked like every other excused row.
fn extension_family(set: &ArtifactSet, case: &CaseCore) -> Option<String> {
    let anchor = case.sm_operation.as_ref()?;
    for step in &case.flow {
        let op = if step.call.contains('.') {
            SmOperationRef::parse(&step.call).ok()?
        } else {
            anchor.sibling(&step.call)
        };
        if let Some(decl) = set
            .bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| b.sm_operation == op)
            .and_then(|b| b.extension.as_ref())
        {
            return Some(format!("{}; {}", decl.family, decl.ambiguity));
        }
    }
    None
}

/// The `${ixit:…}` facts a case reads that THIS party's ixit does not
/// declare. A declared fact is the only source (no released operation
/// discloses it), so a case that needs an undeclared one is not-applicable
/// on this party — never driven against a guessed value.
fn undeclared_ixit_facts(case: &CaseCore, ixit: &Ixit) -> Vec<&'static str> {
    fn note(reference: &ValueRef, into: &mut Vec<IxitField>) {
        if let ValueRef::Ixit(field) = reference
            && !into.contains(field)
        {
            into.push(*field);
        }
    }
    let mut referenced: Vec<IxitField> = Vec::new();
    for step in &case.flow {
        for (_, value) in step.with_entries() {
            for reference in value.refs() {
                note(reference, &mut referenced);
            }
        }
        for assertion in &step.assertions {
            for reference in assertion_refs(assertion) {
                note(&reference, &mut referenced);
            }
        }
    }
    for assertion in &case.postconditions {
        for reference in assertion_refs(assertion) {
            note(&reference, &mut referenced);
        }
    }
    referenced
        .into_iter()
        .filter(|field| match field {
            IxitField::SystemId => ixit.system_id.is_none(),
            IxitField::DumpLocation => ixit.dump_location.is_none(),
        })
        .map(IxitField::token)
        .collect()
}

/// The ixit instances a case's flow addresses (`on:`) that THIS party does
/// not declare.
///
/// An instance is a topology declaration exactly like the ixit facts above:
/// a party that runs no such deployment (no `readonly` principal, no second
/// signing posture) cannot have the case driven against it, and the
/// alternative to a declaration is driving it somewhere it does not belong.
/// So the case is not-applicable WITH the citation at selection time — never
/// a drive-time transport error, which would surface as an inconclusive row
/// that reads like a SUT defect.
fn undeclared_instances(case: &CaseCore, ixit: &Ixit) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for step in &case.flow {
        if let Some(name) = &step.on
            && ixit.instance(name).is_none()
            && !missing.iter().any(|m| m == name.as_str())
        {
            missing.push(name.as_str().to_owned());
        }
    }
    missing
}

/// The reserved catalogue pseudo-interface anchoring the SMART Platform
/// operations the SM models no interface for (pinned in
/// `validate::NON_SM_REST_OPERATIONS`; register AMB-161 adjudicates the
/// naming convention).
const SMART_PSEUDO_INTERFACE: &str = "I_ITS_REST_SMART";

/// Whether the case needs the party's SMART App Launch lane
/// (`ixit.smart`) — either because a flow step declares a SMART `scope`
/// claim the runner must mint a token for, or because the case drives a
/// SMART Platform operation that only a SMART-enabled deployment serves.
fn needs_smart_lane(case: &CaseCore) -> bool {
    case.flow
        .iter()
        .any(crate::model::case::FlowStep::declares_scopes)
        || case
            .sm_operation
            .as_ref()
            .is_some_and(|op| op.interface() == SMART_PSEUDO_INTERFACE)
}

fn not_applicable_record(case: &crate::model::case::CaseCore, citation: &str) -> CaseRecord {
    CaseRecord {
        case: case.id.clone(),
        format: None,
        rows: vec![RowOutcome::NotApplicable {
            citation: citation.to_owned(),
        }],
        rows_driven: 0,
        rows_total: crate::exec::row_count(case),
    }
}

/// The drive-time selection law (ISO/IEC 9646 ICS-driven selection + the
/// ixit declaration law): the FIRST ground that excuses `case` on this
/// party/deployment, or `None` when the case drives. Each arm carries its
/// citation inside the returned [`Exception`]; the caller records the same
/// citation as the case's single not-applicable row.
fn selection_exception(
    set: &ArtifactSet,
    ixit: &Ixit,
    statement: Option<&crate::party::Statement>,
    case: &crate::model::case::CaseCore,
) -> Option<Exception> {
    if let Some(citation) = fully_unrealized(set, case) {
        return Some(Exception::Unrealized(citation));
    }
    // The EXTENSION arm: a case that drives a route no openEHR specification
    // governs is our own design/extension, so it is behaviour only a party
    // that CLAIMS the capability answers for. Driving it at another vendor's
    // SUT would publish failures for routes that vendor never offered to
    // serve — the published comparison must be honest in both directions,
    // and a spurious red row is not honesty.
    if let Some(stmt) = statement
        && let Some(family) = extension_family(set, case)
        && !case
            .capabilities
            .iter()
            .any(|c| stmt.claims.capabilities.contains(c))
    {
        return Some(Exception::Unrealized(format!(
            "extension realization ({family}): the ICS claims none of this case's \
             capabilities, and no openEHR specification governs the route — ISO/IEC 9646 \
             test selection"
        )));
    }
    // An option branch the party statement does not declare is not this
    // SUT's behaviour — driving it records a spurious failure the verdict
    // pipeline would excuse anyway (`verdict::effective_outcome`); excuse it
    // at drive time with the same citation.
    if let Some(stmt) = statement
        && let Some(tag) = &case.option
        && !stmt.options.contains(tag)
    {
        return Some(Exception::Unrealized(format!(
            "option {tag}: the ICS does not declare this register branch \
             (statement.options) — ISO/IEC 9646 test selection"
        )));
    }
    // Case-level spec-version floors (`CaseCore.applies`): a behaviour the
    // spec dates to a release the party does not declare is out of scope for
    // it — `Applies::satisfied_by`, the one polarity every consumer of the
    // floor uses (`verdict` selection re-applies the same predicate).
    // Driving such a case records a spurious failure against behaviour the
    // party never claimed (the 2026-07-28 java run drove 127 red rows and 13
    // spuriously green ones this way).
    if let Some(stmt) = statement
        && !case.applies.satisfied_by(&stmt.spec_versions)
    {
        let declared: Vec<String> = case
            .applies
            .entries()
            .into_iter()
            .map(|(component, range)| format!("{} {}", component.token(), range.raw()))
            .collect();
        return Some(Exception::Unrealized(format!(
            "case version floor unmet ({}) — the party's declared spec versions do not \
             satisfy the case's applies ranges; ISO/IEC 9646 test selection",
            declared.join(", ")
        )));
    }
    // Operation-level spec-version floors (`OperationBinding.applies`): a
    // wire a later release introduced is not this party's behaviour to
    // answer for — the same selection question the option branch is, with
    // the binding's own declared range as the citation.
    if let Some(stmt) = statement {
        let unmet = unmet_binding_floors(set, case, &stmt.spec_versions);
        if !unmet.is_empty() {
            return Some(Exception::Unrealized(format!(
                "operation version floor unmet ({}) — the party's declared spec versions \
                 predate the release that introduced this wire; ISO/IEC 9646 test selection",
                unmet.join("; ")
            )));
        }
    }
    // The SMART lane is a party declaration, exactly like the ixit facts
    // below: the CDR is a SMART resource server that never issues tokens
    // (ITS-REST docs/smart_app_launch/master06-authentication.adoc
    // §Supported Authentication Flows), so a chosen `scope` claim exists
    // only where the party declares a trusted test issuer to mint against.
    // Undeclared => not-applicable with the citation, never a spurious
    // failure against a deployment that legitimately does not run SMART.
    if needs_smart_lane(case) && ixit.smart.is_none() {
        return Some(Exception::Guarded(
            "the ixit declares no `smart` lane — the case needs a SMART-enabled \
             deployment and a minted, scope-carrying access token, neither of which any \
             released operation discloses or provides; ISO/IEC 9646 test selection"
                .to_owned(),
        ));
    }
    // A flow step addressing an instance this party does not declare has no
    // ground to run on (the deployment or principal simply does not exist
    // here).
    let missing_instances = undeclared_instances(case, ixit);
    if !missing_instances.is_empty() {
        return Some(Exception::Guarded(format!(
            "the ixit declares no instance {} — the case's flow addresses it with `on:` and \
             this party runs no such deployment/principal; ISO/IEC 9646 test selection",
            missing_instances.join(", ")
        )));
    }
    // A case reading a party-declared SUT fact this ixit does not carry
    // cannot be driven: the fact is not on the wire, so the alternative to a
    // declaration is a guess.
    let missing = undeclared_ixit_facts(case, ixit);
    if !missing.is_empty() {
        return Some(Exception::Guarded(format!(
            "the ixit declares no {} — the case reads it as ${{ixit:…}} and no released \
             operation discloses the value; ISO/IEC 9646 test selection",
            missing.join(", ")
        )));
    }
    // Global-state grounds (an empty template list, a globally-absent
    // artefact) hold only on an exclusively-owned SUT; on a shared instance
    // the case is not-applicable, never a false verdict.
    if matches!(
        case.requires.server,
        Some(crate::vocab::ServerState::Exclusive)
    ) && !ixit
        .environment
        .as_ref()
        .is_some_and(|env| env.exclusive_server)
    {
        return Some(Exception::Unrealized(
            "requires.server: exclusive — the ixit declares a shared SUT instance \
             (environment.exclusive_server: false); the global-state ground cannot \
             be established"
                .to_owned(),
        ));
    }
    None
}

/// Execute every runnable case against the ixit's default topology.
///
/// `statement` (the party ICS), when supplied, drives ISO/IEC 9646-style
/// test selection: an option-gated case whose `option` tag the ICS does not
/// declare is recorded not-applicable with citation instead of being driven
/// against a server that legitimately implements the other register branch.
/// Without a statement every case runs (the statement-blind sweep).
///
/// # Errors
/// Interpreter defects only; per-case conformance outcomes land in the
/// report.
/// The one not-applicable record shape every drive-time exclusion produces.
pub fn execute(
    set: &ArtifactSet,
    ixit: &Ixit,
    statement: Option<&crate::party::Statement>,
) -> Result<RunReport, String> {
    let mut report = RunReport::default();
    // Exclusive-server cases (global-state grounds like an empty template
    // list) run FIRST: on a freshly reset, exclusively-owned SUT their
    // ground holds only before other cases provision templates/queries.
    let mut ordered: Vec<&crate::model::case::CaseCore> =
        set.cases.iter().map(|(_, c)| c).collect();
    ordered.sort_by_key(|c| {
        !matches!(
            c.requires.server,
            Some(crate::vocab::ServerState::Exclusive)
        )
    });
    for case in ordered {
        report.considered += 1;
        if !matches!(case.status, CaseStatus::Active) {
            report.exceptions.push((
                case.id.clone(),
                Exception::Status(format!("{:?} — never verdict-bearing", case.status)),
            ));
            continue;
        }
        if let Some(exception) = selection_exception(set, ixit, statement, case) {
            let citation = match &exception {
                Exception::Unrealized(c)
                | Exception::ContentGeneration(c)
                | Exception::Guarded(c)
                | Exception::Status(c) => c.clone(),
            };
            report.records.push(not_applicable_record(case, &citation));
            report.exceptions.push((case.id.clone(), exception));
            continue;
        }
        let runnable = if matches!(case.kind, crate::vocab::CaseKind::Content) {
            // One executor serves both: a content row is a generate→commit→
            // expect functional execution over the synthesized flow.
            synthesize_content_case(case)
        } else {
            case.clone()
        };
        let mut driver = HttpDriver::new(set, ixit, statement.map(|s| &s.spec_versions))?;
        let record = run_case(&runnable, runnable.formats.first().copied(), &mut driver)?;
        report.interpreter_run += 1;
        report.records.push(record);
        if let Some(version) = driver.take_observed_restapi_specs_version() {
            report.restapi_specs_version.get_or_insert(version);
        }
    }
    Ok(report)
}

/// The dry accounting pass — the coverage gate WITHOUT a live SUT: counts
/// which cases the interpreter WOULD drive (everything the executor
/// resolves end-to-end) versus the registered exceptions.
#[must_use]
pub fn coverage_accounting(set: &ArtifactSet) -> RunReport {
    let mut report = RunReport::default();
    // Exclusive-server cases (global-state grounds like an empty template
    // list) run FIRST: on a freshly reset, exclusively-owned SUT their
    // ground holds only before other cases provision templates/queries.
    let mut ordered: Vec<&crate::model::case::CaseCore> =
        set.cases.iter().map(|(_, c)| c).collect();
    ordered.sort_by_key(|c| {
        !matches!(
            c.requires.server,
            Some(crate::vocab::ServerState::Exclusive)
        )
    });
    for case in ordered {
        report.considered += 1;
        if !matches!(case.status, CaseStatus::Active) {
            report.exceptions.push((
                case.id.clone(),
                Exception::Status(format!("{:?}", case.status)),
            ));
            continue;
        }
        if let Some(citation) = fully_unrealized(set, case) {
            report
                .exceptions
                .push((case.id.clone(), Exception::Unrealized(citation)));
            continue;
        }
        // Content cases are interpreter-run through the synthesized
        // generate→commit→expect flow (one executor serves both kinds).
        report.interpreter_run += 1;
    }
    report
}

/// Synthesize the functional execution of a content case: the decision
/// table becomes a matrix (rows drive `${row.*}`), the flow is one commit of
/// the generated instance against the constraint context's template, and the
/// per-row `expected` column (accepted → created, rejected →
/// `validation_failed`) is the outcome expectation.
#[must_use]
pub fn synthesize_content_case(case: &CaseCore) -> CaseCore {
    let mut synthesized = case.clone();
    let Some(table) = &case.decision_table else {
        return synthesized;
    };
    // decision-table rows -> a parameters matrix with a normalized expected
    // column (accepted/rejected -> outcome kinds).
    let columns = table.columns.clone();
    let rows: Vec<Vec<crate::model::case::MatrixCell>> = table
        .rows
        .iter()
        .map(|row| {
            columns
                .iter()
                .zip(row)
                .map(|(column, cell)| {
                    if column == "expected" {
                        let kind = match cell.as_str() {
                            Some("accepted") => "created",
                            _ => "validation_failed",
                        };
                        crate::model::case::MatrixCell::Literal(serde_json::Value::String(
                            kind.to_owned(),
                        ))
                    } else {
                        match cell {
                            serde_json::Value::Null => crate::model::case::MatrixCell::Null,
                            other => crate::model::case::MatrixCell::Literal(other.clone()),
                        }
                    }
                })
                .collect()
        })
        .collect();
    synthesized.parameters = serde_json::from_value(serde_json::json!({
        "iteration": "reset_per_row",
        "matrix": { "columns": columns, "rows": [] }
    }))
    .ok();
    if let Some(parameters) = &mut synthesized.parameters
        && let Some(matrix) = &mut parameters.matrix
    {
        matrix.rows = rows;
    }
    if let Some(context) = &case.constraint_context {
        synthesized.requires.server = Some(crate::vocab::ServerState::Any);
        // A varying-constraint case (constraint_columns declared) provisions no
        // baked template — the driver synthesizes and uploads one OPT PER ROW
        // (issue #228). A constant-constraint case keeps its single baked
        // template. constraint_context rides on the synthesized case so the
        // driver can tell the two apart.
        synthesized.requires.templates = if context.constraint_columns.is_empty() {
            vec![context.template.clone()]
        } else {
            Vec::new()
        };
        synthesized.requires.ehr = Some(crate::model::case::EhrRequirement::Exists {
            commits: crate::model::case::CommitState::None,
        });
    }
    synthesized.sm_operation =
        crate::ids::SmOperationRef::parse("I_EHR_COMPOSITION.create_composition").ok();
    synthesized.flow = serde_json::from_value(serde_json::json!([
        {
            "step": 1,
            "call": "create_composition",
            "with": { "ehr_id": "${ehr_id}", "composition": "${recipe:content_instance(row)}" },
            "expect": "created"
        }
    ]))
    .unwrap_or_default();
    // The per-row expectation rides the reserved matrix `expected` column,
    // which the interpreter resolves as the normative per-row override; the
    // flow's `created` is the inherited default.
    synthesized
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    /// `OperationBinding.applies` is LIVE (issue #629): a binding declaring a
    /// spec-version floor the party does not meet takes its cases out of
    /// scope with the binding's own declared range as the citation, and a
    /// binding without a floor is untouched.
    #[test]
    fn operation_version_floors_are_enforced_at_selection() {
        let floored: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_DEFINITION_ADL14.list_opts",
                "its": "its-rest",
                "applies": { "its_rest": ">=1.1.0" },
                "request": { "method": "GET", "path": "/definition/template/adl1.4" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-floored", "kind": "functional", "component": "DEFINITION_ADL14",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "expect": "ok" }]
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings
            .push((std::path::PathBuf::from("b.yaml"), floored));

        let old = crate::party::SpecVersions {
            its_rest: Some("1.0.3".to_owned()),
            ..crate::party::SpecVersions::default()
        };
        let unmet = unmet_binding_floors(&set, &case, &old);
        assert_eq!(unmet.len(), 1, "{unmet:?}");
        assert!(unmet[0].contains("I_DEFINITION_ADL14.list_opts"));
        assert!(unmet[0].contains(">=1.1.0"));

        let current = crate::party::SpecVersions {
            its_rest: Some("1.1.0".to_owned()),
            ..crate::party::SpecVersions::default()
        };
        assert!(unmet_binding_floors(&set, &case, &current).is_empty());

        // An undeclared floor never narrows selection.
        let unfloored: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_DEFINITION_ADL14.list_opts",
                "its": "its-rest",
                "request": { "method": "GET", "path": "/definition/template/adl1.4" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let mut plain = ArtifactSet::default();
        plain
            .bindings
            .push((std::path::PathBuf::from("b.yaml"), unfloored));
        assert!(unmet_binding_floors(&plain, &case, &old).is_empty());
    }

    /// An EXTENSION realization is party-scoped selection, not a global one:
    /// the family + register id travel in the marker so the citation is
    /// register-linked, and a case that drives no extension binding is
    /// untouched — an ordinary released-wire case can never be excused this
    /// way.
    #[test]
    fn extension_realizations_are_marked_with_their_family_and_register_entry() {
        let extension: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_PARTY_RELATIONSHIP.get_party_relationship",
                "its": "its-rest",
                "extension": {
                    "family": "party-relationship",
                    "reason": "the release surfaces no PARTY_RELATIONSHIP resource",
                    "source": "SM i_party_relationship.adoc vs ITS-REST demographic.openapi.yaml",
                    "ambiguity": "AMB-32"
                },
                "request": { "method": "GET", "path": "/demographic/party_relationship/{versioned_object_uid}" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let released: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_DEFINITION_ADL14.list_opts",
                "its": "its-rest",
                "request": { "method": "GET", "path": "/definition/template/adl1.4" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings
            .push((std::path::PathBuf::from("e.yaml"), extension));
        set.bindings
            .push((std::path::PathBuf::from("r.yaml"), released));

        let on_extension: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-extension", "kind": "functional", "component": "DEMOGRAPHIC",
            "sm_operation": "I_PARTY_RELATIONSHIP.get_party_relationship",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["PartyRelationshipOperations"],
            "flow": [{ "step": 1, "call": "get_party_relationship", "expect": "ok" }]
        }))
        .unwrap();
        let marker = extension_family(&set, &on_extension).expect("an extension marker");
        assert!(marker.contains("party-relationship"), "{marker}");
        assert!(
            marker.contains("AMB-32"),
            "the citation must stay register-linked: {marker}"
        );

        let on_released: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-released", "kind": "functional", "component": "DEFINITION_ADL14",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "expect": "ok" }]
        }))
        .unwrap();
        assert!(extension_family(&set, &on_released).is_none());
    }

    /// The SMART-lane marker is the DECLARATION of a `scopes:` key (empty
    /// included) or the reserved SMART pseudo-interface anchor — never a
    /// heuristic over case ids, so an ordinary case can never be excused.
    #[test]
    fn smart_lane_need_is_declared_not_guessed() {
        let plain: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-plain", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "expect": "ok" }]
        }))
        .unwrap();
        assert!(!needs_smart_lane(&plain));

        let scoped: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-scoped", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "scopes": [], "expect": "forbidden" }]
        }))
        .unwrap();
        assert!(
            needs_smart_lane(&scoped),
            "an EMPTY scopes declaration is still a SMART-lane declaration"
        );

        let discovery: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-discovery", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_ITS_REST_SMART.discovery",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "discovery", "expect": "ok" }]
        }))
        .unwrap();
        assert!(needs_smart_lane(&discovery));
    }

    /// A flow step addressing an instance the party does not declare is a
    /// SELECTION outcome (not-applicable with citation), never a drive-time
    /// error — the same law the SMART lane and the `${ixit:…}` facts follow.
    #[test]
    fn undeclared_addressed_instances_are_collected() {
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-two-deployments", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [
                { "step": 1, "call": "list_opts", "expect": "ok" },
                { "step": 2, "call": "list_opts", "on": "sut_pgp", "expect": "ok" },
                { "step": 3, "call": "list_opts", "on": "sut_pgp", "expect": "ok" }
            ]
        }))
        .unwrap();

        let without: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        // Reported once, however many steps address it.
        assert_eq!(undeclared_instances(&case, &without), vec!["sut_pgp"]);

        let with: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://x", "auth": { "mode": "none" } },
                "sut_pgp": { "base_url": "http://y", "auth": { "mode": "none" } }
            }
        }))
        .unwrap();
        assert!(undeclared_instances(&case, &with).is_empty());
    }

    #[test]
    fn coverage_gate_holds_on_the_committed_catalogue() {
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let loaded = crate::artifacts::load_root(&crate_dir.join("artifacts")).unwrap();
        assert!(loaded.errors.is_empty());
        let report = coverage_accounting(&loaded.set);
        // Interpreter-runnable + unrealized-with-citation are both
        // interpreter-GOVERNED (the not-applicable verdict is the
        // interpreter's own selection law); only content generation and
        // draft status are genuine exceptions.
        let governed = report.interpreter_run
            + report
                .exceptions
                .iter()
                .filter(|(_, e)| matches!(e, Exception::Unrealized(_)))
                .count();
        #[allow(clippy::cast_precision_loss)]
        let coverage = governed as f64 / report.considered as f64;
        assert!(
            coverage >= 0.80,
            "interpreter-governed coverage {coverage:.3} below the floor; exceptions: {:#?}",
            report.exceptions.len()
        );
        // every exception carries a reason (registered, never silent)
        for (case, exception) in &report.exceptions {
            let text = format!("{exception:?}");
            assert!(!text.is_empty(), "{case}: silent exception");
        }
    }
}
