//! Query engine: loads .scm query files, compiles tree-sitter queries lazily,
//! and dispatches captures to IR types.

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use tree_sitter::{Node, Query, QueryCursor, QueryMatch, Tree};

use crate::ir::*;
use crate::DomainScanError;

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
}
