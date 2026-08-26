---
name: no-ai-tells-in-prose
description: "Owner ban on AI writing tells in ALL prose Claude drafts (comments, replies, docs, PR text) — especially em dashes"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 7cc03c10-781f-4577-82fe-2a7d7546a44c
  modified: 2026-08-26T10:24:14.099Z
---

Owner directive 2026-08-26, given angrily after a drafted GitHub reply used em dashes: never write with AI tells in any prose meant for humans (issue/PR comments, reply drafts, docs, commit bodies). Banned specifically:

- Em dashes used to attach explanatory clauses. Use a period, comma, or parentheses instead.
- "Not X, but Y" contrast framing.
- Rule-of-three triads ("fast, simple, powerful").
- Buzzwords: delve, robust, elevate, testament, landscape.
- Vague corporate transitions ("In today's fast-paced world", "inflection point").

**Why:** drafted external replies represent the owner personally; AI-sounding text is embarrassing and defeats the point of [[public-comments-one-and-short]].

**How to apply:** before handing over any drafted prose, sweep for em dashes and the tells above and rewrite in plain sentences. Repo docs rule `writing-style.md` covers the website; this applies everywhere.
