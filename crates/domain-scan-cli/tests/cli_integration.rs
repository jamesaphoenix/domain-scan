//! CLI integration tests using assert_cmd.
//!
//! These tests spawn the actual `domain-scan` binary and verify output.

use std::path::Path;

use assert_cmd::Command;

fn fixture_root() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../domain-scan-core/tests/fixtures/cross_file")
        .to_string_lossy()
        .to_string()
}

fn base_cmd() -> Command {
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    cmd.arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q");
    cmd
}

// ---------------------------------------------------------------------------
// scan
// ---------------------------------------------------------------------------

#[test]
fn test_scan_json_is_valid() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("scan")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "scan should succeed");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert!(json.get("files").is_some(), "should have files field");
    assert!(json.get("stats").is_some(), "should have stats field");
}

#[test]
fn test_scan_table_output() {
    let output = base_cmd()
        .arg("--output")
        .arg("table")
        .arg("scan")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Scan:"), "table output should contain 'Scan:'");
    assert!(stdout.contains("3 files"), "should report 3 files");
}

#[test]
fn test_scan_compact_output() {
    let output = base_cmd()
        .arg("--output")
        .arg("compact")
        .arg("scan")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("3 files"), "compact output should mention file count");
}

// ---------------------------------------------------------------------------
// interfaces
// ---------------------------------------------------------------------------

#[test]
fn test_interfaces_json_output() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert!(arr.len() >= 3, "should have at least 3 interfaces");

    let names: Vec<&str> = arr
        .iter()
        .filter_map(|v| v.get("name")?.as_str())
        .collect();
    assert!(names.contains(&"EventHandler"));
    assert!(names.contains(&"Repository"));
}

#[test]
fn test_interfaces_name_filter() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--name")
        .arg("Event")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert_eq!(arr.len(), 1, "only EventHandler should match");
    assert_eq!(arr[0]["name"], "EventHandler");
}

#[test]
fn test_interfaces_table_output() {
    let output = base_cmd()
        .arg("--output")
        .arg("table")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EventHandler"));
    assert!(stdout.contains("interface"));
}

#[test]
fn test_interfaces_compact_output() {
    let output = base_cmd()
        .arg("--output")
        .arg("compact")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("interface:EventHandler"));
}

// ---------------------------------------------------------------------------
// stats
// ---------------------------------------------------------------------------

#[test]
fn test_stats_json() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("stats")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["total_files"], 3);
}

#[test]
fn test_stats_table() {
    let output = base_cmd()
        .arg("stats")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Scan Statistics"));
    assert!(stdout.contains("Files scanned:   3"));
    assert!(stdout.contains("TypeScript"));
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

#[test]
fn test_search_json() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("search")
        .arg("Handler")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    // EventHandler (interface) + LogEventHandler + MetricsHandler (classes)
    assert!(arr.len() >= 3, "should find at least 3 Handler entities, got {}", arr.len());
}

#[test]
fn test_search_with_kind_filter() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("search")
        .arg("Handler")
        .arg("--kind")
        .arg("interface")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert_eq!(arr.len(), 1, "only EventHandler interface should match");
}

// ---------------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------------

#[test]
fn test_validate_exits_1_on_failure() {
    let output = base_cmd()
        .arg("validate")
        .output()
        .expect("command should run");

    // Cross-file fixtures have interfaces without implementors
    assert!(!output.status.success(), "validate should exit 1 with failures");
}

#[test]
fn test_validate_json_output() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("validate")
        .output()
        .expect("command should run");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert!(json.get("violations").is_some());
    assert!(json.get("rules_checked").is_some());
}

// ---------------------------------------------------------------------------
// impls
// ---------------------------------------------------------------------------

#[test]
fn test_impls_all_json() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("impls")
        .arg("--all")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    // Should be an array (may be empty if no impls in TS fixtures)
    assert!(json.is_array());
}

#[test]
fn test_impls_by_name() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("impls")
        .arg("EventHandler")
        .output()
        .expect("command should run");

    assert!(output.status.success());
}

// ---------------------------------------------------------------------------
// cache
// ---------------------------------------------------------------------------

#[test]
fn test_cache_stats() {
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    let output = cmd
        .arg("--root")
        .arg(fixture_root())
        .arg("--no-cache")
        .arg("cache")
        .arg("stats")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain JSON with entries count
    assert!(stdout.contains("entries"));
}

// ---------------------------------------------------------------------------
// --out flag (write to file)
// ---------------------------------------------------------------------------

#[test]
fn test_out_flag_writes_file() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let out_path = dir.path().join("output.json");

    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("-o")
        .arg(out_path.to_str().expect("valid path"))
        .arg("scan")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    assert!(out_path.exists(), "output file should be created");

    let content = std::fs::read_to_string(&out_path).expect("should read file");
    let _: serde_json::Value =
        serde_json::from_str(&content).expect("file should contain valid JSON");
}

// ---------------------------------------------------------------------------
// Snapshot tests for output format stability
// ---------------------------------------------------------------------------

#[test]
fn test_snapshot_stats_json() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("stats")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");

    // Snapshot the stats structure (mask timing fields that vary between runs)
    let mut stable_json = json.clone();
    if let Some(obj) = stable_json.as_object_mut() {
        obj.insert("parse_duration_ms".to_string(), serde_json::json!(0));
    }
    insta::assert_json_snapshot!("stats_json", stable_json);
}

#[test]
fn test_snapshot_interfaces_json() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--name")
        .arg("EventHandler")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");

    // Snapshot the interface structure (mask file paths that vary)
    let mut arr = json.as_array().expect("should be array").clone();
    for item in &mut arr {
        if let Some(obj) = item.as_object_mut() {
            // Normalize file path to just filename
            if let Some(file) = obj.get("file").and_then(|f| f.as_str()) {
                let filename = std::path::Path::new(file)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(file);
                obj.insert("file".to_string(), serde_json::json!(filename));
            }
        }
    }
    insta::assert_json_snapshot!("interfaces_event_handler_json", serde_json::json!(arr));
}

#[test]
fn test_snapshot_search_json() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("search")
        .arg("Repo")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");

    let mut arr = json.as_array().expect("should be array").clone();
    for item in &mut arr {
        if let Some(obj) = item.as_object_mut() {
            if let Some(file) = obj.get("file").and_then(|f| f.as_str()) {
                let filename = std::path::Path::new(file)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(file);
                obj.insert("file".to_string(), serde_json::json!(filename));
            }
        }
    }
    insta::assert_json_snapshot!("search_repo_json", serde_json::json!(arr));
}
