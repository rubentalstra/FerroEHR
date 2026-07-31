//! Schema inclusion resolution: one `P_BMM_SCHEMA` plus the schemas it includes
//! (transitively) merged into a single self-contained schema.
//!
//! "`.bmm` files function as schemas that support schema inclusion and therefore
//! re-use, in a similar manner to the XML schema languages. Thus, a single
//! logical BMM model can be expressed as a _number_ of `.bmm` schema files which
//! are actually `P_BMM_*` object serialisations of parts of the BMM model. A
//! schema reading component has to resolve the schema inclusions and ultimately
//! `BMM_*` object instantiations to obtain the in-memory form of the model"
//! (`LANG/docs/bmm_persistence/master02-overview.adoc` §Conceptual Approach).
//!
//! The merge itself is `P_BMM_SCHEMA.merge (other)` with precondition
//! `includes_to_process.has (included_schema.schema_id)`, and
//! `P_BMM_PACKAGE.merge (other)` — "Merge packages and classes from other (from
//! an included `P_BMM_SCHEMA`) into this package"
//! (`LANG/docs/UML/classes/org.openehr.lang.bmm_persistence.p_bmm_schema.adoc`
//! + `…p_bmm_package.adoc` §Functions).
//!
//! **Precedence: the includer wins.** That is exactly what
//! `P_BMM_CLASS.is_override` records — "True if this class definition overrides
//! one found in an included schema" (`…p_bmm_class.adoc` §Attributes) — so a
//! class name defined by both the including and an included schema keeps the
//! including schema's definition, and that definition is marked
//! `is_override = True`.

use std::collections::BTreeMap;

use crate::bmm_persistence::error::PBmmReadError;
use crate::bmm_persistence::p_bmm_class::PBmmClass;
use crate::bmm_persistence::p_bmm_package::PBmmPackage;
use crate::bmm_persistence::p_bmm_schema::PBmmSchema;

/// Resolve `root`'s inclusions against `loaded`, returning one self-contained
/// schema.
///
/// `loaded` supplies every schema that may be included. Its map KEY is only a
/// caller-side label: resolution indexes each value by its own
/// [`PBmmSchema::schema_id`], which renders lower-cased, and matches a
/// `BMM_INCLUDE_SPEC.id` against it lower-cased too — the vendored schemas write
/// include ids in either case. Resolution is transitive (an included schema's own
/// includes are resolved first) and cycle-safe.
///
/// The returned schema keeps `root`'s header attributes and its `includes`
/// record verbatim: `includes` is a persisted attribute stating what the root
/// schema declares, not a work list to be consumed.
///
/// # Errors
/// Returns [`PBmmReadError::MissingInclude`] when a declared include is not in
/// `loaded`, [`PBmmReadError::IncludeCycle`] when the inclusion graph is cyclic,
/// and [`PBmmReadError::DuplicateSchemaId`] when two entries of `loaded` render
/// the same schema id.
pub fn resolve_includes(
    root: PBmmSchema,
    loaded: &BTreeMap<String, PBmmSchema>,
) -> Result<PBmmSchema, PBmmReadError> {
    let mut by_id: BTreeMap<String, &PBmmSchema> = BTreeMap::new();
    for schema in loaded.values() {
        let id = schema.schema_id();
        if by_id.insert(id.clone(), schema).is_some() {
            return Err(PBmmReadError::DuplicateSchemaId { id });
        }
    }
    let mut chain: Vec<String> = Vec::new();
    resolve(root, &by_id, &mut chain)
}

/// Resolves one schema's inclusions, with `chain` as the active inclusion stack.
fn resolve(
    schema: PBmmSchema,
    by_id: &BTreeMap<String, &PBmmSchema>,
    chain: &mut Vec<String>,
) -> Result<PBmmSchema, PBmmReadError> {
    let id = schema.schema_id();
    if chain.contains(&id) {
        let mut cycle = chain.clone();
        cycle.push(id);
        return Err(PBmmReadError::IncludeCycle {
            chain: cycle.join(" -> "),
        });
    }
    chain.push(id.clone());
    let mut merged = schema;
    // BTreeMap key order — deterministic, so a name collision between two
    // included schemas always resolves the same way.
    let include_ids: Vec<String> = merged
        .includes
        .iter()
        .flat_map(BTreeMap::values)
        .map(|spec| spec.id.clone())
        .collect();
    for include_id in include_ids {
        let key = include_id.to_lowercase();
        let Some(included) = by_id.get(&key) else {
            return Err(PBmmReadError::MissingInclude {
                requester: id.clone(),
                id: include_id,
            });
        };
        let included = resolve((*included).clone(), by_id, chain)?;
        absorb(&mut merged, included);
    }
    chain.pop();
    Ok(merged)
}

/// Merges `included` into `includer`, with `includer` winning every collision.
fn absorb(target: &mut PBmmSchema, source: PBmmSchema) {
    absorb_classes(target, source.primitive_types, ClassList::Primitive);
    absorb_classes(target, source.class_definitions, ClassList::Definitions);
    merge_packages(&mut target.packages, source.packages);
}

/// Which of the schema's two class lists an included class joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassList {
    /// `P_BMM_SCHEMA.primitive_types`.
    Primitive,
    /// `P_BMM_SCHEMA.class_definitions`.
    Definitions,
}

/// Appends every class of `incoming` the includer does not already define.
///
/// A class name is looked up across BOTH of the includer's lists, because
/// `master04-syntax.adoc` §Classes for Primitive Types says primitive types
/// "are just normal class definitions within a `primitive_types` block …
/// otherwise are processed in the same way as types defined in the main
/// `class_definitions` group" — so one name means one class regardless of which
/// list holds it. The surviving (including) definition is marked
/// `is_override = True`.
fn absorb_classes(includer: &mut PBmmSchema, incoming: Vec<PBmmClass>, list: ClassList) {
    for class in incoming {
        let name = class.name().to_owned();
        if let Some(existing) = find_class_mut(includer, &name) {
            existing.set_is_override(true);
            continue;
        }
        match list {
            ClassList::Primitive => includer.primitive_types.push(class),
            ClassList::Definitions => includer.class_definitions.push(class),
        }
    }
}

/// The schema's own definition of `name`, in either class list.
fn find_class_mut<'a>(schema: &'a mut PBmmSchema, name: &str) -> Option<&'a mut PBmmClass> {
    schema
        .primitive_types
        .iter_mut()
        .chain(schema.class_definitions.iter_mut())
        .find(|class| class.name() == name)
}

/// Merges an included package tree into the includer's, recursively.
///
/// `P_BMM_PACKAGE.merge`: "Merge packages and classes from other (from an
/// included `P_BMM_SCHEMA`) into this package" (class doc §Functions). A package
/// the includer does not declare is taken whole; a package both declare keeps the
/// includer's `name`/`documentation` and gains the included package's classes
/// (in included order, after the includer's own) and child packages.
fn merge_packages(
    target: &mut BTreeMap<String, PBmmPackage>,
    source: BTreeMap<String, PBmmPackage>,
) {
    for (key, package) in source {
        match target.get_mut(&key) {
            None => {
                target.insert(key, package);
            }
            Some(existing) => {
                for class in package.classes {
                    if !existing.classes.contains(&class) {
                        existing.classes.push(class);
                    }
                }
                merge_packages(&mut existing.packages, package.packages);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::panic_in_result_fn,
        reason = "the Book ch11 test shape: `?` propagates the read/resolve/model plumbing while the assertions ARE the test — an assertion panic is how these tests fail"
    )]
    use std::collections::BTreeMap;
    use std::fmt::Write;

    use crate::bmm_persistence::error::PBmmReadError;
    use crate::bmm_persistence::include_resolution::resolve_includes;
    use crate::bmm_persistence::p_bmm_class::PBmmClass;
    use crate::bmm_persistence::p_bmm_schema::PBmmSchema;
    use crate::bmm_persistence::reader::read_schema;

    /// Reads one schema fixture.
    fn read(src: &str) -> PBmmSchema {
        read_schema(src).expect("the fixture reads")
    }

    /// The supplied-schemas map, keyed by each schema's own id.
    fn loaded(schemas: Vec<PBmmSchema>) -> BTreeMap<String, PBmmSchema> {
        schemas
            .into_iter()
            .map(|schema| (schema.schema_id(), schema))
            .collect()
    }

    /// A schema named `name` defining `classes`, including `includes`.
    fn schema_src(name: &str, includes: &[&str], classes: &[(&str, &str)]) -> String {
        let mut src = format!(
            r#"
            bmm_version = <"2.4">
            rm_publisher = <"openehr">
            schema_name = <"{name}">
            rm_release = <"1.0.2">
        "#
        );
        if !includes.is_empty() {
            src.push_str("includes = <\n");
            for (index, id) in includes.iter().enumerate() {
                let key = index + 1;
                let _ = writeln!(src, "[\"{key}\"] = < id = <\"{id}\"> >");
            }
            src.push_str(">\n");
        }
        let names: Vec<String> = classes
            .iter()
            .map(|(class, _)| format!("\"{class}\""))
            .collect();
        let _ = writeln!(
            src,
            "packages = <\n[\"org.openehr.{name}\"] = <\nname = <\"org.openehr.{name}\">\nclasses = <{}>\n>\n>",
            names.join(", ")
        );
        src.push_str("class_definitions = <\n");
        for (class, documentation) in classes {
            let _ = writeln!(
                src,
                "[\"{class}\"] = < name = <\"{class}\"> documentation = <\"{documentation}\"> >"
            );
        }
        src.push_str(">\n");
        src
    }

    #[test]
    fn inclusion_is_transitive() -> Result<(), PBmmReadError> {
        let leaf = read(&schema_src("primitive_types", &[], &[("Any", "leaf")]));
        let middle = read(&schema_src(
            "basic_types",
            &["openehr_primitive_types_1.0.2"],
            &[("DV_TEXT", "middle")],
        ));
        let root = read(&schema_src(
            "structures",
            &["openehr_basic_types_1.0.2"],
            &[("ELEMENT", "root")],
        ));
        let merged = resolve_includes(root, &loaded(vec![leaf, middle]))?;
        let mut names: Vec<&str> = merged
            .class_definitions
            .iter()
            .map(PBmmClass::name)
            .collect();
        names.sort_unstable();
        assert_eq!(names, ["Any", "DV_TEXT", "ELEMENT"]);
        // The merged schema keeps the root's own identity and include record.
        assert_eq!(merged.schema_id(), "openehr_structures_1.0.2");
        assert_eq!(merged.includes.as_ref().map(BTreeMap::len), Some(1));
        Ok(())
    }

    #[test]
    fn the_includer_wins_a_name_collision_and_is_marked_as_an_override() -> Result<(), PBmmReadError>
    {
        let included = read(&schema_src(
            "primitive_types",
            &[],
            &[("Any", "from the included schema")],
        ));
        let root = read(&schema_src(
            "structures",
            &["openehr_primitive_types_1.0.2"],
            &[("Any", "from the including schema")],
        ));
        let merged = resolve_includes(root, &loaded(vec![included]))?;
        assert_eq!(merged.class_definitions.len(), 1);
        let class = merged
            .class_definitions
            .first()
            .expect("the surviving definition");
        assert_eq!(class.documentation(), Some("from the including schema"));
        assert!(class.is_override());
        Ok(())
    }

    #[test]
    fn every_class_keeps_the_schema_id_that_defined_it() -> Result<(), PBmmReadError> {
        let included = read(&schema_src("primitive_types", &[], &[("Any", "leaf")]));
        let root = read(&schema_src(
            "structures",
            &["openehr_primitive_types_1.0.2"],
            &[("ELEMENT", "root")],
        ));
        let merged = resolve_includes(root, &loaded(vec![included]))?;
        let sources: BTreeMap<&str, &str> = merged
            .class_definitions
            .iter()
            .map(|class| (class.name(), class.source_schema_id()))
            .collect();
        assert_eq!(
            sources.get("Any").copied(),
            Some("openehr_primitive_types_1.0.2")
        );
        assert_eq!(
            sources.get("ELEMENT").copied(),
            Some("openehr_structures_1.0.2")
        );
        Ok(())
    }

    #[test]
    fn packages_merge_recursively() -> Result<(), PBmmReadError> {
        let included = read(
            r#"
            bmm_version = <"2.4">
            rm_publisher = <"openehr">
            schema_name = <"leaf">
            rm_release = <"1.0.2">
            packages = <
                ["org.openehr.rm"] = <
                    name = <"org.openehr.rm">
                    classes = <"Any">
                    packages = <
                        ["support"] = <
                            name = <"support">
                            classes = <"TERMINOLOGY_ID">
                        >
                    >
                >
            >
            class_definitions = <
                ["Any"] = < name = <"Any"> >
                ["TERMINOLOGY_ID"] = < name = <"TERMINOLOGY_ID"> >
            >
        "#,
        );
        let root = read(
            r#"
            bmm_version = <"2.4">
            rm_publisher = <"openehr">
            schema_name = <"root">
            rm_release = <"1.0.2">
            includes = <
                ["1"] = < id = <"openehr_leaf_1.0.2"> >
            >
            packages = <
                ["org.openehr.rm"] = <
                    name = <"org.openehr.rm">
                    classes = <"ELEMENT">
                    packages = <
                        ["composition"] = <
                            name = <"composition">
                            classes = <"COMPOSITION">
                        >
                    >
                >
            >
            class_definitions = <
                ["ELEMENT"] = < name = <"ELEMENT"> >
                ["COMPOSITION"] = < name = <"COMPOSITION"> >
            >
        "#,
        );
        let merged = resolve_includes(root, &loaded(vec![included]))?;
        let rm = merged
            .packages
            .get("org.openehr.rm")
            .expect("the shared top-level package");
        assert_eq!(rm.classes, ["ELEMENT".to_owned(), "Any".to_owned()]);
        assert_eq!(rm.packages.len(), 2);
        assert!(rm.packages.contains_key("composition"));
        assert!(rm.packages.contains_key("support"));
        Ok(())
    }

    #[test]
    fn a_missing_include_is_refused() {
        let root = read(&schema_src("structures", &["openehr_absent_9.9.9"], &[]));
        let error = resolve_includes(root, &BTreeMap::new()).expect_err("the include is absent");
        assert_eq!(
            error,
            PBmmReadError::MissingInclude {
                requester: "openehr_structures_1.0.2".to_owned(),
                id: "openehr_absent_9.9.9".to_owned(),
            }
        );
    }

    #[test]
    fn an_inclusion_cycle_is_refused() {
        let first = read(&schema_src("first", &["openehr_second_1.0.2"], &[]));
        let second = read(&schema_src("second", &["openehr_first_1.0.2"], &[]));
        let error = resolve_includes(first.clone(), &loaded(vec![first, second]))
            .expect_err("the graph is cyclic");
        assert!(
            matches!(error, PBmmReadError::IncludeCycle { .. }),
            "expected a cycle refusal, got {error:?}"
        );
    }
}
