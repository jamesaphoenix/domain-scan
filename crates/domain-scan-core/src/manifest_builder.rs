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

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ir::ScanIndex;
use crate::manifest::{
    Connection, ConnectionType, DomainDef, ManifestMeta, ManifestStatus, ManifestSubsystem,
    SystemManifest,
};

// ---------------------------------------------------------------------------
// Builder configuration
// ---------------------------------------------------------------------------

/// Options for controlling bootstrap heuristics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
pub fn bootstrap_manifest(index: &ScanIndex, options: &BootstrapOptions) -> SystemManifest {
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
        .or_else(|| root.file_name().map(|n| n.to_string_lossy().into_owned()))
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
                // OR crates/core/src/lib.rs → domain="core", subsystem="core"
                // OR packages/auth/src/handler.ts → domain="auth", subsystem="auth"
                // Heuristic: if first component is a workspace dir ("crates",
                // "packages", etc.) or a source root ("src", "lib", "app"),
                // use the second component as domain and derive subsystem
                // from the next meaningful (non-source-root) component.
                let first = components[0];
                if is_workspace_dir(first) || first == "src" || first == "lib" || first == "app" {
                    let domain = components[1].to_string();
                    // Skip "src"/"lib"/"app" intermediary dirs to avoid
                    // grouping everything under a meaningless "src" subsystem.
                    let subsys = if components.len() > 3 {
                        let third = components[2];
                        if is_source_root(third) && components.len() > 4 {
                            // e.g. packages/auth/src/handlers/... → subsystem="handlers"
                            components[3].to_string()
                        } else if is_source_root(third) {
                            // e.g. packages/auth/src/handler.ts → subsystem="auth" (use domain)
                            components[1].to_string()
                        } else {
                            // e.g. packages/auth/handlers/... → subsystem="handlers"
                            third.to_string()
                        }
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

/// Check if a directory name is a source root that should be skipped during
/// subsystem inference (e.g. `packages/auth/src/...` → skip `src`).
fn is_source_root(name: &str) -> bool {
    matches!(name, "src" | "lib" | "app")
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

    // Fallback: if no subsystems passed the threshold, create one per domain.
    // This prevents bootstrap from producing empty output for monorepos where
    // every individual group is below min_entities.
    if subsystems.is_empty() {
        for (domain, subsys_map) in groups {
            let all_files: Vec<PathBuf> = subsys_map.values().flatten().cloned().collect();
            if !all_files.is_empty() {
                let file_path = compute_common_prefix(&all_files, root);
                let id = make_unique_id(domain, &mut seen_ids);
                subsystems.push(ManifestSubsystem {
                    id,
                    name: humanize_name(domain),
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
        return PathBuf::from(".");
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

        common = common_components[..shared].iter().collect::<PathBuf>();
    }

    if common.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        common
    }
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

/// Maximum number of connections to emit from bootstrap.
/// Prevents the tube map from becoming an unreadable hairball.
const MAX_BOOTSTRAP_CONNECTIONS: usize = 50;

/// Maximum outgoing edges per subsystem. Subsystems that import everything
/// (e.g. barrel files, test harnesses) get their noisiest edges trimmed.
const MAX_EDGES_PER_SUBSYSTEM: usize = 8;

/// Infer connections from cross-directory imports.
///
/// For each import in a file, we check if the import source path resolves to
/// a file in a different subsystem. If so, we add a `depends_on` connection.
/// Results are capped to avoid noisy graphs.
fn infer_connections(index: &ScanIndex, subsystems: &[ManifestSubsystem]) -> Vec<Connection> {
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    // Track how many times each edge is seen (weight = import count)
    let mut edge_weights: BTreeMap<(String, String), usize> = BTreeMap::new();
    let root = &index.root;

    for file in &index.files {
        let rel_path = file.path.strip_prefix(root).unwrap_or(&file.path);
        let from_subsystem = find_subsystem_for_path(rel_path, subsystems);
        let from_id = match from_subsystem {
            Some(id) => id,
            None => continue,
        };

        for import in &file.imports {
            let to_id = resolve_import_to_subsystem(&import.source, &file.path, subsystems, root);
            if let Some(to_id) = to_id {
                if to_id != from_id {
                    let key = (from_id.clone(), to_id);
                    edges.insert(key.clone());
                    *edge_weights.entry(key).or_insert(0) += 1;
                }
            }
        }
    }

    // Per-subsystem outgoing edge cap: keep only the heaviest edges
    let mut outgoing_count: BTreeMap<String, usize> = BTreeMap::new();
    // Sort edges by weight descending so we keep the most significant ones
    let mut weighted_edges: Vec<_> = edges.into_iter().collect::<Vec<_>>();
    weighted_edges.sort_by(|a, b| {
        let wa = edge_weights.get(a).copied().unwrap_or(0);
        let wb = edge_weights.get(b).copied().unwrap_or(0);
        wb.cmp(&wa)
    });

    let mut kept: Vec<(String, String)> = Vec::new();
    for (from, to) in &weighted_edges {
        let count = outgoing_count.get(from.as_str()).copied().unwrap_or(0);
        if count >= MAX_EDGES_PER_SUBSYSTEM {
            continue;
        }
        *outgoing_count.entry(from.clone()).or_insert(0) += 1;
        kept.push((from.clone(), to.clone()));
        if kept.len() >= MAX_BOOTSTRAP_CONNECTIONS {
            break;
        }
    }

    kept.into_iter()
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
    let parts: Vec<&str> = source.split(['/', ':']).filter(|s| !s.is_empty()).collect();

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
                if sub_components
                    .iter()
                    .any(|c| c.to_lowercase() == part_lower)
                {
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
                    if !matches!(
                        last,
                        std::path::Component::RootDir | std::path::Component::Prefix(_)
                    ) {
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
            make_ir_file("/project/src/auth/types.ts", Vec::new()),
            make_ir_file("/project/src/billing/invoice.ts", Vec::new()),
            make_ir_file("/project/src/billing/payment.ts", Vec::new()),
            make_ir_file("/project/src/billing/stripe.ts", Vec::new()),
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
            make_ir_file("/project/src/billing/payment.ts", Vec::new()),
            make_ir_file("/project/src/billing/stripe.ts", Vec::new()),
            make_ir_file("/project/src/auth/handler.ts", Vec::new()),
            make_ir_file("/project/src/auth/middleware.ts", Vec::new()),
            make_ir_file("/project/src/auth/types.ts", Vec::new()),
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
        let files = vec![make_ir_file("/project/src/auth/handler.ts", Vec::new())];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        let json = serialize_manifest(&manifest);
        assert!(json.is_ok(), "Serialization should succeed");

        let parsed: Result<SystemManifest, _> =
            serde_json::from_str(&json.as_ref().map_or("".to_string(), |s| s.clone()));
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
        assert!(
            reparsed.is_ok(),
            "Re-parsed manifest should be valid SystemManifest: {:?}",
            reparsed.err()
        );
    }

    // -----------------------------------------------------------------------
    // B.2: BootstrapOptions default min_entities is 1
    // -----------------------------------------------------------------------

    #[test]
    fn test_bootstrap_min_entities_default() {
        assert_eq!(BootstrapOptions::default().min_entities, 1);
    }

    // -----------------------------------------------------------------------
    // B.3/B.8: Bootstrap on monorepo fixture produces non-empty subsystems
    // -----------------------------------------------------------------------

    #[test]
    fn test_bootstrap_monorepo_layout() {
        // Simulate packages/a/src/..., packages/b/src/..., apps/web/src/...
        let files = vec![
            make_ir_file("/project/packages/auth/src/handler.ts", Vec::new()),
            make_ir_file("/project/packages/auth/src/middleware.ts", Vec::new()),
            make_ir_file("/project/packages/billing/src/invoice.ts", Vec::new()),
            make_ir_file("/project/packages/billing/src/payment.ts", Vec::new()),
            make_ir_file("/project/apps/web/src/index.ts", Vec::new()),
            make_ir_file("/project/apps/web/src/routes.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        assert!(
            !manifest.subsystems.is_empty(),
            "Bootstrap on monorepo must produce non-empty subsystems"
        );
        // Should have at least one subsystem per package/app
        let ids: Vec<&str> = manifest.subsystems.iter().map(|s| s.id.as_str()).collect();
        assert!(
            ids.iter().any(|id| id.contains("auth")),
            "Expected auth subsystem, got: {ids:?}"
        );
        assert!(
            ids.iter().any(|id| id.contains("billing")),
            "Expected billing subsystem, got: {ids:?}"
        );
        assert!(
            ids.iter().any(|id| id.contains("web")),
            "Expected web subsystem, got: {ids:?}"
        );
    }

    #[test]
    fn test_bootstrap_skips_src_intermediary() {
        // packages/auth/src/handler.ts should NOT produce a subsystem named "src"
        let files = vec![
            make_ir_file("/project/packages/auth/src/handler.ts", Vec::new()),
            make_ir_file("/project/packages/auth/src/types.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        for sub in &manifest.subsystems {
            assert_ne!(
                sub.name, "Src",
                "Subsystem name should not be 'Src' — intermediary dirs must be skipped"
            );
            assert_ne!(
                sub.id, "auth-src",
                "Subsystem id should not be 'auth-src' — intermediary dirs must be skipped"
            );
        }
    }

    #[test]
    fn test_bootstrap_fallback_to_domains() {
        // When min_entities is very high, no individual group passes —
        // fallback should create one subsystem per domain.
        let files = vec![
            make_ir_file("/project/packages/auth/src/handler.ts", Vec::new()),
            make_ir_file("/project/packages/billing/src/invoice.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let options = BootstrapOptions {
            project_name: None,
            min_entities: 100, // very high threshold
        };
        let manifest = bootstrap_manifest(&index, &options);

        assert!(
            !manifest.subsystems.is_empty(),
            "Fallback should create at least one subsystem per domain"
        );
    }

    #[test]
    fn test_bootstrap_root_level_files_use_dot_file_path() {
        let files = vec![
            make_ir_file("/project/handler.ts", Vec::new()),
            make_ir_file("/project/repo.ts", Vec::new()),
            make_ir_file("/project/types.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        assert!(
            manifest
                .subsystems
                .iter()
                .all(|sub| sub.file_path == Path::new(".")),
            "Root-level bootstrap subsystems should target '.', got: {:?}",
            manifest
                .subsystems
                .iter()
                .map(|sub| sub.file_path.clone())
                .collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // normalize_path edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_normalize_path_removes_single_dot() {
        let p = PathBuf::from("/project/src/./auth/handler.ts");
        let result = normalize_path(&p);
        assert_eq!(result, PathBuf::from("/project/src/auth/handler.ts"));
    }

    #[test]
    fn test_normalize_path_resolves_parent_dir() {
        let p = PathBuf::from("/project/src/billing/../auth/handler.ts");
        let result = normalize_path(&p);
        assert_eq!(result, PathBuf::from("/project/src/auth/handler.ts"));
    }

    #[test]
    fn test_normalize_path_multiple_parent_dirs() {
        let p = PathBuf::from("/project/src/billing/deep/../../auth/handler.ts");
        let result = normalize_path(&p);
        assert_eq!(result, PathBuf::from("/project/src/auth/handler.ts"));
    }

    #[test]
    fn test_normalize_path_parent_at_root_stays() {
        // When .. goes above root /, it should keep the .. (can't go above root)
        let p = PathBuf::from("/../../etc");
        let result = normalize_path(&p);
        // RootDir cannot be popped, so .. stays
        assert_eq!(result, PathBuf::from("/etc"));
    }

    #[test]
    fn test_normalize_path_relative_with_parent() {
        let p = PathBuf::from("src/billing/../auth/handler.ts");
        let result = normalize_path(&p);
        assert_eq!(result, PathBuf::from("src/auth/handler.ts"));
    }

    #[test]
    fn test_normalize_path_no_ops() {
        let p = PathBuf::from("/project/src/auth/handler.ts");
        let result = normalize_path(&p);
        assert_eq!(result, PathBuf::from("/project/src/auth/handler.ts"));
    }

    #[test]
    fn test_normalize_path_empty() {
        let p = PathBuf::from("");
        let result = normalize_path(&p);
        assert_eq!(result, PathBuf::from(""));
    }

    #[test]
    fn test_normalize_path_only_dots() {
        let p = PathBuf::from("./././.");
        let result = normalize_path(&p);
        assert_eq!(result, PathBuf::from(""));
    }

    // -----------------------------------------------------------------------
    // resolve_import_to_subsystem with @-scoped packages
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_import_scoped_package() {
        // @myapp/auth should resolve to a subsystem containing "auth"
        let subsystems = vec![ManifestSubsystem {
            id: "auth".to_string(),
            name: "Auth".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::New,
            file_path: PathBuf::from("packages/auth/src"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        let result = resolve_import_to_subsystem(
            "@myapp/auth",
            &PathBuf::from("/project/src/billing/invoice.ts"),
            &subsystems,
            &PathBuf::from("/project"),
        );

        assert_eq!(
            result,
            Some("auth".to_string()),
            "@-scoped package imports should resolve to matching subsystem"
        );
    }

    #[test]
    fn test_resolve_import_scoped_package_with_path() {
        // @myapp/auth/handler should also resolve to auth subsystem
        let subsystems = vec![ManifestSubsystem {
            id: "auth".to_string(),
            name: "Auth".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::New,
            file_path: PathBuf::from("packages/auth/src"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        let result = resolve_import_to_subsystem(
            "@myapp/auth/handler",
            &PathBuf::from("/project/src/billing/invoice.ts"),
            &subsystems,
            &PathBuf::from("/project"),
        );

        assert_eq!(
            result,
            Some("auth".to_string()),
            "@-scoped package imports with sub-paths should resolve"
        );
    }

    #[test]
    fn test_resolve_import_crate_reference() {
        // crate::auth::handler should resolve to auth subsystem
        let subsystems = vec![ManifestSubsystem {
            id: "auth".to_string(),
            name: "Auth".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::New,
            file_path: PathBuf::from("src/auth"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        let result = resolve_import_to_subsystem(
            "crate::auth::handler",
            &PathBuf::from("/project/src/billing/invoice.rs"),
            &subsystems,
            &PathBuf::from("/project"),
        );

        assert_eq!(
            result,
            Some("auth".to_string()),
            "Rust crate:: imports should resolve to matching subsystem"
        );
    }

    #[test]
    fn test_resolve_import_no_match() {
        let subsystems = vec![ManifestSubsystem {
            id: "auth".to_string(),
            name: "Auth".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::New,
            file_path: PathBuf::from("src/auth"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        let result = resolve_import_to_subsystem(
            "some-random-package",
            &PathBuf::from("/project/src/billing/invoice.ts"),
            &subsystems,
            &PathBuf::from("/project"),
        );

        assert_eq!(
            result, None,
            "Unrecognized imports should return None"
        );
    }

    #[test]
    fn test_resolve_import_relative_path() {
        let subsystems = vec![ManifestSubsystem {
            id: "auth".to_string(),
            name: "Auth".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::New,
            file_path: PathBuf::from("src/auth"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        let result = resolve_import_to_subsystem(
            "../auth/handler",
            &PathBuf::from("/project/src/billing/invoice.ts"),
            &subsystems,
            &PathBuf::from("/project"),
        );

        assert_eq!(
            result,
            Some("auth".to_string()),
            "Relative imports with .. should resolve to the correct subsystem"
        );
    }

    // -----------------------------------------------------------------------
    // humanize_name edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_humanize_name_empty() {
        assert_eq!(humanize_name(""), "");
    }

    #[test]
    fn test_humanize_name_single_char() {
        assert_eq!(humanize_name("a"), "A");
    }

    #[test]
    fn test_humanize_name_multiple_separators() {
        assert_eq!(humanize_name("my--double--dash"), "My Double Dash");
    }

    #[test]
    fn test_humanize_name_mixed_separators() {
        assert_eq!(humanize_name("my-app_service"), "My App Service");
    }

    #[test]
    fn test_humanize_name_underscore_root() {
        assert_eq!(humanize_name("_root"), "Root");
    }

    // -----------------------------------------------------------------------
    // is_workspace_dir and is_source_root
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_workspace_dir_known_dirs() {
        assert!(is_workspace_dir("crates"));
        assert!(is_workspace_dir("packages"));
        assert!(is_workspace_dir("apps"));
        assert!(is_workspace_dir("modules"));
        assert!(is_workspace_dir("services"));
        assert!(is_workspace_dir("libs"));
        assert!(is_workspace_dir("workspaces"));
    }

    #[test]
    fn test_is_workspace_dir_non_workspace() {
        assert!(!is_workspace_dir("src"));
        assert!(!is_workspace_dir("dist"));
        assert!(!is_workspace_dir("node_modules"));
        assert!(!is_workspace_dir("vendor"));
    }

    #[test]
    fn test_is_source_root_known() {
        assert!(is_source_root("src"));
        assert!(is_source_root("lib"));
        assert!(is_source_root("app"));
    }

    #[test]
    fn test_is_source_root_non_source() {
        assert!(!is_source_root("dist"));
        assert!(!is_source_root("test"));
        assert!(!is_source_root("crates"));
    }

    // -----------------------------------------------------------------------
    // compute_common_prefix edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_compute_common_prefix_empty_files() {
        let result = compute_common_prefix(&[], Path::new("/project"));
        assert_eq!(result, PathBuf::new());
    }

    #[test]
    fn test_compute_common_prefix_single_file() {
        let files = vec![PathBuf::from("/project/src/auth/handler.ts")];
        let result = compute_common_prefix(&files, Path::new("/project"));
        assert_eq!(result, PathBuf::from("src/auth"));
    }

    #[test]
    fn test_compute_common_prefix_same_dir() {
        let files = vec![
            PathBuf::from("/project/src/auth/handler.ts"),
            PathBuf::from("/project/src/auth/types.ts"),
        ];
        let result = compute_common_prefix(&files, Path::new("/project"));
        assert_eq!(result, PathBuf::from("src/auth"));
    }

    #[test]
    fn test_compute_common_prefix_different_dirs() {
        let files = vec![
            PathBuf::from("/project/src/auth/handler.ts"),
            PathBuf::from("/project/src/billing/invoice.ts"),
        ];
        let result = compute_common_prefix(&files, Path::new("/project"));
        assert_eq!(result, PathBuf::from("src"));
    }

    #[test]
    fn test_compute_common_prefix_no_common() {
        let files = vec![
            PathBuf::from("/project/src/handler.ts"),
            PathBuf::from("/project/lib/utils.ts"),
        ];
        let result = compute_common_prefix(&files, Path::new("/project"));
        assert_eq!(result, PathBuf::from("."));
    }

    // -----------------------------------------------------------------------
    // group_files_by_directory edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_group_files_skips_files_outside_root() {
        let files = vec![
            make_ir_file("/other-project/src/auth/handler.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let groups = group_files_by_directory(&index, &PathBuf::from("/project"));

        assert!(
            groups.is_empty(),
            "Files outside scan root should be skipped"
        );
    }

    #[test]
    fn test_group_files_two_component_path() {
        // e.g. src/main.rs -> domain="src", subsystem="src"
        let files = vec![
            make_ir_file("/project/src/main.rs", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let groups = group_files_by_directory(&index, &PathBuf::from("/project"));

        assert!(groups.contains_key("src"), "Should have 'src' domain");
        assert!(
            groups["src"].contains_key("src"),
            "Should have 'src' subsystem"
        );
    }

    // -----------------------------------------------------------------------
    // find_subsystem_for_path edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_subsystem_deepest_match_wins() {
        let subsystems = vec![
            ManifestSubsystem {
                id: "auth".to_string(),
                name: "Auth".to_string(),
                domain: "core".to_string(),
                status: ManifestStatus::New,
                file_path: PathBuf::from("src/auth"),
                interfaces: Vec::new(),
                operations: Vec::new(),
                tables: Vec::new(),
                events: Vec::new(),
                children: Vec::new(),
                dependencies: Vec::new(),
            },
            ManifestSubsystem {
                id: "auth-jwt".to_string(),
                name: "Auth JWT".to_string(),
                domain: "core".to_string(),
                status: ManifestStatus::New,
                file_path: PathBuf::from("src/auth/jwt"),
                interfaces: Vec::new(),
                operations: Vec::new(),
                tables: Vec::new(),
                events: Vec::new(),
                children: Vec::new(),
                dependencies: Vec::new(),
            },
        ];

        let result = find_subsystem_for_path(
            Path::new("src/auth/jwt/token.ts"),
            &subsystems,
        );
        assert_eq!(
            result,
            Some("auth-jwt".to_string()),
            "Deepest path match should win"
        );
    }

    #[test]
    fn test_find_subsystem_no_match() {
        let subsystems = vec![ManifestSubsystem {
            id: "auth".to_string(),
            name: "Auth".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::New,
            file_path: PathBuf::from("src/auth"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        let result = find_subsystem_for_path(
            Path::new("src/billing/invoice.ts"),
            &subsystems,
        );
        assert_eq!(result, None, "Non-matching path should return None");
    }

    #[test]
    fn test_find_subsystem_empty_file_path_skipped() {
        let subsystems = vec![ManifestSubsystem {
            id: "root".to_string(),
            name: "Root".to_string(),
            domain: "_root".to_string(),
            status: ManifestStatus::New,
            file_path: PathBuf::new(),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        let result = find_subsystem_for_path(
            Path::new("handler.ts"),
            &subsystems,
        );
        assert_eq!(
            result, None,
            "Subsystems with empty file_path should be skipped"
        );
    }

    // -----------------------------------------------------------------------
    // Color cycling
    // -----------------------------------------------------------------------

    #[test]
    fn test_domain_color_cycling() {
        // Create more domains than DOMAIN_COLORS (12 colors) to verify cycling
        let mut files = Vec::new();
        for i in 0..15 {
            files.push(make_ir_file(
                &format!("/project/src/domain{i:02}/handler.ts"),
                Vec::new(),
            ));
        }
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        // Should have 15 domains
        assert_eq!(manifest.domains.len(), 15);

        // All colors should come from the palette
        let valid_colors: std::collections::HashSet<&str> =
            DOMAIN_COLORS.iter().copied().collect();
        for domain_def in manifest.domains.values() {
            assert!(
                valid_colors.contains(domain_def.color.as_str()),
                "Color {} should be from the palette",
                domain_def.color
            );
        }

        // With 15 domains and 12 colors, at least some colors must repeat
        let used_colors: Vec<&str> = manifest
            .domains
            .values()
            .map(|d| d.color.as_str())
            .collect();
        let unique_colors: std::collections::HashSet<&&str> = used_colors.iter().collect();
        assert!(
            unique_colors.len() <= DOMAIN_COLORS.len(),
            "Colors should cycle: {unique_colors:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Connection inference edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_self_connections() {
        // An import within the same subsystem should NOT create a connection
        let files = vec![
            make_ir_file(
                "/project/src/auth/handler.ts",
                vec![make_import("./types")],
            ),
            make_ir_file("/project/src/auth/types.ts", Vec::new()),
        ];
        let index = make_scan_index("/project", files);
        let manifest = bootstrap_manifest(&index, &BootstrapOptions::default());

        for conn in &manifest.connections {
            assert_ne!(
                conn.from, conn.to,
                "Self-connections should never be created: {} -> {}",
                conn.from, conn.to
            );
        }
    }

    // -----------------------------------------------------------------------
    // make_unique_id edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_make_unique_id_many_collisions() {
        let mut seen = HashSet::new();
        assert_eq!(make_unique_id("x", &mut seen), "x");
        assert_eq!(make_unique_id("x", &mut seen), "x-2");
        assert_eq!(make_unique_id("x", &mut seen), "x-3");
        assert_eq!(make_unique_id("x", &mut seen), "x-4");
        assert_eq!(make_unique_id("x", &mut seen), "x-5");
        assert_eq!(seen.len(), 5);
    }

    #[test]
    fn test_make_unique_id_different_bases_no_collision() {
        let mut seen = HashSet::new();
        assert_eq!(make_unique_id("a", &mut seen), "a");
        assert_eq!(make_unique_id("b", &mut seen), "b");
        assert_eq!(make_unique_id("c", &mut seen), "c");
        assert_eq!(seen.len(), 3);
    }
}
