---
name: rewrite-fn-docs-on-update
description: "Owner hard rule 2026-08-24 — when updating a function, fully rewrite its doc comment; never orphan or stale it"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 61772e07-7f45-4d64-a77c-e5be26729532
  modified: 2026-08-24T15:53:29.402Z
---

Owner rule (2026-08-24, after two incidents in one day): whenever a function
is updated — signature, behaviour, or a helper inserted above it — its `///`
doc comment is REWRITTEN in full to describe the new reality. Never leave the
old doc standing, never let an insertion orphan a doc comment onto the wrong
item, and never keep "stupid long" stale prose.

**Why:** an Edit whose `old_string` starts at `fn name(` leaves the doc
comment above the match — an inserted item then absorbs the previous
function's doc (compiles fine, docs land on the wrong fn). Happened with
`version_signature`/`body_and_signature` and `first_version_root`.

**How to apply:** every Edit that touches a fn includes the fn's doc comment
in `old_string` (start the match at the `///` block, not at `fn`); after any
signature/behaviour change, rewrite the whole doc block from scratch to the
current contract (comments.md budgets apply). Verify no doc block sits
directly above another `///` block after inserting items.
