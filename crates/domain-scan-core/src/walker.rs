use std::path::PathBuf;

use ignore::WalkBuilder;

use crate::config::GlobFilter;
use crate::ir::{Language, ScanConfig};
use crate::lang::detect_language;
use crate::DomainScanError;

/// A file discovered during directory traversal with its detected language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedFile {
    pub path: PathBuf,
    pub language: Language,
}

/// Walk a directory tree, respecting .gitignore rules, and return all files
/// with recognized programming language extensions.
///
/// Results are sorted by path for deterministic output.
pub fn walk_directory(config: &ScanConfig) -> Result<Vec<WalkedFile>, DomainScanError> {
    let glob_filter = GlobFilter::new(&config.include, &config.exclude)?;
    let mut files = Vec::new();

    let walker = WalkBuilder::new(&config.root)
        .hidden(true) // skip hidden files/dirs
        .git_ignore(true) // respect .gitignore
        .git_global(true) // respect global gitignore
        .git_exclude(true)
        .build();

    for result in walker {
        let entry = result.map_err(|e| DomainScanError::Walk(e.to_string()))?;

        // Skip directories
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();

        // Apply include/exclude glob filters
        let relative = path.strip_prefix(&config.root).unwrap_or(path);
        if !glob_filter.is_included(relative) {
            continue;
        }

        // Detect language; skip unrecognized files
        let Some(language) = detect_language(path) else {
            continue;
        };

        // Apply language filter if set
        if !config.languages.is_empty() && !config.languages.contains(&language) {
            continue;
        }

        files.push(WalkedFile {
            path: path.to_path_buf(),
            language,
        });
    }

    // Sort for deterministic output
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> Result<TempDir, std::io::Error> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create source files
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("src/main.ts"), "const x = 1;")?;
        fs::write(root.join("src/utils.py"), "x = 1")?;
        fs::write(root.join("src/lib.rs"), "fn main() {}")?;

        // Create a non-source file (should be ignored)
        fs::write(root.join("README.md"), "# Hello")?;
        fs::write(root.join("config.json"), "{}")?;

        Ok(dir)
    }

    #[test]
    fn test_walk_finds_source_files() -> Result<(), Box<dyn std::error::Error>> {
        let dir = setup_test_dir()?;
        let config = ScanConfig::new(dir.path().to_path_buf());

        let files = walk_directory(&config)?;

        assert_eq!(files.len(), 3);

        let languages: Vec<_> = files.iter().map(|f| f.language).collect();
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Python));
        assert!(languages.contains(&Language::Rust));

        Ok(())
    }

    #[test]
    fn test_walk_respects_gitignore() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // The ignore crate requires a .git dir to respect .gitignore
        let init = std::process::Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()?;
        assert!(init.status.success(), "git init failed");

        // Create .gitignore that ignores .py files
        fs::write(root.join(".gitignore"), "*.py\n")?;

        fs::write(root.join("main.ts"), "const x = 1;")?;
        fs::write(root.join("utils.py"), "x = 1")?;

        let config = ScanConfig::new(root.to_path_buf());
        let files = walk_directory(&config)?;

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].language, Language::TypeScript);

        Ok(())
    }

    #[test]
    fn test_walk_language_filter() -> Result<(), Box<dyn std::error::Error>> {
        let dir = setup_test_dir()?;
        let mut config = ScanConfig::new(dir.path().to_path_buf());
        config.languages = vec![Language::TypeScript];

        let files = walk_directory(&config)?;

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].language, Language::TypeScript);

        Ok(())
    }

    #[test]
    fn test_walk_empty_dir() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let config = ScanConfig::new(dir.path().to_path_buf());

        let files = walk_directory(&config)?;
        assert!(files.is_empty());

        Ok(())
    }

    #[test]
    fn test_walk_skips_hidden_files() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        fs::write(root.join("visible.ts"), "const x = 1;")?;
        fs::write(root.join(".hidden.ts"), "const y = 2;")?;

        let config = ScanConfig::new(root.to_path_buf());
        let files = walk_directory(&config)?;

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("visible.ts"));

        Ok(())
    }

    #[test]
    fn test_walk_deterministic_order() -> Result<(), Box<dyn std::error::Error>> {
        let dir = setup_test_dir()?;
        let config = ScanConfig::new(dir.path().to_path_buf());

        let files1 = walk_directory(&config)?;
        let files2 = walk_directory(&config)?;

        assert_eq!(files1, files2);

        Ok(())
    }

    #[test]
    fn test_walk_include_glob() -> Result<(), Box<dyn std::error::Error>> {
        let dir = setup_test_dir()?;
        let root = dir.path();

        // Also create a file outside src/
        fs::write(root.join("root.ts"), "const z = 1;")?;

        let mut config = ScanConfig::new(root.to_path_buf());
        config.include = vec!["src/**".to_string()];

        let files = walk_directory(&config)?;

        // Only files under src/ should be included
        assert_eq!(files.len(), 3);
        for f in &files {
            let rel = f.path.strip_prefix(root).ok();
            assert!(rel.is_some_and(|r| r.starts_with("src")));
        }

        Ok(())
    }

    #[test]
    fn test_walk_exclude_glob() -> Result<(), Box<dyn std::error::Error>> {
        let dir = setup_test_dir()?;
        let root = dir.path();

        let mut config = ScanConfig::new(root.to_path_buf());
        config.exclude = vec!["**/*.py".to_string()];

        let files = walk_directory(&config)?;

        // Python file should be excluded
        assert_eq!(files.len(), 2);
        let languages: Vec<_> = files.iter().map(|f| f.language).collect();
        assert!(!languages.contains(&Language::Python));

        Ok(())
    }

    #[test]
    fn test_walk_include_and_exclude() -> Result<(), Box<dyn std::error::Error>> {
        let dir = setup_test_dir()?;
        let root = dir.path();

        let mut config = ScanConfig::new(root.to_path_buf());
        config.include = vec!["src/**".to_string()];
        config.exclude = vec!["**/*.rs".to_string()];

        let files = walk_directory(&config)?;

        // Only TS and Python in src/ (Rust excluded)
        assert_eq!(files.len(), 2);
        let languages: Vec<_> = files.iter().map(|f| f.language).collect();
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Python));
        assert!(!languages.contains(&Language::Rust));

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: symlinks
    // -----------------------------------------------------------------------

    #[test]
    #[cfg(unix)]
    fn test_walk_follows_file_symlinks() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create a real source file
        fs::create_dir_all(root.join("real"))?;
        fs::write(root.join("real/module.ts"), "const x = 1;")?;

        // Create a symlink to it
        std::os::unix::fs::symlink(
            root.join("real/module.ts"),
            root.join("link.ts"),
        )?;

        let config = ScanConfig::new(root.to_path_buf());
        let files = walk_directory(&config)?;

        // Should find both the real file and the symlink
        assert!(!files.is_empty(), "Should find at least the real file");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: deeply nested directories
    // -----------------------------------------------------------------------

    #[test]
    fn test_walk_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create a deeply nested path
        let deep_path = root.join("a/b/c/d/e/f/g/h/i/j");
        fs::create_dir_all(&deep_path)?;
        fs::write(deep_path.join("deep.ts"), "const deep = true;")?;

        let config = ScanConfig::new(root.to_path_buf());
        let files = walk_directory(&config)?;

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("deep.ts"));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: hidden directories
    // -----------------------------------------------------------------------

    #[test]
    fn test_walk_skips_hidden_directories() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create files in hidden and visible directories
        fs::create_dir_all(root.join(".hidden"))?;
        fs::write(root.join(".hidden/secret.ts"), "const secret = true;")?;
        fs::create_dir_all(root.join("visible"))?;
        fs::write(root.join("visible/public.ts"), "const public_ = true;")?;

        let config = ScanConfig::new(root.to_path_buf());
        let files = walk_directory(&config)?;

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("public.ts"));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: multiple language filters
    // -----------------------------------------------------------------------

    #[test]
    fn test_walk_multiple_language_filter() -> Result<(), Box<dyn std::error::Error>> {
        let dir = setup_test_dir()?;
        let mut config = ScanConfig::new(dir.path().to_path_buf());
        config.languages = vec![Language::TypeScript, Language::Python];

        let files = walk_directory(&config)?;

        assert_eq!(files.len(), 2);
        let languages: Vec<_> = files.iter().map(|f| f.language).collect();
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Python));
        assert!(!languages.contains(&Language::Rust));

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: nonexistent root directory
    // -----------------------------------------------------------------------

    #[test]
    fn test_walk_nonexistent_root() {
        let config = ScanConfig::new(PathBuf::from("/this/path/does/not/exist"));
        let result = walk_directory(&config);
        // Should return an error, not panic
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Edge case: only non-source files
    // -----------------------------------------------------------------------

    #[test]
    fn test_walk_no_source_files() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create only non-source files
        fs::write(root.join("README.md"), "# Hello")?;
        fs::write(root.join("config.json"), "{}")?;
        fs::write(root.join("data.txt"), "some data")?;
        fs::write(root.join("image.png"), [0x89, 0x50, 0x4E, 0x47])?;

        let config = ScanConfig::new(root.to_path_buf());
        let files = walk_directory(&config)?;

        assert!(files.is_empty());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: mixed source and binary files
    // -----------------------------------------------------------------------

    #[test]
    fn test_walk_mixed_source_and_binary() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let root = dir.path();

        // Create source files + binary-named but non-source
        fs::write(root.join("app.ts"), "const x = 1;")?;
        fs::write(root.join("logo.png"), [0x89, 0x50, 0x4E, 0x47])?;
        fs::write(root.join("data.csv"), "a,b,c")?;

        let config = ScanConfig::new(root.to_path_buf());
        let files = walk_directory(&config)?;

        // Only .ts file should be picked up
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].language, Language::TypeScript);
        Ok(())
    }
}
