---
name: rm-archetyped-master03-overview
description: Verified findings for RM common master03 §3.1 (PATHABLE/LOCATABLE/Feeder audit prose) — where the app enforces root-LOCATABLE rules and where it does not, plus the FEEDER_AUDIT.change_type spec-internal gap
metadata:
  type: feedback
---

Verified 2026-08-01 against RM 1.2.0
`docs/specs/openehr/RM/docs/common/master03-archetyped_package.adoc` §Overview.

**Root-LOCATABLE enforcement is ASYMMETRIC (confirmed defect).**
`app/ferroehr/src/service/ehr/validation.rs:338 validate_root_locatable`
(archetype_details mandatory + root `archetype_node_id` == stringified
`archetype_details.archetype_id.value` + `Links_valid`) is called ONLY from
`validation.rs:420` (EHR_STATUS), `:549` (EHR_ACCESS) and
`service/demographic/validate.rs:126` (PARTY). COMPOSITION
(`service/ehr/composition.rs`) and FOLDER (`validate_folder`,
`validation.rs:580`) never call it — a COMPOSITION whose root
`archetype_node_id` disagrees with its `archetype_details.archetype_id` commits
200/201. `flat/validation/mod.rs check_archetyped_valid` (:731) only enforces
the NEGATIVE arm (at/id-code node must NOT carry archetype_details); no
positive equality check exists anywhere in `openehr-its` or `app/ferroehr`.
Same asymmetry for `LOCATABLE.Links_valid` (`"links": []` accepted on
COMPOSITION/FOLDER; `check_nonempty_lists` :775 has no `links` rule).

**§3.1 prose references a nonexistent attribute.** "Structural Correspondence"
says to mark a synthesised node by setting FEEDER_AUDIT's `change_type` to
"synthesised", but RM 1.2.0 `UML/classes/org.openehr.rm.common.feeder_audit.adoc`
declares only 5 attributes (no change_type) and `…feeder_audit_details.adoc`
likewise. Terminology group "audit change type" has 252 `synthesis`
(TERM `SupportTerminology/master04-representation.adoc:78`) but it is only
reachable from AUDIT_DETAILS.change_type. Unrepresentable → no ambiguity-register
entry exists (Veredictum's `artifacts/registers/ambiguities.yaml` has no
FEEDER_AUDIT change_type entry).

**Already-adjudicated, do NOT re-report:** the uid-copy form (RM says copy the
object_id GUID only; ITS-REST says copy the full 3-part id) is AMB-65
(ambiguities.yaml:1646, disposition report_only, upstream #1511); the server
stamps the FULL OBJECT_VERSION_ID in `versioning/change.rs:448
stamp_version_uid`, cited correctly.

**Conformant, verified:** all 5 PATHABLE spec functions + parent exist in
`crates/openehr-rm/src/paths.rs` (:755/:779/:786/:793/:802/:823) over the
canonical-JSON tree, consumed at `service/ehr/uri.rs:106`. No
identical-content refusal on update (Version Detection prose satisfied);
`reject_duplicate_persistent` (validation.rs:39) is create-only + CNF-derived.
feeder_audit survives storage (not a structure type → stays inline in the
node `data` fragment, `storage/codec.rs`) and is not pinned across versions
(`check_versioned_composition_invariants` :239 pins only archetype_node_id +
is_persistent).

**Coverage gaps:** no CNF case commits canonical JSON carrying feeder_audit
(only `simplified_formats/SF-MAP-events_audit.yaml` via FLAT at content[0], and
`composition/…null_empty_absent.yaml` asserts ABSENCE); no case commits a new
version with unchanged content; no case with feeder_audit at CLUSTER/ELEMENT
granularity.
