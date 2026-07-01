---
name: transcribe-rm-class
description: >
  Literally transcribes one openEHR spec class (RM, BASE, or Foundation
  Types) into the correct spec crate, from the published specification, not
  from any Java source. Use when the user names an openEHR class (e.g.
  "transcribe DV_QUANTITY" or "write the RM class for HIER_OBJECT_ID").
allowed-tools: [Read, Grep, Glob, Agent]
argument-hint: "<OPENEHR_CLASS_NAME, e.g. DV_QUANTITY>"
---

# /transcribe-rm-class

Transcribes one openEHR class literally from its specification into Rust.
These four crates (`openehr-rm`, `openehr-base`, `openehr-foundation`,
`openehr-terminology`) have no Java to port from — see
`.claude/rules/rm-transcription.md` for the full rule set this skill
implements.

## Steps

1. **Locate the class in the spec inventory.** Check
   `PORT_MASTER_PLAN.md` Section 7.1 (RM class inventory) to find which
   component/package the class belongs to (e.g. `DV_QUANTITY` is
   `rm.data_types.quantity`), and Section 7.2 for any known hazard that
   applies to it.
2. **Pick the target crate and module** per Section 9's crate table:
   Foundation Types → `openehr-foundation`; Base Types (identification,
   resource) → `openehr-base`; Terminology → `openehr-terminology`; RM
   proper → `openehr-rm`, mirroring the spec's own package structure
   (`data_types`, `data_structures`, `common`, `ehr`, `demographic`,
   `integration`, `support`) as the module layout.
3. **Check `docs/ROSETTA.md`** for an existing spec→Rust mapping for this
   class or its parent/siblings, so naming stays consistent across the
   crate.
4. **Delegate to the `rm-transcriber` subagent** (or transcribe inline for a
   single small class) applying `.claude/rules/rm-transcription.md`:
   struct/enum named identically in PascalCase, openEHR name in a doc
   comment, serde rename to the canonical `_type` string, abstract-class
   embedding, constrained generics with trait bounds, covariant
   redefinitions documented, multiple inheritance as composition + traits.
5. **Add invariants as a `Validate` impl.** Where the spec states a class
   invariant (e.g. `DV_QUANTITY` requiring a valid units string), write the
   `Validate` trait method; leave `// TODO(port):` for any invariant not yet
   implemented rather than skipping the impl block entirely.
6. **Add the PORT STATUS trailer** with `source` naming the spec and section
   (e.g. `RM 1.1.0 data_types §5.4.3 DV_QUANTITY`), not a Java file.
7. **Report** confidence, TODO count, and any hazard from Section 7.2 that
   applied.
