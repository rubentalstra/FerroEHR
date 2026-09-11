---
name: never-soft-reset-onto-moved-base
description: Squashing with `git reset --soft origin/main` while the branch base is behind main rewrites the commit as a revert of everything main gained since (deleted #3262's migration on #3265); squash only after merging main, and in zsh pass path lists as arrays, never an unquoted $VAR
metadata:
  type: feedback
---

Hit 2026-09-11 on PR #3265. The branch was cut from main before #3262 merged;
`git fetch && git reset --soft origin/main && git commit` produced a commit
whose tree was "old base + my work", i.e. it REVERTED every file #3262 had
changed and DELETED its new migration, and the following `git merge
origin/main` was a no-op because the base was now main itself. The damage was
only visible as `D`/`M` rows in `git status` that a `grep -v '^A\|^M'` hid.

**Why:** a soft reset moves the base without replaying the work; it is only a
squash when the branch already contains everything the new base has.

**How to apply:** to squash a branch, first `git merge origin/main` (resolve),
THEN `git reset --soft origin/main` is safe; or skip squashing (PRs squash on
merge anyway). Before pushing any rebuilt commit run
`git diff --stat HEAD origin/main` and read the whole list. Recovery: the
remote branch still has the old commits, so `git diff <old-base>
origin/<branch> -- <my files>` yields the real work as a patch. And in zsh an
unquoted `$LIST` is ONE word, so build path lists as arrays
(`FILES=(a b c); git diff -- "${FILES[@]}"`). Related:
[[session-workflow-gotchas]], [[concurrent-sessions-shared-tree]].
