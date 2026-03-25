//! Integration tests for TypeScript query extraction.
//! Each test parses a real TypeScript fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from source string
fn extract_ts(source: &str) -> IrFile {
    let tree = parse_source(source.as_bytes(), Language::TypeScript)
        .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new("test.ts"),
        Language::TypeScript,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract: {e}"))
}

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/typescript/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::TypeScript)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::TypeScript,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// interfaces.scm tests
// =========================================================================

#[test]
fn test_interfaces_fixture_count() {
    let ir = extract_fixture("interfaces.ts");
    assert_eq!(ir.interfaces.len(), 5, "Expected 5 interfaces");
}

#[test]
fn test_interface_names() {
    let ir = extract_fixture("interfaces.ts");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"IUserService"));
    assert!(names.contains(&"ILogger"));
    assert!(names.contains(&"IRepository"));
    assert!(names.contains(&"IConfig"));
    assert!(names.contains(&"IEventEmitter"));
}

#[test]
fn test_interface_methods() {
    let ir = extract_fixture("interfaces.ts");
    let user_svc = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IUserService")
        .expect("IUserService not found");
    assert_eq!(user_svc.methods.len(), 3);
    let method_names: Vec<&str> = user_svc.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"getUser"));
    assert!(method_names.contains(&"createUser"));
    assert!(method_names.contains(&"deleteUser"));
}

#[test]
fn test_interface_method_parameters() {
    let ir = extract_fixture("interfaces.ts");
    let user_svc = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IUserService")
        .expect("IUserService not found");
    let get_user = user_svc
        .methods
        .iter()
        .find(|m| m.name == "getUser")
        .expect("getUser not found");
    assert_eq!(get_user.parameters.len(), 1);
    assert_eq!(get_user.parameters[0].name, "id");
    assert_eq!(
        get_user.parameters[0].type_annotation.as_deref(),
        Some("string")
    );
}

#[test]
fn test_interface_properties() {
    let ir = extract_fixture("interfaces.ts");
    let config = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IConfig")
        .expect("IConfig not found");
    assert_eq!(config.properties.len(), 3);

    let debug = config
        .properties
        .iter()
        .find(|p| p.name == "debug")
        .expect("debug not found");
    assert!(debug.is_optional);

    let api_url = config
        .properties
        .iter()
        .find(|p| p.name == "apiUrl")
        .expect("apiUrl not found");
    assert!(api_url.is_readonly);
}

#[test]
fn test_interface_generics() {
    let ir = extract_fixture("interfaces.ts");
    let repo = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IRepository")
        .expect("IRepository not found");
    assert_eq!(repo.generics, vec!["T"]);
}

#[test]
fn test_interface_extends() {
    let ir = extract_fixture("interfaces.ts");
    let emitter = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IEventEmitter")
        .expect("IEventEmitter not found");
    assert!(emitter.extends.contains(&"IDisposable".to_string()));
}

#[test]
fn test_interface_visibility() {
    let ir = extract_fixture("interfaces.ts");
    let user_svc = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IUserService")
        .expect("IUserService not found");
    assert_eq!(user_svc.visibility, Visibility::Public);

    let logger = ir
        .interfaces
        .iter()
        .find(|i| i.name == "ILogger")
        .expect("ILogger not found");
    assert_eq!(logger.visibility, Visibility::Private);
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_classes_fixture_count() {
    let ir = extract_fixture("classes.ts");
    assert_eq!(ir.classes.len(), 3, "Expected 3 classes");
}

#[test]
fn test_class_names() {
    let ir = extract_fixture("classes.ts");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"));
    assert!(names.contains(&"BaseRepository"));
    assert!(names.contains(&"Logger"));
}

#[test]
fn test_class_abstract() {
    let ir = extract_fixture("classes.ts");
    let base_repo = ir
        .classes
        .iter()
        .find(|c| c.name == "BaseRepository")
        .expect("BaseRepository not found");
    assert!(base_repo.is_abstract);

    let user_svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    assert!(!user_svc.is_abstract);
}

#[test]
fn test_class_implements() {
    let ir = extract_fixture("classes.ts");
    let user_svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    assert!(user_svc.implements.contains(&"IUserService".to_string()));
}

#[test]
fn test_class_methods() {
    let ir = extract_fixture("classes.ts");
    let user_svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    // constructor is skipped, so we expect getUser, createUser, fromConfig
    let method_names: Vec<&str> = user_svc.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"getUser"));
    assert!(method_names.contains(&"createUser"));
    assert!(method_names.contains(&"fromConfig"));
}

#[test]
fn test_class_method_async() {
    let ir = extract_fixture("classes.ts");
    let user_svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    let get_user = user_svc
        .methods
        .iter()
        .find(|m| m.name == "getUser")
        .expect("getUser not found");
    assert!(get_user.is_async);

    let from_config = user_svc
        .methods
        .iter()
        .find(|m| m.name == "fromConfig")
        .expect("fromConfig not found");
    assert!(!from_config.is_async);
    assert!(from_config.is_static);
}

#[test]
fn test_class_method_owner() {
    let ir = extract_fixture("classes.ts");
    let user_svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    for method in &user_svc.methods {
        assert_eq!(method.owner.as_deref(), Some("UserService"));
    }
}

#[test]
fn test_class_method_visibility() {
    let ir = extract_fixture("classes.ts");
    let logger = ir
        .classes
        .iter()
        .find(|c| c.name == "Logger")
        .expect("Logger not found");
    let format = logger
        .methods
        .iter()
        .find(|m| m.name == "formatMessage")
        .expect("formatMessage not found");
    assert_eq!(format.visibility, Visibility::Protected);
}

// =========================================================================
// functions.scm tests
// =========================================================================

#[test]
fn test_functions_fixture_count() {
    let ir = extract_fixture("functions.ts");
    // add, fetchUser, privateHelper, multiply, fetchData, internalTransform, processEvent
    assert!(
        ir.functions.len() >= 7,
        "Expected at least 7 functions, got {}",
        ir.functions.len()
    );
}

#[test]
fn test_function_declaration() {
    let ir = extract_fixture("functions.ts");
    let add = ir
        .functions
        .iter()
        .find(|f| f.name == "add")
        .expect("add not found");
    assert_eq!(add.parameters.len(), 2);
    assert!(!add.is_async);
    assert_eq!(add.visibility, Visibility::Public);
}

#[test]
fn test_async_function() {
    let ir = extract_fixture("functions.ts");
    let fetch_user = ir
        .functions
        .iter()
        .find(|f| f.name == "fetchUser")
        .expect("fetchUser not found");
    assert!(fetch_user.is_async);
}

#[test]
fn test_arrow_function() {
    let ir = extract_fixture("functions.ts");
    let multiply = ir
        .functions
        .iter()
        .find(|f| f.name == "multiply")
        .expect("multiply not found");
    assert_eq!(multiply.parameters.len(), 2);
    assert!(!multiply.is_async);
}

#[test]
fn test_async_arrow_function() {
    let ir = extract_fixture("functions.ts");
    let fetch_data = ir
        .functions
        .iter()
        .find(|f| f.name == "fetchData")
        .expect("fetchData not found");
    assert!(fetch_data.is_async);
}

#[test]
fn test_function_expression() {
    let ir = extract_fixture("functions.ts");
    let process = ir
        .functions
        .iter()
        .find(|f| f.name == "processEvent")
        .expect("processEvent not found");
    assert!(!process.is_async);
}

// =========================================================================
// types.scm tests
// =========================================================================

#[test]
fn test_types_fixture_count() {
    let ir = extract_fixture("types.ts");
    assert!(
        ir.type_aliases.len() >= 5,
        "Expected at least 5 type aliases"
    );
}

#[test]
fn test_type_alias_simple() {
    let ir = extract_fixture("types.ts");
    let user_id = ir
        .type_aliases
        .iter()
        .find(|t| t.name == "UserId")
        .expect("UserId not found");
    assert_eq!(user_id.target, "string");
    assert_eq!(user_id.visibility, Visibility::Public);
}

#[test]
fn test_type_alias_generic() {
    let ir = extract_fixture("types.ts");
    let result = ir
        .type_aliases
        .iter()
        .find(|t| t.name == "Result")
        .expect("Result not found");
    assert!(result.generics.contains(&"T".to_string()));
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_imports_fixture_count() {
    let ir = extract_fixture("imports.ts");
    assert!(
        ir.imports.len() >= 4,
        "Expected at least 4 imports, got {}",
        ir.imports.len()
    );
}

#[test]
fn test_named_import() {
    let ir = extract_fixture("imports.ts");
    let user_import = ir
        .imports
        .iter()
        .find(|i| i.source == "./services/user")
        .expect("user import not found");
    assert!(!user_import.is_wildcard);
    let symbol_names: Vec<&str> = user_import
        .symbols
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert!(symbol_names.contains(&"UserService"));
    assert!(symbol_names.contains(&"UserDto"));
}

#[test]
fn test_namespace_import() {
    let ir = extract_fixture("imports.ts");
    let utils_import = ir
        .imports
        .iter()
        .find(|i| i.source == "./utils")
        .expect("utils import not found");
    assert!(utils_import.is_wildcard);
    assert!(utils_import
        .symbols
        .iter()
        .any(|s| s.is_namespace && s.name == "utils"));
}

#[test]
fn test_default_import() {
    let ir = extract_fixture("imports.ts");
    let express_import = ir
        .imports
        .iter()
        .find(|i| i.source == "express")
        .expect("express import not found");
    assert!(express_import
        .symbols
        .iter()
        .any(|s| s.is_default && s.name == "express"));
}

#[test]
fn test_aliased_import() {
    let ir = extract_fixture("imports.ts");
    let fs_import = ir
        .imports
        .iter()
        .find(|i| i.source == "fs/promises")
        .expect("fs/promises import not found");
    let read_symbol = fs_import
        .symbols
        .iter()
        .find(|s| s.name == "readFile")
        .expect("readFile not found");
    assert_eq!(read_symbol.alias.as_deref(), Some("read"));
}

// =========================================================================
// exports.scm tests
// =========================================================================

#[test]
fn test_exports_fixture() {
    let ir = extract_fixture("exports.ts");
    assert!(!ir.exports.is_empty(), "Expected exports");
}

#[test]
fn test_named_export() {
    let ir = extract_fixture("exports.ts");
    let names: Vec<&str> = ir.exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"API_VERSION"), "Missing API_VERSION export");
    assert!(names.contains(&"createApp"), "Missing createApp export");
    assert!(names.contains(&"Router"), "Missing Router export");
}

#[test]
fn test_default_export() {
    let ir = extract_fixture("exports.ts");
    let default_exports: Vec<&ExportDef> = ir
        .exports
        .iter()
        .filter(|e| e.kind == ExportKind::Default)
        .collect();
    assert!(
        !default_exports.is_empty(),
        "Expected at least one default export"
    );
}

#[test]
fn test_re_export() {
    let ir = extract_fixture("exports.ts");
    let re_exports: Vec<&ExportDef> = ir
        .exports
        .iter()
        .filter(|e| e.kind == ExportKind::ReExport)
        .collect();
    assert!(!re_exports.is_empty(), "Expected re-exports");
    assert!(re_exports.iter().any(|e| e.source.is_some()));
}

// =========================================================================
// methods.scm tests (methods extracted via class extraction)
// =========================================================================

#[test]
fn test_methods_fixture() {
    let ir = extract_fixture("methods.ts");
    assert_eq!(ir.classes.len(), 1);
    let calc = &ir.classes[0];
    assert_eq!(calc.name, "Calculator");
    // constructor skipped, so: add, fetchRate, create, validate
    assert_eq!(
        calc.methods.len(),
        4,
        "Expected 4 methods (constructor skipped)"
    );
}

#[test]
fn test_method_async() {
    let ir = extract_fixture("methods.ts");
    let calc = &ir.classes[0];
    let fetch_rate = calc
        .methods
        .iter()
        .find(|m| m.name == "fetchRate")
        .expect("fetchRate not found");
    assert!(fetch_rate.is_async);
}

#[test]
fn test_method_static() {
    let ir = extract_fixture("methods.ts");
    let calc = &ir.classes[0];
    let create = calc
        .methods
        .iter()
        .find(|m| m.name == "create")
        .expect("create not found");
    assert!(create.is_static);
}

#[test]
fn test_method_private() {
    let ir = extract_fixture("methods.ts");
    let calc = &ir.classes[0];
    let validate = calc
        .methods
        .iter()
        .find(|m| m.name == "validate")
        .expect("validate not found");
    assert_eq!(validate.visibility, Visibility::Private);
}

// =========================================================================
// services.scm tests
// =========================================================================

#[test]
fn test_services_fixture() {
    let ir = extract_fixture("services.ts");
    assert!(
        ir.services.len() >= 2,
        "Expected at least 2 services, got {}",
        ir.services.len()
    );
}

#[test]
fn test_service_controller() {
    let ir = extract_fixture("services.ts");
    let controller = ir
        .services
        .iter()
        .find(|s| s.name == "UserController")
        .expect("UserController not found");
    assert_eq!(controller.kind, ServiceKind::HttpController);
    assert!(!controller.methods.is_empty());
}

#[test]
fn test_service_injectable() {
    let ir = extract_fixture("services.ts");
    let auth = ir
        .services
        .iter()
        .find(|s| s.name == "AuthService")
        .expect("AuthService not found");
    assert_eq!(auth.kind, ServiceKind::Microservice);
}

#[test]
fn test_service_routes() {
    let ir = extract_fixture("services.ts");
    let controller = ir
        .services
        .iter()
        .find(|s| s.name == "UserController")
        .expect("UserController not found");
    assert!(
        !controller.routes.is_empty(),
        "Expected routes on controller"
    );

    let get_routes: Vec<&RouteDef> = controller
        .routes
        .iter()
        .filter(|r| r.method == HttpMethod::Get)
        .collect();
    assert!(!get_routes.is_empty(), "Expected GET routes");
}

#[test]
fn test_service_dependencies() {
    let ir = extract_fixture("services.ts");
    let auth = ir
        .services
        .iter()
        .find(|s| s.name == "AuthService")
        .expect("AuthService not found");
    assert!(!auth.dependencies.is_empty(), "Expected dependencies");
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_schemas_fixture_count() {
    let ir = extract_fixture("schemas.ts");
    assert!(
        ir.schemas.len() >= 3,
        "Expected at least 3 schemas, got {}",
        ir.schemas.len()
    );
}

#[test]
fn test_zod_schema() {
    let ir = extract_fixture("schemas.ts");
    let user_schema = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserSchema")
        .expect("UserSchema not found");
    assert_eq!(user_schema.source_framework, "zod");
    assert_eq!(user_schema.kind, SchemaKind::ValidationSchema);
    assert!(!user_schema.fields.is_empty());
}

#[test]
fn test_effect_schema() {
    let ir = extract_fixture("schemas.ts");
    let product_schema = ir
        .schemas
        .iter()
        .find(|s| s.name == "ProductSchema")
        .expect("ProductSchema not found");
    assert_eq!(product_schema.source_framework, "effect-schema");
    assert_eq!(product_schema.kind, SchemaKind::ValidationSchema);
}

#[test]
fn test_drizzle_schema() {
    let ir = extract_fixture("schemas.ts");
    let users_table = ir
        .schemas
        .iter()
        .find(|s| s.name == "users")
        .expect("users table not found");
    assert_eq!(users_table.source_framework, "drizzle");
    assert_eq!(users_table.kind, SchemaKind::OrmModel);
    assert!(users_table.table_name.is_some());
}

#[test]
fn test_schema_fields() {
    let ir = extract_fixture("schemas.ts");
    let user_schema = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserSchema")
        .expect("UserSchema not found");
    let field_names: Vec<&str> = user_schema.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"name"));
    assert!(field_names.contains(&"email"));
    assert!(field_names.contains(&"age"));

    let age = user_schema
        .fields
        .iter()
        .find(|f| f.name == "age")
        .expect("age field not found");
    assert!(age.is_optional);
}

// =========================================================================
// Full extraction test
// =========================================================================

#[test]
fn test_extract_inline_interface() {
    let ir = extract_ts("interface Foo { bar(): string; }");
    assert_eq!(ir.interfaces.len(), 1);
    assert_eq!(ir.interfaces[0].name, "Foo");
    assert_eq!(ir.interfaces[0].methods.len(), 1);
    assert_eq!(ir.interfaces[0].methods[0].name, "bar");
}

#[test]
fn test_extract_inline_class() {
    let ir = extract_ts("class MyService { getData(): string { return ''; } }");
    assert_eq!(ir.classes.len(), 1);
    assert_eq!(ir.classes[0].name, "MyService");
    assert_eq!(ir.classes[0].methods.len(), 1);
}

#[test]
fn test_extract_inline_function() {
    let ir = extract_ts("function add(a: number, b: number): number { return a + b; }");
    assert_eq!(ir.functions.len(), 1);
    assert_eq!(ir.functions[0].name, "add");
    assert_eq!(ir.functions[0].parameters.len(), 2);
}

#[test]
fn test_extract_inline_type_alias() {
    let ir = extract_ts("type Id = string;");
    assert_eq!(ir.type_aliases.len(), 1);
    assert_eq!(ir.type_aliases[0].name, "Id");
    assert_eq!(ir.type_aliases[0].target, "string");
}

#[test]
fn test_extract_inline_import() {
    let ir = extract_ts("import { Foo } from './bar';");
    assert_eq!(ir.imports.len(), 1);
    assert_eq!(ir.imports[0].source, "./bar");
    assert_eq!(ir.imports[0].symbols[0].name, "Foo");
}

#[test]
fn test_content_hash_populated() {
    let ir = extract_ts("const x = 1;");
    assert!(!ir.content_hash.is_empty());
}

#[test]
fn test_build_status_propagated() {
    let source = "interface A {}";
    let tree = parse_source(source.as_bytes(), Language::TypeScript).unwrap();
    let ir = extract(
        &tree,
        source.as_bytes(),
        Path::new("test.ts"),
        Language::TypeScript,
        BuildStatus::Unbuilt,
    )
    .unwrap();
    assert_eq!(ir.build_status, BuildStatus::Unbuilt);
    assert_eq!(ir.confidence, Confidence::Low);
}
