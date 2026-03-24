//! Cross-file resolution integration tests.
//!
//! These tests parse real TypeScript fixtures through tree-sitter
//! and verify cross-file resolution (imports, implementations, completeness).

use std::path::{Path, PathBuf};

use domain_scan_core::ir::{BuildStatus, Language};
use domain_scan_core::parser;
use domain_scan_core::query_engine;
use domain_scan_core::resolver;
use domain_scan_core::index;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cross_file")
        .join(name)
}

fn parse_fixture(name: &str) -> domain_scan_core::ir::IrFile {
    let path = fixture_path(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {name}: {e}"));
    let tree = parser::parse_source(source.as_bytes(), Language::TypeScript)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {name}: {e}"));
    query_engine::extract(&tree, source.as_bytes(), &path, Language::TypeScript, BuildStatus::Built)
        .unwrap_or_else(|e| panic!("Failed to extract from fixture {name}: {e}"))
}

#[test]
fn test_cross_file_interface_in_a_impl_in_b() {
    let types_ir = parse_fixture("types.ts");
    let handler_ir = parse_fixture("handler.ts");

    // types.ts should have the interfaces
    assert!(
        types_ir.interfaces.len() >= 2,
        "Expected at least 2 interfaces in types.ts, got {}",
        types_ir.interfaces.len()
    );

    let event_handler = types_ir.interfaces.iter().find(|i| i.name == "EventHandler");
    assert!(event_handler.is_some(), "Expected EventHandler interface in types.ts");

    // handler.ts should have classes implementing EventHandler
    assert!(
        handler_ir.classes.len() >= 2,
        "Expected at least 2 classes in handler.ts, got {}",
        handler_ir.classes.len()
    );

    let log_handler = handler_ir.classes.iter().find(|c| c.name == "LogEventHandler");
    assert!(log_handler.is_some(), "Expected LogEventHandler class in handler.ts");
    assert!(
        log_handler
            .is_some_and(|c| c.implements.contains(&"EventHandler".to_string())),
        "LogEventHandler should implement EventHandler"
    );
}

#[test]
fn test_cross_file_resolver_finds_implementors() {
    let types_ir = parse_fixture("types.ts");
    let handler_ir = parse_fixture("handler.ts");
    let repo_ir = parse_fixture("repo.ts");

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cross_file");
    let files = vec![types_ir, handler_ir, repo_ir];
    let result = resolver::resolve(&files, &root);

    // EventHandler should have 2 implementors: LogEventHandler, MetricsHandler
    let event_impls = result.implementors.get("EventHandler");
    assert!(event_impls.is_some(), "Expected implementors for EventHandler");
    let empty_vec = Vec::new();
    let event_impls = event_impls.unwrap_or(&empty_vec);
    assert!(
        event_impls.contains(&"LogEventHandler".to_string()),
        "Expected LogEventHandler to implement EventHandler, got: {event_impls:?}"
    );
    assert!(
        event_impls.contains(&"MetricsHandler".to_string()),
        "Expected MetricsHandler to implement EventHandler, got: {event_impls:?}"
    );
}

#[test]
fn test_cross_file_import_resolution() {
    let types_ir = parse_fixture("types.ts");
    let handler_ir = parse_fixture("handler.ts");

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cross_file");
    let files = vec![types_ir, handler_ir];
    let result = resolver::resolve(&files, &root);

    // handler.ts imports from ./types
    let handler_imports: Vec<_> = result.imports.iter()
        .filter(|i| i.importing_file.ends_with("handler.ts"))
        .collect();
    assert!(
        !handler_imports.is_empty(),
        "Expected imports from handler.ts"
    );

    // The import should resolve to types.ts
    let types_import = handler_imports.iter().find(|i| i.source.contains("types"));
    assert!(types_import.is_some(), "Expected import from ./types in handler.ts");
    assert!(
        types_import.is_some_and(|i| i.resolved_path.is_some()),
        "Expected ./types import to resolve to a file"
    );
}

#[test]
fn test_cross_file_index_build() {
    let types_ir = parse_fixture("types.ts");
    let handler_ir = parse_fixture("handler.ts");
    let repo_ir = parse_fixture("repo.ts");

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cross_file");
    let scan_index = index::build_index(root, vec![types_ir, handler_ir, repo_ir], 0, 0, 0);

    // Check stats
    assert_eq!(scan_index.stats.total_files, 3);
    assert!(scan_index.stats.total_interfaces >= 3, "Expected at least 3 interfaces");
    assert!(scan_index.stats.total_classes >= 3, "Expected at least 3 classes");

    // Query: find implementors of EventHandler
    let implementors = scan_index.get_implementors("EventHandler");
    assert!(
        implementors.len() >= 2,
        "Expected at least 2 implementors of EventHandler, got {}",
        implementors.len()
    );
}

#[test]
fn test_cross_file_implementation_completeness() {
    let types_ir = parse_fixture("types.ts");
    let handler_ir = parse_fixture("handler.ts");

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cross_file");
    let files = vec![types_ir, handler_ir];
    let resolution = resolver::resolve(&files, &root);
    let completeness = resolver::check_all_completeness(&files, &resolution.impl_links);

    // LogEventHandler should be complete (all 3 methods)
    let log_key = ("LogEventHandler".to_string(), "EventHandler".to_string());
    assert!(
        !completeness.contains_key(&log_key),
        "LogEventHandler should be complete"
    );

    // MetricsHandler should be incomplete (missing cleanup)
    let metrics_key = ("MetricsHandler".to_string(), "EventHandler".to_string());
    assert!(
        completeness.contains_key(&metrics_key),
        "MetricsHandler should be incomplete"
    );
    let missing = &completeness[&metrics_key];
    assert!(
        missing.contains(&"cleanup".to_string()),
        "MetricsHandler should be missing 'cleanup', got: {missing:?}"
    );
}
