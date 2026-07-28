# Vendored ITS-XML schemas (provenance + serving policy)

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
| `its-xml-1.0.2-nsv1/` | tag `Release-1.0.2v2` @ `f7a937778bf9ea43b01b0f9d8a616e47f35017c1` | `http://schemas.openehr.org/v1` | The STABLE pre-2.0.0 bundle: flat `ALL/` (11 XSDs) + `AOM2/` (6 XSDs + examples). |
| `its-xml-2.0.0-nsv2/` | tag `Release-2.0.0v2` @ `de8b37ba6c9a5e126623a063cafba3b58ebf1107` (also repo HEAD at vendoring) | `http://schemas.openehr.org/v2` | The 2.0.0 release (TRIAL upstream): full `components/` — RM (1.0.2/1.0.3/1.0.4/1.1.0/latest), BASE (1.1.0/1.2.0/latest), AM (1.4/latest), OET (1.0.1/latest), QUERY/latest (70 XSDs). |

Fetched: 2026-07-04. (The `openEHR/v1/Template` namespace in the OET/Template
schemas is the separate template-document namespace, not the RM namespace.)

## Which namespace does the CDR serve? — v1 default, v2 negotiated

- Upstream marks 2.0.0 **TRIAL** and directs stable consumers to
  `Release-1.0.2` (`docs/specs/openehr/ITS-XML/README.adoc` §Releases and IM
  Versions), so the **v1** namespace is the released-STABLE lineage and the
  served default under the released-spec policy (`docs/VERSIONS.md`).
- Owner ruling 2026-07-28 (#196): the **v2** namespace is served **on
  request** via the `version` media-type parameter on the canonical-XML
  media type (`Accept: application/xml; version=2`). No openEHR spec governs
  namespace selection on the REST wire — the parameter is our own
  design/extension (register AMB-169, `option_select`).

## Generation status — SETTLED (one codec, both namespaces)

- RM *model* stays 1.2.0 internally (JSON serialization unaffected).
- `emit-xml` generates ONE impl set serving **both** wire lineages — they
  differ only by the root `xmlns`, selected at serialize time
  (`crates/openehr-its/src/xml/runtime.rs`); this is NOT an AM-style dual
  generation.
