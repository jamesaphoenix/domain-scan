//! Heuristic manifest builder — bootstraps a `SystemManifest` from scan data.
//!
//! Given a `ScanIndex`, the builder infers:
//! 1. **Domains**: one per top-level `src/` subdirectory (or crate/package root).
//! 2. **Subsystems**: one per second-level directory within each domain.
//! 3. **Connections**: cross-directory imports imply `depends_on` edges.
//!
//! The output is a best-guess `SystemManifest` suitable for manual refinement.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::ir::ScanIndex;
use crate::manifest::{
    Connection, ConnectionType, DomainDef, ManifestMeta, ManifestStatus, ManifestSubsystem,
    SystemManifest,
};

// ---------------------------------------------------------------------------
// Builder configuration
// ---------------------------------------------------------------------------

/// Options for controlling bootstrap heuristics.
#[derive(Debug, Clone)]
pub struct BootstrapOptions {
    /// Project name (used in `meta.name`). Derived from root dir if empty.
    pub project_name: Option<String>,
    /// Minimum number of entities in a directory to qualify as a subsystem.
    pub min_entities: usize,
}

impl Default for BootstrapOptions {
    fn default() -> Self {
        Self {
            project_name: None,
            min_entities: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Static color palette
// ---------------------------------------------------------------------------

const DOMAIN_COLORS: &[&str] = &[
    "#3b82f6", // blue
    "#22c55e", // green
    "#f97316", // orange
    "#a855f7", // purple
    "#ef4444", // red
    "#eab308", // yellow
    "#06b6d4", // cyan
    "#ec4899", // pink
    "#14b8a6", // teal
    "#f59e0b", // amber
    "#6366f1", // indigo
    "#84cc16", // lime
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Bootstrap a `SystemManifest` from a `ScanIndex`.
///
/// This is a deterministic, heuristic-based process:
/// - Domains are inferred from top-level source directories.
/// - Subsystems are inferred from second-level directories.
/// - Connections are inferred from cross-directory import statements.
pub fn bootstrap_manifest(
    index: &ScanIndex,
    options: &BootstrapOptions,
) -> SystemManifest {
    let root = &index.root;

    // Step 1: Group files by their top-level and second-level directories
    let dir_groups = group_files_by_directory(index, root);

    // Step 2: Infer domains from top-level directories
    let domains = infer_domains(&dir_groups);

    // Step 3: Infer subsystems from second-level directories
    let subsystems = infer_subsystems(&dir_groups, root, options.min_entities);

    // Step 4: Infer connections from cross-directory imports
    let connections = infer_connections(index, &subsystems);

    // Step 5: Build the manifest
    let project_name = options
        .project_name
        .clone()
        .or_else(|| {
            root.file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "project".to_string());

    SystemManifest {
        meta: ManifestMeta {
            name: project_name,
            version: "1.0.0".to_string(),
            description: "Auto-generated manifest from domain-scan --bootstrap".to_string(),
        },
        domains: domains
            .into_iter()
            .enumerate()
            .map(|(i, (id, label))| {
                let color = DOMAIN_COLORS[i % DOMAIN_COLORS.len()].to_string();
                (id, DomainDef { label, color })
            })
            .collect(),
        subsystems,
        connections,
    }
}

// ---------------------------------------------------------------------------
// Directory grouping
// ---------------------------------------------------------------------------

/// Intermediate grouping: domain_dir -> subsystem_dir -> file_paths
type DirGroups = BTreeMap<String, BTreeMap<String, Vec<PathBuf>>>;

/// Group scanned files by their top-level and second-level directory relative
/// to the scan root. Files directly in the root are grouped under "_root".
fn group_files_by_directory(index: &ScanIndex, root: &Path) -> DirGroups {
    let mut groups: DirGroups = BTreeMap::new();

    for file in &index.files {
        let rel = match file.path.strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let components: Vec<&str> = rel
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => s.to_str(),
                _ => None,
            })
            .collect();

        // Skip files with fewer than 2 path components (e.g. root-level files)
        // — they don't belong to a clear domain/subsystem hierarchy.
        // Also skip common non-source directories.
        let (domain_dir, subsystem_dir) = match components.len() {
            0 | 1 => {
                // Root-level file or single directory — put in _root domain
                let domain = "_root".to_string();
                let subsys = components.first().map_or("_root", |c| c).to_string();
                (domain, subsys)
            }
            2 => {
                // e.g. src/main.rs → domain="src", subsystem from filename
                let domain = components[0].to_string();
                let subsys = components[0].to_string();
                (domain, subsys)
            }
            _ => {
                // e.g. src/auth/handler.rs → domain="src", subsystem="auth"
                // OR crates/core/src/lib.rs → domain="crates/core", subsystem="src"
                // Heuristic: if first component is "crates" or "packages", use
                // the second component as domain.
                let first = components[0];
                if is_workspace_dir(first)
                    || first == "src"
                    || first == "lib"
                    || first == "app"
                {
                    let domain = components[1].to_string();
                    let subsys = if components.len() > 3 {
                        components[2].to_string()
                    } else {
                        components[1].to_string()
                    };
                    (domain, subsys)
                } else {
                    let domain = first.to_string();
                    let subsys = components[1].to_string();
                    (domain, subsys)
                }
            }
        };

        groups
            .entry(domain_dir)
            .or_default()
            .entry(subsystem_dir)
            .or_default()
            .push(file.path.clone());
    }

    groups
}

/// Check if a directory name is a workspace root (crates, packages, apps, etc.)
fn is_workspace_dir(name: &str) -> bool {
    matches!(
        name,
        "crates" | "packages" | "apps" | "modules" | "services" | "libs" | "workspaces"
    )
}

// ---------------------------------------------------------------------------
// Domain inference
// ---------------------------------------------------------------------------

/// Infer domains from the top-level directory groups.
/// Returns (id, label) pairs in sorted order.
fn infer_domains(groups: &DirGroups) -> Vec<(String, String)> {
    groups
        .keys()
        .map(|dir| {
            let label = humanize_name(dir);
            (dir.clone(), label)
        })
        .collect()
}

/// Convert a directory name to a human-readable label.
/// e.g. "domain-scan-core" → "Domain Scan Core", "auth" → "Auth"
fn humanize_name(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    format!("{upper}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Subsystem inference
// ---------------------------------------------------------------------------

/// Infer subsystems from the directory groups.
fn infer_subsystems(
    groups: &DirGroups,
    root: &Path,
    min_entities: usize,
) -> Vec<ManifestSubsystem> {
    let mut subsystems = Vec::new();
    let mut seen_ids = HashSet::new();

    for (domain, subsys_map) in groups {
        for (subsys_name, files) in subsys_map {
            if files.len() < min_entities {
                continue;
            }

            // Compute the common prefix path for this subsystem's files
            let file_path = compute_common_prefix(files, root);

            // Create a unique ID
            let base_id = if domain == subsys_name {
                subsys_name.clone()
            } else {
                format!("{domain}-{subsys_name}")
            };

            let id = make_unique_id(&base_id, &mut seen_ids);

            subsystems.push(ManifestSubsystem {
                id,
                name: humanize_name(subsys_name),
                domain: domain.clone(),
                status: ManifestStatus::New,
                file_path,
                interfaces: Vec::new(),
                operations: Vec::new(),
                tables: Vec::new(),
                events: Vec::new(),
                children: Vec::new(),
                dependencies: Vec::new(),
            });
        }
    }

    subsystems
}

/// Compute the common directory prefix for a set of file paths, relative to root.
fn compute_common_prefix(files: &[PathBuf], root: &Path) -> PathBuf {
    if files.is_empty() {
        return PathBuf::new();
    }

    let rel_paths: Vec<PathBuf> = files
        .iter()
        .filter_map(|f| f.strip_prefix(root).ok().map(|r| r.to_path_buf()))
        .collect();

    if rel_paths.is_empty() {
        return PathBuf::new();
    }

    // Start with the parent of the first file
    let mut common = match rel_paths[0].parent() {
        Some(p) => p.to_path_buf(),
        None => return PathBuf::new(),
    };

    for path in &rel_paths[1..] {
        let parent = match path.parent() {
            Some(p) => p,
            None => {
                common = PathBuf::new();
                break;
            }
        };

        // Find the longest common prefix
        let common_components: Vec<_> = common.components().collect();
        let path_components: Vec<_> = parent.components().collect();
        let shared = common_components
            .iter()
            .zip(path_components.iter())
            .take_while(|(a, b)| a == b)
            .count();

        common = common_components[..shared]
            .iter()
            .collect::<PathBuf>();
    }

    common
}

/// Make an ID unique by appending a suffix if needed.
fn make_unique_id(base: &str, seen: &mut HashSet<String>) -> String {
    let mut id = base.to_string();
    let mut counter = 2u32;
    while seen.contains(&id) {
        id = format!("{base}-{counter}");
        counter = counter.saturating_add(1);
    }
    seen.insert(id.clone());
    id
}

// ---------------------------------------------------------------------------
// Connection inference
// ---------------------------------------------------------------------------

/// Infer connections from cross-directory imports.
///
/// For each import in a file, we check if the import source path resolves to
/// a file in a different subsystem. If so, we add a `depends_on` connection.
fn infer_connections(
    index: &ScanIndex,
    subsystems: &[ManifestSubsystem],
) -> Vec<Connection> {
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    let root = &index.root;

    for file in &index.files {
        let rel_path = file.path.strip_prefix(root).unwrap_or(&file.path);
        let from_subsystem = find_subsystem_for_path(rel_path, subsystems);
        let from_id = match from_subsystem {
            Some(id) => id,
            None => continue,
        };

        for import in &file.imports {
            // Try to resolve the import source to a subsystem
            let to_id = resolve_import_to_subsystem(&import.source, &file.path, subsystems, root);
            if let Some(to_id) = to_id {
                if to_id != from_id {
                    edges.insert((from_id.clone(), to_id));
                }
            }
        }
    }

    edges
        .into_iter()
        .map(|(from, to)| {
            let label = format!("{from} → {to}");
            Connection {
                from,
                to,
                label,
                connection_type: ConnectionType::DependsOn,
            }
        })
        .collect()
}

/// Find which subsystem a relative file path belongs to (by path prefix match).
fn find_subsystem_for_path(rel_path: &Path, subsystems: &[ManifestSubsystem]) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;

    for sub in subsystems {
        if sub.file_path.as_os_str().is_empty() {
            continue;
        }
        if rel_path.starts_with(&sub.file_path) {
            let depth = sub.file_path.components().count();
            if best.as_ref().is_none_or(|(_, d)| depth > *d) {
                best = Some((&sub.id, depth));
            }
        }
    }

    best.map(|(id, _)| id.to_string())
}

/// Try to resolve an import source string to a subsystem ID.
///
/// Handles common patterns:
/// - Relative imports: `./auth/handler` → resolve relative to importing file
/// - Package imports: `@myapp/auth` → look for "auth" subsystem
/// - Crate imports: `crate::auth::handler` → look for "auth" subsystem
fn resolve_import_to_subsystem(
    source: &str,
    importing_file: &Path,
    subsystems: &[ManifestSubsystem],
    root: &Path,
) -> Option<String> {
    // Try relative path resolution
    if source.starts_with('.') {
        let dir = importing_file.parent()?;
        let resolved = dir.join(source);
        // Normalize the path (remove .. components)
        let normalized = normalize_path(&resolved);
        let rel = normalized.strip_prefix(root).unwrap_or(&normalized);
        return find_subsystem_for_path(rel, subsystems);
    }

    // Try extracting a meaningful name from the import path
    let parts: Vec<&str> = source
        .split(['/', ':'])
        .filter(|s| !s.is_empty())
        .collect();

    // Skip common prefixes
    let meaningful_parts: Vec<&&str> = parts
        .iter()
        .filter(|p| !matches!(**p, "crate" | "super" | "self" | "@" | "src" | "lib"))
        .collect();

    // Try to match any part against subsystem IDs or file paths
    for part in &meaningful_parts {
        let part_lower = part.to_lowercase();
        for sub in subsystems {
            if sub.id.to_lowercase() == part_lower
                || sub.name.to_lowercase().replace(' ', "-") == part_lower
                || sub.name.to_lowercase().replace(' ', "_") == part_lower
            {
                return Some(sub.id.clone());
            }

            // Check if the import path component matches any part of the subsystem file_path
            let sub_path_str = sub.file_path.to_string_lossy();
            if !sub_path_str.is_empty() {
                let sub_components: Vec<&str> = sub_path_str
                    .split(['/', '\\'])
                    .filter(|s| !s.is_empty())
                    .collect();
                if sub_components.iter().any(|c| c.to_lowercase() == part_lower) {
                    return Some(sub.id.clone());
                }
            }
        }
    }

    None
}

/// Normalize a path by resolving `.` and `..` components without filesystem access.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // Pop the last normal component if there is one
                if let Some(last) = components.last() {
                    if !matches!(last, std::path::Component::RootDir | std::path::Component::Prefix(_)) {
                        components.pop();
                        continue;
                    }
                }
                components.push(component);
            }
            std::path::Component::CurDir => {
                // Skip `.` components
            }
            _ => {
                components.push(component);
            }
        }
    }
    components.iter().collect()
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Serialize a `SystemManifest` to pretty-printed JSON.
pub fn serialize_manifest(manifest: &SystemManifest) -> Result<String, crate::DomainScanError> {
    serde_json::to_string_pretty(manifest).map_err(crate::DomainScanError::Serialization)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BuildStatus, IrFile, Language};

    fn make_ir_file(path: &str, imports: Vec<crate::ir::ImportDef>) -> IrFile {
        let mut ir = IrFile::new(
            PathBuf::from(path),
            Language::TypeScript,
            "hash123".to_string(),
            BuildStatus::Built,
        );
        ir.imports = imports;
        ir
    }

    fn make_import(source: &str) -> crate::ir::ImportDef {
        crate::ir::ImportDef {
            source: source.to_string(),
            symbols: Vec::new(),
            is_wildcard: false,
            span: crate::ir::Span {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
                byte_range: (0, 0),
            },
        }
    }

    fn make_scan_index(root: &str, files: Vec<IrFile>) -> ScanIndex {
        let mut index = ScanIndex::new(PathBuf::from(root));
        index.stats.total_files = files.len();
        index.files = files;
        index
    }

    #[test]
    fn test_bootstrap_empty_index() {
        let index = make_scan_index("/project", Vec::new());
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        assert_eq!(manifest.meta.name, "project");
        assert!(manifest.subsystems.is_empty());
        assert!(manifest.connections.is_empty());
        assert!(manifest.domains.is_empty());
    }

    #[test]
    fn test_bootstrap_infers_domains_from_top_level_dirs() {
        let files = vec![
            make_ir_file("/project/src/auth/handler.ts", Vec::new()),
            make_ir_file("/project/src/auth/middleware.ts", Vec::new()),
            make_ir_file("/project/src/billing/invoice.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        assert!(manifest.domains.contains_key("auth"));
        assert!(manifest.domains.contains_key("billing"));
    }

    #[test]
    fn test_bootstrap_infers_subsystems() {
        let files = vec![
            make_ir_file("/project/src/auth/handler.ts", Vec::new()),
            make_ir_file("/project/src/auth/middleware.ts", Vec::new()),
            make_ir_file("/project/src/billing/invoice.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        let subsystem_ids: Vec<&str> = manifest.subsystems.iter().map(|s| s.id.as_str()).collect();
        assert!(subsystem_ids.iter().any(|id| id.contains("auth")));
        assert!(subsystem_ids.iter().any(|id| id.contains("billing")));
    }

    #[test]
    fn test_bootstrap_infers_connections_from_imports() {
        let files = vec![
            make_ir_file(
                "/project/src/billing/invoice.ts",
                vec![make_import("../auth/handler")],
            ),
            make_ir_file("/project/src/auth/handler.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        // Should have at least one connection between billing and auth
        assert!(
            !manifest.connections.is_empty(),
            "Expected at least one inferred connection"
        );
    }

    #[test]
    fn test_bootstrap_workspace_layout() {
        let files = vec![
            make_ir_file("/project/crates/core/src/lib.rs", Vec::new()),
            make_ir_file("/project/crates/core/src/parser.rs", Vec::new()),
            make_ir_file("/project/crates/cli/src/main.rs", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        // Should infer "core" and "cli" as domains
        assert!(manifest.domains.contains_key("core"));
        assert!(manifest.domains.contains_key("cli"));
    }

    #[test]
    fn test_bootstrap_custom_project_name() {
        let index = make_scan_index("/project", Vec::new());
        let options = BootstrapOptions {
            project_name: Some("my-app".to_string()),
            ..Default::default()
        };
        let manifest = bootstrap_manifest(&index, &options);
        assert_eq!(manifest.meta.name, "my-app");
    }

    #[test]
    fn test_humanize_name() {
        assert_eq!(humanize_name("auth"), "Auth");
        assert_eq!(humanize_name("domain-scan-core"), "Domain Scan Core");
        assert_eq!(humanize_name("my_service"), "My Service");
    }

    #[test]
    fn test_serialize_manifest_roundtrip() {
        let files = vec![
            make_ir_file("/project/src/auth/handler.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        let json = serialize_manifest(&manifest);
        assert!(json.is_ok(), "Serialization should succeed");

        let parsed: Result<SystemManifest, _> = serde_json::from_str(&json.as_ref().map_or("".to_string(), |s| s.clone()));
        assert!(parsed.is_ok(), "Deserialized manifest should be valid");
    }

    #[test]
    fn test_make_unique_id() {
        let mut seen = HashSet::new();
        assert_eq!(make_unique_id("auth", &mut seen), "auth");
        assert_eq!(make_unique_id("auth", &mut seen), "auth-2");
        assert_eq!(make_unique_id("auth", &mut seen), "auth-3");
    }

    #[test]
    fn test_bootstrap_produces_valid_system_manifest_schema() {
        let files = vec![
            make_ir_file("/project/src/auth/handler.ts", Vec::new()),
            make_ir_file("/project/src/billing/invoice.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        // Verify the manifest can be serialized and re-parsed
        let json = serialize_manifest(&manifest);
        assert!(json.is_ok());
        let json_str = json.as_ref().map_or("", |s| s.as_str());
        let reparsed = crate::manifest::parse_system_manifest(json_str);
        assert!(reparsed.is_ok(), "Re-parsed manifest should be valid SystemManifest: {:?}", reparsed.err());
    }
}
