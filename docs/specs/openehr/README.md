# Vendored openEHR specifications (normative text)

The **authoritative openEHR spec text**, vendored verbatim from the official
`github.com/openEHR/specifications-*` repos at the pins in `docs/VERSIONS.md`.
**This is the conformance authority for all work in this repo (ADR-008):**
when implementing or reviewing anything spec-facing, read the relevant section
*here* — do not rely on memory, EHRbase behaviour, or blog posts. EHRbase is
prior art; these documents are the oracle.

- Vendored by `scripts/vendor-spec-docs.sh` (re-run to refresh; pins live in
  the script — keep `docs/VERSIONS.md` in sync).
- Text formats only (`.adoc`, `.md`, `.txt`, `.csv`, `.json`, `.yaml`,
  `.robot`, `.xml`, `.opt`). Images/UML/XSD/PDF excluded — see each
  component's `PROVENANCE.md` for the pinned commit to fetch them from.
- **Not a build input.** Codegen consumes `tools/openehr-codegen/vendor/**`
  (BMM/XSD/OAS) and `crates/openehr-its/schemas/**`; those stay authoritative
  for generation. This tree is for *reading and conformance-checking*.
- Never hand-edit anything under `docs/specs/openehr/` except this README.

## Map (component → what to read)

Spec documents are AsciiDoc books: `docs/<spec>/master.adoc` includes the
`master*.adoc` chapter files beside it. Grep the chapter files directly.

| Dir | Component (pin) | Key content |
|---|---|---|
| `BASE/` | BASE 1.3.0 | `docs/foundation_types/` (primitives, intervals, ISO 8601), `docs/base_types/` (identification: `OBJECT_VERSION_ID`, `ARCHETYPE_ID`, …), `docs/resource/`, `docs/architecture_overview/` |
| `RM/` | RM 1.2.0 | `docs/ehr/` (EHR, COMPOSITION, versioning semantics), `docs/common/` (LOCATABLE, VERSION, CONTRIBUTION, audit/attestation), `docs/data_types/`, `docs/data_structures/`, `docs/demographic/`, `docs/support/` (terminology-service interfaces) |
| `AM/` | AM 2.4.0 + 1.4 | `docs/ADL1.4/`, `docs/AOM1.4/` (our OPT 1.4 target), `docs/ADL2/`, `docs/AOM2/`, `docs/OPT2/`, `docs/Identification/` |
| `QUERY/` | QUERY 1.1.0 (tag) | `docs/AQL/` — the AQL spec (grammar, operators, functions, result set), `docs/AQL_examples/` |
| `TERM/` | TERM 3.1.0 | `docs/SupportTerminology/` — the openEHR support terminology (code sets, term sets) |
| `LANG/` | LANG (master) | `docs/bmm/`, `docs/bmm3/`, `docs/bmm_persistence/` (P_BMM), `docs/odin/`, `docs/EL/`, `docs/BEL/` |
| `SM/` | SM (master) | `docs/openehr_platform/` (the abstract platform service model behind ITS-REST), `docs/serial_data_formats/` + `docs/simplified_im_b/` (**SDT: FLAT/STRUCTURED semantics** — P14/P17 authority) |
| `CNF/` | CNF (master) | `docs/platform_test_schedule/` (**the Platform Conformance Test Schedule — the P19 acceptance instrument**, per-endpoint + per-data-type test cases), `docs/guide/`, `docs/profiles/`, `tests/platform/robot/` (the executable Robot conformance suite + fixtures: `.opt` templates, canonical JSON/XML test data) |
| `ITS-REST/` | ITS-REST 1.0.3 (tag) | `specifications/` + `development/` (API definitions the OAS is built from), `docs/` (overview). Note: ADMIN API exists only on the upstream development branch. |
| `ITS-XML/` | ITS-XML (master) | docs only; the XSDs themselves are vendored at `crates/openehr-its/schemas/xml/` |
| `ITS-JSON/` | ITS-JSON @ `5acae05` | `components/**` — the canonical-JSON schemas (same pin as the fidelity-gate schema in `crates/openehr-its/schemas/`) |

ITS-BMM is deliberately not here: it is vendored verbatim (all serializations)
at `tools/openehr-codegen/vendor/bmm/` as the codegen input.

## How to use (agents + humans)

1. Identify the owning component (RM behaviour → `RM/`, REST wire → `ITS-REST/`
   + `SM/`, AQL semantics → `QUERY/`, validation/constraints → `AM/` + `RM/`,
   conformance expectations → `CNF/`).
2. Grep the class/attribute/endpoint name across that component's `docs/`
   chapter files; read the surrounding section, including invariants and
   inherited-class semantics.
3. For behaviour the CNF schedule tests, read the matching
   `CNF/docs/platform_test_schedule/master*.adoc` chapter **and** the
   corresponding Robot suite under `CNF/tests/platform/robot/` — they define
   the exact requests, status codes, and payloads a conformant server must
   produce.
4. Cite the spec section (file + heading) in the PR/commit description for any
   conformance-relevant decision; record deliberate gaps with `// PORT NOTE:`.
