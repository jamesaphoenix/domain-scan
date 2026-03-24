//! Real codebase integration tests.
//!
//! These tests clone real open-source repos and scan them end-to-end.
//! They are `#[ignore]`'d by default — run with:
//!
//!     cargo test -p domain-scan-core --test real_codebase -- --ignored
//!
//! CI runs them on a schedule, not on every push.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use domain_scan_core::ir::{BuildStatus, Language, ScanConfig, ScanIndex};
use domain_scan_core::{index, parser, query_engine, walker};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Clone a repo (or reuse a cached clone) into a temp directory.
///
/// Caches clones in `target/test-repos/<name>` so repeated runs don't
/// re-clone. Uses `--depth 1` for speed.
fn clone_or_cache(url: &str, name: &str) -> PathBuf {
    let cache_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-repos")
        .join(name);

    if cache_dir.join(".git").exists() {
        return cache_dir;
    }

    std::fs::create_dir_all(cache_dir.parent().expect("has parent")).expect("mkdir");
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(&cache_dir)
        .status()
        .expect("git clone");
    assert!(status.success(), "Failed to clone {url}");
    cache_dir
}

/// Run the full scan pipeline on a directory.
fn scan_directory(dir: &Path) -> ScanIndex {
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

// ---------------------------------------------------------------------------
// Tests (all #[ignore]'d — run with `--ignored`)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn real_codebase_tokio() {
    let dir = clone_or_cache("https://github.com/tokio-rs/tokio", "tokio");
    let idx = scan_directory(&dir);

    // Smoke: tokio has hundreds of traits and impls
    assert!(
        idx.stats.total_interfaces > 20,
        "Expected 20+ traits, got {}",
        idx.stats.total_interfaces
    );

    // No panics, no empty names, no invalid spans
    for file in &idx.files {
        for iface in &file.interfaces {
            assert!(
                !iface.name.is_empty(),
                "Empty interface name in {:?}",
                file.path
            );
        }
    }
}

#[test]
#[ignore]
fn real_codebase_deno_polyglot() {
    let dir = clone_or_cache("https://github.com/denoland/deno", "deno");
    let idx = scan_directory(&dir);

    // Must detect both Rust and TypeScript files
    let languages: HashSet<Language> = idx.files.iter().map(|f| f.language).collect();
    assert!(
        languages.contains(&Language::Rust),
        "Missing Rust files in deno"
    );
    assert!(
        languages.contains(&Language::TypeScript),
        "Missing TypeScript files in deno"
    );
}

#[test]
#[ignore]
fn real_codebase_nestjs() {
    let dir = clone_or_cache("https://github.com/nestjs/nest", "nestjs");
    let idx = scan_directory(&dir);

    // NestJS should have many services detected
    assert!(
        idx.stats.total_files > 10,
        "Expected 10+ files, got {}",
        idx.stats.total_files
    );
    assert!(
        idx.stats.total_classes > 5,
        "Expected 5+ classes in NestJS, got {}",
        idx.stats.total_classes
    );
}

#[test]
#[ignore]
fn real_codebase_flask() {
    let dir = clone_or_cache("https://github.com/pallets/flask", "flask");
    let idx = scan_directory(&dir);

    // Flask is Python — should have classes and functions
    let languages: HashSet<Language> = idx.files.iter().map(|f| f.language).collect();
    assert!(
        languages.contains(&Language::Python),
        "Missing Python files in flask"
    );
    assert!(
        idx.stats.total_classes > 0,
        "Expected classes in Flask, got {}",
        idx.stats.total_classes
    );
}

#[test]
#[ignore]
fn real_codebase_spring_boot() {
    let dir = clone_or_cache(
        "https://github.com/spring-projects/spring-boot",
        "spring-boot",
    );
    let idx = scan_directory(&dir);

    // Spring Boot is Java-heavy with many interfaces
    let languages: HashSet<Language> = idx.files.iter().map(|f| f.language).collect();
    assert!(
        languages.contains(&Language::Java),
        "Missing Java files in spring-boot"
    );
    assert!(
        idx.stats.total_interfaces > 10,
        "Expected 10+ interfaces in spring-boot, got {}",
        idx.stats.total_interfaces
    );
}
