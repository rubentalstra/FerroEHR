---
name: comments-less-is-more
description: Owner hard rule — RFC 505/1574 comment style, hard budgets, no essays in code; machine-enforced
metadata:
  type: feedback
---

Owner rulings 2026-08-04: code comments follow the official Rust conventions
(RFC 505 + RFC 1574) with hard budgets — `// NOTE:` = citation + one sentence
(≤3 lines), plain `//` runs ≤8 lines, block comments banned, every TODO is
`TODO(#NNNN):`. Adjudication essays and change-narration live on the PR/issue,
never in source. Full guide: `.claude/rules/comments.md`; enforced by
`scripts/check-comment-style.sh` (edit hook + `comment-style` CI job) and
`clippy::too_long_first_doc_paragraph`.

**Why:** comments are the fastest-rotting artifact in the repo — 30-line
adjudication essays were accumulating (191 over-budget runs, 100 over-budget
NOTEs at measurement, 2026-08-04) and stale fast.

**How to apply:** write the citation + one sentence, put the reasoning in the
PR description; when touching a file with an over-budget essay, condense it.
Records live on the tracker (root CLAUDE.md §Issue workflow).
