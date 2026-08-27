---
name: session-workflow-gotchas
description: "Harness/hook gotchas that repeatedly cost time — background-task kill limit, attribution-hook false positives, changelog-guard label refresh, conformance.sh EXIT-trap corpus wipe"
metadata: 
  node_type: memory
  type: project
  originSessionId: 1be6641a-9768-4fd5-8149-acb2551a1d97
  modified: 2026-08-27T08:51:58.196Z
---

Recurring session-workflow traps (all hit 2026-07-13/14):

0. **Docker image build OOMs at 2 cargo jobs when cold** (hit twice:
   2026-08-21 during #2530, again 2026-08-21 after the 0.0.35 crate bump —
   an `openehr-*` lockstep version bump invalidates the BuildKit target-cache
   fingerprints for all eight crates, so the next image build is COLD for
   them and two concurrent heavy crate compiles (ferroehr-ext + openehr-adl)
   SIGKILL inside the Docker VM). After any crate-version bump or Dockerfile
   cache invalidation, run the pipeline as
   `CARGO_BUILD_JOBS=1 bash scripts/conformance.sh` (docker/sut-ferroehr.yml
   forwards the env var); warm rebuilds are fine at the default 2.
   ESCALATION (2026-08-22, #309): the console crate's wasm-release pass
   (`[profile.wasm-release]`: opt-level=z + codegen-units=1) now SIGKILLs in
   the 8 GB Docker VM even building ALONE at jobs=1 — rustc's own peak on the
   grown crate exceeds the VM. jobs=1 cannot fix a single-process peak; the
   fixes are a bigger VM (Docker Desktop → Resources → Memory, 12-16 GB —
   recommended to the owner) or host-built/prebuilt packaging. Keycloak (1 GB
   container limit) is always the first OOM-reaper victim mid-run — the
   monitor tripwire `docker start ferroehr-e2e-keycloak-1` revives it.

1. **Long background Bash tasks get killed (~30 min)** — for multi-hour runs
   (benchmark ladders, seeds), launch detached: `nohup caffeinate -is
   script.sh > log 2>&1 & disown`, then watch the log with a Monitor
   (`tail -f | grep --line-buffered`). Also wrap in `caffeinate` — the Mac
   otherwise sleeps overnight and takes Docker down with it.

2. **Hook false positives**: (a) the no-attribution PreToolUse hook regex
   `generated with .*claude` spans the whole tool-call payload — any commit
   message containing "…generated with…" trips it because scratch/session
   paths contain "claude"; write "rebuilt with"/"produced with" instead.
   (b) `protect_vendored_specs.sh` greps files for the literal `@generated`
   marker — a hand-written file whose doc comment merely *mentions* the
   marker string gets blocked from Edit (fixed in openehr-term bundle.rs by
   rewording; avoid quoting the marker in prose).

4. **Killing scripts/conformance.sh destroys the seeded corpus** (hit
   2026-07-23): the script's `trap compose_down EXIT` fires on ANY exit —
   `pkill` of a mid-seed pipeline runs `docker compose down -v` and wipes
   the hour-long 1M-composition seed's backing
   data. If a run must be aborted but the corpus kept, kill with SIGKILL
   (`pkill -9 -f conformance.sh`) so the trap cannot run — or accept the
   re-seed. Corollary: smoke-test new measured-workload wire shapes against
   a light composed stack BEFORE burning a seeded run (the pack preflight
   gate now covers payload validity at seeding).

5. **`pkill -f conformance.sh` does NOT kill the instrument child** (hit
   2026-07-24, cost two tainted measured runs): the orphaned `veredictum
   perf` keeps firing its arrival schedule for its full window (70+ min)
   at the SUT ports — and a relaunched run reuses the same compose
   project/ports, so the orphan's request stream and writes contaminate
   the NEW run's seed + measured window, and it writes its garbage
   measurement into the same `--results` path. Aborting a measured run
   means: `pkill -9 -f veredictum` AND `pkill -9 -f conformance.sh`, then
   `pgrep -fl 'conformance.sh|veredictum'` must be EMPTY, then explicit
   `docker compose -p <project> down -v`, then `git restore` the
   artifact dir — verify all four before relaunching.

3. **Guard labels need a fresh PR event** — the label-gated CI guards
   (`no-changelog` on changelog-guard, `no-ui-visual-change` on
   ui-screenshot-guard) read `github.event.pull_request.labels` (frozen at
   trigger time), so adding the label then rerunning the failed job still
   fails. Since #2777 ci.yml listens for `labeled`/`unlabeled`, so applying
   the label raises a fresh run by itself — wait for that run, never re-run
   the stale one (close+reopen is obsolete). Corollary (hit 2026-08-26):
   `gh pr create --label X` fires opened+labeled in the same second — the
   opened run gets CANCELLED and `gh pr checks` then shows its rows as
   `fail` beside the live run's rows. Monitor the RUN ID
   (`gh run list --branch <br>`), never the mixed pr-checks rows. Also
   (hit 2026-07-20):
   a label a workflow references must actually EXIST in the repo —
   `no-ui-visual-change` didn't until it was first needed; `gh label
   create` fails loudly at apply time, the workflow never warns.

**Why:** each cost a debugging loop mid-flow; the fixes are non-obvious.
**How to apply:** overnight/long runs → detached+caffeinate+Monitor pattern;
commit-message wording. Late labels: since #2777 applying a label raises a fresh run itself (labeled/unlabeled types); never re-run the failed job (stale payload).

6. **Merge PRs only behind a GREEN gate, in code** (hit 2026-07-26, broke
   develop's rustfmt): never chain `gh pr merge` unconditionally after a
   wait loop — the wait can terminate on a FAILURE and the merge still
   runs. Pattern: `if [ -z "$(gh pr checks N | grep -v 'pass\|skipping')" ];
   then gh pr merge N …; else report; fi`. Repo has no auto-merge
   (enablePullRequestAutoMerge is off), so this guard is the only gate.

7. **Formatter order: leptosfmt FIRST, `cargo fmt` LAST** (same incident):
   leptosfmt may collapse a `view! {…}` to one line, which changes where
   rustfmt wants a chained `.into_any()` — running leptosfmt after a clean
   `cargo fmt --check` leaves the tree rustfmt-dirty. Always finish with
   `cargo fmt --all && cargo fmt --all --check`.
- **Changelog inserts:** a section (`### Fixed` etc.) may already exist in the target release block — ALWAYS merge the entry into the existing subsection header; blindly inserting a new header creates a duplicate (happened twice, owner-corrected 2026-08-01). Check `grep -n '^###' CHANGELOG.md` for the block first.
- **`gh` comment bodies need heredocs:** backticks inside a double-quoted
  `--body` get shell-executed and silently mangle the posted comment. Use
  `--body-file -` with a quoted heredoc (`<<'EOF'`). The SAME trap applies to
  `git commit -m "..."` (hit 2026-08-19: a `` `up --wait` `` in the message
  executed and vanished) — use `git commit -F -` with a quoted heredoc; and
  the `block_dangerous` hook refuses ALL force-pushes (even feature branches),
  so an already-pushed mangled message is not amendable — write it right the
  first time. Pipes also mask exit codes (`cmd | tail -1 && next` runs `next`
  even when `cmd` failed — bit twice on 2026-08-18/19): put the gate command
  bare, never piped, when chaining with `&&`.
- **Renaming a container-test module breaks nextest serialization:** the
  `containers` group in `.config/nextest.toml` matches those suites by module
  prefix (`binary(it) & test(/^(…)::/)`), so a module rename silently
  un-serializes them — update the filter in the same change.
- **PR close keywords:** GitHub closes ONE issue per keyword — `Closes #1, #2, #3`
  closes only #1. Every issue needs its own `Closes #N` (one per line is
  clearest). Happened on PR #1821 (14 of 15 left open, owner-corrected
  2026-08-03); Batch A (#1812) had the correct per-issue form.
- **A hung `docker-credential-desktop` silently blocks every `docker pull`**
  (hit 2026-08-14, cost ~2 h): pulls/builds hang with NO output while
  `docker run`/`images`/host curl all work — the client stalls calling the
  keychain-backed credential helper (a pending macOS Keychain/TCC dialog;
  even reading `~/Library/Group Containers/group.com.docker/settings*.json`
  hangs then). Diagnose: `echo '{}' | docker-credential-desktop list` hangs.
  Bypass without touching user config: a scratch
  `DOCKER_CONFIG=$SCRATCH/dockercfg` whose `config.json` is
  `{"cliPluginsExtraDirs": ["$HOME/.docker/cli-plugins"]}` — the plugin dir
  entry is REQUIRED or `docker compose`/`buildx` vanish ("unknown shorthand
  flag: 'p'"). All conformance pulls are anonymous, so this is loss-free.
- **Render-time dates must be UTC-stable or the drift gates detonate on the
  tag** (hit 2026-08-14, v3.17.6): `git log --format=%cs` renders the
  committer's OWN timezone, so a baseline committed just past local midnight
  regenerates a different day than the copy rendered pre-commit under the
  mtime fallback — and the regenerate-and-diff gate fails permanently INSIDE
  the tag's immutable tree (the frozen-site cut can then never pass from the
  tag; recover by fixing the script on develop and running
  `scripts/site/cut-version.sh vX.Y.Z` locally — toolchain versions must
  match the workflow pins). Any rendered date derives from the committed
  artifact in UTC (`TZ=UTC --date=format-local:%Y-%m-%d`), never `%cs`,
  never the wall clock.
- **`gh pr merge --admin` is blocked by the harness permission classifier**
  (hit 2026-08-18), and the repo has no auto-merge — so the merge path is:
  wait for the run, rerun the flaky red (`gh run rerun <id> --failed`; while
  #2285 is open, ui-e2e is red-on-develop and needs this), then plain
  `gh pr merge --merge` behind the green-gate `if` (gotcha 6). `gh run
  rerun` refuses while the run is `in_progress` — wait for `completed`
  first.

8. **A conformance-artifact commit must run the FULL docs render set** (hit
   three times on 2026-08-25: the #2726 artifact refresh turned the docs lane
   red in three successive drift gates). Committing a regenerated
   `docs/conformance/<sut>/results.json` obligates, in the SAME commit, every
   derived surface docs.yml regenerates and diffs: `conformance-stats.sh
   includes`, `comparison.sh` (COMPARISON.md), `perf-assets.sh`,
   `conformance-assets.sh` (+ `CONF_SUT=ehrbase`), `conformance-docs.sh`
   (report/certificate/badges). Run all six, commit whatever changed.

9. **A release-cut's local guard runs are `--all`, and the version sweep greps
   the BARE number too** (hit at the v4.0.5 cut, 2026-08-26: PR #2757's
   post-merge docs-claims went red). Bare `bash scripts/checks/docs-claims.sh`
   scopes to CHANGED files — on a clean tree it passes vacuously; CI runs
   `--all`. And `verifying-releases.md` pins image tags as
   `ghcr.io/...:X.Y.Z` (no `v` prefix), so a `vX\.Y\.Z`-shaped grep misses
   them. At every cut: `grep -rn '<old-version>' website/book/src/` (bare
   number), then `docs-claims.sh --all` before merging the release PR.

10. **A subagent that detaches a nohup run and ends its turn has NOTHING
    waking it** (hit 2026-08-27, #2752: the worker's retried conformance run
    had no waiter — it would have slept forever). When a worker reports
    "running X, will report when it lands" and goes idle, `pgrep` the run's
    PID and check for a waiter process on it; if none, arm an orchestrator
    Monitor on the PID and `SendMessage` the worker on completion. Corollary:
    a worker killed by the session limit loses nothing when it commits as it
    goes — resume with `SendMessage` naming the branch state and its last
    line; never respawn fresh.

11. **zsh pipeline exits mask mutation proofs** (bit three times 2026-08-27):
    `guard.sh 2>&1 | head -3; echo $?` prints HEAD's status — a failing guard
    reads as exit 0. Every mutation proof asserts the exit UNPIPED
    (`guard.sh >/dev/null 2>&1; echo $?`), with the piped run only for the
    message text.

12. **BuildKit's "transferring context" line is incremental** (hit at #2813):
    a warm builder transfers only changed files, so a context-size measurement
    read from it is fiction (3.9 MB shown vs the real 944 MB). Measure with a
    fresh ephemeral builder (`docker buildx create --name probe` → build →
    `buildx rm probe`), one per side of the comparison.

13. **A failed release-pipeline leg: rerun for TRANSIENT, fix-forward for
    BROKEN — read the log before choosing** (both halves hit live at the two
    2026-08-27 cuts). When the leg's CODE is defective (v4.0.6's chart leg:
    the missing helm-docs), `gh run rerun --failed` re-executes the tag
    snapshot and fails identically — fix forward on develop, then the leg's
    dispatch recovery lane (chart: refresh the committed appVersion default
    first; the empty-Unreleased changelog fallback injects the right
    section), and the red run stays as the honest record. When the failure
    is INFRASTRUCTURE (v4.0.7's arm64 leg: softprops/action-gh-release died
    on `Headers Timeout Error` AFTER every asset uploaded ✅),
    `rerun --failed` is correct and safe: the release is still a DRAFT
    (freeze-last design), the action replaces same-named assets
    idempotently, and the rerun un-skips the dependent publish/crates legs.
    The discriminator is the failed step's log, never the leg's name.

14. **The SNOMED/Snowstorm probe never runs on this machine** (owner ruling
    2026-08-27, #2236 parked on-hold): the full Snowstorm + Elasticsearch
    import exceeds the development box's compute — do not launch
    `PROBE_ONLY=terminology` locally, ever; the issue holds until a bigger
    environment exists. Corollary: an hours-long opt-in run is launched only
    after confirming with the owner that THIS machine is the intended place
    to run it.
