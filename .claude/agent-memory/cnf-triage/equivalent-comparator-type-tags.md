---
name: equivalent-comparator-type-tags
description: RUNNER bin — the `equivalent` comparator counts optional canonical-JSON `_type` self-tags as content, so every XML-commit case whose JSON twin is sparsely tagged fails
metadata:
  type: project
---

`assertions::equivalent` (Veredictum's `src/exec/assertions.rs:307`) ends in
`resultset::cells_equal`, whose object rule requires **equal key sets**
(`resultset.rs:47`, `x.len() == y.len()`). A canonical-XML commit is decoded
through the RM (`negotiate::rm_value` → `from_canonical_xml` →
`to_canonical_value`), and the generated `ToJson` writes
`w.field_str("_type", …)` **unconditionally on every object** — so the stored /
served JSON carries `_type` on concretely-typed slots that a sparsely tagged
JSON twin omits (the CNF-Robot-derived `minimal_event.v1/.v2` fixtures have 31
untagged objects each). Result: strict tree inequality on RM-identical content.

**Why the SUT is right:** ITS-REST `specifications/docs/overview/Resources.md`
§JSON Format — "`_type` … **should** be used to specify the RM type whenever
polymorphism is involved, or when the underlying definition in RM type is
abstract"; the MUST governs only the VALUE ("MUST be the uppercase class
name"). Presence in a concrete slot is optional, so it is representation, not
content. §JSON Format also: attribute ORDER "is not mandatory" (and the store is
`jsonb`, which reorders keys by length — that is why every read-back preview
starts `{"uid":…`; it is never diagnostic).

**Tell:** XML-equivalence cases pass iff their JSON twin is exhaustively
`_type`-tagged (`create_party-xml`, `create_directory-xml` pass;
`create_composition-xml` / `update_composition-xml` fail). Do NOT "fix" it by
re-tagging a fixture (bending an artifact to the SUT) or by enumerating 31
ignore paths — fix the comparator: a `_type` present on ONE side only is not a
difference; present on BOTH it must be equal (keeps polymorphic-type checking).

**How to apply:** any future `equivalent`-across-format red row → check the
twin's `_type` density first. Also note `equivalence_mismatch`
(`exec/driver.rs:816`) truncates both sides to 80 chars, so results.json alone
never shows the real diff — reproduce or widen the diagnostic.
