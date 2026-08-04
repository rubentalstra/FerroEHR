# The public roadmap board (GitHub Project v2)

The tracker is GitHub Issues (`CLAUDE.md` §Issue workflow); milestones are the
release spine; labels carry type + priority; native edges carry
decomposition/sequencing (`issue-relationships.md`). The **"FerroEHR Roadmap"
Project** (a GitHub Project v2 under the repo owner, public) exists for one
reason: **outward transparency** — anyone can see what is planned, in
progress, and shipped, without reading the raw issue list. It is a **VIEW over
the tracker, never a second tracker.** This file is the policy (what the board
may and may not carry) and the canonical commands (the one sanctioned write
path is `scripts/gh-project.sh`).

## The one-datum rule

**Status (`Todo` / `In Progress` / `Done`) is the ONLY board-managed datum.**
Everything else the board displays is read straight from the issue and already
has a canonical home:

| Fact | Canonical home | NEVER duplicated as |
|---|---|---|
| Priority | `P0`–`P3` labels | a board Priority field |
| Type | `bug`/`enhancement`/… labels | a board Type field |
| Release | the `vX.Y.Z` milestone | a board Release/Iteration field |
| Decomposition | native sub-issue edges | a board hierarchy field |
| Sequencing | native blocked-by edges | a board Blocked column/field |

Do not add custom fields, iteration fields, estimate fields, or extra Status
options. A board-only fact has no backlink, is invisible to `gh issue`
consumers (the SessionStart dump, `/phase-status`, `/next-task`), and rots the
first time it disagrees with the label/milestone it shadows — the same decay
class `issue-relationships.md` §No duplication bans for issue bodies. If the
board ever needs to show a new fact, give the fact a canonical home on the
ISSUE (label, milestone, native edge) and let the board filter/group on it.

## Status semantics + who moves it

- **`Todo`** — every open issue starts here (the auto-add workflow sets it).
- **`In Progress`** — set at pickup, when work on the issue actually starts
  in a session: `scripts/gh-project.sh status <n> in-progress`. `/next-task`
  does this as its final step. This is the ONE manual move in the lifecycle —
  GitHub has no built-in "branch/PR opened → In Progress" workflow.
- **`Done`** — never set by hand. The issue closes via the PR's `Closes #N`
  and the built-in "item closed → Done" workflow moves it. A reopened issue
  goes back to `Todo` automatically.

An issue abandoned mid-flight (session ended, work parked) goes back to
`todo` explicitly — a stale `In Progress` column is a false public claim.

## The one sanctioned command surface — `scripts/gh-project.sh`

Projects v2 writes (`gh project item-edit`) take four opaque GraphQL node ids
(project, field, option, item) — never the issue `#number`. The helper
resolves them all from the `#number` and fails loud (the `gh-rel.sh` pattern).
Requires the `project` token scope (`gh auth refresh -s project`).

| Intent | Command |
|---|---|
| Start work on #n | `scripts/gh-project.sh status <n> in-progress` |
| Park #n (work stopped, not done) | `scripts/gh-project.sh status <n> todo` |
| Put #n on the board (auto-add missed it) | `scripts/gh-project.sh add <n>` |
| Read #n's board status | `scripts/gh-project.sh show <n>` |
| Print the whole board by column | `scripts/gh-project.sh board` |
| Print the project URL | `scripts/gh-project.sh url` |

Never move `Done` by hand, never `gh project item-edit` raw, and never
`item-archive`/`item-delete` — closed items stay visible as the shipped
record (the built-in auto-archive workflow stays OFF).

## Board configuration (the intent, for anyone recreating it)

Fields: the built-in `Status` with exactly `Todo` / `In Progress` / `Done`.
Views (view creation is UI-only — no API):

1. **Board** — kanban grouped by Status; the "what is going on right now"
   surface.
2. **Roadmap** — layout Roadmap, items placed by **milestone due date**; the
   release timeline (every open `vX.Y.Z` milestone carries a due date —
   set one when creating a milestone).
3. **Current focus** — table filtered `label:P0,P1 is:open`, grouped by
   milestone.

Built-in workflows (verified gh 2.96.0 / docs 2026-08-04: **no API — UI-only
toggles**; visibility + repo-link ARE scriptable via `gh project edit
--visibility` / `gh project link`): a new project ships **Item closed →
`Done`** and **Pull request merged → `Done` enabled by default**; enable by
hand **Auto-add to project** (filter `is:issue is:open`), **Item added to
project → `Todo`**, and **Item reopened → `Todo`**; leave auto-archive OFF.
(An Actions-based alternative — `actions/add-to-project` — exists but needs a
PAT secret for a user-owned project; three one-time UI toggles beat a standing
secret.) Visibility: public.

## Interaction with the rest of the workflow

- **`/next-task`** moves the picked issue to `In Progress` once the plan is
  accepted and work starts.
- **`/phase-done`** verifies the closing issue lands in `Done` (the merge +
  workflow do it; the skill only checks).
- **`/phase-status`** may cite `scripts/gh-project.sh board` for the public
  view, but the issue list stays the working ground truth.
- The **board readme** (`gh project edit --readme`) carries the durable
  direction themes — the former root `ROADMAP.md` was retired into it
  (2026-08-04, issue #1867); never resurrect a roadmap markdown file, and
  keep the readme themes-only (no item-level state, no quotable numbers —
  those live in issues and the committed artifacts). `README.md` links the
  board for visitors.

## Official documentation (durable citations)

- gh project commands — https://cli.github.com/manual/gh_project
- Projects v2 API — https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-api-to-manage-projects
- Built-in workflows — https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-built-in-automations
- Roadmap layout — https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-roadmap-layout
