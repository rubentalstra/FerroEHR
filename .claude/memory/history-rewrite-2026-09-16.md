---
name: history-rewrite-2026-09-16
description: main was rewritten on 2026-09-16 to drop the #2708 contributor commit (fc680e96f); 371 commits re-signed; tags v4.0.14 to v4.3.0 sit on the old lineage, so nothing may resolve "latest release" by ancestry
metadata:
  type: project
---

On 2026-09-16 the owner had `main` rewritten to remove the one external-contributor commit, fc680e96f (PR #2708, six lines in a book page already gone from the tree). Method: a scratch clone in the session scratchpad sharing objects with the checkout, `git rebase --onto fc680e96f^ fc680e96f main` (one conflict at the commit that had removed the lines, resolved by taking that commit's file), then `git rebase --exec 'git commit --amend --no-edit --no-verify -S'` to re-sign all 371 commits with the owner's key after the owner cached the passphrase (`echo test | /usr/local/MacGPG2/bin/gpg --clearsign -u E8A20FFEBE345065`), verification of an identical tree, and the owner's own force-push with `--force-with-lease` (the hook and the ruleset block it for me; the owner holds the bypass). Old head e2adeadfc, new head 3e205b9f2; local ref `backup/main-before-2708-rewrite` keeps the old head.

**Why it matters:** the 14 release tags v4.0.14 to v4.3.0 are immutable and stay on the old lineage, so `git describe` from `main` finds no tag until v4.3.1 is cut. The immutability guard was switched to the highest version tag regardless of ancestry (PR after #3449). Any other tool that resolves "the latest release" by reachability breaks the same way; the changelog compare link for v4.3.1 will show a merge base at fc680e96f^ and over-count, as after [[history-rewrite-2026-08-31]].

**How to apply:** never resolve the latest release by ancestry; when asked to remove a commit from history, verify tree identity, re-sign, hand the owner the lease-guarded push, and tell every open worker branch to `git rebase --onto origin/main <old-main-sha> HEAD`.

**Second consequence (found the same evening):** re-signing restamps every COMMITTER date to the rewrite day; the conformance renderers read `git log --format=%cd`/`%cs` for the run date, so the Docs build's drift gate failed on every main push until they read the author date (`%ad`/`%as`). Anything that derives a date from git must use the author date.
