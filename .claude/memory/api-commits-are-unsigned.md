---
name: api-commits-are-unsigned
description: Commits created through the GitHub Git Data API with the user's token are NOT signed and trip the owner's vigilant mode; commit locally (signed) even when a worker holds the checkout
metadata:
  type: feedback
---

Commits I create through `gh api .../git/commits` (or the contents API) with the user's token are attributed to the owner but unsigned; GitHub shows "This commit is not signed, but one or more authors requires that any commit attributed to them is signed" and the owner asked "what the hell is this" (2026-09-14). Only GITHUB_TOKEN/App commits are GitHub-signed; a user-token API commit is not.

**Why:** the owner's vigilant mode and the verified-commits rule ([[verified-commits-hard-rule]]) require every commit attributed to them to carry a signature.

**How to apply:** never commit through the API from a session; when a worker holds the checkout, stage the change in the scratchpad and commit locally (signed) once the checkout is free, or hand the file to the worker. Squash-merging an unsigned branch commit still yields a signed main commit, but the branch history is what the owner sees.
