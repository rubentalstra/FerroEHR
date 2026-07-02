---
name: rosetta-mapping
description: >
  Appends one new Java-to-Rust or openEHR-spec-to-Rust mapping row to
  docs/ROSETTA.md, deduping against existing entries first. Use whenever a
  porting or transcription task discovers a mapping worth reusing across
  files (a type mapping, an idiom mapping, or a naming convention).
allowed-tools: [Read, Edit, Grep]
argument-hint: "<mapping to add, e.g. 'Java Optional<BigDecimal> -> Rust Option<rust_decimal::Decimal>'>"
---

# /rosetta-mapping

`docs/ROSETTA.md` is the living lookup table referenced by every porter and
transcriber (`docs/PORTING.md` is the static rule set this file extends with
concrete, discovered mappings). It has two tables: **Java → Rust** (for
ported EHRbase application code) and **openEHR spec → Rust** (for native
transcription).

## Steps

1. **Read `docs/ROSETTA.md`** in full — it is small enough to hold in
   context. If it does not exist yet, this is the first entry; create it
   with the two table headers (`Java construct | Rust equivalent | Notes`
   and `Spec class/concept | Rust type | Notes`).
2. **Classify the mapping.** Decide which of the two tables it belongs to.
   If it is really a restatement of a rule already in
   `PORT_MASTER_PLAN.md` Section 14.2-14.4 or `docs/PORTING.md`, do not add
   it — ROSETTA is for mappings discovered during actual porting work, not
   a copy of the plan.
3. **Dedupe.** Grep the target table for the same Java construct or spec
   class already having a row. If found:
   - Same mapping → do nothing, report "already present."
   - Conflicting mapping → do not silently overwrite; report the conflict
     back to the caller so a human or the `rosetta-curator` agent resolves
     it.
4. **Append the row**, keeping the table sorted the way it already is
   (alphabetical by the left column, if that is the existing convention).
   Keep the "Notes" column terse — one clause, not a paragraph.
5. **Never invent a mapping you have not actually seen used.** This skill
   records what a port/transcription task did, not what it might do.
