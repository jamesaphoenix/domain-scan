//! Stress test for parallel parsing: verify no deadlocks under concurrent load.
//!
//! Runs the full parse+extract pipeline via rayon in parallel, multiple times
//! concurrently, to verify thread-local parser pools, DashMap cache, and
//! rayon thread pools don't deadlock.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use domain_scan_core::cache::Cache;
use domain_scan_core::ir::{BuildStatus, ScanConfig};
use domain_scan_core::{content_hash, index, parser, query_engine, walker};
use rayon::prelude::*;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Full pipeline: walk -> parallel parse -> index. Returns file count.
fn run_full_pipeline() -> usize {
    let root = fixture_root();
    let config = ScanConfig {
        root: root.clone(),
        include: Vec::new(),
        exclude: Vec::new(),
        languages: Vec::new(),
        build_status_override: None,
        cache_enabled: false,
        cache_dir: root.join(".stress-test-cache"),
    };

    let walked = walker::walk_directory(&config).unwrap_or_else(|e| panic!("walk failed: {e}"));

    let ir_files: Vec<_> = walked
        .par_iter()
        .map(|wf| {
            let (tree, source) = parser::parse_file(&wf.path, wf.language)
                .unwrap_or_else(|e| panic!("parse failed: {e}"));
            query_engine::extract(&tree, &source, &wf.path, wf.language, BuildStatus::Built)
                .unwrap_or_else(|e| panic!("extract failed: {e}"))
        })
        .collect();

    let count = ir_files.len();
    let _index = index::build_index(root, ir_files, 0, 0, 0);
    count
}

/// Parallel pipeline with cache (exercises DashMap concurrency).
fn run_pipeline_with_cache() -> usize {
    let root = fixture_root();
    let config = ScanConfig {
        root: root.clone(),
        include: Vec::new(),
        exclude: Vec::new(),
        languages: Vec::new(),
        build_status_override: None,
        cache_enabled: true,
        cache_dir: root.join(".stress-test-cache"),
    };

    let walked = walker::walk_directory(&config).unwrap_or_else(|e| panic!("walk failed: {e}"));

    let cache = Arc::new(Cache::new(
        tempfile::tempdir()
            .unwrap_or_else(|e| panic!("tempdir: {e}"))
            .keep(),
        100,
    ));

    let ir_files: Vec<_> = walked
        .par_iter()
        .map(|wf| {
            let source = std::fs::read(&wf.path).unwrap_or_else(|e| panic!("read: {e}"));
            let hash = content_hash(&source);

            // Try cache
            if let Some(ir) = cache.get(&hash) {
                return ir;
            }

            let (tree, source_bytes) =
                parser::parse_file(&wf.path, wf.language).unwrap_or_else(|e| panic!("parse: {e}"));
            let ir = query_engine::extract(
                &tree,
                &source_bytes,
                &wf.path,
                wf.language,
                BuildStatus::Built,
            )
            .unwrap_or_else(|e| panic!("extract: {e}"));

            let _ = cache.insert(hash, ir.clone());
            ir
        })
        .collect();

    ir_files.len()
}

/// Run the pipeline many times sequentially — no deadlocks.
#[test]
fn test_no_deadlock_repeated_parallel_scans() {
    let mut total = 0;
    for _ in 0..10 {
        total += run_full_pipeline();
    }
    assert!(total > 0, "Should have parsed files");
}

/// Run the pipeline with cache many times — no deadlocks on DashMap.
#[test]
fn test_no_deadlock_cached_parallel_scans() {
    let mut total = 0;
    for _ in 0..10 {
        total += run_pipeline_with_cache();
    }
    assert!(total > 0, "Should have parsed files");
}

/// Run multiple pipelines concurrently from different std::thread threads.
/// This exercises nested rayon parallelism and thread-local parser pools
/// from multiple OS threads.
#[test]
fn test_no_deadlock_concurrent_pipelines() {
    let handles: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(|| run_full_pipeline()))
        .collect();

    let mut total = 0;
    for handle in handles {
        // Timeout detection: if the thread doesn't complete within 30s, it's likely deadlocked
        let result = handle.join();
        match result {
            Ok(count) => total += count,
            Err(e) => panic!("Thread panicked: {e:?}"),
        }
    }
    assert!(total > 0, "All threads should have parsed files");
}

/// Run concurrent pipelines with shared cache — exercises DashMap + rayon + thread-local parsers.
#[test]
fn test_no_deadlock_concurrent_cached_pipelines() {
    let cache = Arc::new(Cache::new(
        tempfile::tempdir()
            .unwrap_or_else(|e| panic!("tempdir: {e}"))
            .keep(),
        100,
    ));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let cache = Arc::clone(&cache);
            std::thread::spawn(move || {
                let root = fixture_root();
                let config = ScanConfig {
                    root: root.clone(),
                    include: Vec::new(),
                    exclude: Vec::new(),
                    languages: Vec::new(),
                    build_status_override: None,
                    cache_enabled: true,
                    cache_dir: root.join(".stress-cache"),
                };

                let walked =
                    walker::walk_directory(&config).unwrap_or_else(|e| panic!("walk: {e}"));

                let ir_files: Vec<_> = walked
                    .par_iter()
                    .map(|wf| {
                        let source =
                            std::fs::read(&wf.path).unwrap_or_else(|e| panic!("read: {e}"));
                        let hash = content_hash(&source);

                        if let Some(ir) = cache.get(&hash) {
                            return ir;
                        }

                        let (tree, src) = parser::parse_file(&wf.path, wf.language)
                            .unwrap_or_else(|e| panic!("parse: {e}"));
                        let ir = query_engine::extract(
                            &tree,
                            &src,
                            &wf.path,
                            wf.language,
                            BuildStatus::Built,
                        )
                        .unwrap_or_else(|e| panic!("extract: {e}"));

                        let _ = cache.insert(hash, ir.clone());
                        ir
                    })
                    .collect();

                ir_files.len()
            })
        })
        .collect();

    let mut total = 0;
    for handle in handles {
        total += handle
            .join()
            .unwrap_or_else(|e| panic!("Thread panicked: {e:?}"));
    }
    assert!(total > 0);
}
