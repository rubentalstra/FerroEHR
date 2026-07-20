# Vendored ITS-XML schemas (provenance + parity policy)

XSD schemas from `openEHR/specifications-ITS-XML`, vendored **verbatim** (full
upstream `components/` trees) for canonical-XML (de)serialization. Reference /
validation inputs and the `emit-xml` codegen oracle; no code here.

Source repo: https://github.com/openEHR/specifications-ITS-XML

## Two lineages are vendored — they use DIFFERENT XML namespaces

The ITS-XML repo's `Release-2.0.0` restructure changed the XML target namespace
from `http://schemas.openehr.org/v1` to `http://schemas.openehr.org/v2`
repo-wide and re-stamped every historical release folder to `v2`. So the
*namespace* is an axis independent of the RM model version. Each lineage is
vendored as the upstream `components/` tree, verbatim:

| Vendored dir | Repo ref | Namespace | What it is |
|---|---|---|---|
| `its-xml-1.0.2-nsv1/` | tag `Release-1.0.2v2` @ `f7a937778bf9ea43b01b0f9d8a616e47f35017c1` | `http://schemas.openehr.org/v1` | The STABLE pre-2.0.0 bundle: flat `ALL/` (11 XSDs) + `AOM2/` (6 XSDs + examples). This is the archie schema set. |
| `its-xml-2.0.0-nsv2/` | `master` @ `de8b37ba6c9a5e126623a063cafba3b58ebf1107` | `http://schemas.openehr.org/v2` | Latest openEHR spec: full `components/` — RM (1.0.2/1.0.3/1.0.4/1.1.0/latest), BASE (1.1.0/1.2.0/latest), AM (1.4/latest), OET (1.0.1/latest), QUERY/latest (70 XSDs). |

Fetched: 2026-07-04. (The `openEHR/v1/Template` namespace in the OET/Template
schemas is the separate template-document namespace, not the RM namespace.)

## Which one does stock EHRbase actually speak? -> v1

Confirmed from EHRbase's own Java + XML test fixtures in this repo (parity
baseline = EHRbase v2.33.0):

- `crates/openehr-server/.../TemplateServiceImp.java` sets the OPT root QName to
  namespace `http://schemas.openehr.org/v1`.
- `crates/openehr-server/tests/resources/service/samples/*.xml` (real
  composition fixtures) declare `xmlns:v1="http://schemas.openehr.org/v1"` and
  use attribute-based `xsi:type="OBJECT_VERSION_ID"` discriminators.

EHRbase gets its RM canonical XML from the external `archie` library, which
bundles the **v1-namespace** schemas. So for a 1:1 faithful port, our XML
*output* must be v1-namespace to match EHRbase byte-for-byte at the REST surface.

## Decision status — SETTLED (both namespaces generated)

- RM *model* stays 1.2.0 internally (JSON serialization unaffected).
- `emit-xml` generates **both** wire lineages: **v1**
  (`its-xml-1.0.2-nsv1/`, namespace `.../v1`) is the **default / Stage-1 parity
  target** (what stock EHRbase emits); **v2** (`its-xml-2.0.0-nsv2/`,
  `.../v2`) is generated behind a flag as the latest-spec target for the
  eventual Stage-3 improvement.
