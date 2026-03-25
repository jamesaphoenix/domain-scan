//! Benchmark: CLI startup / small project scan
//!
//! Target: <100ms for a project with <50 files.
//! Measures the full pipeline: walk -> parse -> extract -> index -> output.

use std::path::{Path, PathBuf};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use domain_scan_core::ir::{BuildStatus, Language, ScanConfig};
use domain_scan_core::output::{self, OutputFormat};
use domain_scan_core::{index, parser, query_engine, walker};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run the full pipeline on all fixtures (simulates scanning a small project).
fn full_pipeline_scan() -> String {
    let root = fixture_root();
    let config = ScanConfig {
        root: root.clone(),
        include: Vec::new(),
        exclude: Vec::new(),
        languages: Vec::new(), // all languages
        build_status_override: None,
        cache_enabled: false,
        cache_dir: root.join(".bench-cache"),
    };

    // Walk
    let walked = walker::walk_directory(&config).unwrap_or_else(|e| panic!("walk failed: {e}"));

    // Parse + Extract
    let mut ir_files = Vec::new();
    for walked_file in &walked {
        let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)
            .unwrap_or_else(|e| panic!("parse failed: {e}"));

        let build_status = config.build_status_override.unwrap_or(BuildStatus::Built);

        let ir = query_engine::extract(
            &tree,
            &source,
            &walked_file.path,
            walked_file.language,
            build_status,
        )
        .unwrap_or_else(|e| panic!("extract failed: {e}"));

        ir_files.push(ir);
    }

    // Index
    let scan_index = index::build_index(root, ir_files, 0, 0, 0);

    // Output
    output::format_scan_index(&scan_index, OutputFormat::Json)
        .unwrap_or_else(|e| panic!("format failed: {e}"))
}

/// Run the full pipeline for just TypeScript fixtures (simulates scanning a TS project).
fn ts_only_pipeline_scan() -> String {
    let root = fixture_root();
    let config = ScanConfig {
        root: root.clone(),
        include: Vec::new(),
        exclude: Vec::new(),
        languages: vec![Language::TypeScript],
        build_status_override: None,
        cache_enabled: false,
        cache_dir: root.join(".bench-cache"),
    };

    let walked = walker::walk_directory(&config).unwrap_or_else(|e| panic!("walk failed: {e}"));

    let mut ir_files = Vec::new();
    for walked_file in &walked {
        let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)
            .unwrap_or_else(|e| panic!("parse failed: {e}"));

        let build_status = config.build_status_override.unwrap_or(BuildStatus::Built);

        let ir = query_engine::extract(
            &tree,
            &source,
            &walked_file.path,
            walked_file.language,
            build_status,
        )
        .unwrap_or_else(|e| panic!("extract failed: {e}"));

        ir_files.push(ir);
    }

    let scan_index = index::build_index(root, ir_files, 0, 0, 0);

    output::format_scan_index(&scan_index, OutputFormat::Json)
        .unwrap_or_else(|e| panic!("format failed: {e}"))
}

fn bench_cli_startup(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_startup");
    group.sample_size(20);

    // Full pipeline: all languages, all fixtures (~66 files)
    group.bench_function("full_pipeline_all_languages", |b| {
        b.iter(|| {
            black_box(full_pipeline_scan());
        });
    });

    // TypeScript only (~12 files)
    group.bench_function("full_pipeline_typescript_only", |b| {
        b.iter(|| {
            black_box(ts_only_pipeline_scan());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_cli_startup);
criterion_main!(benches);
