//! Concurrent scan adversarial tests.
//!
//! Multiple scans running in parallel must not race, deadlock, or
//! produce corrupt indices. These tests exercise the thread-local
//! parser pool and DashMap cache under concurrent load.

use std::path::PathBuf;
use std::sync::Arc;

use domain_scan_core::ir::{BuildStatus, ScanConfig};
use domain_scan_core::{index, parser, query_engine, walker};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typescript")
}

fn scan_fixture_dir(dir: &std::path::Path) -> domain_scan_core::ir::ScanIndex {
    let config = ScanConfig::new(dir.to_path_buf());
    let walked = walker::walk_directory(&config).unwrap_or_default();
    let build_status = BuildStatus::Built;
    let mut ir_files = Vec::new();
    for wf in &walked {
        if let Ok((tree, source)) = parser::parse_file(&wf.path, wf.language) {
            if let Ok(ir) =
                query_engine::extract(&tree, &source, &wf.path, wf.language, build_status)
            {
                ir_files.push(ir);
            }
        }
    }
    index::build_index(dir.to_path_buf(), ir_files, 0, 0, 0)
}

#[test]
fn concurrent_scans_produce_consistent_results() {
    let dir = fixtures_dir();
    // Run the scan once to get the baseline
    let baseline = scan_fixture_dir(&dir);

    // Run 4 concurrent scans and verify they all produce the same result
    let dir = Arc::new(dir);
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let d = Arc::clone(&dir);
            std::thread::spawn(move || scan_fixture_dir(&d))
        })
        .collect();

    for handle in handles {
        let result = handle.join().expect("thread panicked");
        assert_eq!(
            result.stats.total_files, baseline.stats.total_files,
            "Concurrent scan produced different file count"
        );
        assert_eq!(
            result.stats.total_interfaces, baseline.stats.total_interfaces,
            "Concurrent scan produced different interface count"
        );
        assert_eq!(
            result.stats.total_services, baseline.stats.total_services,
            "Concurrent scan produced different service count"
        );
    }
}

#[test]
fn concurrent_scans_different_directories_no_interference() {
    let ts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typescript");
    let rust_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rust");

    let ts_handle = std::thread::spawn(move || scan_fixture_dir(&ts_dir));
    let rust_handle = std::thread::spawn(move || scan_fixture_dir(&rust_dir));

    let ts_result = ts_handle.join().expect("ts thread panicked");
    let rust_result = rust_handle.join().expect("rust thread panicked");

    // Both should complete successfully with non-zero entities
    assert!(
        ts_result.stats.total_files > 0,
        "TypeScript scan found no files"
    );
    assert!(
        rust_result.stats.total_files > 0,
        "Rust scan found no files"
    );
}

#[test]
fn rapid_sequential_scans_no_state_leak() {
    let dir = fixtures_dir();
    // Run 10 scans in rapid sequence to check for state leaks
    let mut results = Vec::new();
    for _ in 0..10 {
        results.push(scan_fixture_dir(&dir));
    }
    // All should produce identical stats
    let first = &results[0];
    for (i, r) in results.iter().enumerate().skip(1) {
        assert_eq!(
            r.stats.total_files, first.stats.total_files,
            "Scan {i} produced different file count"
        );
        assert_eq!(
            r.stats.total_interfaces, first.stats.total_interfaces,
            "Scan {i} produced different interface count"
        );
    }
}
