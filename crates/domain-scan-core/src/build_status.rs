use std::path::Path;
use std::time::SystemTime;

use crate::ir::BuildStatus;
use crate::lang::detect_language;
use crate::DomainScanError;

/// Detect the build status of a project at the given root path.
///
/// Heuristics:
/// 1. No build artifacts -> `Unbuilt`
/// 2. Artifacts exist + many uncommitted git changes -> `Rebuild`
/// 3. Artifacts exist + stale (source newer than artifacts) or lock conflicts -> `Error`
/// 4. Artifacts exist + fresh -> `Built`
pub fn detect_build_status(root: &Path) -> Result<BuildStatus, DomainScanError> {
    let artifact_dirs = find_artifact_dirs(root);

    // No artifacts at all -> Unbuilt
    if artifact_dirs.is_empty() {
        return Ok(BuildStatus::Unbuilt);
    }

    // Many uncommitted changes -> Rebuild (active refactor)
    if is_git_repo(root) && has_many_uncommitted_changes(root)? {
        return Ok(BuildStatus::Rebuild);
    }

    // Lock file conflicts or stale artifacts -> Error
    if has_lock_conflicts(root)? || artifacts_stale(root, &artifact_dirs)? {
        return Ok(BuildStatus::Error);
    }

    Ok(BuildStatus::Built)
}

/// Threshold for uncommitted changes to trigger Rebuild status.
const REBUILD_CHANGE_THRESHOLD: usize = 10;

/// Find well-known build artifact directories that exist under `root`.
fn find_artifact_dirs(root: &Path) -> Vec<std::path::PathBuf> {
    let candidates = [
        root.join("target"),       // Rust
        root.join("node_modules"), // TypeScript/JS
        root.join("__pycache__"),  // Python (top-level)
        root.join("build"),        // Java/Kotlin/Scala/general
        root.join(".build"),       // Swift
    ];
    candidates.into_iter().filter(|p| p.exists()).collect()
}

fn is_git_repo(root: &Path) -> bool {
    root.join(".git").exists()
}

/// Check if there are many uncommitted changes (indicates active refactoring).
fn has_many_uncommitted_changes(root: &Path) -> Result<bool, DomainScanError> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        // git command failed; not a valid repo or git not available
        return Ok(false);
    }

    let change_count = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .count();

    Ok(change_count > REBUILD_CHANGE_THRESHOLD)
}

/// Check if any lock file contains merge conflict markers.
fn has_lock_conflicts(root: &Path) -> Result<bool, DomainScanError> {
    let lock_files = [
        "Cargo.lock",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "go.sum",
        "Gemfile.lock",
    ];

    for name in &lock_files {
        let path = root.join(name);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            if content.contains("<<<<<<<") || content.contains(">>>>>>>") {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Check if any source file is newer than the newest artifact directory.
fn artifacts_stale(
    root: &Path,
    artifact_dirs: &[std::path::PathBuf],
) -> Result<bool, DomainScanError> {
    // Find the newest artifact directory mtime
    let newest_artifact_mtime = artifact_dirs
        .iter()
        .filter_map(|d| std::fs::metadata(d).ok())
        .filter_map(|m| m.modified().ok())
        .max();

    let Some(artifact_time) = newest_artifact_mtime else {
        return Ok(false);
    };

    // Walk source files and check if any is newer
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        // Only check recognized source files
        if detect_language(entry.path()).is_none() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if mtime > artifact_time {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Detect build status for a specific language's artifact patterns.
pub fn detect_build_status_for_language(
    root: &Path,
    language: crate::ir::Language,
) -> Result<BuildStatus, DomainScanError> {
    use crate::ir::Language;

    let artifact_exists = match language {
        Language::Rust => root.join("target").exists(),
        Language::TypeScript => {
            root.join("node_modules").exists() || root.join("node_modules/.cache").exists()
        }
        Language::Python => has_pycache(root),
        Language::Java | Language::Kotlin | Language::Scala => {
            root.join("build").exists() || root.join("target").exists()
        }
        Language::Go => root.join("go.sum").exists(),
        Language::CSharp => root.join("bin").exists() || root.join("obj").exists(),
        Language::Swift => root.join(".build").exists(),
        Language::Cpp => root.join("build").exists(),
        Language::PHP => root.join("vendor").exists(),
        Language::Ruby => root.join("Gemfile.lock").exists(),
    };

    if !artifact_exists {
        return Ok(BuildStatus::Unbuilt);
    }

    // Delegate to the general detector for stale/rebuild checks
    detect_build_status(root)
}

/// Check if __pycache__ directories exist anywhere under root (non-recursive, just top-level).
fn has_pycache(root: &Path) -> bool {
    root.join("__pycache__").exists()
}

/// Get the newest modification time of any file in a directory (non-recursive).
fn _newest_file_mtime(dir: &Path) -> Result<Option<SystemTime>, DomainScanError> {
    if !dir.exists() {
        return Ok(None);
    }
    let mut newest: Option<SystemTime> = None;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            if let Ok(mtime) = meta.modified() {
                newest = Some(match newest {
                    Some(current) if current > mtime => current,
                    _ => mtime,
                });
            }
        }
    }
    Ok(newest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Core status detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_unbuilt_no_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Only source files, no artifacts
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("src/main.rs"), "fn main() {}")?;

        let status = detect_build_status(root)?;
        assert_eq!(status, BuildStatus::Unbuilt);
        Ok(())
    }

    #[test]
    fn test_built_fresh_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create source file with old timestamp
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("src/main.rs"), "fn main() {}")?;
        let past =
            filetime::FileTime::from_system_time(SystemTime::now() - Duration::from_secs(3600));
        filetime::set_file_mtime(root.join("src/main.rs"), past)?;

        // Create fresh artifact directory
        fs::create_dir_all(root.join("target/debug"))?;
        fs::write(root.join("target/debug/binary"), "binary")?;

        let status = detect_build_status(root)?;
        assert_eq!(status, BuildStatus::Built);
        Ok(())
    }

    #[test]
    fn test_error_stale_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create artifact directory with old timestamp
        fs::create_dir_all(root.join("target/debug"))?;
        fs::write(root.join("target/debug/binary"), "binary")?;
        let past =
            filetime::FileTime::from_system_time(SystemTime::now() - Duration::from_secs(3600));
        filetime::set_file_mtime(root.join("target"), past)?;

        // Create source file (current time = newer than artifact)
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("src/main.rs"), "fn main() {}")?;

        let status = detect_build_status(root)?;
        assert_eq!(status, BuildStatus::Error);
        Ok(())
    }

    #[test]
    fn test_error_lock_conflicts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create artifacts so it's not Unbuilt
        fs::create_dir_all(root.join("target"))?;

        // Create lock file with conflict markers
        fs::write(
            root.join("Cargo.lock"),
            "<<<<<<< HEAD\nversion1\n=======\nversion2\n>>>>>>>\n",
        )?;

        let status = detect_build_status(root)?;
        assert_eq!(status, BuildStatus::Error);
        Ok(())
    }

    #[test]
    fn test_rebuild_many_uncommitted_changes() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create artifacts so it's not Unbuilt
        fs::create_dir_all(root.join("target/debug"))?;
        fs::write(root.join("target/debug/binary"), "binary")?;

        // Init git repo
        let init = std::process::Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()?;
        assert!(init.status.success(), "git init failed");

        // Initial commit
        fs::write(root.join(".gitignore"), "target/\n")?;
        fs::write(root.join("initial.rs"), "// initial")?;
        let _ = std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .output()?;
        let _ = std::process::Command::new("git")
            .args([
                "-c",
                "user.email=test@test.com",
                "-c",
                "user.name=Test",
                "commit",
                "-m",
                "initial",
            ])
            .current_dir(root)
            .output()?;

        // Create many uncommitted source files (> REBUILD_CHANGE_THRESHOLD)
        for i in 0..(REBUILD_CHANGE_THRESHOLD + 5) {
            fs::write(root.join(format!("file{i}.rs")), format!("// file {i}"))?;
        }

        let status = detect_build_status(root)?;
        assert_eq!(status, BuildStatus::Rebuild);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Language-specific artifact detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_rust_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // No target/ -> Unbuilt
        assert_eq!(
            detect_build_status_for_language(root, crate::ir::Language::Rust)?,
            BuildStatus::Unbuilt
        );

        // With target/ -> delegates to general detector
        fs::create_dir_all(root.join("target"))?;
        let status = detect_build_status_for_language(root, crate::ir::Language::Rust)?;
        assert_ne!(status, BuildStatus::Unbuilt);
        Ok(())
    }

    #[test]
    fn test_typescript_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // No node_modules -> Unbuilt
        assert_eq!(
            detect_build_status_for_language(root, crate::ir::Language::TypeScript)?,
            BuildStatus::Unbuilt
        );

        // With node_modules -> not Unbuilt
        fs::create_dir_all(root.join("node_modules"))?;
        let status = detect_build_status_for_language(root, crate::ir::Language::TypeScript)?;
        assert_ne!(status, BuildStatus::Unbuilt);
        Ok(())
    }

    #[test]
    fn test_python_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // No __pycache__ -> Unbuilt
        assert_eq!(
            detect_build_status_for_language(root, crate::ir::Language::Python)?,
            BuildStatus::Unbuilt
        );

        // With __pycache__ -> not Unbuilt
        fs::create_dir_all(root.join("__pycache__"))?;
        let status = detect_build_status_for_language(root, crate::ir::Language::Python)?;
        assert_ne!(status, BuildStatus::Unbuilt);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_git_repo_skips_rebuild_check() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Artifacts exist but no git repo -> can't be Rebuild
        fs::create_dir_all(root.join("target"))?;

        let status = detect_build_status(root)?;
        assert_ne!(status, BuildStatus::Rebuild);
        Ok(())
    }

    #[test]
    fn test_clean_lock_file_not_error() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        fs::create_dir_all(root.join("target"))?;
        fs::write(root.join("Cargo.lock"), "[package]\nname = \"foo\"\n")?;

        let status = detect_build_status(root)?;
        assert_ne!(status, BuildStatus::Error);
        Ok(())
    }
}
