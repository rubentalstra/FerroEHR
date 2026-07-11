# EHRbase (Java) 2.34.0 vs ehrbase-rs — spec-grounded difference analysis

**Objective (owner ruling, 2026-07-11, mid-run):** run our ECC catalogue
against **upstream EHRbase Java 2.34.0** and, for every difference, decide
**who is wrong against the official openEHR specs — Java or our Rust** — because
our own server still has compliance bugs to fix. **Adjudication exemptions are
removed for this run** (no fairness register applied): the numbers below are the
**raw** diff so nothing is hidden.

**Run:** 2026-07-11 · SUT `ehrbase-java` **2.34.0** (digest
`sha256:89e52635fc72dca3eda368b601f3a7aa6cbaa24e7582f5ee00e953759626904d`) ·
base `http://localhost:8091/ehrbase/rest/openehr/v1` · admin
`/ehrbase/rest/admin` (`ADMINAPI_ACTIVE=true`) · Basic auth · **no
`--adjudications`** (raw). Oracle: vendored specs at `docs/specs/openehr/`.
No case was edited or weakened; our own baseline stays **341 / 315 / 0**.

## 1. Raw numbers

| Outcome | Count |
|---|---:|
| Passed | **95** |
| Failed | **212** |
| Skipped (suite self-skip: MSG native-only, etc.) | 26 |
| **Total executed** | **333** |

## 2. The difference ledger — who is wrong, per spec

Every failing family, root cause verified live (§4), mapped to the responsible
side against the vendored spec:

| # | Difference | Cases | Who is wrong per spec | Spec citation |
|---|---|---:|---|---|
| **A** | Java omits mandatory `EHR.ehr_access` from the EHR resource | **7** | **JAVA** (our Rust is correct) | RM EHR `ehr_access` 1..1; ITS-JSON `EHR.required` (all RM releases) |
| **B1** | Our Rust **accepts** `EHR_STATUS.subject._type = PARTY_IDENTIFIED` | **4** | **OUR RUST** (Java correctly rejects, 400) | RM `EHR_STATUS.subject : PARTY_SELF` |
| **B2** | Our Rust rejects an empty (anonymous) `subject: {}` | **1** | **OUR RUST (likely)** — Java accepts; needs review | RM `master04` — "PARTY_SELF allows completely anonymous EHRs" |
| **C** | AQL result-column `path` notation (`/ehr_id/value` vs `e/ehr_id/value`) | **9** | **NEITHER** — spec explicitly undefined | QUERY `master04-result_structure` §annotated results not defined |
| **D** | OPT upload → `406` (our runner sends `Accept: application/json`) | **164** | **OUR RUNNER** (Java strict-correct; our server lenient) | ITS-REST upload op = `application/xml` only, no Accept param |
| **E** | ehrbase-rs features Java does not implement (scope, not a bug) | **27** | neither — feature-scope difference | see §3 |

212 = 7 (A) + 5 (B1+B2) + 9 (C) + 164 (D) + 27 (E).

### The one thing this run CANNOT answer yet

**Validation depth (the 119 VAL cases + 19 COM + 12 CTB + 8 TPL) is
undeterminable** because all of it is blocked at the OPT-upload step by runner
bug **D**. This is exactly the comparison the owner most wants (does Java — or
our Rust — validate archetype constraints as deeply as the AM spec requires?).
It only becomes visible after D is fixed and the suite re-run. Upstream is known
to validate shallowly (blueprint §2.3 row 1), and our own VAL depth is partial
too — **both sides likely have defects here that this run masks.**

## 3. Category E — ehrbase-rs features Java lacks (27, not compliance bugs)

| Feature | Cases | Evidence |
|---|---:|---|
| Demographic REST API (`I_DEMOGRAPHIC_SERVICE` wire) | 23 (DEM) | Java → `404 No resource found` |
| Bulk `DELETE /admin/ehr/all` convenience | 2 (ADM-004/005) | Java → `400 Invalid UUID string: all` |
| AQL `TERMINOLOGY('expand',…)` function | 1 (TS-001) | Java → `400 Not implemented: Only primitive operands are supported` |
| Version signing (`VERSION.signature`) | 1 (SIG) | served `ORIGINAL_VERSION` carries no `signature` |

(3 more SIG/TS cases fail on the OPT-upload runner bug D and are counted there.)
These are scope differences: Java never claims these surfaces. They are *not*
Java defects, and not our bugs — they are ehrbase-rs capabilities beyond the
ITS-REST 1.0.x core. TERMINOLOGY() is an **optional** AQL capability
(QUERY master03 lines 748-767); the others are ehrbase-rs extensions.

## 4. Evidence (live curl, 2026-07-11)

### A — `EHR.ehr_access` omitted → **Java defect, our Rust correct** (7)

```
POST /ehr (no body) → 201 ; GET /ehr/{id} → {_type, system_id, ehr_id, ehr_status, time_created}
```
No `ehr_access`. It is **`1..1` in the RM class model** and in the **`required`
set of every vendored ITS-JSON EHR schema** (`openehr_rm_1.0.3_all.json`,
`_1.0.4_`, `_1.1.0_`) and mandatory in the RM 1.1.0 XSD — so this is **not** an
RM-version wire-skew artefact. ehrbase-rs emits it and is correct.
*(EHR_ACCESS is deprecated-in-practice and Java drops it deliberately; the
vendored spec still mandates it, so per the oracle it is a Java defect.)*
- `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.ehr.ehr.adoc` (ehr_access 1..1)
- `crates/openehr-its/schemas/json/openehr_rm_1.1.0_all.json` (`EHR.required` ∋ `ehr_access`)

Cases: EHR-008, EHR-013, STA-001, STA-003, STA-005, STA-007, STA-009.

### B1 — `subject` = `PARTY_IDENTIFIED` accepted → **our Rust is wrong** (4)

```
POST /ehr  {EHR_STATUS.subject._type:"PARTY_IDENTIFIED"} → Java 400
  "Class PartyIdentified not subtype of PartySelf"
```
`EHR_STATUS.subject` is **`PARTY_SELF` (1..1)** in every RM release; our
generated type is literally `subject: PartySelf`. **Java is correct to reject;
ehrbase-rs wrongly accepts** (our baseline passes EHR-002/005/007/009). This is
a leniency bug in our canonical-JSON `_type` handling: a foreign concrete
`_type` in a `PARTY_SELF` slot must be rejected. **Fix target:** the
`OpenEhrType` deserialize / RM validation in `openehr-its` (likely affects any
monomorphic slot that currently tolerates a wrong `_type`, not just PARTY_SELF).
- `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.ehr.ehr_status.adoc` (`subject: PARTY_SELF`)
- `docs/specs/openehr/RM/docs/ehr/master04-ehr_package.adoc:44,219`

*(The runner ALSO has a fixture bug here — `ehr_status_row` in
`tools/conformance/src/suites/ehr.rs` builds `PARTY_IDENTIFIED`; the vendored
corpus fixture uses `PARTY_SELF`. Fixing the fixture will stop our server
passing on leniency, exposing B1 in our own suite.)*

### B2 — anonymous `subject: {}` rejected → **our Rust likely wrong** (1, EHR-012)

Java accepts 2 of 11 "invalid" fixtures; both acceptances are spec-defensible:
- `001_ehr_status_subject_empty.json` — `subject: {}` = empty `PARTY_SELF` = a
  **valid anonymous EHR** ("PARTY_SELF allows completely anonymous EHRs").
- `000_ehr_status_type_missing.json` — a top-level `_type` inferable from the
  `create_ehr` endpoint contract.

Our baseline rejects all 11, so **ehrbase-rs may be over-strict on anonymous
EHRs** — review against `master04-ehr_package.adoc:44,219` and relax if warranted.

### C — AQL column `path` notation → **neither wrong** (9)

```
golden:   [{"name":"#0","path":"/ehr_id/value"}]
Java:     [{"name":"#0","path":"e/ehr_id/value"}]
```
The AQL spec **explicitly does not define** result-set column descriptors:
"annotated results … not formally defined by this specification … an artefact of
the relevant API or service definition" —
`docs/specs/openehr/QUERY/docs/AQL/master04-result_structure.adoc:5`. Both
notations are valid; the ECC goldens encode *our* convention. (Deeper AQL
divergences may hide behind this first reported one — normalise the notation in
the comparison before drawing an AQL-depth conclusion.)
Cases: QRY-002/003/006/007/008/009/010/011/013.

### D — OPT upload `406` → **our runner bug; Java strict-correct** (164)

```
POST /definition/template/adl1.4  Content-Type: application/xml  Accept: application/json → 406
POST /definition/template/adl1.4  Content-Type: application/xml  (no Accept)              → 201 (application/xml)
```
Our runner (`tools/conformance/src/suites/support.rs:56-58,88-90`) sends
`Accept: application/json`. Per ITS-REST the upload endpoint **produces
`application/xml` only** — it declares **no `Accept` parameter** and its `201`
body is `OperationalTemplate`/xml:
- `docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl1.4_upload.yaml`
- `docs/specs/openehr/ITS-REST/specifications/responses/201_Template_adl1_4_upload.yaml`

So Java's `406` is spec-correct; ehrbase-rs `2xx` is lenient. **Proof Java is
otherwise functional** — after provisioning OPTs without the bad header:
```
POST /ehr/{id}/composition  nested.en.v1__full.json               → 201
POST /ehr/{id}/composition  nested.en.v1__invalid_wrong_structure  → 400
```
Cases: all VAL (119), COM (19), CTB (12), TPL (8), QRY-004, + SIG/TS overflow.

## 5. Action list

**Fix in our Rust server (compliance — the owner's priority):**
1. **B1** — reject a foreign concrete `_type` in a `PARTY_SELF` (and every
   monomorphic-slot) canonical-JSON deserialize; today we silently accept
   `PARTY_IDENTIFIED` as an `EHR_STATUS.subject`. *(openehr-its `_type`
   validation / RM validation.)*
2. **B2** — review anonymous-EHR handling; likely relax to accept `subject: {}`.
3. **D (server side)** — decide the correct content-negotiation for OPT upload
   (spec = `application/xml`); our leniency isn't wrong per se but should be
   deliberate, not accidental.

**Fix in our ECC runner (unblocks the real comparison):**
4. **R1 / D** — OPT upload must send `Accept: application/xml` (or none), not
   `application/json` (`suites/support.rs`). Unblocks 164 cases and the whole
   VAL/COM/CTB/TPL depth comparison.
5. **R2 / B1** — build `EHR_STATUS.subject` as a `PARTY_SELF` with `external_ref`
   (`suites/ehr.rs ehr_status_row`), matching the vendored corpus.
6. **R3 / C** — normalise the AQL result-column `path` notation in the golden
   comparison (spec-undefined), so a valid alternative notation isn't a failure.

**Genuine upstream (Java) finding, per spec:**
7. **A** — Java omits the mandatory `EHR.ehr_access`. (Report; do not work around.)

**Re-run after 4/5/6** to get the validation-depth verdict on both servers —
that is where the remaining, most important, compliance differences live.

## 6. Open questions for the orchestrator

- **B2 / EHR-012**: is our anonymous-EHR rejection a bug, or is the corpus's
  "invalid" labelling wrong? (Both readings are spec-plausible; leaning our bug.)
- **VAL depth (blocked)**: after fixing D, how do Java and our Rust each compare
  to the AM constraint-validation spec? Expect defects on **both** sides.
- Whether to re-introduce fairness adjudications (extensions → N/A) for the
  eventual public comparison page, or keep the raw diff as the working
  instrument for the compliance rewrite. (Owner removed them for now.)
