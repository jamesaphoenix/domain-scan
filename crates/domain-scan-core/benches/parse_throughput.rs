//! Benchmark: parse throughput
//!
//! Target: >500 files/sec on 8 cores (parallel via rayon).
//! Uses the existing test fixtures, parsing each file through tree-sitter + query extraction.

use std::path::{Path, PathBuf};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use domain_scan_core::ir::{BuildStatus, Language};
use domain_scan_core::parser;
use domain_scan_core::query_engine;
use rayon::prelude::*;

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
                if path.is_file() && !path.extension().is_some_and(|e| e == "json" || e == "bincode") {
                    files.push((path, *language));
                }
            }
        }
    }

    files
}

/// Parse + extract a single file (sequential, no caching).
fn parse_and_extract_one(path: &Path, language: Language) {
    let (tree, source) = parser::parse_file(path, language)
        .unwrap_or_else(|e| panic!("parse failed: {e}"));
    let _ir = query_engine::extract(&tree, &source, path, language, BuildStatus::Built)
        .unwrap_or_else(|e| panic!("extract failed: {e}"));
}

fn bench_parse_throughput(c: &mut Criterion) {
    let files = fixture_files();
    let file_count = files.len();
    assert!(file_count > 0, "No fixture files found");

    let mut group = c.benchmark_group("parse_throughput");
    group.throughput(Throughput::Elements(file_count as u64));
    group.sample_size(20);

    // Sequential: parse all fixture files one at a time
    group.bench_function(
        BenchmarkId::new("sequential", file_count),
        |b| {
            b.iter(|| {
                for (path, lang) in &files {
                    parse_and_extract_one(black_box(path), *lang);
                }
            });
        },
    );

    // Parallel: parse all fixture files via rayon
    group.bench_function(
        BenchmarkId::new("parallel_rayon", file_count),
        |b| {
            b.iter(|| {
                files.par_iter().for_each(|(path, lang)| {
                    parse_and_extract_one(black_box(path), *lang);
                });
            });
        },
    );

    group.finish();
}

criterion_group!(benches, bench_parse_throughput);
criterion_main!(benches);
