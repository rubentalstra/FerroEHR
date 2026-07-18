# Templates & EHR browsing

## Template Manager

Upload ADL 1.4 operational templates (the CDR's validation diagnostics
surface verbatim on rejection) and browse what is installed.

![Templates](img/templates/templates.png)

The template detail screen shows the **path catalog** — the template's tree
with each node's archetype path, RM type, and constrained value sets — plus
the raw OPT XML and a CDR-generated example composition in any supported
format.

![Template detail](img/templates/template-detail.png)

The list also shows each template's root **archetype id**, and the detail
screen opens with an identity card — concept, version, default language,
languages, and the template **UID** — read from the operational template
itself.

## EHR browser

Find an EHR by id (or browse the most recent), then work through its tabs:
EHR status, the folder directory, the composition list, and contribution
lookup.

![EHRs](img/ehrs/ehrs.png)

The EHR detail screen resolves the EHR status (queryable / modifiable) and
lists the EHR's compositions with their template, time, and version count.

![EHR detail](img/ehrs/compositions/list.png)

### Creating EHRs and committing compositions

The EHRs screen can **create an EHR** — empty, or bound to an external
subject (id + namespace) — and find an existing one **by subject id** as
well as by EHR id. The EHR detail screen's compositions tab includes a
**Commit composition** form: paste a canonical JSON, canonical XML, or
FLAT document (FLAT requires the template id, sent as the
`openehr-template-id` header) and the CDR's validation diagnostics are
shown verbatim on rejection.

### Directory editing & folder templates

The Directory tab creates the EHR's FOLDER directory when none exists —
from an empty root or a console-local **folder template** (two built-ins
ship: episodes-by-year and clinical-areas).

![Directory create](img/ehrs/directory/create.png)

Once the directory exists, the tab is a full **structured tree editor**:
add, rename, and remove sub-folders at any node, and attach or remove
`OBJECT_REF` items — a picker lists the EHR's compositions, and a manual
form covers arbitrary references. Edits accumulate locally until the sticky
save bar commits them as one new version (`If-Match` concurrency: a
concurrent change never silently overwrites — a conflict banner keeps your
unsaved edits and offers an explicit reload-or-overwrite choice). An
advanced mode still edits the canonical JSON directly.

![Directory](img/ehrs/directory/directory.png)

The toolbar adds the read-side tools: **version history** (every directory
version, read-only preview, one-click restore of an older tree), a
**`version_at_time`** time-travel lookup, a **`path=` sub-folder query**,
and the two-step **directory delete** (a logical delete — the history stays
readable, and a new directory can be created afterwards).

![Directory history](img/ehrs/directory/history.png)

## Composition viewer

Any composition renders in canonical JSON, canonical XML, FLAT, or
STRUCTURED — switch freely; the CDR converts. The version dropdown walks
the revision history, and each version's audit (committer, time, change
type) is shown alongside.

![Composition viewer](img/ehrs/compositions/viewer.png)

**Edit as new version** opens the currently displayed canonical JSON in
an editor and commits it as the next version (`If-Match` on the latest
version — a concurrent change is reported instead of overwritten).

A version **timeline strip** walks the revision history at a glance, and
the **At time** picker resolves whichever version was current at a chosen
moment (`version_at_time`).

![Composition editor](img/ehrs/compositions/editor.png)

The EHR detail's contributions tab lists the EHR's contributions — id,
commit time, committer, change type — with the by-uid lookup kept
underneath.

![Contributions](img/ehrs/contributions/contributions.png)

The commit form accepts canonical JSON, canonical XML, or FLAT:

![Commit composition](img/ehrs/compositions/commit.png)

The EHR status tab renders the full `EHR_STATUS` document:

![EHR status](img/ehrs/status/status.png)
