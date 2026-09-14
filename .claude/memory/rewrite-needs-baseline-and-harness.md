---
name: rewrite-needs-baseline-and-harness
description: Owner direction 2026-09-14 - a performance rewrite is SHOWN faster with a harness, a committed pre-rewrite baseline and an after comparison; measurement issues are filed as sub-issues alongside the design, and the baseline blocks the rewrite
metadata:
  type: feedback
---

A rewrite that claims performance ships with its own measurement program: a harness that runs the same operations against the old and the new design and writes a comparable committed record, a baseline recorded BEFORE the rewrite (a blocked-by edge from the rewrite issue), plan-shape tests that pin the access paths in the ordinary test battery, an after-rewrite comparison with a stated tolerance, and a repeatable (dispatch-only) lane. Every number reaches a page only through a generated include over the record.

**Why:** the owner, on the storage redesign (#3337, 2026-09-14): "we definitely need a proper test to make sure that we go faster or more optimized, so create more issues for this test to measure performance". The plan had named instruments per hypothesis but only one measurement issue and no harness; #3367 to #3370 filled that.

**How to apply:** when filing a design's decomposition, file the measurement sub-issues in the same batch and give the baseline issue a `blocked-by` edge INTO the rewrite issue. Related: [[measurement-environment-discipline]], [[book-numbers-are-generated-includes]], [[no-stress-test-v430]].
