---
paths:
  - "website/**/*.md"
  - "*.md"
  - ".github/**/*.md"
  - "docs/*.md"
---

# Prose style — no AI tells (owner directive 2026-08-24, issue #2623)

Applies to every piece of prose a human reads as text: the website book and
landing page, the root `README.md`, `CONTRIBUTING.md` and its siblings,
issue and PR bodies, release notes, forum/announcement drafts, and doc
comments where they carry prose. It does not rewrite the vendored specs
(`docs/specs/**` is never edited) and it does not loosen the technical rules
(citations, honesty, comment budgets all still apply).

## The banned tells

1. **The "Not X, but Y" setup.** Framing points as contrasts: "It's not
   just a tool; it's an ecosystem", "X is not a feature, it is a
   philosophy", and the same move spelled "rather than", "instead of
   merely", "never simply". State what the thing IS and stop. A contrast is
   allowed when the reader genuinely holds the wrong belief and the sentence
   corrects it with facts on both sides.
2. **The rule of three.** Adjectives or clauses grouped in neat triads on a
   metronome beat: "Fast, simple, powerful", "parse, validate, and
   flatten" as decoration. Real enumerations of real things keep their real
   length; decorative triads get cut to the one word that matters.
3. **Overused buzzwords.** delve, robust, elevate, testament, landscape,
   seamless, leverage, empower, unlock, journey (metaphorical), cutting-edge,
   state-of-the-art, game-changing, holistic, synergy. Use the plain verb:
   read, strong, improve, shows, area, works with, use, let.
4. **The em dash habit.** Em dashes used as a crutch to bolt explanatory
   clauses onto sentences, several per paragraph. Most of them are a comma,
   a period, or parentheses. Budget: an em dash is fine occasionally; two in
   one paragraph is the tell firing. A bullet that defines a term uses a
   colon inside the bold, never a dash: `- **Change events:** text`, not
   `- **Change events** — text` (owner directive 2026-08-24).
5. **Vague transitions.** Corporate filler openings: "In today's fast-paced
   digital world", "We stand at an inflection point", "As the healthcare
   landscape evolves". Open with the subject of the section.

## How to write instead

Short sentences. Concrete nouns and numbers over adjectives. Say who does
what. If a sentence still reads fine after deleting a clause, the clause was
decoration. Prefer "the server refuses X with a 400" over any sentence about
what the server "is designed to" do.

## Enforcement

Review-enforced (prose has no lint). The page-per-page website sweep is
#2623; new prose is held to this rule at review from 2026-08-24 on.
