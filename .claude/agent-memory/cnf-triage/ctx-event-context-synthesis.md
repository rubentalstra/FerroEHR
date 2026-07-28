---
name: ctx-event-context-synthesis
description: ctx/ defaults FILL rm-tree nodes, they do not CREATE an EVENT_CONTEXT — only event-context ctx keys make context/start_time appear
metadata:
  type: project
---

CONFIRMED 2026-07-28. `ctx/time` defaults are a VALUE rule, not a container
rule:

- ITS-REST `simplified_formats/master06-context_information.adoc` preamble —
  "the flat format offers the option to set context values which **set default
  values in the rm-tree**"; §time — "set the default time for `ACTION.time`,
  `COMPOSITION.context.start_time`, `OBSERVATION.history.origin`" and
  "`ctx/time` will be set to `now()` if not set explicitly".
- `master04-basic_concepts.adoc` §Context — "The `ctx/time` field, if not
  explicitly set, defaults to the current server time (`now()`)"; the mandatory
  ctx set is only language + territory.
- RM `UML/classes/org.openehr.rm.composition.composition.adoc` — `context` is
  `0..1` with NO invariant tying it to `category`; `ehr/master05` §Event context
  is explicitly optional. So a COMPOSITION with no EVENT_CONTEXT is RM-valid
  even at `433|event|`.

=> No released sentence obliges a server to SYNTHESIZE an EVENT_CONTEXT for a
simplified payload that expresses none. The SUT builds one only when the ctx
node carries event-context keys (`build.rs::has_explicit_event_context`:
time / end_time / setting / location / health_care_facility /
`participation_*`) — spec-defensible, NOTE-documented.

**How to apply:** an `assert: field, path: context/start_time, exists: true`
row is grounded ONLY if the case's fixture carries at least one of those keys
(`cnf.flat.vitals.minimal_ctx` does; `cnf.structured.vitals.minimal` does NOT —
its ctx is language/territory/composer_self only). Asserting it on a
ctx-eventless fixture is a CATALOGUE defect.
