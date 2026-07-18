# Cabolabs EHRServer — Point-and-Click Query Builder: UX / Interaction Study

**Purpose.** A behaviour-and-interaction specification of the Cabolabs
EHRServer (Grails) visual query builder, for a ground-up reimplementation in
Rust/Leptos on our AQL 1.1 engine. This is an **idea/UX study**: it describes
what the user sees and does and what the model means, not a code port. Groovy/
GSP identifiers appear only as tiny anchors so a reader can find the source.

**Source repo.** `github.com/ppazos/cabolabs-ehrserver` (default branch
`master`), cloned at study time. All citations are repo-relative paths.

**One-paragraph orientation.** EHRServer does *not* let the user type AQL. The
builder is a template-driven, cascading-select form: pick an Operational
Template (OPT) → pick an archetype inside it → pick a data node (path) → the
server returns a datatype-specific "criteria spec" that the browser turns into
operand + value widgets → the user fills them and clicks **Add criteria** →
each criterion becomes a leaf card in a **criteria builder** area, where two
leaves can be joined with **+AND / +OR** into a nested boolean tree. There are
two query *types*: **composition** (WHERE-only, returns whole documents or a
count) and **datavalue** (SELECT/projection, returns extracted values, chartable).
The whole thing serializes to a JSON `query` object POSTed to `save`/`update`,
and can be exercised live in an inline **Test** panel before saving. Internally
each criterion maps to a `DataCriteriaDV_*` domain object that knows how to emit
an SQL `WHERE` fragment — that emission is the closest analog to what our AQL
`WHERE` generator must do.

---

## 1. The end-to-end builder flow as the user experiences it

Primary files: `grails-app/views/query/create.gsp` (2671 lines — markup +
almost all builder JS inline), `grails-app/assets/javascripts/query_create.js`
(archetype-tree select population), `grails-app/controllers/com/cabolabs/
ehrserver/query/QueryController.groovy` (AJAX endpoints).

### Screen skeleton (top to bottom), `create.gsp` body ~L2274–2665

1. **Common header block** (always visible):
   - **Name** — one text input *per configured UI language* (`name.<lang>`),
     rendered from `grailsApplication.config.languages`; the query name is an
     i18n map, not a scalar (`create.gsp` L2312–2319).
   - **Type** — a `<select>` with options `composition` / `datavalue`
     (`Query.constrainedProperties.type.inList`), blank by default (L2336;
     forced blank on ready L1943). An info tooltip explains each type.
   - **Query Group** — optional grouping bucket select (`queryGroup`, L2344).
   - **Is public?** — checkbox, **admin-only** (`ROLE_ADMIN`, L2348–2359).

2. **Nothing else is shown until a Type is chosen.** All `div.query_build`
   blocks start hidden; `show_controls(type)` reveals `#query_common` plus the
   type-specific block and the bottom toolbar (L1921–1926, L2156). Changing the
   type after data exists prompts a confirm() and wipes criteria/projections
   (L2132–2179).

3. **`#query_common` — concept & datapoint pickers** (shared by both types,
   L2366–2416). Three tall multi-line `<select size=10>` list boxes, filled by
   cascading AJAX:
   - **Templates** (`view_template_id`) — the org's OPT indexes for the current
     language (`templateIndexes`, L2381).
   - **Concept / archetype** (`view_archetype_id`) — populated on template
     change (`query_create.js` L35–108) by `getArchetypesInTemplate`, rendered
     as an **indented tree** (nested archetypes get `└ ` prefixes and
     non-breaking-space indentation, L110–135). A checkbox
     **"allow any archetype version"** sits under it (L2398) — stores the
     archetypeId with the version stripped so the criterion matches `.v1/.v2/…`.
   - **Datapoint / path** (`view_archetype_path`) — populated on archetype
     change (L2186–2198) by `get_and_render_archetype_paths` →
     `getArchetypePaths`, which returns `ArchetypeIndexItem`s (path, name,
     rmTypeName) filtered to datatype leaf nodes. Each option carries
     `data-type` (RM type) and `data-name`. A checkbox
     **"display null flavours"** toggles visibility of null-flavour options
     (L2409, L1931–1941).

4. **On datapoint selection** (L2204–2228): if the chosen option is a datavalue
   leaf, the browser calls `getCriteriaSpec(archetypeId, path, datatype)`
   (`get_criteria_specs`, L1393). The returned spec is rendered *in place* into
   `#composition_criteria_builder` as operand+value widgets (see §2). If a
   non-leaf node is chosen, a PNotify error tells the user to pick a simple-type
   node and de-selects it.

5. **Compose one criterion** — the user picks operand(s) from a dropdown and
   fills value control(s), optionally ticks **NOT** (negation), then clicks the
   **Add criteria** button (`#addCriteria`, L2434). `dom_add_criteria_2`
   (L1012) validates that a path is selected and every visible value input is
   non-empty (number inputs validated as numbers), builds a `Criteria` JS
   object, and calls `criteria_builder.add_criteria(...)`, which pushes a leaf
   into the builder model and re-renders the tree.

6. **Criteria builder area** (`#criteria_builder`, L2462–2468) — shows each
   criterion as a bordered table card (archetypeId / path / name / RM type /
   human-readable criteria string) inside a `ul/li` tree. Above it sit three
   buttons: **+AND**, **+OR** (both disabled until exactly two root cards are
   checkbox-selected) and **Remove** (enabled when one is selected). This is the
   boolean-composition surface (see §3).

7. **Type-specific tail:**
   - *composition* (`#query_composition`, L2421–2535): an **Is count?**
     checkbox (L2424) at the top; then Criteria builder; then a **Parameters**
     table — *Filter by document type* (`templateId` select), *Show UI*
     (yes/no radio, controls whether results link to a rendered UI),
     *Default format* (JSON/XML).
   - *datavalue* (`#query_datavalue`, L2539–2624): a **Data projection**
     section — **Add projection** button (`#addSelection`) appends the current
     archetype+path to a **Projections** table (`#selection`); then **Filters**
     — *Filter by document type* (`dv_templateId`), *Default format* (JSON/XML),
     *Default group* (`none` / `composition` / `path`, L2615).

8. **Bottom toolbar** (`.btn-toolbar.bottom`, L2646–2659): **Test** (toggles the
   inline test panel open, `toggle_test` L2628), **Save** (`create_button`) and
   **Update** (`update_button`, shown only in edit mode). All three route
   through `ajax_submit_test_or_save(action)` (L978).

9. **Inline Test panel** (`#query_test`, server-included `test` action,
   `test.gsp`): filter inputs (EHR id, from/to date pickers, composer uid/name)
   and, for composition, a *retrieve data yes/no* toggle; **Execute** button;
   results render either as a raw JSON/XML `<pre>` (toggle "show data"), a
   **table**, or a **Highcharts line chart** for numeric datavalue results
   (`query_test_and_execution.js`).

### What updates dynamically / what is collapsible

- Template → archetype → path → criteria-spec is a **live AJAX cascade**; each
  step clears the ones below it (`create.gsp` L2189, `query_create.js` L40–49).
- The criteria-spec widget region re-renders every time the path changes
  (`$('#composition_criteria_builder').empty()` then append, L1410/L1676).
- The +AND/+OR/Remove buttons are **state-driven** (enabled by checkbox count,
  L349–373).
- Info tooltips (`.info img` click toggles `.content`, L2123–2126) and the
  raw-data `<pre>` (`#show_data` toggle) are the only genuinely collapsible bits.
- **Defaults:** type = blank (must choose); group = `none`; format = JSON;
  show UI = no; isCount = off; isPublic = off; allow-any-version = off.
- **Validation feedback** is entirely via **PNotify** modal toasts (centered,
  bootstrap3 styling): "select a datapoint", "select/fill criteria", "number
  field empty or invalid", "empty projections/criteria" (L1020, L1067, L1062,
  L609/626). Server-side bean errors render as a `<ul class="errors">` list.

---

## 2. The per-datatype criteria widget catalog (the load-bearing section)

### 2.1 How a spec becomes widgets (the generic engine)

Every `DataCriteriaDV_*` domain class exposes a static
`criteriaSpec(archetypeId, path, returnCodes=true)` returning a **list of
alternative specs**; each spec is a map `attribute → { operand → valueKind }`
plus optional `codes`/`units`/`values`/`mediaTypes`/`criteria_constraints`
payloads. `QueryController.getCriteriaSpec` dispatches on the RM type name to
the right class (`QueryController.groovy` L808–858). The browser
(`get_criteria_specs`, `create.gsp` L1393–1689) renders it:

- **Multiple specs → a radio group.** If a datatype returns >1 spec (e.g.
  DV_CODED_TEXT offers "by code+terminology" vs "by value"; DV_IDENTIFIER offers
  "full" vs "id+type"), each spec gets a **radio button**; the selected radio's
  index is stored as `data-spec` and persisted as the criterion's `spec` int.
  Only the checked spec's conditions are read on Add.
- **Per attribute** the row shows: attribute label + hidden `attribute` field;
  an optional **NOT** checkbox (`name=negation`) — *suppressed for
  `DV_BOOLEAN` and `DV_IDENTIFIER`* via `avoid_negation_for_types` (L1392,
  L1445); an **operand `<select>`** built from the spec's operand keys (keys
  starting `_` are skipped — internal payloads, L1503); and a **value area**.
- **Value control by `valueKind`** (L1552–1666):
  - `'value'` → single input (`<input>` or, if the spec supplied `codes`/`units`/
    possible values, a `<select>`).
  - `'range'` → **two** inputs joined by `..` (min/max), or two selects for
    DV_ORDINAL where values are enumerated (L1608, L1640).
  - `'list'` → a **repeatable** input: a `[+]` (`criteria_list_add`) button
    clones the input so the user can enter N values (L1602–1606, L1911–1914).
  - `'eq_one'` → a `<select>` of the fixed allowed values (used by
    DV_PROPORTION.type, L1564).
  - `'snomed_exp'` → a `<textarea>` plus a **Validate** button that AJAX-checks
    the SNOMED expression (`validateSnomedExpression`, L1573, L1723).
- **Input typing** (L1519–1546): numeric attributes (magnitude, size,
  numerator/denominator, ordinal value, count/duration magnitude) get
  `type=number`; DV_DATE_TIME/DV_DATE `value` gets a Bootstrap **datetime/date
  picker** applied by class (`input_datetime`/`input_date`, L1696–1719) because
  native `date` inputs are unreliable cross-browser.
- **OPT-derived value lists.** When `returnCodes` is true the class consults the
  `OptManager` for the node's constraint and injects concrete allowed values —
  coded-text codes+rubrics, ordinal value/symbol pairs, quantity units, duration
  min/max seconds — so the value control becomes a **constrained dropdown**
  rather than free text.

### 2.2 The operand vocabulary (base class)

`DataCriteria.operandMap` (`DataCriteria.groovy` L~48): `eq`→`=`, `lt`→`<`,
`gt`→`>`, `neq`→`<>`, `le`→`<=`, `ge`→`>=`, `in_list`→`IN`, `contains`→`LIKE`
(ILIKE on Postgres), `between`→`BETWEEN`. Extra operands not in the map:
`contains_like` (OR-of-LIKEs over a list), `in_snomed_exp` (expand SNOMED
expression to a code `IN` list), `eq_one` (choose one of a fixed set).
**Negation** is a per-attribute boolean prefixing `NOT ` to the emitted
fragment (base class `toSQL`, `DataCriteriaDV_DATE_TIME.evaluateFunction`).

### 2.3 Per-class catalog

All from `grails-app/domain/com/cabolabs/ehrserver/query/datatypes/`.

**DV_QUANTITY** (`DataCriteriaDV_QUANTITY.groovy`)
- Attributes: `magnitude`, `units`. One spec.
- `magnitude` operands: `eq, lt, gt, neq, le, ge` (→ single number input),
  `between` (→ two number inputs). `units` operand: `eq` only.
- Quirk: `units` is rendered as a **dropdown of the OPT-constrained unit
  strings** (e.g. `mm[Hg]`) when the archetype constrains them (L1336–1349);
  otherwise a free `eq` value. Magnitude+units are **independent conditions**
  ANDed together, not a paired composite. Negation offered on both.

**DV_CODED_TEXT** (`DataCriteriaDV_CODED_TEXT.groovy`) — the richest.
- Two specs (radio choice): **[0] code + terminologyId**, **[1] value** (the
  free-text rubric).
- `code` operands: `eq` (single), `in_list` (repeatable list), and
  `in_snomed_exp` (textarea + validate) **only if SNOMED query is enabled**.
- `terminologyId` operands: `eq`, `contains`.
- `value` operands: `contains`, `eq`.
- Quirks: code values become a **dropdown of the archetype's local/openEHR
  codes with resolved rubrics** (per session language, L162–212). Special-cases:
  `/null_flavour` paths inject the 4 null-flavour codes; `/context/setting`
  injects the openEHR setting codes; otherwise the terminology dropdown is
  filled from the OPT's terminology ref or the org's `TerminologyId` list. Three
  independent negation flags (code/terminologyId/value).

**DV_TEXT** (`DataCriteriaDV_TEXT.groovy`)
- Attribute `value`, one spec. Operands: `contains_like` (→ repeatable **list**,
  emitted as OR-of-LIKEs), `eq`. Negation on `value`.
- Quirk: `contains` was replaced by `contains_like` (list) — "like %v0% OR like
  %v1% …" — so a single text criterion can match any of several substrings.

**DV_DATE_TIME** and **DV_DATE** (`DataCriteriaDV_DATE_TIME.groovy`,
`DataCriteriaDV_DATE.groovy`) — identical shape.
- Three specs (radio): **[0] value**, **[1] age_in_years**, **[2] age_in_months**
  (the latter two are *functions*, not stored attributes).
- Each: operands `eq, lt, gt, neq, le, ge` (single) + `between` (range).
- `value` uses the datetime/date picker; `between` gives two pickers.
- Quirk: **age functions** compute a date threshold at query time (now − N
  years/months) and compare against the stored value — a computed/derived
  comparison with no stored column (`evaluateFunction`, DV_DATE_TIME L630–684).
  DV_DATE reuses the same datetime storage column. Range values are sorted;
  the "high age" maps to the "low date" bound.

**DV_COUNT** (`DataCriteriaDV_COUNT.groovy`)
- Attribute `magnitude` (Long). Operands `eq, lt, gt, neq, le, ge` (single),
  `between` (range). Number input. Negation on magnitude.

**DV_DURATION** (`DataCriteriaDV_DURATION.groovy`)
- Attribute `magnitude` — an **EHRServer synthetic field = duration in
  seconds** (ISO-8601 duration is stored, but querying is by seconds for
  sane numeric comparison). Operands `eq, lt, gt, neq, le, ge, between`.
- Quirk: reads the OPT `CDuration` range and passes `criteria_constraints
  {min,max}` seconds, which the UI applies as HTML `min`/`max` on the number
  input (L1584–1592). (Note: the shipped `criteriaSpec` has a stray
  `println constraint...` before `constraint` is defined — a latent bug; the
  reimplementation should skip that and just read the range.)

**DV_PROPORTION** (`DataCriteriaDV_PROPORTION.groovy`)
- Attributes `numerator`, `denominator`, `type`. One spec.
- `numerator` & `denominator`: `eq, lt, gt, neq, le, ge, between` (number
  inputs). `type`: `eq_one` → dropdown of the allowed proportion-kind ints
  (ratio/unitary/percent/fraction/integer-fraction).
- Quirk: numerator and denominator are separate ANDed numeric conditions; there
  is **no single "ratio value" comparison** — the chart later divides
  numerator/denominator for display only.

**DV_ORDINAL** (`DataCriteriaDV_ORDINAL.groovy`)
- Three specs: **[0] value** (the ordinal integer), **[1] symbol_value**
  (text rubric), **[2] symbol_code + symbol_terminology_id** (coded).
- `value`: `eq, lt, gt, neq, le, ge, between`. `symbol_value`: `contains, eq`.
  `symbol_code`: `eq, in_list, in_snomed_exp`. `symbol_terminology_id`:
  `eq, contains`.
- Quirk: the OPT's `C_DV_ORDINAL` list fills `value` with **ordinal→rubric**
  options, so the user picks e.g. "1) mild / 2) moderate / 3) severe" by number,
  and `between` on value renders as **two selects** of those enumerated ordinals.
  (Source has a missing comma between spec[0] and spec[1] in the literal — a
  bug to avoid.)

**DV_BOOLEAN** (`DataCriteriaDV_BOOLEAN.groovy`)
- Attribute `value`, one spec. Operand `eq`; values fixed `{true, false}` →
  rendered as a true/false dropdown. **No negation** (in
  `avoid_negation_for_types`).

**DV_IDENTIFIER** (`DataCriteriaDV_IDENTIFIER.groovy`)
- Two specs: **[0] full** (`identifier, type, issuer, assigner`), **[1]
  id+type** (`identifier, type`). Every attribute: `contains` (LIKE) + `eq`.
  **No negation.**

**DV_MULTIMEDIA** (`DataCriteriaDV_MULTIMEDIA.groovy`)
- Four specs (radio): **[0] size**, **[1] mediaType**, **[2] alternateText**,
  **[3] uri**.
- `size`: `eq, lt, gt, neq, le, ge, between` (number). `mediaType`: `eq`,
  `in_list` — values are a **dropdown of the allowed media-type strings**
  (`DvMultimediaIndex.constrainedProperties.mediaType.inList`). `alternateText`:
  `contains`. `uri`: `contains`. Negation flags on all four.

**DV_PARSABLE** (`DataCriteriaDV_PARSABLE.groovy`)
- Two specs: **[0] value + formalism**, **[1] formalism**. `value`: `contains`.
  `formalism`: `eq`, `in_list` — values are a fixed dropdown
  (`text/xml, text/rtf, text/plain, text/html, application/json, ISO8601,
  HL7_GTS`). Negation on value/formalism.

**LOCATABLE_REF** (`DataCriteriaLOCATABLE_REF.groovy`)
- Attribute `locatable_ref_path`. Operands `contains, eq`. Note in source: for
  LOCATABLE_REF the actual `value` is instance-dependent and treated as a query
  **parameter**, not a fixed criterion (custom handling).

**String** (`DataCriteriaString.groovy`) — internal fallback for plain string
index nodes. Attribute `value`, operands `contains, eq`. Negation on value.

**Negation summary.** Negation is a per-attribute checkbox that prefixes `NOT`
to that attribute's fragment. It is **offered for every datatype except
DV_BOOLEAN and DV_IDENTIFIER**. There is no whole-criterion or whole-subtree
NOT — you cannot negate an AND/OR node, only leaves.

---

## 3. The AND/OR expression tree UX

Model + UI: `create.gsp` `criteria_builder` object (L150–344) and the
`DataCriteriaExpression` domain tree (`DataCriteriaExpression.groovy`).

- **Data model.** The builder holds `items[]`, a forest of trees. A **leaf** is
  `{_type:'COND', cid, archetypeId, path, rmTypeName, class, allowAny…, …attrs}`.
  A **branch** is `{_type:'AND'|'OR', left, right, cid}` — a strict **binary
  tree** (exactly two children per operator). A valid query has **exactly one
  root tree** (`is_valid()` = `items.length <= 1`, L211–214) — zero criteria is
  also valid (composition query with only filters).
- **Rendering** (`render_recursive`, L306–343): the tree is drawn as a nested
  `ul/li`. Leaves render as a bordered 5-column **table card**
  (archetypeId/path/name/type/criteria-string); the criteria-string is
  synthesized client-side from the stored operands/values
  (`criteria_to_string`, L251–305, e.g. `magnitude eq 33`, joining multiple
  attributes with ` AND `). CSS draws tree connector lines via `li::before/
  ::after` pseudo-elements (L75–118). Branch nodes render a small `<span>AND</
  span>` / `<span>OR</span>` label above their sub-`ul`.
- **Building nesting** (the core affordance): only **root-level** cards show a
  checkbox (children's are hidden, L248–249). The user **checks exactly two**
  root items; this enables **+AND** / **+OR** (L349–363). Clicking one
  (`add_criteria_item_handler`, L377–408) creates a branch node with those two
  as children, **detaches** their `<li>`s from the root, nests them under a new
  branch `<li>` (whose own checkbox is now the selectable handle), and appends
  it back at root. Repeating this against branch handles builds arbitrarily deep
  nesting — **you compose the tree bottom-up by repeatedly pairing two current
  root nodes**.
- **No drag-and-drop, no free reordering.** Nesting order is entirely
  determined by *which two you pair and in what sequence*. `left`/`right` come
  from checkbox selection order (`input:checked[0]`/`[1]`).
- **Remove** (`#criteria_builder_remove_criteria`, L2050+): with one root node
  checked, Remove deletes it; if it was a branch, its two children are **spliced
  back up to the root** (flattened one level, L215–228) rather than deleted — so
  removing an operator un-groups its operands.
- **Editing an existing tree** (`QueryTagLib.query_criteria_edit`,
  `QueryTagLib.groovy` L86+): on edit the server **emits JavaScript** that
  replays the construction — for each leaf it prints
  `criteria.add_condition(...)` + `criteria_builder.add_criteria(...)`, and for
  each operator `criteria_builder.add_complex_criteria(idL, idR, 'AND'|'OR')`,
  post-order, then a final `criteria_builder.render("#criteria_builder")`. So
  edit *reconstructs* the same in-memory forest and renders it — there is no
  separate "load tree" path.
- **Server-side linearization.** `DataCriteriaExpression.treeToExpression`
  (L~60) walks the JSON tree and flattens it into an ordered list of expression
  items each carrying `left_assoc`/`right_assoc` operator tags — the persisted
  form is a **list with association markers**, not a nested table, reconstructed
  into SQL at execution. `getInitialExpression` (L~180) is the JSON→typed
  `DataCriteria*` dispatch (one `case` per class), including value coercion
  (Doubles for quantity magnitude, date parsing for date types).
- **Limits.** Binary only (no n-ary AND of 3 in one node — you nest). Operators
  cannot be negated. There is no parenthesis UI beyond the implicit nesting;
  precedence is exactly the tree shape.

---

## 4. Query type + grouping semantics

Domain: `Query.groovy` (`type inList ['composition','datavalue']`,
`group inList ['none','composition','path']`, `isCount`, L240–243).

- **composition query** = **WHERE-only.** The criteria tree is the filter; the
  result is the set of matching **whole compositions** (documents). Extra
  narrowing parameters: `templateId` (document-type filter), date range,
  EHR id, composer — all supplied at *test/run* time, not baked into the
  definition. `showUI` decides whether each result row links to a rendered
  document view vs raw XML. **`isCount`** turns the query into a
  **count-only** query — returns the number of matching compositions, not their
  content (used by `QueryGroup.executeCount`, `QueryGroup.groovy`, which runs
  every composition query in a group and returns `uid → count` — a small
  dashboard/aggregation primitive).
- **datavalue query** = **projection/SELECT.** The user adds one or more
  **projections** (archetype+path pairs → `DataGet` rows, `DataGet.groovy`);
  the result is the **extracted data values** at those paths, optionally
  filtered. Criteria (WHERE) and projections (SELECT) are mutually the "payload"
  — a datavalue query needs ≥1 projection; a composition query needs ≥1
  criterion (validated L605–638).
- **Grouping** (datavalue only, `select[name=group]`):
  - `none` → flat list of value rows.
  - `composition` → results grouped by source composition; rendered as a
    **table** (`queryDataRenderTable`, `query_test_and_execution.js` L195), one
    column per projected path (with per-path sub-columns for the datatype's
    attributes, e.g. magnitude+units), plus links to the source doc.
    `Query.queryDataGroupComposition` builds the headers/rows server-side
    (L383+).
  - `path` → results grouped by path; numeric series become a **Highcharts line
    chart** (`queryDataRenderChart`, L47). Only DV_QUANTITY/DV_COUNT/
    DV_PROPORTION/DV_ORDINAL/DV_DURATION are chartable (numeric point builders,
    L65–97); non-numeric types are filtered out of charting.
  - The test panel picks table vs chart based on the group value at run time
    (L913–928); grouping can also be overridden per-run in the test panel
    (`if (!group) group = this.group`, `Query.groovy` L360).

---

## 5. Save / share / versioning of queries

- **Save/Update.** `ajax_submit_test_or_save('save'|'update')` (L978) →
  `save_or_update_query` (L544) assembles the `query` JS object and POSTs
  `JSON.stringify({query})` to `query/save` or `query/update` (L658–691), then
  redirects to `show/<uid>`. The controller `save`/`update`
  (`QueryController.groovy` L352, L419) rebuilds a `Query` domain object,
  stamps `author` and `organizationUid`, validates, and persists; private
  queries are auto-shared with the current org.
- **Name / i18n.** `name` is a **map lang→string** (one input per configured
  language, L2312). Edit reconstructs it via `query.add_name(lang,value)`
  (L1972). (There is no separate long "description" field in this version — the
  per-language name is the human label; `show.gsp` displays it.)
- **Public / private.** `isPublic` (admin-only checkbox). Public queries are
  visible to all orgs and carry no shares; private queries are shared with
  specific organizations. On update, going public **cleans** shares; staying
  private re-shares with the current org (`update` L432–436).
- **Sharing.** `QueryShare` domain + `share.gsp`: a multi-select of the user's
  organizations (`selectWithCurrentUserOrganizations`) grants read/exec access;
  the current org's share is protected (can't be removed or the query becomes
  inaccessible). Handled by a separate `resource/saveSharesQuery` action.
- **Query Group.** Optional `queryGroup` association (a named bucket) used for
  batch count execution and organization.
- **Versioning.** There is **no query versioning** — save/update mutate in
  place; the durable identity is the `uid`. (Export to JSON/XML exists via
  `export`/`getJSON`/`getXML` for portability, `QueryController.groovy` L873.)
- **Test-run panel.** Inline (`test.gsp`, included into `create.gsp` L2663):
  choose EHR id / date range / composer, Execute, and see raw JSON/XML, a table,
  or a chart **before saving** — the same code path the standalone execute page
  uses. Composition test adds a "retrieve data yes/no" toggle.

---

## 6. What makes the builder feel good — and what to fix

### Keep (the good UX decisions)

- **Template-first, spec-driven widgets.** The user never types a path or an
  operator name; the OPT drives archetype/path pickers and the server returns a
  datatype-appropriate operand+value palette. Value dropdowns are populated with
  the archetype's **actual constrained codes/units/ordinals with rubrics** in
  the user's language. This is the single strongest idea to carry over.
- **Progressive disclosure via cascade.** Template → archetype → path → criteria
  spec; each level unlocks the next and clears stale downstream state. Type
  selection gates the entire form so the user isn't shown irrelevant controls.
- **Per-datatype "alternative specs" as radios.** Offering "search by code" vs
  "search by rubric text", "full identifier" vs "id+type", is a clean way to
  expose the legitimate ways to constrain a complex datatype without a mode
  switch.
- **Human-readable criterion cards** with a synthesized string
  (`magnitude eq 33`) make the boolean tree legible at a glance.
- **Inline test-before-save** with three result renderings (raw / table /
  chart) — the chart for numeric series is a genuinely nice touch for clinical
  data exploration.
- **i18n-native naming** and **null-flavour / any-version** toggles show real
  openEHR-awareness.
- **Count mode + query groups** give a lightweight aggregation/dashboard story
  for free.

### Fix (weaknesses worth designing out)

- **Bottom-up checkbox pairing is unintuitive** for nesting. "Check two, click
  +AND" scales poorly past a few conditions and gives no way to reorder,
  regroup, or drag. A modern builder should offer inline **group/ungroup**,
  drag-to-nest, and an "add condition into this group" affordance.
- **Binary-only tree** forces deep nesting for "A AND B AND C". Support n-ary
  AND/OR groups.
- **No negation on groups** and no NOT for DV_BOOLEAN/DV_IDENTIFIER — asymmetric
  and surprising. Make NOT uniform and available at group level.
- **No live query preview.** The user never sees the query language (AQL) that
  will run; the only preview is executing it. Show a live, read-only AQL preview.
- **Everything re-renders from strings** (`criteria += '<...>'`) and validation
  is toast-only — fragile, no field-level inline errors, no undo. Our Leptos
  version should be reactive/state-driven with inline validation.
- **No description field, no versioning, no query history.** Add a description
  and immutable version history.
- **Latent source bugs** (DV_ORDINAL missing comma, DV_DURATION `println`
  before `constraint` is defined) show the spec-builder logic is under-tested —
  reimplement the spec catalog with types + tests, not literal maps.
- **Age-as-function** and **DV_DURATION magnitude-in-seconds** are EHRServer
  conveniences that don't exist in the RM; decide deliberately whether to keep
  them as sugar over AQL.

---

## 7. Mapping notes to AQL 1.1 (our engine)

Their model emits SQL over decomposed index tables (`DvQuantityIndex` etc.,
via `DataCriteria.toSQL`, `DataCriteria.groovy`). We emit **AQL WHERE** over RM
paths. Each widget maps to an AQL fragment shape. Below, `c` is the bound
COMPOSITION/archetype variable and the path is the archetype path to the leaf;
`$x` are bind parameters.

| Widget / operand | Their SQL shape | AQL 1.1 WHERE fragment to emit |
|---|---|---|
| DV_QUANTITY magnitude `eq/lt/gt/neq/le/ge` + units `eq` | `dqi.magnitude = ? AND dqi.units = ?` | `c/…/value/magnitude = $m AND c/…/value/units = $u` (op per operand) |
| DV_QUANTITY magnitude `between` | `dqi.magnitude BETWEEN a AND b` | `c/…/value/magnitude >= $a AND c/…/value/magnitude <= $b` (+ units eq) |
| DV_COUNT magnitude ops / between | `dci.magnitude <op> ?` | `c/…/value/magnitude <op> $n` (between → `>= $a AND <= $b`) |
| DV_DURATION magnitude(seconds) ops | `dduri.magnitude <op> ?` | Compare on `value/value` (ISO-8601). **No native seconds attribute in AQL** — either compare the ISO string with duration-aware ordering or precompute; flag: our engine needs duration comparison semantics, not a synthetic seconds column. |
| DV_DATE_TIME/DV_DATE `value` ops / between | `ddti.value <op> 'iso'` | `c/…/value/value <op> $dt` (between → two bounds). Use AQL datetime literals. |
| DV_DATE age_in_years/months | computed `now − N` compared to value | **No AQL equivalent** — AQL has no "current date" function in 1.1. Emit as an absolute date threshold computed at query-build time: `c/…/value/value >= $threshold`. Flag as build-time-computed. |
| DV_TEXT `eq` | `dti.value = ?` | `c/…/value/value = $s` |
| DV_TEXT `contains_like` (list) | `(dti.value LIKE %a% OR … )` | `c/…/value/value matches {…}`? No — AQL string containment is via `LIKE`/`matches`. Emit `c/…/value/value LIKE '%$a%' OR … ` per the value list (parenthesized OR group). |
| DV_CODED_TEXT code `eq` | `dcti.code = ?` | `c/…/value/defining_code/code_string = $code` (and `/terminology_id/value = $term`) |
| DV_CODED_TEXT code `in_list` | `dcti.code IN (…)` | `c/…/defining_code/code_string matches {$c1, $c2, …}` |
| DV_CODED_TEXT code `in_snomed_exp` | expand expression → `code IN (…)` | Expand the SNOMED expression to a code set at build time, then `code_string matches {…}`. Flag: needs a terminology-expansion step outside AQL. |
| DV_CODED_TEXT value `contains`/`eq` | `dcti.value LIKE`/`=` | `c/…/value/value LIKE '%$v%'` / `= $v` |
| DV_TEXT/ID/PARSABLE/LOCATABLE_REF `contains` | `LIKE %v%` | `<path> LIKE '%$v%'` |
| DV_BOOLEAN `eq` | `dbi.value = true/false` | `c/…/value/value = true` |
| DV_ORDINAL value ops/between | `dvol.value <op> ?` | `c/…/value/value <op> $n` |
| DV_ORDINAL symbol_code `eq/in_list` | `dvol.symbol_code …` | `c/…/value/symbol/defining_code/code_string = $c` / `matches {…}` |
| DV_PROPORTION numerator/denominator ops | two conditions ANDed | `c/…/value/numerator <op> $n AND c/…/value/denominator <op> $d` |
| DV_PROPORTION type `eq_one` | `dpi.type = ?` | `c/…/value/type = $k` (proportion-kind int) |
| DV_IDENTIFIER id/type/issuer/assigner `eq/contains` | per-attr `=`/`LIKE` | `c/…/value/id = $x` / `LIKE`; likewise `/type`, `/issuer`, `/assigner` |
| DV_MULTIMEDIA size ops/between; mediaType `eq/in_list`; alternateText/uri `contains` | index columns | `c/…/value/size <op> $s`; `c/…/value/media_type/code_string = $m` or `matches {…}`; `c/…/value/alternate_text LIKE '%$t%'`; `c/…/value/uri/value LIKE '%$u%'` |
| DV_PARSABLE value `contains`; formalism `eq/in_list` | `LIKE` / `IN` | `c/…/value/value LIKE '%$v%'`; `c/…/value/formalism = $f` / `matches {…}` |
| **NOT (per attribute)** | `NOT <frag>` | AQL: negate the comparator (`!=`, `NOT matches`) or wrap `NOT (…)`. AQL 1.1 supports `NOT` on predicate expressions. |
| **AND/OR tree** | list with assoc markers → parenthesized SQL | Nested `(… AND …) OR (…)` in the AQL WHERE, mirroring the binary tree exactly. |
| **allow-any-archetype-version** | `archetypeId LIKE 'openEHR-…%'` | AQL: use the archetype predicate without a version, or a version-wildcard CONTAINS; our engine already resolves abstract→concrete. |

**Things with no clean AQL equivalent (flag for design):**
- **`age_in_years` / `age_in_months` functions** — no "now"/date-arithmetic in
  AQL 1.1; must be resolved to an absolute date bound at build time.
- **`in_snomed_exp`** — requires a terminology-service expansion pass before AQL;
  the query definition must capture either the expression (re-expand per run) or
  the frozen code set.
- **DV_DURATION "magnitude in seconds"** — a synthetic index column, not an RM
  attribute; AQL must compare the ISO-8601 `value` with duration ordering.
- **`isCount`** — maps to AQL `SELECT COUNT(...)` — clean, but their count runs
  the full composition query and counts rows; we should emit a real count query.
- **Projections vs criteria as two query "types"** — in AQL both are one query
  (`SELECT` list + `WHERE`); our UI can unify them, but the composition-vs-
  datavalue split (whole-doc vs extracted-value results) still maps to
  "SELECT c" vs "SELECT c/…/value" AQL shapes plus different result rendering.
- **`group by composition|path`** — AQL has no GROUP BY in 1.1 for this; it is a
  **result post-processing / rendering** concern (pivot rows by composition uid
  or by path), done in our result layer, not in the AQL.

---

## Appendix — source file index (repo-relative)

- Builder UI + almost all builder JS: `grails-app/views/query/create.gsp`
- Archetype-tree select population: `grails-app/assets/javascripts/query_create.js`
- Test/exec rendering (table + Highcharts): `grails-app/assets/javascripts/query_test_and_execution.js`
- Test panel markup: `grails-app/views/query/test.gsp`
- Share UI: `grails-app/views/query/share.gsp`
- AJAX endpoints (getTemplateJson / getArchetypesInTemplate / getArchetypePaths / getCriteriaSpec / validateSnomedExpression / save / update / edit / export): `grails-app/controllers/com/cabolabs/ehrserver/query/QueryController.groovy`
- Query domain (type/group/isCount/name-i18n/execute): `grails-app/domain/com/cabolabs/ehrserver/query/Query.groovy`
- Projection: `.../query/DataGet.groovy`
- Criterion base (operandMap, toSQL, toGUI, getCriteriaMap): `.../query/DataCriteria.groovy`
- Boolean tree persistence: `.../query/DataCriteriaExpression.groovy`
- Count-group primitive: `.../query/QueryGroup.groovy`; sharing: `.../query/QueryShare.groovy`
- Per-datatype widget specs: `.../query/datatypes/DataCriteriaDV_*.groovy` (QUANTITY, CODED_TEXT, TEXT, DATE_TIME, DATE, COUNT, DURATION, PROPORTION, ORDINAL, BOOLEAN, IDENTIFIER, MULTIMEDIA, PARSABLE) + `DataCriteriaLOCATABLE_REF.groovy`, `DataCriteriaString.groovy`
- Edit re-render + read-only render taglibs: `grails-app/taglib/ehr/QueryTagLib.groovy`
