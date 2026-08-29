---
name: no-ai-tells-in-prose
description: "Owner ban on AI writing tells in ALL prose Claude drafts (comments, replies, docs, PR text) — and drafted replies must match the owner's own plain forum register, not just avoid banned tokens"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 7cc03c10-781f-4577-82fe-2a7d7546a44c
  modified: 2026-08-29T17:58:12.937Z
---

Owner directive 2026-08-26, given angrily after a drafted GitHub reply used em dashes; reinforced angrily 2026-08-29 ("the text sounds so much like AI what the fuck") after a Discourse reply draft avoided every banned token yet still read as AI. Never write with AI tells in any prose meant for humans (issue/PR comments, reply drafts, docs, commit bodies). Banned specifically:

- Em dashes used to attach explanatory clauses. Use a period, comma, or parentheses instead.
- "Not X, but Y" contrast framing.
- Rule-of-three triads ("fast, simple, powerful").
- Buzzwords: delve, robust, elevate, testament, landscape.
- Vague corporate transitions ("In today's fast-paced world", "inflection point").

**The 2026-08-29 lesson: token-avoidance is not enough.** The rejected draft had no banned token but still had composed-essay rhythm: parallel paragraph structure, aphoristic closers ("That is a normal review cost, and I would rather name it than have someone discover it"), abstract framing sentences ("The number that matters is therefore not the page count"). The accepted rewrite imitated the owner's OWN prior forum posts: short factual sentences, concrete numbers up front, informal direct asks ("What I need from the SEC is: which channel, what pace"), no rhetorical shaping.

**Why:** drafted external replies represent the owner personally; AI-sounding text is embarrassing and defeats the point of [[public-comments-one-and-short]].

**How to apply:** before handing over any drafted prose, (1) sweep for em dashes and the tells above, (2) find a real post the owner wrote (Discourse/GitHub history is in most threads being replied to) and match ITS register, sentence length and informality, (3) delete any sentence that sounds quotable — if it would work as a slide caption, the owner would not have written it.
