# Vendored ITS-XML schemas (provenance + parity policy)

XSD schemas from `openEHR/specifications-ITS-XML`, vendored for the Phase 05
canonical-XML serialization work. Reference/validation inputs only; no code
here. Kept beside the crate that uses them per the spec-cache policy
(`docs/plans/phase-05-serialization-xml.md`, ADR-003 cache rule).

Source repo: https://github.com/openEHR/specifications-ITS-XML

## Two lineages are vendored — they use DIFFERENT XML namespaces

The ITS-XML repo's `Release-2.0.0` restructure changed the XML target
namespace from `http://schemas.openehr.org/v1` to
`http://schemas.openehr.org/v2` repo-wide, and re-stamped every historical
release folder to `v2`. So the *namespace* is an axis independent of the RM
model version:

| Vendored dir | Repo ref | Namespace | RM lineage | What it is |
|---|---|---|---|---|
| `its-xml-2.0.0-nsv2/` | `master` @ `de8b37ba6c9a5e126623a063cafba3b58ebf1107` | `http://schemas.openehr.org/v2` | RM 1.1.0 (latest) | Latest openEHR spec. RM/Release-1.1.0 + BASE/Release-1.2.0 + AM/Release-1.4 + OET/Release-1.0.1 + QUERY/latest. |
| `its-xml-1.0.2-nsv1/` | tag `Release-1.0.2v2` @ `f7a937778bf9ea43b01b0f9d8a616e47f35017c1` | `http://schemas.openehr.org/v1` | RM 1.0.2-lineage | The STABLE pre-2.0.0 bundle (flat `components/ALL/`). This is the archie schema set. |

(The `openEHR/v1/Template` namespace in the OET/Template schemas is the
separate template-document namespace, not the RM namespace.)

## Which one does stock EHRbase actually speak? → v1

Confirmed from EHRbase's own Java + XML test fixtures in this repo (parity
baseline = EHRbase v2.33.0):

- `crates/openehr-server/.../TemplateServiceImp.java` sets the OPT root
  QName to namespace `http://schemas.openehr.org/v1`.
- `crates/openehr-server/tests/resources/service/samples/*.xml` (real
  composition fixtures, e.g. `RIPPLE-ConformanceTest.xml`) declare
  `xmlns:v1="http://schemas.openehr.org/v1"` on `<composition>` and use
  attribute-based `xsi:type="OBJECT_VERSION_ID"` discriminators.

EHRbase gets its RM canonical XML from the external `archie` library, which
bundles the **v1-namespace** schemas. So for a 1:1 faithful port, our XML
*output* must be v1-namespace to match EHRbase byte-for-byte at the REST
surface. Targeting v2/RM-1.1.0 XML would be an improvement (Stage 3), not a
faithful port.

## Decision status — SETTLED 2026-07-03: target v1

- RM *model* stays 1.1.0 internally (JSON serialization is unaffected).
- XML *wire serialization* target: **`its-xml-1.0.2-nsv1/`** (namespace
  `http://schemas.openehr.org/v1`). It is the latest *stable* ITS-XML (2.0.0
  is TRIAL/in-development) and is what stock EHRbase emits, so it is what the
  1:1 faithful port requires. See the DECISION note in
  `docs/plans/phase-05-serialization-xml.md`.
- `its-xml-2.0.0-nsv2/` is retained as latest-spec reference for the Stage-3
  improvement that may adopt v2; it is NOT a Stage-1 serialization target.
