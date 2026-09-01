---
name: leptos-lookup
description: >
  Finds and reads the authoritative Leptos guidance for a topic (signals,
  effects, <For>, resources, Suspense/Transition, server functions,
  extractors, SSR modes, hydration bugs, router, ActionForm, islands,
  binary size) in the official Leptos book, cached locally. Use before
  implementing or reviewing any viewer (app/ferroehr-viewer) behaviour
  the rule file doesn't fully settle, or when a "how does Leptos do X"
  question comes up.
allowed-tools: [Read, Grep, Glob, Bash]
argument-hint: "<signal / resource / hydration / server fn / router topic>"
---

# /leptos-lookup

Answer Leptos questions from the official book text — never from memory
(Leptos is pre-1.0 and moves fast; training data drifts). The distilled
rules live in `.claude/rules/leptos-ui.md`; this skill is for going back to
the source when a case isn't covered or needs the full context.

## Procedure

1. **Ensure the book cache exists** (shared, survives sessions):

   ```bash
   BOOK=~/.cache/ferroehr/leptos-book
   [ -d "$BOOK/src" ] || git clone --depth 1 https://github.com/leptos-rs/book "$BOOK"
   ```

   If the cache is older than ~30 days (`git -C "$BOOK" log -1 --format=%cr`),
   `git -C "$BOOK" pull --ff-only` first. The book's `main` targets the
   current Leptos 0.x line — cross-check any version-sensitive answer
   against the pinned Leptos version in the UI crate's `Cargo.toml`.

2. **Route to the owning chapter** (`src/SUMMARY.md` is the index):
   - signals / effects / memos / derived → `src/reactivity/*`,
     appendices `appendix_reactive_graph.md`, `appendix_life_cycle.md`
   - components / props / children / context → `src/view/03…09_*`,
     `src/interlude_projecting_children.md`
   - lists & keys → `src/view/04_iteration.md`, `04b_iteration.md`
   - control flow / errors → `src/view/06…07_*`
   - forms → `src/view/05_forms.md`; progressive enhancement →
     `src/progressive_enhancement/*`
   - async (Resource/Suspense/Transition/Action) → `src/async/*`
   - server fns / extractors / responses → `src/server/*`
   - SSR modes / page lifecycle / **hydration bugs** → `src/ssr/*`
   - router → `src/router/*`; global state → `src/15_global_state.md`
   - islands → `src/islands.md`; WASM size / deployment →
     `src/deployment/*`; styling → `src/interlude_styling.md`;
     head/meta → `src/metadata.md`; testing → `src/testing.md`;
     web_sys/JS interop → `src/web_sys.md`
3. **Grep** the exact API name (`Suspend`, `ForEnumerate`, `bind:value`,
   `extract_with_state`, …) across `src/**/*.md` when the routing isn't
   obvious.
4. **Read the surrounding section**, including the warnings/admonish blocks
   — the book's DON'Ts live there.
5. **Answer with citations** (`<chapter>.md` + heading). For API-signature
   detail beyond the book, follow its docs.rs links via Context7/WebFetch
   rather than guessing. If the answer belongs in the standing rules,
   propose the `.claude/rules/leptos-ui.md` addition explicitly.
