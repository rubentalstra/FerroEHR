---
name: comments-less-is-more
description: Owner 2026-08-04 — drastically fewer comments; one citation + one sentence max
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-04T09:37:11.189Z
---

Owner (2026-08-04): "why do we need always so many comments!!! ... less is more ... it gets stale very fast."

**Why:** Long NOTE blocks restating design reasoning go stale and make code harder to read; the durable record is the spec citation + the tracker/PR, not prose in code.

**How to apply:** A `// NOTE:` is the spec citation plus AT MOST one sentence. No multi-paragraph adjudication essays in code — the adjudication lives on the issue/PR; the code carries the pointer. Same for doc comments: summary line + `# Errors`, not narratives. Related: [[no-task-ids-in-code]], [[todo-only-markers]].
