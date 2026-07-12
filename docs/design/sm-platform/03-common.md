# SM Common Package (`platform.common`) — spec-conformance audit

Read-only audit (2026-07-12) of the SM common package against its
implementation. This chapter is small, foundational, and — unlike the
Subject Proxy service (`10-subject-proxy.md`) — realized with **high
fidelity**: every `CALL_STATUS_TYPE` member is present and correctly named,
`UPDATE_VERSION`/`UPDATE_AUDIT` follow §Version Update Semantics, and
`I_VALIDITY_CHECKER` is implemented. The gap register below is therefore short
and made mostly of documented deviations, one dead-code observation, and two
dangling doc references — not missing behaviour.

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master03-common_package.adoc`
  — the chapter: §Overview, §Representing Call Status, §Version Update
  Semantics, §Class Definitions.
- Its `include::`d class files under `docs/specs/openehr/SM/docs/UML/classes/`:
  `i_status.adoc`, `call_status.adoc`, `call_status_type.adoc`,
  `update_version.adoc`, `update_audit.adoc`, `i_validity_checker.adoc`.
- `platform_service.adoc` — named in the chapter §Overview as a common-package
  member but **not** `include::`d in §Class Definitions (see G-7).
- Adjacent: `master02-overview.adoc` §Functional Style + §List Handling
  (the stateless-mapping and paging sanctions the impl leans on).

**Current implementation** (verified 2026-07-12):

- Call-status model: `app/ehrbase-sm/src/error.rs`
  — `CallStatusType` enum (error.rs:43), `SmError` (error.rs:172),
  `CallStatus` struct (error.rs:222).
- Version-commit envelope + `PLATFORM_SERVICE`: `app/ehrbase-sm/src/types.rs`
  — `UpdateAudit` (types.rs:77), `UpdateAttestation` (types.rs:101),
  `UpdateVersion<T>` (types.rs:146), `PlatformService` (types.rs:427).
- `I_VALIDITY_CHECKER` trait: `app/ehrbase-sm/src/services/validity.rs:14`;
  concrete impl `app/ehrbase/src/service/api/mod.rs:57`.
- SM → HTTP mapping (the wire realization of `CALL_STATUS`):
  `app/ehrbase-rest/src/error.rs:51` (`sm_api_error`).
- `I_STATUS`: no dedicated type — mapped to the stateless `Result<T, SmError>`
  style (documented `app/ehrbase-sm/src/error.rs:9-13`).

---

## 1. Requirement inventory (what the chapter defines)

| Spec element | Members / attributes | Where in spec |
|---|---|---|
| `I_STATUS` | `last_call_failed(): Boolean`, `last_call_status(): CALL_STATUS` | `i_status.adoc` |
| `CALL_STATUS` | `code`, `call_name`, `call_string`, `meaning`, `message` (all `1..1`) | `call_status.adoc` |
| `CALL_STATUS_TYPE` | `success`, `auth_failure`, `precondition_violation`, `object_version_does_not_exist`, `versioned_object_does_not_exist`, `exception`, `ehr_id_does_not_exist`, `party_id_does_not_exist`, `file_not_writable`, `version_mismatch` (10 members) | `call_status_type.adoc` |
| `UPDATE_VERSION<T>` | `preceding_version_uid [0..1]`, `lifecycle_state [1..1]`, `attestations [0..1]`, `data [1..1]`, `audit [1..1]` | `update_version.adoc` |
| `UPDATE_AUDIT` | `change_type [1..1]`, `description [0..1]`, `committer [1..1]`; invariant `Change_type_valid` | `update_audit.adoc` |
| `I_VALIDITY_CHECKER` | `definitions_valid(a_content): Boolean`, `content_valid(a_content): Boolean` | `i_validity_checker.adoc` |
| `PLATFORM_SERVICE` | `Admin`, `Definitions`, `Ehr`, `Ehr_index`, `Demographic`, `Message`, `Query`, `System_log` (8 members) | `platform_service.adoc` (§Overview mention only) |

**Member-by-member verdict on `CALL_STATUS_TYPE` (the audit's core question):**
all 10 base members are present in `CallStatusType` (error.rs:46-66) with
byte-exact `sm_name()` literals (error.rs:127-137). **None dropped.** The enum
is a *superset*: it also carries `EHR_CALL_STATUS_TYPE` /
`DEFINITION_CALL_STATUS_TYPE` descendant codes (error.rs:75-102) and
prose-only names (error.rs:104-119) — all spec-sanctioned by
`master03 §Representing Call Status` ("Particular services may add more codes
by inheriting from this class"). The only genuine **invention** is
`NotImplemented` (error.rs:71), explicitly documented as a non-SM adapter
affordance for `501` routes — see G-2.

---

## 2. Gap register (what is not spec-true today)

Every gap cites the governing spec text. All are LOW severity — the model is
faithful; these are deviations-with-rationale, a dead struct, and stale doc
links, not behavioural holes.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **`CALL_STATUS`'s informational fields are never surfaced.** The `CallStatus` struct (all five mandatory attributes: `code`, `call_name`, `call_string`, `meaning`, `message`) is modelled faithfully but is **dead code** — it is constructed nowhere in the workspace (only its `impl` at error.rs:235). The live error path is `SmError` (error.rs:172), which carries only `{status, message}`; at the wire (`sm_api_error`, ehrbase-rest error.rs:51) the response body is `{error, message}` or the ITS-REST `Error` object. So `call_name`/`call_string`/`meaning` are dropped everywhere they would appear. | `call_status.adoc` (all attrs `1..1`); `master03 §Representing Call Status` | Modelled, never used. Defensible — the SM obtains `CALL_STATUS` after-the-fact via `I_STATUS.last_call_status()`, and ITS-REST 1.0.3 defines its own error body (the wire wins) — but the model→wire drop should be an explicit note, not an unused struct pretending otherwise. |
| G-2 | **`NotImplemented` is a non-spec `CALL_STATUS_TYPE` member.** `CALL_STATUS_TYPE` defines no `not_implemented`; the enum invents it (error.rs:71) for `501` routes and mock seams. | `call_status_type.adoc` (10 members, no `not_implemented`) | Present and documented as an adapter affordance (→ `501`, ehrbase-rest error.rs:79). A legitimate extension via the spec's own inheritance mechanism, but it is *our* code, not a vendored `*_CALL_STATUS_TYPE` descendant — worth a standing PORT NOTE (already present in-code). |
| G-3 | **`version_mismatch` semantics are our interpretation of a blank-meaning member.** The spec leaves the `version_mismatch` *Meaning* cell **empty** (`call_status_type.adoc:52-53`); we assign it optimistic-concurrency (`If-Match` → `412`) semantics (error.rs:63-66, ehrbase-rest error.rs:68). | `call_status_type.adoc` (blank meaning); ITS-REST `If-Match`/`412` | Interpreted, cited to the wire. Correct reading, but it is an inference over a spec gap — recorded here so it is not mistaken for a spec-stated meaning. |
| G-4 | **`I_STATUS` is realized statelessly, with no `last_call_failed`/`last_call_status` surface.** The spec interface is stateful ("obtain status of previous calls; use by inheritance"). We map it to `Result<T, SmError>`; there is no method that answers "did the *last* call fail" against retained state. | `i_status.adoc`; sanctioned by `master02 §Functional Style` | Fully documented (error.rs:9-13) and spec-sanctioned ("Either style can be used, and can be trivially mapped"). `last_call_failed()` ≡ `Result::is_err()`, `last_call_status()` ≡ the returned `SmError`. Compliant by the stateless-mapping allowance; no code change needed — noted for completeness. |
| G-5 | **`UPDATE_VERSION.attestations` carries the wire-partial `UpdateAttestation`, not full RM `ATTESTATION`.** `master03 §Version Update Semantics` says "`ATTESTATION` instances can be supplied in their full form"; `update_version.adoc` types `attestations` as `List<ATTESTATION>`. Our field is `Option<Vec<UpdateAttestation>>` (types.rs:156), a partial form the server completes. | `master03 §Version Update Semantics`; `update_version.adoc` | Deliberate: the ITS-REST wire (`UpdateVersion.yaml` / `UpdateAttestation.yaml`) carries the partial shape, and the standing wire-precedence rule makes the wire win at the boundary (documented PORT NOTE, types.rs:89-99, 137-144). A divergence from the *SM prose*, reconciled with the *wire oracle* — legitimate, cited. |
| G-6 | **`UPDATE_VERSION` carries a `signature` field absent from the SM class.** `update_version.adoc` defines no `signature`; ours adds `signature: Option<String>` (types.rs:164). | `update_version.adoc` (no such attribute) | Wire-driven (ITS-REST `UpdateVersion.yaml` carries it; fed to the `signing` module). Documented PORT NOTE (types.rs:143-144). Extension over the SM, justified by the wire. |
| G-7 | **`PLATFORM_SERVICE` is realized but not part of this chapter's `include::` set.** `master03 §Overview` names `PLATFORM_SERVICE` as a common-package member, yet §Class Definitions (master03:31-43) does **not** `include::` `platform_service.adoc` — the chapter describes it in prose but omits its class table. Our `PlatformService` (types.rs:427) implements the 8 vendored members faithfully and PORT-NOTEs the spec's own omission of `Terminology`/`Subject_proxy`. | `master03 §Overview` vs §Class Definitions; `platform_service.adoc` | Implementation is correct and honestly annotated. The **spec defect** is the missing `include::` in master03 — record it as a chapter TBD (like the SPS orphan classes), not an implementation task. |
| G-8 | **`I_VALIDITY_CHECKER.definitions_valid` checks templates only, not archetypes.** The spec says "archetype **and** template identifiers"; the impl resolves the template id and returns `true` when content declares none (ehrbase api/mod.rs:58-66). | `i_validity_checker.adoc` (`definitions_valid`) | Documented PORT NOTE (api/mod.rs:50-55): no archetype store exists yet, so archetype-identifier resolution is unimplemented. A real (if minor) coverage gap in the validity check — closes when an archetype store lands. Trait defaults return `not_implemented` (validity.rs:18-33); the concrete impl overrides both. |
| G-9 | **Two dangling design-doc references in common-package code.** error.rs:9-13/32, types.rs:8, and ehrbase-rest error.rs:43 cite `docs/design/sm-platform/02-ehr-service.md` and `08-target-architecture.md`, which no longer exist (only `10-subject-proxy.md` + `README.md` remain). | n/a (repo hygiene) | Same rot the SPS redesign flagged. Scrub to cite this chapter (`03-common.md`) and the spec files directly. |

**Not gaps (verified faithful):** `UPDATE_VERSION` cardinalities match
§Version Update Semantics exactly — `preceding_version_uid` optional
(`None` only for a first version, types.rs:149-150), `lifecycle_state`
mandatory (types.rs:152); `UPDATE_AUDIT` has the exact three attributes with
`description` optional (types.rs:78-87); the `commit_audit` serde rename
(types.rs:160) matches the wire; the `Change_type_valid` invariant is
delegated to the service boundary via `openehr-term` (documented, types.rs:72).

---

## 3. Target design (the small corrective set)

This chapter needs **no structural redesign** — only three honest touch-ups.
No behavioural change to the wire or the version-commit path.

1. **Resolve the `CallStatus` dead-struct (G-1).** Either (a) delete the
   unused `CallStatus` struct + `impl` (error.rs:222-253) and keep the model
   at `SmError`, documenting in the module header that the SM's five
   `CALL_STATUS` informational fields collapse to `{status, message}` because
   ITS-REST 1.0.3 owns the wire error body; or (b) actually populate and
   surface it (e.g. an optional structured `CALL_STATUS` block in error
   responses) — only if a consumer needs `call_name`/`meaning`. Prefer (a):
   the wire is the oracle and defines its own error body, so a faithful-but-
   unused model is misleading rather than compliant. Whichever is chosen, the
   drop from five fields to two must be an explicit, cited note.
2. **Record the `version_mismatch` interpretation (G-3) and the `PLATFORM_SERVICE`
   include omission (G-7) as chapter spec-defects** in this document's §5 (done
   below) so they are not re-litigated. No code change.
3. **Scrub the dangling doc references (G-9)** in `app/ehrbase-sm/src/error.rs`,
   `app/ehrbase-sm/src/types.rs`, and `app/ehrbase-rest/src/error.rs` to point
   at `03-common.md` and the spec files, matching the spec-citation-only rule.

Everything else (G-2, G-4, G-5, G-6, G-8) is a standing, already-documented
PORT NOTE and stays as-is — see §5.

---

## 4. Verification

- **Existing coverage is adequate for the model.** error.rs:259 asserts every
  `CallStatusType` has a distinct `sm_name`; types.rs:591 round-trips the
  wire-shaped `UPDATE_VERSION` (`commit_audit`, partial attestations,
  `signature`); ehrbase-rest error.rs:151-183 asserts the two error-body shapes
  and the `422` validation body.
- **Add on touch-up 1:** if `CallStatus` is deleted, a compile check suffices;
  if surfaced, a wire test asserting the structured block. On G-8, an
  archetype-store landing adds a `definitions_valid` archetype-resolution test.
- **SM → HTTP completeness** is already compiler-enforced: `CallStatusType` is
  deliberately not `#[non_exhaustive]` (error.rs:36-41), so a new variant
  fails to compile until `sm_api_error` maps it — the mapping table cannot
  silently miss a status.
- Gates unchanged: workspace suites green, clippy clean, ECC zero-drift.

---

## 5. Standing PORT NOTEs / spec defects (the honest residue)

- **`NotImplemented` (G-2):** a non-vendored `CALL_STATUS_TYPE` code, added as
  a `501` adapter affordance via the spec's own inheritance mechanism.
- **`version_mismatch` meaning (G-3):** the vendored enum leaves the *Meaning*
  cell blank (`call_status_type.adoc:52-53`); we implement the evident intent —
  optimistic-concurrency failure → `412` — and record the blank as a spec TBD.
- **`I_STATUS` stateless mapping (G-4):** realized as `Result<T, SmError>` per
  the `master02 §Functional Style` "either style" sanction; no retained
  last-call state.
- **`UPDATE_VERSION` wire divergences (G-5/G-6):** `attestations` carries the
  ITS-REST partial `UpdateAttestation` (not full RM `ATTESTATION`), and a
  wire-only `signature` field is added — both because the wire oracle wins at
  the boundary; the SM prose divergence is documented in-code.
- **`definitions_valid` template-only (G-8):** archetype-identifier resolution
  is unimplemented until an archetype store exists; content with no template
  resolves `true`.
- **`PLATFORM_SERVICE` chapter omission (G-7 — spec defect):** `master03`
  names the enum in §Overview but does not `include::` `platform_service.adoc`
  in §Class Definitions; separately, `platform_service.adoc` omits
  `Terminology`/`Subject_proxy` though the SM defines those interfaces. Both are
  vendored-spec defects recorded here, not implementation tasks; our
  `PlatformService` carries the eight vendored members verbatim.
