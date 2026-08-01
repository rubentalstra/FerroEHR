---
name: codegen-degrade-and-optionality-conventions
description: Two openehr-codegen emission conventions that silently lose spec semantics — serde_json::Value degrade for unresolvable class refs, and Vec for optional containers; where each bites
metadata:
  type: project
---

Two settled `openehr-codegen` conventions that a reviewer must actively look
through, because both are invisible in the generated output. Verified
2026-07-31.

1. **Unresolvable class ref → `serde_json::Value`**
   (`analyze/mod.rs:354-356`, `:376-379`). A referenced class in neither the
   local nor the external set becomes free-form JSON, with **no `// NOTE:` and
   no citation at the field**. Confirmed instances:
   `EL_CASE.value_constraint: C_OBJECT` → `serde_json::Value`
   (`bmm3/expression/el_case.rs:15`) — an upstream layering inversion (a LANG
   class referencing an AM class, and `am → lang` is the dependency
   direction); `BMM_INTERVAL_VALUE.type` / `BMM_CONTAINER_VALUE.type` (both
   inherit `BMM_LITERAL_VALUE<T>` with `T` unbound and the vendored UML states
   no binding either). These DO get canonical-JSON codecs in `openehr-its`
   (`json_codec/generated/impls.rs`), so the degrade is wire-reachable, not
   merely internal. `plan/overrides.rs` `CLASS_BINDINGS` (:283-326) is the
   sanctioned fix mechanism and every entry there carries a spec citation —
   an undocumented degrade is the anomaly, not the norm.

2. **Every container is `Vec<T>` regardless of `is_mandatory`**
   (`render/emit.rs:653-668`; single-valued props DO honour optionality at
   :647-651). Verified: zero `Option<Vec<` in any generated crate. Usually
   harmless — but it destroys the Void-vs-empty distinction, and some spec
   functions are DEFINED on exactly that distinction. Confirmed case:
   `EL_AGENT.open_args` is 0..1, whose Void state normatively means "infer the
   missing args from `definition`" AND is the literal definition of
   `is_callable()` (`…bmm3.el_agent.adoc` §Functions, `Post_result_validity:
   Result = open_arguments = Void`) — which three invariants reduce to
   (`EL_AGENT_CALL.Inv_valid_call`, `EL_FUNCTION_CALL.Inv_valid_agent`,
   `BMM_PROCEDURE_CALL.Inv_valid_agent`). Latent only because nothing
   evaluates EL.

**How to apply:** when auditing a generated crate, grep the classes in scope
for `serde_json::Value` and cross-check every 0..1 container attribute against
the spec functions/invariants that read it. Neither convention is
re-litigable per class (root `CLAUDE.md` §Conventions) — the finding is the
missing `// NOTE:`/`// TODO:` and, where a spec predicate keys off the lost
state, a targeted `type_override`. See
[[lang-bmm3-two-schema-chimera]].
