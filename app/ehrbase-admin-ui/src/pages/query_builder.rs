//! The `/queries/builder` screen: the point-and-click Query Builder.
//!
//! A template-first, cascading builder over the console's AQL engine. The user
//! picks a template, walks its path catalog, and turns selectable data-value
//! leaves into typed criteria and projection columns. The whole editable state
//! is one [`BuilderQuery`] (`crate::builder::model`); the live AQL is produced
//! by [`to_aql`] on every change (never hand-assembled here), and the result
//! runs against `POST query/aql` through the shared [`run_aql`] server fn. No
//! openEHR spec governs an admin UI — our own design / product extension; the
//! wire it drives IS spec-bound (ITS-REST Query API).
//!
//! Discipline (rules §0/§1/§2/§6/§8): no new `#[server]` fn is added — the
//! screen reuses [`list_templates`], [`fetch_template_catalog`], [`run_aql`]
//! and [`store_query`], each of which guards its own session. The view is
//! composed from `.into_any()`-erased sections and recursive `AnyView` fns
//! (the criterion tree and the path picker); refetching resources render under
//! `<Transition>`. The whole builder state lives in one `RwSignal`; a separate
//! `struct_ver` signal gates the tree/output re-renders so that typing into a
//! field updates only the live preview (which subscribes to the query), never
//! the surrounding editor — inputs keep focus.

use std::collections::HashMap;

use leptos::component;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

use crate::builder::catalog::CatalogNode;
use crate::builder::lower::{BuilderError, to_aql};
use crate::builder::model::{
    BoolOp, BuilderQuery, Criterion, CriterionKind, CriterionNode, OrderRule, QueryShape,
    SelectedColumn,
};
use crate::error::AdminUiError;
use crate::pages::ehrs::{PAGE_SIZE, ResultPage, cell_text, error_bar, table_skeleton};
use crate::pages::template_detail::fetch_template_catalog;
use crate::pages::templates::list_templates;
use crate::queries_api::{run_aql, store_query};

/// The shared, all-`Copy` signal bundle the builder's recursive views thread
/// through instead of a long argument list. `struct_ver` is bumped only on
/// structural edits (add/remove/regroup/toggle/shape) so text-field edits
/// re-render only the live preview, not the tree.
#[derive(Clone, Copy)]
struct BuilderCtx {
    /// The single source of truth for the whole builder state.
    query: RwSignal<BuilderQuery>,
    /// Bumped on every structural mutation to force a tree/output re-render.
    struct_ver: RwSignal<u32>,
    /// Path → the catalog node that spawned a criterion/column, so the typed
    /// editor and the readable sentence can recover the RM type, label, and
    /// the constrained code/unit option lists (keyed by the leaf `aql_path`).
    leaf_meta: RwSignal<HashMap<String, CatalogNode>>,
    /// The group a new criterion is added into (defaults to the root group).
    active_path: RwSignal<Vec<usize>>,
}

impl BuilderCtx {
    /// Signal a structural change: re-render the tree/output sections.
    fn bump(self) {
        self.struct_ver.update(|v| *v = v.saturating_add(1));
    }
}

/// The Query Builder screen: template picker, path catalog, criterion tree,
/// output shape, a live AQL preview, and the run/save surface.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn QueryBuilderPage() -> impl IntoView {
    let ctx = BuilderCtx {
        query: RwSignal::new(BuilderQuery::new(String::new())),
        struct_ver: RwSignal::new(0),
        leaf_meta: RwSignal::new(HashMap::new()),
        active_path: RwSignal::new(Vec::new()),
    };
    let offset = RwSignal::new(0_u32);
    let ran = RwSignal::new(None::<String>);
    let save_name = RwSignal::new(String::new());

    let templates: Resource<Result<Vec<crate::pages::templates::TemplateRow>, AdminUiError>> =
        Resource::new(|| (), |()| async move { list_templates().await });
    let selected_template = Signal::derive(move || ctx.query.with(|q| q.template_id.clone()));
    let catalog: Resource<Result<Option<CatalogNode>, AdminUiError>> = Resource::new(
        move || selected_template.get(),
        |id| async move {
            if id.is_empty() {
                Ok(None)
            } else {
                fetch_template_catalog(id).await.map(Some)
            }
        },
    );
    let results: Resource<Result<Option<ResultPage>, AdminUiError>> = Resource::new(
        move || (ran.get(), offset.get()),
        |(ran, off)| async move {
            match ran {
                Some(aql) => run_aql(aql, "{}".to_owned(), off).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let save_action: Action<(String, String), Result<(), AdminUiError>> =
        Action::new(|input: &(String, String)| {
            let (name, aql) = input.clone();
            async move { store_query(name, aql).await }
        });

    // The live AQL / validation, recomputed from the whole state on any change.
    let preview = Memo::new(move |_| ctx.query.with(to_aql));

    let template_step = template_step_section(ctx, ran, templates);
    let picker = picker_section(ctx, catalog);
    let criteria = criteria_section(ctx);
    let output = output_section(ctx);
    let preview_run = preview_run_section(preview, ran, offset, save_name, save_action);
    let results_pane = results_section(ctx, results, offset);

    view! {
        <Title text="Query builder" />
        <div class="p-4 space-y-4">
            <h1 class="text-xl font-semibold">"Query builder"</h1>
            {template_step}
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
                <thaw::Card>
                    <thaw::CardHeader>
                        <div class="text-sm font-semibold">"Path catalog"</div>
                    </thaw::CardHeader>
                    <div class="p-3 overflow-auto max-h-[70vh]">{picker}</div>
                </thaw::Card>
                <div class="lg:col-span-2 space-y-4">
                    <thaw::Card>
                        <thaw::CardHeader>
                            <div class="text-sm font-semibold">"Criteria"</div>
                        </thaw::CardHeader>
                        <div class="p-3">{criteria}</div>
                    </thaw::Card>
                    <thaw::Card>
                        <div class="p-3">{output}</div>
                    </thaw::Card>
                </div>
            </div>
            {preview_run}
            {results_pane}
        </div>
    }
}

// ---------------------------------------------------------------------------
// Template step
// ---------------------------------------------------------------------------

/// The template picker: a `<select>` over the CDR's templates. Choosing a
/// template re-seeds the whole builder state ([`BuilderQuery::new`]) and clears
/// the catalog-derived metadata, the active target, and any prior run — a plain
/// re-set, no confirmation dialog (rules §5: controlled `<select>`).
fn template_step_section(
    ctx: BuilderCtx,
    ran: RwSignal<Option<String>>,
    templates: Resource<Result<Vec<crate::pages::templates::TemplateRow>, AdminUiError>>,
) -> AnyView {
    view! {
        <thaw::Card>
            <div class="p-3 flex flex-col gap-1 max-w-md">
                <label class="text-sm font-medium" r#for="qb-template">
                    "Template"
                </label>
                <Suspense fallback=move || {
                    view! { <span class="text-sm text-neutral-500">"Loading templates…"</span> }
                }>
                    <ErrorBoundary fallback=error_bar>
                        {move || Suspend::new(async move {
                            let rows = templates.await?;
                            Ok::<_, AdminUiError>(template_select(ctx, ran, rows))
                        })}
                    </ErrorBoundary>
                </Suspense>
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// The template `<select>` itself, once the list has loaded.
fn template_select(
    ctx: BuilderCtx,
    ran: RwSignal<Option<String>>,
    rows: Vec<crate::pages::templates::TemplateRow>,
) -> AnyView {
    let options = rows
        .into_iter()
        .map(|row| {
            let id = row.template_id.clone();
            view! { <option value=id.clone()>{row.template_id}</option> }
        })
        .collect::<Vec<_>>();
    view! {
        <select
            id="qb-template"
            class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-3 py-1.5 text-sm"
            prop:value=move || ctx.query.with(|q| q.template_id.clone())
            on:change:target=move |ev| {
                let id = ev.target().value();
                ctx.query.set(BuilderQuery::new(id));
                ctx.leaf_meta.set(HashMap::new());
                ctx.active_path.set(Vec::new());
                ran.set(None);
                ctx.bump();
            }
        >
            <option value="">"— any template —"</option>
            {options}
        </select>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Path picker
// ---------------------------------------------------------------------------

/// The path catalog pane: the template's [`CatalogNode`] tree under a
/// `<Transition>` (rules §6), with click-to-add affordances on selectable
/// data-value leaves.
fn picker_section(
    ctx: BuilderCtx,
    catalog: Resource<Result<Option<CatalogNode>, AdminUiError>>,
) -> AnyView {
    let shape_is_dv = Signal::derive(move || ctx.query.with(|q| q.shape == QueryShape::DataValues));
    view! {
        <Transition fallback=table_skeleton>
            <ErrorBoundary fallback=error_bar>
                {move || Suspend::new(async move {
                    let root = catalog.await?;
                    Ok::<
                        _,
                        AdminUiError,
                    >(
                        match root {
                            None => {
                                view! {
                                    <p class="text-sm text-neutral-500">
                                        "Pick a template to browse its paths."
                                    </p>
                                }
                                    .into_any()
                            }
                            Some(node) => {
                                view! {
                                    <ul class="text-sm">
                                        {picker_node(&node, ctx, shape_is_dv, 0)}
                                    </ul>
                                }
                                    .into_any()
                            }
                        },
                    )
                })}
            </ErrorBoundary>
        </Transition>
    }
    .into_any()
}

/// One catalog node in the picker: non-selectable branches expand/collapse;
/// selectable data-value leaves offer "+ condition" (always) and "+ column"
/// (only when the shape is a data-value projection). Returns [`AnyView`] at
/// every level so the recursion has a finite type (rules §1).
fn picker_node(
    node: &CatalogNode,
    ctx: BuilderCtx,
    shape_is_dv: Signal<bool>,
    depth: i32,
) -> AnyView {
    let has_children = !node.children.is_empty();
    let expanded = RwSignal::new(depth < 2);
    let child_views = node
        .children
        .iter()
        .map(|child| picker_node(child, ctx, shape_is_dv, depth + 1))
        .collect::<Vec<_>>();

    let disclosure = if has_children {
        view! {
            <button
                type="button"
                class="w-4 shrink-0 text-neutral-500"
                on:click=move |_| expanded.update(|open| *open = !*open)
            >
                {move || if expanded.get() { "▾" } else { "▸" }}
            </button>
        }
        .into_any()
    } else {
        view! { <span class="inline-block w-4 shrink-0"></span> }.into_any()
    };

    let label = node.label.clone();
    let rm_type = node.rm_type.clone();
    let row = if node.selectable {
        let add_node = node.clone();
        let col_node = node.clone();
        view! {
            <div class="flex items-center gap-2 flex-wrap">
                <span>{label}</span>
                <span class="font-mono text-xs text-neutral-500">{rm_type}</span>
                <button
                    type="button"
                    class="text-xs rounded border border-blue-500 text-blue-600 px-1.5 hover:bg-blue-50 dark:hover:bg-blue-950"
                    on:click=move |_| add_criterion(ctx, &add_node)
                >
                    "+ condition"
                </button>
                <button
                    type="button"
                    class="text-xs rounded border border-emerald-500 text-emerald-600 px-1.5 hover:bg-emerald-50 dark:hover:bg-emerald-950"
                    class:hidden=move || !shape_is_dv.get()
                    on:click=move |_| add_column(ctx, &col_node)
                >
                    "+ column"
                </button>
            </div>
        }
        .into_any()
    } else {
        view! {
            <button
                type="button"
                class="flex items-center gap-2 text-left rounded px-1 hover:bg-neutral-100 dark:hover:bg-neutral-800"
                on:click=move |_| expanded.update(|open| *open = !*open)
            >
                <span>{label}</span>
                <span class="font-mono text-xs text-neutral-500">{rm_type}</span>
            </button>
        }
        .into_any()
    };

    let children_list = if has_children {
        view! {
            <ul class="ml-4" class:hidden=move || !expanded.get()>
                {child_views}
            </ul>
        }
        .into_any()
    } else {
        ().into_any()
    };

    view! {
        <li class="py-0.5">
            <div class="flex items-center gap-1">{disclosure}{row}</div>
            {children_list}
        </li>
    }
    .into_any()
}

/// Turn a selectable catalog leaf into a criterion in the active group, keying
/// its default typed constraint off the RM type and recording the node in the
/// metadata map (so the editor and sentence can recover its options/label).
fn add_criterion(ctx: BuilderCtx, node: &CatalogNode) {
    let criterion = Criterion {
        aql_path: node.aql_path.clone(),
        negated: false,
        kind: default_kind_for(&node.rm_type),
    };
    let target = ctx.active_path.get_untracked();
    ctx.query.update(|q| add_leaf_at(q, &target, criterion));
    let node = node.clone();
    ctx.leaf_meta.update(move |m| {
        m.insert(node.aql_path.clone(), node);
    });
    ctx.bump();
}

/// Add a selectable catalog leaf as a projection column (data-value shape).
/// Ignores a duplicate path so a double-click is idempotent.
fn add_column(ctx: BuilderCtx, node: &CatalogNode) {
    let path = node.aql_path.clone();
    ctx.query.update(|q| {
        if !q.columns.iter().any(|col| col.aql_path == path) {
            q.columns.push(SelectedColumn {
                aql_path: path,
                alias: String::new(),
            });
        }
    });
    ctx.bump();
}

// ---------------------------------------------------------------------------
// Criteria tree
// ---------------------------------------------------------------------------

/// The criterion-tree pane: re-rendered on every structural change
/// (`struct_ver`), reading the query untracked so that typing into a leaf field
/// does not tear down the editor. The empty state invites adding a condition.
fn criteria_section(ctx: BuilderCtx) -> AnyView {
    view! {
        <div>
            {move || {
                ctx.struct_ver.get();
                let snapshot = ctx.query.get_untracked();
                match snapshot.criteria {
                    None => {
                        view! {
                            <p class="text-sm text-neutral-500">
                                "No conditions yet — add one from the path catalog on the left."
                            </p>
                        }
                            .into_any()
                    }
                    Some(root) => criterion_view(root, Vec::new(), ctx),
                }
            }}
        </div>
    }
    .into_any()
}

/// Render one node of the criterion tree as a card. A group shows its
/// connective (AND/OR), a whole-group NOT, an "add group" and a remove/clear
/// control, an "add here" target toggle, and its children (n-ary). A leaf shows
/// a live readable sentence, its typed editor, a per-leaf NOT, and remove.
/// Returns [`AnyView`] at every level (finite recursion type; rules §1).
fn criterion_view(node: CriterionNode, path: Vec<usize>, ctx: BuilderCtx) -> AnyView {
    match node {
        CriterionNode::Leaf(criterion) => leaf_card(&criterion, path, ctx),
        CriterionNode::Group {
            op,
            negated,
            children,
        } => group_card(op, negated, children, path, ctx),
    }
}

/// The card for a leaf condition.
fn leaf_card(criterion: &Criterion, path: Vec<usize>, ctx: BuilderCtx) -> AnyView {
    let kind = criterion.kind.clone();
    let meta = ctx
        .leaf_meta
        .with_untracked(|m| m.get(&criterion.aql_path).cloned());
    let sentence_path = path.clone();
    let sentence = move || {
        ctx.query
            .with(|q| sentence_of(q, &sentence_path, ctx.leaf_meta))
    };
    let editor = leaf_editor(kind, meta, path.clone(), ctx);
    let not_path = path.clone();
    let remove_path = path;
    view! {
        <div class="rounded border border-neutral-300 dark:border-neutral-700 p-2 bg-white dark:bg-neutral-900">
            <div class="flex items-start justify-between gap-2">
                <div class="text-sm">{sentence}</div>
                <div class="flex gap-1 shrink-0">
                    <button
                        type="button"
                        class=toggle_class(criterion.negated)
                        on:click=move |_| {
                            ctx.query.update(|q| toggle_negated(q, &not_path));
                            ctx.bump();
                        }
                    >
                        "NOT"
                    </button>
                    <button
                        type="button"
                        class="text-xs rounded border border-red-400 text-red-600 px-1.5 hover:bg-red-50 dark:hover:bg-red-950"
                        on:click=move |_| {
                            ctx.query.update(|q| remove_at(&mut q.criteria, &remove_path));
                            ctx.active_path.set(Vec::new());
                            ctx.bump();
                        }
                    >
                        "✕"
                    </button>
                </div>
            </div>
            <div class="mt-2">{editor}</div>
        </div>
    }
    .into_any()
}

/// The card for an n-ary boolean group.
fn group_card(
    op: BoolOp,
    negated: bool,
    children: Vec<CriterionNode>,
    path: Vec<usize>,
    ctx: BuilderCtx,
) -> AnyView {
    let is_root = path.is_empty();
    let op_word = if op == BoolOp::And { "AND" } else { "OR" };
    let child_views = children
        .into_iter()
        .enumerate()
        .map(|(i, child)| {
            let mut child_path = path.clone();
            child_path.push(i);
            criterion_view(child, child_path, ctx)
        })
        .collect::<Vec<_>>();
    let toolbar = group_toolbar(op_word, negated, is_root, path, ctx);

    view! {
        <div class="rounded border border-neutral-300 dark:border-neutral-700 p-2 bg-neutral-50 dark:bg-neutral-800/40">
            {toolbar}
            <div class="pl-3 border-l border-neutral-300 dark:border-neutral-700 space-y-2">
                {child_views}
            </div>
        </div>
    }
    .into_any()
}

/// The header toolbar of a group card: connective toggle, whole-group NOT,
/// "add group", the "add here" target toggle, and remove/clear.
fn group_toolbar(
    op_word: &'static str,
    negated: bool,
    is_root: bool,
    path: Vec<usize>,
    ctx: BuilderCtx,
) -> AnyView {
    let op_path = path.clone();
    let neg_path = path.clone();
    let add_path = path.clone();
    let target_path = path.clone();
    let remove_path = path.clone();
    let active_here = move || ctx.active_path.with(|p| *p == target_path);
    let target_set_path = path;
    view! {
        <div class="flex items-center gap-1 flex-wrap mb-2">
            <button
                type="button"
                class="text-xs font-semibold rounded border border-neutral-400 px-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-800"
                on:click=move |_| {
                    ctx.query.update(|q| toggle_op(q, &op_path));
                    ctx.bump();
                }
            >
                {op_word}
            </button>
            <button
                type="button"
                class=toggle_class(negated)
                on:click=move |_| {
                    ctx.query.update(|q| toggle_negated(q, &neg_path));
                    ctx.bump();
                }
            >
                "NOT"
            </button>
            <button
                type="button"
                class="text-xs rounded border border-neutral-400 px-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-800"
                on:click=move |_| {
                    ctx.query.update(|q| add_group_at(q, &add_path));
                    ctx.bump();
                }
            >
                "+ group"
            </button>
            <button
                type="button"
                class=move || {
                    if active_here() {
                        "text-xs rounded border border-blue-500 bg-blue-100 dark:bg-blue-900 px-1.5"
                            .to_owned()
                    } else {
                        "text-xs rounded border border-neutral-400 px-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-800"
                            .to_owned()
                    }
                }
                on:click=move |_| ctx.active_path.set(target_set_path.clone())
            >
                "add here"
            </button>
            <button
                type="button"
                class="text-xs rounded border border-red-400 text-red-600 px-1.5 hover:bg-red-50 dark:hover:bg-red-950"
                on:click=move |_| {
                    ctx.query.update(|q| remove_at(&mut q.criteria, &remove_path));
                    ctx.active_path.set(Vec::new());
                    ctx.bump();
                }
            >
                {if is_root { "clear" } else { "✕" }}
            </button>
        </div>
    }
    .into_any()
}

/// The active/negated pill style shared by the NOT toggles.
fn toggle_class(active: bool) -> &'static str {
    if active {
        "text-xs font-semibold rounded border border-amber-500 bg-amber-100 dark:bg-amber-900 text-amber-800 dark:text-amber-200 px-1.5"
    } else {
        "text-xs rounded border border-neutral-400 px-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-800"
    }
}

// ---------------------------------------------------------------------------
// Per-datatype leaf editors
// ---------------------------------------------------------------------------

/// The typed editor for a leaf, dispatched by its [`CriterionKind`]. Each
/// editor seeds local field signals from the snapshot and writes the rebuilt
/// kind back into the query on every input — untracked by the tree, so inputs
/// keep focus while the live preview updates.
fn leaf_editor(
    kind: CriterionKind,
    meta: Option<CatalogNode>,
    path: Vec<usize>,
    ctx: BuilderCtx,
) -> AnyView {
    match kind {
        CriterionKind::QuantityRange { min, max, units } => {
            quantity_editor(min, max, units, meta, path, ctx)
        }
        CriterionKind::CountRange { min, max } => count_editor(min, max, path, ctx),
        CriterionKind::ProportionNumeratorRange { min, max } => {
            proportion_editor(min, max, path, ctx)
        }
        CriterionKind::CodedIn { codes, terminology } => {
            coded_editor(codes, terminology, meta, path, ctx)
        }
        CriterionKind::OrdinalIn { values } => ordinal_editor(values, meta, path, ctx),
        CriterionKind::TextEquals { text } => text_editor(false, text, path, ctx),
        CriterionKind::TextLike { pattern } => {
            text_editor(true, strip_stars(&pattern).to_owned(), path, ctx)
        }
        CriterionKind::DateTimeRange { from, to } => datetime_editor(from, to, path, ctx),
        CriterionKind::BooleanIs { value } => boolean_editor(value, path, ctx),
        CriterionKind::Exists => view! {
            <p class="text-xs text-neutral-500 italic">
                "Presence check only for this type — matches when the node exists."
            </p>
        }
        .into_any(),
    }
}

/// A controlled labelled numeric input (`type=number`) bound to `signal`,
/// rerunning `apply` on every input so the rebuilt criterion kind reaches the
/// query. Blank text parses to `None` at the editor level.
fn number_input(
    id: String,
    label: &'static str,
    signal: RwSignal<String>,
    apply: impl Fn() + 'static,
) -> AnyView {
    view! {
        <label class="flex flex-col gap-0.5 text-xs">
            <span class="text-neutral-500">{label}</span>
            <input
                id=id
                type="number"
                step="any"
                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-28"
                prop:value=move || signal.get()
                on:input:target=move |ev| {
                    signal.set(ev.target().value());
                    apply();
                }
            />
        </label>
    }
    .into_any()
}

/// `DV_QUANTITY`: magnitude min/max plus a units selector (a `<select>` from
/// the catalog's constrained units, or free text when the node constrains none).
fn quantity_editor(
    min: Option<f64>,
    max: Option<f64>,
    units: String,
    meta: Option<CatalogNode>,
    path: Vec<usize>,
    ctx: BuilderCtx,
) -> AnyView {
    let min_s = RwSignal::new(fmt_opt_f64(min));
    let max_s = RwSignal::new(fmt_opt_f64(max));
    let units_s = RwSignal::new(units);
    let key = path_key(&path);
    let apply = move || {
        let kind = CriterionKind::QuantityRange {
            min: parse_opt_f64(&min_s.get_untracked()),
            max: parse_opt_f64(&max_s.get_untracked()),
            units: units_s.get_untracked(),
        };
        ctx.query.update(|q| set_leaf_kind(q, &path, kind));
    };
    let unit_options = meta.map(|m| m.unit_options).unwrap_or_default();
    let min_field = number_input(format!("{key}-min"), "min magnitude", min_s, apply.clone());
    let max_field = number_input(format!("{key}-max"), "max magnitude", max_s, apply.clone());
    let units_control = units_input(unit_options, units_s, apply);

    view! { <div class="flex flex-wrap items-end gap-2">{min_field}{max_field}{units_control}</div> }
    .into_any()
}

/// The units control: a constrained `<select>` when the catalog supplies unit
/// options, else a free-text input.
fn units_input(
    options: Vec<String>,
    units_s: RwSignal<String>,
    apply: impl Fn() + Clone + 'static,
) -> AnyView {
    if options.is_empty() {
        let apply = apply.clone();
        return view! {
            <label class="flex flex-col gap-0.5 text-xs">
                <span class="text-neutral-500">"units"</span>
                <input
                    type="text"
                    class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-28"
                    prop:value=move || units_s.get()
                    on:input:target=move |ev| {
                        units_s.set(ev.target().value());
                        apply();
                    }
                />
            </label>
        }
        .into_any();
    }
    let opts = options
        .into_iter()
        .map(|u| {
            let label = u.clone();
            view! { <option value=u>{label}</option> }
        })
        .collect::<Vec<_>>();
    view! {
        <label class="flex flex-col gap-0.5 text-xs">
            <span class="text-neutral-500">"units"</span>
            <select
                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm"
                prop:value=move || units_s.get()
                on:change:target=move |ev| {
                    units_s.set(ev.target().value());
                    apply();
                }
            >
                <option value="">"(any units)"</option>
                {opts}
            </select>
        </label>
    }
    .into_any()
}

/// `DV_COUNT`: integer magnitude min/max.
fn count_editor(min: Option<i64>, max: Option<i64>, path: Vec<usize>, ctx: BuilderCtx) -> AnyView {
    let min_s = RwSignal::new(fmt_opt_i64(min));
    let max_s = RwSignal::new(fmt_opt_i64(max));
    let key = path_key(&path);
    let apply = move || {
        let kind = CriterionKind::CountRange {
            min: parse_opt_i64(&min_s.get_untracked()),
            max: parse_opt_i64(&max_s.get_untracked()),
        };
        ctx.query.update(|q| set_leaf_kind(q, &path, kind));
    };
    let min_field = number_input(format!("{key}-min"), "min count", min_s, apply.clone());
    let max_field = number_input(format!("{key}-max"), "max count", max_s, apply);
    view! { <div class="flex flex-wrap items-end gap-2">{min_field}{max_field}</div> }.into_any()
}

/// `DV_PROPORTION`: numerator min/max (v1 numerator-only, per the model).
fn proportion_editor(
    min: Option<f64>,
    max: Option<f64>,
    path: Vec<usize>,
    ctx: BuilderCtx,
) -> AnyView {
    let min_s = RwSignal::new(fmt_opt_f64(min));
    let max_s = RwSignal::new(fmt_opt_f64(max));
    let key = path_key(&path);
    let apply = move || {
        let kind = CriterionKind::ProportionNumeratorRange {
            min: parse_opt_f64(&min_s.get_untracked()),
            max: parse_opt_f64(&max_s.get_untracked()),
        };
        ctx.query.update(|q| set_leaf_kind(q, &path, kind));
    };
    let min_field = number_input(format!("{key}-min"), "min numerator", min_s, apply.clone());
    let max_field = number_input(format!("{key}-max"), "max numerator", max_s, apply);
    view! { <div class="flex flex-wrap items-end gap-2">{min_field}{max_field}</div> }.into_any()
}

/// `DV_CODED_TEXT`: a checkbox multi-pick of the catalog's constrained codes
/// plus a terminology id (default `local`).
fn coded_editor(
    codes: Vec<String>,
    terminology: String,
    meta: Option<CatalogNode>,
    path: Vec<usize>,
    ctx: BuilderCtx,
) -> AnyView {
    let selected = RwSignal::new(codes);
    let term_s = RwSignal::new(terminology);
    let apply = move || {
        let kind = CriterionKind::CodedIn {
            codes: selected.get_untracked(),
            terminology: term_s.get_untracked(),
        };
        ctx.query.update(|q| set_leaf_kind(q, &path, kind));
    };
    let options = meta.map(|m| m.code_options).unwrap_or_default();
    let boxes = if options.is_empty() {
        view! {
            <p class="text-xs text-neutral-500 italic">
                "No coded options in the template for this node."
            </p>
        }
        .into_any()
    } else {
        let apply = apply.clone();
        let items = options
            .into_iter()
            .map(|opt| {
                let code = opt.code.clone();
                let checked_code = opt.code.clone();
                let toggle_code = opt.code.clone();
                let apply = apply.clone();
                let text = if opt.label == opt.code {
                    opt.code.clone()
                } else {
                    format!("{} ({})", opt.label, opt.code)
                };
                view! {
                    <label class="flex items-center gap-1 text-sm">
                        <input
                            type="checkbox"
                            prop:checked=move || selected.with(|s| s.contains(&checked_code))
                            on:change:target=move |ev| {
                                let on = ev.target().checked();
                                selected
                                    .update(|s| {
                                        if on {
                                            if !s.contains(&toggle_code) {
                                                s.push(toggle_code.clone());
                                            }
                                        } else {
                                            s.retain(|c| c != &toggle_code);
                                        }
                                    });
                                apply();
                            }
                        />
                        <span>{text}</span>
                        <span class="font-mono text-xs text-neutral-500">{code}</span>
                    </label>
                }
            })
            .collect::<Vec<_>>();
        view! { <div class="flex flex-col gap-1">{items}</div> }.into_any()
    };
    view! {
        <div class="space-y-2">
            {boxes} <label class="flex flex-col gap-0.5 text-xs">
                <span class="text-neutral-500">"terminology"</span>
                <input
                    type="text"
                    class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-40"
                    prop:value=move || term_s.get()
                    on:input:target=move |ev| {
                        term_s.set(ev.target().value());
                        apply();
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

/// `DV_ORDINAL`: a checkbox multi-pick of the catalog's ordinal steps, keyed by
/// their ordinal integer.
fn ordinal_editor(
    values: Vec<i64>,
    meta: Option<CatalogNode>,
    path: Vec<usize>,
    ctx: BuilderCtx,
) -> AnyView {
    let selected = RwSignal::new(values);
    let apply = move || {
        let kind = CriterionKind::OrdinalIn {
            values: selected.get_untracked(),
        };
        ctx.query.update(|q| set_leaf_kind(q, &path, kind));
    };
    let options = meta
        .map(|m| m.code_options)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|o| o.ordinal.map(|ord| (i64::from(ord), o.label, o.code)))
        .collect::<Vec<_>>();
    if options.is_empty() {
        return view! {
            <p class="text-xs text-neutral-500 italic">
                "No ordinal steps in the template for this node."
            </p>
        }
        .into_any();
    }
    let items = options
        .into_iter()
        .map(|(ord, label, code)| {
            let apply = apply.clone();
            let text = if label == code || label.is_empty() {
                format!("{ord}")
            } else {
                format!("{ord} · {label}")
            };
            view! {
                <label class="flex items-center gap-1 text-sm">
                    <input
                        type="checkbox"
                        prop:checked=move || selected.with(|s| s.contains(&ord))
                        on:change:target=move |ev| {
                            let on = ev.target().checked();
                            selected
                                .update(|s| {
                                    if on {
                                        if !s.contains(&ord) {
                                            s.push(ord);
                                        }
                                    } else {
                                        s.retain(|v| *v != ord);
                                    }
                                });
                            apply();
                        }
                    />
                    <span>{text}</span>
                </label>
            }
        })
        .collect::<Vec<_>>();
    view! { <div class="flex flex-col gap-1">{items}</div> }.into_any()
}

/// `DV_TEXT`: an equals/contains mode radio plus the text. `contains` lowers to
/// a `*text*` LIKE pattern; `equals` to an exact match.
fn text_editor(is_contains: bool, text: String, path: Vec<usize>, ctx: BuilderCtx) -> AnyView {
    let contains = RwSignal::new(is_contains);
    let text_s = RwSignal::new(text);
    let key = path_key(&path);
    let name = format!("text-mode-{key}");
    let apply = move || {
        let value = text_s.get_untracked();
        let kind = if contains.get_untracked() {
            CriterionKind::TextLike {
                pattern: format!("*{value}*"),
            }
        } else {
            CriterionKind::TextEquals { text: value }
        };
        ctx.query.update(|q| set_leaf_kind(q, &path, kind));
    };
    let apply_eq = apply.clone();
    let apply_ct = apply.clone();
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <label class="flex items-center gap-1 text-sm">
                <input
                    type="radio"
                    name=name.clone()
                    prop:checked=move || !contains.get()
                    on:change:target=move |_| {
                        contains.set(false);
                        apply_eq();
                    }
                />
                "equals"
            </label>
            <label class="flex items-center gap-1 text-sm">
                <input
                    type="radio"
                    name=name
                    prop:checked=move || contains.get()
                    on:change:target=move |_| {
                        contains.set(true);
                        apply_ct();
                    }
                />
                "contains"
            </label>
            <input
                type="text"
                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm flex-1 min-w-40"
                prop:value=move || text_s.get()
                on:input:target=move |ev| {
                    text_s.set(ev.target().value());
                    apply();
                }
            />
        </div>
    }
    .into_any()
}

/// `DV_DATE_TIME` / `DV_DATE` / `DV_TIME`: ISO-8601 from/to text bounds.
fn datetime_editor(from: String, to: String, path: Vec<usize>, ctx: BuilderCtx) -> AnyView {
    let from_s = RwSignal::new(from);
    let to_s = RwSignal::new(to);
    let apply = move || {
        let kind = CriterionKind::DateTimeRange {
            from: from_s.get_untracked(),
            to: to_s.get_untracked(),
        };
        ctx.query.update(|q| set_leaf_kind(q, &path, kind));
    };
    let apply_from = apply.clone();
    let apply_to = apply;
    view! {
        <div class="flex flex-wrap items-end gap-2">
            <label class="flex flex-col gap-0.5 text-xs">
                <span class="text-neutral-500">"from"</span>
                <input
                    type="text"
                    placeholder="2026-01-01T00:00:00Z"
                    class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-56"
                    prop:value=move || from_s.get()
                    on:input:target=move |ev| {
                        from_s.set(ev.target().value());
                        apply_from();
                    }
                />
            </label>
            <label class="flex flex-col gap-0.5 text-xs">
                <span class="text-neutral-500">"to"</span>
                <input
                    type="text"
                    placeholder="2026-12-31T23:59:59Z"
                    class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-56"
                    prop:value=move || to_s.get()
                    on:input:target=move |ev| {
                        to_s.set(ev.target().value());
                        apply_to();
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

/// `DV_BOOLEAN`: a true/false radio.
fn boolean_editor(value: bool, path: Vec<usize>, ctx: BuilderCtx) -> AnyView {
    let val = RwSignal::new(value);
    let key = path_key(&path);
    let name = format!("bool-{key}");
    let path_true = path.clone();
    let path_false = path;
    view! {
        <div class="flex items-center gap-3 text-sm">
            <label class="flex items-center gap-1">
                <input
                    type="radio"
                    name=name.clone()
                    prop:checked=move || val.get()
                    on:change:target=move |_| {
                        val.set(true);
                        ctx.query
                            .update(|q| set_leaf_kind(
                                q,
                                &path_true,
                                CriterionKind::BooleanIs {
                                    value: true,
                                },
                            ));
                    }
                />
                "true"
            </label>
            <label class="flex items-center gap-1">
                <input
                    type="radio"
                    name=name
                    prop:checked=move || !val.get()
                    on:change:target=move |_| {
                        val.set(false);
                        ctx.query
                            .update(|q| set_leaf_kind(
                                q,
                                &path_false,
                                CriterionKind::BooleanIs {
                                    value: false,
                                },
                            ));
                    }
                />
                "false"
            </label>
        </div>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Output: shape, columns, order, limit
// ---------------------------------------------------------------------------

/// The output section: the result shape radio and, per shape, the projection
/// columns; then the order-by rules and the limit. Re-renders on `struct_ver`.
fn output_section(ctx: BuilderCtx) -> AnyView {
    view! {
        <div>
            <div class="text-sm font-semibold mb-2">"Output"</div>
            {move || {
                ctx.struct_ver.get();
                let snapshot = ctx.query.get_untracked();
                let shape = shape_radio(ctx, snapshot.shape);
                let columns = if snapshot.shape == QueryShape::DataValues {
                    columns_editor(ctx, &snapshot.columns)
                } else {
                    ().into_any()
                };
                let order = order_editor(ctx, &snapshot.order_by);
                let limit = limit_editor(ctx, snapshot.limit);
                view! { <div class="space-y-3">{shape}{columns}{order}{limit}</div> }.into_any()
            }}
        </div>
    }
    .into_any()
}

/// The result-shape radio (compositions / data values / count).
fn shape_radio(ctx: BuilderCtx, current: QueryShape) -> AnyView {
    let option = |shape: QueryShape, label: &'static str| {
        let checked = current == shape;
        view! {
            <label class="flex items-center gap-1 text-sm">
                <input
                    type="radio"
                    name="qb-shape"
                    prop:checked=checked
                    on:change:target=move |_| {
                        ctx.query.update(|q| q.shape = shape);
                        ctx.bump();
                    }
                />
                {label}
            </label>
        }
    };
    view! {
        <div class="flex flex-wrap items-center gap-4">
            {option(QueryShape::Compositions, "Compositions")}
            {option(QueryShape::DataValues, "Data values")} {option(QueryShape::Count, "Count")}
        </div>
    }
    .into_any()
}

/// The projection-columns editor (data-value shape): one row per column with an
/// alias input and a remove button; empty invites adding from the catalog.
fn columns_editor(ctx: BuilderCtx, columns: &[SelectedColumn]) -> AnyView {
    if columns.is_empty() {
        return view! {
            <p class="text-xs text-neutral-500">
                "Add projection columns with \"+ column\" in the path catalog."
            </p>
        }
        .into_any();
    }
    let rows = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let alias = RwSignal::new(col.alias.clone());
            let path_text = col.aql_path.clone();
            view! {
                <div class="flex items-center gap-2">
                    <span
                        class="font-mono text-xs text-neutral-500 truncate max-w-xs"
                        title=col.aql_path.clone()
                    >
                        {path_text}
                    </span>
                    <input
                        id=format!("qb-col-alias-{i}")
                        type="text"
                        placeholder="alias"
                        class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-40"
                        prop:value=move || alias.get()
                        on:input:target=move |ev| {
                            let value = ev.target().value();
                            alias.set(value.clone());
                            ctx.query
                                .update(|q| {
                                    if let Some(c) = q.columns.get_mut(i) {
                                        value.clone_into(&mut c.alias);
                                    }
                                });
                        }
                    />
                    <button
                        type="button"
                        class="text-xs rounded border border-red-400 text-red-600 px-1.5 hover:bg-red-50 dark:hover:bg-red-950"
                        on:click=move |_| {
                            ctx.query
                                .update(|q| {
                                    if i < q.columns.len() {
                                        q.columns.remove(i);
                                    }
                                });
                            ctx.bump();
                        }
                    >
                        "✕"
                    </button>
                </div>
            }
        })
        .collect::<Vec<_>>();
    view! {
        <div class="space-y-1">
            <div class="text-xs font-medium text-neutral-500">"Columns"</div>
            {rows}
        </div>
    }
    .into_any()
}

/// One order-by row: a path text input, a direction select, and a remove
/// button — all addressed by the rule's position `i`.
fn order_row(ctx: BuilderCtx, i: usize, rule: &OrderRule) -> AnyView {
    let path_s = RwSignal::new(rule.aql_path.clone());
    let desc = rule.descending;
    view! {
        <div class="flex items-center gap-2">
            <input
                id=format!("qb-order-path-{i}")
                type="text"
                placeholder="context/start_time/value"
                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm flex-1 min-w-48 font-mono"
                prop:value=move || path_s.get()
                on:input:target=move |ev| {
                    let value = ev.target().value();
                    path_s.set(value.clone());
                    ctx.query
                        .update(|q| {
                            if let Some(r) = q.order_by.get_mut(i) {
                                value.clone_into(&mut r.aql_path);
                            }
                        });
                }
            />
            <select
                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm"
                prop:value=move || if desc { "desc" } else { "asc" }
                on:change:target=move |ev| {
                    let descending = ev.target().value() == "desc";
                    ctx.query
                        .update(|q| {
                            if let Some(r) = q.order_by.get_mut(i) {
                                r.descending = descending;
                            }
                        });
                    ctx.bump();
                }
            >
                <option value="asc">"asc"</option>
                <option value="desc">"desc"</option>
            </select>
            <button
                type="button"
                class="text-xs rounded border border-red-400 text-red-600 px-1.5 hover:bg-red-50 dark:hover:bg-red-950"
                on:click=move |_| {
                    ctx.query
                        .update(|q| {
                            if i < q.order_by.len() {
                                q.order_by.remove(i);
                            }
                        });
                    ctx.bump();
                }
            >
                "✕"
            </button>
        </div>
    }
    .into_any()
}

/// The order-by editor: an add button plus one row per rule (path text, a
/// direction select, remove).
fn order_editor(ctx: BuilderCtx, rules: &[OrderRule]) -> AnyView {
    let rows = rules
        .iter()
        .enumerate()
        .map(|(i, rule)| order_row(ctx, i, rule))
        .collect::<Vec<_>>();
    view! {
        <div class="space-y-1">
            <div class="flex items-center gap-2">
                <span class="text-xs font-medium text-neutral-500">"Order by"</span>
                <button
                    type="button"
                    class="text-xs rounded border border-neutral-400 px-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-800"
                    on:click=move |_| {
                        ctx.query
                            .update(|q| {
                                q.order_by
                                    .push(OrderRule {
                                        aql_path: String::new(),
                                        descending: false,
                                    });
                            });
                        ctx.bump();
                    }
                >
                    "+ add"
                </button>
            </div>
            {rows}
        </div>
    }
    .into_any()
}

/// The limit editor: a numeric fetch size; empty clears the limit.
fn limit_editor(ctx: BuilderCtx, limit: Option<u32>) -> AnyView {
    let limit_s = RwSignal::new(limit.map(|n| n.to_string()).unwrap_or_default());
    view! {
        <label class="flex items-center gap-2 text-xs">
            <span class="text-neutral-500">"Limit"</span>
            <input
                id="qb-limit"
                type="number"
                min="1"
                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-28"
                prop:value=move || limit_s.get()
                on:input:target=move |ev| {
                    let raw = ev.target().value();
                    limit_s.set(raw.clone());
                    let parsed = raw.trim().parse::<u32>().ok();
                    ctx.query.update(|q| q.limit = parsed);
                }
            />
        </label>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Preview + run + save
// ---------------------------------------------------------------------------

/// The live AQL preview and the run/save surface. The preview reads the whole
/// query through [`to_aql`]; on `Ok` it shows the AQL and enables Run/Save, on
/// `Err` it shows the [`BuilderError`] inline and disables them.
fn preview_run_section(
    preview: Memo<Result<String, BuilderError>>,
    ran: RwSignal<Option<String>>,
    offset: RwSignal<u32>,
    save_name: RwSignal<String>,
    save_action: Action<(String, String), Result<(), AdminUiError>>,
) -> AnyView {
    let disabled = Signal::derive(move || preview.with(Result::is_err));
    let save_disabled = Signal::derive(move || {
        preview.with(Result::is_err) || save_name.with(std::string::String::is_empty)
    });
    let run_click = move |_| {
        if let Ok(aql) = preview.get_untracked() {
            ran.set(Some(aql));
            offset.set(0);
        }
    };
    let save_click = move |_| {
        if let Ok(aql) = preview.get_untracked() {
            save_action.dispatch((save_name.get_untracked(), aql));
        }
    };

    view! {
        <thaw::Card>
            <thaw::CardHeader>
                <div class="text-sm font-semibold">"AQL preview"</div>
            </thaw::CardHeader>
            <div class="p-3 space-y-3">
                {move || match preview.get() {
                    Ok(aql) => {
                        let href = format!("/queries/aql?aql={}", encode_query_value(&aql));
                        view! {
                            <div class="space-y-2">
                                <pre class="overflow-auto rounded border border-neutral-300 dark:border-neutral-700 p-3 text-xs whitespace-pre-wrap">
                                    {aql}
                                </pre>
                                <A href=href attr:class="text-sm text-blue-600 hover:underline">
                                    "Open in raw editor →"
                                </A>
                            </div>
                        }
                            .into_any()
                    }
                    Err(error) => {
                        view! {
                            <thaw::MessageBar intent=thaw::MessageBarIntent::Warning>
                                <thaw::MessageBarBody>{error.to_string()}</thaw::MessageBarBody>
                            </thaw::MessageBar>
                        }
                            .into_any()
                    }
                }} <div class="flex flex-wrap items-end gap-3">
                    <thaw::Button
                        appearance=thaw::ButtonAppearance::Primary
                        disabled=disabled
                        on_click=run_click
                    >
                        "Run"
                    </thaw::Button>
                    <div class="flex items-end gap-2">
                        <label class="flex flex-col gap-0.5 text-xs">
                            <span class="text-neutral-500">"Save as (namespace::name)"</span>
                            <input
                                id="qb-save-name"
                                type="text"
                                placeholder="org::my_query"
                                class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent px-2 py-1 text-sm w-56"
                                prop:value=move || save_name.get()
                                on:input:target=move |ev| save_name.set(ev.target().value())
                            />
                        </label>
                        <thaw::Button disabled=save_disabled on_click=save_click>
                            "Save"
                        </thaw::Button>
                    </div>
                </div> {save_feedback(save_action)}
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// The save action's inline feedback.
fn save_feedback(save_action: Action<(String, String), Result<(), AdminUiError>>) -> AnyView {
    view! {
        <div class="text-sm">
            <Show when=move || save_action.pending().get()>
                <span class="text-neutral-500">"Saving…"</span>
            </Show>
            {move || match save_action.value().get() {
                Some(Ok(())) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Success>
                            <thaw::MessageBarBody>"Query saved."</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                Some(Err(error)) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>{error.to_string()}</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                None => ().into_any(),
            }}
        </div>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// The run result: a paged table (or a single big number for a count query)
/// under a `<Transition>` so paging keeps the prior page visible (rules §6).
fn results_section(
    ctx: BuilderCtx,
    results: Resource<Result<Option<ResultPage>, AdminUiError>>,
    offset: RwSignal<u32>,
) -> AnyView {
    view! {
        <Transition fallback=table_skeleton>
            <ErrorBoundary fallback=error_bar>
                {move || Suspend::new(async move {
                    let page = results.await?;
                    Ok::<
                        _,
                        AdminUiError,
                    >(
                        match page {
                            None => ().into_any(),
                            Some(page) => {
                                let is_count = ctx
                                    .query
                                    .with_untracked(|q| q.shape == QueryShape::Count);
                                let controls = paging_buttons(offset, page.rows.len());
                                let body = results_view(&page, is_count);
                                view! {
                                    <thaw::Card>
                                        <thaw::CardHeader>
                                            <div class="text-sm font-semibold">"Results"</div>
                                        </thaw::CardHeader>
                                        <div class="p-3">{body}{controls}</div>
                                    </thaw::Card>
                                }
                                    .into_any()
                            }
                        },
                    )
                })}
            </ErrorBoundary>
        </Transition>
    }
    .into_any()
}

/// Render one page of an AQL `RESULT_SET`: a big single stat for a count query,
/// the empty state, or a table. Shared with the raw AQL editor screen.
#[allow(clippy::must_use_candidate)] // consumed by the caller's view!
pub(crate) fn results_view(page: &ResultPage, is_count: bool) -> AnyView {
    if is_count {
        let n = page
            .rows
            .first()
            .and_then(|r| r.first())
            .map(cell_text)
            .unwrap_or_default();
        return view! {
            <div class="py-6 text-center">
                <div class="text-4xl font-semibold tabular-nums">{n}</div>
                <div class="text-xs text-neutral-500 mt-1">"matching rows"</div>
            </div>
        }
        .into_any();
    }
    if page.rows.is_empty() {
        return view! { <p class="text-sm text-neutral-500">"No rows."</p> }.into_any();
    }
    let headers = page
        .columns
        .iter()
        .map(|name| {
            view! { <th class="text-left font-medium text-neutral-500 py-1 pr-4">{name.clone()}</th> }
        })
        .collect::<Vec<_>>();
    let rows = page
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let key = format!(
                "{i}\u{1}{}",
                row.iter().map(cell_text).collect::<Vec<_>>().join("|")
            );
            (key, row.clone())
        })
        .collect::<Vec<_>>();
    let body = view! {
        <For each=move || rows.clone() key=|(k, _)| k.clone() let:entry>
            {result_row(&entry.1)}
        </For>
    };
    view! {
        <div class="overflow-x-auto">
            <table class="w-full text-sm border-collapse">
                <thead>
                    <tr class="border-b border-neutral-200 dark:border-neutral-700">{headers}</tr>
                </thead>
                <tbody>{body}</tbody>
            </table>
        </div>
    }
    .into_any()
}

/// One generic result row (all cells rendered as plain text).
fn result_row(row: &[serde_json::Value]) -> AnyView {
    let cells = row
        .iter()
        .map(|value| {
            let text = cell_text(value);
            view! { <td class="py-1 pr-4 align-top">{text}</td> }
        })
        .collect::<Vec<_>>();
    view! { <tr class="border-b border-neutral-100 dark:border-neutral-800">{cells}</tr> }
        .into_any()
}

/// Prev/next paging buttons wired to a local `offset` signal (page window is
/// [`PAGE_SIZE`]). Prev is disabled at the first page; next when the page is
/// not full. Offsets use saturating arithmetic (reliability rule).
#[allow(clippy::must_use_candidate)] // consumed by the caller's view!
pub(crate) fn paging_buttons(offset: RwSignal<u32>, row_count: usize) -> AnyView {
    let full = u32::try_from(row_count).unwrap_or(u32::MAX) >= PAGE_SIZE;
    let prev_disabled = Signal::derive(move || offset.get() == 0);
    let next_disabled = Signal::derive(move || !full);
    view! {
        <div class="mt-3 flex gap-2">
            <thaw::Button
                size=thaw::ButtonSize::Small
                disabled=prev_disabled
                on_click=move |_| offset.update(|o| *o = o.saturating_sub(PAGE_SIZE))
            >
                "← Previous"
            </thaw::Button>
            <thaw::Button
                size=thaw::ButtonSize::Small
                disabled=next_disabled
                on_click=move |_| offset.update(|o| *o = o.saturating_add(PAGE_SIZE))
            >
                "Next →"
            </thaw::Button>
        </div>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested)
// ---------------------------------------------------------------------------

/// Pick the initial typed constraint for a freshly-added criterion from its RM
/// type. Unknown types get a presence check (`Exists`) rather than a silent
/// drop.
fn default_kind_for(rm_type: &str) -> CriterionKind {
    match rm_type {
        "DV_QUANTITY" => CriterionKind::QuantityRange {
            min: None,
            max: None,
            units: String::new(),
        },
        "DV_COUNT" => CriterionKind::CountRange {
            min: None,
            max: None,
        },
        "DV_PROPORTION" => CriterionKind::ProportionNumeratorRange {
            min: None,
            max: None,
        },
        "DV_CODED_TEXT" => CriterionKind::CodedIn {
            codes: Vec::new(),
            terminology: "local".to_owned(),
        },
        "DV_ORDINAL" => CriterionKind::OrdinalIn { values: Vec::new() },
        "DV_TEXT" => CriterionKind::TextEquals {
            text: String::new(),
        },
        "DV_DATE_TIME" | "DV_DATE" | "DV_TIME" => CriterionKind::DateTimeRange {
            from: String::new(),
            to: String::new(),
        },
        "DV_BOOLEAN" => CriterionKind::BooleanIs { value: true },
        _ => CriterionKind::Exists,
    }
}

/// A UI-local percent-encoder for a single query-string VALUE (the `aql=` link
/// to the raw editor). It keeps the RFC 3986 unreserved set (`A-Za-z0-9-._~`)
/// and percent-encodes every other byte of the UTF-8 encoding.
///
/// NOTE: this is deliberately distinct from the owner's wire-percent-codec rule
/// (`urlencoding`, server-side): it is a tiny browser-side helper for building a
/// client route link and pulls in no server-only dependency — no openEHR spec
/// governs an admin UI's internal links (our own design / product extension).
fn encode_query_value(value: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Human-readable one-line summary of a leaf condition, used on the card.
fn criterion_sentence(criterion: &Criterion, meta: Option<&CatalogNode>) -> String {
    let label = meta
        .map(|m| m.label.clone())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| last_segment(&criterion.aql_path));
    let body = match &criterion.kind {
        CriterionKind::QuantityRange { min, max, units } => {
            let mut s = format!(
                "{label} {}",
                range_phrase("magnitude", num_opt(*min), num_opt(*max))
            );
            if !units.is_empty() {
                s.push(' ');
                s.push_str(units);
            }
            s
        }
        CriterionKind::CountRange { min, max } => {
            format!(
                "{label} {}",
                range_phrase("count", int_opt(*min), int_opt(*max))
            )
        }
        CriterionKind::ProportionNumeratorRange { min, max } => format!(
            "{label} {}",
            range_phrase("numerator", num_opt(*min), num_opt(*max))
        ),
        CriterionKind::CodedIn { codes, .. } => {
            let names = code_labels(codes, meta);
            if names.is_empty() {
                format!("{label} is a coded value")
            } else {
                format!("{label} is one of {names}")
            }
        }
        CriterionKind::OrdinalIn { values } => {
            let names = ordinal_labels(values, meta);
            if names.is_empty() {
                format!("{label} is a graded value")
            } else {
                format!("{label} is one of {names}")
            }
        }
        CriterionKind::TextEquals { text } => format!("{label} equals \"{text}\""),
        CriterionKind::TextLike { pattern } => {
            format!("{label} contains \"{}\"", strip_stars(pattern))
        }
        CriterionKind::DateTimeRange { from, to } => match (from.is_empty(), to.is_empty()) {
            (false, false) => format!("{label} from {from} to {to}"),
            (false, true) => format!("{label} on or after {from}"),
            (true, false) => format!("{label} on or before {to}"),
            (true, true) => format!("{label} (any date)"),
        },
        CriterionKind::BooleanIs { value } => format!("{label} is {value}"),
        CriterionKind::Exists => format!("{label} is present"),
    };
    if criterion.negated {
        format!("NOT ({body})")
    } else {
        body
    }
}

/// Look up the leaf at `path` in the live query and render its sentence.
fn sentence_of(
    query: &BuilderQuery,
    path: &[usize],
    leaf_meta: RwSignal<HashMap<String, CatalogNode>>,
) -> String {
    let node = query.criteria.as_ref().and_then(|root| node_at(root, path));
    match node {
        Some(CriterionNode::Leaf(criterion)) => {
            let meta = leaf_meta.with(|m| m.get(&criterion.aql_path).cloned());
            criterion_sentence(criterion, meta.as_ref())
        }
        _ => String::new(),
    }
}

/// `"magnitude between 36 and 38.5"` / `"…at least 36"` / `"…at most 38.5"`.
fn range_phrase(word: &str, min: Option<String>, max: Option<String>) -> String {
    match (min, max) {
        (Some(a), Some(b)) => format!("{word} between {a} and {b}"),
        (Some(a), None) => format!("{word} at least {a}"),
        (None, Some(b)) => format!("{word} at most {b}"),
        (None, None) => format!("{word} (any)"),
    }
}

/// Resolve code strings to their catalog labels (falling back to the code).
fn code_labels(codes: &[String], meta: Option<&CatalogNode>) -> String {
    codes
        .iter()
        .map(
            |code| match meta.and_then(|m| m.code_options.iter().find(|o| &o.code == code)) {
                Some(opt) if opt.label != opt.code && !opt.label.is_empty() => {
                    format!("{} ({})", opt.label, opt.code)
                }
                _ => code.clone(),
            },
        )
        .collect::<Vec<_>>()
        .join(", ")
}

/// Resolve ordinal ints to their catalog labels.
fn ordinal_labels(values: &[i64], meta: Option<&CatalogNode>) -> String {
    values
        .iter()
        .map(|value| {
            match meta.and_then(|m| {
                m.code_options
                    .iter()
                    .find(|o| o.ordinal.map(i64::from) == Some(*value))
            }) {
                Some(opt) if !opt.label.is_empty() && opt.label != opt.code => {
                    format!("{value} · {}", opt.label)
                }
                _ => value.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format an optional float as a display string.
fn fmt_opt_f64(value: Option<f64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

/// Format an optional integer as a display string.
fn fmt_opt_i64(value: Option<i64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

/// Parse a numeric field: blank → `None`, valid → `Some`, junk → `None`.
fn parse_opt_f64(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse::<f64>().ok()
    }
}

/// Parse an integer field: blank → `None`, valid → `Some`, junk → `None`.
fn parse_opt_i64(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse::<i64>().ok()
    }
}

/// `Option<f64>` → the pre-formatted `Option<String>` the sentence builder uses.
fn num_opt(value: Option<f64>) -> Option<String> {
    value.map(|v| v.to_string())
}

/// `Option<i64>` → pre-formatted `Option<String>`.
fn int_opt(value: Option<i64>) -> Option<String> {
    value.map(|v| v.to_string())
}

/// Strip a single leading and trailing `*` (the contains-mode LIKE wrapping).
fn strip_stars(pattern: &str) -> &str {
    let head = pattern.strip_prefix('*').unwrap_or(pattern);
    head.strip_suffix('*').unwrap_or(head)
}

/// The last `/`-separated segment of a path, for a fallback label.
fn last_segment(path: &str) -> String {
    path.trim_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_owned()
}

/// A deterministic, hydration-safe key for a tree path (radio `name`s, input
/// `id`s). Uses the child indices, not any random value.
fn path_key(path: &[usize]) -> String {
    if path.is_empty() {
        "root".to_owned()
    } else {
        path.iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("-")
    }
}

// ---------------------------------------------------------------------------
// Pure criterion-tree mutations (unit-tested)
// ---------------------------------------------------------------------------

/// Follow `path` (child indices) from a tree node to an immutable descendant.
fn node_at<'a>(root: &'a CriterionNode, path: &[usize]) -> Option<&'a CriterionNode> {
    match path.split_first() {
        None => Some(root),
        Some((idx, rest)) => match root {
            CriterionNode::Group { children, .. } => {
                children.get(*idx).and_then(|child| node_at(child, rest))
            }
            CriterionNode::Leaf(_) => None,
        },
    }
}

/// Follow `path` to a mutable descendant.
fn node_at_mut<'a>(root: &'a mut CriterionNode, path: &[usize]) -> Option<&'a mut CriterionNode> {
    match path.split_first() {
        None => Some(root),
        Some((idx, rest)) => match root {
            CriterionNode::Group { children, .. } => children
                .get_mut(*idx)
                .and_then(|child| node_at_mut(child, rest)),
            CriterionNode::Leaf(_) => None,
        },
    }
}

/// Add a leaf into the group at `path` (falling back to the root group, which
/// is created if absent).
fn add_leaf_at(query: &mut BuilderQuery, path: &[usize], criterion: Criterion) {
    if query.criteria.is_none() {
        query.criteria = Some(CriterionNode::Group {
            op: BoolOp::And,
            negated: false,
            children: Vec::new(),
        });
    }
    let pushed = query
        .criteria
        .as_mut()
        .and_then(|root| node_at_mut(root, path))
        .and_then(|target| match target {
            CriterionNode::Group { children, .. } => {
                children.push(CriterionNode::Leaf(criterion.clone()));
                Some(())
            }
            CriterionNode::Leaf(_) => None,
        });
    if pushed.is_none()
        && let Some(CriterionNode::Group { children, .. }) = query.criteria.as_mut()
    {
        children.push(CriterionNode::Leaf(criterion));
    }
}

/// Add an empty AND group into the group at `path`.
fn add_group_at(query: &mut BuilderQuery, path: &[usize]) {
    if let Some(root) = query.criteria.as_mut()
        && let Some(CriterionNode::Group { children, .. }) = node_at_mut(root, path)
    {
        children.push(CriterionNode::Group {
            op: BoolOp::And,
            negated: false,
            children: Vec::new(),
        });
    }
}

/// Remove the node at `path`. Removing the root (empty path), or emptying the
/// root group, clears the whole criteria tree.
fn remove_at(criteria: &mut Option<CriterionNode>, path: &[usize]) {
    match path.split_last() {
        None => *criteria = None,
        Some((last, parent)) => {
            if let Some(root) = criteria.as_mut()
                && let Some(CriterionNode::Group { children, .. }) = node_at_mut(root, parent)
                && *last < children.len()
            {
                children.remove(*last);
            }
            if let Some(CriterionNode::Group { children, .. }) = criteria.as_ref()
                && children.is_empty()
            {
                *criteria = None;
            }
        }
    }
}

/// Flip the NOT flag of the node (leaf or group) at `path`.
fn toggle_negated(query: &mut BuilderQuery, path: &[usize]) {
    if let Some(root) = query.criteria.as_mut() {
        match node_at_mut(root, path) {
            Some(CriterionNode::Leaf(criterion)) => criterion.negated = !criterion.negated,
            Some(CriterionNode::Group { negated, .. }) => *negated = !*negated,
            None => {}
        }
    }
}

/// Flip a group's connective (AND ↔ OR) at `path`.
fn toggle_op(query: &mut BuilderQuery, path: &[usize]) {
    if let Some(root) = query.criteria.as_mut()
        && let Some(CriterionNode::Group { op, .. }) = node_at_mut(root, path)
    {
        *op = if *op == BoolOp::And {
            BoolOp::Or
        } else {
            BoolOp::And
        };
    }
}

/// Set the typed constraint of the leaf at `path`.
fn set_leaf_kind(query: &mut BuilderQuery, path: &[usize], kind: CriterionKind) {
    if let Some(root) = query.criteria.as_mut()
        && let Some(CriterionNode::Leaf(criterion)) = node_at_mut(root, path)
    {
        criterion.kind = kind;
    }
}

#[cfg(test)]
#[allow(clippy::panic)] // test assertions on tree shapes
mod tests {
    use super::{
        add_group_at, add_leaf_at, criterion_sentence, default_kind_for, encode_query_value,
        node_at, remove_at, set_leaf_kind, strip_stars, toggle_negated, toggle_op,
    };
    use crate::builder::catalog::{CatalogNode, CodeOption};
    use crate::builder::model::{BoolOp, BuilderQuery, Criterion, CriterionKind, CriterionNode};

    fn quantity_leaf() -> Criterion {
        Criterion {
            aql_path: "content[...]/items[at0004]/value".to_owned(),
            negated: false,
            kind: CriterionKind::QuantityRange {
                min: Some(36.0),
                max: Some(38.5),
                units: "°C".to_owned(),
            },
        }
    }

    fn temp_meta() -> CatalogNode {
        CatalogNode {
            label: "Temperature".to_owned(),
            rm_type: "DV_QUANTITY".to_owned(),
            aql_path: "content[...]/items[at0004]/value".to_owned(),
            node_id: "at0004".to_owned(),
            selectable: true,
            code_options: Vec::new(),
            unit_options: vec!["°C".to_owned()],
            children: Vec::new(),
        }
    }

    #[test]
    fn encoder_keeps_unreserved_and_escapes_the_rest() {
        assert_eq!(encode_query_value("abcXYZ012-._~"), "abcXYZ012-._~");
        // Space, ampersand, equals, percent, hash, question mark, plus.
        assert_eq!(encode_query_value(" &=%#?+"), "%20%26%3D%25%23%3F%2B");
        // Multi-byte UTF-8 is encoded byte-by-byte (°C).
        assert_eq!(encode_query_value("°C"), "%C2%B0C");
    }

    #[test]
    fn quantity_sentence_reads_naturally() {
        let leaf = quantity_leaf();
        let meta = temp_meta();
        assert_eq!(
            criterion_sentence(&leaf, Some(&meta)),
            "Temperature magnitude between 36 and 38.5 °C"
        );
    }

    #[test]
    fn negated_sentence_is_wrapped() {
        let mut leaf = quantity_leaf();
        leaf.negated = true;
        let meta = temp_meta();
        assert_eq!(
            criterion_sentence(&leaf, Some(&meta)),
            "NOT (Temperature magnitude between 36 and 38.5 °C)"
        );
    }

    #[test]
    fn coded_sentence_uses_option_labels() {
        let criterion = Criterion {
            aql_path: "p/value".to_owned(),
            negated: false,
            kind: CriterionKind::CodedIn {
                codes: vec!["at0037".to_owned()],
                terminology: "local".to_owned(),
            },
        };
        let meta = CatalogNode {
            label: "Position".to_owned(),
            rm_type: "DV_CODED_TEXT".to_owned(),
            aql_path: "p/value".to_owned(),
            node_id: String::new(),
            selectable: true,
            code_options: vec![CodeOption {
                code: "at0037".to_owned(),
                label: "Sitting".to_owned(),
                ordinal: None,
            }],
            unit_options: Vec::new(),
            children: Vec::new(),
        };
        assert_eq!(
            criterion_sentence(&criterion, Some(&meta)),
            "Position is one of Sitting (at0037)"
        );
    }

    #[test]
    fn boolean_and_exists_sentences() {
        let boolean = Criterion {
            aql_path: "p/value".to_owned(),
            negated: false,
            kind: CriterionKind::BooleanIs { value: true },
        };
        assert_eq!(criterion_sentence(&boolean, None), "value is true");
        let exists = Criterion {
            aql_path: "a/b/present".to_owned(),
            negated: false,
            kind: CriterionKind::Exists,
        };
        assert_eq!(criterion_sentence(&exists, None), "present is present");
    }

    #[test]
    fn default_kind_maps_rm_types() {
        assert!(matches!(
            default_kind_for("DV_QUANTITY"),
            CriterionKind::QuantityRange { .. }
        ));
        assert!(matches!(
            default_kind_for("DV_CODED_TEXT"),
            CriterionKind::CodedIn { .. }
        ));
        assert!(matches!(
            default_kind_for("DV_DATE"),
            CriterionKind::DateTimeRange { .. }
        ));
        assert!(matches!(
            default_kind_for("SOMETHING_ELSE"),
            CriterionKind::Exists
        ));
    }

    #[test]
    fn strip_stars_unwraps_contains_pattern() {
        assert_eq!(strip_stars("*fever*"), "fever");
        assert_eq!(strip_stars("plain"), "plain");
        assert_eq!(strip_stars("*lead"), "lead");
    }

    #[test]
    fn add_leaf_creates_root_group_then_appends() {
        let mut q = BuilderQuery::new(String::new());
        add_leaf_at(&mut q, &[], quantity_leaf());
        add_leaf_at(&mut q, &[], quantity_leaf());
        match &q.criteria {
            Some(CriterionNode::Group { op, children, .. }) => {
                assert_eq!(*op, BoolOp::And);
                assert_eq!(children.len(), 2);
            }
            other => panic!("expected a two-child And group, got {other:?}"),
        }
    }

    #[test]
    fn add_leaf_into_nested_group() {
        let mut q = BuilderQuery::new(String::new());
        add_leaf_at(&mut q, &[], quantity_leaf()); // root child 0
        add_group_at(&mut q, &[]); // root child 1 = empty group
        add_leaf_at(&mut q, &[1], quantity_leaf()); // into the nested group
        let nested = node_at(q.criteria.as_ref().unwrap(), &[1]).unwrap();
        match nested {
            CriterionNode::Group { children, .. } => assert_eq!(children.len(), 1),
            CriterionNode::Leaf(_) => panic!("expected nested group"),
        }
    }

    #[test]
    fn remove_last_child_clears_tree() {
        let mut q = BuilderQuery::new(String::new());
        add_leaf_at(&mut q, &[], quantity_leaf());
        remove_at(&mut q.criteria, &[0]);
        assert!(q.criteria.is_none());
    }

    #[test]
    fn toggle_op_and_negation_flip() {
        let mut q = BuilderQuery::new(String::new());
        add_leaf_at(&mut q, &[], quantity_leaf());
        toggle_op(&mut q, &[]);
        toggle_negated(&mut q, &[0]);
        match &q.criteria {
            Some(CriterionNode::Group { op, children, .. }) => {
                assert_eq!(*op, BoolOp::Or);
                match &children[0] {
                    CriterionNode::Leaf(c) => assert!(c.negated),
                    CriterionNode::Group { .. } => panic!("expected leaf"),
                }
            }
            _ => panic!("expected group"),
        }
    }

    #[test]
    fn set_leaf_kind_replaces_constraint() {
        let mut q = BuilderQuery::new(String::new());
        add_leaf_at(&mut q, &[], quantity_leaf());
        set_leaf_kind(&mut q, &[0], CriterionKind::Exists);
        match node_at(q.criteria.as_ref().unwrap(), &[0]).unwrap() {
            CriterionNode::Leaf(c) => assert!(matches!(c.kind, CriterionKind::Exists)),
            CriterionNode::Group { .. } => panic!("expected leaf"),
        }
    }
}
