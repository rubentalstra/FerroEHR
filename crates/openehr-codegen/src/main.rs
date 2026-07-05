#![allow(clippy::format_push_string, clippy::too_many_lines)]

//! `openehr-codegen` — generates the openEHR spec crates from the vendored BMM
//! meta-model (ADR-004).
//!
//! Usage:
//!   `openehr-codegen check`          — load + validate the vendored BMM schemas.
//!   `openehr-codegen emit [OUTDIR]`  — emit Rust into OUTDIR (default:
//!                                       `target/codegen-preview`).

mod bmm;
mod emit;
mod emit_opt;
mod emit_rest;
mod emit_xml;
mod naming;
mod oas;
mod xsd;

use bmm::BmmSchema;
use emit::{External, Model};
use std::path::{Path, PathBuf};

const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/bmm");
/// The `openehr-its` crate root (holds the vendored XSDs/OAS and receives the
/// generated XML/REST code).
#[allow(dead_code)] // used by the emit-xml/emit-rest writers (landing incrementally)
const ITS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../openehr-its");
/// v1 (namespace `.../v1`) RM-instance XSD bundle dir — the Stage-1 parity target.
const XSD_V1_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../openehr-its/schemas/xml/its-xml-1.0.2-nsv1/ALL"
);
/// v2 (namespace `.../v2`) XSD root (per-component release folders). Reserved for
/// a future v2-specific trait if the wire shape ever diverges from v1 (ADR-005).
#[allow(dead_code)]
const XSD_V2_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../openehr-its/schemas/xml/its-xml-2.0.0-nsv2"
);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map_or("check", String::as_str);
    let result = match cmd {
        "check" => cmd_check(),
        "emit" => cmd_emit(args.get(1).map(PathBuf::from)),
        "check-xsd" => cmd_check_xsd(),
        "emit-xml" => cmd_emit_xml(),
        "emit-rest" => cmd_emit_rest(),
        "emit-opt" => cmd_emit_opt(),
        other => {
            eprintln!(
                "unknown command {other:?}; use `check`, `emit [OUTDIR]`, `check-xsd`, `emit-xml`, `emit-rest`, or `emit-opt`"
            );
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

// Paths are relative to `VENDOR` and mirror the upstream ITS-BMM layout
// (`components/<COMPONENT>/json/…`) — the full meta-model is vendored verbatim
// (json + odin + yaml, all released versions); the JSON forms below are the
// codegen input for our pinned versions (see `docs/VERSIONS.md`).
const BASE_BMM: &str = "components/BASE/json/openehr_base_1.3.0.bmm.json";
const RM_BMM: &str = "components/RM/json/openehr_rm_1.2.0.bmm.json";
const TERM_BMM: &str = "components/TERM/json/openehr_term_3.1.0.bmm.json";
const AM14_BMM: &str = "components/AM/json/openehr_am_1.4.0.bmm.json";
const AM24_BMM: &str = "components/AM/json/openehr_am_2.4.0.bmm.json";
const LANG_BMM: &str = "components/LANG/json/openehr_lang_1.1.0.bmm.json";
/// LANG's model spans two vendored files: the primary one above (persisted BMM
/// with `EXPR_*` and `STATEMENT_SET`/`ASSERTION`, which AM's rules/slots
/// reference) and this BMM-3 file (the full `BMM_*` object model with the
/// `EL_*` expression language, which AM's persisted-archetype rules reference).
/// Both are merged into the `openehr-lang` crate.
const LANG_BMM3: &str = "components/LANG/json/openehr_lang_1.1.0-bmm3.bmm.json";

fn load(file: &str) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(Path::new(VENDOR).join(file))?;
    Ok(BmmSchema::parse_json(&src)?)
}

fn cmd_check() -> Result<(), Box<dyn std::error::Error>> {
    for file in [BASE_BMM, RM_BMM, TERM_BMM] {
        let s = load(file)?;
        let abstract_n = s.classes.values().filter(|c| c.is_abstract).count();
        let generic_n = s
            .classes
            .values()
            .filter(|c| !c.generic_params.is_empty())
            .count();
        println!(
            "✓ {file}: schema={} release={} classes={} (abstract={abstract_n}, generic={generic_n})",
            s.schema_name,
            s.rm_release,
            s.classes.len(),
        );
    }
    Ok(())
}

/// Emit the ITS-REST contract (DTOs, param structs, server trait, route table)
/// for each API group into `openehr-its/src/rest/generated/` (ADR-005).
fn cmd_emit_rest() -> Result<(), Box<dyn std::error::Error>> {
    // Groups with operations (overview is an index, system has none).
    const GROUPS: &[&str] = &["admin", "definition", "demographic", "ehr", "query"];
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let base_model = Model::merged(&[&base]);
    let rm_model = Model::merged(&[&base, &rm]);
    // OAS $ref names are PascalCase (`EhrStatus`) — the same as the emitted Rust
    // type names — so map each crate's emittable spec names through `type_name`.
    let names = emit_rest::RmNames {
        base: emit::emittable_specs(&base_model, &base)
            .iter()
            .map(|s| naming::type_name(s))
            .collect(),
        rm: emit::emittable_specs(&rm_model, &rm)
            .iter()
            .map(|s| naming::type_name(s))
            .collect(),
    };

    let oas_dir = Path::new(ITS_ROOT).join("vendor/rest-oas");
    let gen_dir = Path::new(ITS_ROOT).join("src/rest/generated");
    std::fs::create_dir_all(&gen_dir)?;

    let mut written = Vec::new();
    for group in GROUPS {
        let oas = oas::Oas::parse_file(&oas_dir.join(format!("{group}-codegen.openapi.yaml")))?;
        let body = emit_rest::emit_group(&oas, group, &names);
        let path = gen_dir.join(format!("{group}.rs"));
        std::fs::write(&path, &body)?;
        written.push(path);
    }
    let mod_rs = {
        let mut s = String::from(
            "// @generated by openehr-codegen (emit-rest, ADR-005) — DO NOT EDIT.\n\
             //! ITS-REST contract, one module per API group.\n\n",
        );
        for g in GROUPS {
            s.push_str(&format!("pub mod {g};\n"));
        }
        s
    };
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&mod_path, mod_rs)?;
    written.push(mod_path);

    rustfmt(&written)?;
    println!("emitted {} files into {}", written.len(), gen_dir.display());
    Ok(())
}

/// Emit canonical-XML `ToXml`/`FromXml` impls for the RM/BASE spec types into
/// `openehr-its/src/xml/generated/` (ADR-005). Generates both wire lineages: v1
/// (`.../v1`, parity target) and v2 (`.../v2`, latest).
fn cmd_emit_xml() -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let base_model = Model::merged(&[&base]);
    let rm_model = Model::merged(&[&base, &rm]);

    // The RM-instance wire shape (element names, order, xsi:type, attributes) is
    // identical across the two ITS-XML lineages; they differ only by the root
    // `xmlns` string, which the `Namespace` serialize-time param selects. So a
    // single `ToXml` impl set — generated from the v1 (parity) XSD — serves both
    // (one impl per type; a second set would be a duplicate-impl conflict). The
    // v2 XSDs stay vendored; a genuine v2 structural divergence, if it ever
    // appears, would get its own trait then.
    let v1 = xsd::XsdModel::parse_files(&xsd::v1_files(Path::new(XSD_V1_DIR)))?;

    let gen_dir = Path::new(ITS_ROOT).join("src/xml/generated");
    std::fs::create_dir_all(&gen_dir)?;

    let schemas = [
        emit_xml::XmlSchema {
            model: &base_model,
            schema: &base,
            prelude: "openehr_base::prelude",
        },
        emit_xml::XmlSchema {
            model: &rm_model,
            schema: &rm,
            prelude: "openehr_rm::prelude",
        },
    ];
    let mut unmatched = Vec::new();
    let body = emit_xml::emit_file(&schemas, &v1, &mut unmatched);
    let impls_path = gen_dir.join("impls.rs");
    std::fs::write(&impls_path, &body)?;

    let mod_rs = "// @generated by openehr-codegen (emit-xml, ADR-005) — DO NOT EDIT.\n\
        //! Canonical-XML `ToXml`/`FromXml` impls for the RM/BASE spec types.\n\n\
        mod impls;\n";
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&mod_path, mod_rs)?;

    let written = vec![impls_path, mod_path];
    rustfmt(&written)?;
    println!(
        "emitted {} files into {} ({} XSD-only elements without a BMM field skipped)",
        written.len(),
        gen_dir.display(),
        unmatched.len()
    );
    Ok(())
}

/// Emit the OPT 1.4 model (`opt14`): typed Rust types + canonical-XML
/// `ToXml`/`FromXml` for `OPERATIONAL_TEMPLATE`, generated from the AM/OPT
/// constraint XSD closure (`Template.xsd` + includes). RM instance types
/// resolve to the already-generated `openehr-base`/`openehr-rm` impls (ADR-005).
fn cmd_emit_opt() -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let base_model = Model::merged(&[&base]);
    let rm_model = Model::merged(&[&base, &rm]);
    let base_specs = emit::emittable_specs(&base_model, &base);
    let rm_specs = emit::emittable_specs(&rm_model, &rm);

    // The AM/OPT constraint schemas share `Resource.xsd`+`BaseTypes.xsd` with the
    // RM-instance set; those shared types resolve to the RM/BASE XML impls.
    let xsd = xsd::XsdModel::parse_files(&xsd::am_files_v1(Path::new(XSD_V1_DIR)))?;
    let model = emit_opt::OptModel::new(&xsd, &base_specs, &rm_specs);

    let gen_dir = Path::new(ITS_ROOT).join("src/opt14");
    std::fs::create_dir_all(&gen_dir)?;

    let mut unmatched = Vec::new();
    let types_path = gen_dir.join("types.rs");
    let impls_path = gen_dir.join("impls.rs");
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&types_path, model.emit_types())?;
    std::fs::write(&impls_path, model.emit_impls(&mut unmatched))?;
    std::fs::write(&mod_path, emit_opt::OptModel::emit_mod())?;

    let written = vec![types_path, impls_path, mod_path];
    rustfmt(&written)?;
    println!(
        "emitted {} files into {} ({} XSD-only elements without a matching field skipped)",
        written.len(),
        gen_dir.display(),
        unmatched.len()
    );
    Ok(())
}

/// Diagnostic: parse the vendored v1 RM-instance XSDs and print a summary +
/// a couple of flattened views, to validate the XSD reader (ADR-005).
fn cmd_check_xsd() -> Result<(), Box<dyn std::error::Error>> {
    let files = xsd::v1_files(Path::new(XSD_V1_DIR));
    let model = xsd::XsdModel::parse_files(&files)?;
    println!(
        "✓ v1 XSDs: namespace={} types={}",
        model.namespace,
        model.types.len()
    );
    let abstract_n = model.types.values().filter(|t| t.is_abstract).count();
    println!("  abstract={abstract_n}");
    for probe in ["ELEMENT", "DV_CODED_TEXT", "COMPOSITION"] {
        if let Some(t) = model.types.get(probe) {
            let (attrs, elems) = model.flattened(probe);
            println!(
                "  {probe}: base={:?} attrs=[{}] elems=[{}]",
                t.base,
                attrs
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                elems
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
    }
    let dv = model.descendants("DATA_VALUE");
    println!(
        "  DATA_VALUE concrete descendants ({}): {}",
        dv.len(),
        dv.join(",")
    );
    Ok(())
}

const BASE_DOC: &str = "openEHR BASE (foundation + base types), generated from the BMM meta-model.";

fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("crates")
}

const AM_DOC: &str = "openEHR AM (Archetype Model): am14 (AM 1.4.0, for ADL 1.4) and am24 \
    (AM 2.4.0, for ADL 2) — both generated from BMM. Both ADL versions are in use.";

const RM_DOC: &str = "openEHR RM (Reference Model), generated from the BMM meta-model.";
const TERM_DOC: &str = "openEHR TERM (Terminology) data model, generated from the BMM \
    meta-model. The vendored terminology XML content lives in `assets/` (data, not \
    generated); an XML→model loader is added when composition validation needs it.";
const LANG_DOC: &str = "openEHR LANG: the BMM / P_BMM object model, generated from the BMM \
    meta-model. The generator's own BMM reader lives in openehr-codegen (tooling, not spec); \
    the runtime ODIN and EL parsers are future hand-written work (P8/P9).";

fn cmd_emit(_outdir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let term = load(TERM_BMM)?;

    // openehr-base: single version, no dependency crates.
    let base_model = Model::merged(&[&base]);
    let no_ext = External::default();
    write_crate(
        "openehr-base",
        &emit::emit_crate(&base_model, &base, &no_ext, BASE_DOC),
    )?;

    // Types exported by openehr-base — downstream crates resolve references to
    // these against `openehr_base::prelude` instead of degrading to Value.
    let base_specs = emit::emittable_specs(&base_model, &base);
    let ext_base = External::default().with(base_specs, "openehr_base::prelude");

    // openehr-rm: single version, depends on openehr-base.
    let rm_model = Model::merged(&[&base, &rm]);
    write_crate(
        "openehr-rm",
        &emit::emit_crate(&rm_model, &rm, &ext_base, RM_DOC),
    )?;

    // openehr-lang: the BMM/P_BMM object model (86 classes), fully generated.
    // The generator's own reader lives here in `openehr-codegen`, so there is no
    // bootstrap cycle. The runtime ODIN/EL parsers are future hand-written work.
    // Emitted before AM because AM's rule model references LANG types
    // (`ARCHETYPE.rules : List<STATEMENT_SET>`, `ARCHETYPE_SLOT.includes :
    // List<ASSERTION>`), so AM resolves them against `openehr_lang::prelude`.
    let lang = load(LANG_BMM)?.combined(&load(LANG_BMM3)?);
    let lang_model = Model::merged(&[&base, &lang]);
    write_crate(
        "openehr-lang",
        &emit::emit_crate(&lang_model, &lang, &ext_base, LANG_DOC),
    )?;
    let lang_specs = emit::emittable_specs(&lang_model, &lang);
    let ext_base_lang = External::default()
        .with(
            emit::emittable_specs(&base_model, &base),
            "openehr_base::prelude",
        )
        .with(lang_specs, "openehr_lang::prelude");

    // openehr-am: two versions in one crate, each depending on openehr-base and
    // openehr-lang. Each version merges BASE so its ancestors (e.g. ARCHETYPE ←
    // AUTHORED_RESOURCE) resolve; the two are kept in separate models because
    // AM 1.4 and 2.4 share class names.
    let am14 = load(AM14_BMM)?;
    let am24 = load(AM24_BMM)?;
    let m14 = Model::merged(&[&base, &am14]);
    let m24 = Model::merged(&[&base, &am24]);
    let am_files = emit::emit_multi_crate(
        &[("am14", &m14, &am14), ("am24", &m24, &am24)],
        &ext_base_lang,
        AM_DOC,
    );
    write_crate("openehr-am", &am_files)?;

    // openehr-term: the TERM data model (CODE_SET, TERMINOLOGY, …), depends on
    // openehr-base (TERMINOLOGY.date : Iso8601_date). The vendored terminology
    // XML in `assets/` is data (outside `src/`, survives regen).
    let term_model = Model::merged(&[&base, &term]);
    write_crate(
        "openehr-term",
        &emit::emit_crate(&term_model, &term, &ext_base, TERM_DOC),
    )?;
    Ok(())
}

/// Write a generated crate's `src/` in place. Wipes the crate's `src/` first
/// (there is no hand-written `*_impl.rs` yet; when there is, this must preserve
/// it).
fn write_crate(
    crate_name: &str,
    files: &[emit::GenFile],
) -> Result<(), Box<dyn std::error::Error>> {
    let src = crates_root().join(crate_name).join("src");
    if src.exists() {
        std::fs::remove_dir_all(&src)?;
    }
    std::fs::create_dir_all(&src)?;
    let mut written = Vec::with_capacity(files.len());
    for f in files {
        let full = src.join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, &f.body)?;
        written.push(full);
    }
    rustfmt(&written)?;
    println!("emitted {} files into {}", files.len(), src.display());
    Ok(())
}

/// Run `rustfmt` over the generated files so the emitted output is exactly what
/// `cargo fmt --all --check` expects (line wrapping, empty `{}`, import order,
/// …), instead of the emitter having to reproduce every rustfmt rule by hand.
fn rustfmt(files: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        return Ok(());
    }
    let status = std::process::Command::new("rustfmt")
        .args(["--edition", "2024", "--quiet"])
        .args(files)
        .status()?;
    if !status.success() {
        return Err(format!("rustfmt failed with status {status}").into());
    }
    Ok(())
}
