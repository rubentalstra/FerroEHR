---
name: valid-invalid-twins-rule
description: Owner hard rule 2026-07-29 — an adjudicated spec-correct refusal always yields BOTH twins (valid fixture fixed + invalid shape preserved as refusal case)
metadata: 
  node_type: memory
  type: feedback
  originSessionId: bcb4b8b9-623e-4930-9578-9873ac0afe1d
  modified: 2026-07-29T18:28:40.694Z
---

Owner hard rule (2026-07-29, stated during the VTCBK triage): when a CNF
red row is adjudicated "the SUT was spec-RIGHT to refuse this invalid
artifact", NEVER just fix the fixture — the invalid shape is preserved as
its own corpus entry (`validity: invalid` + defect + spec_ref) with a
refusal case beside the corrected valid twin. Valid proves acceptance,
invalid pins the refusal; a lenient server must fail the invalid twin.

**Why:** the owner's words: "if something fails because our setup is
spec-compliant then we should of course always have valid and invalid
ones" — deleting the invalid shape silently narrows coverage exactly where
the instrument just proved it has teeth. Extends [[cnf-spec-oracle-attribution]]
and the #671 leniency discipline.

**How to apply:** on every catalogue-attributed fixture defect the SUT
correctly refused: (1) fix the fixture with the spec citation, (2) resurrect
the defective shape under a new id as an invalid corpus entry, (3) author
the refusal case (expect validation_failed/the documented refusal), (4)
ratchet the owning capability floor. Recorded in `.claude/rules/testing.md`
§CNF coverage. First instance: the undefined-ac-code OPT
(`dt_coded_text_binding_undefined_ac.opt`, VATDF/VACDF).
