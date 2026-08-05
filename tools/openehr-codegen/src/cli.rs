//! The command-line interface: argument dispatch, input loading, and the
//! stage-orchestrating `cmd_*` handlers that wire LOAD → ANALYZE → PLAN →
//! RENDER together and write each emit target's files.

use crate::analyze::{augment_with_reemit, cross_schema_reemit, emittable_specs};
use crate::load::bmm::BmmSchema;
use crate::load::impls::SiblingImpls;
use crate::load::{oas, xsd};
use crate::plan::composition::{self, Composed, compose};
use crate::render::emit::{
    GenFile, crate_generations, emit_crate, emit_generations, emit_multi_crate, type_module_path,
};
use crate::render::{
    emit, emit_json, emit_opt, emit_rest, emit_rm_model, emit_validate, emit_xml, model_query,
    naming,
};
use std::path::{Path, PathBuf};
/// The `openehr-its` crate root (holds the vendored XSDs/OAS and receives the
/// generated XML/REST code). `../../crates/openehr-its` from this tool's
/// `tools/openehr-codegen` manifest dir.
const ITS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/openehr-its");
/// v1 (namespace `.../v1`) RM-instance XSD bundle dir — the Stage-1 parity target.
const XSD_V1_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/openehr-its/schemas/xml/its-xml-1.0.2-nsv1/ALL"
);
/// The AOM2 archetype-schema dir of the v1 bundle — the input to `emit-aom2`.
/// The bundle's own `examples/` documents live beside it and are the corpus the
/// generated codec is gated against.
const XSD_AOM2_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/openehr-its/schemas/xml/its-xml-1.0.2-nsv1/AOM2"
);
/// v2 (namespace `.../v2`) XSD root (per-component release folders). Supplies the
/// RM-instance types the v1 `ALL/` bundle lacks (EHR + demographic) or carries
/// stale (extract) to the emit-xml input.
const XSD_V2_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/openehr-its/schemas/xml/its-xml-2.0.0-nsv2"
);

/// Exit code for an unrecognized subcommand (distinct from a pipeline failure,
/// which exits 1, so a wrapper script can tell a typo from a codegen error).
const EXIT_USAGE: u8 = 2;

pub(crate) fn run() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map_or("check", String::as_str);
    let result = match cmd {
        "check" => cmd_check(),
        "emit" => cmd_emit(args.get(1).map(PathBuf::from)),
        "check-xsd" => cmd_check_xsd(),
        "emit-xml" => cmd_emit_xml(),
        "emit-json" => cmd_emit_json(),
        "emit-rest" => cmd_emit_rest(),
        "emit-opt" => cmd_emit_opt(),
        "emit-aom2" => cmd_emit_aom2(),
        "emit-rm-model" => cmd_emit_rm_model(),
        "emit-validate" => cmd_emit_validate(),
        "model-query" => cmd_model_query(args.get(1..).unwrap_or_default()),
        other => {
            eprintln!(
                "unknown command {other:?}; use `check`, `emit [OUTDIR]`, `check-xsd`, `emit-xml`, `emit-json`, `emit-rest`, `emit-opt`, `emit-aom2`, `emit-rm-model`, `emit-validate`, or `model-query`"
            );
            return std::process::ExitCode::from(EXIT_USAGE);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn cmd_check() -> Result<(), Box<dyn std::error::Error>> {
    for file in [
        composition::BASE_BMM,
        composition::RM_BMM,
        composition::TERM_BMM,
    ] {
        let s = composition::load_bmm(file)?;
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

/// Report what the vendored BMM states about every class attribute of the
/// loaded components, beside the Rust field shape the emitter currently emits
/// for it — a read-only query over the same LOAD → ANALYZE → PLAN → RENDER
/// decisions `emit` drives (see [`model_query`] for the BMM column definitions
/// and their `LANG` citations).
///
/// Usage: `model-query [--class NAME] [--attribute NAME] [--component KEY]
/// [--flattened] [--format table|tsv|json]`; no filter reports the whole loaded
/// model.
///
/// `--flattened` switches from one row per class × DECLARED attribute to one
/// row per class × **carried** attribute (inherited ones included), adding the
/// declaring class in the `declared_on` column — the inheritance dimension a
/// declared-only view cannot express.
fn cmd_model_query(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (query, format) = parse_model_query_args(args)?;
    print!("{}", model_query::render(&query, format)?);
    Ok(())
}

/// Parse `model-query`'s options (`--flag value` and `--flag=value` both work).
///
/// # Errors
/// Returns an error naming the valid options if an option is unknown, and an
/// error naming the valid values if `--format` is not one of them.
fn parse_model_query_args(
    args: &[String],
) -> Result<(model_query::Query<'_>, model_query::Format), Box<dyn std::error::Error>> {
    let mut query = model_query::Query::default();
    let mut format = model_query::Format::Table;
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) => (f, Some(v)),
            None => (arg.as_str(), None),
        };
        // A valueless switch must not swallow the next argument.
        if flag == "--flattened" {
            query.flattened = true;
            i += 1;
            continue;
        }
        let value = if let Some(v) = inline {
            i += 1;
            Some(v)
        } else {
            let v = args.get(i + 1).map(String::as_str);
            i += 2;
            v
        };
        let required = || -> Result<&str, Box<dyn std::error::Error>> {
            value.ok_or_else(|| format!("option {flag} needs a value").into())
        };
        match flag {
            "--class" => query.class = Some(required()?),
            "--attribute" => query.attribute = Some(required()?),
            "--component" => query.component = Some(required()?),
            "--format" => format = model_query::Format::parse(required()?)?,
            other => {
                return Err(format!(
                    "unknown model-query option {other:?}; valid options: --class NAME, \
                     --attribute NAME, --component KEY, --flattened, --format {}",
                    model_query::Format::VALID
                )
                .into());
            }
        }
    }
    Ok((query, format))
}

/// Emit the ITS-REST contract (DTOs, param structs, server trait, route table)
/// for each API group into `openehr-its/src/rest/generated/`.
fn cmd_emit_rest() -> Result<(), Box<dyn std::error::Error>> {
    // Every API group whose OAS declares operations. `overview` is excluded
    // because it is the release's index document: it declares no `paths`.
    // `system` DOES declare one — `system-codegen.openapi.yaml` `paths` `/`
    // `options` (operationId `options`, the STABLE Options-and-Conformance
    // operation) — so it is emitted like every other group. The completeness
    // rule admits no "nothing consumes it yet" exclusion.
    const GROUPS: &[&str] = &[
        "admin",
        "definition",
        "demographic",
        "ehr",
        "query",
        "system",
    ];
    let base = compose("base")?;
    let rm = compose("rm")?;
    // OAS $ref names are PascalCase (`EhrStatus`) — the same as the emitted Rust
    // type names — so map each crate's emittable spec names through `type_name`.
    let names = emit_rest::RmNames {
        base: emittable_specs(&base.model, &base.own_schema)
            .iter()
            .map(|s| naming::type_name(s))
            .collect(),
        rm: emittable_specs(&rm.model, &rm.own_schema)
            .iter()
            .map(|s| naming::type_name(s))
            .collect(),
    };

    let oas_dir = Path::new(ITS_ROOT).join("vendor/rest-oas");
    let gen_dir = Path::new(ITS_ROOT).join("src/rest/generated");
    std::fs::create_dir_all(&gen_dir)?;

    // Load every bundle up front: the cross-group hoist analysis needs the
    // whole set (schemas identical in every declaring group hoist into the
    // shared `common` module — `emit_rest::hoist_set`).
    let mut bundles: Vec<(&str, oas::Oas)> = Vec::new();
    for group in GROUPS {
        let oas = oas::Oas::parse_file(&oas_dir.join(format!("{group}-codegen.openapi.yaml")))?;
        bundles.push((group, oas));
    }
    let hoisted = emit_rest::hoist_set(&bundles, &names);

    let mut written = Vec::new();
    // `common` always emits from the merged per-name view (first declarer
    // wins; the copies are identical by the hoist analysis) — ONE emission
    // path, so no representative-vs-merged equivalence needs testing (#1854).
    {
        let merged = oas::Oas::merged_schemas(&bundles, &hoisted);
        let body = emit_rest::emit_common(&merged, &names, &hoisted);
        let path = gen_dir.join("common.rs");
        std::fs::write(&path, emit::guard_value_carriers(&body))?;
        written.push(path);
    }
    for (group, oas) in &bundles {
        let body = emit_rest::emit_group(oas, group, &names, &hoisted);
        let path = gen_dir.join(format!("{group}.rs"));
        std::fs::write(&path, emit::guard_value_carriers(&body))?;
        written.push(path);
    }
    let mod_rs = {
        let mut s = String::from(
            "// @generated by openehr-codegen (emit-rest) — DO NOT EDIT.\n\
             //! ITS-REST contract, one module per API group, plus the shared\n\
             //! `common` module (cross-group hoisted component schemas).\n\n\
             pub mod common;\n",
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
/// `openehr-its/src/xml/generated/`. Generates both wire lineages: v1
/// (`.../v1`, parity target) and v2 (`.../v2`, latest).
fn cmd_emit_xml() -> Result<(), Box<dyn std::error::Error>> {
    let base = compose("base")?;
    let rm = compose("rm")?;
    let rm_aug = augmented_schema(&rm);

    // The RM-instance wire shape (element names, order, xsi:type, attributes) is
    // identical across the two ITS-XML lineages; they differ only by the root
    // `xmlns` string, which the `Namespace` serialize-time param selects. So a
    // single `ToXml` impl set — generated from the v1 (parity) XSD — serves both
    // (one impl per type; a second set would be a duplicate-impl conflict).
    //
    // The v1 `ALL/` bundle is not a complete RM closure: it has no `Ehr.xsd` and
    // no demographic schema, and its `Extract.xsd` is the stale RM-1.0.2 model.
    // So the emit-xml input is the v1 *served* core (which wins for shared types
    // via first-wins `.or_insert`) followed by the v2 RM-1.1.0 EHR/demographic/
    // extract schemas, which supply the LOCATABLE subtypes the v1 bundle lacks —
    // resolving `archetype_node_id` as the required XML **attribute** for
    // EHR_STATUS/EHR_ACCESS, the demographic PARTY hierarchy, and the extract
    // LOCATABLE subtypes. Same wire shape bar the root `xmlns`.
    let v1 = xsd::XsdModel::parse_files(&xsd::xml_emit_files(
        Path::new(XSD_V1_DIR),
        Path::new(XSD_V2_DIR),
    ))?;

    let gen_dir = Path::new(ITS_ROOT).join("src/xml/generated");
    std::fs::create_dir_all(&gen_dir)?;

    let schemas = [
        emit_xml::XmlSchema {
            model: &base.model,
            schema: &base.own_schema,
            prelude: "openehr_base::prelude",
        },
        emit_xml::XmlSchema {
            model: &rm.model,
            // The XML codec covers the crate's re-emitted closure twins too
            // (#1699: the BASE Interval/Iso8601 family re-emitted into
            // openehr-rm) — the impls must exist for every type a field of
            // the augmented schema names.
            schema: &rm_aug,
            prelude: "openehr_rm::prelude",
        },
    ];
    let mut unmatched = Vec::new();
    let body = emit_xml::emit_file(&schemas, &v1, &mut unmatched)?;
    let impls_path = gen_dir.join("impls.rs");
    std::fs::write(&impls_path, emit::guard_value_carriers(&body))?;

    let mod_rs = "// @generated by openehr-codegen (emit-xml) — DO NOT EDIT.\n\
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

/// Emit the canonical-JSON `serde::Serialize`/`serde::Deserialize` impls for
/// every generated spec type (BASE / RM / LANG / AM 1.4 + 2.4 / TERM).
///
/// The impls land in each spec crate's own `src/json_serde.rs` — both traits and
/// the spec types are foreign to `openehr-its`, so an impl there would violate
/// the orphan rule — and the `_type`-keyed structural dispatch stays in
/// `openehr-its/src/json_codec/generated/`, where it can name every crate.
/// Covers the same crate composition `emit` consumes, including AM 2.4's
/// cross-schema re-emission closure.
fn cmd_emit_json() -> Result<(), Box<dyn std::error::Error>> {
    let base = compose("base")?;
    let rm = compose("rm")?;
    let lang = compose("lang")?;
    let am14 = compose("am14")?;
    let am24 = compose("am24")?;
    let term = compose("term")?;

    // Every crate whose emission grafts a cross-schema re-emission closure
    // (see `augmented_schema` — #1699: rm, am14, am24) needs the JSON codec to
    // cover the re-emitted crate-local types too, so `json_types` must see
    // exactly the class set `emit` renders.
    let rm_aug = augmented_schema(&rm);
    let am14_aug = augmented_schema(&am14);
    let am24_aug = augmented_schema(&am24);

    // The codec is keyed by the crate PRELUDE path, and `_type` dispatch admits
    // exactly one impl per Rust type — so a crate composed of several BMM
    // generations contributes one codec per prelude-OWNED class, per generation.
    // For LANG that means the v3 (bmm3) shape for the 18 names both generations
    // declare and the v2 shape for everything only the v2 file declares; the v2
    // twins of the colliding names get NO wire codec, because they are not in
    // the prelude and no wire route serves them (they are the in-process
    // reflection surface the P_BMM pipeline materialises —
    // `LANG/docs/bmm/master06-persistence.adoc`).
    let no_unexported = std::collections::BTreeMap::new();
    // A crate composed of several BMM generations exports one type per Rust NAME
    // from its prelude, so for a class name both generations declare, the losing
    // twin has to be named by its full module path. Both twins keep a codec: they
    // are distinct Rust types, so the impls do not conflict and the emitted model
    // stays codec-complete (see `emit_json::JsonSchema::unexported`).
    let lang_unexported: Vec<std::collections::BTreeMap<String, String>> = lang
        .generations
        .iter()
        .map(|g| {
            g.schema
                .classes
                .keys()
                .filter(|name| !g.owned.contains(*name))
                .map(|name| {
                    (
                        name.clone(),
                        format!("openehr_lang::{}", type_module_path(&g.schema, name)),
                    )
                })
                .collect()
        })
        .collect();

    let base_schema = emit_json::JsonSchema {
        model: &base.model,
        schema: &base.own_schema,
        prelude: "openehr_base::prelude",
        unexported: &no_unexported,
    };
    let rm_schema = emit_json::JsonSchema {
        model: &rm.model,
        schema: &rm_aug,
        prelude: "openehr_rm::prelude",
        unexported: &no_unexported,
    };
    let lang_schemas: Vec<emit_json::JsonSchema<'_>> = lang
        .generations
        .iter()
        .zip(&lang_unexported)
        .map(|(g, unexported)| emit_json::JsonSchema {
            model: &g.model,
            schema: &g.schema,
            prelude: "openehr_lang::prelude",
            unexported,
        })
        .collect();
    let meta_schemas = [
        emit_json::JsonSchema {
            model: &am14.model,
            schema: &am14_aug,
            prelude: "openehr_am::am14::prelude",
            unexported: &no_unexported,
        },
        emit_json::JsonSchema {
            model: &am24.model,
            schema: &am24_aug,
            prelude: "openehr_am::am24::prelude",
            unexported: &no_unexported,
        },
        emit_json::JsonSchema {
            model: &term.model,
            schema: &term.own_schema,
            prelude: "openehr_term::prelude",
            unexported: &no_unexported,
        },
    ];

    // The structural dispatch is keyed by the bare canonical-JSON `_type`
    // string, so a class name several components' BMMs declare (110 of them —
    // `RESOURCE_DESCRIPTION`, `AUTHORED_RESOURCE`, the `BMM_*` family) resolves
    // by SCHEMA PRIORITY, and this is the priority order: RM first, then BASE,
    // then the archetype/meta components. Rationale: the dispatch's caller is
    // the RM wire-boundary validator (`openehr_its::wire_validate`), and the
    // same-named twins differ materially (RM's `RESOURCE_DESCRIPTION_ITEM.language`
    // is a `CODE_PHRASE`, BASE's a `Terminology_code`), so decoding an RM wire
    // node with another component's shape would be wrong. No openEHR spec
    // governs a cross-component `_type` namespace — our own design.
    let mut structural_schemas = vec![rm_schema, base_schema];
    structural_schemas.extend(lang_schemas.iter().copied());
    structural_schemas.extend(meta_schemas);

    // One emitted file per SPEC CRATE: the impls must live where the types are
    // defined (orphan rule), and being in-crate is also what lets them read the
    // `pub(crate)` fields of a validated class and construct through its
    // hand-written door (`plan::construction`).
    let per_crate: [(&str, &str, Vec<emit_json::JsonSchema<'_>>); 5] = [
        ("openehr-base", "openehr_base", vec![base_schema]),
        ("openehr-rm", "openehr_rm", vec![rm_schema]),
        ("openehr-lang", "openehr_lang", lang_schemas.clone()),
        (
            "openehr-am",
            "openehr_am",
            vec![meta_schemas[0], meta_schemas[1]],
        ),
        ("openehr-term", "openehr_term", vec![meta_schemas[2]]),
    ];
    let mut written = Vec::new();
    for (dir, krate, schemas) in &per_crate {
        let src = crates_root().join(dir).join("src");
        std::fs::create_dir_all(&src)?;
        let path = src.join("json_serde.rs");
        std::fs::write(
            &path,
            emit::guard_value_carriers(&emit_json::emit_file(schemas, krate)),
        )?;
        declare_json_serde_module(&src.join("lib.rs"), &mut written)?;
        written.push(path);
    }

    let gen_dir = Path::new(ITS_ROOT).join("src/json_codec/generated");
    std::fs::create_dir_all(&gen_dir)?;

    let structural = emit_json::emit_structural_file(&structural_schemas);
    let structural_path = gen_dir.join("structural.rs");
    std::fs::write(&structural_path, emit::guard_value_carriers(&structural))?;
    written.push(structural_path);

    let mod_rs = "// @generated by openehr-codegen (emit-json) — DO NOT EDIT.\n\
        //! The canonical-JSON `_type` dispatch (the per-type `serde` impls live in\n\
        //! each spec crate's own `json_serde` module).\n\n\
        pub mod structural;\n";
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&mod_path, mod_rs)?;
    written.push(mod_path);

    rustfmt(&written)?;
    println!("emitted {} files", written.len());
    Ok(())
}

/// Declare the emitted `json_serde` module in a spec crate's generated
/// `lib.rs`.
///
/// `emit` regenerates `lib.rs` (and deletes every `@generated` file, this one
/// included), so the declaration is (re)appended by the target that owns the
/// file. Both intermediate states are self-consistent: after `emit` neither the
/// module nor its declaration exists, after `emit-json` both do.
fn declare_json_serde_module(
    lib: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    const DECL: &str = "mod json_serde;";
    let mut body = std::fs::read_to_string(lib)?;
    if body.contains(DECL) {
        return Ok(());
    }
    body.push_str(
        "\n// canonical-JSON `serde` impls (openehr-codegen -- emit-json), auto-declared:\n",
    );
    body.push_str(DECL);
    body.push('\n');
    std::fs::write(lib, &body)?;
    written.push(lib.to_path_buf());
    Ok(())
}

/// Emit one XSD-driven constraint-model module (`types.rs` + `impls.rs` +
/// `mod.rs`) under `crates/openehr-its/src/<dir>`.
///
/// Shared by every `emit_opt` target so the three modules (`opt14`, `aom2`,
/// `aom2_model`) stay structurally identical: only the XSD closure, the emission
/// target (module path + banners) and the module surface differ. The generate/
/// resolve partition is the same in all three — the closures share
/// `Resource.xsd`+`BaseTypes.xsd` with the RM-instance set, and those shared types
/// resolve to the already-generated `openehr-base`/`openehr-rm` XML impls while the
/// archetype constraint model is generated fresh.
fn emit_xsd_model(
    base_specs: &std::collections::BTreeSet<String>,
    rm_specs: &std::collections::BTreeSet<String>,
    files: &[PathBuf],
    dir: &str,
    target: &'static emit_opt::ModelTarget,
    module: &emit_opt::ModuleSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    let xsd = xsd::XsdModel::parse_files(files)?;
    let model = emit_opt::OptModel::new(&xsd, base_specs, rm_specs, target);

    let gen_dir = Path::new(ITS_ROOT).join("src").join(dir);
    std::fs::create_dir_all(&gen_dir)?;

    let mut unmatched = Vec::new();
    let types_path = gen_dir.join("types.rs");
    let impls_path = gen_dir.join("impls.rs");
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&types_path, emit::guard_value_carriers(&model.emit_types()))?;
    std::fs::write(
        &impls_path,
        emit::guard_value_carriers(&model.emit_impls(&mut unmatched)),
    )?;
    std::fs::write(&mod_path, emit_opt::emit_module(module))?;

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

/// Emit the OPT 1.4 model (`opt14`): typed Rust types + canonical-XML
/// `ToXml`/`FromXml` for `OPERATIONAL_TEMPLATE`, generated from the AM/OPT
/// constraint XSD closure (`Template.xsd` + includes). RM instance types
/// resolve to the already-generated `openehr-base`/`openehr-rm` impls.
fn cmd_emit_opt() -> Result<(), Box<dyn std::error::Error>> {
    let base = compose("base")?;
    let rm = compose("rm")?;
    let base_specs = emittable_specs(&base.model, &base.own_schema);
    let rm_specs = emittable_specs(&rm.model, &rm.own_schema);

    emit_xsd_model(
        &base_specs,
        &rm_specs,
        &xsd::am_files_v1(Path::new(XSD_V1_DIR)),
        "opt14",
        &emit_opt::OPT_TARGET,
        &emit_opt::OPT_MODULE,
    )
}

/// Emit BOTH AOM2 archetype XML serializations the vendored bundle publishes,
/// each from its own closure into its own module:
///
/// - `aom2` — the **persistent** form (`P_Archetype.xsd` → `P_AUTHORED_ARCHETYPE`),
///   the shape the bundle's 8 example documents carry;
/// - `aom2_model` — the AOM **model** form (`Archetype.xsd` → `AUTHORED_ARCHETYPE`).
///
/// They are two closures rather than one merged model because both schemas
/// declare the same top-level element `archetype` with different root types and
/// define same-named supporting types; see [`xsd::AOM2_FILES`] /
/// [`xsd::AOM2_MODEL_FILES`] for the full adjudication, including why the model
/// form's entry points are typed to `AUTHORED_ARCHETYPE` and not to the
/// `abstract` `ARCHETYPE` the schema's global element names.
fn cmd_emit_aom2() -> Result<(), Box<dyn std::error::Error>> {
    let base = compose("base")?;
    let rm = compose("rm")?;
    let base_specs = emittable_specs(&base.model, &base.own_schema);
    let rm_specs = emittable_specs(&rm.model, &rm.own_schema);
    let aom2_dir = Path::new(XSD_AOM2_DIR);

    emit_xsd_model(
        &base_specs,
        &rm_specs,
        &xsd::aom2_files(aom2_dir),
        "aom2",
        &emit_opt::AOM2_TARGET,
        &emit_opt::AOM2_MODULE,
    )?;
    emit_xsd_model(
        &base_specs,
        &rm_specs,
        &xsd::aom2_model_files(aom2_dir),
        "aom2_model",
        &emit_opt::AOM2_MODEL_TARGET,
        &emit_opt::AOM2_MODEL_MODULE,
    )
}

/// Emit the static RM attribute/type model (`openehr-rm/src/model/`) — the AQL
/// planner's spec-pinned oracle. Generated from the same
/// BASE + RM BMM `emit` consumes. Writes the `model/` subtree in place (does not
/// touch the generated spec files) and declares `pub mod model;` in `lib.rs` if
/// absent, so it is correct run standalone; `emit` produces the identical output.
fn cmd_emit_rm_model() -> Result<(), Box<dyn std::error::Error>> {
    let rm = compose("rm")?;
    let files = emit_rm_model::emit_files(&rm.model);

    let src = crates_root().join("openehr-rm").join("src");
    let mut written = Vec::new();
    for f in &files {
        let full = src.join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, emit::guard_value_carriers(&f.body))?;
        written.push(full);
    }
    // `emit` is the usual authority for lib.rs; ensure the module is declared so
    // the standalone target is correct + byte-identical to `emit`'s output.
    let lib = src.join("lib.rs");
    if lib.exists() {
        let mut body = std::fs::read_to_string(&lib)?;
        if !body.contains("pub mod model;") {
            body.push_str("pub mod model;\n");
            std::fs::write(&lib, &body)?;
            written.push(lib);
        }
    }
    rustfmt(&written)?;
    println!(
        "emitted {} files into {}",
        files.len(),
        src.join("model").display()
    );
    Ok(())
}

/// Emit the RM class-invariant cores (`openehr-rm/src/validate/generated.rs`) —
/// the single source both the typed `Validate` impls and the fast path call.
/// Writes the one generated file in place; the module declaration
/// (`pub(crate) mod generated;`) is a permanent hand edit in the hand-written
/// `validate.rs`, so this target never touches a hand-written file. `emit`
/// produces the identical output (via `inject_validate`).
fn cmd_emit_validate() -> Result<(), Box<dyn std::error::Error>> {
    let rm = compose("rm")?;
    let rm_aug = augmented_schema(&rm);
    let files = emit_validate::emit_files(&rm.model, &rm_aug);

    let src = crates_root().join("openehr-rm").join("src");
    let mut written = Vec::new();
    for f in &files {
        let full = src.join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, emit::guard_value_carriers(&f.body))?;
        written.push(full);
    }
    rustfmt(&written)?;
    println!(
        "emitted {} file(s) into {}",
        files.len(),
        src.join("validate").display()
    );
    Ok(())
}

/// Append the generated invariant-core file to `openehr-rm`'s file set. The
/// module is declared by the permanent `pub(crate) mod generated;` hand edit in
/// the hand-written `validate.rs`, so nothing else is touched here.
fn inject_validate(files: &mut Vec<GenFile>, mut validate_files: Vec<GenFile>) {
    files.append(&mut validate_files);
}

/// Append the generated RM-model files to `openehr-rm`'s file set and declare the
/// module in its `lib.rs` (the authority for the crate layout).
fn inject_rm_model(files: &mut Vec<GenFile>, mut model_files: Vec<GenFile>) {
    for f in files.iter_mut() {
        if f.path == "lib.rs" && !f.body.contains("pub mod model;") {
            f.body.push_str("pub mod model;\n");
        }
    }
    files.append(&mut model_files);
}

/// Diagnostic: parse the vendored v1 RM-instance XSDs and print a summary +
/// a couple of flattened views, to validate the XSD reader.
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

/// A composition's own schema with its cross-schema re-emission closure
/// grafted in (`cross_schema_reemit` → `augment_with_reemit`): every upstream
/// class whose Rust form WIDENS in this crate (a closed-subtype-set enum the
/// crate's own classes extend) is re-emitted crate-locally at its source
/// package path, so downstream references resolve against the widened local
/// twin — the completeness hard rule (owner 2026-07-19), applied uniformly to
/// EVERY composition rather than am24 alone (#1699: rm re-emits BASE's
/// `Interval`/`Iso8601_type` family; am14 re-emits `AUTHORED_RESOURCE` +
/// `RESOURCE_DESCRIPTION`). A composition with an empty closure comes back
/// unchanged, so applying this unconditionally is safe by construction.
fn augmented_schema(c: &Composed) -> BmmSchema {
    let reemit = cross_schema_reemit(&c.model, &c.own_schema);
    let dep_refs: Vec<&BmmSchema> = c.dep_schemas.iter().collect();
    augment_with_reemit(&c.own_schema, &c.model, &reemit, &dep_refs)
}

/// The hand-written `*_impl.rs` siblings a generated crate already carries — an
/// emitter INPUT (`crate::load::impls`), read before rendering so a type file's
/// banner names a sibling only when one exists.
fn sibling_impls(crate_name: &str) -> SiblingImpls {
    SiblingImpls::scan(&crates_root().join(crate_name).join("src"))
}

fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("crates")
}

fn cmd_emit(_outdir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Each crate's schema composition (member BMM files, dependency preludes) is
    // the declarative `plan::composition::COMPOSITIONS` table; `compose` resolves
    // an entry into the merged model + own schema + prelude index. See that table
    // for the `includes` citations behind each merge.

    // openehr-base: single version, no dependency crates.
    let base = compose("base")?;
    write_crate(
        "openehr-base",
        &emit_crate(
            &base.model,
            &base.own_schema,
            &base.external,
            base.doc,
            base.spec_version,
            &sibling_impls("openehr-base"),
        ),
    )?;

    // openehr-rm: depends on openehr-base. Also carries the static RM attribute/
    // type model, emitted here too so a plain `emit` keeps the crate
    // self-consistent (lib.rs declares `model`, and a later `emit` regenerates it
    // byte-identically to the standalone `emit-rm-model` target).
    let rm = compose("rm")?;
    let rm_aug = augmented_schema(&rm);
    let mut rm_files = emit_crate(
        &rm.model,
        &rm_aug,
        &rm.external,
        rm.doc,
        rm.spec_version,
        &sibling_impls("openehr-rm"),
    );
    inject_rm_model(&mut rm_files, emit_rm_model::emit_files(&rm.model));
    inject_validate(&mut rm_files, emit_validate::emit_files(&rm.model, &rm_aug));
    write_crate("openehr-rm", &rm_files)?;

    // openehr-lang: the BMM/P_BMM object model, fully generated. The generator's
    // own reader lives in `openehr-codegen`, so there is no bootstrap cycle.
    // Emitted before AM because AM's rule model references LANG types
    // (`ARCHETYPE.rules : List<STATEMENT_SET>`, `ARCHETYPE_SLOT.includes :
    // List<ASSERTION>`), so AM resolves them against `openehr_lang::prelude`.
    // LANG is composed of TWO BMM generations (the stable v2.x BMM + P_BMM +
    // beom, and the v3 development line), each emitted completely at its own
    // source-package path — see `plan::composition`'s LANG entry.
    let lang = compose("lang")?;
    write_crate(
        "openehr-lang",
        &emit_generations(
            &crate_generations(&lang),
            &lang.external,
            lang.doc,
            lang.spec_version,
            &sibling_impls("openehr-lang"),
        ),
    )?;

    // openehr-am: two versions in one crate, each depending on openehr-base and
    // openehr-lang. Each version merges BASE so its ancestors (e.g. ARCHETYPE ←
    // AUTHORED_RESOURCE) resolve; the two are kept in separate models because
    // AM 1.4 and 2.4 share class names.
    // AM 2.4's `rules` package declares subtypes of LANG's beom expression
    // classes (EXPR_ARCHETYPE_REF ⊂ EXPR_VALUE_REF, EXPR_CONSTRAINT ⊂ EXPR_LEAF).
    // Per the owner ruling 2026-07-19, that cross-`includes` extension is re-opened
    // at the DOWNSTREAM crate: the reachable beom expression/statement closure is
    // re-emitted into `openehr-am` as crate-local types (an extender-level enum
    // set composing the LANG variants + the AM leaves), so `ARCHETYPE.rules` /
    // `ARCHETYPE_SLOT.includes` resolve against the AM-level types. openehr-lang
    // stays byte-identical (the closure is emitted only here, never upstream).
    let am14 = compose("am14")?;
    let am24 = compose("am24")?;
    // `cross_schema_reemit` computes the COMPLETE set of upstream classes whose
    // Rust form widens downstream and grafts them into each crate's schema at
    // the source package paths — a full, non-minimal re-emission (owner ruling
    // 2026-07-19), applied uniformly by `augmented_schema` (#1699): am14
    // re-emits BASE's AUTHORED_RESOURCE + RESOURCE_DESCRIPTION (ARCHETYPE
    // extends the former), am24 the reachable LANG beom closure.
    let am14_aug = augmented_schema(&am14);
    let am24_aug = augmented_schema(&am24);
    let am_files = emit_multi_crate(
        &[
            ("am14", &am14.model, &am14_aug),
            ("am24", &am24.model, &am24_aug),
        ],
        &am24.external,
        am24.doc,
        am24.spec_version,
        &sibling_impls("openehr-am"),
    );
    write_crate("openehr-am", &am_files)?;

    // openehr-term: the TERM data model (CODE_SET, TERMINOLOGY, …), depends on
    // openehr-base (TERMINOLOGY.date : Iso8601_date). The vendored terminology
    // XML in `assets/` is data (outside `src/`, survives regen).
    let term = compose("term")?;
    write_crate(
        "openehr-term",
        &emit_crate(
            &term.model,
            &term.own_schema,
            &term.external,
            term.doc,
            term.spec_version,
            &sibling_impls("openehr-term"),
        ),
    )?;
    Ok(())
}

/// Write a generated crate's `src/` in place. Wipes the crate's `src/` first
/// (there is no hand-written `*_impl.rs` yet; when there is, this must preserve
/// it).
fn write_crate(crate_name: &str, files: &[GenFile]) -> Result<(), Box<dyn std::error::Error>> {
    let src = crates_root().join(crate_name).join("src");
    // Preserve hand-written code: delete only previously-`@generated`
    // files, never the hand-written `*_impl.rs` / spec-behaviour modules beside
    // them. (A stale generated file no longer emitted this run is `@generated` →
    // removed; a hand-written sibling is kept.)
    if src.exists() {
        remove_generated_files(&src)?;
    }
    std::fs::create_dir_all(&src)?;
    let mut written = Vec::with_capacity(files.len());
    for f in files {
        let full = src.join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, emit::guard_value_carriers(&f.body))?;
        written.push(full);
    }
    // Weave hand-written modules into the generated tree: any hand-written `.rs`
    // beside a generated `mod.rs` (or at the crate root beside `lib.rs`) is
    // declared `pub mod <name>;` so the hand-written `*_impl.rs` files compile without the
    // generator owning them. Deterministic (sorted scan) → drift-check-stable.
    declare_hand_written_modules(&src, &mut written)?;
    rustfmt(&written)?;
    println!("emitted {} files into {}", files.len(), src.display());
    Ok(())
}

/// Whether a file's first line marks it as generated (`// @generated …`).
fn is_generated_file(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .is_ok_and(|s| s.lines().next().is_some_and(|l| l.contains("@generated")))
}

/// Recursively delete every `@generated` file under `dir` (leaving hand-written
/// files + their directories in place), then prune directories left empty.
fn remove_generated_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            remove_generated_files(&path)?;
            if std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
            }
        } else if is_generated_file(&path) {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// For each directory whose module anchor (`lib.rs` at the crate root, else
/// `mod.rs`) is **generated**, append `pub mod <name>;` for every hand-written
/// child module not already declared — both hand-written `.rs` files
/// (`foo_impl.rs`) and hand-written module directories (a subdir whose own
/// `mod.rs` is hand-written, e.g. `odin/`). A hand-written anchor is left
/// untouched: it manages its own submodules (so we never duplicate an `odin/`
/// mod.rs's private `mod lexer;`). Modified anchors are added to `written` for
/// rustfmt.
fn declare_hand_written_modules(
    src: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // (dir, module-anchor file) pairs: the root anchors on lib.rs, each subdir on
    // its mod.rs. Collect all dirs first (sorted) for deterministic output.
    let mut dirs = vec![src.to_path_buf()];
    collect_dirs(src, &mut dirs)?;
    dirs.sort();
    for dir in dirs {
        let anchor = if dir == src {
            dir.join("lib.rs")
        } else {
            dir.join("mod.rs")
        };
        // Only extend a GENERATED anchor. A hand-written `mod.rs` (a hand-written
        // module directory) owns its own `mod`/`pub mod` declarations, so
        // appending here would duplicate them. The crate-root `lib.rs` is always
        // (re)generated by `emit_lib` — its `@generated` marker sits on a later
        // line (line 1 is the crate doc), so recognise it as generated directly.
        let anchor_generated = dir == src || is_generated_file(&anchor);
        if !anchor.exists() || !anchor_generated {
            continue;
        }
        let mut hand_written: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let p = entry?.path();
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if p.is_dir() {
                // A hand-written module directory: it has a `mod.rs` that is not
                // generated. A fully-generated subpackage's `mod.rs` is already
                // declared by the parent's generated module tree, so skip it.
                let child_mod = p.join("mod.rs");
                if child_mod.exists() && !is_generated_file(&child_mod) {
                    hand_written.push(stem.to_owned());
                }
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs")
                && !matches!(stem, "mod" | "lib" | "prelude")
                && !is_generated_file(&p)
            {
                hand_written.push(stem.to_owned());
            }
        }
        if hand_written.is_empty() {
            continue;
        }
        hand_written.sort();
        let mut body = std::fs::read_to_string(&anchor)?;
        let mut appended = false;
        for m in hand_written {
            let decl = format!("pub mod {m};");
            if !body.contains(&decl) {
                if !appended {
                    body.push_str("\n// hand-written modules (spec behaviour), auto-declared:\n");
                    appended = true;
                }
                body.push_str(&decl);
                body.push('\n');
            }
        }
        if appended {
            std::fs::write(&anchor, &body)?;
            written.push(anchor);
        }
    }
    Ok(())
}

/// Recursively collect subdirectories of `dir` into `out`.
fn collect_dirs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            out.push(path.clone());
            collect_dirs(&path, out)?;
        }
    }
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
