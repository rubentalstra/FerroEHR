---
name: component-fixes-ride-current-patch
description: "Owner triage rule — a spec-component fix goes to the OPEN program milestone for that component if one exists; once the component's program milestone is closed, its fixes ride the CURRENT patch milestone"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: b978ad0a-c940-4317-b4ca-ba6f1ec0796f
  modified: 2026-07-31T10:29:32.217Z
---

Owner ruling (2026-07-31, during the v3.15.1 cycle): milestone assignment for spec-component fixes follows the component-program ladder — but only while the program is OPEN. Once a component's chapter-audit program milestone has been closed/released, new fixes for that component are NOT parked on a dead program; they join the current patch milestone and ship now.

**Why:** milestones are delivery promises tied to releases; a closed program milestone can never deliver again, and the owner wants component fixes flowing continuously instead of queuing for a hypothetical reopened program.

**How to apply:** when filing/triaging a fix, check the component ladder (v3.16.0 LANG, v3.17.0 TERM, v3.18.0 RM, v3.19.0 SM open as of 2026-07-31; ITS closed with v3.12.0, QUERY closed with v3.14.0, BASE closed with v3.13.0, AM program closed with v3.15.0/x): open program → that milestone; closed program → the current patch milestone (and implement it in the current cycle — assignment implies delivery). Examples: #1349 (ITS) and #1347 (QUERY) both pulled into v3.15.1. See [[spec-chapter-audit-programs]] for the ladder itself.
