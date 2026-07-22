# Recipe `opt_synth` — per-row operational-template synthesis for content cases

Deterministic `(case, row) -> OPT 1.4 XML` contract (issue #228). A content
case whose `constraint_context.constraint_columns` names constraint-axis columns
does NOT commit every decision-table row against one baked template — the
archetype/template constraint varies per row, so no single OPT makes every row's
verdict correct. Instead the runner synthesizes one OPT per row from that row's
constraint-axis cells, uploads it under a deterministic per-row template id, and
commits the row's instance against it.

This contract is realized by two pure Rust modules (the runner implementation
must match this spec):

- `tools/cnf-runner/src/exec/content_synth.rs` — the single dispatch entry
  (`synthesize_opt`) plus the STRUCTURAL families (carrier-shape constraints).
- `tools/cnf-runner/src/exec/opt_synth.rs` — the VALUE + INTERVAL families
  (`synthesize_value_opt`, ELEMENT.value domain constraints).

## Template id and determinism

- Per-row template id: `cnf.tpl.<case-id-slug>.r<row>` where the slug lowercases
  the case id and replaces every non-`[a-z0-9]` char with `_`
  (`recipes::synth_template_id`). The carrier stamps the same id into
  `archetype_details.template_id`, so upload and commit agree.
- The OPT `<uid>` is a UUIDv5 over the fixed namespace
  `6f9619ff-8b86-d011-b42d-00cf4fc964ff` and the template id (mirrors the Python
  reference `det_uid`). Output is byte-identical for identical inputs — no
  clock, no randomness.
- Upload is 409-tolerated (a re-run row re-uploads the same deterministic OPT).

## Carrier skeleton

`openEHR-EHR-COMPOSITION.minimal.v1` wrapping `openEHR-EHR-OBSERVATION.minimal.v1`
(HISTORY at0001 / EVENT at0002 / ITEM_TREE at0003 / ELEMENT at0004), or an
`openEHR-EHR-EVALUATION.minimal.v1` for the ITEM_STRUCTURE type-narrowing family.
This mirrors the committed Python reference
`corpus/templates/generate_content_opts.py`, itself the vendored CNF Robot
`minimal_observation.opt`.

## Constraint shapes (AM AOM1.4)

- **Value families** (ELEMENT.value): `C_INTEGER`/`C_REAL` list+range,
  `C_STRING` pattern+list, `C_DATE`/`C_TIME`/`C_DATE_TIME` patterns
  (`yyyy-mm-dd`/`HH:MM:SS` with `??`/`XX` per validity kind — AM ADL1.4
  master05-cadl §Patterns) and ranges, `C_DURATION` slot patterns
  (`P[Y][M][W][D][T[H][M][S]]` — §Duration Constraints) and ranges,
  `C_CODE_PHRASE`, `CONSTRAINT_REF`, `C_DV_ORDINAL`, and the `DV_INTERVAL<T>`
  inner-limit objects.
- **Structural families** (carrier shape): `C_MULTIPLE_ATTRIBUTE.cardinality`
  on COMPOSITION.content / HISTORY.events, `C_ATTRIBUTE.existence` on
  COMPOSITION.context / EVENT.state / HISTORY.summary / OBSERVATION.state+protocol,
  and `C_OBJECT.rm_type_name` narrowing of EVENT / ITEM_STRUCTURE slots. The
  cardinality token also sets the container's existence (`1plus`/`3plus`/`mand`/
  `3to5` => 1..1) so an absent container is rejected by existence — cardinality
  alone never fires on an omitted attribute.

## Column-shapes that are NOT expressible in AOM1.4 (flagged, not synthesized)

- `millisecond_validity` / `timezone_validity` on C_TIME/C_DATE_TIME — the
  AOM1.4 constraint pattern (§Patterns) has only `hh:mm:ss` plus an optional
  appended timezone requirement; there is no millisecond slot and no way to
  PROHIBIT a timezone.
- `seconds_allowed` vs `fractional_seconds_allowed` on C_DURATION — the pattern
  has a single `S` slot; seconds and fractional seconds cannot be separated.
- `DV_SCALE` symbol lists — AOM1.4 has no `C_DV_SCALE` (DV_SCALE postdates ADL
  1.4); a DV_SCALE constraint is only a plain C_COMPLEX_OBJECT.

Rows resting on those distinctions are handled by decision-table
re-adjudication (spec-cited in the affected case files), not by synthesis.

## Seed

None — the outputs are pure functions of (case id, row, constraint cells).
