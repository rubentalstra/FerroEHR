---
name: sibling-products
description: The Ferro family is three repos side by side under ~/RustroverProjects (ferroehr, FerroTERM, FerroBRIDGE); FerroTERM's checkout was renamed from notio on 2026-09-03; FerroBRIDGE (Apache-2.0) owns the FHIRconnect bridge and the in-tree FHIR extension is expected to move there
metadata:
  type: project
---

Four products, four repositories: FerroEHR (this repo, the CDR), FerroCKM (`../FerroCKM`, the Clinical Knowledge Manager, created 2026-09-03, BUSL-1.1 with FerroEHR's grant so the two share the application layer; research on its #1), FerroTERM (`../FerroTERM`, the FHIR terminology server; the local checkout was `../notio` until 2026-09-03 and the harness memory link was moved with it) and FerroBRIDGE (`../FerroBRIDGE`, the bridge from openEHR to HL7 FHIR (FHIRconnect spec, openFHIR as prior art) and to the OMOP CDM (OMOCL spec, Eos as prior art), created 2026-09-03, Apache-2.0 by owner choice; FerroTERM is Apache-2.0 too (its LICENSE and Cargo manifest, checked 2026-09-03), only FerroEHR and FerroCKM are BUSL-1.1). FerroEHR issues #2646 and #2652 were transferred there (FerroBRIDGE #1 and #2).

**Why:** the owner treats each as a separate product with its own licence story; the bridge is meant to be adopted widely, so it stays open source.

**How to apply:** never edit a sibling repo from a FerroEHR session unless the owner asks; FerroEHR's `ferroehr-ext` FHIR feature is expected to be retired in favour of FerroBRIDGE once the bridge exists (a FerroEHR issue is filed when the bridge's first milestone is scoped); FerroBRIDGE consumes FerroEHR strictly over ITS-REST, never as a crate dependency. See [[license-busl]].
