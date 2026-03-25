//! Performance assertion tests — verify that benchmark targets are met.
//!
//! These are standard #[test] functions (not criterion) so they run in CI.
//! The authoritative measurements come from criterion benchmarks (`cargo bench`),
//! which run in release mode. These tests use relaxed thresholds to account for
//! the test profile (opt-level = 1) and CI variability.
//!
//! Spec targets (release mode):
//! - Parse throughput: >500 files/sec on 8 cores
//! - Cached re-scan: >5000 files/sec
//! - CLI startup: <100ms for <50 files

use std::path::{Path, PathBuf};
use std::time::Instant;

use domain_scan_core::cache::Cache;
use domain_scan_core::ir::{BuildStatus, Language, ScanConfig};
use domain_scan_core::output::{self, OutputFormat};
use domain_scan_core::{content_hash, index, parser, query_engine, walker};
use rayon::prelude::*;

fn fixture_files() -> Vec<(PathBuf, Language)> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files = Vec::new();

    let lang_dirs: &[(&str, Language)] = &[
        ("typescript", Language::TypeScript),
        ("rust", Language::Rust),
        ("go", Language::Go),
        ("python", Language::Python),
        ("java", Language::Java),
        ("kotlin", Language::Kotlin),
        ("scala", Language::Scala),
        ("csharp", Language::CSharp),
        ("swift", Language::Swift),
        ("cpp", Language::Cpp),
        ("php", Language::PHP),
        ("ruby", Language::Ruby),
    ];

    for (dir_name, language) in lang_dirs {
        let lang_dir = fixtures_dir.join(dir_name);
        if let Ok(entries) = std::fs::read_dir(&lang_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && !path
                        .extension()
                        .is_some_and(|e| e == "json" || e == "bincode")
                {
                    files.push((path, *language));
                }
            }
        }
    }

    files
}

/// Parse throughput (parallel via rayon): >500 files/sec.
/// Uses parallel parsing to match the spec target ("on 8 cores").
/// Relaxed to 200 files/sec for test profile (opt-level=1).
#[test]
fn test_parse_throughput_target() {
    let files = fixture_files();
    let file_count = files.len();
    assert!(file_count > 0, "No fixture files found");

    let start = Instant::now();
    let results: Vec<_> = files
        .par_iter()
        .map(|(path, lang)| {
            let (tree, source) = parser::parse_file(path, *lang)
                .unwrap_or_else(|e| panic!("parse failed for {}: {e}", path.display()));
            query_engine::extract(&tree, &source, path, *lang, BuildStatus::Built)
                .unwrap_or_else(|e| panic!("extract failed for {}: {e}", path.display()))
        })
        .collect();
    let elapsed = start.elapsed();

    assert_eq!(results.len(), file_count);
    let files_per_sec = file_count as f64 / elapsed.as_secs_f64();
    // Relaxed: 200 files/sec in test profile. Criterion bench (release) verifies >500.
    assert!(
        files_per_sec > 200.0,
        "Parse throughput {files_per_sec:.0} files/sec is below 200 threshold ({file_count} files in {elapsed:?})"
    );
}

/// Cached re-scan: >5000 files/sec with warm cache.
/// Relaxed to 2000 files/sec for test profile.
#[test]
fn test_cached_rescan_target() {
    let files = fixture_files();
    let file_count = files.len();
    assert!(file_count > 0, "No fixture files found");

    let cache_dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let cache = Cache::new(cache_dir.path().to_path_buf(), 100);

    // Populate cache
    let mut file_hashes: Vec<(PathBuf, String)> = Vec::new();
    for (path, lang) in &files {
        let source = std::fs::read(path).unwrap_or_else(|e| panic!("read: {e}"));
        let hash = content_hash(&source);
        let (tree, source_bytes) =
            parser::parse_file(path, *lang).unwrap_or_else(|e| panic!("parse: {e}"));
        let ir = query_engine::extract(&tree, &source_bytes, path, *lang, BuildStatus::Built)
            .unwrap_or_else(|e| panic!("extract: {e}"));
        cache
            .insert(hash.clone(), ir)
            .unwrap_or_else(|e| panic!("cache insert: {e}"));
        file_hashes.push((path.clone(), hash));
    }

    // Measure cache-hit path
    let start = Instant::now();
    for (path, expected_hash) in &file_hashes {
        let source = std::fs::read(path).unwrap_or_else(|e| panic!("read: {e}"));
        let hash = content_hash(&source);
        assert_eq!(&hash, expected_hash);
        let ir = cache.get(&hash);
        assert!(ir.is_some(), "expected cache hit");
    }
    let elapsed = start.elapsed();

    let files_per_sec = file_count as f64 / elapsed.as_secs_f64();
    // Relaxed: 2000 files/sec in test profile. Criterion bench (release) verifies >5000.
    assert!(
        files_per_sec > 2000.0,
        "Cached re-scan {files_per_sec:.0} files/sec is below 2000 threshold ({file_count} files in {elapsed:?})"
    );
}

/// CLI startup: full pipeline completes quickly for small projects.
/// Relaxed to 500ms for test profile. Criterion bench (release) verifies <100ms.
#[test]
fn test_cli_startup_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let config = ScanConfig {
        root: root.clone(),
        include: Vec::new(),
        exclude: Vec::new(),
        languages: Vec::new(),
        build_status_override: None,
        cache_enabled: false,
        cache_dir: root.join(".bench-cache"),
    };

    let start = Instant::now();

    let walked = walker::walk_directory(&config).unwrap_or_else(|e| panic!("walk failed: {e}"));

    let mut ir_files = Vec::new();
    for walked_file in &walked {
        let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)
            .unwrap_or_else(|e| panic!("parse failed for {}: {e}", walked_file.path.display()));

        let build_status = config.build_status_override.unwrap_or(BuildStatus::Built);

        let ir = query_engine::extract(
            &tree,
            &source,
            &walked_file.path,
            walked_file.language,
            build_status,
        )
        .unwrap_or_else(|e| panic!("extract failed for {}: {e}", walked_file.path.display()));

        ir_files.push(ir);
    }

    let scan_index = index::build_index(root, ir_files, 0, 0, 0);

    let _json = output::format_scan_index(&scan_index, OutputFormat::Json)
        .unwrap_or_else(|e| panic!("format failed: {e}"));

    let elapsed = start.elapsed();

    // Relaxed: 500ms in test profile. Criterion bench (release) verifies <100ms.
    assert!(
        elapsed.as_millis() < 500,
        "Full pipeline took {}ms, exceeding 500ms threshold ({} files)",
        elapsed.as_millis(),
        scan_index.stats.total_files,
    );
}
