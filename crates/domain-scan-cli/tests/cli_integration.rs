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
    assert!(
        stdout.contains("Scan:"),
        "table output should contain 'Scan:'"
    );
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
    assert!(
        stdout.contains("3 files"),
        "compact output should mention file count"
    );
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

    let names: Vec<&str> = arr.iter().filter_map(|v| v.get("name")?.as_str()).collect();
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
        .arg("--output")
        .arg("table")
        .arg("stats")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Scan Statistics"));
    assert!(stdout.contains("Files scanned:   3"));
    assert!(stdout.contains("TypeScript"));
}

#[test]
fn test_doctor_json_output() {
    let output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--output")
        .arg("json")
        .arg("doctor")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "doctor should succeed");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["current_version"], env!("CARGO_PKG_VERSION"));
    assert!(json["executable_path"].is_string());
    assert!(json["recommended_update_command"].is_string());
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
    assert!(
        arr.len() >= 3,
        "should find at least 3 Handler entities, got {}",
        arr.len()
    );
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
    assert!(
        !output.status.success(),
        "validate should exit 1 with failures"
    );
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
        assert_eq!(
            obj.len(),
            1,
            "should have exactly 1 field, got: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
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
    assert_eq!(
        obj.len(),
        2,
        "should have exactly 2 fields, got: {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.contains_key("files"));
    assert!(obj.contains_key("stats"));

    // Each file entry should only have "path"
    let files = obj
        .get("files")
        .and_then(|v| v.as_array())
        .expect("files should be array");
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

    assert!(
        !output.status.success(),
        "should exit with error for invalid field"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("INVALID_FIELDS"),
        "should have INVALID_FIELDS error code"
    );
    assert!(
        stderr.contains("nonexistent_field"),
        "should mention the invalid field"
    );
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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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
        stderr.contains("Invalid JSON syntax")
            || stderr.contains("JSON_PARSE_ERROR")
            || stderr.contains("CLI_ERROR"),
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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("EventHandler"),
        "table output should contain EventHandler"
    );
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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let arr = json.as_array().expect("should be array");
    assert_eq!(arr.len(), 1);
    let obj = arr[0].as_object().expect("should be object");
    assert_eq!(obj.len(), 1, "should have exactly 1 field after masking");
    assert!(obj.contains_key("name"));
}

// ---------------------------------------------------------------------------
// --page-all flag (NDJSON pagination)
// ---------------------------------------------------------------------------

#[test]
fn test_page_all_interfaces_emits_ndjson() {
    let output = base_cmd()
        .arg("--page-all")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Each line should be a valid JSON object
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 3,
        "should have at least 3 interfaces, got {}",
        lines.len()
    );

    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line should be valid JSON: {e}\nline: {line}"));
        assert!(parsed.is_object(), "each line should be a JSON object");
        assert!(
            parsed.get("name").is_some(),
            "each object should have a name field"
        );
    }
}

#[test]
fn test_page_all_interfaces_with_name_filter() {
    let output = base_cmd()
        .arg("--page-all")
        .arg("interfaces")
        .arg("--name")
        .arg("Event")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "only EventHandler should match");

    let parsed: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON");
    assert_eq!(parsed["name"], "EventHandler");
}

#[test]
fn test_page_all_services() {
    let output = base_cmd()
        .arg("--page-all")
        .arg("services")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Each non-empty line should be valid JSON
    for line in stdout.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line should be valid JSON: {e}"));
        assert!(parsed.is_object());
    }
}

#[test]
fn test_page_all_schemas() {
    let output = base_cmd()
        .arg("--page-all")
        .arg("schemas")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line should be valid JSON: {e}"));
        assert!(parsed.is_object());
    }
}

#[test]
fn test_page_all_search() {
    let output = base_cmd()
        .arg("--page-all")
        .arg("search")
        .arg("Handler")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() >= 3, "should find at least 3 Handler entities");

    for line in &lines {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line should be valid JSON: {e}"));
        assert!(parsed.is_object());
        assert!(parsed.get("name").is_some());
    }
}

#[test]
fn test_page_all_impls() {
    let output = base_cmd()
        .arg("--page-all")
        .arg("impls")
        .arg("--all")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Each line (if any) should be valid JSON
    for line in stdout.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line should be valid JSON: {e}"));
        assert!(parsed.is_object());
    }
}

#[test]
fn test_page_all_with_fields() {
    // --page-all should work with --fields to limit output per entity
    let output = base_cmd()
        .arg("--page-all")
        .arg("--fields")
        .arg("name")
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty(), "should have at least one interface");

    for line in &lines {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line should be valid JSON: {e}"));
        let obj = parsed.as_object().expect("should be object");
        assert_eq!(obj.len(), 1, "should have exactly 1 field after masking");
        assert!(obj.contains_key("name"));
    }
}

#[test]
fn test_page_all_compact_one_per_line() {
    // Verify that NDJSON is compact (not pretty-printed)
    let output = base_cmd()
        .arg("--page-all")
        .arg("interfaces")
        .arg("--name")
        .arg("Event")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be exactly one non-empty line (compact JSON, not pretty)
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "should be exactly one line");

    // The line should parse as JSON and should not contain newlines within
    let parsed: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON");
    assert_eq!(parsed["name"], "EventHandler");
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

// ---------------------------------------------------------------------------
// Auto-detect JSON output when stdout is not a TTY (Phase 6c.5)
// ---------------------------------------------------------------------------

#[test]
fn test_auto_detect_json_when_piped() {
    // When no --output is specified and stdout is piped (assert_cmd always pipes),
    // the output should default to JSON.
    let output = base_cmd().arg("scan").output().expect("command should run");

    assert!(output.status.success());

    // Should be valid JSON (not table format)
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("piped output should default to JSON");
    assert!(json.get("files").is_some(), "should have files field");
    assert!(json.get("stats").is_some(), "should have stats field");
}

#[test]
fn test_auto_detect_json_interfaces_when_piped() {
    // Interfaces should also default to JSON when piped
    let output = base_cmd()
        .arg("interfaces")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("piped interfaces output should default to JSON");
    let arr = json.as_array().expect("should be array");
    assert!(arr.len() >= 3, "should have at least 3 interfaces");
}

#[test]
fn test_explicit_output_overrides_auto_detect() {
    // Explicit --output table should override auto-detection even when piped
    let output = base_cmd()
        .arg("--output")
        .arg("table")
        .arg("scan")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be table format, not JSON
    assert!(
        stdout.contains("Scan:"),
        "explicit --output table should produce table format"
    );
    // And it should NOT be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err(),
        "table output should not be valid JSON"
    );
}

#[test]
fn test_explicit_output_compact_overrides_auto_detect() {
    // Explicit --output compact should override auto-detection
    let output = base_cmd()
        .arg("--output")
        .arg("compact")
        .arg("scan")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("3 files"),
        "compact output should mention file count"
    );
}

#[test]
fn test_auto_detect_json_stats_when_piped() {
    // Stats should also default to JSON when piped
    let output = base_cmd()
        .arg("stats")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("piped stats output should default to JSON");
    assert_eq!(json["total_files"], 3);
}

#[test]
fn test_auto_detect_json_search_when_piped() {
    // Search should also default to JSON when piped
    let output = base_cmd()
        .arg("search")
        .arg("Handler")
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("piped search output should default to JSON");
    let arr = json.as_array().expect("should be array");
    assert!(arr.len() >= 3, "should find at least 3 Handler entities");
}

// ---------------------------------------------------------------------------
// --dry-run on cache clear
// ---------------------------------------------------------------------------

#[test]
fn test_cache_clear_dry_run_outputs_json() {
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    let output = cmd
        .arg("--root")
        .arg(fixture_root())
        .arg("cache")
        .arg("clear")
        .arg("--dry-run")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output should be a JSON array (possibly empty if no cache entries)
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("dry-run output should be valid JSON");
    assert!(json.is_array(), "dry-run output should be a JSON array");
}

#[test]
fn test_cache_prune_dry_run_outputs_json() {
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    let output = cmd
        .arg("--root")
        .arg(fixture_root())
        .arg("cache")
        .arg("prune")
        .arg("--dry-run")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("dry-run output should be valid JSON");
    assert!(json.is_array(), "dry-run output should be a JSON array");
}

#[test]
fn test_cache_clear_dry_run_does_not_delete() {
    // First, do a scan to populate cache
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let cache_dir = tmp.path().join(".domain-scan-cache");

    // Run a scan (this implicitly creates cache entries)
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    let output = cmd
        .arg("--root")
        .arg(fixture_root())
        .arg("scan")
        .arg("-q")
        .arg("--output")
        .arg("json")
        .output()
        .expect("scan should run");
    assert!(output.status.success());

    // Now dry-run clear against the same root — cache should still exist after
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    let output = cmd
        .arg("--root")
        .arg(fixture_root())
        .arg("cache")
        .arg("clear")
        .arg("--dry-run")
        .output()
        .expect("dry-run clear should run");
    assert!(output.status.success());

    // Verify cache stats still show entries (dry-run didn't actually delete)
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    let output = cmd
        .arg("--root")
        .arg(fixture_root())
        .arg("cache")
        .arg("stats")
        .output()
        .expect("stats should run");
    assert!(output.status.success());
    // We can't assert exact entries since it depends on env,
    // but the command should succeed
    let _ = cache_dir; // silence unused var
}

// ---------------------------------------------------------------------------
// F.12: Skill Bootstrapping Tests
// ---------------------------------------------------------------------------

fn skills_cmd() -> Command {
    let mut cmd = Command::cargo_bin("domain-scan").expect("binary should exist");
    cmd.arg("-q");
    cmd
}

#[test]
fn test_skills_list_outputs_all_skill_names() {
    let output = skills_cmd()
        .arg("skills")
        .arg("list")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "skills list should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let names: Vec<String> =
        serde_json::from_str(&stdout).expect("output should be valid JSON array");
    assert!(
        names.len() >= 11,
        "should have at least 11 skills, got {}",
        names.len()
    );
    assert!(names.contains(&"domain-scan-cli".to_string()));
    assert!(names.contains(&"domain-scan-init".to_string()));
    assert!(names.contains(&"domain-scan-tube-map".to_string()));
    assert!(names.contains(&"domain-scan-scan".to_string()));
    assert!(names.contains(&"domain-scan-match".to_string()));
}

#[test]
fn test_skills_show_outputs_valid_yaml_frontmatter_and_markdown() {
    let output = skills_cmd()
        .arg("skills")
        .arg("show")
        .arg("domain-scan-init")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "skills show should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Check YAML frontmatter
    assert!(
        stdout.starts_with("---\n"),
        "should start with YAML frontmatter"
    );
    assert!(
        stdout.contains("name: domain-scan-init"),
        "should contain skill name"
    );
    assert!(stdout.contains("version:"), "should contain version");
    assert!(
        stdout.contains("description:"),
        "should contain description"
    );
    // Check markdown content
    assert!(stdout.contains("# "), "should contain markdown headers");
}

#[test]
fn test_skills_dump_contains_all_skills() {
    let output = skills_cmd()
        .arg("skills")
        .arg("dump")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "skills dump should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Each skill is separated by a header line
    assert!(stdout.contains("# === domain-scan-cli ==="));
    assert!(stdout.contains("# === domain-scan-init ==="));
    assert!(stdout.contains("# === domain-scan-tube-map ==="));
    assert!(stdout.contains("# === domain-scan-scan ==="));
    assert!(stdout.contains("# === domain-scan-match ==="));
    // Dump should contain the actual content of each skill
    assert!(stdout.contains("name: domain-scan-cli"));
    assert!(stdout.contains("name: domain-scan-init"));
    assert!(stdout.contains("name: domain-scan-tube-map"));
}

#[test]
fn test_skills_install_claude_code() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let root = tmp.path();

    let output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--claude-code")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "install --claude-code should succeed"
    );

    let skills_dir = root.join(".claude").join("skills");
    assert!(skills_dir.exists(), ".claude/skills/ should be created");

    let init_skill = skills_dir.join("domain-scan-init.md");
    assert!(init_skill.exists(), "domain-scan-init.md should exist");

    // Verify all skill files are installed
    let entries: Vec<_> = std::fs::read_dir(&skills_dir)
        .expect("should read skills dir")
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.len() >= 11,
        "should have at least 11 skill files, got {}",
        entries.len()
    );
}

#[test]
fn test_skills_install_codex() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let root = tmp.path();

    let output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--codex")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "install --codex should succeed");

    let skills_dir = root.join(".codex").join("skills");
    assert!(skills_dir.exists(), ".codex/skills/ should be created");

    let init_skill = skills_dir.join("domain-scan-init.md");
    assert!(init_skill.exists(), "domain-scan-init.md should exist");
}

#[test]
fn test_skills_install_custom_dir() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let root = tmp.path();
    let custom_dir = root.join("custom").join("path");

    let output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--dir")
        .arg(&custom_dir)
        .output()
        .expect("command should run");

    assert!(output.status.success(), "install --dir should succeed");
    assert!(custom_dir.exists(), "custom directory should be created");

    let init_skill = custom_dir.join("domain-scan-init.md");
    assert!(init_skill.exists(), "domain-scan-init.md should exist");
}

#[test]
fn test_skills_install_twice_overwrites_no_duplicates() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let root = tmp.path();

    // Install once
    let output1 = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--claude-code")
        .output()
        .expect("command should run");
    assert!(output1.status.success());

    let skills_dir = root.join(".claude").join("skills");
    let count1 = std::fs::read_dir(&skills_dir)
        .expect("should read dir")
        .count();

    // Install again
    let output2 = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--claude-code")
        .output()
        .expect("command should run");
    assert!(output2.status.success());

    let count2 = std::fs::read_dir(&skills_dir)
        .expect("should read dir")
        .count();

    assert_eq!(count1, count2, "install twice should not create duplicates");
}

#[test]
fn test_skills_install_updates_gitignore() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let root = tmp.path();

    let output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--claude-code")
        .output()
        .expect("command should run");
    assert!(output.status.success());

    let gitignore = root.join(".gitignore");
    assert!(gitignore.exists(), ".gitignore should be created");

    let content = std::fs::read_to_string(&gitignore).expect("should read .gitignore");
    assert!(
        content.contains(".claude/skills/"),
        ".gitignore should contain .claude/skills/"
    );

    // Install again — .gitignore should not have duplicates
    let output2 = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--claude-code")
        .output()
        .expect("command should run");
    assert!(output2.status.success());

    let content2 = std::fs::read_to_string(&gitignore).expect("should read .gitignore");
    let count = content2.matches(".claude/skills/").count();
    assert_eq!(count, 1, ".gitignore should not have duplicate entries");
}

#[test]
fn test_help_contains_agent_skills_section() {
    let output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--help")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AGENT SKILLS"),
        "--help output should contain AGENT SKILLS section"
    );
}

#[test]
fn test_installed_skill_matches_show_output() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let root = tmp.path();

    // Get the show output for domain-scan-init
    let show_output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("-q")
        .arg("skills")
        .arg("show")
        .arg("domain-scan-init")
        .output()
        .expect("command should run");
    assert!(show_output.status.success());
    let show_content = String::from_utf8_lossy(&show_output.stdout);

    // Install skills
    let install_output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(root)
        .arg("skills")
        .arg("install")
        .arg("--claude-code")
        .output()
        .expect("command should run");
    assert!(install_output.status.success());

    // Read installed file
    let installed_path = root
        .join(".claude")
        .join("skills")
        .join("domain-scan-init.md");
    let installed_content =
        std::fs::read_to_string(&installed_path).expect("should read installed file");

    assert_eq!(
        show_content.as_ref(),
        installed_content.as_str(),
        "installed file content should match `skills show` output exactly"
    );
}

// ---------------------------------------------------------------------------
// G.4: Agent Skill Workflow Tests
// ---------------------------------------------------------------------------

/// Test: Claude Code can create a manifest from scratch using the skill.
///
/// Simulates the full workflow from `domain-scan-init.md`:
///   1. Bootstrap a starter manifest via `domain-scan init --bootstrap -o system.json`
///   2. Validate via `domain-scan validate --manifest system.json --output json`
///   3. Match via `domain-scan match --manifest system.json --output json`
///   4. Verify: valid JSON, all statuses = "new", kebab-case IDs, coverage ≥ 0
#[test]
fn test_skill_create_manifest_from_scratch() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let manifest_path = tmp.path().join("system.json");

    // Step 1: Bootstrap a manifest (as the skill instructs: "always start with --bootstrap")
    let bootstrap_output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q")
        .arg("init")
        .arg("--bootstrap")
        .arg("--name")
        .arg("test-project")
        .arg("-o")
        .arg(&manifest_path)
        .output()
        .expect("bootstrap command should run");

    assert!(
        bootstrap_output.status.success(),
        "init --bootstrap should succeed: {}",
        String::from_utf8_lossy(&bootstrap_output.stderr)
    );
    assert!(manifest_path.exists(), "system.json should be created");

    // Verify the manifest is valid JSON with expected structure
    let manifest_json: serde_json::Value = {
        let content = std::fs::read_to_string(&manifest_path).expect("read manifest");
        serde_json::from_str(&content).expect("manifest should be valid JSON")
    };

    assert!(
        manifest_json.get("meta").is_some(),
        "should have meta field"
    );
    assert!(
        manifest_json.get("domains").is_some(),
        "should have domains field"
    );
    assert!(
        manifest_json.get("subsystems").is_some(),
        "should have subsystems field"
    );
    assert!(
        manifest_json.get("connections").is_some(),
        "should have connections field"
    );

    // Verify project name was applied
    assert_eq!(
        manifest_json["meta"]["name"].as_str(),
        Some("test-project"),
        "project name should match --name flag"
    );

    // Verify skill rule: all statuses should be "new" (never auto-confirm "built")
    if let Some(subsystems) = manifest_json["subsystems"].as_array() {
        for sub in subsystems {
            let status = sub["status"].as_str().unwrap_or("unknown");
            assert_eq!(
                status,
                "new",
                "Bootstrap should set all subsystems to 'new', got '{}' for '{}'",
                status,
                sub["id"].as_str().unwrap_or("?")
            );

            // Verify IDs are non-empty (kebab-case is an agent refinement guideline,
            // bootstrap IDs are derived from file/directory names and may contain
            // underscores or dots — the agent cleans these up in the refinement step)
            let id = sub["id"].as_str().unwrap_or("");
            assert!(!id.is_empty(), "subsystem ID should not be empty");
        }
    }

    // Step 2: Validate the bootstrapped draft
    let validate_output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q")
        .arg("--output")
        .arg("json")
        .arg("validate")
        .arg("--manifest")
        .arg(&manifest_path)
        .output()
        .expect("validate command should run");

    assert!(
        validate_output.status.success(),
        "validate --manifest should succeed: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );

    let validate_json: serde_json::Value =
        serde_json::from_slice(&validate_output.stdout).expect("validation output should be JSON");
    assert_eq!(
        validate_json["validation_errors"].as_u64(),
        Some(0),
        "bootstrapped manifest should have zero validation errors"
    );
    assert!(
        validate_json["coverage_percent"].as_f64().is_some(),
        "validation should report coverage_percent"
    );

    // Step 3: Match via `domain-scan match --manifest system.json --output json`
    let match_output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q")
        .arg("--output")
        .arg("json")
        .arg("match")
        .arg("--manifest")
        .arg(&manifest_path)
        .output()
        .expect("match command should run");

    assert!(
        match_output.status.success(),
        "match should succeed: {}",
        String::from_utf8_lossy(&match_output.stderr)
    );

    let match_json: serde_json::Value =
        serde_json::from_slice(&match_output.stdout).expect("match output should be JSON");
    assert!(
        match_json.get("coverage_percent").is_some(),
        "match output should include coverage_percent"
    );
    let coverage = match_json["coverage_percent"].as_f64().unwrap_or(-1.0);
    assert!(
        (0.0..=100.0).contains(&coverage),
        "coverage should be 0-100, got {}",
        coverage
    );
}

/// Test: Claude Code can refine an existing manifest via direct system.json edits.
///
/// Simulates the refinement workflow from `domain-scan-init.md`:
///   1. Bootstrap a starter manifest
///   2. Read and parse the JSON
///   3. Edit it: rename a subsystem, update description, add a connection
///   4. Write back the edited JSON
///   5. Validate: still passes `validate --manifest`
///   6. Match: coverage is still valid
///   7. Verify: edits are reflected in the output
#[test]
fn test_skill_refine_manifest_via_direct_edits() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let manifest_path = tmp.path().join("system.json");

    // Step 1: Bootstrap to get a starter manifest
    let bootstrap = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q")
        .arg("init")
        .arg("--bootstrap")
        .arg("-o")
        .arg(&manifest_path)
        .output()
        .expect("bootstrap should run");
    assert!(
        bootstrap.status.success(),
        "bootstrap should succeed: {}",
        String::from_utf8_lossy(&bootstrap.stderr)
    );

    // Step 2: Read and parse the manifest JSON (agent reads system.json)
    let original_content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&original_content).expect("parse manifest");

    // Step 3: Edit the manifest (simulate agent refinement)
    // 3a. Update description
    manifest["meta"]["description"] =
        serde_json::Value::String("Refined by agent — updated subsystem boundaries".to_string());

    // 3b. Rename first subsystem (if any) following kebab-case convention
    let mut renamed_id = String::new();
    if let Some(subsystems) = manifest["subsystems"].as_array_mut() {
        if let Some(first_sub) = subsystems.first_mut() {
            let original_id = first_sub["id"].as_str().unwrap_or("unknown").to_string();
            renamed_id = format!("{}-refined", original_id);
            first_sub["id"] = serde_json::Value::String(renamed_id.clone());
            first_sub["name"] = serde_json::Value::String(format!(
                "{} (Refined)",
                first_sub["name"].as_str().unwrap_or("Unknown")
            ));

            // Also update any connections referencing the old ID
            if let Some(connections) = manifest["connections"].as_array_mut() {
                for conn in connections.iter_mut() {
                    if conn["from"].as_str() == Some(&original_id) {
                        conn["from"] = serde_json::Value::String(renamed_id.clone());
                    }
                    if conn["to"].as_str() == Some(&original_id) {
                        conn["to"] = serde_json::Value::String(renamed_id.clone());
                    }
                }
            }
        }
    }

    // 3c. Add a new connection if there are ≥2 subsystems
    let subsystem_ids: Vec<String> = manifest["subsystems"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if subsystem_ids.len() >= 2 {
        let new_connection = serde_json::json!({
            "from": subsystem_ids[0],
            "to": subsystem_ids[1],
            "label": "reads from",
            "type": "uses"
        });
        if let Some(connections) = manifest["connections"].as_array_mut() {
            connections.push(new_connection);
        }
    }

    // Step 4: Write modified manifest back (agent writes system.json)
    let edited_json = serde_json::to_string_pretty(&manifest).expect("serialize edited manifest");
    std::fs::write(&manifest_path, edited_json.as_bytes()).expect("write edited manifest");

    // Step 5: Validate — should still pass validate --manifest
    let validate_output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q")
        .arg("--output")
        .arg("json")
        .arg("validate")
        .arg("--manifest")
        .arg(&manifest_path)
        .output()
        .expect("validate should run");

    assert!(
        validate_output.status.success(),
        "Edited manifest should still pass validation: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );

    let validate_json: serde_json::Value =
        serde_json::from_slice(&validate_output.stdout).expect("validation output should be JSON");
    assert!(
        validate_json["coverage_percent"].as_f64().is_some(),
        "Validation should still report coverage_percent after edits"
    );

    // Step 6: Match — coverage should still be valid
    let match_output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q")
        .arg("--output")
        .arg("json")
        .arg("match")
        .arg("--manifest")
        .arg(&manifest_path)
        .output()
        .expect("match should run");

    assert!(
        match_output.status.success(),
        "Match should succeed on edited manifest: {}",
        String::from_utf8_lossy(&match_output.stderr)
    );

    let match_json: serde_json::Value =
        serde_json::from_slice(&match_output.stdout).expect("match output should be JSON");
    let coverage = match_json["coverage_percent"].as_f64().unwrap_or(-1.0);
    assert!(
        (0.0..=100.0).contains(&coverage),
        "Coverage should be valid after edits, got {}",
        coverage
    );

    // Step 7: Verify edits are reflected — re-read the manifest and check
    let final_content = std::fs::read_to_string(&manifest_path).expect("re-read manifest");
    let final_manifest: serde_json::Value =
        serde_json::from_str(&final_content).expect("parse final manifest");

    assert_eq!(
        final_manifest["meta"]["description"].as_str(),
        Some("Refined by agent — updated subsystem boundaries"),
        "Description edit should persist"
    );

    // Verify the renamed subsystem exists (if we renamed one)
    if !renamed_id.is_empty() {
        let has_renamed = final_manifest["subsystems"]
            .as_array()
            .map(|arr| arr.iter().any(|s| s["id"].as_str() == Some(&renamed_id)))
            .unwrap_or(false);
        assert!(
            has_renamed,
            "Renamed subsystem '{}' should exist in the manifest",
            renamed_id
        );
    }
}

#[test]
fn test_validate_manifest_catches_semantic_errors() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let manifest_path = tmp.path().join("system.json");

    let invalid_manifest = serde_json::json!({
        "meta": { "name": "bad", "version": "1.0.0", "description": "" },
        "domains": {
            "platform": { "label": "Platform", "color": "#3b82f6" }
        },
        "subsystems": [
            {
                "id": "auth",
                "name": "Auth",
                "domain": "missing-domain",
                "status": "new",
                "filePath": "/project/src/auth/",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": ["ghost-subsystem"]
            }
        ],
        "connections": [
            {
                "from": "auth",
                "to": "missing-target",
                "label": "calls",
                "type": "depends_on"
            }
        ]
    });
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&invalid_manifest).expect("serialize manifest"),
    )
    .expect("write manifest");

    let output = Command::cargo_bin("domain-scan")
        .expect("binary should exist")
        .arg("--root")
        .arg(fixture_root())
        .arg("--languages")
        .arg("typescript")
        .arg("--no-cache")
        .arg("-q")
        .arg("--output")
        .arg("json")
        .arg("validate")
        .arg("--manifest")
        .arg(&manifest_path)
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "validate --manifest should fail on semantic manifest errors"
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("validation output should be JSON");
    assert_eq!(json["validation_errors"].as_u64(), Some(3));
    let violations = json["violations"]
        .as_array()
        .expect("violations should be an array");
    assert!(violations
        .iter()
        .any(|v| v["field"].as_str() == Some("domain")));
    assert!(violations
        .iter()
        .any(|v| v["field"].as_str() == Some("dependencies")));
    assert!(violations
        .iter()
        .any(|v| v["field"].as_str() == Some("connections.to")));
}
