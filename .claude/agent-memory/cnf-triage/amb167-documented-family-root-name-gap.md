---
name: amb167-documented-family-root-name-gap
description: AMB-167's handling forbids asserting an XML root name for EVERY family, contradicting its own finding that DOCUMENTED families' §XML Format MUSTs are enforceable
metadata:
  type: project
---

AMB-167 classifies canonical-XML per resource: **DOCUMENTED** (a published
global `xs:element` exists — `composition`, `version`, `versioned_object`,
`template`, `archetype`, `extract`, `extract_request`, `items`) vs **UNDEFINED**
(EHR, EHR_STATUS, FOLDER, PARTY*, CONTRIBUTION, REVISION_HISTORY, ITEM_TAG).
Its ambiguity text says: "For a resource WITH a document element, offering XML is
fully released behaviour and the §XML Format MUSTs are enforceable as written."

But its `handling` then says: "ROOT ELEMENT NAME where a family IS offered: no
openEHR spec governs it — our own design/extension — so no case asserts one."
That blanket carve-out is correct only for the UNDEFINED families and **wrong for
the DOCUMENTED ones**, where `Composition.xsd` (`targetNamespace=
http://schemas.openehr.org/v1`, `elementFormDefault="qualified"`) fixes both the
root NAME and its NAMESPACE, and Resources.md §XML Format makes responses "MUST
conform to the [published XSDs]".

Consequence measured 2026-07-28: EHRbase 2.34.0 serves `<composition …>` with the
root in NO namespace and **no case in the catalogue detects it** — a coverage
gap on a MUST, not a latitude.

Two further consequences of the same undefined/documented split:
- The `<family>-xml-supported` arms are being used for **WRITE** rows
  (`create_ehr-xml`, `create_directory-xml`, `set_ehr_*-xml_body`). A write
  necessarily commits to a root element name, which for an UNDEFINED family is
  exactly what AMB-167 says is spec-silent — so those rows assert an
  unauthorable document. Same defect class as the AMB-165 read-side claim
  ([[contribution-xml-read-not-a-released-document]]).
- The released OAS declares `application/json` ONLY on the request bodies of
  `ehr_create`, `ehr_status_update` and `directory_create`, so there is no
  released ground for an XML request body on those routes at all.

**How to apply:** CATALOGUE bin. Split AMB-167's handling by classification —
assert root name + namespace on the DOCUMENTED families, keep the silence for
UNDEFINED ones, and re-ground or retire the UNDEFINED-family write rows.
