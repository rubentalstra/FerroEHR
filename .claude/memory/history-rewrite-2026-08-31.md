---
name: history-rewrite-2026-08-31
description: main's history was rewritten 2026-08-31 to a single import root; it is disjoint from every tag up to v4.0.13, which changes the first release cut after it
metadata:
  type: project
---

On 2026-08-31 (issue #2962) `main` was rewritten: the 4864 inherited EHRbase
commits were replaced by one labelled root commit `Import: EHRbase as the fork
point` (`5d93ba82f`), followed only by this project's own 2927 commits, all
re-signed with the owner's key. Tags and releases were deliberately left
untouched, so `main` is DISJOINT from every tag up to `v4.0.13`.

**Why:** the inherited commits put ~60 upstream authors on the repository's
contributor list, misrepresenting a project that shares no code with EHRbase.
Tags could not move because releases from `v3.17.4` on are immutable, which
locks each tag to its commit.

**How to apply:** at the FIRST release cut after the rewrite (`v4.0.14`), expect
`git describe` against `main` to find no tag, "commits since last release" to be
uncomputable, and the `v4.0.13...v4.0.14` changelog link to render a full-tree
diff. Do not switch the release lane to GitHub's generated notes for that cut;
notes come from `CHANGELOG.md` already. Later cuts behave normally. The
pre-rewrite lineage stays reachable through the old tags (EHRbase's root is still
an ancestor of `v4.0.13`), so the inherited history is recoverable from the
remote; the mirror backup was deleted by owner decision the same day, so there is
no way back to `main`'s exact pre-rewrite object graph. See
[[verified-commits-hard-rule]] for why every rewritten commit had to be
re-signed.
