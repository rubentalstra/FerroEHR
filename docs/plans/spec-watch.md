# Spec-update watcher — design dossier (tracker issue #137)

- Status: in-progress
- Started: 2026-07-20. Deleted in the PR that implements it.
- Every claim below was live-verified on 2026-07-20 (see §1–§5).


Scheduled GitHub Actions workflow that watches for newly COMPLETED openEHR
specification changes and opens one triageable `spec-update` issue per change.

Every claim below was verified against the LIVE web on 2026-07-20 (curl to the
openEHR Jira Cloud REST API, `gh api` to the upstream GitHub repos) and against
the local repo. Nothing here is from memory.

---

## 1. The openEHR Jira instance (openehr.atlassian.net)

### Anonymous access — YES, unauthenticated REST works

`GET https://openehr.atlassian.net/rest/api/2/project` returned HTTP 200 with
the full project array, no auth header. Anonymous read is enabled on the SPEC*
(and other public) projects.

### The old search endpoint is REMOVED (hard finding)

```
GET https://openehr.atlassian.net/rest/api/2/search?jql=...
→ HTTP 410
{"errorMessages":["The requested API has been removed. Please migrate to the
 /rest/api/3/search/jql API. A full migration guideline is available at
 https://developer.atlassian.com/changelog/#CHANGE-2046"]}
```

You MUST use the enhanced search endpoint:
`GET https://openehr.atlassian.net/rest/api/3/search/jql` (also
`/rest/api/2/search/jql`). This endpoint:
- returns `{"issues":[...], "nextPageToken":"...", "isLast":bool}` —
  **token pagination, not `startAt`/`total`**. Loop until `isLast:true`,
  passing `nextPageToken`.
- takes `jql`, `maxResults`, `fields`, `nextPageToken` as query params.
- Does NOT return a total count. For counts use
  `POST /rest/api/3/search/approximate-count` with `{"jql":"..."}` →
  `{"count":75}` (verified: SPECRM Resolved/Closed = 75).

### SPEC* projects (fetched live, anonymously readable)

From `/rest/api/2/project`, projectCategory "Specifications":

| Key | Name |
|---|---|
| SPECRM | Specifications: Reference Model |
| SPECBASE | Specifications: Base |
| SPECAM | Specifications: Archetype model |
| SPECLANG | Specifications: Languages |
| SPECQUERY | Specifications: Querying |
| SPECITS | Specifications: Implementation Technologies |
| SPECTERM | Specifications: Terminology |
| SPECSM | Specifications: Service model |
| SPECCNF | Specifications: Conformance |
| SPECPROC | Specifications: Process Model |
| SPECCDS | Specifications: Clinical Decision Support |
| SPECINTG | Specifications: Integration |
| SPECPUB | Specifications - publication |
| SPECPR | Specifications Problem Report (PR) |
| SPEC | Specification |

(All 15 confirmed present. The issue body's list is right; add SPECSM,
SPECCNF, SPECPROC, SPECCDS, SPECINTG, SPECPUB, SPEC to be complete. The ones
that map to our vendored components + the PR tracker are the load-bearing set:
SPECRM, SPECBASE, SPECAM, SPECLANG, SPECQUERY, SPECITS, SPECTERM, SPECSM,
SPECCNF, SPECPR.)

### Workflow statuses & "completed" semantics (verified verbatim)

`GET /rest/api/2/project/SPECRM/statuses` yields these status→category pairs:

```
Open        | new
Reopened    | new
Analysis    | indeterminate
In progress | indeterminate
In Review   | done          ← TRAP: green category but NOT resolved
Resolved    | done
Closed      | done
```

**Critical subtlety:** filtering by `statusCategory = Done` OVER-CAPTURES.
Live example, SPECRM-129 ("Add BMM definition of RM…"): status `In Review`,
`statusCategory.key = "done"` (green), yet `resolution = null` and
`resolutiondate = null`. "In Review" work is NOT completed.

The reliable "completed" signal is the **resolution** field being set. Real
resolved issue (`GET /rest/api/2/issue/SPECBASE-48`), fields verbatim:
- `status.name = "Resolved"`, `statusCategory.key = "done"`
- `resolution = {name:"Done", description:"Work has been completed on this issue."}`
- `resolutiondate = "2026-07-20T09:10:50.797+0100"`
- `issuetype.name = "Change Request"` (desc "A CR documents proposed and implemented changes")
- `fixVersions = [{name:"Release-1.3.0", description:"Release 1.3.0", released:false}]`
- `components = [{name:"BMM", description:"Basic Meta-Model specification"}]`

So: **`fixVersions[].name` gives the spec release (e.g. `Release-1.3.0`)** — the
exact field to cross-check against our `docs/VERSIONS.md` pins. `components`
gives sub-area (BMM, etc.). Older resolved issues (SPECRM-87/18/107) all carry
`resolution.name = "Done"` + a populated `resolutiondate`.

### JQL filtering by completion date — VERIFIED WORKING

```
project in (SPECRM,SPECBASE,SPECAM,SPECLANG,SPECQUERY,SPECITS,SPECTERM,SPECPR)
  AND status in (Resolved,Closed)
  AND resolutiondate >= -3000d
  ORDER BY resolutiondate DESC
```
→ HTTP 200, returned real recent completions:
`SPECBASE-48` (Resolved 2026-07-20), `SPECPR-454`/`406`/`395` (Closed
2026-07-19), `SPECITS-82` (Closed 2026-07-19). Relative dates (`-8d`),
multi-project `project in (...)`, and `ORDER BY resolutiondate` all work.
`resolutiondate` (alias `resolved`) is the transition timestamp; use it for the
"since last run" window. `status in (Resolved,Closed)` (not the broader "done"
category) plus a resolution filter is the precise gate.

Recommended poll JQL (one call, all components):
```
project in (SPECRM,SPECBASE,SPECAM,SPECLANG,SPECQUERY,SPECITS,SPECTERM,SPECSM,SPECCNF,SPECPR)
  AND status in (Resolved, Closed)
  AND resolutiondate >= -14d
  ORDER BY resolutiondate DESC
```
(14d window > the twice-weekly cadence so a missed run self-heals; dedup, not
the window, is the correctness guarantee — see §4.)

### Rate limits & pagination (anonymous)

- Atlassian Cloud REST is cost-budget rate-limited per client; over-budget →
  HTTP **429** with a `Retry-After` header (Atlassian rate-limit responses doc,
  developer.atlassian.com). Anonymous callers get a tighter budget than
  authenticated. Our load is tiny (1 search + N issue reads, twice weekly), so
  429 is not expected; still, treat any 429 as a hard failure (§5), do not
  swallow it.
- Pagination: token-based via `nextPageToken`/`isLast` on `/search/jql`
  (no `startAt`). Request modest `maxResults` (e.g. 50) and loop on the token.
- Fetch only needed `fields` (`summary,status,resolution,resolutiondate,fixVersions,components,issuetype`)
  to keep responses small.

**Fallback if anonymous ever breaks:** the same data is reachable via the
public Jira UI issue-navigator and RSS (`/sr/jira.issueviews:searchrequest-xml`),
but the JSON `/rest/api/3/search/jql` is the clean path and works today — no
fallback needed now.

---

## 2. Amendment-record tables in the upstream `specifications-*` repos

Every spec *document* carries an Amendment Record. Two formats exist:

### (a) AsciiDoc `master00-amendment_record.adoc` (the norm — RM/BASE/AM/LANG/QUERY/TERM/SM/CNF)

Verified format (`RM/docs/common/master00-amendment_record.adoc`, live at
`repos/openEHR/specifications-RM/contents/docs/common/master00-amendment_record.adoc`):

```asciidoc
= Amendment Record

[cols="1a,6,2,2a", options="header"]
|===
|Issue|Details|Raiser|Completed

|[[latest_issue]]2.1.5
|{spec_tickets}/SPECRM-87[SPECRM-87^]: Support tags: added <<tags, Tags Package>> ...
|S Iancu, + B Næss, + M Polajnar
|[[latest_issue_date]]17 Nov 2022

|
|{spec_tickets}/SPECRM-107[SPECRM-107^]: Add `inactive` and `abandoned` states ...
|J Holslag, + Jelte Zeilstra, + Bas Janssen
|06 Oct 2022
...
4+^h|*RM Release 1.1.0*     ← release-boundary header row
```

Columns are exactly **Issue | Details | Raiser | Completed**. Key facts for a
poller:
- The **Jira key is embedded in every Details cell** as
  `{spec_tickets}/SPECRM-87[SPECRM-87^]` — this is the join key to the Jira
  poll (dedup by key).
- The newest row is anchored `[[latest_issue]]` / `[[latest_issue_date]]`.
- Release boundaries are `N+^h|*<COMPONENT> Release <ver>*` header rows — this
  is where a doc's version bumps.
- `BASE/docs/base_types` uses `[cols="1,6,2,2"]` (non-`a`) but identical
  columns; column widths vary, headers do not.

### (b) HTML-table `Amendment_record.md` (ITS-REST overview)

`ITS-REST` overview uses Markdown-wrapped HTML
(`specifications/docs/overview/Amendment_record.md`):
```html
<table> ... <th>Issue</th><th>Details</th><th>Raiser, Implementer</th><th>Completed</th> ...
  <tr><td>5.9</td>
      <td><a href="https://specifications.openehr.org/tickets/SPECITS-95" ...>SPECITS-95</a>: Fix UPDATE_AUDIT.change_type ...</td>
      <td>H Heiser, S Iancu</td><td>11 Jun 2026</td></tr>
```
Same four columns (Raiser column adds "Implementer"); Jira key is an `<a href=…/tickets/SPECITS-95>`.
A parser must handle both `{spec_tickets}/KEY[KEY^]` (adoc) and
`.../tickets/KEY` (html) key forms.

### New-row detection — via the GitHub commits API on the file path

Verified: `gh api "repos/openEHR/specifications-RM/commits?path=docs/common/master00-amendment_record.adoc&per_page=3"`
returns commit SHA + date + message per touch of that file. Detection strategy:
for each watched amendment path, get its latest commit date; if newer than the
vendored pin's commit date (the SHAs in `scripts/vendor-spec-docs.sh`), the doc
gained rows. The precise NEW keys are then the Jira keys present in upstream
`master` HEAD of that file but absent from the vendored copy — a text diff of
the two file versions, extracting `SPEC[A-Z]+-\d+` tokens.

### Repos + paths to watch (all confirmed to exist; org `openEHR`)

Amendment records live at `<repo>/docs/<document>/master00-amendment_record.adoc`.
The full set (from the local vendored mirror, which was rsync'd from these
repos, plus the vendor script's repo list):

- `specifications-RM`: docs/{common,ehr,data_structures,data_types,ehr_extract,demographic,integration,support}/master00-amendment_record.adoc
- `specifications-BASE`: docs/{foundation_types,base_types,resource,architecture_overview}/master00-amendment_record.adoc
- `specifications-AM`: docs/{ADL2,AOM2,OPT2,ADL1.4,AOM1.4,Identification,Overview}/master00-amendment_record.adoc
- `specifications-LANG`: docs/{bmm,bmm3,bmm_persistence,odin,EL,BEL}/master00-amendment_record.adoc
- `specifications-QUERY`: docs/{AQL,AQL_examples}/master00-amendment_record.adoc
- `specifications-TERM`: docs/SupportTerminology/master00-amendment_record.adoc
- `specifications-SM`: docs/{openehr_platform,serial_data_formats,simplified_im_b}/master00-amendment_record.adoc
- `specifications-CNF`: docs/{guide,platform_test_schedule,certificate,profiles}/master00-amendment_record.adoc
- `specifications-ITS-REST`: specifications/docs/overview/Amendment_record.md **(HTML table, different path/format)** plus docs/{smart_app_launch,simplified_data_template,simplified_formats}/master00-amendment_record.adoc
- `specifications-ITS-XML`, `specifications-ITS-JSON`: check for their own amendment docs at re-vendor time.

### Our vendored pins (from `docs/VERSIONS.md` + `scripts/vendor-spec-docs.sh`)

Component | pin | vendored commit
---|---|---
BASE | 1.3.0 | `49f5bbe10992a645d7bd1e90c86d188b9587d13b`
RM | 1.2.0 | `c52de2b80503f3e8613dd4b7455b1b60336e9fac`
AM | 1.4.0 + 2.4.0 | `da06d63297e8549a351c854d8b1c45cd9f1d577c`
TERM | 3.1.0 | `007d0dddcdd77648711681878b54ace021b2fbd5`
LANG | 1.1.0 | `201b647034f7b1ddfe207e4c3c6f52f6878869b8`
QUERY | 1.1.0 | `a87bb51fa1c515b863c9610a9444a2d5570dc05a`
SM | master | `23ffc4711c10bae2ae43724b1948fe3b24a0964e`
CNF | master | `33251d2abe5a75c042e11c9385d2e9a79aa15904`
ITS-REST | development | `e8a093e9d6da2ae68d7cfc29cf260a7edb065f47`
ITS-XML | master | `de8b37ba6c9a5e126623a063cafba3b58ebf1107`
ITS-JSON | master | `5acae056248e917a4b4c56f7e712f4fcfeb616a6`

**Live evidence the pins are already stale:** `SPECBASE-48` resolved
2026-07-20 with `fixVersions=Release-1.3.0`; the BASE base_types amendment
record already shows a `1.3.1` latest row (07 Mar 2025) beyond our "1.3.0" pin.
The watcher will have real material to flag on first run.

---

## 3. Existing repo context (read locally)

- **`docs/VERSIONS.md`** — the pin table above; §"openEHR specification matrix"
  is the source of truth to compare against.
- **`scripts/vendor-spec-docs.sh`** — the `COMPONENTS=(…)` array is the exact
  `component|repo|human-ref|commit-SHA` list the watcher should read to know
  what is vendored and at which commit (the watermark denominator for §2).
- **Label taxonomy (live `gh label list`)** — all exist:
  `spec-update` (#B60205), component labels `spec:RM|BASE|AM|LANG|QUERY|ITS|TERM|SM|CNF`
  (all #0E8A16), triage `spec-impact:behaviour` / `spec-impact:docs-only` /
  `spec-impact:none`. Also `ci`, `P0/P1/P2`, and Conventional-type labels
  (`chore/perf/refactor/enhancement/bug/documentation`).
  NOTE: there is a `spec:ITS` label but the Jira/repos split ITS into
  ITS-REST/ITS-XML/ITS-JSON and SPECITS — map all of those to `spec:ITS`.
  There is **no** `spec:PROC`/`spec:CDS`/`spec:INTG`/`spec:PUB` label — those
  SPEC* projects have no component label and no vendored pin, so exclude them
  from the watch (or they'd land label-less).
- **Issue #137** — labels `ci`, `P2`, milestone `v3.4.0`. Contract = the 3
  steps (Jira poll, amendment cross-check, one issue per change); exit criteria:
  workflow in `.github/workflows/`, green on schedule + dispatch; dedup'd
  correctly-labelled issues proven by a real/dry-run cycle; watermark survives
  runs.
- **CLAUDE.md issue workflow** — one issue per work item; issue body = contract
  + exit-criteria checklist; PRs declare `Closes #N`; milestones = releases;
  exactly one `spec-impact:*` added at triage (by a human, not the workflow).

---

## 4. GitHub Actions design specifics

Sources: GitHub Actions docs — `on.schedule` (POSIX cron, UTC, min interval
5 min, may be delayed under load); `permissions` key; `workflow_dispatch`;
`GITHUB_TOKEN` auto-provisioned per run.

- **Triggers:**
  ```yaml
  on:
    schedule:
      - cron: "17 6 * * 1,4"   # Mon & Thu 06:17 UTC (twice weekly; off-the-hour to dodge load spikes)
    workflow_dispatch:
      inputs:
        dry_run: { type: boolean, default: false }
        window_days: { type: string, default: "14" }
  ```
- **Permissions (least privilege):** creating issues needs `issues: write`.
  With the dedup-by-key design (no committed state file), `contents: read`
  suffices.
  ```yaml
  permissions:
    issues: write
    contents: read
  ```
  `GITHUB_TOKEN` with `issues: write` can create/search issues via `gh issue
  create` / `gh issue list --search` / `gh api`. No PAT needed.
- **Scheduled workflows run only on the default branch**, and GitHub disables
  them after 60 days of repo inactivity — a non-issue for an active repo, worth
  a comment.

### Watermark / dedup — evaluation and recommendation

- **(a) Dedup purely via issue search (one issue per Jira key) — RECOMMENDED.**
  For each completed Jira key, `gh issue list --state all --search
  "\"SPECRM-129\" in:title" --json number` (or `gh api search/issues`); create
  only if zero matches. The open+closed issue set IS the watermark — it survives
  across runs by construction, needs no write-back, no race with concurrent
  merges, and self-heals (a missed run is caught by the next run's window).
  Satisfies "watermark survives across runs" with **zero extra state**. Matches
  the issue body's own suggestion. Minimal perms (`issues: write` only).
  Guard against title-search false-negatives by putting the key verbatim in the
  title and quoting the search.
- **(b) Committed state file** — needs `contents: write` + a bot commit each
  run; adds commit churn to `develop`, a possible push race, and a second
  failure mode. Rejected: more moving parts for no benefit over (a).
- **(c) Repository variables** (`gh variable set`) — the default `GITHUB_TOKEN`
  **cannot** write repo variables (needs admin/PAT); also opaque vs. the visible
  issue list. Rejected.

Choose **(a)**. Use a comfortably wide JQL window (`-14d`) so correctness never
depends on run timing; dedup does the deduplication.

For the amendment-record half, dedup is the *same* Jira-key namespace: a row's
`SPEC*-NNN` key is the dedup id, so an amendment-detected change and a
Jira-detected change for the same ticket collapse to one issue (search by key
first, regardless of which source found it). Keys with no Jira project we watch
(pure editorial rows without a ticket) get a synthetic id like
`amendment:specifications-RM:docs/common@<short-sha>`.

---

## 5. Failure honesty (fail loud, never green-with-zero)

The run MUST go RED on any of: Jira HTTP != 200 (esp. 410 = API moved again,
429 = rate-limited, 5xx), unexpected JSON shape (missing `issues`/`isLast`, or
`fields.resolution` absent when expected), GitHub API errors, or a repo/path
404 on an amendment file (repo renamed/moved). Concretely:
- `curl --fail --show-error` (non-2xx → non-zero exit) or explicit status-code
  check; `set -euo pipefail` in any bash step; no `|| true`, no
  `continue-on-error`.
- Validate the response shape before use (`jq -e '.issues'`, `jq -e 'has("isLast")'`);
  a schema mismatch is a hard error, not "zero results".
- **Distinguish "0 new completions" (legitimate, exit 0) from "query/parse
  failed" (exit 1).** Only a *successful* poll that genuinely matched nothing is
  green-with-no-issues; any transport/shape failure is red.
- Do NOT retry-until-silent-success. A 429 with `Retry-After` may get ONE bounded
  retry, then fail red.
- A red scheduled run surfaces in the Actions tab; optionally have failure open/
  update a single tracking issue (still `issues: write`) so a silent watcher
  can't rot unnoticed.

---

## 6. "Already implemented?" tracking — YES, use GitHub, one place

The owner's instinct is correct: the GitHub **issue lifecycle IS the
implementation-state tracker** — no second system, no state file. Because dedup
searches `--state all`, the per-Jira-key issue's own state encodes exactly where
each spec change stands:

- **No issue for key K** → not yet detected. The next poll creates it.
- **OPEN `spec-update` issue for K** → detected, awaiting triage/implementation.
  Triage adds one `spec-impact:*`; `spec-impact:none`/`docs-only` issues close
  immediately as "no work"; `spec-impact:behaviour` stays open as the work item.
- **CLOSED issue for K** → done. Closed either by the triager (no impact) or by a
  PR declaring `Closes #<n>` after re-vendor + regenerate + implement. Since the
  watcher searches `--state all` and skips any existing match, **a completed
  (closed) spec change is never re-opened** — closed = "already implemented",
  tracked automatically.

This is fully consistent with the repo's in-flight TRACKER migration (WORKLIST →
GitHub Issues): the open issue board is the tracker, milestones = releases, PRs
auto-close with `Closes #N`. The spec-watcher just becomes another *producer* of
issues on that same board. "Everything in one place" = the Issues tab: what's
pending (open), what's triaged (labels), what's shipped (closed + linked PR),
which release (milestone / `fixVersions` in the body). The only durable state the
watcher itself needs — "have I already filed this key?" — is answered by that
board, so there is nothing extra to persist.

Two small robustness notes so the board stays a reliable ledger:
- Put the Jira key **verbatim and unique** in the title so the `--state all`
  search is exact (quote it: `"SPECRM-129" in:title`).
- A spec ticket that is *re-opened* upstream (status → Reopened) after we closed
  its issue is an edge case: the watcher will find a closed issue and skip it. If
  that matters, additionally check the closed issue's completion date vs the new
  `resolutiondate` and comment on / re-open it. Low priority — flag as a `// TODO`
  follow-up, not v1.

---

## Recommended design (concrete)

**File:** `.github/workflows/spec-update-watcher.yml`
**Trigger:** `schedule: cron "17 6 * * 1,4"` (Mon+Thu 06:17 UTC) + `workflow_dispatch`
(inputs `dry_run`, `window_days`). **Permissions:** `issues: write`,
`contents: read`. Runner `ubuntu-latest`, `set -euo pipefail`, `--fail` on curl.

**Step A — Jira poll** (`GET /rest/api/3/search/jql`, anonymous, token-paginated):
```
jql = project in (SPECRM,SPECBASE,SPECAM,SPECLANG,SPECQUERY,SPECITS,SPECTERM,SPECSM,SPECCNF,SPECPR)
      AND status in (Resolved, Closed)
      AND resolutiondate >= -{window_days}d
      ORDER BY resolutiondate DESC
fields = summary,status,resolution,resolutiondate,fixVersions,components,issuetype
```
Loop on `nextPageToken` until `isLast`. Each issue → candidate {key, summary,
resolutiondate, fixVersions[].name, components}.

**Step B — Amendment cross-check** (per repo/path in the list in §2): fetch
upstream `master` raw file, extract `SPEC[A-Z]+-\d+` keys, diff against the
vendored copy's keys (or gate on commit-date > vendored-pin date via the commits
API). New keys → candidates tagged with their component.

**Step C — Component→label map** (Jira project prefix → `spec:*`):
SPECRM→spec:RM, SPECBASE→spec:BASE, SPECAM→spec:AM, SPECLANG→spec:LANG,
SPECQUERY→spec:QUERY, SPEC(ITS|ITS-REST|ITS-XML|ITS-JSON)→spec:ITS,
SPECTERM→spec:TERM, SPECSM→spec:SM, SPECCNF→spec:CNF. SPECPR (cross-cutting
problem reports) → derive component from `components`/`fixVersions`, else no
`spec:*` (only `spec-update`).

**Step D — Dedup + create** (design (a)): for each candidate key,
`gh issue list --state all --search "\"<KEY>\" in:title" --json number`; skip if
present. Else (or log-only if `dry_run`) `gh issue create`.

**Issue title:** `[spec-update] <KEY> — <summary> (<component> pin <ourpin>)`
e.g. `[spec-update] SPECBASE-48 — Add BMM definition of BASE (BASE pin 1.3.0)`

**Issue labels:** `spec-update` + the mapped `spec:<COMPONENT>` (component label
only; triage adds exactly one `spec-impact:*` later — the workflow never sets
`spec-impact:*`). Milestone left unset (a human triager assigns the release).

**Issue body template:**
```markdown
Upstream openEHR spec change completed — conformance-impact triage needed.

- **Jira:** <KEY> — https://openehr.atlassian.net/browse/<KEY>
- **Completed (resolved):** <resolutiondate>
- **Fix version(s):** <fixVersions[].name>   e.g. Release-1.3.0
- **Jira component(s):** <components>
- **Our vendored pin:** <component> <ver> @ <vendored-commit> (docs/VERSIONS.md)
- **Amendment record:** <repo>/<path> (row "<issue-no> … <date>")   [if source B]

### Summary
<Jira summary / amendment Details cell>

### Triage checklist
- [ ] Re-vendor the affected spec (scripts/vendor-spec-docs.sh — bump the pin + SHA)
- [ ] Regenerate if BMM/OAS/XSD changed (/regen-codegen)
- [ ] Assess behaviour impact → add exactly one `spec-impact:*` label
- [ ] Implement where behaviour changes; update docs/VERSIONS.md

_Opened automatically by .github/workflows/spec-update-watcher.yml_
```

**Failure modes:** any non-2xx from Jira/GitHub, missing `issues`/`isLast` in
the payload, a 404 on a watched amendment path, or a 410/429 → the step exits
non-zero and the run is RED. Zero *new* completions on a *successful* poll is
the only green-with-no-issues outcome. No `continue-on-error`, no `|| true`.

**Watermark:** none stored — the existing open+closed issue set, deduped by the
verbatim Jira key in the title (`--state all`), is the durable watermark; a wide
`-14d` JQL window makes a skipped run self-healing.
