//! The claim-completeness battery (issue #622): the four gates that make a
//! hollow certification claim unrepresentable are each proven by SEEDING the
//! violation into a copy of the production world and asserting the gate
//! fires. A gate with no failing case is a wish, not a rule
//! (`.claude/rules/reliability.md`).
//!
//! The world here is BOTH committed halves — the artifact tree AND the party
//! statements beside it — because every one of these gates is a relation
//! between a claim and the catalogue.
#![allow(clippy::panic, clippy::expect_used)] // test assertions/fixtures

use cnf_runner::artifacts::load_root;
use cnf_runner::validate::{Context, Finding, validate};

fn crate_dir() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn specs() -> std::path::PathBuf {
    crate_dir()
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("docs/specs/openehr")
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// A temp copy of the production world: `<tmp>/artifacts` + `<tmp>/party`,
/// the same sibling layout `load_root` sweeps.
struct World {
    dir: assert_fs::TempDir,
}

impl World {
    fn new() -> Self {
        let dir = assert_fs::TempDir::new().expect("temp dir");
        copy_tree(
            &crate_dir().join("artifacts"),
            &dir.path().join("artifacts"),
        );
        copy_tree(&crate_dir().join("party"), &dir.path().join("party"));
        Self { dir }
    }

    fn edit(&self, relative: &str, edit: impl FnOnce(String) -> String) {
        let path = self.dir.path().join(relative);
        let before = std::fs::read_to_string(&path).expect("read");
        let after = edit(before.clone());
        assert_ne!(
            after, before,
            "the seeded edit of {relative} changed nothing"
        );
        std::fs::write(&path, after).expect("write");
    }

    fn findings(&self) -> Vec<Finding> {
        let loaded = load_root(&self.dir.path().join("artifacts")).expect("schema compilation");
        validate(&Context {
            set: &loaded.set,
            load_errors: &loaded.errors,
            spec_root: Some(&specs()),
        })
    }
}

fn assert_gate(findings: &[Finding], gate: &str, fragment: &str) {
    let hit = findings
        .iter()
        .any(|f| f.check.token() == gate && f.message.contains(fragment));
    assert!(
        hit,
        "expected a `{gate}` finding containing {fragment:?}, got:\n{}",
        findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

const MATRIX: &str = "artifacts/vocab/capability_matrix.yaml";
const EHR_OPS_HEAD: &str =
    "EhrOperations: { family: Platform, tier: CORE, required: true, min_cases: 23,";

/// A matrix row no catalogue case names, seeded to construct the hollow claim
/// the `claim-completeness` gate exists to reject. Every committed row now
/// carries a battery (#610/#624), so the violation is BUILT rather than
/// borrowed from a transient catalogue hole — the test can no longer pass or
/// fail for reasons unrelated to the gate.
const SEEDED_HOLLOW_ROW: &str = "SeededHollowCapability: { family: Platform, tier: OPTIONS, \
     required: false, min_cases: 0, source: \"seeded defect — no catalogue case names this \
     capability\" }";

/// The committed world — artifact tree AND party statements — is clean under
/// every gate, including the four new ones. Everything below seeds one
/// violation into a copy of exactly this world, so any finding is
/// attributable to the seed.
#[test]
fn the_committed_world_including_party_claims_is_clean() {
    let loaded = load_root(&crate_dir().join("artifacts")).expect("schema compilation");
    assert!(
        !loaded.set.parties.is_empty(),
        "the party sweep found no statement.json beside the artifact root — every claim gate \
         below would be vacuous"
    );
    let findings = validate(&Context {
        set: &loaded.set,
        load_errors: &loaded.errors,
        spec_root: Some(&specs()),
    });
    assert!(
        findings.is_empty(),
        "the committed artifact tree + party statements must be clean, found:\n{}",
        findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The floors are DERIVED, not asserted: every committed `min_cases` must be
/// at or below the count the catalogue actually carries today. That
/// inequality IS the ratchet — raising a floor to the current depth is always
/// safe, and a battery that later shrinks below its floor fails
/// `capability-depth`.
#[test]
fn every_committed_floor_is_at_or_below_its_derived_count() {
    let loaded = load_root(&crate_dir().join("artifacts")).expect("schema compilation");
    let (_, matrix) = loaded.set.matrix.as_ref().expect("capability matrix");
    let mut floors = 0_usize;
    for (name, entry) in matrix.entries() {
        let derived = cnf_runner::validate::verdict_bearing(&loaded.set, name).len();
        assert!(
            entry.min_cases <= derived,
            "{name}: committed floor {} exceeds the derived verdict-bearing count {derived}",
            entry.min_cases
        );
        floors += entry.min_cases;
    }
    assert!(
        floors > 0,
        "every floor is zero — the depth gate would be vacuous"
    );
}

/// (1) A claim without cases: declaring a capability IS the obligation to run
/// the framework against it, so a claim the catalogue cannot test at all
/// fails before any SUT is composed.
#[test]
fn a_claimed_capability_with_no_cases_fails_validate() {
    let world = World::new();
    // Seed a matrix row no case names, then claim it: that pairing IS the
    // hollow claim, independent of whatever the committed catalogue happens
    // to cover today.
    world.edit(MATRIX, |text| {
        text.replacen(
            EHR_OPS_HEAD,
            &format!("{SEEDED_HOLLOW_ROW}\n{EHR_OPS_HEAD}"),
            1,
        )
    });
    world.edit("party/ehrbase-rs/statement.json", |text| {
        text.replacen(
            "\"capabilities\": [\n",
            "\"capabilities\": [\n      \"SeededHollowCapability\",\n",
            1,
        )
    });
    assert_gate(
        &world.findings(),
        "claim-completeness",
        "claimed capability SeededHollowCapability has zero verdict-bearing catalogue cases",
    );
}

/// (1b) A capability whose every case resolves excused must NAME the register
/// entry that adjudicated it — an unevidenceable claim is a certification
/// hole, not a selection outcome.
#[test]
fn an_excused_only_capability_without_its_adjudication_fails_validate() {
    let world = World::new();
    // Delete the first authored `evidence_exception` block, from its leading
    // comma through the `" }` that closes it (the reason is the block's only
    // quoted scalar, so the closing `" }, ` is unambiguous).
    world.edit(MATRIX, |text| {
        let start = text
            .find(", evidence_exception: {")
            .expect("an evidence_exception block is authored");
        let end = start
            + text
                .get(start..)
                .expect("slice")
                .find("\" }, ")
                .expect("block end")
            + 3;
        let mut out = text;
        out.replace_range(start..end, "");
        out
    });
    assert_gate(
        &world.findings(),
        "claim-completeness",
        "resolves excused or deselected",
    );
}

/// (1c) The mirror ratchet: an `evidence_exception` on a capability that CAN
/// carry executed evidence is stale and must be deleted, so the excuse can
/// never outlive the wire it excused.
#[test]
fn a_stale_evidence_exception_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace(
            EHR_OPS_HEAD,
            "EhrOperations: { family: Platform, tier: CORE, required: true, min_cases: 23, \
             evidence_exception: { register: AMB-34, reason: \"stale\" },",
        )
    });
    assert_gate(
        &world.findings(),
        "claim-completeness",
        "EhrOperations: evidence_exception (AMB-34) is stale",
    );
}

/// (2) The depth floor: one token case never certifies a capability, and a
/// battery that shrinks below its recorded floor is a finding naming the
/// shortfall.
#[test]
fn a_battery_below_its_min_cases_floor_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace(
            EHR_OPS_HEAD,
            "EhrOperations: { family: Platform, tier: CORE, required: true, min_cases: 999,",
        )
    });
    assert_gate(
        &world.findings(),
        "capability-depth",
        "EhrOperations: 23 verdict-bearing case(s) against a floor of 999 — short by 976",
    );
}

/// (3) The measured workload: a claimed capability the hospital simulation
/// never touches needs an adjudicated exclusion, so the certificate can never
/// carry a bare `NO — catalogue gap` row.
#[test]
fn an_unexercised_claimed_capability_without_an_exclusion_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace(
            ", workload_exclusion: { register: AMB-170, reason: \"not yet exercised by the \
             hospital simulation - pending journey-catalogue extension (#625)\" }",
            "",
        )
    });
    assert_gate(
        &world.findings(),
        "workload-coverage",
        "is neither exercised by the measured hospital-simulation workload nor carries a \
         `workload_exclusion`",
    );
}

/// (3b) The mirror ratchet: an exclusion on a capability the simulation now
/// exercises is stale, so a landed journey forces the row's deletion.
#[test]
fn a_stale_workload_exclusion_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace(
            EHR_OPS_HEAD,
            "EhrOperations: { family: Platform, tier: CORE, required: true, min_cases: 23, \
             workload_exclusion: { register: AMB-170, reason: \"stale\" },",
        )
    });
    assert_gate(
        &world.findings(),
        "workload-coverage",
        "EhrOperations: workload_exclusion (AMB-170) is stale",
    );
}

/// (4) The realization marker: an `extension` capability is verified over
/// routes no openEHR specification governs, so it may never be `required` —
/// no openEHR profile tier may rest on our own extension surface.
#[test]
fn a_required_extension_capability_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace(
            EHR_OPS_HEAD,
            "EhrOperations: { family: Platform, tier: CORE, required: true, min_cases: 23, \
             realization: extension,",
        )
    });
    assert_gate(
        &world.findings(),
        "vocab-drift",
        "EhrOperations: realization `extension` may not be `required`",
    );
}

const RELATIONSHIP_READ_BINDING: &str =
    "artifacts/bindings/its-rest/I_PARTY_RELATIONSHIP.get_party_relationship.yaml";
const RELATIONSHIP_ROW_HEAD: &str = "PartyRelationshipOperations: { family: Platform, tier: \
                                     OPTIONS, required: false, realization: extension,";

/// (5) Realization scoping, issue #623 — an extension binding may only drive a
/// route the SUT DECLARES outwardly, so an undeclared family is a finding. The
/// fence is what keeps "our own extension" from becoming an unaudited way to
/// claim capabilities over routes nobody wrote down.
#[test]
fn an_extension_binding_naming_an_undeclared_family_fails_validate() {
    let world = World::new();
    world.edit(RELATIONSHIP_READ_BINDING, |text| {
        text.replace("family: party-relationship", "family: no-such-family")
    });
    assert_gate(
        &world.findings(),
        "realization-scope",
        "extension family \"no-such-family\" is not declared in the served_extensions axis",
    );
}

/// (5b) …and only a route that family actually lists: a path the axis never
/// declares is an undeclared surface, however plausible it looks.
#[test]
fn an_extension_binding_driving_an_undeclared_route_fails_validate() {
    let world = World::new();
    world.edit(RELATIONSHIP_READ_BINDING, |text| {
        text.replace(
            "path: /demographic/party_relationship/{versioned_object_uid}",
            "path: /demographic/party_relationship_undeclared",
        )
    });
    assert_gate(
        &world.findings(),
        "realization-scope",
        "is not one of the routes the \"party-relationship\" served_extensions family declares",
    );
}

/// (5c) A capability whose cases ALL drive extension routes must SAY so: a
/// released-wire marker on such a row would claim openEHR wire conformance the
/// release does not define.
#[test]
fn an_extension_only_capability_marked_released_wire_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace(
            RELATIONSHIP_ROW_HEAD,
            "PartyRelationshipOperations: { family: Platform, tier: OPTIONS, required: false,",
        )
    });
    assert_gate(
        &world.findings(),
        "realization-scope",
        "PartyRelationshipOperations: every one of its 15 verdict-bearing case(s) drives \
         EXTENSION routes only",
    );
}

/// (5d) The mirror ratchet: an `extension` marker on a capability whose cases
/// drive RELEASED operations is stale and understates the conformance the
/// product earned, so it must go.
#[test]
fn a_stale_extension_realization_marker_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace(
            "PartyOperations: { family: Platform, tier: OPTIONS, required: false, min_cases: 52,",
            "PartyOperations: { family: Platform, tier: OPTIONS, required: false, realization: \
             extension, min_cases: 52,",
        )
    });
    assert_gate(
        &world.findings(),
        "realization-scope",
        "PartyOperations: `realization: extension` is stale",
    );
}

/// An unresolvable register link in either adjudication block is caught — the
/// blocks are register-LINKED by construction, never free prose.
#[test]
fn an_adjudication_citing_an_absent_register_entry_fails_validate() {
    let world = World::new();
    world.edit(MATRIX, |text| {
        text.replace("register: AMB-170", "register: AMB-99999")
    });
    assert_gate(
        &world.findings(),
        "workload-coverage",
        "workload_exclusion cites AMB-99999 which is not in the register",
    );
}
