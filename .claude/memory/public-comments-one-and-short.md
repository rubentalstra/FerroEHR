---
name: public-comments-one-and-short
description: "Public-facing GitHub comments (external contributors' issues, discussions) are ONE short plain comment — never two, never long, never AI-sounding"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32d068af-12e7-4654-9ece-124240b2367f
  modified: 2026-08-26T08:41:37.878Z
---

On issues opened by external contributors (and any public-facing thread), post exactly ONE comment, short and plain. Never split into a main comment plus a follow-up precision; never write multi-section adjudication essays there.

**Why:** owner reaction 2026-08-26 ("you wrote two comments what the hell and do not write to much okay and do not amke it sound like AI") after two long comments landed on #2759 (Joost's issue). Both were deleted and replaced with one short comment.

**How to apply:** draft the comment, cut it to the facts (transcript snippet, one-line ruling, the concrete remedy), apply [[owner-work-style]] no-AI-tells rules (no triads, no "not X but Y", minimal em dashes), and post once. Long derivations belong on internal PRs/issue bodies, not on an external contributor's thread. If new facts arrive after posting, EDIT the existing comment (gh api PATCH) instead of adding another.
