//! Integration tests for Ruby query extraction.
//! Each test parses a real Ruby fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/ruby/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Ruby)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Ruby,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// modules.scm tests
// =========================================================================

#[test]
fn test_ruby_modules_count() {
    let ir = extract_fixture("modules.rb");
    assert_eq!(ir.interfaces.len(), 2, "Expected 2 modules, got {}", ir.interfaces.len());
}

#[test]
fn test_ruby_module_names() {
    let ir = extract_fixture("modules.rb");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"Authenticatable"), "Missing Authenticatable");
    assert!(names.contains(&"Serializable"), "Missing Serializable");
}

#[test]
fn test_ruby_module_kind() {
    let ir = extract_fixture("modules.rb");
    for module in &ir.interfaces {
        assert_eq!(module.language_kind, InterfaceKind::Module,
            "Ruby modules should have InterfaceKind::Module");
    }
}

#[test]
fn test_ruby_module_methods() {
    let ir = extract_fixture("modules.rb");
    let auth = ir.interfaces.iter().find(|i| i.name == "Authenticatable").unwrap();
    assert_eq!(auth.methods.len(), 2, "Authenticatable should have 2 methods");
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_ruby_classes_count() {
    let ir = extract_fixture("classes.rb");
    // UserService, OrderController, SimpleModel
    assert_eq!(ir.classes.len(), 3, "Expected 3 classes, got {}", ir.classes.len());
}

#[test]
fn test_ruby_class_names() {
    let ir = extract_fixture("classes.rb");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"OrderController"), "Missing OrderController");
    assert!(names.contains(&"SimpleModel"), "Missing SimpleModel");
}

#[test]
fn test_ruby_class_extends() {
    let ir = extract_fixture("classes.rb");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    assert_eq!(user_service.extends.as_deref(), Some("BaseService"), "UserService should extend BaseService");
}

#[test]
fn test_ruby_class_methods() {
    let ir = extract_fixture("classes.rb");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    // initialize, find_by_id, save, create (self.create)
    assert!(user_service.methods.len() >= 3,
        "UserService should have at least 3 methods, got {}", user_service.methods.len());
}

#[test]
fn test_ruby_class_singleton_method() {
    let ir = extract_fixture("classes.rb");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    let create = user_service.methods.iter().find(|m| m.name == "create");
    assert!(create.is_some(), "Missing singleton method 'create'");
    assert!(create.unwrap().is_static, "'self.create' should be detected as static");
}

#[test]
fn test_ruby_class_no_extends() {
    let ir = extract_fixture("classes.rb");
    let simple = ir.classes.iter().find(|c| c.name == "SimpleModel").unwrap();
    assert!(simple.extends.is_none(), "SimpleModel should not extend anything");
}

#[test]
fn test_ruby_method_parameters() {
    let ir = extract_fixture("classes.rb");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    let init = user_service.methods.iter().find(|m| m.name == "initialize").unwrap();
    assert_eq!(init.parameters.len(), 2, "initialize should have 2 parameters");
}

#[test]
fn test_ruby_method_ownership() {
    let ir = extract_fixture("classes.rb");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    for method in &user_service.methods {
        assert_eq!(method.owner.as_deref(), Some("UserService"), "Methods should have owner set");
    }
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_ruby_imports_count() {
    let ir = extract_fixture("imports.rb");
    // require 'json', require 'net/http', require_relative 'lib/user_service',
    // require_relative 'lib/order_service', include Comparable, extend ActiveModel::Naming
    assert!(ir.imports.len() >= 4, "Expected at least 4 imports, got {}", ir.imports.len());
}

#[test]
fn test_ruby_import_sources() {
    let ir = extract_fixture("imports.rb");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"json"), "Missing json require");
    assert!(sources.contains(&"net/http"), "Missing net/http require");
}

#[test]
fn test_ruby_require_relative() {
    let ir = extract_fixture("imports.rb");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"lib/user_service"), "Missing require_relative 'lib/user_service'");
}

#[test]
fn test_ruby_include_is_wildcard() {
    let ir = extract_fixture("imports.rb");
    let include_import = ir.imports.iter().find(|i| i.source == "Comparable");
    assert!(include_import.is_some(), "Missing include Comparable");
    assert!(include_import.unwrap().is_wildcard, "include should be marked as wildcard");
}
