# Vendored openEHR specifications (normative text)

The **authoritative openEHR spec text**, vendored verbatim from the official
`github.com/openEHR/specifications-*` repos at the pins in `docs/VERSIONS.md`.
**This is the conformance authority for all work in this repo:**
when implementing or reviewing anything spec-facing, read the relevant section
*here* — do not rely on memory, EHRbase behaviour, or blog posts. EHRbase is
prior art; these documents are the oracle.

- Vendored by `scripts/vendor/spec-docs.sh` (re-run to refresh; pins live in
  the script — keep `docs/VERSIONS.md` in sync).
- Text formats only (`.adoc`, `.md`, `.txt`, `.csv`, `.json`, `.yaml`,
  `.robot`, `.xml`, `.opt`), **plus every figure the chapters actually
  reference**. The three figure attributes are defined in
  `openEHR/specifications-AA_GLOBAL`, `docs/boilerplate/global_vars.adoc`, all
  resolved relative to a component's `docs/` root, and the vendored mirror
  keeps that layout so the references resolve as published:

  | Attribute | Expands to | Vendored files |
  |---|---|---|
  | `:uml_diagrams_uri: UML/diagrams` | `docs/UML/diagrams/<name>.svg` | **129** — BASE 15, RM 33, AM 27, LANG 31, SM 22, TERM 1 |
  | `:diagrams_uri: {doc_name}/diagrams` | `docs/<doc_name>/diagrams/<name>` | **200** together with `images` — RM 70, AM 44, LANG 38, BASE 34, CNF 6, ITS-REST 4, SM 3, TERM 1 |
  | `:images_uri: {doc_name}/images` | `docs/<doc_name>/images/<name>` | (as above) |

  `{doc_name}` is the document directory the referencing chapter lives in
  (`docs/common`, `docs/AOM2`, `docs/bmm`, …), so the per-document sets are
  derived per directory. Components not listed reference none. Only referenced
  files are vendored, byte-for-byte from the same pinned commit as the text —
  a reference with no file at the pin fails the vendoring run, and there are
  currently no such dangling references. Unreferenced figures, UML
  `.xmi`/`.mdzip`, XSDs and PDFs are excluded — see each component's
  `PROVENANCE.md` for the pinned commit to fetch them from.
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
| `SM/` | SM (master) | `docs/openehr_platform/` (the abstract platform service model behind ITS-REST), `docs/serial_data_formats/` + `docs/simplified_im_b/` (DEVELOPMENT-state model documents — never implemented; the STABLE ITS-REST `simplified_formats` sub-spec is the FLAT/STRUCTURED wire authority). The SM UML diagrams are LOAD-BEARING: text-free path art carrying inheritance/bindings absent from the class tables — rasterize to read, and never let a re-vendor drop them |
| `CNF/` | CNF (master) | `docs/platform_test_schedule/` (**the Platform Conformance Test Schedule — a STALLED structural guide, never the correctness authority**, per-endpoint + per-data-type test cases), `docs/guide/`, `docs/profiles/`, `tests/platform/robot/` (the executable Robot conformance suite + fixtures: `.opt` templates, canonical JSON/XML test data) |
| `ITS-REST/` | ITS-REST Release-1.1.0 @ `24058992d` | `docs/` (the docs text — THE wire oracle) + `specifications/` (the OAS the released text presents as its computable artifacts; subordinate — wins only where the docs text is silent). All 7 API groups vendored (Overview/System/EHR/Query/Definition/Formats STABLE; Demographic/Admin/SMART DEVELOPMENT) |
| `ITS-XML/` | ITS-XML (master) | docs only; the XSDs themselves are vendored at `crates/openehr-its/schemas/xml/` |
| `ITS-JSON/` | ITS-JSON @ `5acae05` | `components/**` — the canonical-JSON schemas (same pin as the fidelity-gate schema in `crates/openehr-its/schemas/`) |

PROC (Process / Task Planning) is deliberately NOT vendored — openEHR publishes no releasable text for it. SM ch.10 nonetheless types `DATA_FRAME.primary_method/fallback_method` against PROC's `SYSTEM_CALL` (recoverable only from the rasterized §10.6 diagram); the tracker's SM upstream reports carry that gap.

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
   conformance-relevant decision; record a settled deliberate deviation with a
   `// NOTE:` carrying that citation, and anything still missing with a
   `// TODO:`.
