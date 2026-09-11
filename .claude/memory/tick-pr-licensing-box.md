---
name: tick-pr-licensing-box
description: Every PR body must carry the template's EXACT ticked licensing line; a paraphrase fails contribution-licence-guard
metadata:
  type: feedback
---

Every PR body I open carries the contribution-licensing line, ticked, in the
template's EXACT wording (`.github/pull_request_template.md` § Licensing of
contributions):

`- [x] I accept the terms in [CONTRIBUTING.md § Licensing of contributions](../CONTRIBUTING.md#licensing-of-contributions): I have the right to submit this work, I license it under the project licence of the version it lands in, and I grant the Licensor the relicensing right stated there.`

**Why:** `scripts/checks/contribution-licence.sh` greps for that literal
prefix. A paraphrase ("I license this contribution under the terms in
CONTRIBUTING.md") turned #3270 red on 2026-09-11 on a formality, as did
leaving the box unticked earlier.

**How to apply:** copy the line from the template (or from the previous PR
body), never retype it; fix a red guard with `gh pr edit --body-file`, which
raises a fresh run by itself. See [[session-workflow-gotchas]].
