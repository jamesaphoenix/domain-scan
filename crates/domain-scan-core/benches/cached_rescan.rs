//! Benchmark: cached re-scan
//!
//! Target: >5000 files/sec when cache is warm.
//! Measures the overhead of hash-check + cache-hit vs full parse.

use std::path::{Path, PathBuf};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use domain_scan_core::cache::Cache;
use domain_scan_core::ir::{BuildStatus, Language};
use domain_scan_core::parser;
use domain_scan_core::query_engine;

/// Collect all fixture files with their detected language.
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

fn bench_cached_rescan(c: &mut Criterion) {
    let files = fixture_files();
    let file_count = files.len();
    assert!(file_count > 0, "No fixture files found");

    // Populate cache with all fixture files
    let cache_dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let cache = Cache::new(cache_dir.path().to_path_buf(), 100);

    let mut file_hashes: Vec<(PathBuf, Language, String)> = Vec::new();
    for (path, lang) in &files {
        let source = std::fs::read(path).unwrap_or_else(|e| panic!("read: {e}"));
        let hash = domain_scan_core::content_hash(&source);
        let (tree, source_bytes) =
            parser::parse_file(path, *lang).unwrap_or_else(|e| panic!("parse: {e}"));
        let ir = query_engine::extract(&tree, &source_bytes, path, *lang, BuildStatus::Built)
            .unwrap_or_else(|e| panic!("extract: {e}"));
        cache
            .insert(hash.clone(), ir)
            .unwrap_or_else(|e| panic!("cache insert: {e}"));
        file_hashes.push((path.clone(), *lang, hash));
    }

    let mut group = c.benchmark_group("cached_rescan");
    group.throughput(Throughput::Elements(file_count as u64));
    group.sample_size(50);

    // Benchmark: read source, compute hash, cache lookup (cache hit path)
    group.bench_function(BenchmarkId::new("cache_hit", file_count), |b| {
        b.iter(|| {
            for (path, _lang, expected_hash) in &file_hashes {
                let source = std::fs::read(black_box(path)).unwrap_or_else(|e| panic!("read: {e}"));
                let hash = domain_scan_core::content_hash(&source);
                assert_eq!(&hash, expected_hash);
                let ir = cache.get(&hash);
                assert!(ir.is_some(), "expected cache hit");
                black_box(ir);
            }
        });
    });

    // Benchmark: just the hash computation (to isolate overhead)
    group.bench_function(BenchmarkId::new("hash_only", file_count), |b| {
        // Pre-read all file contents to isolate hashing cost
        let contents: Vec<Vec<u8>> = file_hashes
            .iter()
            .map(|(path, _, _)| std::fs::read(path).unwrap_or_else(|e| panic!("read: {e}")))
            .collect();
        b.iter(|| {
            for content in &contents {
                black_box(domain_scan_core::content_hash(black_box(content)));
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_cached_rescan);
criterion_main!(benches);
