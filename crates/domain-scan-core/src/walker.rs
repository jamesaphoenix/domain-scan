use std::path::PathBuf;

use ignore::WalkBuilder;

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
    let mut files = Vec::new();

    let walker = WalkBuilder::new(&config.root)
        .hidden(true)     // skip hidden files/dirs
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
}
