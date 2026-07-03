//! `openehr-codegen` — generates the openEHR spec crates from the vendored BMM
//! meta-model (ADR-004).
//!
//! Current stage: load + validate the vendored BMM schemas and report a
//! summary. The Rust emitter lands next.

use openehr_lang::bmm::{BmmPropKind, BmmSchema, BmmType};
use std::path::Path;

const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/bmm");

const SCHEMAS: &[&str] = &[
    "openehr_base_1.2.0.bmm",
    "openehr_rm_1.1.0.bmm",
    "openehr_term_3.1.0.bmm",
];

fn main() {
    let mut failures = 0;
    for file in SCHEMAS {
        match load(file) {
            Ok(s) => report(file, &s),
            Err(e) => {
                eprintln!("✗ {file}: {e}");
                failures += 1;
            }
        }
    }
    if failures > 0 {
        std::process::exit(1);
    }
}

fn load(file: &str) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    let path = Path::new(VENDOR).join(file);
    let src = std::fs::read_to_string(&path)?;
    Ok(BmmSchema::parse(&src)?)
}

fn report(file: &str, s: &BmmSchema) {
    let abstract_n = s.classes.values().filter(|c| c.is_abstract).count();
    let generic_n = s
        .classes
        .values()
        .filter(|c| !c.generic_params.is_empty())
        .count();
    println!(
        "✓ {file}: schema={} release={} classes={} (abstract={}, generic={}) includes={:?}",
        s.schema_name,
        s.rm_release,
        s.classes.len(),
        abstract_n,
        generic_n,
        s.includes,
    );

    // Spot-check DV_QUANTITY when present (proves property/type parsing on real data).
    if let Some(q) = s.classes.get("DV_QUANTITY") {
        println!(
            "    DV_QUANTITY ancestors={:?} props={}",
            q.ancestors,
            q.properties.len()
        );
        for p in &q.properties {
            let ty = match &p.kind {
                BmmPropKind::Single(t) => describe(t),
                BmmPropKind::Container {
                    container_type,
                    item,
                    ..
                } => {
                    format!("{container_type}<{}>", describe(item))
                }
            };
            let opt = if p.is_mandatory { "" } else { "?" };
            println!("      - {}{opt}: {ty}", p.name);
        }
    }
}

fn describe(t: &BmmType) -> String {
    match t {
        BmmType::Simple(s) => s.clone(),
        BmmType::Generic { root, params } => {
            let inner: Vec<String> = params.iter().map(describe).collect();
            format!("{root}<{}>", inner.join(", "))
        }
    }
}
