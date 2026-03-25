use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use crate::ir::{Language, ScanConfig};
use crate::DomainScanError;

// ---------------------------------------------------------------------------
// TOML Config File Structure
// ---------------------------------------------------------------------------

/// The on-disk `.domain-scan.toml` configuration file.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub project: ProjectSection,
    #[serde(default)]
    pub scan: ScanSection,
    #[serde(default)]
    pub cache: CacheSection,
    #[serde(default)]
    pub services: ServicesSection,
    #[serde(default)]
    pub output: OutputSection,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ProjectSection {
    #[serde(default)]
    pub name: Option<String>,
    /// Scan root, relative to config file location.
    #[serde(default)]
    pub root: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ScanSection {
    /// Glob patterns for files to include. Empty = include all.
    #[serde(default)]
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Language filter. Empty = all detected.
    #[serde(default)]
    pub languages: Vec<String>,
    /// Follow symbolic links during walk.
    #[serde(default)]
    pub follow_symlinks: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CacheSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Cache directory, relative to project root.
    #[serde(default = "default_cache_dir")]
    pub dir: String,
    /// Maximum cache size in MB.
    #[serde(default = "default_max_size_mb")]
    pub max_size_mb: u64,
}

impl Default for CacheSection {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: default_cache_dir(),
            max_size_mb: default_max_size_mb(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_cache_dir() -> String {
    ".domain-scan/cache".to_string()
}

fn default_max_size_mb() -> u64 {
    100
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ServicesSection {
    /// Custom service detection patterns.
    #[serde(default)]
    pub custom: Vec<CustomServiceDef>,
}

/// A user-defined service detection pattern from config.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CustomServiceDef {
    pub name: String,
    /// File glob pattern for matching.
    pub pattern: String,
    /// Required decorator/annotation (optional).
    #[serde(default)]
    pub decorator: Option<String>,
    /// Required trait implementation (optional).
    #[serde(default)]
    pub trait_name: Option<String>,
    /// Service kind string (maps to ServiceKind::Custom).
    #[serde(default = "default_custom_kind")]
    pub kind: String,
}

fn default_custom_kind() -> String {
    "custom".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OutputSection {
    /// Default output format: json | table | compact.
    #[serde(default = "default_output_format")]
    pub default_format: String,
    /// Show full file paths or relative.
    #[serde(default = "default_true")]
    pub show_file_paths: bool,
    /// Sort results by: name | file | kind | methods.
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
}

impl Default for OutputSection {
    fn default() -> Self {
        Self {
            default_format: default_output_format(),
            show_file_paths: true,
            sort_by: default_sort_by(),
        }
    }
}

fn default_output_format() -> String {
    "table".to_string()
}

fn default_sort_by() -> String {
    "name".to_string()
}

// ---------------------------------------------------------------------------
// GlobFilter - compiled include/exclude patterns
// ---------------------------------------------------------------------------

/// Compiled glob patterns for efficient file filtering.
#[derive(Debug, Clone)]
pub struct GlobFilter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

impl GlobFilter {
    /// Build a GlobFilter from include/exclude pattern lists.
    pub fn new(include: &[String], exclude: &[String]) -> Result<Self, DomainScanError> {
        let include = if include.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pattern in include {
                let glob = Glob::new(pattern).map_err(|e| {
                    DomainScanError::Config(format!("invalid include glob '{pattern}': {e}"))
                })?;
                builder.add(glob);
            }
            Some(builder.build().map_err(|e| {
                DomainScanError::Config(format!("failed to compile include globs: {e}"))
            })?)
        };

        let exclude = if exclude.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pattern in exclude {
                let glob = Glob::new(pattern).map_err(|e| {
                    DomainScanError::Config(format!("invalid exclude glob '{pattern}': {e}"))
                })?;
                builder.add(glob);
            }
            Some(builder.build().map_err(|e| {
                DomainScanError::Config(format!("failed to compile exclude globs: {e}"))
            })?)
        };

        Ok(Self { include, exclude })
    }

    /// Returns true if the path should be included in the scan.
    pub fn is_included(&self, path: &Path) -> bool {
        // If include patterns are set, the path must match at least one
        if let Some(ref include) = self.include {
            if !include.is_match(path) {
                return false;
            }
        }

        // If exclude patterns are set, the path must not match any
        if let Some(ref exclude) = self.exclude {
            if exclude.is_match(path) {
                return false;
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// Config Loading
// ---------------------------------------------------------------------------

/// Search for `.domain-scan.toml` starting from `start_dir` and walking up
/// to parent directories. Returns the path if found.
pub fn find_config(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        let candidate = current.join(".domain-scan.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Load and parse a `.domain-scan.toml` file from the given path.
pub fn load_config(path: &Path) -> Result<ConfigFile, DomainScanError> {
    let content = std::fs::read_to_string(path).map_err(DomainScanError::Io)?;
    parse_config(&content)
}

/// Parse a TOML string into a `ConfigFile`.
pub fn parse_config(content: &str) -> Result<ConfigFile, DomainScanError> {
    toml::from_str(content)
        .map_err(|e| DomainScanError::Config(format!("failed to parse config: {e}")))
}

/// Convert a `ConfigFile` into a `ScanConfig`, resolving paths relative to
/// the config file's parent directory.
pub fn config_to_scan_config(
    config: &ConfigFile,
    config_dir: &Path,
) -> Result<ScanConfig, DomainScanError> {
    // Resolve root relative to config file location
    let root = match &config.project.root {
        Some(r) => config_dir.join(r),
        None => config_dir.to_path_buf(),
    };

    // Parse language strings into Language enum values
    let languages = config
        .scan
        .languages
        .iter()
        .filter_map(|s| parse_language(s))
        .collect();

    let cache_dir = root.join(&config.cache.dir);

    Ok(ScanConfig {
        root,
        include: config.scan.include.clone(),
        exclude: config.scan.exclude.clone(),
        languages,
        build_status_override: None,
        cache_enabled: config.cache.enabled,
        cache_dir,
    })
}

/// Parse a language string (case-insensitive) into a Language enum.
fn parse_language(s: &str) -> Option<Language> {
    match s.to_lowercase().as_str() {
        "typescript" | "ts" => Some(Language::TypeScript),
        "python" | "py" => Some(Language::Python),
        "rust" | "rs" => Some(Language::Rust),
        "go" | "golang" => Some(Language::Go),
        "java" => Some(Language::Java),
        "kotlin" | "kt" => Some(Language::Kotlin),
        "csharp" | "c#" | "cs" => Some(Language::CSharp),
        "swift" => Some(Language::Swift),
        "php" => Some(Language::PHP),
        "ruby" | "rb" => Some(Language::Ruby),
        "scala" => Some(Language::Scala),
        "cpp" | "c++" | "cxx" => Some(Language::Cpp),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_config() {
        let config = parse_config("").ok();
        assert!(config.is_some());
        let config = config.as_ref();
        assert!(config.is_some_and(|c| c.project.name.is_none()));
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[project]
name = "my-project"
root = "."

[scan]
include = ["src/**", "lib/**"]
exclude = ["**/node_modules/**", "**/target/**"]
languages = ["typescript", "rust"]
follow_symlinks = false

[cache]
enabled = true
dir = ".domain-scan/cache"
max_size_mb = 100

[[services.custom]]
name = "DomainService"
pattern = "**/*Service.ts"
decorator = "@DomainService"
kind = "microservice"

[[services.custom]]
name = "EventProcessor"
pattern = "src/processors/**/*.rs"
trait_name = "EventProcessor"
kind = "event-handler"

[output]
default_format = "table"
show_file_paths = true
sort_by = "name"
"#;

        let config = parse_config(toml);
        assert!(config.is_ok());
        let config = config.ok();
        let config = config.as_ref();

        assert!(config.is_some_and(|c| c.project.name.as_deref() == Some("my-project")));
        let c = config.as_ref().copied();
        let c = c.as_ref();

        let scan = c.map(|c| &c.scan);
        assert!(scan.is_some_and(|s| s.include.len() == 2));
        assert!(scan.is_some_and(|s| s.exclude.len() == 2));
        assert!(scan.is_some_and(|s| s.languages.len() == 2));

        let services = c.map(|c| &c.services);
        assert!(services.is_some_and(|s| s.custom.len() == 2));

        let first_svc = c.and_then(|c| c.services.custom.first());
        assert!(first_svc.is_some_and(|s| s.name == "DomainService"));
        assert!(first_svc.is_some_and(|s| s.decorator.as_deref() == Some("@DomainService")));

        let second_svc = c.and_then(|c| c.services.custom.get(1));
        assert!(second_svc.is_some_and(|s| s.name == "EventProcessor"));
        assert!(second_svc.is_some_and(|s| s.trait_name.as_deref() == Some("EventProcessor")));
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[project]
name = "minimal"
"#;

        let config = parse_config(toml);
        assert!(config.is_ok());
        let c = config.as_ref().ok();
        // Defaults should be applied
        assert!(c.is_some_and(|c| c.cache.enabled));
        assert!(c.is_some_and(|c| c.cache.max_size_mb == 100));
        assert!(c.is_some_and(|c| c.scan.include.is_empty()));
        assert!(c.is_some_and(|c| c.services.custom.is_empty()));
        assert!(c.is_some_and(|c| c.output.default_format == "table"));
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = parse_config("this is not [valid toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_to_scan_config() {
        let mut config = ConfigFile::default();
        config.project.root = Some(".".to_string());
        config.scan.include = vec!["src/**".to_string()];
        config.scan.exclude = vec!["**/test/**".to_string()];
        config.scan.languages = vec!["typescript".to_string(), "rust".to_string()];
        config.cache.enabled = false;

        let scan_config = config_to_scan_config(&config, Path::new("/tmp/project"));
        assert!(scan_config.is_ok());
        let sc = scan_config.as_ref().ok();

        assert!(sc.is_some_and(|c| c.root == Path::new("/tmp/project/.")));
        assert!(sc.is_some_and(|c| c.languages.len() == 2));
        assert!(sc.is_some_and(|c| c.languages.contains(&Language::TypeScript)));
        assert!(sc.is_some_and(|c| c.languages.contains(&Language::Rust)));
        assert!(sc.is_some_and(|c| !c.cache_enabled));
        assert!(sc.is_some_and(|c| c.include.len() == 1));
        assert!(sc.is_some_and(|c| c.exclude.len() == 1));
    }

    #[test]
    fn test_config_to_scan_config_no_root() {
        let config = ConfigFile::default();
        let scan_config = config_to_scan_config(&config, Path::new("/tmp/project"));
        assert!(scan_config.is_ok());
        let sc = scan_config.as_ref().ok();
        assert!(sc.is_some_and(|c| c.root == Path::new("/tmp/project")));
    }

    #[test]
    fn test_parse_language_variants() {
        assert_eq!(parse_language("typescript"), Some(Language::TypeScript));
        assert_eq!(parse_language("ts"), Some(Language::TypeScript));
        assert_eq!(parse_language("TypeScript"), Some(Language::TypeScript));
        assert_eq!(parse_language("python"), Some(Language::Python));
        assert_eq!(parse_language("py"), Some(Language::Python));
        assert_eq!(parse_language("rust"), Some(Language::Rust));
        assert_eq!(parse_language("go"), Some(Language::Go));
        assert_eq!(parse_language("golang"), Some(Language::Go));
        assert_eq!(parse_language("java"), Some(Language::Java));
        assert_eq!(parse_language("kotlin"), Some(Language::Kotlin));
        assert_eq!(parse_language("kt"), Some(Language::Kotlin));
        assert_eq!(parse_language("csharp"), Some(Language::CSharp));
        assert_eq!(parse_language("c#"), Some(Language::CSharp));
        assert_eq!(parse_language("cs"), Some(Language::CSharp));
        assert_eq!(parse_language("swift"), Some(Language::Swift));
        assert_eq!(parse_language("php"), Some(Language::PHP));
        assert_eq!(parse_language("ruby"), Some(Language::Ruby));
        assert_eq!(parse_language("rb"), Some(Language::Ruby));
        assert_eq!(parse_language("scala"), Some(Language::Scala));
        assert_eq!(parse_language("cpp"), Some(Language::Cpp));
        assert_eq!(parse_language("c++"), Some(Language::Cpp));
        assert_eq!(parse_language("unknown"), None);
    }

    #[test]
    fn test_glob_filter_empty() {
        let filter = GlobFilter::new(&[], &[]);
        assert!(filter.is_ok());
        let filter = filter.as_ref().ok();
        assert!(filter.is_some_and(|f| f.is_included(Path::new("anything.ts"))));
    }

    #[test]
    fn test_glob_filter_include_only() {
        let include = vec!["src/**".to_string()];
        let filter = GlobFilter::new(&include, &[]);
        assert!(filter.is_ok());
        let f = filter.as_ref().ok();
        assert!(f.is_some_and(|f| f.is_included(Path::new("src/main.ts"))));
        assert!(f.is_some_and(|f| !f.is_included(Path::new("lib/utils.ts"))));
    }

    #[test]
    fn test_glob_filter_exclude_only() {
        let exclude = vec!["**/node_modules/**".to_string()];
        let filter = GlobFilter::new(&[], &exclude);
        assert!(filter.is_ok());
        let f = filter.as_ref().ok();
        assert!(f.is_some_and(|f| f.is_included(Path::new("src/main.ts"))));
        assert!(f.is_some_and(|f| !f.is_included(Path::new("node_modules/foo/index.ts"))));
    }

    #[test]
    fn test_glob_filter_include_and_exclude() {
        let include = vec!["src/**".to_string()];
        let exclude = vec!["**/*.test.ts".to_string()];
        let filter = GlobFilter::new(&include, &exclude);
        assert!(filter.is_ok());
        let f = filter.as_ref().ok();
        assert!(f.is_some_and(|f| f.is_included(Path::new("src/main.ts"))));
        assert!(f.is_some_and(|f| !f.is_included(Path::new("src/main.test.ts"))));
        assert!(f.is_some_and(|f| !f.is_included(Path::new("lib/main.ts"))));
    }

    #[test]
    fn test_glob_filter_invalid_pattern() {
        let include = vec!["[invalid".to_string()];
        let result = GlobFilter::new(&include, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_config_in_tempdir() {
        let dir = tempfile::TempDir::new();
        if let Ok(dir) = dir {
            let config_path = dir.path().join(".domain-scan.toml");
            std::fs::write(&config_path, "[project]\nname = \"test\"\n").ok();
            let found = find_config(dir.path());
            assert!(found.is_some_and(|p| p == config_path));
        }
    }

    #[test]
    fn test_find_config_walks_up() {
        let dir = tempfile::TempDir::new();
        if let Ok(dir) = dir {
            // Put config at root
            let config_path = dir.path().join(".domain-scan.toml");
            std::fs::write(&config_path, "[project]\nname = \"test\"\n").ok();

            // Create a subdirectory
            let sub = dir.path().join("src").join("deep");
            std::fs::create_dir_all(&sub).ok();

            // Should find config from subdirectory
            let found = find_config(&sub);
            assert!(found.is_some_and(|p| p == config_path));
        }
    }

    #[test]
    fn test_find_config_not_found() {
        let dir = tempfile::TempDir::new();
        if let Ok(dir) = dir {
            let found = find_config(dir.path());
            assert!(found.is_none());
        }
    }

    #[test]
    fn test_load_config_file() {
        let dir = tempfile::TempDir::new();
        if let Ok(dir) = dir {
            let config_path = dir.path().join(".domain-scan.toml");
            let content = r#"
[project]
name = "test-project"

[scan]
include = ["src/**"]
exclude = ["**/test/**"]
"#;
            std::fs::write(&config_path, content).ok();

            let result = load_config(&config_path);
            assert!(result.is_ok());
            let c = result.as_ref().ok();
            assert!(c.is_some_and(|c| c.project.name.as_deref() == Some("test-project")));
            assert!(c.is_some_and(|c| c.scan.include.len() == 1));
        }
    }

    #[test]
    fn test_load_config_missing_file() {
        let result = load_config(Path::new("/nonexistent/.domain-scan.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_service_def_defaults() {
        let toml = r#"
[[services.custom]]
name = "MyService"
pattern = "**/*Service.ts"
"#;
        let config = parse_config(toml);
        assert!(config.is_ok());
        let c = config.as_ref().ok();
        let svc = c.and_then(|c| c.services.custom.first());
        assert!(svc.is_some_and(|s| s.kind == "custom"));
        assert!(svc.is_some_and(|s| s.decorator.is_none()));
        assert!(svc.is_some_and(|s| s.trait_name.is_none()));
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = ConfigFile {
            project: ProjectSection {
                name: Some("test".to_string()),
                root: Some(".".to_string()),
            },
            scan: ScanSection {
                include: vec!["src/**".to_string()],
                exclude: vec!["**/test/**".to_string()],
                languages: vec!["typescript".to_string()],
                follow_symlinks: false,
            },
            cache: CacheSection::default(),
            services: ServicesSection {
                custom: vec![CustomServiceDef {
                    name: "Svc".to_string(),
                    pattern: "**/*.ts".to_string(),
                    decorator: None,
                    trait_name: None,
                    kind: "custom".to_string(),
                }],
            },
            output: OutputSection::default(),
        };

        let serialized = toml::to_string(&config);
        assert!(serialized.is_ok());
        if let Ok(s) = serialized {
            let deserialized: Result<ConfigFile, _> = toml::from_str(&s);
            assert!(deserialized.is_ok());
            assert!(deserialized.ok().is_some_and(|d| d == config));
        }
    }
}
