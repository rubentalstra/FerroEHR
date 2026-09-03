// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Storage spike: measure the candidate greenfield schema on
//! a real `PostgreSQL` 18 before committing to migrations.
//!
//! What it measures (the storage design's open questions):
//! 1. **Node granularity** — fine (every structure node incl. `ELEMENT` gets
//!    a row) vs coarse (`ELEMENT`/`FEEDER_AUDIT` stay inline in their parent
//!    fragment): row counts, fragment sizes, table+index size.
//! 2. **The load-bearing queries** — CONTAINS as a nested-set interval join,
//!    typed leaf extraction, magnitude ORDER BY via the `ext` function, a
//!    promoted-column predicate, and a GIN `jsonb_ops` `$.**` anchor —
//!    timings + whether plans use the intended indexes.
//! 3. **The temporal versioning model** — `PRIMARY KEY … WITHOUT OVERLAPS`
//!    (PG18) + a `upper_inf` partial index for the current version.
//!
//! Run explicitly (it is a measurement harness, not a CI gate):
//! `SPIKE_SCALE=200 cargo nextest run -p ferroehr storage_spike --run-ignored all --no-capture`

#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    let_underscore_drop,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]
#![expect(
    clippy::too_many_lines,
    reason = "an end-to-end suite drives one long lifecycle per test on purpose: \
              splitting a case would hide the order its assertions depend on"
)]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{AssertSqlSafe, PgPool, Row};
use uuid::Uuid;

// ─── candidate schema (draft — final DDL becomes migrations after the spike) ─

const DDL: &str = r#"
CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE node (
    vo_id     uuid    NOT NULL,
    num       integer NOT NULL,
    num_cap   integer NOT NULL,
    parent_num integer NOT NULL,
    citem_num integer,
    rm_type   text    NOT NULL,
    archetype text,
    name      text,
    path      text COLLATE "C" NOT NULL,
    data      jsonb   NOT NULL,
    PRIMARY KEY (vo_id, num)
);
-- containment + archetype filtering
CREATE INDEX node_type_archetype_idx ON node (rm_type, archetype);
-- deep-leaf equality anchors ($.** needs jsonb_ops, not jsonb_path_ops)
CREATE INDEX node_data_gin ON node USING gin (data jsonb_ops);

CREATE TABLE vo_version (
    vo_id       uuid      NOT NULL,
    kind        text      NOT NULL,
    ehr_id      uuid      NOT NULL,
    sys_version integer   NOT NULL,
    sys_period  tstzrange NOT NULL,
    deleted     boolean   NOT NULL DEFAULT false,
    PRIMARY KEY (vo_id, sys_period WITHOUT OVERLAPS)
);
CREATE UNIQUE INDEX vo_version_current_idx ON vo_version (vo_id) WHERE upper_inf(sys_period);
CREATE UNIQUE INDEX vo_version_num_idx ON vo_version (vo_id, sys_version);
"#;

/// Draft `ext` magnitude function: numeric `DV_ORDERED` kinds are immutable-
/// safe; temporal kinds go through our own deterministic partial-ISO parser
/// (spike: date-time only, UTC-normalized — full spec formula lands in the
/// real `ext` migration).
const MAGNITUDE_FN: &str = r"
CREATE FUNCTION openehr_magnitude(dv jsonb) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE AS $$
DECLARE
    t text := dv->>'_type';
    v text;
BEGIN
    CASE t
        WHEN 'DV_QUANTITY', 'DV_COUNT' THEN
            RETURN (dv->>'magnitude')::numeric;
        WHEN 'DV_ORDINAL', 'DV_SCALE' THEN
            RETURN (dv->>'value')::numeric;
        WHEN 'DV_PROPORTION' THEN
            IF (dv->>'denominator')::numeric = 0 THEN RETURN NULL; END IF;
            RETURN (dv->>'numerator')::numeric / (dv->>'denominator')::numeric;
        WHEN 'DV_DATE_TIME' THEN
            v := dv->>'value';
            -- deterministic: parse via to_timestamp on a normalized form;
            -- partial values are padded (spike-grade)
            IF length(v) = 4 THEN v := v || '-01-01T00:00:00Z'; END IF;
            IF length(v) = 7 THEN v := v || '-01T00:00:00Z'; END IF;
            IF length(v) = 10 THEN v := v || 'T00:00:00Z'; END IF;
            IF v !~ '[Zz+]|(-\d\d:?\d\d$)' THEN v := v || 'Z'; END IF;
            RETURN extract(epoch FROM (v::timestamptz AT TIME ZONE 'UTC'));
        ELSE
            RETURN NULL;
    END CASE;
EXCEPTION WHEN others THEN
    RETURN NULL;
END $$;
";

// ─── spike decomposer (canonical JSON → node rows; promoted to src/ after) ──

/// RM structure types (LOCATABLE descendants that occur in versioned-object
/// content, + `EVENT_CONTEXT` and `FEEDER_AUDIT`). Fine granularity = all of
/// them; coarse = without `ELEMENT/FEEDER_AUDIT` (leaf values stay inline).
const STRUCTURE_TYPES: &[&str] = &[
    "COMPOSITION",
    "EHR_STATUS",
    "FOLDER",
    "EVENT_CONTEXT",
    "SECTION",
    "GENERIC_ENTRY",
    "ADMIN_ENTRY",
    "OBSERVATION",
    "EVALUATION",
    "INSTRUCTION",
    "ACTION",
    "ACTIVITY",
    "HISTORY",
    "POINT_EVENT",
    "INTERVAL_EVENT",
    "ITEM_TREE",
    "ITEM_LIST",
    "ITEM_SINGLE",
    "ITEM_TABLE",
    "CLUSTER",
    "ELEMENT",
    "FEEDER_AUDIT",
];
const COARSE_EXCLUDED: &[&str] = &["ELEMENT", "FEEDER_AUDIT"];

#[derive(Debug)]
struct SpikeNode {
    num: i32,
    num_cap: i32,
    parent_num: i32,
    citem_num: Option<i32>,
    rm_type: String,
    archetype: Option<String>,
    name: Option<String>,
    path: String,
    data: Value,
}

struct Decomposer<'a> {
    structure: &'a BTreeSet<&'a str>,
    nodes: Vec<SpikeNode>,
}

impl<'a> Decomposer<'a> {
    /// Decompose a canonical composition into node rows: pre-order `num`,
    /// subtree `num_cap`, structure children pruned out of parent fragments,
    /// readable `path` (full attribute names + array index, `.`-terminated
    /// so byte order = tree order under `COLLATE "C"`).
    fn decompose(root: Value, structure: &'a BTreeSet<&'a str>) -> Vec<SpikeNode> {
        let mut d = Decomposer {
            structure,
            nodes: Vec::new(),
        };
        d.node(root, String::new(), -1, None);
        // num_cap: children always follow parents in the vec — reverse pass
        let mut caps: Vec<i32> = d.nodes.iter().map(|n| n.num).collect();
        for i in (0..d.nodes.len()).rev() {
            let parent = d.nodes[i].parent_num;
            if let Ok(p) = usize::try_from(parent) {
                let cap = caps[i];
                if cap > caps[p] {
                    caps[p] = cap;
                }
            }
            d.nodes[i].num_cap = caps[i];
        }
        d.nodes
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the decomposer consumes each node's JSON and its own path \
                  buffer as it walks"
    )]
    fn node(&mut self, mut json: Value, path: String, parent: i32, citem: Option<i32>) -> i32 {
        let num = i32::try_from(self.nodes.len()).expect("spike node count fits i32");
        let rm_type = json
            .get("_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let archetype = json
            .get("archetype_node_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let name = json
            .get("name")
            .and_then(|n| n.get("value"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        // the archetype ancestor for at-code scoping — an archetype ROOT is
        // decided by the RM's own node-id reading (`openehr_rm::v1_2::paths`), not a
        // prefix guess.
        let my_citem = if archetype
            .as_deref()
            .is_some_and(openehr_rm::v1_2::paths::is_archetype_root_node_id)
        {
            Some(num)
        } else {
            citem
        };
        self.nodes.push(SpikeNode {
            num,
            num_cap: num,
            parent_num: parent.max(0),
            citem_num: citem,
            rm_type,
            archetype,
            name,
            path: path.clone(),
            data: Value::Null, // filled after pruning below
        });

        // prune structure children out, recursing in document order
        if let Value::Object(map) = &mut json {
            let attributes: Vec<String> = map.keys().cloned().collect();
            for attribute in attributes {
                match map.get(&attribute) {
                    Some(child @ Value::Object(_)) if self.is_structure(child) => {
                        let owned = map.shift_remove(&attribute).unwrap();
                        let child_path = format!("{path}{attribute}.");
                        self.node(owned, child_path, num, my_citem);
                    }
                    Some(Value::Array(items))
                        if items.first().is_some_and(|c| self.is_structure(c)) =>
                    {
                        let Some(Value::Array(items)) = map.shift_remove(&attribute) else {
                            continue;
                        };
                        for (i, item) in items.into_iter().enumerate() {
                            let child_path = format!("{path}{attribute}{i}.");
                            self.node(item, child_path, num, my_citem);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.nodes[usize::try_from(num).expect("a node num is never negative")].data = json;
        num
    }

    fn is_structure(&self, v: &Value) -> bool {
        v.get("_type")
            .and_then(Value::as_str)
            .is_some_and(|t| self.structure.contains(t))
    }
}

// ─── harness ────────────────────────────────────────────────────────────────

fn corpus() -> Vec<Value> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("corpus dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "json") {
            let text = std::fs::read_to_string(&path).expect("read corpus file");
            if let Ok(v) = serde_json::from_str::<Value>(&text)
                && v.get("_type").and_then(Value::as_str) == Some("COMPOSITION")
            {
                out.push(v);
            }
        }
    }
    assert!(!out.is_empty(), "no corpus compositions found");
    out
}

async fn insert_nodes(pool: &PgPool, table: &str, nodes: &[SpikeNode], vo_id: Uuid) {
    // batched UNNEST insert; jsonb goes over as text[]::jsonb[]
    let (mut nums, mut caps, mut parents, mut citems) = (vec![], vec![], vec![], vec![]);
    let (mut types, mut archetypes, mut names, mut paths, mut datas) =
        (vec![], vec![], vec![], vec![], vec![]);
    for n in nodes {
        nums.push(n.num);
        caps.push(n.num_cap);
        parents.push(n.parent_num);
        citems.push(n.citem_num);
        types.push(n.rm_type.clone());
        archetypes.push(n.archetype.clone());
        names.push(n.name.clone());
        paths.push(n.path.clone());
        datas.push(n.data.to_string());
    }
    // batched UNNEST insert; jsonb travels as text[] and is cast server-side
    sqlx::query(AssertSqlSafe(format!(
        "INSERT INTO {table} (vo_id, num, num_cap, parent_num, citem_num, rm_type, archetype, name, path, data)
         SELECT $1,
                u.num, u.num_cap, u.parent_num, u.citem_num, u.rm_type,
                u.archetype, u.name, u.path, u.data::jsonb
         FROM unnest($2::int[], $3::int[], $4::int[], $5::int[],
                     $6::text[], $7::text[], $8::text[], $9::text[], $10::text[])
              AS u(num, num_cap, parent_num, citem_num, rm_type, archetype, name, path, data)"
    )))
    .bind(vo_id)
    .bind(&nums)
    .bind(&caps)
    .bind(&parents)
    .bind(&citems)
    .bind(&types)
    .bind(&archetypes)
    .bind(&names)
    .bind(&paths)
    .bind(&datas)
    .execute(pool)
    .await
    .expect("insert nodes");
}

async fn timed_scalar(pool: &PgPool, label: &str, sql: &'static str, report: &mut String) {
    // warm once, then time 5 runs
    const RUNS: u32 = 5;
    let _ = sqlx::query(sql).fetch_all(pool).await.expect(label);
    let start = Instant::now();
    let mut rows = 0usize;
    for _ in 0..RUNS {
        rows = sqlx::query(sql).fetch_all(pool).await.expect(label).len();
    }
    let avg_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(RUNS);
    let plan: String = sqlx::query_scalar(AssertSqlSafe(format!("EXPLAIN (FORMAT TEXT) {sql}")))
        .fetch_all(pool)
        .await
        .map(|lines: Vec<String>| lines.join(" | "))
        .unwrap_or_default();
    let plan_head = plan.split(" | ").take(2).collect::<Vec<_>>().join(" | ");
    writeln!(
        report,
        "  {label}: {avg_ms:.2} ms avg ({rows} rows) — plan: {plan_head}"
    )
    .unwrap();
}

#[tokio::test]
#[ignore = "storage spike — run explicitly with --run-ignored all"]
#[expect(
    clippy::disallowed_methods,
    reason = "SPIKE_SCALE is this ignored measurement test's own knob, not server \
              configuration — it must not enter the ferroehr::config tree"
)]
async fn storage_spike() {
    let scale: usize = std::env::var("SPIKE_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    // A pristine (non-migrated) database on the shared testkit server — the
    // spike lays down its own DDL and must not collide with the migrated
    // schema.
    let db = testkit::empty_db().await.expect("testkit empty database");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(db.url())
        .await
        .expect("pool");

    sqlx::raw_sql(DDL).execute(&pool).await.expect("ddl");
    sqlx::raw_sql(MAGNITUDE_FN)
        .execute(&pool)
        .await
        .expect("magnitude fn");
    // coarse-granularity variant of the node table (same shape, no GIN —
    // fragment-size and CONTAINS comparisons only)
    sqlx::raw_sql(
        "CREATE TABLE node_coarse (LIKE node INCLUDING ALL);
         ALTER TABLE node_coarse ALTER COLUMN path TYPE text COLLATE \"C\";",
    )
    .execute(&pool)
    .await
    .expect("coarse table");

    let mut report = String::from("\n===== STORAGE SPIKE =====\n");
    let corpus = corpus();
    writeln!(
        report,
        "corpus: {} compositions, scale ×{scale}",
        corpus.len()
    )
    .unwrap();

    let fine: BTreeSet<&str> = STRUCTURE_TYPES.iter().copied().collect();
    let coarse: BTreeSet<&str> = STRUCTURE_TYPES
        .iter()
        .copied()
        .filter(|t| !COARSE_EXCLUDED.contains(t))
        .collect();

    // ── load ────────────────────────────────────────────────────────────
    for (table, set) in [("node", &fine), ("node_coarse", &coarse)] {
        let start = Instant::now();
        let mut total_rows = 0usize;
        for _ in 0..scale {
            for comp in &corpus {
                let nodes = Decomposer::decompose(comp.clone(), set);
                total_rows += nodes.len();
                insert_nodes(&pool, table, &nodes, Uuid::now_v7()).await;
            }
        }
        let secs = start.elapsed().as_secs_f64();
        sqlx::query(AssertSqlSafe(format!("ANALYZE {table}")))
            .execute(&pool)
            .await
            .expect("analyze");
        let (table_sz, index_sz, avg_frag): (String, String, String) =
            sqlx::query_as(AssertSqlSafe(format!(
                "SELECT pg_size_pretty(pg_relation_size('{table}')),
                    pg_size_pretty(pg_indexes_size('{table}')),
                    pg_size_pretty(avg(pg_column_size(data))::bigint)
             FROM {table}"
            )))
            .fetch_one(&pool)
            .await
            .expect("sizes");
        writeln!(
            report,
            "{table}: {total_rows} rows in {secs:.1}s — table {table_sz}, indexes {index_sz}, avg fragment {avg_frag}"
        )
        .unwrap();
    }

    // ── the load-bearing queries (fine variant) ─────────────────────────
    writeln!(report, "queries (fine granularity):").unwrap();
    // Q1: CONTAINS — COMPOSITION c CONTAINS OBSERVATION o[archetype]
    timed_scalar(
        &pool,
        "Q1 CONTAINS interval join",
        "SELECT c.vo_id FROM node c
         JOIN node o ON o.vo_id = c.vo_id AND o.num BETWEEN c.num AND c.num_cap
         WHERE c.num = 0 AND c.rm_type = 'COMPOSITION'
           AND o.rm_type = 'OBSERVATION' AND o.archetype LIKE 'openEHR-EHR-OBSERVATION.%'",
        &mut report,
    )
    .await;
    // Q2: typed leaf extraction from small fragments
    timed_scalar(
        &pool,
        "Q2 leaf extraction",
        "SELECT jsonb_path_query_first(data, '$.value.magnitude') FROM node
         WHERE rm_type = 'ELEMENT' AND jsonb_path_exists(data, '$.value.magnitude')",
        &mut report,
    )
    .await;
    // Q3: ORDER BY magnitude through the ext function
    timed_scalar(
        &pool,
        "Q3 magnitude ORDER BY",
        "SELECT openehr_magnitude(data->'value') AS m FROM node
         WHERE rm_type = 'ELEMENT' AND data->'value'->>'_type' = 'DV_QUANTITY'
         ORDER BY m DESC NULLS LAST LIMIT 50",
        &mut report,
    )
    .await;
    // Q4: promoted-column predicate
    timed_scalar(
        &pool,
        "Q4 name/value predicate",
        "SELECT vo_id, num FROM node WHERE rm_type = 'OBSERVATION' AND name = 'Blood pressure'",
        &mut report,
    )
    .await;
    // Q1b: the realistic hot path — CONTAINS scoped to one versioned object
    timed_scalar(
        &pool,
        "Q1b CONTAINS scoped to one vo_id",
        "SELECT o.num FROM node c
         JOIN node o ON o.vo_id = c.vo_id AND o.num BETWEEN c.num AND c.num_cap
         WHERE c.vo_id = (SELECT vo_id FROM node WHERE rm_type = 'OBSERVATION' LIMIT 1)
           AND c.num = 0 AND o.rm_type = 'OBSERVATION'",
        &mut report,
    )
    .await;
    // Q5: GIN $.** anchor (deep equality inside a fragment); no LIMIT so the
    // planner can't hide behind an early-abort seq scan
    timed_scalar(
        &pool,
        "Q5 GIN $.** anchor (full)",
        "SELECT count(*) FROM node
         WHERE data @? 'strict $.** ? (@.code_string == \"238\")'",
        &mut report,
    )
    .await;
    // Q6: expression-index candidate — magnitude btree matching Q3 exactly
    // (same partial predicate; DESC NULLS LAST spelled in the index so the
    // ORDER BY can use it)
    sqlx::raw_sql(
        "CREATE INDEX node_magnitude_idx ON node
         ((openehr_magnitude(data->'value')) DESC NULLS LAST)
         WHERE rm_type = 'ELEMENT' AND data->'value'->>'_type' = 'DV_QUANTITY'",
    )
    .execute(&pool)
    .await
    .expect("expression index");
    sqlx::raw_sql("ANALYZE node")
        .execute(&pool)
        .await
        .expect("analyze");
    timed_scalar(
        &pool,
        "Q6 magnitude ORDER BY (with expression index)",
        "SELECT openehr_magnitude(data->'value') AS m FROM node
         WHERE rm_type = 'ELEMENT' AND data->'value'->>'_type' = 'DV_QUANTITY'
         ORDER BY openehr_magnitude(data->'value') DESC NULLS LAST LIMIT 50",
        &mut report,
    )
    .await;

    // ── temporal versioning model ────────────────────────────────────────
    let vo = Uuid::now_v7();
    let ehr = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, sys_period)
         VALUES ($1, 'COMPOSITION', $2, 1, tstzrange('2026-01-01T00:00:00Z', NULL))",
    )
    .bind(vo)
    .bind(ehr)
    .execute(&pool)
    .await
    .expect("v1 insert");
    // an overlapping period must be rejected by the temporal PK
    let overlap = sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, sys_period)
         VALUES ($1, 'COMPOSITION', $2, 2, tstzrange('2026-02-01T00:00:00Z', NULL))",
    )
    .bind(vo)
    .bind(ehr)
    .execute(&pool)
    .await;
    assert!(
        overlap.is_err(),
        "temporal PK must reject overlapping periods"
    );
    // close v1, then v2 opens — adjacent ranges are fine
    sqlx::query(
        "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), '2026-02-01T00:00:00Z')
         WHERE vo_id = $1 AND upper_inf(sys_period)",
    )
    .bind(vo)
    .execute(&pool)
    .await
    .expect("close v1");
    sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, sys_period)
         VALUES ($1, 'COMPOSITION', $2, 2, tstzrange('2026-02-01T00:00:00Z', NULL))",
    )
    .bind(vo)
    .bind(ehr)
    .execute(&pool)
    .await
    .expect("v2 insert after closing v1");
    let current: i32 = sqlx::query(
        "SELECT sys_version FROM vo_version WHERE vo_id = $1 AND upper_inf(sys_period)",
    )
    .bind(vo)
    .fetch_one(&pool)
    .await
    .expect("current lookup")
    .get(0);
    assert_eq!(current, 2);
    writeln!(
        report,
        "vo_version: WITHOUT OVERLAPS enforced ✓, current-version partial index ✓ (LATEST=v{current}, ALL_VERSIONS=unfiltered)"
    )
    .unwrap();

    // round-trip: reassemble every corpus composition from its rows, both
    // granularities
    for set in [&fine, &coarse] {
        for comp in &corpus {
            let nodes = Decomposer::decompose(comp.clone(), set);
            let reassembled = reassemble(&nodes);
            assert_eq!(&reassembled, comp, "decompose→reassemble must be lossless");
        }
    }
    writeln!(
        report,
        "codec: decompose→reassemble lossless on all {} corpus compositions × both granularities ✓",
        corpus.len()
    )
    .unwrap();

    report.push_str("===== END SPIKE =====\n");
    println!("{report}");
}

/// Spike reassembly: parents come before children; re-attach each pruned
/// child at its path (inverse of the decomposer).
fn reassemble(nodes: &[SpikeNode]) -> Value {
    let mut root = nodes[0].data.clone();
    for node in &nodes[1..] {
        // path is absolute from the root; parent paths are prefixes
        attach(&mut root, &node.path, node.data.clone());
    }
    root
}

/// Attaches `child` at `rel_path` — `content0.` or `data.events1.`, i.e.
/// attribute-plus-optional-index steps — under `target`.
fn attach(target: &mut Value, rel_path: &str, child: Value) {
    let mut current = target;
    let steps: Vec<&str> = rel_path.trim_end_matches('.').split('.').collect();
    for (i, step) in steps.iter().enumerate() {
        let is_leaf = i == steps.len() - 1;
        let (attr, idx) = split_step(step);
        let map = current.as_object_mut().expect("object on path");
        match idx {
            None if is_leaf => {
                map.insert(attr.to_owned(), child);
                return;
            }
            None => current = map.get_mut(attr).expect("ancestor attr"),
            Some(idx) => {
                let arr = map
                    .entry(attr.to_owned())
                    .or_insert_with(|| Value::Array(vec![]))
                    .as_array_mut()
                    .expect("array on path");
                if is_leaf {
                    if arr.len() <= idx {
                        arr.resize(idx + 1, Value::Null);
                    }
                    arr[idx] = child;
                    return;
                }
                current = arr.get_mut(idx).expect("ancestor idx");
            }
        }
    }
}

/// Splits one path step into its attribute name and optional array index.
fn split_step(step: &str) -> (&str, Option<usize>) {
    let digits = step.chars().rev().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return (step, None);
    }
    let (attr, num) = step.split_at(step.len() - digits);
    (attr, num.parse().ok())
}
