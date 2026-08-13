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

The list filters by id, concept, or archetype id as you type, and is paged by
the shared footer under the table — rows on screen out of how many,
previous/next, and 25/50/100 rows per page, all in the URL (see
[Paging](index.md#paging)). The filter narrows the rows; the footer counts what
the filter left.

The list also shows each template's root **archetype id**, and the detail
screen opens with an identity card — concept, version, default language,
languages, and the template **UID** — read from the operational template
itself.

### Deleting a template

When the CDR's admin API is enabled, each list row and the detail screen
offer **Delete**. It opens a confirmation dialog naming that template, and
nothing is sent until you confirm there. The CDR refuses a template that a
committed composition still uses — the refusal is shown with the referencing
count, so delete or migrate those compositions first, and it likewise refuses
a session without the ADMIN role, naming what is missing. If the admin API is
off, no delete button is shown at all: the console asks the server which API
groups it serves (the openEHR System API conformance manifest) before offering
any of them.

![Template delete](img/templates/templates-admin-delete.png)

> [!WARNING]
> This is a physical delete of the template registration, not a versioned
> one. The server-side switch is `admin.enabled`
> (`FERROEHR__ADMIN__ENABLED`), off by default — see
> [`[admin]`](../installation/config-auth.md#admin).

## EHR browser

Find an EHR by id (or browse the most recent), then work through its tabs:
EHR status, the status version history, the folder directory, the composition
list, and contribution lookup. Find-by-id is a plain form: it works in a
browser with JavaScript disabled, and `/ehrs?find=<ehr_id>` is a shareable
shortcut straight to an EHR.

![EHRs](img/ehrs/ehrs.png)

The EHR detail screen opens with a **summary header** read from the EHR
itself — its id, the system that created it, when it was created, and the
reference to its current EHR status. A mistyped or unknown id is reported
there, once, instead of once per tab. Below it, the tabs resolve the EHR
status (queryable / modifiable) and list the EHR's compositions with their
template, time, and version count.

![EHR detail](img/ehrs/compositions/list.png)

### Deleting an EHR

With the CDR's admin API enabled, the EHR detail screen offers **Delete EHR**
above the tabs. The confirmation dialog spells out the EHR id and what goes
with it: this is the CDR's *physical* delete — every composition,
contribution and audit record under the EHR is removed, and it cannot be
undone. On success the console returns to the EHR list; a session without the
ADMIN role is refused with a message naming what is missing. Without the
admin API the button is not rendered at all.

![EHR delete](img/ehrs/ehr-admin-delete.png)

> [!WARNING]
> Use this for test data. It is not the openEHR logical delete: nothing
> stays readable afterwards.

### Creating EHRs and committing compositions

The EHRs screen can **create an EHR** — empty, or bound to an external
subject (id + namespace) — and find an existing one **by subject id** as
well as by EHR id. The create card also takes an optional **EHR id**: leave
it blank and the CDR mints one, or supply a UUID to create that exact EHR.
A value that is not a UUID is refused before anything is sent (openEHR
strongly recommends a UUID for a client-supplied EHR id), and an id that is
already in use comes back as the CDR's own conflict — nothing is silently
overwritten.

The EHR detail screen's compositions tab includes a
**Commit composition** form: paste a canonical JSON, canonical XML, or
FLAT document (FLAT requires the template id, sent as the
`openehr-template-id` header) and the CDR's validation diagnostics are
shown verbatim on rejection.

The contributions tab opens with a **contribution activity** timeline —
writes to this EHR per day, from the same contribution data the list below
it shows.

### Directory editing

The Directory tab creates the EHR's FOLDER directory when none exists: it
commits the empty root folder, which the tree editor then fills. There is no
console-side library of folder shapes — the console stores nothing of its own,
and every folder you build is an ordinary directory version the CDR owns and
every other openEHR client can see.

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

Every document pane in the console — the composition viewer, the EHR status
tab, the directory raw mode, a contribution, a template's OPT and example
tabs — is the same viewer:

- **Highlighted** (the default): the document exactly as the CDR returned it,
  with JSON and XML syntax highlighting. The highlighter is pure Rust, like
  everything else in the console; very large documents are shown unstyled
  rather than tokenized.
- **Raw**: the same text with no highlighting.
- **Rendered**: a template-free clinical reading of a canonical openEHR JSON
  document — RM section headings with their type and archetype node id, and
  one label/value row per `ELEMENT` (quantities with their units, coded text
  with its terminology code, a null-flavoured leaf saying so). It needs no
  operational template, so a composition whose template has since been
  removed still reads normally. The tab appears only for canonical JSON;
  bookkeeping (language, territory, category, uid) is folded away — the raw
  views remain the complete record.
- **Copy** puts the raw document text on the clipboard.

**Edit as new version** opens the currently displayed canonical JSON in
an editor and commits it as the next version (`If-Match` on the latest
version — a concurrent change is reported instead of overwritten).

A version **timeline strip** walks the revision history at a glance, and
the **At time** picker resolves whichever version was current at a chosen
moment (`version_at_time`).

![Composition editor](img/ehrs/compositions/editor.png)

A **Versioned object** card below the audit reads the versioned composition
itself and the selected version directly: the versioned-object id, the owning
EHR, when the object was first created, and — for whichever version the
selector shows — its lifecycle state, its preceding version, the contribution
it was committed under, whether it carries a signature, and whether it still
carries content.

### Deleting a composition

**Delete composition** on the viewer performs the openEHR *logical* delete of
the composition's latest version, with a confirmation dialog first. The CDR
commits a deleted version on top of the current one: the composition stops
resolving as current and leaves the EHR's composition list, while every
earlier version and the audit trail stay readable. It is a normal versioned
write, so it needs no admin API — but it does need the version to still be
the latest one: if it moved on since the screen loaded, the CDR refuses the
delete and the message says to reload the history and retry.

> [!NOTE]
> This is not the same operation as **Delete EHR** above, which is the CDR's
> physical admin delete and leaves nothing readable.

The EHR detail's contributions tab lists the EHR's contributions — id,
commit time, committer, change type — with the by-uid lookup kept
underneath.

![Contributions](img/ehrs/contributions/contributions.png)

The commit form accepts canonical JSON, canonical XML, or FLAT:

![Commit composition](img/ehrs/compositions/commit.png)

### EHR status

The **Status** tab renders the EHR's current `EHR_STATUS`: the queryable and
modifiable flags as badges, the subject, the version the document is, and the
full document itself. A non-queryable EHR is called out — AQL over it returns
nothing.

![EHR status](img/ehrs/status/status.png)

Below the document, **Edit status** changes the two flags and `other_details`:

- tick or untick **is_queryable** to include the EHR in population queries
  (AQL), and **is_modifiable** to allow new content to be committed to it;
- **other_details** takes a canonical-JSON `ITEM_STRUCTURE` (for example an
  `ITEM_TREE`); leaving it blank removes the attribute. A value that is not a
  JSON object is refused before anything is sent.

Saving commits a **new EHR_STATUS version** on top of the one the screen
loaded, and every other attribute — the subject included — is sent back
exactly as the CDR served it, so nothing the form does not show can be lost.

> [!NOTE]
> The save is conditional on the loaded version. If another client committed a
> new status in the meantime, the CDR refuses the write and the console says
> so ("EHR status changed on the server") instead of overwriting the change:
> reload the tab and reapply your edit. A rejected document keeps the CDR's own
> diagnostic on screen, beside the form.

### EHR status history

The **Status history** tab is the versioned view of the same object: the
`VERSIONED_EHR_STATUS` container and the selected version's envelope facts
(lifecycle state, preceding version, contribution, whether it is signed), the
revision history newest-first, and a date-and-time lookup that resolves the
version extant at that instant. Opening any row — or a resolved instant —
shows that version's `EHR_STATUS` document exactly as it stood at that commit.

![EHR status history](img/ehrs/status/history.png)
