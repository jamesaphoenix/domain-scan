//! Query engine: loads .scm query files, compiles tree-sitter queries lazily,
//! and dispatches captures to IR types.

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use tree_sitter::{Node, Query, QueryCursor, QueryMatch, Tree};

use crate::ir::*;
use crate::DomainScanError;

// ---------------------------------------------------------------------------
// Rust .scm sources (embedded at compile time)
// ---------------------------------------------------------------------------

const RS_TRAITS_SCM: &str = include_str!("../queries/rust/traits.scm");
const RS_IMPLS_SCM: &str = include_str!("../queries/rust/impls.scm");
const RS_FUNCTIONS_SCM: &str = include_str!("../queries/rust/functions.scm");
const RS_TYPES_SCM: &str = include_str!("../queries/rust/types.scm");
const RS_IMPORTS_SCM: &str = include_str!("../queries/rust/imports.scm");
const RS_SERVICES_SCM: &str = include_str!("../queries/rust/services.scm");
const RS_SCHEMAS_SCM: &str = include_str!("../queries/rust/schemas.scm");

// ---------------------------------------------------------------------------
// Go .scm sources (embedded at compile time)
// ---------------------------------------------------------------------------

const GO_INTERFACES_SCM: &str = include_str!("../queries/go/interfaces.scm");
const GO_STRUCTS_SCM: &str = include_str!("../queries/go/structs.scm");
const GO_FUNCTIONS_SCM: &str = include_str!("../queries/go/functions.scm");
const GO_METHODS_SCM: &str = include_str!("../queries/go/methods.scm");
const GO_IMPORTS_SCM: &str = include_str!("../queries/go/imports.scm");
#[allow(dead_code)]
const GO_SERVICES_SCM: &str = include_str!("../queries/go/services.scm");
const GO_SCHEMAS_SCM: &str = include_str!("../queries/go/schemas.scm");

// ---------------------------------------------------------------------------
// Python .scm sources (embedded at compile time)
// ---------------------------------------------------------------------------

const PY_CLASSES_SCM: &str = include_str!("../queries/python/classes.scm");
const PY_FUNCTIONS_SCM: &str = include_str!("../queries/python/functions.scm");
#[allow(dead_code)]
const PY_PROTOCOLS_SCM: &str = include_str!("../queries/python/protocols.scm");
#[allow(dead_code)]
const PY_ABSTRACT_SCM: &str = include_str!("../queries/python/abstract.scm");
const PY_IMPORTS_SCM: &str = include_str!("../queries/python/imports.scm");
#[allow(dead_code)]
const PY_DECORATORS_SCM: &str = include_str!("../queries/python/decorators.scm");
const PY_SERVICES_SCM: &str = include_str!("../queries/python/services.scm");
const PY_SCHEMAS_SCM: &str = include_str!("../queries/python/schemas.scm");

// ---------------------------------------------------------------------------
// Java .scm sources (embedded at compile time)
// ---------------------------------------------------------------------------

const JV_INTERFACES_SCM: &str = include_str!("../queries/java/interfaces.scm");
const JV_CLASSES_SCM: &str = include_str!("../queries/java/classes.scm");
#[allow(dead_code)]
const JV_METHODS_SCM: &str = include_str!("../queries/java/methods.scm");
const JV_IMPORTS_SCM: &str = include_str!("../queries/java/imports.scm");
const JV_SERVICES_SCM: &str = include_str!("../queries/java/services.scm");
const JV_SCHEMAS_SCM: &str = include_str!("../queries/java/schemas.scm");

// ---------------------------------------------------------------------------
// Kotlin .scm sources (embedded at compile time)
// ---------------------------------------------------------------------------

const KT_INTERFACES_SCM: &str = include_str!("../queries/kotlin/interfaces.scm");
const KT_CLASSES_SCM: &str = include_str!("../queries/kotlin/classes.scm");
#[allow(dead_code)]
const KT_METHODS_SCM: &str = include_str!("../queries/kotlin/methods.scm");
const KT_IMPORTS_SCM: &str = include_str!("../queries/kotlin/imports.scm");
const KT_SERVICES_SCM: &str = include_str!("../queries/kotlin/services.scm");
const KT_SCHEMAS_SCM: &str = include_str!("../queries/kotlin/schemas.scm");

// ---------------------------------------------------------------------------
// Scala .scm sources (embedded at compile time)
// ---------------------------------------------------------------------------

const SC_TRAITS_SCM: &str = include_str!("../queries/scala/traits.scm");
const SC_CLASSES_SCM: &str = include_str!("../queries/scala/classes.scm");
#[allow(dead_code)]
const SC_METHODS_SCM: &str = include_str!("../queries/scala/methods.scm");
const SC_IMPORTS_SCM: &str = include_str!("../queries/scala/imports.scm");

// ---------------------------------------------------------------------------
// TypeScript .scm sources (embedded at compile time)
// ---------------------------------------------------------------------------

const TS_INTERFACES_SCM: &str = include_str!("../queries/typescript/interfaces.scm");
const TS_CLASSES_SCM: &str = include_str!("../queries/typescript/classes.scm");
#[allow(dead_code)]
const TS_METHODS_SCM: &str = include_str!("../queries/typescript/methods.scm");
const TS_FUNCTIONS_SCM: &str = include_str!("../queries/typescript/functions.scm");
const TS_TYPES_SCM: &str = include_str!("../queries/typescript/types.scm");
const TS_IMPORTS_SCM: &str = include_str!("../queries/typescript/imports.scm");
const TS_EXPORTS_SCM: &str = include_str!("../queries/typescript/exports.scm");
const TS_SERVICES_SCM: &str = include_str!("../queries/typescript/services.scm");
const TS_SCHEMAS_SCM: &str = include_str!("../queries/typescript/schemas.scm");

// ---------------------------------------------------------------------------
// Lazy-compiled query statics
// ---------------------------------------------------------------------------

// Rust query statics
static RS_TRAITS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static RS_IMPLS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static RS_FUNCTIONS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static RS_TYPES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static RS_IMPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static RS_SERVICES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static RS_SCHEMAS_Q: OnceLock<Result<Query, String>> = OnceLock::new();

// Go query statics
static GO_INTERFACES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static GO_STRUCTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static GO_FUNCTIONS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static GO_METHODS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static GO_IMPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static GO_SCHEMAS_Q: OnceLock<Result<Query, String>> = OnceLock::new();

// Python query statics
static PY_CLASSES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static PY_FUNCTIONS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
#[allow(dead_code)]
static PY_PROTOCOLS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static PY_IMPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static PY_SERVICES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static PY_SCHEMAS_Q: OnceLock<Result<Query, String>> = OnceLock::new();

// Java query statics
static JV_INTERFACES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static JV_CLASSES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static JV_IMPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static JV_SERVICES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static JV_SCHEMAS_Q: OnceLock<Result<Query, String>> = OnceLock::new();

// Kotlin query statics
static KT_INTERFACES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static KT_CLASSES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static KT_IMPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static KT_SERVICES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static KT_METHODS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static KT_SCHEMAS_Q: OnceLock<Result<Query, String>> = OnceLock::new();

// Scala query statics
static SC_TRAITS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static SC_CLASSES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static SC_IMPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();

// TypeScript query statics
static TS_INTERFACES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static TS_CLASSES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static TS_FUNCTIONS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static TS_TYPES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static TS_IMPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static TS_EXPORTS_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static TS_SERVICES_Q: OnceLock<Result<Query, String>> = OnceLock::new();
static TS_SCHEMAS_Q: OnceLock<Result<Query, String>> = OnceLock::new();

fn compile_ts_query(source: &str) -> Result<Query, String> {
    let lang = tree_sitter_typescript::language_typescript();
    Query::new(&lang, source).map_err(|e| format!("{e}"))
}

fn get_ts_query(
    lock: &'static OnceLock<Result<Query, String>>,
    source: &str,
) -> Result<&'static Query, DomainScanError> {
    let result = lock.get_or_init(|| compile_ts_query(source));
    result
        .as_ref()
        .map_err(|e| DomainScanError::QueryCompile(e.clone()))
}

fn compile_rs_query(source: &str) -> Result<Query, String> {
    let lang = tree_sitter_rust::language();
    Query::new(&lang, source).map_err(|e| format!("{e}"))
}

fn get_rs_query(
    lock: &'static OnceLock<Result<Query, String>>,
    source: &str,
) -> Result<&'static Query, DomainScanError> {
    let result = lock.get_or_init(|| compile_rs_query(source));
    result
        .as_ref()
        .map_err(|e| DomainScanError::QueryCompile(e.clone()))
}

fn compile_go_query(source: &str) -> Result<Query, String> {
    let lang = tree_sitter_go::language();
    Query::new(&lang, source).map_err(|e| format!("{e}"))
}

fn get_go_query(
    lock: &'static OnceLock<Result<Query, String>>,
    source: &str,
) -> Result<&'static Query, DomainScanError> {
    let result = lock.get_or_init(|| compile_go_query(source));
    result
        .as_ref()
        .map_err(|e| DomainScanError::QueryCompile(e.clone()))
}

fn compile_py_query(source: &str) -> Result<Query, String> {
    let lang = tree_sitter_python::language();
    Query::new(&lang, source).map_err(|e| format!("{e}"))
}

fn get_py_query(
    lock: &'static OnceLock<Result<Query, String>>,
    source: &str,
) -> Result<&'static Query, DomainScanError> {
    let result = lock.get_or_init(|| compile_py_query(source));
    result
        .as_ref()
        .map_err(|e| DomainScanError::QueryCompile(e.clone()))
}

fn compile_jv_query(source: &str) -> Result<Query, String> {
    let lang = tree_sitter_java::language();
    Query::new(&lang, source).map_err(|e| format!("{e}"))
}

fn get_jv_query(
    lock: &'static OnceLock<Result<Query, String>>,
    source: &str,
) -> Result<&'static Query, DomainScanError> {
    let result = lock.get_or_init(|| compile_jv_query(source));
    result
        .as_ref()
        .map_err(|e| DomainScanError::QueryCompile(e.clone()))
}

fn compile_kt_query(source: &str) -> Result<Query, String> {
    let lang = crate::parser::kotlin_language();
    Query::new(&lang, source).map_err(|e| format!("{e}"))
}

fn get_kt_query(
    lock: &'static OnceLock<Result<Query, String>>,
    source: &str,
) -> Result<&'static Query, DomainScanError> {
    let result = lock.get_or_init(|| compile_kt_query(source));
    result
        .as_ref()
        .map_err(|e| DomainScanError::QueryCompile(e.clone()))
}

fn compile_sc_query(source: &str) -> Result<Query, String> {
    let lang = crate::parser::scala_language();
    Query::new(&lang, source).map_err(|e| format!("{e}"))
}

fn get_sc_query(
    lock: &'static OnceLock<Result<Query, String>>,
    source: &str,
) -> Result<&'static Query, DomainScanError> {
    let result = lock.get_or_init(|| compile_sc_query(source));
    result
        .as_ref()
        .map_err(|e| DomainScanError::QueryCompile(e.clone()))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extract all structural entities from a parsed tree-sitter tree.
pub fn extract(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    language: Language,
    build_status: BuildStatus,
) -> Result<IrFile, DomainScanError> {
    let content_hash = crate::content_hash(source);
    let mut ir = IrFile::new(path.to_path_buf(), language, content_hash, build_status);

    match language {
        Language::TypeScript => extract_typescript(tree, source, path, &mut ir)?,
        Language::Rust => extract_rust(tree, source, path, &mut ir)?,
        Language::Go => extract_go(tree, source, path, &mut ir)?,
        Language::Python => extract_python(tree, source, path, &mut ir)?,
        Language::Java => extract_java(tree, source, path, &mut ir)?,
        Language::Kotlin => extract_kotlin(tree, source, path, &mut ir)?,
        Language::Scala => extract_scala(tree, source, path, &mut ir)?,
        other => return Err(DomainScanError::UnsupportedLanguage(other)),
    }

    Ok(ir)
}

// ---------------------------------------------------------------------------
// TypeScript extraction
// ---------------------------------------------------------------------------

fn extract_typescript(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    ir: &mut IrFile,
) -> Result<(), DomainScanError> {
    ir.interfaces = extract_ts_interfaces(tree, source, path)?;
    ir.classes = extract_ts_classes(tree, source, path)?;
    ir.functions = extract_ts_functions(tree, source, path)?;
    ir.type_aliases = extract_ts_types(tree, source, path)?;
    ir.imports = extract_ts_imports(tree, source)?;
    ir.exports = extract_ts_exports(tree, source)?;
    ir.services = extract_ts_services(tree, source, path)?;
    ir.schemas = extract_ts_schemas(tree, source, path)?;
    Ok(())
}

// --- Interfaces ---

fn extract_ts_interfaces(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<InterfaceDef>, DomainScanError> {
    let query = get_ts_query(&TS_INTERFACES_Q, TS_INTERFACES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut interfaces = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "interface.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "interface.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = ts_visibility(def_node);
        let generics = extract_ts_generics(def_node, source);
        let extends = extract_ts_extends(def_node, source);

        // Extract methods and properties from interface body
        let body_node = def_node.child_by_field_name("body");
        let methods = body_node
            .map(|b| extract_ts_method_signatures(b, source))
            .unwrap_or_default();
        let properties = body_node
            .map(|b| extract_ts_properties(b, source))
            .unwrap_or_default();

        interfaces.push(InterfaceDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            methods,
            properties,
            language_kind: InterfaceKind::Interface,
            decorators: Vec::new(),
        });
    }

    Ok(interfaces)
}

// --- Classes ---

fn extract_ts_classes(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ClassDef>, DomainScanError> {
    let query = get_ts_query(&TS_CLASSES_Q, TS_CLASSES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut classes = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "class.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "class.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = ts_visibility(def_node);
        let is_abstract = def_node.kind() == "abstract_class_declaration";
        let generics = extract_ts_generics(def_node, source);
        let decorators = extract_ts_decorators(def_node, source);

        // Extract extends and implements from heritage
        let (extends, implements) = extract_ts_class_heritage(def_node, source);

        // Extract methods and properties from class body
        let body_node = def_node.child_by_field_name("body");
        let mut methods = body_node
            .map(|b| extract_ts_class_methods(b, source, path))
            .unwrap_or_default();
        let properties = body_node
            .map(|b| extract_ts_class_properties(b, source))
            .unwrap_or_default();

        // Set owner for all methods
        for method in &mut methods {
            method.owner = Some(name.clone());
        }

        classes.push(ClassDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            implements,
            methods,
            properties,
            is_abstract,
            decorators,
        });
    }

    Ok(classes)
}

// --- Functions ---

fn extract_ts_functions(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<FunctionDef>, DomainScanError> {
    let query = get_ts_query(&TS_FUNCTIONS_Q, TS_FUNCTIONS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut functions = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "function.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "function.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = ts_visibility(def_node);

        // For function declarations, extract from def_node directly.
        // For arrow/expression functions, extract from the value node.
        let func_node = find_capture(&m, query, "function.value").unwrap_or(def_node);

        let is_async = has_async_keyword(func_node);
        let is_generator = has_child_of_kind(func_node, "*");

        let params_node = func_node.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_ts_parameters(p, source))
            .unwrap_or_default();

        let return_type = func_node
            .child_by_field_name("return_type")
            .map(|rt| extract_type_annotation_text(rt, source));

        let decorators = extract_ts_decorators(def_node, source);

        functions.push(FunctionDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async,
            is_generator,
            parameters,
            return_type,
            decorators,
        });
    }

    Ok(functions)
}

// --- Type Aliases ---

fn extract_ts_types(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<TypeAlias>, DomainScanError> {
    let query = get_ts_query(&TS_TYPES_Q, TS_TYPES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut types = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "type.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "type.def") else {
            continue;
        };
        let Some(value_node) = find_capture(&m, query, "type.value") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = ts_visibility(def_node);
        let target = node_text(value_node, source);
        let generics = extract_ts_generics(def_node, source);

        types.push(TypeAlias {
            name,
            file: path.to_path_buf(),
            span,
            target,
            generics,
            visibility,
        });
    }

    Ok(types)
}

// --- Imports ---

fn extract_ts_imports(
    tree: &Tree,
    source: &[u8],
) -> Result<Vec<ImportDef>, DomainScanError> {
    let query = get_ts_query(&TS_IMPORTS_Q, TS_IMPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut imports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(source_node) = find_capture(&m, query, "import.source") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "import.def") else {
            continue;
        };

        let source_str = strip_quotes(&node_text(source_node, source));
        let span = node_span(def_node);

        // Walk the import statement to extract symbols
        let (symbols, is_wildcard) = extract_ts_import_symbols(def_node, source);

        imports.push(ImportDef {
            source: source_str,
            symbols,
            is_wildcard,
            span,
        });
    }

    Ok(imports)
}

// --- Exports ---

fn extract_ts_exports(
    tree: &Tree,
    source: &[u8],
) -> Result<Vec<ExportDef>, DomainScanError> {
    let query = get_ts_query(&TS_EXPORTS_Q, TS_EXPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut exports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(def_node) = find_capture(&m, query, "export.def") else {
            continue;
        };

        let span = node_span(def_node);

        // Check if it's a default export (anonymous keyword child)
        let mut walk = def_node.walk();
        let is_default = def_node.children(&mut walk).any(|c| c.kind() == "default");

        // Check for re-export source: export { ... } from '...'
        let re_export_source = def_node
            .child_by_field_name("source")
            .map(|s| strip_quotes(&node_text(s, source)));

        // Extract exported names
        let exported =
            extract_ts_export_names(def_node, source, is_default, &re_export_source);
        exports.extend(exported.into_iter().map(|(name, kind)| ExportDef {
            name,
            kind,
            source: re_export_source.clone(),
            span: span.clone(),
        }));
    }

    Ok(exports)
}

// --- Services ---

fn extract_ts_services(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ServiceDef>, DomainScanError> {
    let query = get_ts_query(&TS_SERVICES_Q, TS_SERVICES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut services = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "service.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "service.def") else {
            continue;
        };

        // Deduplicate: class with multiple decorators produces multiple matches
        if !seen.insert(def_node.start_byte()) {
            continue;
        }

        let decorators = extract_ts_decorators(def_node, source);

        // Determine service kind from decorators
        let Some(kind) = classify_service_kind(&decorators) else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);

        // Extract methods
        let body_node = def_node.child_by_field_name("body");
        let mut methods = body_node
            .map(|b| extract_ts_class_methods(b, source, path))
            .unwrap_or_default();
        for method in &mut methods {
            method.owner = Some(name.clone());
        }

        // Extract dependencies from constructor parameters
        let dependencies = body_node
            .map(|b| extract_ts_constructor_deps(b, source))
            .unwrap_or_default();

        // Extract routes from method decorators
        let routes = extract_ts_routes(&methods);

        services.push(ServiceDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            methods,
            dependencies,
            decorators,
            routes,
        });
    }

    Ok(services)
}

// --- Schemas ---

fn extract_ts_schemas(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<SchemaDef>, DomainScanError> {
    let query = get_ts_query(&TS_SCHEMAS_Q, TS_SCHEMAS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut schemas = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "schema.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "schema.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = ts_visibility(def_node);

        // Determine schema kind and framework from captures
        let (kind, framework, mut table_name) =
            if let Some(obj_node) = find_capture(&m, query, "schema.object") {
                let obj = node_text(obj_node, source);
                let prop = find_capture(&m, query, "schema.property")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                classify_schema_member(&obj, &prop)
            } else if let Some(fn_node) = find_capture(&m, query, "schema.function") {
                let fn_name = node_text(fn_node, source);
                classify_schema_function(&fn_name)
            } else {
                continue;
            };

        let Some(kind) = kind else {
            continue; // Not a recognized schema pattern
        };

        // For Drizzle, capture the table name from the first string argument
        if table_name.is_none() {
            table_name = find_capture(&m, query, "schema.table_name")
                .map(|n| strip_quotes(&node_text(n, source)));
        }

        // Extract fields from arguments
        let fields_node = find_capture(&m, query, "schema.fields");
        let fields_source_text = fields_node
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let fields = parse_schema_fields(&fields_source_text);

        schemas.push(SchemaDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            fields,
            source_framework: framework,
            table_name,
            derives: Vec::new(), // Not applicable for TypeScript
            visibility,
        });
    }

    Ok(schemas)
}

// ===========================================================================
// Rust extraction
// ===========================================================================

fn extract_rust(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    ir: &mut IrFile,
) -> Result<(), DomainScanError> {
    ir.interfaces = extract_rs_traits(tree, source, path)?;
    ir.implementations = extract_rs_impls(tree, source, path)?;
    ir.functions = extract_rs_functions(tree, source, path)?;
    let (classes, type_aliases) = extract_rs_types(tree, source, path)?;
    ir.classes = classes;
    ir.type_aliases = type_aliases;
    ir.imports = extract_rs_imports(tree, source)?;
    ir.schemas = extract_rs_schemas(tree, source, path)?;
    ir.services = extract_rs_services(tree, source, path)?;
    Ok(())
}

// --- Rust Traits ---

fn extract_rs_traits(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<InterfaceDef>, DomainScanError> {
    let query = get_rs_query(&RS_TRAITS_Q, RS_TRAITS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut traits = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "trait.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "trait.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "trait.body");

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = rs_visibility(def_node);
        let generics = extract_rs_generics(def_node, source);
        let extends = extract_rs_trait_bounds(def_node, source);

        // Extract method signatures from trait body
        let methods = body_node
            .map(|b| extract_rs_trait_methods(b, source))
            .unwrap_or_default();

        traits.push(InterfaceDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            methods,
            properties: Vec::new(),
            language_kind: InterfaceKind::Trait,
            decorators: Vec::new(),
        });
    }

    Ok(traits)
}

fn extract_rs_trait_methods(body: Node<'_>, source: &[u8]) -> Vec<MethodSignature> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        // function_signature_item = method without body (required)
        // function_item = method with default impl
        let (is_sig, node) = match child.kind() {
            "function_signature_item" => (true, child),
            "function_item" => (false, child),
            _ => continue,
        };

        let name = node
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let span = node_span(node);

        let params_node = node.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_rs_parameters(p, source))
            .unwrap_or_default();

        let return_type = node
            .child_by_field_name("return_type")
            .map(|rt| node_text(rt, source));

        // Check for async keyword (Rust uses function_modifiers > async)
        let is_async = rs_is_async(node);

        methods.push(MethodSignature {
            name,
            span,
            is_async,
            parameters,
            return_type,
            has_default: !is_sig,
        });
    }

    methods
}

// --- Rust Impls ---

fn extract_rs_impls(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ImplDef>, DomainScanError> {
    let query = get_rs_query(&RS_IMPLS_Q, RS_IMPLS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut impls = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(target_node) = find_capture(&m, query, "impl.target") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "impl.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "impl.body");

        let target = node_text(target_node, source);
        let span = node_span(def_node);

        // Check for trait impl: `impl Trait for Type`
        let trait_name = def_node
            .child_by_field_name("trait")
            .map(|t| node_text(t, source));

        // Extract methods from impl body
        let methods = body_node
            .map(|b| extract_rs_impl_methods(b, source, path))
            .unwrap_or_default();

        impls.push(ImplDef {
            target,
            trait_name,
            file: path.to_path_buf(),
            span,
            methods,
        });
    }

    Ok(impls)
}

fn extract_rs_impl_methods(body: Node<'_>, source: &[u8], path: &Path) -> Vec<MethodDef> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "function_item" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();

        let span = node_span(child);
        let visibility = rs_visibility(child);
        let is_async = rs_is_async(child);

        let params_node = child.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_rs_parameters(p, source))
            .unwrap_or_default();

        let return_type = child
            .child_by_field_name("return_type")
            .map(|rt| node_text(rt, source));

        methods.push(MethodDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async,
            is_static: false,
            is_generator: false,
            parameters,
            return_type,
            decorators: Vec::new(),
            owner: None, // Set by caller
            implements: None,
        });
    }

    methods
}

// --- Rust Functions ---

fn extract_rs_functions(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<FunctionDef>, DomainScanError> {
    let query = get_rs_query(&RS_FUNCTIONS_Q, RS_FUNCTIONS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut functions = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "function.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "function.def") else {
            continue;
        };

        // Only capture top-level functions (not inside impl/trait bodies)
        if let Some(parent) = def_node.parent() {
            if parent.kind() == "declaration_list" {
                continue;
            }
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = rs_visibility(def_node);
        let is_async = rs_is_async(def_node);

        let params_node = def_node.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_rs_parameters(p, source))
            .unwrap_or_default();

        let return_type = def_node
            .child_by_field_name("return_type")
            .map(|rt| node_text(rt, source));

        functions.push(FunctionDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async,
            is_generator: false,
            parameters,
            return_type,
            decorators: Vec::new(),
        });
    }

    Ok(functions)
}

// --- Rust Types (struct, enum, type alias) ---

fn extract_rs_types(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<(Vec<ClassDef>, Vec<TypeAlias>), DomainScanError> {
    let query = get_rs_query(&RS_TYPES_Q, RS_TYPES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut classes = Vec::new();
    let mut type_aliases = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "type.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "type.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = rs_visibility(def_node);
        let generics = extract_rs_generics(def_node, source);

        match def_node.kind() {
            "struct_item" => {
                let properties = extract_rs_struct_fields(def_node, source);
                classes.push(ClassDef {
                    name,
                    file: path.to_path_buf(),
                    span,
                    visibility,
                    generics,
                    extends: None,
                    implements: Vec::new(),
                    methods: Vec::new(),
                    properties,
                    is_abstract: false,
                    decorators: extract_rs_attributes(def_node, source),
                });
            }
            "enum_item" => {
                classes.push(ClassDef {
                    name,
                    file: path.to_path_buf(),
                    span,
                    visibility,
                    generics,
                    extends: None,
                    implements: Vec::new(),
                    methods: Vec::new(),
                    properties: Vec::new(),
                    is_abstract: false,
                    decorators: extract_rs_attributes(def_node, source),
                });
            }
            "type_item" => {
                let target = find_capture(&m, query, "type.value")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                type_aliases.push(TypeAlias {
                    name,
                    file: path.to_path_buf(),
                    span,
                    target,
                    generics,
                    visibility,
                });
            }
            _ => {}
        }
    }

    Ok((classes, type_aliases))
}

// --- Rust Imports ---

fn extract_rs_imports(
    tree: &Tree,
    source: &[u8],
) -> Result<Vec<ImportDef>, DomainScanError> {
    let query = get_rs_query(&RS_IMPORTS_Q, RS_IMPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut imports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(path_node) = find_capture(&m, query, "import.path") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "import.def") else {
            continue;
        };

        let source_str = node_text(path_node, source);
        let span = node_span(def_node);
        let is_wildcard = source_str.contains('*');

        // Parse use path into symbols
        let symbols = parse_rs_use_path(&source_str);

        imports.push(ImportDef {
            source: source_str,
            symbols,
            is_wildcard,
            span,
        });
    }

    Ok(imports)
}

// --- Rust Schemas (serde derive structs) ---

fn extract_rs_schemas(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<SchemaDef>, DomainScanError> {
    let query = get_rs_query(&RS_SCHEMAS_Q, RS_SCHEMAS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut schemas = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "schema.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "schema.def") else {
            continue;
        };

        let derives = extract_rs_attributes(def_node, source);

        // Only include if it has Serialize or Deserialize derive
        let has_serde = derives.iter().any(|d| {
            d.contains("Serialize") || d.contains("Deserialize")
        });
        if !has_serde {
            continue;
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = rs_visibility(def_node);
        let fields = extract_rs_struct_schema_fields(def_node, source);

        schemas.push(SchemaDef {
            name,
            file: path.to_path_buf(),
            span,
            kind: SchemaKind::DataTransfer,
            fields,
            source_framework: "serde".to_string(),
            table_name: None,
            derives,
            visibility,
        });
    }

    Ok(schemas)
}

// --- Rust Services (detect from attribute patterns) ---

fn extract_rs_services(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ServiceDef>, DomainScanError> {
    let query = get_rs_query(&RS_SERVICES_Q, RS_SERVICES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut services = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(target_node) = find_capture(&m, query, "service.target") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "service.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "service.body");

        let attrs = extract_rs_attributes(def_node, source);

        // Detect tonic/gRPC trait impls or actix/axum route handlers
        let trait_name = def_node
            .child_by_field_name("trait")
            .map(|t| node_text(t, source));

        let kind = if attrs.iter().any(|a| a.contains("tonic") || a.contains("async_trait")) {
            if trait_name.is_some() {
                Some(ServiceKind::GrpcService)
            } else {
                None
            }
        } else {
            None
        };

        let Some(kind) = kind else {
            continue;
        };

        let name = node_text(target_node, source);
        let span = node_span(def_node);

        let methods = body_node
            .map(|b| extract_rs_impl_methods(b, source, path))
            .unwrap_or_default();

        services.push(ServiceDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            methods,
            dependencies: Vec::new(),
            decorators: attrs,
            routes: Vec::new(),
        });
    }

    Ok(services)
}

// --- Rust Helpers ---

fn rs_visibility(node: Node<'_>) -> Visibility {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            // Check child structure for pub(crate), pub(super), etc.
            let mut inner = child.walk();
            let has_restriction = child.named_children(&mut inner).any(|c| {
                c.kind() == "crate" || c.kind() == "super" || c.kind() == "self"
                    || c.kind() == "identifier"
            });
            return if has_restriction {
                Visibility::Crate
            } else {
                Visibility::Public
            };
        }
    }
    Visibility::Private
}

fn rs_is_async(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "function_modifiers" {
            let mut inner = child.walk();
            return child.children(&mut inner).any(|c| c.kind() == "async");
        }
    }
    false
}

fn extract_rs_generics(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let Some(type_params) = node.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut generics = Vec::new();
    let mut cursor = type_params.walk();
    for child in type_params.named_children(&mut cursor) {
        if child.kind() == "type_identifier" || child.kind() == "lifetime" || child.kind() == "constrained_type_parameter" {
            generics.push(node_text(child, source));
        }
    }
    generics
}

fn extract_rs_trait_bounds(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut extends = Vec::new();
    if let Some(bounds) = node.child_by_field_name("bounds") {
        let mut cursor = bounds.walk();
        for child in bounds.named_children(&mut cursor) {
            extends.push(node_text(child, source));
        }
    }
    extends
}

fn extract_rs_parameters(params_node: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut parameters = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.named_children(&mut cursor) {
        match child.kind() {
            "parameter" => {
                let name = child
                    .child_by_field_name("pattern")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| node_text(t, source));
                parameters.push(Parameter {
                    name,
                    type_annotation,
                    is_optional: false,
                    has_default: false,
                    is_rest: false,
                });
            }
            "self_parameter" => {
                parameters.push(Parameter {
                    name: "self".to_string(),
                    type_annotation: Some(node_text(child, source)),
                    is_optional: false,
                    has_default: false,
                    is_rest: false,
                });
            }
            _ => {}
        }
    }

    parameters
}

fn extract_rs_attributes(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut attrs = Vec::new();

    // Look at preceding siblings for attribute_item nodes
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "attribute_item" {
            let text = node_text(sib, source);
            let trimmed = text.trim_start_matches("#[").trim_end_matches(']');
            attrs.push(trimmed.to_string());
        } else if sib.kind() != "line_comment" && sib.kind() != "block_comment" {
            break;
        }
        prev = sib.prev_sibling();
    }
    attrs.reverse();
    attrs
}

fn extract_rs_struct_fields(node: Node<'_>, source: &[u8]) -> Vec<PropertyDef> {
    let Some(body) = node.child_by_field_name("body") else {
        return Vec::new();
    };
    let mut properties = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }
        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let type_annotation = child
            .child_by_field_name("type")
            .map(|t| node_text(t, source));
        let vis = rs_visibility(child);

        let is_optional = type_annotation
            .as_ref()
            .map(|t| t.starts_with("Option<"))
            .unwrap_or(false);

        properties.push(PropertyDef {
            name,
            type_annotation,
            is_optional,
            is_readonly: false,
            visibility: vis,
        });
    }

    properties
}

fn extract_rs_struct_schema_fields(node: Node<'_>, source: &[u8]) -> Vec<SchemaField> {
    let Some(body) = node.child_by_field_name("body") else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }
        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let type_annotation = child
            .child_by_field_name("type")
            .map(|t| node_text(t, source));

        let is_optional = type_annotation
            .as_ref()
            .map(|t| t.starts_with("Option<"))
            .unwrap_or(false);

        fields.push(SchemaField {
            name,
            type_annotation,
            is_optional,
            is_primary_key: false,
            constraints: Vec::new(),
        });
    }

    fields
}

fn parse_rs_use_path(path: &str) -> Vec<ImportedSymbol> {
    // Parse patterns like: std::collections::HashMap, crate::ir::*
    let parts: Vec<&str> = path.rsplitn(2, "::").collect();
    if parts.len() == 2 {
        let name = parts[0].trim_matches(|c| c == '{' || c == '}');
        if name == "*" {
            return vec![ImportedSymbol {
                name: "*".to_string(),
                alias: None,
                is_default: false,
                is_namespace: true,
            }];
        }
        // Handle use list: {A, B, C}
        if parts[0].starts_with('{') {
            return parts[0]
                .trim_matches(|c| c == '{' || c == '}')
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let s = s.trim();
                    ImportedSymbol {
                        name: s.to_string(),
                        alias: None,
                        is_default: false,
                        is_namespace: false,
                    }
                })
                .collect();
        }
        vec![ImportedSymbol {
            name: name.to_string(),
            alias: None,
            is_default: false,
            is_namespace: false,
        }]
    } else {
        vec![ImportedSymbol {
            name: path.to_string(),
            alias: None,
            is_default: false,
            is_namespace: false,
        }]
    }
}

// ===========================================================================
// Go extraction
// ===========================================================================

fn extract_go(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    ir: &mut IrFile,
) -> Result<(), DomainScanError> {
    ir.interfaces = extract_go_interfaces(tree, source, path)?;
    ir.classes = extract_go_structs(tree, source, path)?;
    ir.functions = extract_go_functions(tree, source, path)?;
    ir.implementations = extract_go_methods(tree, source, path)?;
    ir.imports = extract_go_imports(tree, source)?;
    ir.schemas = extract_go_schemas(tree, source, path)?;
    Ok(())
}

// --- Go Interfaces ---

fn extract_go_interfaces(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<InterfaceDef>, DomainScanError> {
    let query = get_go_query(&GO_INTERFACES_Q, GO_INTERFACES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut interfaces = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "interface.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "interface.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "interface.body");

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = go_visibility(&name);

        // Extract method signatures from interface body
        let (methods, extends) = body_node
            .map(|b| extract_go_interface_members(b, source))
            .unwrap_or_default();

        interfaces.push(InterfaceDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics: Vec::new(),
            extends,
            methods,
            properties: Vec::new(),
            language_kind: InterfaceKind::Interface,
            decorators: Vec::new(),
        });
    }

    Ok(interfaces)
}

fn extract_go_interface_members(
    body: Node<'_>,
    source: &[u8],
) -> (Vec<MethodSignature>, Vec<String>) {
    let mut methods = Vec::new();
    let mut extends = Vec::new();

    fn collect_members(node: Node<'_>, source: &[u8], methods: &mut Vec<MethodSignature>, extends: &mut Vec<String>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "method_elem" | "method_spec" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| node_text(n, source))
                        .unwrap_or_default();
                    let span = node_span(child);

                    let params_node = child.child_by_field_name("parameters");
                    let parameters = params_node
                        .map(|p| extract_go_parameters(p, source))
                        .unwrap_or_default();

                    let return_type = child
                        .child_by_field_name("result")
                        .map(|rt| node_text(rt, source));

                    methods.push(MethodSignature {
                        name,
                        span,
                        is_async: false,
                        parameters,
                        return_type,
                        has_default: false,
                    });
                }
                // Embedded interfaces (constraint_elem wrapping type_identifier in newer grammars)
                "type_identifier" | "qualified_type" => {
                    extends.push(node_text(child, source));
                }
                "constraint_elem" | "struct_elem" | "type_elem" => {
                    // May contain embedded type identifiers
                    let mut inner = child.walk();
                    for inner_child in child.named_children(&mut inner) {
                        if inner_child.kind() == "type_identifier" || inner_child.kind() == "qualified_type" {
                            extends.push(node_text(inner_child, source));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    collect_members(body, source, &mut methods, &mut extends);
    (methods, extends)
}

// --- Go Structs ---

fn extract_go_structs(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ClassDef>, DomainScanError> {
    let query = get_go_query(&GO_STRUCTS_Q, GO_STRUCTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut structs = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "struct.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "struct.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "struct.body");

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = go_visibility(&name);

        let properties = body_node
            .map(|b| extract_go_struct_fields(b, source))
            .unwrap_or_default();

        structs.push(ClassDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics: Vec::new(),
            extends: None,
            implements: Vec::new(),
            methods: Vec::new(),
            properties,
            is_abstract: false,
            decorators: Vec::new(),
        });
    }

    Ok(structs)
}

// --- Go Functions ---

fn extract_go_functions(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<FunctionDef>, DomainScanError> {
    let query = get_go_query(&GO_FUNCTIONS_Q, GO_FUNCTIONS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut functions = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "function.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "function.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = go_visibility(&name);

        let params_node = def_node.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_go_parameters(p, source))
            .unwrap_or_default();

        let return_type = def_node
            .child_by_field_name("result")
            .map(|rt| node_text(rt, source));

        functions.push(FunctionDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async: false,
            is_generator: false,
            parameters,
            return_type,
            decorators: Vec::new(),
        });
    }

    Ok(functions)
}

// --- Go Methods (receiver methods -> ImplDef) ---

fn extract_go_methods(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ImplDef>, DomainScanError> {
    let query = get_go_query(&GO_METHODS_Q, GO_METHODS_SCM)?;
    let mut cursor = QueryCursor::new();
    // Group methods by receiver type
    let mut method_map: std::collections::HashMap<String, Vec<MethodDef>> =
        std::collections::HashMap::new();
    let mut span_map: std::collections::HashMap<String, Span> =
        std::collections::HashMap::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "method.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "method.def") else {
            continue;
        };
        let receiver_node = find_capture(&m, query, "method.receiver");

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = go_visibility(&name);

        // Extract receiver type name
        let receiver_type = receiver_node
            .map(|r| extract_go_receiver_type(r, source))
            .unwrap_or_default();

        let params_node = def_node.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_go_parameters(p, source))
            .unwrap_or_default();

        let return_type = def_node
            .child_by_field_name("result")
            .map(|rt| node_text(rt, source));

        let method = MethodDef {
            name,
            file: path.to_path_buf(),
            span: span.clone(),
            visibility,
            is_async: false,
            is_static: false,
            is_generator: false,
            parameters,
            return_type,
            decorators: Vec::new(),
            owner: Some(receiver_type.clone()),
            implements: None,
        };

        span_map.entry(receiver_type.clone()).or_insert(span);
        method_map
            .entry(receiver_type)
            .or_default()
            .push(method);
    }

    let impls = method_map
        .into_iter()
        .map(|(target, methods)| {
            let span = span_map
                .remove(&target)
                .unwrap_or_default();
            ImplDef {
                target,
                trait_name: None,
                file: path.to_path_buf(),
                span,
                methods,
            }
        })
        .collect();

    Ok(impls)
}

// --- Go Imports ---

fn extract_go_imports(
    tree: &Tree,
    source: &[u8],
) -> Result<Vec<ImportDef>, DomainScanError> {
    let query = get_go_query(&GO_IMPORTS_Q, GO_IMPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut imports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(def_node) = find_capture(&m, query, "import.def") else {
            continue;
        };

        let span = node_span(def_node);

        // Walk children to find import specs
        let mut walk = def_node.walk();
        for child in def_node.named_children(&mut walk) {
            match child.kind() {
                "import_spec" => {
                    let import = parse_go_import_spec(child, source, &span);
                    imports.push(import);
                }
                "import_spec_list" => {
                    let mut list_cursor = child.walk();
                    for spec in child.named_children(&mut list_cursor) {
                        if spec.kind() == "import_spec" {
                            let import = parse_go_import_spec(spec, source, &span);
                            imports.push(import);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(imports)
}

fn parse_go_import_spec(node: Node<'_>, source: &[u8], span: &Span) -> ImportDef {
    let path_node = node.child_by_field_name("path");
    let source_str = path_node
        .map(|p| strip_quotes(&node_text(p, source)))
        .unwrap_or_default();

    let alias = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source));

    let is_wildcard = alias.as_deref() == Some(".");

    let name = source_str
        .rsplit('/')
        .next()
        .unwrap_or(&source_str)
        .to_string();

    let symbols = vec![ImportedSymbol {
        name,
        alias,
        is_default: false,
        is_namespace: is_wildcard,
    }];

    ImportDef {
        source: source_str,
        symbols,
        is_wildcard,
        span: span.clone(),
    }
}

// --- Go Schemas (tagged structs) ---

fn extract_go_schemas(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<SchemaDef>, DomainScanError> {
    let query = get_go_query(&GO_SCHEMAS_Q, GO_SCHEMAS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut schemas = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "schema.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "schema.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "schema.body");

        // Check if struct has tagged fields (json, db, etc.)
        let has_tags = body_node
            .map(|b| go_struct_has_tags(b, source))
            .unwrap_or(false);

        if !has_tags {
            continue;
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = go_visibility(&name);

        let fields = body_node
            .map(|b| extract_go_schema_fields(b, source))
            .unwrap_or_default();

        schemas.push(SchemaDef {
            name,
            file: path.to_path_buf(),
            span,
            kind: SchemaKind::DataTransfer,
            fields,
            source_framework: "go-tags".to_string(),
            table_name: None,
            derives: Vec::new(),
            visibility,
        });
    }

    Ok(schemas)
}

// --- Go Helpers ---

fn go_visibility(name: &str) -> Visibility {
    // In Go, exported names start with uppercase
    if name.starts_with(|c: char| c.is_uppercase()) {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

fn extract_go_parameters(params_node: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut parameters = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.named_children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            let type_node = child.child_by_field_name("type");
            let type_annotation = type_node.map(|t| node_text(t, source));

            // Parameter declarations can have multiple names
            let mut name_cursor = child.walk();
            let mut found_name = false;
            for name_child in child.named_children(&mut name_cursor) {
                if name_child.kind() == "identifier" {
                    parameters.push(Parameter {
                        name: node_text(name_child, source),
                        type_annotation: type_annotation.clone(),
                        is_optional: false,
                        has_default: false,
                        is_rest: false,
                    });
                    found_name = true;
                }
            }
            // If no name (type-only parameter), still record it
            if !found_name {
                parameters.push(Parameter {
                    name: String::new(),
                    type_annotation,
                    is_optional: false,
                    has_default: false,
                    is_rest: false,
                });
            }
        } else if child.kind() == "variadic_parameter_declaration" {
            let type_node = child.child_by_field_name("type");
            let type_annotation = type_node.map(|t| node_text(t, source));
            let mut name_cursor = child.walk();
            let name = child
                .named_children(&mut name_cursor)
                .find(|c| c.kind() == "identifier")
                .map(|n| node_text(n, source))
                .unwrap_or_default();

            parameters.push(Parameter {
                name,
                type_annotation,
                is_optional: true,
                has_default: false,
                is_rest: true,
            });
        }
    }

    parameters
}

fn extract_go_receiver_type(receiver: Node<'_>, source: &[u8]) -> String {
    let mut cursor = receiver.walk();
    for child in receiver.named_children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            if let Some(type_node) = child.child_by_field_name("type") {
                let text = node_text(type_node, source);
                // Strip pointer: *MyType -> MyType
                return text.trim_start_matches('*').to_string();
            }
        }
    }
    String::new()
}

fn extract_go_struct_fields(body: Node<'_>, source: &[u8]) -> Vec<PropertyDef> {
    let mut properties = Vec::new();

    // body is struct_type; iterate its named children to find field_declaration_list
    // or directly find field_declaration nodes
    fn collect_fields(node: Node<'_>, source: &[u8], props: &mut Vec<PropertyDef>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| node_text(t, source));

                let mut name_cursor = child.walk();
                for name_child in child.named_children(&mut name_cursor) {
                    if name_child.kind() == "field_identifier" {
                        let name = node_text(name_child, source);
                        let vis = go_visibility(&name);
                        props.push(PropertyDef {
                            name,
                            type_annotation: type_annotation.clone(),
                            is_optional: false,
                            is_readonly: false,
                            visibility: vis,
                        });
                    }
                }
            } else if child.kind() == "field_declaration_list" {
                collect_fields(child, source, props);
            }
        }
    }

    collect_fields(body, source, &mut properties);
    properties
}

fn go_struct_has_tags(body: Node<'_>, source: &[u8]) -> bool {
    fn check_tags(node: Node<'_>, source: &[u8]) -> bool {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "field_declaration" {
                if let Some(tag) = child.child_by_field_name("tag") {
                    let tag_text = node_text(tag, source);
                    if tag_text.contains("json:") || tag_text.contains("db:") || tag_text.contains("xml:") || tag_text.contains("yaml:") {
                        return true;
                    }
                }
            } else if child.kind() == "field_declaration_list"
                && check_tags(child, source)
            {
                return true;
            }
        }
        false
    }
    check_tags(body, source)
}

fn extract_go_schema_fields(body: Node<'_>, source: &[u8]) -> Vec<SchemaField> {
    let mut fields = Vec::new();

    fn collect_schema_fields(node: Node<'_>, source: &[u8], fields: &mut Vec<SchemaField>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| node_text(t, source));

                let tag = child
                    .child_by_field_name("tag")
                    .map(|t| node_text(t, source));

                let is_optional = type_annotation
                    .as_ref()
                    .map(|t| t.starts_with('*'))
                    .unwrap_or(false);

                let mut constraints = Vec::new();
                if let Some(ref tag_text) = tag {
                    if tag_text.contains("omitempty") {
                        constraints.push("omitempty".to_string());
                    }
                }

                let mut name_cursor = child.walk();
                for name_child in child.named_children(&mut name_cursor) {
                    if name_child.kind() == "field_identifier" {
                        let name = node_text(name_child, source);
                        fields.push(SchemaField {
                            name,
                            type_annotation: type_annotation.clone(),
                            is_optional,
                            is_primary_key: false,
                            constraints: constraints.clone(),
                        });
                    }
                }
            } else if child.kind() == "field_declaration_list" {
                collect_schema_fields(child, source, fields);
            }
        }
    }

    collect_schema_fields(body, source, &mut fields);
    fields
}

// ===========================================================================
// Python extraction
// ===========================================================================

fn extract_python(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    ir: &mut IrFile,
) -> Result<(), DomainScanError> {
    // Extract classes (includes protocols and ABCs via post-processing)
    let all_classes = extract_py_classes(tree, source, path)?;

    // Separate protocols and ABCs from regular classes
    let mut classes = Vec::new();
    let mut interfaces = Vec::new();

    for cls in all_classes {
        let bases: Vec<&str> = cls.implements.iter().map(|s| s.as_str()).collect();
        if bases.iter().any(|b| *b == "Protocol" || b.ends_with(".Protocol")) {
            interfaces.push(InterfaceDef {
                name: cls.name,
                file: cls.file,
                span: cls.span,
                visibility: cls.visibility,
                generics: cls.generics,
                extends: cls.implements,
                methods: Vec::new(),
                properties: cls.properties,
                language_kind: InterfaceKind::Protocol,
                decorators: cls.decorators,
            });
        } else if bases.iter().any(|b| *b == "ABC" || b.ends_with(".ABC")) || cls.is_abstract {
            interfaces.push(InterfaceDef {
                name: cls.name,
                file: cls.file,
                span: cls.span,
                visibility: cls.visibility,
                generics: cls.generics,
                extends: cls.implements,
                methods: Vec::new(),
                properties: cls.properties,
                language_kind: InterfaceKind::AbstractClass,
                decorators: cls.decorators,
            });
        } else {
            classes.push(cls);
        }
    }

    ir.classes = classes;
    ir.interfaces = interfaces;
    ir.functions = extract_py_functions(tree, source, path)?;
    ir.imports = extract_py_imports(tree, source)?;
    ir.schemas = extract_py_schemas(tree, source, path)?;
    ir.services = extract_py_services(tree, source, path)?;
    Ok(())
}

// --- Python Classes ---

fn extract_py_classes(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ClassDef>, DomainScanError> {
    let query = get_py_query(&PY_CLASSES_Q, PY_CLASSES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut classes = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "class.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "class.def") else {
            continue;
        };

        if !seen.insert(def_node.start_byte()) {
            continue;
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = py_visibility(&name);

        // Extract base classes
        let bases = def_node
            .child_by_field_name("superclasses")
            .map(|b| extract_py_bases(b, source))
            .unwrap_or_default();

        // Check for decorators (class may be inside decorated_definition)
        let decorators = extract_py_class_decorators(def_node, source);

        let is_abstract = decorators.iter().any(|d| d.contains("abstractmethod"))
            || bases.iter().any(|b| b == "ABC" || b.ends_with(".ABC"));

        // Extract methods and properties from class body
        let body_node = find_capture(&m, query, "class.body");
        let mut methods = body_node
            .map(|b| extract_py_class_methods(b, source, path))
            .unwrap_or_default();
        let properties = body_node
            .map(|b| extract_py_class_properties(b, source))
            .unwrap_or_default();

        for method in &mut methods {
            method.owner = Some(name.clone());
        }

        classes.push(ClassDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics: Vec::new(),
            extends: bases.first().cloned(),
            implements: bases,
            methods,
            properties,
            is_abstract,
            decorators,
        });
    }

    Ok(classes)
}

// --- Python Functions ---

fn extract_py_functions(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<FunctionDef>, DomainScanError> {
    let query = get_py_query(&PY_FUNCTIONS_Q, PY_FUNCTIONS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut functions = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "function.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "function.def") else {
            continue;
        };

        // Only top-level functions (not inside class body)
        if let Some(parent) = def_node.parent() {
            if parent.kind() == "block" {
                if let Some(grandparent) = parent.parent() {
                    if grandparent.kind() == "class_definition" {
                        continue;
                    }
                }
            }
            // Also skip if inside decorated_definition that's inside a class
            if parent.kind() == "decorated_definition" {
                if let Some(gp) = parent.parent() {
                    if gp.kind() == "block" {
                        if let Some(ggp) = gp.parent() {
                            if ggp.kind() == "class_definition" {
                                continue;
                            }
                        }
                    }
                }
            }
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = py_visibility(&name);

        let is_async = def_node.kind() == "function_definition"
            && has_child_of_kind(def_node, "async");

        let params_node = def_node.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_py_parameters(p, source))
            .unwrap_or_default();

        let return_type = def_node
            .child_by_field_name("return_type")
            .map(|rt| node_text(rt, source));

        let decorators = extract_py_func_decorators(def_node, source);

        functions.push(FunctionDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async,
            is_generator: false,
            parameters,
            return_type,
            decorators,
        });
    }

    Ok(functions)
}

// --- Python Imports ---

fn extract_py_imports(
    tree: &Tree,
    source: &[u8],
) -> Result<Vec<ImportDef>, DomainScanError> {
    let query = get_py_query(&PY_IMPORTS_Q, PY_IMPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut imports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(def_node) = find_capture(&m, query, "import.def") else {
            continue;
        };

        let span = node_span(def_node);

        match def_node.kind() {
            "import_statement" => {
                // import foo, import foo.bar
                let mut walk = def_node.walk();
                for child in def_node.named_children(&mut walk) {
                    match child.kind() {
                        "dotted_name" => {
                            let source_str = node_text(child, source);
                            imports.push(ImportDef {
                                source: source_str.clone(),
                                symbols: vec![ImportedSymbol {
                                    name: source_str,
                                    alias: None,
                                    is_default: false,
                                    is_namespace: true,
                                }],
                                is_wildcard: false,
                                span: span.clone(),
                            });
                        }
                        "aliased_import" => {
                            let name_node = child.child_by_field_name("name");
                            let alias_node = child.child_by_field_name("alias");
                            let source_str = name_node
                                .map(|n| node_text(n, source))
                                .unwrap_or_default();
                            let alias = alias_node.map(|a| node_text(a, source));
                            imports.push(ImportDef {
                                source: source_str.clone(),
                                symbols: vec![ImportedSymbol {
                                    name: source_str,
                                    alias,
                                    is_default: false,
                                    is_namespace: true,
                                }],
                                is_wildcard: false,
                                span: span.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }
            "import_from_statement" => {
                // from foo import bar, baz
                let module_name = def_node
                    .child_by_field_name("module_name")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();

                let module_name_start = def_node
                    .child_by_field_name("module_name")
                    .map(|n| n.start_byte());

                let mut symbols = Vec::new();
                let mut is_wildcard = false;
                let mut walk = def_node.walk();

                for child in def_node.named_children(&mut walk) {
                    // Skip the module name (first dotted_name or relative_import)
                    if Some(child.start_byte()) == module_name_start {
                        continue;
                    }
                    match child.kind() {
                        "wildcard_import" => {
                            is_wildcard = true;
                            symbols.push(ImportedSymbol {
                                name: "*".to_string(),
                                alias: None,
                                is_default: false,
                                is_namespace: true,
                            });
                        }
                        "import_prefix" | "relative_import" => {}
                        _ => {
                            // Named imports (dotted_name, aliased_import, etc.)
                            extract_py_import_names(child, source, &mut symbols);
                        }
                    }
                }

                imports.push(ImportDef {
                    source: module_name,
                    symbols,
                    is_wildcard,
                    span: span.clone(),
                });
            }
            _ => {}
        }
    }

    Ok(imports)
}

fn extract_py_import_names(
    node: Node<'_>,
    source: &[u8],
    symbols: &mut Vec<ImportedSymbol>,
) {
    match node.kind() {
        "dotted_name" | "identifier" => {
            symbols.push(ImportedSymbol {
                name: node_text(node, source),
                alias: None,
                is_default: false,
                is_namespace: false,
            });
        }
        "aliased_import" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| node_text(n, source))
                .unwrap_or_default();
            let alias = node
                .child_by_field_name("alias")
                .map(|a| node_text(a, source));
            symbols.push(ImportedSymbol {
                name,
                alias,
                is_default: false,
                is_namespace: false,
            });
        }
        _ => {
            // Walk children for compound patterns
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                extract_py_import_names(child, source, symbols);
            }
        }
    }
}

// --- Python Schemas (Pydantic, dataclass, TypedDict, SQLAlchemy) ---

fn extract_py_schemas(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<SchemaDef>, DomainScanError> {
    let query = get_py_query(&PY_SCHEMAS_Q, PY_SCHEMAS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut schemas = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "schema.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "schema.def") else {
            continue;
        };

        if !seen.insert(def_node.start_byte()) {
            continue;
        }

        let bases = def_node
            .child_by_field_name("superclasses")
            .map(|b| extract_py_bases(b, source))
            .unwrap_or_default();

        let decorators = extract_py_class_decorators(def_node, source);

        // Classify schema kind
        let (kind, framework) = classify_py_schema(&bases, &decorators);
        let Some(kind) = kind else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = py_visibility(&name);

        let body_node = find_capture(&m, query, "schema.body");
        let fields = body_node
            .map(|b| extract_py_schema_fields(b, source))
            .unwrap_or_default();

        schemas.push(SchemaDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            fields,
            source_framework: framework,
            table_name: None,
            derives: Vec::new(),
            visibility,
        });
    }

    Ok(schemas)
}

// --- Python Services (FastAPI, Flask, Django) ---

fn extract_py_services(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ServiceDef>, DomainScanError> {
    let query = get_py_query(&PY_SERVICES_Q, PY_SERVICES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut services = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "service.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "service.def") else {
            continue;
        };

        if !seen.insert(def_node.start_byte()) {
            continue;
        }

        let decorator_node = find_capture(&m, query, "service.decorator");
        let dec_text = decorator_node
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let dec_text = dec_text.strip_prefix('@').unwrap_or(&dec_text).to_string();

        // Classify the service kind from decorator
        let kind = classify_py_service_kind(&dec_text);
        let Some(kind) = kind else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);

        // Extract routes from decorator
        let routes = extract_py_routes(&dec_text, &name);

        services.push(ServiceDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            methods: Vec::new(),
            dependencies: Vec::new(),
            decorators: vec![dec_text],
            routes,
        });
    }

    Ok(services)
}

// --- Python Helpers ---

fn py_visibility(name: &str) -> Visibility {
    if name.starts_with("__") && !name.ends_with("__") {
        Visibility::Private
    } else if name.starts_with('_') {
        Visibility::Protected
    } else {
        Visibility::Unknown
    }
}

fn extract_py_bases(bases_node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut bases = Vec::new();
    let mut cursor = bases_node.walk();
    for child in bases_node.named_children(&mut cursor) {
        match child.kind() {
            "identifier" | "attribute" => {
                bases.push(node_text(child, source));
            }
            "keyword_argument" => {
                // metaclass=ABCMeta, etc.
            }
            _ => {}
        }
    }
    bases
}

fn extract_py_class_decorators(class_node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut decorators = Vec::new();

    // Check if parent is decorated_definition
    if let Some(parent) = class_node.parent() {
        if parent.kind() == "decorated_definition" {
            let mut cursor = parent.walk();
            for child in parent.named_children(&mut cursor) {
                if child.kind() == "decorator" {
                    let text = node_text(child, source);
                    let trimmed = text.strip_prefix('@').unwrap_or(&text);
                    decorators.push(trimmed.to_string());
                }
            }
        }
    }

    decorators
}

fn extract_py_func_decorators(func_node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut decorators = Vec::new();

    if let Some(parent) = func_node.parent() {
        if parent.kind() == "decorated_definition" {
            let mut cursor = parent.walk();
            for child in parent.named_children(&mut cursor) {
                if child.kind() == "decorator" {
                    let text = node_text(child, source);
                    let trimmed = text.strip_prefix('@').unwrap_or(&text);
                    decorators.push(trimmed.to_string());
                }
            }
        }
    }

    decorators
}

fn extract_py_parameters(params_node: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut parameters = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.named_children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                let name = node_text(child, source);
                if name == "self" || name == "cls" {
                    continue;
                }
                parameters.push(Parameter {
                    name,
                    type_annotation: None,
                    is_optional: false,
                    has_default: false,
                    is_rest: false,
                });
            }
            "typed_parameter" => {
                let name_from_field = child.child_by_field_name("name");
                let name_node = if let Some(n) = name_from_field {
                    Some(n)
                } else {
                    let mut c = child.walk();
                    let found = child.named_children(&mut c).find(|n| n.kind() == "identifier");
                    found
                };
                let name = name_node
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                if name == "self" || name == "cls" {
                    continue;
                }
                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| node_text(t, source));
                parameters.push(Parameter {
                    name,
                    type_annotation,
                    is_optional: false,
                    has_default: false,
                    is_rest: false,
                });
            }
            "default_parameter" | "typed_default_parameter" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                if name == "self" || name == "cls" {
                    continue;
                }
                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| node_text(t, source));
                parameters.push(Parameter {
                    name,
                    type_annotation,
                    is_optional: true,
                    has_default: true,
                    is_rest: false,
                });
            }
            "list_splat_pattern" | "dictionary_splat_pattern" => {
                let mut inner_cursor = child.walk();
                let name = child
                    .named_children(&mut inner_cursor)
                    .find(|c| c.kind() == "identifier")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                parameters.push(Parameter {
                    name,
                    type_annotation: None,
                    is_optional: true,
                    has_default: false,
                    is_rest: true,
                });
            }
            _ => {}
        }
    }

    parameters
}

fn extract_py_class_methods(body: Node<'_>, source: &[u8], path: &Path) -> Vec<MethodDef> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        let func_node = match child.kind() {
            "function_definition" => child,
            "decorated_definition" => {
                // Get the function inside the decorator
                if let Some(def) = child.child_by_field_name("definition") {
                    if def.kind() == "function_definition" {
                        def
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        let name = func_node
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();

        // Skip __init__ as it's a constructor
        if name == "__init__" {
            continue;
        }

        let span = node_span(func_node);
        let visibility = py_visibility(&name);

        let params_node = func_node.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_py_parameters(p, source))
            .unwrap_or_default();

        let return_type = func_node
            .child_by_field_name("return_type")
            .map(|rt| node_text(rt, source));

        let decorators = extract_py_func_decorators(func_node, source);
        // If the function itself is the child, check the decorated_definition parent
        let all_decorators = if decorators.is_empty() && child.kind() == "decorated_definition" {
            let mut decs = Vec::new();
            let mut dec_cursor = child.walk();
            for dec in child.named_children(&mut dec_cursor) {
                if dec.kind() == "decorator" {
                    let text = node_text(dec, source);
                    let trimmed = text.strip_prefix('@').unwrap_or(&text);
                    decs.push(trimmed.to_string());
                }
            }
            decs
        } else {
            decorators
        };

        let is_static = all_decorators.iter().any(|d| d == "staticmethod");
        let is_async = has_child_of_kind(func_node, "async");

        methods.push(MethodDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async,
            is_static,
            is_generator: false,
            parameters,
            return_type,
            decorators: all_decorators,
            owner: None,
            implements: None,
        });
    }

    methods
}

fn extract_py_class_properties(body: Node<'_>, source: &[u8]) -> Vec<PropertyDef> {
    let mut properties = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        // Python class-level type annotations: name: Type
        if child.kind() == "expression_statement" {
            let mut inner = child.walk();
            for expr in child.named_children(&mut inner) {
                if expr.kind() == "type" || expr.kind() == "assignment" {
                    // type annotation
                } else if expr.kind() == "identifier" {
                    // bare assignment (no annotation)
                }
            }
        }
        if child.kind() == "type_alias_statement" || child.kind() == "expression_statement" {
            let mut walk = child.walk();
            for inner in child.named_children(&mut walk) {
                if inner.kind() == "assignment" {
                    if let Some(left) = inner.child_by_field_name("left") {
                        if left.kind() == "identifier" {
                            let name = node_text(left, source);
                            let vis = py_visibility(&name);
                            properties.push(PropertyDef {
                                name,
                                type_annotation: None,
                                is_optional: false,
                                is_readonly: false,
                                visibility: vis,
                            });
                        }
                    }
                }
            }
        }
    }

    properties
}

fn classify_py_schema(bases: &[String], decorators: &[String]) -> (Option<SchemaKind>, String) {
    // Check base classes
    for base in bases {
        match base.as_str() {
            "BaseModel" | "pydantic.BaseModel" => {
                return (Some(SchemaKind::ValidationSchema), "pydantic".to_string());
            }
            "TypedDict" | "typing.TypedDict" => {
                return (Some(SchemaKind::DataTransfer), "typeddict".to_string());
            }
            "Base" | "DeclarativeBase" | "db.Model" => {
                return (Some(SchemaKind::OrmModel), "sqlalchemy".to_string());
            }
            _ => {}
        }
    }

    // Check decorators
    for dec in decorators {
        if dec == "dataclass" || dec.starts_with("dataclass(") || dec.starts_with("dataclasses.dataclass") {
            return (Some(SchemaKind::DataTransfer), "dataclass".to_string());
        }
    }

    (None, String::new())
}

fn extract_py_schema_fields(body: Node<'_>, source: &[u8]) -> Vec<SchemaField> {
    let mut fields = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let mut walk = child.walk();
            for inner in child.named_children(&mut walk) {
                if inner.kind() == "assignment" {
                    // name: Type = value or name = value
                    if let Some(left) = inner.child_by_field_name("left") {
                        if left.kind() == "type" {
                            // name: Type = value
                            let mut type_walk = left.walk();
                            let id = left
                                .named_children(&mut type_walk)
                                .find(|c| c.kind() == "identifier");
                            if let Some(id_node) = id {
                                let name = node_text(id_node, source);
                                let type_annotation = left
                                    .named_children(&mut left.walk())
                                    .find(|c| c.kind() != "identifier")
                                    .map(|t| node_text(t, source));
                                let is_optional = type_annotation
                                    .as_ref()
                                    .map(|t| t.contains("Optional"))
                                    .unwrap_or(false);
                                fields.push(SchemaField {
                                    name,
                                    type_annotation,
                                    is_optional,
                                    is_primary_key: false,
                                    constraints: Vec::new(),
                                });
                            }
                        } else if left.kind() == "identifier" {
                            let name = node_text(left, source);
                            fields.push(SchemaField {
                                name,
                                type_annotation: None,
                                is_optional: false,
                                is_primary_key: false,
                                constraints: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    fields
}

fn classify_py_service_kind(decorator: &str) -> Option<ServiceKind> {
    let dec_name = decorator.split('(').next().unwrap_or_default();
    let dec_parts: Vec<&str> = dec_name.split('.').collect();
    let last_part = dec_parts.last().copied().unwrap_or_default();

    match last_part {
        "get" | "post" | "put" | "patch" | "delete" | "route" | "api_view" => {
            Some(ServiceKind::HttpController)
        }
        "websocket" => Some(ServiceKind::EventHandler),
        "middleware" => Some(ServiceKind::Middleware),
        _ => None,
    }
}

fn extract_py_routes(decorator: &str, handler_name: &str) -> Vec<RouteDef> {
    let dec_name = decorator.split('(').next().unwrap_or_default();
    let dec_parts: Vec<&str> = dec_name.split('.').collect();
    let last_part = dec_parts.last().copied().unwrap_or_default();

    let http_method = match last_part {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        _ => None,
    };

    let Some(http_method) = http_method else {
        return Vec::new();
    };

    let path = extract_decorator_string_arg(decorator);

    vec![RouteDef {
        method: http_method,
        path,
        handler: handler_name.to_string(),
    }]
}

// ===========================================================================
// Java extraction
// ===========================================================================

fn extract_java(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    ir: &mut IrFile,
) -> Result<(), DomainScanError> {
    ir.interfaces = extract_jv_interfaces(tree, source, path)?;
    ir.classes = extract_jv_classes(tree, source, path)?;
    ir.functions = extract_jv_functions(tree, source, path)?;
    ir.imports = extract_jv_imports(tree, source)?;
    ir.services = extract_jv_services(tree, source, path)?;
    ir.schemas = extract_jv_schemas(tree, source, path)?;
    Ok(())
}

// --- Java Interfaces ---

fn extract_jv_interfaces(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<InterfaceDef>, DomainScanError> {
    let query = get_jv_query(&JV_INTERFACES_Q, JV_INTERFACES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut interfaces = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "interface.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "interface.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "interface.body");

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = jv_visibility(def_node);
        let generics = extract_jv_generics(def_node, source);
        let extends = extract_jv_interface_extends(def_node, source);

        let methods = body_node
            .map(|b| extract_jv_interface_methods(b, source))
            .unwrap_or_default();

        interfaces.push(InterfaceDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            methods,
            properties: Vec::new(),
            language_kind: InterfaceKind::Interface,
            decorators: extract_jv_annotations(def_node, source),
        });
    }

    Ok(interfaces)
}

fn extract_jv_interface_methods(body: Node<'_>, source: &[u8]) -> Vec<MethodSignature> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "method_declaration" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let span = node_span(child);

        let params_node = child.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_jv_parameters(p, source))
            .unwrap_or_default();

        let return_type = child
            .child_by_field_name("type")
            .map(|rt| node_text(rt, source));

        // Has body = default method
        let has_default = child.child_by_field_name("body").is_some();

        methods.push(MethodSignature {
            name,
            span,
            is_async: false,
            parameters,
            return_type,
            has_default,
        });
    }

    methods
}

// --- Java Classes ---

fn extract_jv_classes(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ClassDef>, DomainScanError> {
    let query = get_jv_query(&JV_CLASSES_Q, JV_CLASSES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut classes = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "class.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "class.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = jv_visibility(def_node);
        let is_abstract = jv_has_modifier(def_node, "abstract");
        let generics = extract_jv_generics(def_node, source);
        let decorators = extract_jv_annotations(def_node, source);

        let extends = def_node
            .child_by_field_name("superclass")
            .and_then(|sc| {
                let mut c = sc.walk();
                let result = sc.named_children(&mut c).next().map(|n| node_text(n, source));
                result
            });

        let implements = extract_jv_implements(def_node, source);

        let body_node = find_capture(&m, query, "class.body");
        let mut methods = body_node
            .map(|b| extract_jv_class_methods(b, source, path))
            .unwrap_or_default();
        let properties = body_node
            .map(|b| extract_jv_class_fields(b, source))
            .unwrap_or_default();

        for method in &mut methods {
            method.owner = Some(name.clone());
        }

        classes.push(ClassDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            implements,
            methods,
            properties,
            is_abstract,
            decorators,
        });
    }

    Ok(classes)
}

fn extract_jv_class_methods(body: Node<'_>, source: &[u8], path: &Path) -> Vec<MethodDef> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "method_declaration" && child.kind() != "constructor_declaration" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let span = node_span(child);
        let visibility = jv_member_visibility(child);
        let is_static = jv_has_modifier(child, "static");
        let decorators = extract_jv_annotations(child, source);

        let params_node = child.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_jv_parameters(p, source))
            .unwrap_or_default();

        let return_type = child
            .child_by_field_name("type")
            .map(|rt| node_text(rt, source));

        methods.push(MethodDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async: false,
            is_static,
            is_generator: false,
            parameters,
            return_type,
            decorators,
            owner: None,
            implements: None,
        });
    }

    methods
}

fn extract_jv_class_fields(body: Node<'_>, source: &[u8]) -> Vec<PropertyDef> {
    let mut fields = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        let type_text = child
            .child_by_field_name("type")
            .map(|t| node_text(t, source));
        let visibility = jv_member_visibility(child);
        let is_readonly = jv_has_modifier(child, "final");

        // field_declaration can have multiple declarators
        let mut dcursor = child.walk();
        for dchild in child.named_children(&mut dcursor) {
            if dchild.kind() == "variable_declarator" {
                let name = dchild
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                fields.push(PropertyDef {
                    name,
                    type_annotation: type_text.clone(),
                    is_optional: false,
                    is_readonly,
                    visibility,
                });
            }
        }
    }

    fields
}

// --- Java Functions (top-level static methods in classes) ---

fn extract_jv_functions(
    _tree: &Tree,
    _source: &[u8],
    _path: &Path,
) -> Result<Vec<FunctionDef>, DomainScanError> {
    // Java has no top-level functions; methods are always in classes
    Ok(Vec::new())
}

// --- Java Imports ---

fn extract_jv_imports(tree: &Tree, source: &[u8]) -> Result<Vec<ImportDef>, DomainScanError> {
    let query = get_jv_query(&JV_IMPORTS_Q, JV_IMPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut imports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(def_node) = find_capture(&m, query, "import.def") else {
            continue;
        };

        let full_text = node_text(def_node, source);
        let span = node_span(def_node);

        // Parse "import static? <path>.*?" or "import static? <path>.<symbol>"
        let trimmed = full_text
            .trim()
            .trim_start_matches("import")
            .trim()
            .trim_start_matches("static")
            .trim()
            .trim_end_matches(';')
            .trim();

        let is_wildcard = trimmed.ends_with(".*");
        let source_path = if is_wildcard {
            trimmed.trim_end_matches(".*").to_string()
        } else if let Some(pos) = trimmed.rfind('.') {
            trimmed[..pos].to_string()
        } else {
            trimmed.to_string()
        };

        let symbols = if is_wildcard {
            Vec::new()
        } else if let Some(pos) = trimmed.rfind('.') {
            vec![ImportedSymbol {
                name: trimmed[pos + 1..].to_string(),
                alias: None,
                is_default: false,
                is_namespace: false,
            }]
        } else {
            Vec::new()
        };

        imports.push(ImportDef {
            source: source_path,
            symbols,
            is_wildcard,
            span,
        });
    }

    Ok(imports)
}

// --- Java Services ---

fn extract_jv_services(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ServiceDef>, DomainScanError> {
    let query = get_jv_query(&JV_SERVICES_Q, JV_SERVICES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut services = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "service.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "service.def") else {
            continue;
        };

        let decorators = extract_jv_annotations(def_node, source);
        let kind = classify_jv_service_kind(&decorators);
        let Some(kind) = kind else { continue; };

        let name = node_text(name_node, source);
        let span = node_span(def_node);

        let body_node = find_capture(&m, query, "service.body");
        let methods = body_node
            .map(|b| extract_jv_class_methods(b, source, path))
            .unwrap_or_default();

        let routes = extract_jv_routes(&methods);

        services.push(ServiceDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            methods,
            dependencies: Vec::new(),
            decorators,
            routes,
        });
    }

    Ok(services)
}

fn classify_jv_service_kind(annotations: &[String]) -> Option<ServiceKind> {
    for ann in annotations {
        match ann.as_str() {
            "RestController" | "Controller" => return Some(ServiceKind::HttpController),
            "Service" => return Some(ServiceKind::Microservice),
            "Repository" => return Some(ServiceKind::Repository),
            "Component" => return Some(ServiceKind::Microservice),
            _ => {}
        }
    }
    None
}

fn extract_jv_routes(methods: &[MethodDef]) -> Vec<RouteDef> {
    let mut routes = Vec::new();
    for method in methods {
        for dec in &method.decorators {
            let ann_name = dec.split('(').next().unwrap_or_default();
            let http_method = match ann_name {
                "GetMapping" => Some(HttpMethod::Get),
                "PostMapping" => Some(HttpMethod::Post),
                "PutMapping" => Some(HttpMethod::Put),
                "PatchMapping" => Some(HttpMethod::Patch),
                "DeleteMapping" => Some(HttpMethod::Delete),
                _ => None,
            };
            if let Some(http_method) = http_method {
                let path = extract_decorator_string_arg(dec);
                routes.push(RouteDef {
                    method: http_method,
                    path,
                    handler: method.name.clone(),
                });
            }
        }
    }
    routes
}

// --- Java Schemas ---

fn extract_jv_schemas(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<SchemaDef>, DomainScanError> {
    let query = get_jv_query(&JV_SCHEMAS_Q, JV_SCHEMAS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut schemas = Vec::new();
    let mut seen = HashSet::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "schema.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "schema.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);

        // Deduplicate (same name can match both record and class patterns)
        let key = (name.clone(), span.start_line);
        if seen.contains(&key) {
            continue;
        }

        let is_record = def_node.kind() == "record_declaration";
        let annotations = extract_jv_annotations(def_node, source);
        let has_entity_annotation = annotations.iter().any(|a| a == "Entity");

        if !is_record && !has_entity_annotation {
            continue;
        }

        let (kind, framework) = if is_record {
            (SchemaKind::DataTransfer, "java-record".to_string())
        } else {
            (SchemaKind::OrmModel, "jpa".to_string())
        };

        let fields = if is_record {
            let params = def_node.child_by_field_name("parameters");
            params
                .map(|p| extract_jv_record_fields(p, source))
                .unwrap_or_default()
        } else {
            let body = find_capture(&m, query, "schema.body");
            body.map(|b| extract_jv_schema_class_fields(b, source))
                .unwrap_or_default()
        };

        seen.insert(key);
        schemas.push(SchemaDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            fields,
            source_framework: framework,
            table_name: None,
            derives: Vec::new(),
            visibility: jv_visibility(def_node),
        });
    }

    Ok(schemas)
}

fn extract_jv_record_fields(params: Node<'_>, source: &[u8]) -> Vec<SchemaField> {
    let mut fields = Vec::new();
    let mut cursor = params.walk();

    for child in params.named_children(&mut cursor) {
        if child.kind() != "formal_parameter" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let type_annotation = child
            .child_by_field_name("type")
            .map(|t| node_text(t, source));

        fields.push(SchemaField {
            name,
            type_annotation,
            is_optional: false,
            is_primary_key: false,
            constraints: Vec::new(),
        });
    }

    fields
}

fn extract_jv_schema_class_fields(body: Node<'_>, source: &[u8]) -> Vec<SchemaField> {
    let mut fields = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        let type_text = child
            .child_by_field_name("type")
            .map(|t| node_text(t, source));
        let annotations = extract_jv_annotations(child, source);
        let is_primary_key = annotations.iter().any(|a| a == "Id");

        let mut dcursor = child.walk();
        for dchild in child.named_children(&mut dcursor) {
            if dchild.kind() == "variable_declarator" {
                let name = dchild
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                fields.push(SchemaField {
                    name,
                    type_annotation: type_text.clone(),
                    is_optional: false,
                    is_primary_key,
                    constraints: Vec::new(),
                });
            }
        }
    }

    fields
}

// --- Java Helpers ---

fn jv_visibility(node: Node<'_>) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mc = child.walk();
            for modifier in child.children(&mut mc) {
                match modifier.kind() {
                    "public" => return Visibility::Public,
                    "private" => return Visibility::Private,
                    "protected" => return Visibility::Protected,
                    _ => {}
                }
            }
        }
    }
    // Java default: package-private → map to Unknown
    Visibility::Unknown
}

fn jv_member_visibility(node: Node<'_>) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mc = child.walk();
            for modifier in child.children(&mut mc) {
                match modifier.kind() {
                    "public" => return Visibility::Public,
                    "private" => return Visibility::Private,
                    "protected" => return Visibility::Protected,
                    _ => {}
                }
            }
        }
    }
    Visibility::Unknown
}

fn jv_has_modifier(node: Node<'_>, modifier_name: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mc = child.walk();
            for modifier in child.children(&mut mc) {
                if modifier.kind() == modifier_name {
                    return true;
                }
            }
        }
    }
    false
}

fn extract_jv_generics(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let Some(type_params) = node.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut generics = Vec::new();
    let mut cursor = type_params.walk();
    for child in type_params.named_children(&mut cursor) {
        if child.kind() == "type_parameter" {
            let mut inner = child.walk();
            for inner_child in child.named_children(&mut inner) {
                if inner_child.kind() == "identifier" || inner_child.kind() == "type_identifier" {
                    generics.push(node_text(inner_child, source));
                    break;
                }
            }
        }
    }
    generics
}

fn extract_jv_interface_extends(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut extends = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "extends_interfaces" {
            let mut inner = child.walk();
            for inner_child in child.named_children(&mut inner) {
                if inner_child.kind() == "type_list" {
                    let mut tl = inner_child.walk();
                    for t in inner_child.named_children(&mut tl) {
                        extends.push(node_text(t, source));
                    }
                }
            }
        }
    }
    extends
}

fn extract_jv_implements(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut implements = Vec::new();
    if let Some(ifaces) = node.child_by_field_name("interfaces") {
        let mut cursor = ifaces.walk();
        for child in ifaces.named_children(&mut cursor) {
            if child.kind() == "type_list" {
                let mut tl = child.walk();
                for t in child.named_children(&mut tl) {
                    implements.push(node_text(t, source));
                }
            }
        }
    }
    implements
}

fn extract_jv_parameters(params: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut parameters = Vec::new();
    let mut cursor = params.walk();

    for child in params.named_children(&mut cursor) {
        match child.kind() {
            "formal_parameter" | "spread_parameter" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();
                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| node_text(t, source));
                let is_rest = child.kind() == "spread_parameter";

                parameters.push(Parameter {
                    name,
                    type_annotation,
                    is_optional: false,
                    has_default: false,
                    is_rest,
                });
            }
            _ => {}
        }
    }

    parameters
}

fn extract_jv_annotations(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut annotations = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mc = child.walk();
            for modifier in child.named_children(&mut mc) {
                match modifier.kind() {
                    "annotation" => {
                        if let Some(name_node) = modifier.child_by_field_name("name") {
                            annotations.push(node_text(name_node, source));
                        }
                    }
                    "marker_annotation" => {
                        // marker_annotation has the name as a direct child
                        let mut inner = modifier.walk();
                        for inner_child in modifier.named_children(&mut inner) {
                            if inner_child.kind() == "identifier" {
                                annotations.push(node_text(inner_child, source));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    annotations
}

// ===========================================================================
// Kotlin extraction
// ===========================================================================

fn extract_kotlin(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    ir: &mut IrFile,
) -> Result<(), DomainScanError> {
    ir.interfaces = extract_kt_interfaces(tree, source, path)?;
    ir.classes = extract_kt_classes(tree, source, path)?;
    ir.functions = extract_kt_functions(tree, source, path)?;
    ir.imports = extract_kt_imports(tree, source)?;
    ir.services = extract_kt_services(tree, source, path)?;
    ir.schemas = extract_kt_schemas(tree, source, path)?;
    Ok(())
}

// --- Kotlin Interfaces ---

fn extract_kt_interfaces(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<InterfaceDef>, DomainScanError> {
    let query = get_kt_query(&KT_INTERFACES_Q, KT_INTERFACES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut interfaces = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "interface.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "interface.def") else {
            continue;
        };

        // Only match actual interfaces (class_declaration with "interface" keyword)
        if !kt_is_interface(def_node) {
            continue;
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = kt_visibility(def_node, source);
        let generics = extract_kt_generics(def_node, source);
        let extends = extract_kt_supertypes(def_node, source);

        let body = kt_class_body(def_node);
        let methods = body
            .map(|b| extract_kt_interface_methods(b, source))
            .unwrap_or_default();

        interfaces.push(InterfaceDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            methods,
            properties: Vec::new(),
            language_kind: InterfaceKind::Interface,
            decorators: extract_kt_annotations(def_node, source),
        });
    }

    Ok(interfaces)
}

fn extract_kt_interface_methods(body: Node<'_>, source: &[u8]) -> Vec<MethodSignature> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "function_declaration" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let span = node_span(child);

        let parameters = extract_kt_function_params(child, source);
        let return_type = extract_kt_return_type(child, source);
        let has_default = kt_has_function_body(child);

        methods.push(MethodSignature {
            name,
            span,
            is_async: false,
            parameters,
            return_type,
            has_default,
        });
    }

    methods
}

// --- Kotlin Classes ---

fn extract_kt_classes(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ClassDef>, DomainScanError> {
    let query = get_kt_query(&KT_CLASSES_Q, KT_CLASSES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut classes = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "class.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "class.def") else {
            continue;
        };

        // Skip interfaces (handled separately)
        if kt_is_interface(def_node) {
            continue;
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = kt_visibility(def_node, source);
        let is_abstract = kt_has_class_modifier(def_node, source, "abstract");
        let generics = extract_kt_generics(def_node, source);
        let decorators = extract_kt_annotations(def_node, source);

        let supertypes = extract_kt_supertypes(def_node, source);
        let extends = supertypes.first().cloned();
        let implements = if supertypes.len() > 1 {
            supertypes[1..].to_vec()
        } else {
            Vec::new()
        };

        let body = kt_class_body(def_node);
        let mut methods = body
            .map(|b| extract_kt_class_methods(b, source, path))
            .unwrap_or_default();
        let properties = body
            .map(|b| extract_kt_class_properties(b, source))
            .unwrap_or_default();

        for method in &mut methods {
            method.owner = Some(name.clone());
        }

        classes.push(ClassDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            implements,
            methods,
            properties,
            is_abstract,
            decorators,
        });
    }

    Ok(classes)
}

fn extract_kt_class_methods(body: Node<'_>, source: &[u8], path: &Path) -> Vec<MethodDef> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "function_declaration" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let span = node_span(child);
        let visibility = kt_visibility(child, source);
        let decorators = extract_kt_annotations(child, source);
        let parameters = extract_kt_function_params(child, source);
        let return_type = extract_kt_return_type(child, source);

        methods.push(MethodDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async: false,
            is_static: false,
            is_generator: false,
            parameters,
            return_type,
            decorators,
            owner: None,
            implements: None,
        });
    }

    methods
}

fn extract_kt_class_properties(body: Node<'_>, source: &[u8]) -> Vec<PropertyDef> {
    let mut properties = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "property_declaration" {
            continue;
        }

        // Get variable_declaration child for name
        if let Some(var_decl) = find_named_child(child, "variable_declaration") {
            let name = var_decl
                .child_by_field_name("name")
                .or_else(|| find_named_child_any(var_decl, &["simple_identifier", "identifier"]))
                .map(|n| node_text(n, source))
                .unwrap_or_default();

            let type_annotation = extract_kt_property_type(child, source);
            let is_readonly = kt_property_is_val(child, source);

            properties.push(PropertyDef {
                name,
                type_annotation,
                is_optional: false,
                is_readonly,
                visibility: kt_visibility(child, source),
            });
        }
    }

    properties
}

// --- Kotlin Functions ---

fn extract_kt_functions(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<FunctionDef>, DomainScanError> {
    let query = get_kt_query(&KT_METHODS_Q, KT_METHODS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut functions = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "method.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "method.def") else {
            continue;
        };

        // Only top-level functions (parent is source_file)
        if let Some(parent) = def_node.parent() {
            if parent.kind() != "source_file" {
                continue;
            }
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = kt_visibility(def_node, source);
        let decorators = extract_kt_annotations(def_node, source);
        let parameters = extract_kt_function_params(def_node, source);
        let return_type = extract_kt_return_type(def_node, source);

        functions.push(FunctionDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async: false,
            is_generator: false,
            parameters,
            return_type,
            decorators,
        });
    }

    Ok(functions)
}

// --- Kotlin Imports ---

fn extract_kt_imports(tree: &Tree, source: &[u8]) -> Result<Vec<ImportDef>, DomainScanError> {
    let query = get_kt_query(&KT_IMPORTS_Q, KT_IMPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut imports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(def_node) = find_capture(&m, query, "import.def") else {
            continue;
        };

        let full_text = node_text(def_node, source);
        let span = node_span(def_node);

        let trimmed = full_text
            .trim()
            .trim_start_matches("import")
            .trim();

        let is_wildcard = trimmed.ends_with(".*");
        let source_path = if is_wildcard {
            trimmed.trim_end_matches(".*").to_string()
        } else if let Some(pos) = trimmed.rfind('.') {
            trimmed[..pos].to_string()
        } else {
            trimmed.to_string()
        };

        let symbols = if is_wildcard {
            Vec::new()
        } else if let Some(pos) = trimmed.rfind('.') {
            vec![ImportedSymbol {
                name: trimmed[pos + 1..].to_string(),
                alias: None,
                is_default: false,
                is_namespace: false,
            }]
        } else {
            Vec::new()
        };

        imports.push(ImportDef {
            source: source_path,
            symbols,
            is_wildcard,
            span,
        });
    }

    Ok(imports)
}

// --- Kotlin Services ---

fn extract_kt_services(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ServiceDef>, DomainScanError> {
    let query = get_kt_query(&KT_SERVICES_Q, KT_SERVICES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut services = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "service.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "service.def") else {
            continue;
        };

        let decorators = extract_kt_annotations(def_node, source);
        let kind = classify_jv_service_kind(&decorators);
        let Some(kind) = kind else { continue; };

        let name = node_text(name_node, source);
        let span = node_span(def_node);

        let body = kt_class_body(def_node);
        let methods = body
            .map(|b| extract_kt_class_methods(b, source, path))
            .unwrap_or_default();

        services.push(ServiceDef {
            name,
            file: path.to_path_buf(),
            span,
            kind,
            methods,
            dependencies: Vec::new(),
            decorators,
            routes: Vec::new(),
        });
    }

    Ok(services)
}

// --- Kotlin Schemas ---

fn extract_kt_schemas(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<SchemaDef>, DomainScanError> {
    let query = get_kt_query(&KT_SCHEMAS_Q, KT_SCHEMAS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut schemas = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "schema.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "schema.def") else {
            continue;
        };

        // Only data classes are schemas
        if !kt_has_class_modifier(def_node, source, "data") {
            continue;
        }

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = kt_visibility(def_node, source);

        let fields = extract_kt_data_class_fields(def_node, source);

        schemas.push(SchemaDef {
            name,
            file: path.to_path_buf(),
            span,
            kind: SchemaKind::DataTransfer,
            fields,
            source_framework: "kotlin-data-class".to_string(),
            table_name: None,
            derives: Vec::new(),
            visibility,
        });
    }

    Ok(schemas)
}

fn extract_kt_data_class_fields(node: Node<'_>, source: &[u8]) -> Vec<SchemaField> {
    let mut fields = Vec::new();

    // Find primary_constructor -> class_parameters -> class_parameter
    let Some(constructor) = find_named_child(node, "primary_constructor") else {
        return fields;
    };
    let Some(class_params) = find_named_child(constructor, "class_parameters") else {
        return fields;
    };

    let mut cp = class_params.walk();
    for param in class_params.named_children(&mut cp) {
        if param.kind() != "class_parameter" {
            continue;
        }

        let name = find_named_child_any(param, &["simple_identifier", "identifier"])
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let type_annotation = find_named_child_any(param, &["user_type", "nullable_type", "type"])
            .map(|n| node_text(n, source));

        fields.push(SchemaField {
            name,
            type_annotation,
            is_optional: false,
            is_primary_key: false,
            constraints: Vec::new(),
        });
    }

    fields
}

// --- Kotlin Helpers ---

fn kt_is_interface(node: Node<'_>) -> bool {
    // Check if the class_declaration has "interface" keyword
    let full_text_start = node.start_byte();
    // Walk children to check for "interface" keyword token
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "interface" && child.start_byte() >= full_text_start {
            return true;
        }
    }
    false
}

fn kt_class_body(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let result = node.named_children(&mut cursor)
        .find(|c| c.kind() == "class_body");
    result
}

fn kt_visibility(node: Node<'_>, source: &[u8]) -> Visibility {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mc = child.walk();
            for modifier in child.named_children(&mut mc) {
                if modifier.kind() == "visibility_modifier" {
                    let text = node_text(modifier, source);
                    return match text.as_str() {
                        "public" => Visibility::Public,
                        "private" => Visibility::Private,
                        "protected" => Visibility::Protected,
                        "internal" => Visibility::Internal,
                        _ => Visibility::Public,
                    };
                }
            }
        }
    }
    // Kotlin default visibility is public
    Visibility::Public
}

fn kt_has_class_modifier(node: Node<'_>, source: &[u8], modifier_name: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mc = child.walk();
            for modifier in child.named_children(&mut mc) {
                if modifier.kind() == "class_modifier" || modifier.kind() == "inheritance_modifier" {
                    let text = node_text(modifier, source);
                    if text == modifier_name {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn extract_kt_generics(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut generics = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "type_parameters" {
            let mut tc = child.walk();
            for tp in child.named_children(&mut tc) {
                if tp.kind() == "type_parameter" {
                    let mut inner = tp.walk();
                    for inner_child in tp.named_children(&mut inner) {
                        if inner_child.kind() == "type_identifier" || inner_child.kind() == "identifier" {
                            generics.push(node_text(inner_child, source));
                            break;
                        }
                    }
                }
            }
        }
    }
    generics
}

fn extract_kt_supertypes(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut supertypes = Vec::new();

    let Some(deleg_specs) = find_named_child(node, "delegation_specifiers") else {
        return supertypes;
    };

    let mut dc = deleg_specs.walk();
    for ds in deleg_specs.named_children(&mut dc) {
        if ds.kind() != "delegation_specifier" {
            continue;
        }

        if let Some(user_type) = find_named_child(ds, "user_type") {
            supertypes.push(node_text(user_type, source));
        } else if let Some(ctor_inv) = find_named_child(ds, "constructor_invocation") {
            // e.g. `BaseClass(args)` - extract just the type name
            if let Some(type_node) = ctor_inv.named_child(0) {
                supertypes.push(node_text(type_node, source));
            }
        }
    }

    supertypes
}

fn extract_kt_function_params(node: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut parameters = Vec::new();

    let Some(fvp) = find_named_child(node, "function_value_parameters") else {
        return parameters;
    };

    let mut pc = fvp.walk();
    for param in fvp.named_children(&mut pc) {
        if param.kind() != "parameter" {
            continue;
        }

        let name = find_named_child_any(param, &["simple_identifier", "identifier"])
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let type_annotation = find_named_child_any(param, &["user_type", "nullable_type", "type"])
            .map(|n| node_text(n, source));
        let has_default = find_named_child(param, "expression").is_some();

        parameters.push(Parameter {
            name,
            type_annotation,
            is_optional: has_default,
            has_default,
            is_rest: false,
        });
    }

    parameters
}

fn extract_kt_return_type(node: Node<'_>, source: &[u8]) -> Option<String> {
    // In Kotlin, return type follows the parameters, indicated by ":"
    // Look for a type child that is NOT inside type_parameters
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "user_type" || child.kind() == "nullable_type" {
            return Some(node_text(child, source));
        }
    }
    None
}

fn kt_has_function_body(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let result = node.named_children(&mut cursor)
        .any(|c| c.kind() == "function_body");
    result
}

fn kt_property_is_val(node: Node<'_>, source: &[u8]) -> bool {
    // Check if property starts with "val" (immutable)
    let text = node_text(node, source);
    let trimmed = text.trim_start();
    trimmed.starts_with("val ")
}

fn extract_kt_property_type(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "user_type" || child.kind() == "nullable_type" {
            return Some(node_text(child, source));
        }
    }
    None
}

fn extract_kt_annotation_from_node(ann: Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(user_type) = find_named_child(ann, "user_type") {
        Some(node_text(user_type, source))
    } else if let Some(ctor_inv) = find_named_child(ann, "constructor_invocation") {
        ctor_inv.named_child(0).map(|type_node| node_text(type_node, source))
    } else {
        None
    }
}

fn extract_kt_annotations(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut annotations = Vec::new();

    // Check modifiers child (normal case: annotations attached to the class)
    if let Some(modifiers) = find_named_child(node, "modifiers") {
        let mut mc = modifiers.walk();
        for modifier in modifiers.named_children(&mut mc) {
            if modifier.kind() != "annotation" {
                continue;
            }
            if let Some(name) = extract_kt_annotation_from_node(modifier, source) {
                annotations.push(name);
            }
        }
    }

    // Also check preceding siblings for annotations that tree-sitter-kotlin-ng
    // parses as separate nodes (happens when annotation has arguments like
    // @RequestMapping("/api/users") before a class — these get wrapped in
    // annotated_expression nodes)
    let mut sibling = node.prev_named_sibling();
    while let Some(sib) = sibling {
        if sib.kind() == "annotation" {
            if let Some(name) = extract_kt_annotation_from_node(sib, source) {
                annotations.push(name);
            }
            sibling = sib.prev_named_sibling();
        } else if sib.kind() == "annotated_expression" {
            // Recursively collect annotations from annotated_expression trees
            collect_kt_annotations_from_expr(sib, source, &mut annotations);
            sibling = sib.prev_named_sibling();
        } else {
            break;
        }
    }

    annotations
}

fn collect_kt_annotations_from_expr(node: Node<'_>, source: &[u8], annotations: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "annotation" {
            if let Some(name) = extract_kt_annotation_from_node(child, source) {
                annotations.push(name);
            }
        } else if child.kind() == "annotated_expression" {
            collect_kt_annotations_from_expr(child, source, annotations);
        }
    }
}

// ===========================================================================
// Scala extraction
// ===========================================================================

fn extract_scala(
    tree: &Tree,
    source: &[u8],
    path: &Path,
    ir: &mut IrFile,
) -> Result<(), DomainScanError> {
    ir.interfaces = extract_sc_traits(tree, source, path)?;
    ir.classes = extract_sc_classes(tree, source, path)?;
    ir.functions = extract_sc_functions(tree, source, path)?;
    ir.imports = extract_sc_imports(tree, source)?;
    Ok(())
}

// --- Scala Traits ---

fn extract_sc_traits(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<InterfaceDef>, DomainScanError> {
    let query = get_sc_query(&SC_TRAITS_Q, SC_TRAITS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut traits = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "interface.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "interface.def") else {
            continue;
        };
        let body_node = find_capture(&m, query, "interface.body");

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = sc_visibility(def_node, source);
        let generics = extract_sc_generics(def_node, source);
        let extends = extract_sc_extends(def_node, source);

        let methods = body_node
            .map(|b| extract_sc_trait_methods(b, source))
            .unwrap_or_default();

        traits.push(InterfaceDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends,
            methods,
            properties: Vec::new(),
            language_kind: InterfaceKind::Trait,
            decorators: Vec::new(),
        });
    }

    Ok(traits)
}

fn extract_sc_trait_methods(body: Node<'_>, source: &[u8]) -> Vec<MethodSignature> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "function_definition" && child.kind() != "function_declaration" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let span = node_span(child);

        let parameters = extract_sc_parameters(child, source);
        let return_type = child
            .child_by_field_name("return_type")
            .map(|rt| node_text(rt, source));

        let has_default = child.child_by_field_name("body").is_some();

        methods.push(MethodSignature {
            name,
            span,
            is_async: false,
            parameters,
            return_type,
            has_default,
        });
    }

    methods
}

// --- Scala Classes & Objects ---

fn extract_sc_classes(
    tree: &Tree,
    source: &[u8],
    path: &Path,
) -> Result<Vec<ClassDef>, DomainScanError> {
    let query = get_sc_query(&SC_CLASSES_Q, SC_CLASSES_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut classes = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(name_node) = find_capture(&m, query, "class.name") else {
            continue;
        };
        let Some(def_node) = find_capture(&m, query, "class.def") else {
            continue;
        };

        let name = node_text(name_node, source);
        let span = node_span(def_node);
        let visibility = sc_visibility(def_node, source);
        let is_abstract = sc_has_modifier(def_node, source, "abstract");
        let generics = extract_sc_generics(def_node, source);
        let extends = extract_sc_extends(def_node, source);

        let extends_first = extends.first().cloned();
        let implements = if extends.len() > 1 {
            extends[1..].to_vec()
        } else {
            Vec::new()
        };

        let body_node = find_capture(&m, query, "class.body");
        let mut methods = body_node
            .map(|b| extract_sc_class_methods(b, source, path))
            .unwrap_or_default();

        for method in &mut methods {
            method.owner = Some(name.clone());
        }

        let is_object = def_node.kind() == "object_definition";

        classes.push(ClassDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            generics,
            extends: extends_first,
            implements,
            methods,
            properties: Vec::new(),
            is_abstract: is_abstract || is_object,
            decorators: Vec::new(),
        });
    }

    Ok(classes)
}

fn extract_sc_class_methods(body: Node<'_>, source: &[u8], path: &Path) -> Vec<MethodDef> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "function_definition" && child.kind() != "function_declaration" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();
        let span = node_span(child);
        let visibility = sc_visibility(child, source);
        let parameters = extract_sc_parameters(child, source);
        let return_type = child
            .child_by_field_name("return_type")
            .map(|rt| node_text(rt, source));

        methods.push(MethodDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async: false,
            is_static: false,
            is_generator: false,
            parameters,
            return_type,
            decorators: Vec::new(),
            owner: None,
            implements: None,
        });
    }

    methods
}

// --- Scala Functions ---

fn extract_sc_functions(
    _tree: &Tree,
    _source: &[u8],
    _path: &Path,
) -> Result<Vec<FunctionDef>, DomainScanError> {
    // Scala top-level functions only exist in Scala 3 and are rare.
    // Methods inside objects/classes are handled via class extraction.
    Ok(Vec::new())
}

// --- Scala Imports ---

fn extract_sc_imports(tree: &Tree, source: &[u8]) -> Result<Vec<ImportDef>, DomainScanError> {
    let query = get_sc_query(&SC_IMPORTS_Q, SC_IMPORTS_SCM)?;
    let mut cursor = QueryCursor::new();
    let mut imports = Vec::new();

    for m in cursor.matches(query, tree.root_node(), source) {
        let Some(def_node) = find_capture(&m, query, "import.def") else {
            continue;
        };

        let full_text = node_text(def_node, source);
        let span = node_span(def_node);

        let trimmed = full_text
            .trim()
            .trim_start_matches("import")
            .trim();

        let is_wildcard = trimmed.ends_with("._") || trimmed.ends_with(".*");
        let source_path = if is_wildcard {
            trimmed
                .trim_end_matches("._")
                .trim_end_matches(".*")
                .to_string()
        } else if trimmed.contains('{') {
            // Multi-import: import java.util.{List, Map}
            if let Some(pos) = trimmed.find('{') {
                trimmed[..pos].trim_end_matches('.').to_string()
            } else {
                trimmed.to_string()
            }
        } else if let Some(pos) = trimmed.rfind('.') {
            trimmed[..pos].to_string()
        } else {
            trimmed.to_string()
        };

        let symbols = if is_wildcard {
            Vec::new()
        } else if trimmed.contains('{') {
            // Parse {List, Map} or {List => JList}
            if let Some(start) = trimmed.find('{') {
                if let Some(end) = trimmed.find('}') {
                    let inner = &trimmed[start + 1..end];
                    inner
                        .split(',')
                        .map(|s| {
                            let s = s.trim();
                            if let Some((name, alias)) = s.split_once("=>") {
                                ImportedSymbol {
                                    name: name.trim().to_string(),
                                    alias: Some(alias.trim().to_string()),
                                    is_default: false,
                                    is_namespace: false,
                                }
                            } else {
                                ImportedSymbol {
                                    name: s.to_string(),
                                    alias: None,
                                    is_default: false,
                                    is_namespace: false,
                                }
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else if let Some(pos) = trimmed.rfind('.') {
            vec![ImportedSymbol {
                name: trimmed[pos + 1..].to_string(),
                alias: None,
                is_default: false,
                is_namespace: false,
            }]
        } else {
            Vec::new()
        };

        imports.push(ImportDef {
            source: source_path,
            symbols,
            is_wildcard,
            span,
        });
    }

    Ok(imports)
}

// --- Scala Helpers ---

fn sc_visibility(node: Node<'_>, source: &[u8]) -> Visibility {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "access_modifier" {
            let text = node_text(child, source);
            if text.contains("private") {
                return Visibility::Private;
            } else if text.contains("protected") {
                return Visibility::Protected;
            }
        }
        if child.kind() == "modifiers" {
            let mut mc = child.walk();
            for modifier in child.named_children(&mut mc) {
                if modifier.kind() == "access_modifier" {
                    let text = node_text(modifier, source);
                    if text.contains("private") {
                        return Visibility::Private;
                    } else if text.contains("protected") {
                        return Visibility::Protected;
                    }
                }
            }
        }
    }
    Visibility::Public
}

fn sc_has_modifier(node: Node<'_>, source: &[u8], modifier_name: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "modifiers" {
            let text = node_text(child, source);
            if text.contains(modifier_name) {
                return true;
            }
        }
    }
    false
}

fn extract_sc_generics(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let Some(type_params) = node.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut generics = Vec::new();
    let mut cursor = type_params.walk();
    for child in type_params.named_children(&mut cursor) {
        if child.kind() == "identifier" || child.kind() == "type_identifier" {
            generics.push(node_text(child, source));
        } else if child.kind() == "type_parameter" {
            if let Some(name) = child.child_by_field_name("name") {
                generics.push(node_text(name, source));
            }
        }
    }
    generics
}

fn extract_sc_extends(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut extends = Vec::new();
    if let Some(extend_clause) = node.child_by_field_name("extend") {
        let mut cursor = extend_clause.walk();
        for child in extend_clause.named_children(&mut cursor) {
            match child.kind() {
                "type_identifier" | "generic_type" | "stable_type_identifier" => {
                    extends.push(node_text(child, source));
                }
                _ => {}
            }
        }
    }
    extends
}

fn extract_sc_parameters(node: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut parameters = Vec::new();
    let mut cursor = node.walk();

    for child in node.children_by_field_name("parameters", &mut cursor) {
        if child.kind() == "parameters" {
            let mut pc = child.walk();
            for param in child.named_children(&mut pc) {
                if param.kind() == "parameter" {
                    let name = param
                        .child_by_field_name("name")
                        .or_else(|| find_named_child(param, "identifier"))
                        .map(|n| node_text(n, source))
                        .unwrap_or_default();
                    let type_annotation = param
                        .child_by_field_name("type")
                        .map(|t| node_text(t, source));
                    let has_default = param
                        .child_by_field_name("default")
                        .is_some();

                    parameters.push(Parameter {
                        name,
                        type_annotation,
                        is_optional: has_default,
                        has_default,
                        is_rest: false,
                    });
                }
            }
        }
    }

    parameters
}

// ---------------------------------------------------------------------------
// Helper: Node text extraction
// ---------------------------------------------------------------------------

fn node_text(node: Node<'_>, source: &[u8]) -> String {
    source
        .get(node.start_byte()..node.end_byte())
        .and_then(|s| std::str::from_utf8(s).ok())
        .unwrap_or_default()
        .to_string()
}

fn node_span(node: Node<'_>) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start_line: start.row as u32,
        start_col: start.column as u32,
        end_line: end.row as u32,
        end_col: end.column as u32,
        byte_range: (node.start_byte(), node.end_byte()),
    }
}

fn strip_quotes(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string()
}

// ---------------------------------------------------------------------------
// Helper: Query capture lookup
// ---------------------------------------------------------------------------

fn find_capture<'tree>(
    m: &QueryMatch<'_, 'tree>,
    query: &Query,
    name: &str,
) -> Option<Node<'tree>> {
    let names = query.capture_names();
    m.captures.iter().find_map(|c| {
        let cap_name = names.get(c.index as usize)?;
        if *cap_name == name {
            Some(c.node)
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// Helper: TypeScript visibility
// ---------------------------------------------------------------------------

fn ts_visibility(node: Node<'_>) -> Visibility {
    // Walk up parents to find export_statement ancestor
    let mut current = node.parent();
    for _ in 0..4 {
        match current {
            Some(n) if n.kind() == "export_statement" => return Visibility::Public,
            Some(n) => current = n.parent(),
            None => break,
        }
    }
    Visibility::Private
}

// ---------------------------------------------------------------------------
// Helper: Generics (type parameters)
// ---------------------------------------------------------------------------

fn extract_ts_generics(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let Some(type_params) = node.child_by_field_name("type_parameters") else {
        return Vec::new();
    };
    let mut generics = Vec::new();
    let mut cursor = type_params.walk();
    for child in type_params.named_children(&mut cursor) {
        if child.kind() == "type_parameter" {
            if let Some(name) = child.child_by_field_name("name") {
                generics.push(node_text(name, source));
            }
        }
    }
    generics
}

// ---------------------------------------------------------------------------
// Helper: Interface extends
// ---------------------------------------------------------------------------

fn extract_ts_extends(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut extends = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "extends_type_clause" {
            let mut inner_cursor = child.walk();
            for ext_child in child.named_children(&mut inner_cursor) {
                if ext_child.kind() == "type_identifier" || ext_child.kind() == "generic_type" {
                    extends.push(node_text(ext_child, source));
                }
            }
        }
    }
    extends
}

// ---------------------------------------------------------------------------
// Helper: Class heritage (extends + implements)
// ---------------------------------------------------------------------------

fn extract_ts_class_heritage(node: Node<'_>, source: &[u8]) -> (Option<String>, Vec<String>) {
    let mut extends = None;
    let mut implements = Vec::new();
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "class_heritage" => {
                let mut inner_cursor = child.walk();
                for heritage_child in child.named_children(&mut inner_cursor) {
                    extract_heritage_clause(heritage_child, source, &mut extends, &mut implements);
                }
            }
            "extends_clause" | "implements_clause" => {
                extract_heritage_clause(child, source, &mut extends, &mut implements);
            }
            _ => {}
        }
    }

    (extends, implements)
}

fn extract_heritage_clause(
    clause: Node<'_>,
    source: &[u8],
    extends: &mut Option<String>,
    implements: &mut Vec<String>,
) {
    match clause.kind() {
        "extends_clause" => {
            let mut cursor = clause.walk();
            for child in clause.named_children(&mut cursor) {
                if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                    if extends.is_none() {
                        *extends = Some(node_text(child, source));
                    }
                    break;
                }
            }
        }
        "implements_clause" => {
            let mut cursor = clause.walk();
            for child in clause.named_children(&mut cursor) {
                if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                    implements.push(node_text(child, source));
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Helper: Decorators
// ---------------------------------------------------------------------------

fn extract_ts_decorators(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut decorators = Vec::new();

    // Check direct children
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "decorator" {
            let text = node_text(child, source);
            let trimmed = text.strip_prefix('@').unwrap_or(&text);
            decorators.push(trimmed.to_string());
        }
    }

    // For exported classes, decorators live under the export_statement parent
    if decorators.is_empty() {
        if let Some(parent) = node.parent() {
            if parent.kind() == "export_statement" {
                let mut parent_cursor = parent.walk();
                for child in parent.named_children(&mut parent_cursor) {
                    if child.kind() == "decorator" {
                        let text = node_text(child, source);
                        let trimmed = text.strip_prefix('@').unwrap_or(&text);
                        decorators.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    decorators
}

// ---------------------------------------------------------------------------
// Helper: Method signatures (inside interface body)
// ---------------------------------------------------------------------------

fn extract_ts_method_signatures(body: Node<'_>, source: &[u8]) -> Vec<MethodSignature> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "method_signature" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();

        let span = node_span(child);

        let params_node = child.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_ts_parameters(p, source))
            .unwrap_or_default();

        let return_type = child
            .child_by_field_name("return_type")
            .map(|rt| extract_type_annotation_text(rt, source));

        methods.push(MethodSignature {
            name,
            span,
            is_async: false, // Interface method signatures are not async
            parameters,
            return_type,
            has_default: false,
        });
    }

    methods
}

// ---------------------------------------------------------------------------
// Helper: Properties (inside interface body)
// ---------------------------------------------------------------------------

fn extract_ts_properties(body: Node<'_>, source: &[u8]) -> Vec<PropertyDef> {
    let mut properties = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "property_signature" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();

        let type_annotation = child
            .child_by_field_name("type")
            .map(|t| extract_type_annotation_text(t, source));

        // Check for optional marker: look for "?" anonymous child
        let mut walk = child.walk();
        let is_optional = child.children(&mut walk).any(|c| c.kind() == "?");

        let is_readonly = {
            let mut walk2 = child.walk();
            let result = child.children(&mut walk2).any(|c| c.kind() == "readonly");
            result
        };

        properties.push(PropertyDef {
            name,
            type_annotation,
            is_optional,
            is_readonly,
            visibility: Visibility::Public, // Interface properties are always public
        });
    }

    properties
}

// ---------------------------------------------------------------------------
// Helper: Class methods
// ---------------------------------------------------------------------------

fn extract_ts_class_methods(body: Node<'_>, source: &[u8], path: &Path) -> Vec<MethodDef> {
    let mut methods = Vec::new();
    let mut cursor = body.walk();
    // Decorators in class bodies are siblings of method_definition, not children.
    // Track pending decorators and attach them to the next method.
    let mut pending_decorators: Vec<String> = Vec::new();

    for child in body.named_children(&mut cursor) {
        if child.kind() == "decorator" {
            let text = node_text(child, source);
            let trimmed = text.strip_prefix('@').unwrap_or(&text);
            pending_decorators.push(trimmed.to_string());
            continue;
        }

        if child.kind() != "method_definition" {
            pending_decorators.clear();
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();

        // Skip constructor
        if name == "constructor" {
            pending_decorators.clear();
            continue;
        }

        let span = node_span(child);
        let visibility = extract_member_visibility(child, source);
        let is_async = has_async_keyword(child);
        let is_static = has_static_keyword(child);
        let is_generator = has_child_of_kind(child, "*");

        let params_node = child.child_by_field_name("parameters");
        let parameters = params_node
            .map(|p| extract_ts_parameters(p, source))
            .unwrap_or_default();

        let return_type = child
            .child_by_field_name("return_type")
            .map(|rt| extract_type_annotation_text(rt, source));

        let decorators = std::mem::take(&mut pending_decorators);

        methods.push(MethodDef {
            name,
            file: path.to_path_buf(),
            span,
            visibility,
            is_async,
            is_static,
            is_generator,
            parameters,
            return_type,
            decorators,
            owner: None, // Set by caller
            implements: None,
        });
    }

    methods
}

// ---------------------------------------------------------------------------
// Helper: Class properties
// ---------------------------------------------------------------------------

fn extract_ts_class_properties(body: Node<'_>, source: &[u8]) -> Vec<PropertyDef> {
    let mut properties = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        // tree-sitter-typescript uses "public_field_definition" for class fields
        if child.kind() != "public_field_definition" && child.kind() != "property_definition" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();

        let type_annotation = child
            .child_by_field_name("type")
            .map(|t| extract_type_annotation_text(t, source));

        let mut walk = child.walk();
        let is_optional = child.children(&mut walk).any(|c| c.kind() == "?");

        let is_readonly = {
            let mut walk2 = child.walk();
            let result = child.children(&mut walk2).any(|c| c.kind() == "readonly");
            result
        };

        let visibility = extract_member_visibility(child, source);

        properties.push(PropertyDef {
            name,
            type_annotation,
            is_optional,
            is_readonly,
            visibility,
        });
    }

    properties
}

// ---------------------------------------------------------------------------
// Helper: Parameters
// ---------------------------------------------------------------------------

fn extract_ts_parameters(params_node: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut parameters = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.named_children(&mut cursor) {
        match child.kind() {
            "required_parameter" | "optional_parameter" => {
                let name = child
                    .child_by_field_name("pattern")
                    .map(|n| node_text(n, source))
                    .unwrap_or_default();

                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| extract_type_annotation_text(t, source));

                let is_optional = child.kind() == "optional_parameter";
                let has_default = child.child_by_field_name("value").is_some();

                parameters.push(Parameter {
                    name,
                    type_annotation,
                    is_optional: is_optional || has_default,
                    has_default,
                    is_rest: false,
                });
            }
            "rest_parameter" => {
                let name = if let Some(pat) = child.child_by_field_name("pattern") {
                    node_text(pat, source)
                } else {
                    let mut inner_cursor = child.walk();
                    let id_node = child
                        .named_children(&mut inner_cursor)
                        .find(|c| c.kind() == "identifier");
                    id_node
                        .map(|n| node_text(n, source))
                        .unwrap_or_default()
                };

                let type_annotation = child
                    .child_by_field_name("type")
                    .map(|t| extract_type_annotation_text(t, source));

                parameters.push(Parameter {
                    name,
                    type_annotation,
                    is_optional: true,
                    has_default: false,
                    is_rest: true,
                });
            }
            _ => {}
        }
    }

    parameters
}

// ---------------------------------------------------------------------------
// Helper: Type annotation text
// ---------------------------------------------------------------------------

fn extract_type_annotation_text(node: Node<'_>, source: &[u8]) -> String {
    // type_annotation nodes contain ": TypeName"; strip the colon
    let text = node_text(node, source);
    text.strip_prefix(':').unwrap_or(&text).trim().to_string()
}

// ---------------------------------------------------------------------------
// Helper: Import symbols
// ---------------------------------------------------------------------------

fn extract_ts_import_symbols(
    import_node: Node<'_>,
    source: &[u8],
) -> (Vec<ImportedSymbol>, bool) {
    let mut symbols = Vec::new();
    let mut is_wildcard = false;

    let mut cursor = import_node.walk();
    for child in import_node.named_children(&mut cursor) {
        if child.kind() == "import_clause" {
            extract_import_clause_symbols(child, source, &mut symbols, &mut is_wildcard);
        }
    }

    (symbols, is_wildcard)
}

fn extract_import_clause_symbols(
    clause: Node<'_>,
    source: &[u8],
    symbols: &mut Vec<ImportedSymbol>,
    is_wildcard: &mut bool,
) {
    let mut cursor = clause.walk();
    for child in clause.named_children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                // Default import: import Foo from '...'
                symbols.push(ImportedSymbol {
                    name: node_text(child, source),
                    alias: None,
                    is_default: true,
                    is_namespace: false,
                });
            }
            "named_imports" => {
                let mut inner_cursor = child.walk();
                for specifier in child.named_children(&mut inner_cursor) {
                    if specifier.kind() == "import_specifier" {
                        let name = specifier
                            .child_by_field_name("name")
                            .map(|n| node_text(n, source))
                            .unwrap_or_default();
                        let alias = specifier
                            .child_by_field_name("alias")
                            .map(|a| node_text(a, source));
                        symbols.push(ImportedSymbol {
                            name,
                            alias,
                            is_default: false,
                            is_namespace: false,
                        });
                    }
                }
            }
            "namespace_import" => {
                *is_wildcard = true;
                let mut inner_cursor = child.walk();
                let id_node = child
                    .named_children(&mut inner_cursor)
                    .find(|c| c.kind() == "identifier");
                if let Some(id) = id_node {
                    symbols.push(ImportedSymbol {
                        name: node_text(id, source),
                        alias: None,
                        is_default: false,
                        is_namespace: true,
                    });
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: Export names
// ---------------------------------------------------------------------------

fn extract_ts_export_names(
    export_node: Node<'_>,
    source: &[u8],
    is_default: bool,
    re_export_source: &Option<String>,
) -> Vec<(String, ExportKind)> {
    let mut names = Vec::new();

    // Handle namespace re-export: export * from './module'
    let mut all_cursor = export_node.walk();
    let has_wildcard = export_node
        .children(&mut all_cursor)
        .any(|c| c.kind() == "*");
    if has_wildcard && re_export_source.is_some() {
        names.push(("*".to_string(), ExportKind::ReExport));
        return names;
    }

    let mut cursor = export_node.walk();
    for child in export_node.named_children(&mut cursor) {
        match child.kind() {
            // export { name1, name2 } [from '...']
            "export_clause" => {
                let mut inner_cursor = child.walk();
                for specifier in child.named_children(&mut inner_cursor) {
                    if specifier.kind() == "export_specifier" {
                        let name = specifier
                            .child_by_field_name("name")
                            .map(|n| node_text(n, source))
                            .unwrap_or_else(|| node_text(specifier, source));
                        let kind = if re_export_source.is_some() {
                            ExportKind::ReExport
                        } else {
                            ExportKind::Named
                        };
                        names.push((name, kind));
                    }
                }
            }
            // export default expr
            _ if is_default => {
                let name = match child.kind() {
                    "identifier" => node_text(child, source),
                    "function_declaration"
                    | "class_declaration"
                    | "abstract_class_declaration" => child
                        .child_by_field_name("name")
                        .map(|n| node_text(n, source))
                        .unwrap_or_else(|| "default".to_string()),
                    _ => "default".to_string(),
                };
                names.push((name, ExportKind::Default));
                break;
            }
            // export const/function/class/interface/type/enum
            "function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    names.push((node_text(name_node, source), ExportKind::Named));
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                let mut inner_cursor = child.walk();
                for decl in child.named_children(&mut inner_cursor) {
                    if decl.kind() == "variable_declarator" {
                        if let Some(name_node) = decl.child_by_field_name("name") {
                            names.push((node_text(name_node, source), ExportKind::Named));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    names
}

// ---------------------------------------------------------------------------
// Helper: Keyword modifiers
// ---------------------------------------------------------------------------

fn has_async_keyword(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| c.kind() == "async");
    result
}

fn has_static_keyword(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| c.kind() == "static");
    result
}

fn has_child_of_kind(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| c.kind() == kind);
    result
}

/// Find first named child of a given kind.
fn find_named_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let count = node.named_child_count();
    for i in 0..count {
        if let Some(child) = node.named_child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

/// Find first named child matching any of the given kinds.
fn find_named_child_any<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let count = node.named_child_count();
    for i in 0..count {
        if let Some(child) = node.named_child(i) {
            if kinds.contains(&child.kind()) {
                return Some(child);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Helper: Member visibility
// ---------------------------------------------------------------------------

fn extract_member_visibility(node: Node<'_>, source: &[u8]) -> Visibility {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "accessibility_modifier" {
            let text = node_text(child, source);
            return match text.as_str() {
                "public" => Visibility::Public,
                "private" => Visibility::Private,
                "protected" => Visibility::Protected,
                _ => Visibility::Public,
            };
        }
    }
    // Default for class members in TypeScript
    Visibility::Public
}

// ---------------------------------------------------------------------------
// Helper: Service classification
// ---------------------------------------------------------------------------

fn classify_service_kind(decorators: &[String]) -> Option<ServiceKind> {
    for dec in decorators {
        let name = dec.split('(').next().unwrap_or_default();
        match name {
            "Controller" => return Some(ServiceKind::HttpController),
            "Injectable" | "Service" => return Some(ServiceKind::Microservice),
            "Resolver" => return Some(ServiceKind::GraphqlResolver),
            "Middleware" => return Some(ServiceKind::Middleware),
            "Processor" | "Process" => return Some(ServiceKind::Worker),
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Helper: Constructor dependency extraction
// ---------------------------------------------------------------------------

fn extract_ts_constructor_deps(body: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut deps = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        if child.kind() != "method_definition" {
            continue;
        }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(n, source))
            .unwrap_or_default();

        if name == "constructor" {
            if let Some(params) = child.child_by_field_name("parameters") {
                let mut params_cursor = params.walk();
                for param in params.named_children(&mut params_cursor) {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        let type_text = extract_type_annotation_text(type_node, source);
                        deps.push(type_text);
                    }
                }
            }
        }
    }

    deps
}

// ---------------------------------------------------------------------------
// Helper: Route extraction from method decorators
// ---------------------------------------------------------------------------

fn extract_ts_routes(methods: &[MethodDef]) -> Vec<RouteDef> {
    let mut routes = Vec::new();
    for method in methods {
        for dec in &method.decorators {
            let dec_name = dec.split('(').next().unwrap_or_default();
            let http_method = match dec_name {
                "Get" => Some(HttpMethod::Get),
                "Post" => Some(HttpMethod::Post),
                "Put" => Some(HttpMethod::Put),
                "Patch" => Some(HttpMethod::Patch),
                "Delete" => Some(HttpMethod::Delete),
                "Head" => Some(HttpMethod::Head),
                "Options" => Some(HttpMethod::Options),
                _ => None,
            };
            if let Some(http_method) = http_method {
                let path = extract_decorator_string_arg(dec);
                routes.push(RouteDef {
                    method: http_method,
                    path,
                    handler: method.name.clone(),
                });
            }
        }
    }
    routes
}

fn extract_decorator_string_arg(decorator: &str) -> String {
    if let Some(start) = decorator.find('(') {
        let inner = &decorator[start + 1..];
        if let Some(end) = inner.rfind(')') {
            let arg = inner[..end].trim();
            return strip_quotes(arg);
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Helper: Schema classification
// ---------------------------------------------------------------------------

fn classify_schema_member(obj: &str, prop: &str) -> (Option<SchemaKind>, String, Option<String>) {
    match (obj, prop) {
        ("Schema" | "S", "Struct") => (
            Some(SchemaKind::ValidationSchema),
            "effect-schema".to_string(),
            None,
        ),
        ("z", "object") | ("z", "enum") => (
            Some(SchemaKind::ValidationSchema),
            "zod".to_string(),
            None,
        ),
        _ => (None, String::new(), None),
    }
}

fn classify_schema_function(fn_name: &str) -> (Option<SchemaKind>, String, Option<String>) {
    match fn_name {
        "pgTable" | "mysqlTable" | "sqliteTable" => (
            Some(SchemaKind::OrmModel),
            "drizzle".to_string(),
            None,
        ),
        _ => (None, String::new(), None),
    }
}

// ---------------------------------------------------------------------------
// Schema field parsing
// ---------------------------------------------------------------------------

/// Parse schema fields from the raw source text of the schema arguments.
/// Best-effort parser for common patterns like `{ name: z.string(), age: z.number() }`.
pub fn parse_schema_fields(source: &str) -> Vec<SchemaField> {
    let mut fields = Vec::new();
    let mut inner = source.trim();

    // Strip outer parens then braces (handles `({...})` from tree-sitter arguments node)
    if inner.starts_with('(') && inner.ends_with(')') {
        inner = inner[1..inner.len() - 1].trim();
    }
    if inner.starts_with('{') && inner.ends_with('}') {
        inner = inner[1..inner.len() - 1].trim();
    }

    let parts = split_top_level(inner, ',');

    for part in &parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Parse "name: type" patterns
        if let Some((name, value)) = part.split_once(':') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();

            let is_optional = value.contains(".optional()") || value.contains(".nullable()");
            let is_primary_key = value.contains(".primaryKey()") || value.contains("primaryKey");

            let mut constraints = Vec::new();
            if value.contains(".unique()") {
                constraints.push("unique".to_string());
            }
            if value.contains(".nullable()") {
                constraints.push("nullable".to_string());
            }
            if value.contains(".default(") {
                constraints.push("default".to_string());
            }

            fields.push(SchemaField {
                name,
                type_annotation: Some(value),
                is_optional,
                is_primary_key,
                constraints,
            });
        }
    }

    fields
}

/// Split a string by delimiter at the top level only
/// (not inside parentheses, brackets, or braces).
fn split_top_level(s: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;

    for ch in s.chars() {
        match ch {
            '(' | '[' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(ch);
            }
            c if c == delimiter && depth == 0 => {
                parts.push(current.clone());
                current.clear();
            }
            c => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quotes() {
        assert_eq!(strip_quotes("'hello'"), "hello");
        assert_eq!(strip_quotes("\"hello\""), "hello");
        assert_eq!(strip_quotes("`hello`"), "hello");
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn test_classify_service_kind() {
        assert_eq!(
            classify_service_kind(&["Controller()".to_string()]),
            Some(ServiceKind::HttpController)
        );
        assert_eq!(
            classify_service_kind(&["Injectable()".to_string()]),
            Some(ServiceKind::Microservice)
        );
        assert_eq!(
            classify_service_kind(&["Component()".to_string()]),
            None
        );
    }

    #[test]
    fn test_classify_schema_member() {
        let (kind, fw, _) = classify_schema_member("Schema", "Struct");
        assert_eq!(kind, Some(SchemaKind::ValidationSchema));
        assert_eq!(fw, "effect-schema");

        let (kind, fw, _) = classify_schema_member("z", "object");
        assert_eq!(kind, Some(SchemaKind::ValidationSchema));
        assert_eq!(fw, "zod");

        let (kind, _, _) = classify_schema_member("foo", "bar");
        assert_eq!(kind, None);
    }

    #[test]
    fn test_classify_schema_function() {
        let (kind, fw, _) = classify_schema_function("pgTable");
        assert_eq!(kind, Some(SchemaKind::OrmModel));
        assert_eq!(fw, "drizzle");

        let (kind, _, _) = classify_schema_function("createRouter");
        assert_eq!(kind, None);
    }

    #[test]
    fn test_parse_schema_fields() {
        let fields = parse_schema_fields("{ name: z.string(), age: z.number().optional() }");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "name");
        assert!(!fields[0].is_optional);
        assert_eq!(fields[1].name, "age");
        assert!(fields[1].is_optional);
    }

    #[test]
    fn test_parse_schema_fields_with_constraints() {
        let fields = parse_schema_fields(
            "{ id: serial('id').primaryKey(), email: varchar('email', { length: 255 }).unique() }",
        );
        assert_eq!(fields.len(), 2);
        assert!(fields[0].is_primary_key);
        assert!(fields[1].constraints.contains(&"unique".to_string()));
    }

    #[test]
    fn test_split_top_level() {
        let parts = split_top_level("a, b(1, 2), c", ',');
        assert_eq!(parts, vec!["a", " b(1, 2)", " c"]);
    }

    #[test]
    fn test_split_top_level_nested() {
        let parts = split_top_level("a, b({x: 1, y: 2}), c", ',');
        assert_eq!(parts, vec!["a", " b({x: 1, y: 2})", " c"]);
    }

    #[test]
    fn test_extract_decorator_string_arg() {
        assert_eq!(extract_decorator_string_arg("Get('/users')"), "/users");
        assert_eq!(extract_decorator_string_arg("Get()"), "");
        assert_eq!(extract_decorator_string_arg("Injectable"), "");
    }

    #[test]
    fn test_ts_query_compilation() {
        // Verify all TypeScript queries compile successfully
        let lang = tree_sitter_typescript::language_typescript();
        let queries = [
            ("interfaces", TS_INTERFACES_SCM),
            ("classes", TS_CLASSES_SCM),
            ("methods", TS_METHODS_SCM),
            ("functions", TS_FUNCTIONS_SCM),
            ("types", TS_TYPES_SCM),
            ("imports", TS_IMPORTS_SCM),
            ("exports", TS_EXPORTS_SCM),
            ("services", TS_SERVICES_SCM),
            ("schemas", TS_SCHEMAS_SCM),
        ];
        for (name, source) in queries {
            let result = Query::new(&lang, source);
            assert!(result.is_ok(), "Failed to compile {name}.scm: {:?}", result.err());
        }
    }

    #[test]
    fn test_java_query_compilation() {
        let lang = tree_sitter_java::language();
        let queries = [
            ("interfaces", JV_INTERFACES_SCM),
            ("classes", JV_CLASSES_SCM),
            ("methods", JV_METHODS_SCM),
            ("imports", JV_IMPORTS_SCM),
            ("services", JV_SERVICES_SCM),
            ("schemas", JV_SCHEMAS_SCM),
        ];
        for (name, source) in queries {
            let result = Query::new(&lang, source);
            assert!(result.is_ok(), "Failed to compile java/{name}.scm: {:?}", result.err());
        }
    }

    #[test]
    fn test_kotlin_query_compilation() {
        let lang = crate::parser::kotlin_language();
        let queries = [
            ("interfaces", KT_INTERFACES_SCM),
            ("classes", KT_CLASSES_SCM),
            ("methods", KT_METHODS_SCM),
            ("imports", KT_IMPORTS_SCM),
            ("services", KT_SERVICES_SCM),
            ("schemas", KT_SCHEMAS_SCM),
        ];
        for (name, source) in queries {
            let result = Query::new(&lang, source);
            assert!(result.is_ok(), "Failed to compile kotlin/{name}.scm: {:?}", result.err());
        }
    }

    #[test]
    fn test_scala_query_compilation() {
        let lang = crate::parser::scala_language();
        let queries = [
            ("traits", SC_TRAITS_SCM),
            ("classes", SC_CLASSES_SCM),
            ("methods", SC_METHODS_SCM),
            ("imports", SC_IMPORTS_SCM),
        ];
        for (name, source) in queries {
            let result = Query::new(&lang, source);
            assert!(result.is_ok(), "Failed to compile scala/{name}.scm: {:?}", result.err());
        }
    }
}
