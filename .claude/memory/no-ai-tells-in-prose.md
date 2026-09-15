---
name: no-ai-tells-in-prose
description: HARD RULE (owner 2026-08-24, made strict 2026-09-15) — every issue comment, Discourse reply, PR body and page is written the way a person writes to a colleague; a fixed list of AI words, transitions, hedges, buzzwords and closing formulas is banned outright, plus flattery and padding
metadata:
  type: feedback
---

Owner, 2026-09-15, after two rewrites of a Discourse reply: "make it not sound like AI", "nobody writes like this at all", "remove all this filling stuff", and the full list below is "never ever allowed", for Discourse replies AND tracker comments alike.

**Banned words:** delve, underscore, pivotal, realm, harness, illuminate, shed light on, facilitate, refine, bolster, differentiate, streamline, leverage, elevate, testament, landscape, seamless, empower, unlock, journey, robust, revolutionize, innovative, cutting-edge, game-changing, transformative, holistic, synergy, scalable solution, seamless integration.
**Banned transitions and formulas:** "that being said", "at its core", "to put it simply", "this underscores the importance of", "a key takeaway is", "from a broader perspective", "in today's …", "so, to X's original question", "in short", "in summary", "two things follow", "where I land", "the honest summary is", any sentence that announces what the next sentence does.
**Banned hedges:** generally speaking, typically, tends to, arguably, to some extent, broadly speaking, "I think the position can be stated".
**Banned padding:** praise of the reader's work ("you did the hard part", "that saved me a day", "careful report", "exactly the right question"), recaps of what the reader already knows, closing summaries, rhetorical set-ups ("Two things from our side."), em dashes as glue, triads for rhythm, "not X but Y".
**Banned claims:** saying something was done when it was not (an upstream report "filed" when only a tracker issue exists); always give the full link to what exists.

**Why:** the owner signs these texts; a reader on openEHR Discourse or GitHub recognises the patterns instantly and discounts the content.

**How to apply:** write facts in the order a colleague would say them, address people by handle once, one thank-you clause at most, cite files and links, stop when the facts stop. Before posting, grep the draft against the banned lists above. The durable rule is `.claude/rules/writing-style.md` items 6 to 8. Related: [[public-comments-one-and-short]].
