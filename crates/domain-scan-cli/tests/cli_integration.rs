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
// --fields flag (field mask)
// ---------------------------------------------------------------------------

#[test]
fn test_fields_interfaces_name_only() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("--fields")
        .arg("name")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert!(!arr.is_empty(), "should have at least one interface");

    // Each element should only have "name" field
    for item in arr {
        let obj = item.as_object().expect("should be object");
        assert_eq!(obj.len(), 1, "should have exactly 1 field, got: {:?}", obj.keys().collect::<Vec<_>>());
        assert!(obj.contains_key("name"), "should have 'name' field");
    }
}

#[test]
fn test_fields_interfaces_name_and_methods() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("--fields")
        .arg("name,methods")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");

    for item in arr {
        let obj = item.as_object().expect("should be object");
        assert_eq!(obj.len(), 2, "should have exactly 2 fields");
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("methods"));
    }
}

#[test]
fn test_fields_scan_dot_notation() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("--fields")
        .arg("files.path,stats")
        .arg("scan")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let obj = json.as_object().expect("should be object");

    // Should only have "files" and "stats"
    assert_eq!(obj.len(), 2, "should have exactly 2 fields, got: {:?}", obj.keys().collect::<Vec<_>>());
    assert!(obj.contains_key("files"));
    assert!(obj.contains_key("stats"));

    // Each file entry should only have "path"
    let files = obj.get("files").and_then(|v| v.as_array()).expect("files should be array");
    for file in files {
        let file_obj = file.as_object().expect("file should be object");
        assert_eq!(file_obj.len(), 1, "file should have exactly 1 field");
        assert!(file_obj.contains_key("path"));
    }
}

#[test]
fn test_fields_ignored_for_table_output() {
    // --fields should be silently ignored when output is table
    let output = base_cmd()
        .arg("--output")
        .arg("table")
        .arg("--fields")
        .arg("name")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Table output should still contain full content
    assert!(stdout.contains("EventHandler"));
}

#[test]
fn test_fields_invalid_field_exits_1() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("--fields")
        .arg("nonexistent_field")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "should exit with error for invalid field");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("INVALID_FIELDS"), "should have INVALID_FIELDS error code");
    assert!(stderr.contains("nonexistent_field"), "should mention the invalid field");
}

#[test]
fn test_fields_stats_single_field() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("--fields")
        .arg("total_files")
        .arg("stats")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let obj = json.as_object().expect("should be object");
    assert_eq!(obj.len(), 1, "should have exactly 1 field");
    assert!(obj.contains_key("total_files"));
    assert_eq!(obj["total_files"], 3);
}

// ---------------------------------------------------------------------------
// --json flag (raw JSON payload input)
// ---------------------------------------------------------------------------

#[test]
fn test_json_interfaces_name_filter() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--json")
        .arg(r#"{"name": "Event"}"#)
        .output()
        .expect("command should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert_eq!(arr.len(), 1, "only EventHandler should match");
    assert_eq!(arr[0]["name"], "EventHandler");
}

#[test]
fn test_json_interfaces_show_methods() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--json")
        .arg(r#"{"name": "Event", "show_methods": true}"#)
        .output()
        .expect("command should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert_eq!(arr.len(), 1);
}

#[test]
fn test_json_interfaces_empty_object() {
    // An empty JSON object should return all interfaces (no filter)
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--json")
        .arg("{}")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert!(arr.len() >= 3, "should have at least 3 interfaces");
}

#[test]
fn test_json_search() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("search")
        .arg("--json")
        .arg(r#"{"query": "Handler"}"#)
        .output()
        .expect("command should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert!(arr.len() >= 3, "should find at least 3 Handler entities");
}

#[test]
fn test_json_search_with_kind() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("search")
        .arg("--json")
        .arg(r#"{"query": "Handler", "kind": "interface"}"#)
        .output()
        .expect("command should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert_eq!(arr.len(), 1, "only EventHandler interface should match");
}

#[test]
fn test_json_conflict_with_flags() {
    // --json and --name should conflict
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--name")
        .arg("Event")
        .arg("--json")
        .arg(r#"{"name": "Repo"}"#)
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "should fail with conflict");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "should mention mutually exclusive, got: {stderr}"
    );
}

#[test]
fn test_json_invalid_syntax() {
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--json")
        .arg("{not valid json}")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "should fail with invalid JSON");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid JSON syntax") || stderr.contains("JSON_PARSE_ERROR") || stderr.contains("CLI_ERROR"),
        "should report JSON parse error, got: {stderr}"
    );
}

#[test]
fn test_json_unknown_field_rejected() {
    // deny_unknown_fields should reject unknown fields
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--json")
        .arg(r#"{"name": "Event", "nonexistent_field": true}"#)
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "should fail with unknown field");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("nonexistent_field") || stderr.contains("schema"),
        "should mention the unknown field or schema mismatch, got: {stderr}"
    );
}

#[test]
fn test_json_wrong_type() {
    // show_methods should be bool, not string
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--json")
        .arg(r#"{"show_methods": "yes"}"#)
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "should fail with type mismatch");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("schema") || stderr.contains("interfaces"),
        "should mention schema mismatch, got: {stderr}"
    );
}

#[test]
fn test_json_depth_limit() {
    // Create deeply nested JSON (depth > 32)
    let mut json = String::from(r#"{"name": "#);
    for _ in 0..35 {
        json.push_str(r#"{"nested": "#);
    }
    json.push_str(r#""deep""#);
    for _ in 0..35 {
        json.push('}');
    }
    json.push('}');

    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("interfaces")
        .arg("--json")
        .arg(&json)
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "should fail with depth limit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("depth") || stderr.contains("nesting"),
        "should mention depth limit, got: {stderr}"
    );
}

#[test]
fn test_json_works_with_table_output() {
    // --json should work with any output format, not just JSON output
    let output = base_cmd()
        .arg("--output")
        .arg("table")
        .arg("interfaces")
        .arg("--json")
        .arg(r#"{"name": "Event"}"#)
        .output()
        .expect("command should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EventHandler"), "table output should contain EventHandler");
}

#[test]
fn test_json_works_with_fields() {
    // --json and --fields should be compatible (--json is input, --fields is output)
    let output = base_cmd()
        .arg("--output")
        .arg("json")
        .arg("--fields")
        .arg("name")
        .arg("interfaces")
        .arg("--json")
        .arg(r#"{"name": "Event"}"#)
        .output()
        .expect("command should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert_eq!(arr.len(), 1);
    let obj = arr[0].as_object().expect("should be object");
    assert_eq!(obj.len(), 1, "should have exactly 1 field after masking");
    assert!(obj.contains_key("name"));
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
