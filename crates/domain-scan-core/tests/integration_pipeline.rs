//! Full pipeline integration test: walk -> parse -> index -> query -> output.
//!
//! Uses the cross_file fixtures as a mini-project, exercises the entire pipeline
//! from directory walking through to JSON output.

use std::path::{Path, PathBuf};

use domain_scan_core::ir::{BuildStatus, Language, ScanConfig};
use domain_scan_core::walker;
use domain_scan_core::parser;
use domain_scan_core::query_engine;
use domain_scan_core::index;
use domain_scan_core::output::{self, OutputFormat};
use domain_scan_core::validate;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cross_file")
}

/// Run the full pipeline: walk -> parse -> extract -> index -> output
fn run_pipeline() -> domain_scan_core::ir::ScanIndex {
    let root = fixture_root();
    let config = ScanConfig {
        root: root.clone(),
        include: Vec::new(),
        exclude: Vec::new(),
        languages: vec![Language::TypeScript],
        build_status_override: None,
        cache_enabled: false,
        cache_dir: root.join(".domain-scan-cache"),
    };

    // Step 1: Walk
    let walked = walker::walk_directory(&config)
        .unwrap_or_else(|e| panic!("Walk failed: {e}"));
    assert!(!walked.is_empty(), "Walk should find files");

    // Step 2: Parse + Extract
    let mut ir_files = Vec::new();
    for walked_file in &walked {
        let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)
            .unwrap_or_else(|e| panic!("Parse failed for {}: {e}", walked_file.path.display()));

        let build_status = config
            .build_status_override
            .unwrap_or(BuildStatus::Built);

        let ir = query_engine::extract(&tree, &source, &walked_file.path, walked_file.language, build_status)
            .unwrap_or_else(|e| panic!("Extract failed for {}: {e}", walked_file.path.display()));

        ir_files.push(ir);
    }

    // Step 3: Build Index
    index::build_index(root, ir_files, 0, 0, 0)
}

#[test]
fn test_pipeline_walks_all_ts_files() {
    let scan_index = run_pipeline();
    assert_eq!(scan_index.stats.total_files, 3, "Should find 3 .ts files");
    assert_eq!(
        scan_index.stats.files_by_language.get(&Language::TypeScript),
        Some(&3),
        "All files should be TypeScript"
    );
}

#[test]
fn test_pipeline_extracts_entities() {
    let scan_index = run_pipeline();

    // types.ts: EventHandler, Repository, Serializable + repo.ts: User (private interface)
    assert!(
        scan_index.stats.total_interfaces >= 3,
        "Expected at least 3 interfaces, got {}",
        scan_index.stats.total_interfaces
    );

    // handler.ts: LogEventHandler, MetricsHandler + repo.ts: UserRepo
    assert!(
        scan_index.stats.total_classes >= 3,
        "Expected at least 3 classes, got {}",
        scan_index.stats.total_classes
    );
}

#[test]
fn test_pipeline_resolves_implementations() {
    let scan_index = run_pipeline();

    let event_impls = scan_index.get_implementors("EventHandler");
    assert!(
        event_impls.len() >= 2,
        "EventHandler should have at least 2 implementors, got {}",
        event_impls.len()
    );
}

#[test]
fn test_pipeline_query_interfaces() {
    let scan_index = run_pipeline();

    let all_interfaces = scan_index.get_interfaces(None);
    assert!(
        all_interfaces.len() >= 3,
        "Expected at least 3 interfaces, got {}",
        all_interfaces.len()
    );

    let handler_interfaces = scan_index.get_interfaces(Some("Handler"));
    assert_eq!(
        handler_interfaces.len(), 1,
        "Expected 1 interface matching 'Handler'"
    );
    assert_eq!(handler_interfaces[0].name, "EventHandler");
}

#[test]
fn test_pipeline_json_output() {
    let scan_index = run_pipeline();

    let json = output::format_scan_index(&scan_index, OutputFormat::Json)
        .unwrap_or_else(|e| panic!("JSON formatting failed: {e}"));

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("Output is not valid JSON: {e}"));

    // Check top-level structure
    assert!(parsed.get("root").is_some());
    assert!(parsed.get("version").is_some());
    assert!(parsed.get("files").is_some());
    assert!(parsed.get("stats").is_some());

    let files = parsed["files"].as_array();
    assert!(files.is_some());
    assert_eq!(files.map_or(0, |f| f.len()), 3);
}

#[test]
fn test_pipeline_table_output() {
    let scan_index = run_pipeline();

    let table = output::format_scan_index(&scan_index, OutputFormat::Table)
        .unwrap_or_else(|e| panic!("Table formatting failed: {e}"));

    assert!(table.contains("3 files"), "Table should mention 3 files");
    assert!(table.contains("Interfaces:"), "Table should have Interfaces line");
}

#[test]
fn test_pipeline_compact_output() {
    let scan_index = run_pipeline();

    let compact = output::format_scan_index(&scan_index, OutputFormat::Compact)
        .unwrap_or_else(|e| panic!("Compact formatting failed: {e}"));

    assert!(compact.contains("3 files"), "Compact should mention 3 files");
}

#[test]
fn test_pipeline_validate() {
    let scan_index = run_pipeline();

    let result = validate::validate(&scan_index);
    assert_eq!(result.rules_checked, 10, "Should check all 10 rules");

    // We expect some warnings/violations since some interfaces have no impls
    // in this small test fixture, but the pipeline should complete without panic
    assert!(result.rules_checked > 0);
}

#[test]
fn test_pipeline_search() {
    let scan_index = run_pipeline();

    let results = scan_index.search("Repo");
    assert!(
        !results.is_empty(),
        "Search for 'Repo' should find results"
    );
}
