//! Integration tests for Scala query extraction.
//! Each test parses a real Scala fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/scala/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Scala)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Scala,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// traits.scm tests
// =========================================================================

#[test]
fn test_scala_traits_count() {
    let ir = extract_fixture("traits.scala");
    assert_eq!(ir.interfaces.len(), 4, "Expected 4 traits");
}

#[test]
fn test_scala_trait_names() {
    let ir = extract_fixture("traits.scala");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"UserRepository"), "Missing UserRepository");
    assert!(names.contains(&"Closeable"), "Missing Closeable");
    assert!(names.contains(&"EventHandler"), "Missing EventHandler");
    assert!(names.contains(&"InternalCache"), "Missing InternalCache");
}

#[test]
fn test_scala_trait_kind() {
    let ir = extract_fixture("traits.scala");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::Trait);
    }
}

#[test]
fn test_scala_trait_methods() {
    let ir = extract_fixture("traits.scala");
    let user_repo = ir
        .interfaces
        .iter()
        .find(|i| i.name == "UserRepository")
        .expect("UserRepository not found");
    assert_eq!(user_repo.methods.len(), 4);
    let method_names: Vec<&str> = user_repo.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"findById"));
    assert!(method_names.contains(&"findAll"));
    assert!(method_names.contains(&"save"));
    assert!(method_names.contains(&"delete"));
}

#[test]
fn test_scala_trait_generics() {
    let ir = extract_fixture("traits.scala");
    let handler = ir
        .interfaces
        .iter()
        .find(|i| i.name == "EventHandler")
        .expect("EventHandler not found");
    assert!(
        !handler.generics.is_empty(),
        "EventHandler should have generics"
    );
}

#[test]
fn test_scala_trait_extends() {
    let ir = extract_fixture("traits.scala");
    let handler = ir
        .interfaces
        .iter()
        .find(|i| i.name == "EventHandler")
        .expect("EventHandler not found");
    assert!(
        !handler.extends.is_empty(),
        "EventHandler should extend Closeable"
    );
}

// =========================================================================
// classes.scm tests (includes case classes and objects)
// =========================================================================

#[test]
fn test_scala_classes_count() {
    let ir = extract_fixture("classes.scala");
    // UserService, BaseEntity, User (case), CreateUserRequest (case), UserService (object) = 5
    assert!(
        ir.classes.len() >= 4,
        "Expected at least 4 classes, got {}",
        ir.classes.len()
    );
}

#[test]
fn test_scala_class_names() {
    let ir = extract_fixture("classes.scala");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"BaseEntity"), "Missing BaseEntity");
    assert!(names.contains(&"User"), "Missing User case class");
    assert!(
        names.contains(&"CreateUserRequest"),
        "Missing CreateUserRequest case class"
    );
}

#[test]
fn test_scala_class_abstract() {
    let ir = extract_fixture("classes.scala");
    let base = ir
        .classes
        .iter()
        .find(|c| c.name == "BaseEntity")
        .expect("BaseEntity not found");
    assert!(base.is_abstract, "BaseEntity should be abstract");
}

#[test]
fn test_scala_class_methods() {
    let ir = extract_fixture("classes.scala");
    let svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService" && !c.is_abstract)
        .expect("UserService class not found");
    assert!(
        svc.methods.len() >= 2,
        "UserService should have at least 2 methods, got {}",
        svc.methods.len()
    );
}

#[test]
fn test_scala_class_method_visibility() {
    let ir = extract_fixture("classes.scala");
    let svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService" && !c.is_abstract)
        .expect("UserService class not found");
    let validate = svc.methods.iter().find(|m| m.name == "validateEmail");
    if let Some(v) = validate {
        assert_eq!(
            v.visibility,
            Visibility::Private,
            "validateEmail should be private"
        );
    }
}

// =========================================================================
// objects.scm tests (objects are captured as classes with is_abstract=true)
// =========================================================================

#[test]
fn test_scala_objects_count() {
    let ir = extract_fixture("objects.scala");
    assert!(
        ir.classes.len() >= 3,
        "Expected at least 3 objects, got {}",
        ir.classes.len()
    );
}

#[test]
fn test_scala_object_names() {
    let ir = extract_fixture("objects.scala");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"AppConfig"), "Missing AppConfig");
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"Main"), "Missing Main");
}

#[test]
fn test_scala_objects_are_abstract() {
    let ir = extract_fixture("objects.scala");
    // Objects are marked as abstract in the current implementation
    for obj in &ir.classes {
        assert!(
            obj.is_abstract,
            "Object {} should be marked abstract",
            obj.name
        );
    }
}

#[test]
fn test_scala_object_methods() {
    let ir = extract_fixture("objects.scala");
    let app_config = ir
        .classes
        .iter()
        .find(|c| c.name == "AppConfig")
        .expect("AppConfig not found");
    assert!(
        !app_config.methods.is_empty(),
        "AppConfig should have at least 1 method"
    );
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_scala_imports_count() {
    let ir = extract_fixture("imports.scala");
    assert!(
        ir.imports.len() >= 3,
        "Expected at least 3 imports, got {}",
        ir.imports.len()
    );
}

#[test]
fn test_scala_import_sources() {
    let ir = extract_fixture("imports.scala");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(
        sources.iter().any(|s| s.contains("java.util")),
        "Should have java.util imports"
    );
}

#[test]
fn test_scala_wildcard_import() {
    let ir = extract_fixture("imports.scala");
    let wildcard = ir.imports.iter().find(|i| i.is_wildcard);
    assert!(
        wildcard.is_some(),
        "Should have at least one wildcard import (com.example.models._)"
    );
}

#[test]
fn test_scala_brace_import() {
    let ir = extract_fixture("imports.scala");
    // import java.util.{List, Map} should extract List and Map symbols
    let brace_import = ir.imports.iter().find(|i| i.symbols.len() >= 2);
    assert!(
        brace_import.is_some(),
        "Should have a multi-symbol brace import"
    );
    if let Some(bi) = brace_import {
        let sym_names: Vec<&str> = bi.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(
            sym_names.contains(&"List"),
            "Brace import should contain List"
        );
        assert!(
            sym_names.contains(&"Map"),
            "Brace import should contain Map"
        );
    }
}

// =========================================================================
// Cross-cutting: build_status and confidence
// =========================================================================

#[test]
fn test_scala_build_status() {
    let ir = extract_fixture("traits.scala");
    assert_eq!(ir.build_status, BuildStatus::Built);
    assert_eq!(ir.confidence, Confidence::High);
}

#[test]
fn test_scala_language() {
    let ir = extract_fixture("traits.scala");
    assert_eq!(ir.language, Language::Scala);
}
