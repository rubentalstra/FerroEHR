---
name: event-node-collapse-rule
description: A sole EVENT slot with max=1 is COLLAPSED out of the web template (master04) — FLAT `any_event:i` paths are then unauthorable; SF carriers need max>1
metadata:
  type: project
---

DECISIVE released rule — ITS-REST `docs/simplified_formats/master04-basic_concepts.adoc`
§"Conditionally Collapsed Wrapper Types": "An `EVENT` node is collapsed when
both of the following hold: 1. Its maximum occurrence is 1 (i.e., `max = 1`),
AND 2. No sibling `EVENT` nodes (of any concrete event type) exist in the same
parent `HISTORY`." Retained when multiple event types exist or `max > 1`.

Consequence for SF cases: on a template whose only events child is `0..1`, the
event node is hoisted away, so
`<root>/<obs>:0/any_event:0/...` addresses nothing → `FlatError::UnknownPath`
→ 422 (every FLAT input error maps to 422 via
`ferroehr-rest::formats::dispatch::flat_input_err`). Event-level master05
mappings (`/time`, `/width`, `/math_function`, `|sample_count`) are then
unreachable, because the collapsed event is re-materialized structurally from
`slot_types` with no way to supply its attributes.

- `cnf.opt.vitals` events child is `0..*` → retained (`any_event:i` works,
  SF-INDEX-multi_event_commit passes).
- `cnf.tpl.sf_interval_event.opt` at0002 is `0..1` → collapsed; that is why
  SF-MAP-interval_event 422s. CATALOGUE bin: the carrier OPT must declare
  `upper_unbounded` on the events slot (or a sibling event type).

Implementation mirror: `openehr-its::flat::webtemplate::shape.rs`
`SINGLE_COMPACTABLE` + `is_compactable` (`child.max == 1 && …`) — spec-correct.
