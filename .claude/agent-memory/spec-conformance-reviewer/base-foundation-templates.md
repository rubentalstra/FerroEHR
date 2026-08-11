---
name: base-foundation-templates
description: BASE openehr-base template audit (#2252) — the inverted "CNF outranks prose" justifications, the ISO-8601 leniencies, and which invariants have no Validate impl
metadata:
  type: project
---

Audited `tools/openehr-codegen/templates/openehr-base/` (26 hand-written
spec-behaviour templates stamped into `crates/openehr-base/src/v1_2|v1_3/`).

**Recurring defect class — inverted precedence in justifications.** Two files
justify accepting what the released BASE text refuses by asserting a
"CNF data outranks the prose" rule: `party_ref_impl.rs` (admits
`type="ANY"` past `PARTY_REF.Type_validity`, citing
`CNF/.../create_composition-persistent.robot`) and `terminology_id_impl.rs`
(admits `"SNOMED CT"` past the `name-str` production). The real rule is the
opposite (`.claude/rules/spec-adherence.md`: CNF is a stalled guide, the
released spec wins). `terminology_id_impl.rs` additionally QUOTES master05
§Terminology Identifiers as saying identifiers "include, but are not limited
to" — that string appears nowhere in `docs/specs/openehr/`. When auditing a
BASE leniency, grep the claimed quotation first.
The defensible released ground for `ANY` is unused and sits one class up:
`org.openehr.base.base_types.object_ref.adoc` §Attributes, `type` row.

**Where invariants are and are not realized.** Every identification/interval
template carries a `Validate` impl + a validating `new()`; the FOUR ISO-8601
templates carry NEITHER (their class-table invariants are enforced only by the
RM `DV_*` wrappers in `openehr-rm`), and `authored_resource_impl.rs` has no
`Validate` at all (`Translations_valid` + `Description_valid` are mechanically
checkable and unchecked).

**ISO-8601 specifics worth re-checking on any change.** `parse_timezone`
bounds hh at 14 for BOTH signs, but `iso8601_timezone.adoc` §Invariants
`Min_hour_valid` caps a negative offset at `Min_timezone_hour` = 12.
`parse_duration` enforces neither designator ORDER nor at-most-once, so
`P1D1M` parses and `P1Y1Y` silently discards the first count.
`is_positive_int` rejects `01`, which `Trunk_version_valid`
(`is_integer and then as_integer >= 1`) and `number = digit,{digit}` both
admit — an invented prohibition. The definite-vs-nominal split IS correct
(average lengths for `add`, calendar clamp for `add_nominal`,
`master06-time_types.adoc` §Computational Functions) — do not re-litigate it.

**Comment-guard blind spot:** `scripts/checks/comment-style.sh` applies the
NOTE_MAX=3 / RUN_MAX=8 budgets only to plain `//` lines (`if (is_line)`), so
`//!`/`///` NOTE essays are unchecked; the ISO-8601 module headers carry
20–25-line adjudication essays there. `comments.md` names this as
review-enforced.

**CORRECTION (verified 2026-08-11, #2255):** the two `parse_*` claims above are
STALE. `iso8601_parse.rs::parse_timezone` now bounds `-` at `MIN_TIMEZONE_HOUR`
(12) and `+` at 14, and `parse_duration` now enforces designator order +
at-most-once via a `last_slot` ratchet (and rejects second `60`). The lenient
side today is `openehr-rm`'s own `validate.rs` copies of those grammars.
