//! LLM sub-agent prompt generation.
//!
//! Generates structured prompts that instruct an LLM orchestrator to launch
//! N sub-agents, each responsible for scanning a partition of the codebase.
//!
//! Partitioning strategy is chosen automatically based on file count:
//! - < 500 files: by concern (5 agent categories)
//! - 500-2000 files: hybrid (concern + directory)
//! - > 2000 files: by directory with concern sub-partitions

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use crate::ir::{BuildStatus, IrFile, ScanIndex, ScanStats};
use crate::DomainScanError;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Configuration for prompt generation.
#[derive(Debug, Clone)]
pub struct PromptConfig {
    /// Number of sub-agents to generate assignments for.
    pub agents: usize,
    /// Optional entity name regex to scope prompt to matching files only.
    pub focus: Option<String>,
    /// Embed full scan JSON in the prompt.
    pub include_scan: bool,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            agents: 5,
            focus: None,
            include_scan: false,
        }
    }
}

/// Generate the full LLM prompt from a scan index.
pub fn generate_prompt(
    index: &ScanIndex,
    config: &PromptConfig,
) -> Result<String, DomainScanError> {
    // Apply focus filter if set
    let files = if let Some(ref pattern) = config.focus {
        filter_files_by_focus(&index.files, pattern)
    } else {
        index.files.iter().collect()
    };

    let strategy = select_strategy(files.len(), config.agents);
    let assignments = build_assignments(&files, &strategy, config.agents, &index.root);
    let project_name = project_name_from_root(&index.root);

    let mut out = String::new();

    write_header(&mut out, &project_name, &index.root, &index.stats);
    write_task_section(&mut out, config.agents);
    write_assignments(&mut out, &assignments);
    write_synthesis_section(&mut out);

    if config.include_scan {
        write_scan_embed(&mut out, index)?;
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Partitioning strategy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartitionStrategy {
    /// < 500 files: each agent gets a structural concern category.
    ByConcern,
    /// 500-2000 files: combine concern + directory splitting.
    Hybrid,
    /// > 2000 files: primary split by directory, sub-partition by concern.
    ByDirectory,
}

fn select_strategy(file_count: usize, _agents: usize) -> PartitionStrategy {
    if file_count < 500 {
        PartitionStrategy::ByConcern
    } else if file_count <= 2000 {
        PartitionStrategy::Hybrid
    } else {
        PartitionStrategy::ByDirectory
    }
}

// ---------------------------------------------------------------------------
// Agent assignment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct AgentAssignment {
    id: usize,
    title: String,
    scope: String,
    directory_focus: String,
    instructions: Vec<String>,
    built_files: Vec<PathBuf>,
    non_built_files: Vec<PathBuf>,
}

/// Standard concern-based agent roles.
const CONCERN_AGENTS: &[(&str, &str)] = &[
    (
        "Interface & Type Boundary Audit",
        "All interface, trait, and protocol definitions",
    ),
    (
        "Service Architecture Map",
        "All service definitions (HTTP controllers, gRPC, workers, etc.)",
    ),
    (
        "Method Signature Census",
        "All public methods across classes, structs, impls",
    ),
    (
        "Cross-Cutting Concerns",
        "Decorators, middleware, annotations, generic constraints",
    ),
    (
        "Implementation Completeness Audit",
        "All impl blocks, class implementations, protocol conformances",
    ),
];

/// Instructions per concern-based agent role.
const CONCERN_INSTRUCTIONS: &[&[&str]] = &[
    // Agent 1: Interface & Type Boundary Audit
    &[
        "Read every interface/trait/protocol definition in your assigned files",
        "For each, document: name, methods, extends chain, which types implement it",
        "Flag any interface with >10 methods (possible god-interface)",
        "Flag any interface with 0 implementors (dead interface)",
        "Flag any partial implementations (missing methods)",
    ],
    // Agent 2: Service Architecture Map
    &[
        "Read every service definition in your assigned files",
        "Document: name, kind, routes/methods, injected dependencies",
        "Map the dependency graph between services",
        "Flag any service with >15 methods (possible god-service)",
        "Flag circular dependencies between services",
    ],
    // Agent 3: Method Signature Census
    &[
        "Catalog all public methods with their full signatures",
        "Group by owner (class/struct/trait)",
        "Flag inconsistent naming patterns (mixedCase vs snake_case in same module)",
        "Flag methods with >5 parameters (possible refactor target)",
        "Identify async/sync boundary crossings",
    ],
    // Agent 4: Cross-Cutting Concerns
    &[
        "Catalog all decorator/annotation usage patterns",
        "Identify middleware chains and their ordering",
        "Map generic type parameter constraints",
        "Flag unused or redundant decorators",
        "Document the authentication/authorization boundary",
    ],
    // Agent 5: Implementation Completeness Audit
    &[
        "For every interface/trait, verify all implementations are complete",
        "Document which methods have default implementations vs required",
        "Flag orphaned implementations (impl for trait that doesn't exist)",
        "Map the inheritance/composition hierarchy",
        "Identify diamond inheritance or conflicting implementations",
    ],
];

fn build_assignments(
    files: &[&IrFile],
    strategy: &PartitionStrategy,
    agents: usize,
    root: &Path,
) -> Vec<AgentAssignment> {
    match strategy {
        PartitionStrategy::ByConcern => build_concern_assignments(files, agents, root),
        PartitionStrategy::Hybrid => build_hybrid_assignments(files, agents, root),
        PartitionStrategy::ByDirectory => build_directory_assignments(files, agents, root),
    }
}

/// Partition files by structural concern (< 500 files).
fn build_concern_assignments(
    files: &[&IrFile],
    agents: usize,
    root: &Path,
) -> Vec<AgentAssignment> {
    let agent_count = agents.min(CONCERN_AGENTS.len());

    // Categorize files by what they contain
    let mut interface_files = Vec::new();
    let mut service_files = Vec::new();
    let mut method_files = Vec::new();
    let mut decorator_files = Vec::new();
    let mut impl_files = Vec::new();

    for file in files {
        if !file.interfaces.is_empty() {
            interface_files.push(*file);
        }
        if !file.services.is_empty() {
            service_files.push(*file);
        }
        if !file.classes.is_empty() || !file.functions.is_empty() {
            method_files.push(*file);
        }
        // Files with classes that have decorators, or services with decorators
        if file.classes.iter().any(|c| !c.decorators.is_empty())
            || file.services.iter().any(|s| !s.decorators.is_empty())
            || file.interfaces.iter().any(|i| !i.decorators.is_empty())
        {
            decorator_files.push(*file);
        }
        if !file.implementations.is_empty() {
            impl_files.push(*file);
        }
    }

    let file_sets: Vec<Vec<&IrFile>> = vec![
        interface_files,
        service_files,
        method_files,
        decorator_files,
        impl_files,
    ];

    let mut assignments = Vec::new();
    for i in 0..agent_count {
        let concern_files = &file_sets[i];
        let (built, non_built) = partition_by_build_status(concern_files);
        let dirs = collect_directories(concern_files, root);

        assignments.push(AgentAssignment {
            id: i + 1,
            title: CONCERN_AGENTS[i].0.to_string(),
            scope: CONCERN_AGENTS[i].1.to_string(),
            directory_focus: dirs,
            instructions: CONCERN_INSTRUCTIONS[i].iter().map(|s| (*s).to_string()).collect(),
            built_files: built,
            non_built_files: non_built,
        });
    }

    assignments
}

/// Hybrid partitioning: concern + directory splitting (500-2000 files).
fn build_hybrid_assignments(
    files: &[&IrFile],
    agents: usize,
    root: &Path,
) -> Vec<AgentAssignment> {
    // Group files by top-level directory first
    let dir_groups = group_by_top_level_dir(files, root);
    let dir_keys: Vec<&String> = dir_groups.keys().collect();

    let agent_count = agents.min(CONCERN_AGENTS.len());
    let mut assignments = Vec::new();

    // Distribute directories across agents, keeping concern roles
    for i in 0..agent_count {
        // Each agent gets a concern + a subset of directories
        let start = (dir_keys.len() * i) / agent_count;
        let end = (dir_keys.len() * (i + 1)) / agent_count;

        let mut agent_files: Vec<&IrFile> = Vec::new();
        for dir_key in &dir_keys[start..end] {
            if let Some(dir_files) = dir_groups.get(*dir_key) {
                agent_files.extend(dir_files.iter());
            }
        }

        // If no files in assigned dirs, fall back to concern-based filtering
        if agent_files.is_empty() {
            agent_files = filter_by_concern(files, i);
        }

        let (built, non_built) = partition_by_build_status(&agent_files);
        let dirs = collect_directories(&agent_files, root);

        assignments.push(AgentAssignment {
            id: i + 1,
            title: CONCERN_AGENTS[i].0.to_string(),
            scope: format!(
                "{} (directories: {})",
                CONCERN_AGENTS[i].1,
                &dir_keys[start..end]
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            directory_focus: dirs,
            instructions: CONCERN_INSTRUCTIONS[i].iter().map(|s| (*s).to_string()).collect(),
            built_files: built,
            non_built_files: non_built,
        });
    }

    assignments
}

/// Directory-based partitioning with concern sub-partitions (> 2000 files).
fn build_directory_assignments(
    files: &[&IrFile],
    agents: usize,
    root: &Path,
) -> Vec<AgentAssignment> {
    let dir_groups = group_by_top_level_dir(files, root);
    let mut dir_entries: Vec<(String, Vec<&IrFile>)> = dir_groups.into_iter().collect();
    // Sort by file count descending for balanced distribution
    dir_entries.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let agent_count = agents.max(1);
    let mut assignments: Vec<AgentAssignment> = (0..agent_count)
        .map(|i| AgentAssignment {
            id: i + 1,
            title: format!("Directory Group {}", i + 1),
            scope: String::new(),
            directory_focus: String::new(),
            instructions: vec![
                "Catalog all interfaces, traits, protocols, services, and classes".to_string(),
                "Document method signatures and their owners".to_string(),
                "Flag god-objects (>10 methods on interface, >15 on service)".to_string(),
                "Map implementation completeness for each trait/interface".to_string(),
                "Identify cross-cutting patterns (decorators, middleware, annotations)".to_string(),
            ],
            built_files: Vec::new(),
            non_built_files: Vec::new(),
        })
        .collect();

    // Round-robin assign directory groups to agents
    let mut dir_names_per_agent: Vec<Vec<String>> = vec![Vec::new(); agent_count];
    for (idx, (dir_name, dir_files)) in dir_entries.into_iter().enumerate() {
        let agent_idx = idx % agent_count;
        let (built, non_built) = partition_by_build_status(&dir_files);
        assignments[agent_idx].built_files.extend(built);
        assignments[agent_idx].non_built_files.extend(non_built);
        dir_names_per_agent[agent_idx].push(dir_name);
    }

    // Update scope and directory focus
    for (i, assignment) in assignments.iter_mut().enumerate() {
        let dirs = &dir_names_per_agent[i];
        assignment.scope = format!("All entities in: {}", dirs.join(", "));
        assignment.directory_focus = dirs.join(", ");
        assignment.title = format!(
            "Directory Group {} ({})",
            i + 1,
            dirs.join(", ")
        );
    }

    assignments
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Filter files by entity name matching focus pattern.
fn filter_files_by_focus<'a>(files: &'a [IrFile], pattern: &str) -> Vec<&'a IrFile> {
    files
        .iter()
        .filter(|f| file_has_matching_entity(f, pattern))
        .collect()
}

fn file_has_matching_entity(file: &IrFile, pattern: &str) -> bool {
    let pat = pattern.to_lowercase();
    file.interfaces.iter().any(|i| i.name.to_lowercase().contains(&pat))
        || file.services.iter().any(|s| s.name.to_lowercase().contains(&pat))
        || file.classes.iter().any(|c| c.name.to_lowercase().contains(&pat))
        || file.functions.iter().any(|f| f.name.to_lowercase().contains(&pat))
        || file.schemas.iter().any(|s| s.name.to_lowercase().contains(&pat))
        || file.implementations.iter().any(|i| i.target.to_lowercase().contains(&pat))
        || file.type_aliases.iter().any(|t| t.name.to_lowercase().contains(&pat))
}

/// Split files into (built paths, non-built paths).
fn partition_by_build_status(files: &[&IrFile]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut built = Vec::new();
    let mut non_built = Vec::new();
    for f in files {
        match f.build_status {
            BuildStatus::Built => built.push(f.path.clone()),
            _ => non_built.push(f.path.clone()),
        }
    }
    built.sort();
    non_built.sort();
    (built, non_built)
}

/// Collect unique directory paths relative to root.
fn collect_directories(files: &[&IrFile], root: &Path) -> String {
    let dirs: BTreeSet<String> = files
        .iter()
        .filter_map(|f| {
            f.path.parent().and_then(|p| {
                p.strip_prefix(root)
                    .ok()
                    .map(|rel| rel.to_string_lossy().to_string())
            })
        })
        .filter(|d| !d.is_empty())
        .collect();
    dirs.into_iter().collect::<Vec<_>>().join(", ")
}

/// Group files by their top-level directory relative to root.
fn group_by_top_level_dir<'a>(
    files: &[&'a IrFile],
    root: &Path,
) -> BTreeMap<String, Vec<&'a IrFile>> {
    let mut groups: BTreeMap<String, Vec<&'a IrFile>> = BTreeMap::new();
    for file in files {
        let dir = file
            .path
            .strip_prefix(root)
            .ok()
            .and_then(|rel| rel.components().next())
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        groups.entry(dir).or_default().push(file);
    }
    groups
}

/// Filter files by concern category index.
fn filter_by_concern<'a>(files: &[&'a IrFile], concern_idx: usize) -> Vec<&'a IrFile> {
    files
        .iter()
        .copied()
        .filter(|f| match concern_idx {
            0 => !f.interfaces.is_empty(),
            1 => !f.services.is_empty(),
            2 => !f.classes.is_empty() || !f.functions.is_empty(),
            3 => {
                f.classes.iter().any(|c| !c.decorators.is_empty())
                    || f.services.iter().any(|s| !s.decorators.is_empty())
            }
            4 => !f.implementations.is_empty(),
            _ => true,
        })
        .collect()
}

/// Extract project name from root path.
fn project_name_from_root(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}

// ---------------------------------------------------------------------------
// Prompt rendering
// ---------------------------------------------------------------------------

fn write_header(out: &mut String, project_name: &str, root: &Path, stats: &ScanStats) {
    let _ = writeln!(out, "# Codebase Structural Analysis: {project_name}");
    let _ = writeln!(out);
    let _ = writeln!(out, "## Context");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "You are analyzing the codebase at `{}`.",
        root.display()
    );
    let _ = writeln!(
        out,
        "A structural scan has identified the following high-level statistics:"
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "- **Total files:** {}", stats.total_files);
    let _ = writeln!(out, "- **Interfaces:** {}", stats.total_interfaces);
    let _ = writeln!(out, "- **Services:** {}", stats.total_services);
    let _ = writeln!(out, "- **Classes:** {}", stats.total_classes);
    let _ = writeln!(out, "- **Functions:** {}", stats.total_functions);
    let _ = writeln!(out, "- **Methods:** {}", stats.total_methods);
    let _ = writeln!(out, "- **Schemas:** {}", stats.total_schemas);
    let _ = writeln!(out, "- **Type aliases:** {}", stats.total_type_aliases);
    let _ = writeln!(
        out,
        "- **Implementations:** {}",
        stats.total_implementations
    );

    if !stats.files_by_language.is_empty() {
        let _ = writeln!(out, "- **Languages:** {}", format_language_stats(stats));
    }
    let _ = writeln!(out);
}

fn format_language_stats(stats: &ScanStats) -> String {
    let mut lang_counts: Vec<(String, usize)> = stats
        .files_by_language
        .iter()
        .map(|(lang, count)| (lang.to_string(), *count))
        .collect();
    lang_counts.sort_by(|a, b| b.1.cmp(&a.1));
    lang_counts
        .iter()
        .map(|(lang, count)| format!("{lang} ({count})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_task_section(out: &mut String, agents: usize) {
    let _ = writeln!(out, "## Your Task");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Launch {agents} sub-agents to perform a deep structural analysis of this codebase."
    );
    let _ = writeln!(
        out,
        "Each sub-agent should scan its assigned partition and report back with findings."
    );
    let _ = writeln!(out);
}

fn write_assignments(out: &mut String, assignments: &[AgentAssignment]) {
    let _ = writeln!(out, "## Sub-Agent Assignments");
    let _ = writeln!(out);

    for assignment in assignments {
        let _ = writeln!(
            out,
            "### Agent {}: {}",
            assignment.id, assignment.title
        );
        let _ = writeln!(out, "**Scope:** {}", assignment.scope);
        if !assignment.directory_focus.is_empty() {
            let _ = writeln!(out, "**Directory focus:** {}", assignment.directory_focus);
        }
        let _ = writeln!(out, "**Instructions:**");
        for (i, instruction) in assignment.instructions.iter().enumerate() {
            let _ = writeln!(out, "{}. {instruction}", i + 1);
        }

        // Build-status-aware file lists
        if !assignment.built_files.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "**Files to scan (built — structural output is authoritative):**");
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "> These files are from modules that compile successfully. The domain-scan structural output is authoritative. Verify the scan results are complete, catalog any patterns the static analysis missed (e.g. runtime registration, reflection-based DI), and document the architecture."
            );
            let _ = writeln!(out);
            let _ = writeln!(out, "```");
            for path in &assignment.built_files {
                let _ = writeln!(out, "{}", path.display());
            }
            let _ = writeln!(out, "```");
        }

        if !assignment.non_built_files.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "**Files to scan (unbuilt/error/rebuild — best-effort extraction):**");
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "> These files are from modules that do not currently build. The domain-scan output is a best-effort extraction. Read each file carefully. Infer the intended interfaces and services from naming patterns, comments, and partial definitions. Flag conflicts between old and new definitions. Mark your findings with confidence levels."
            );
            let _ = writeln!(out);
            let _ = writeln!(out, "```");
            for path in &assignment.non_built_files {
                let _ = writeln!(out, "{}", path.display());
            }
            let _ = writeln!(out, "```");
        }

        if assignment.built_files.is_empty() && assignment.non_built_files.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "**Files to scan:** (no files matched this category)");
        }

        let _ = writeln!(out);
    }
}

fn write_synthesis_section(out: &mut String) {
    let _ = writeln!(out, "## Synthesis");
    let _ = writeln!(out);
    let _ = writeln!(out, "After all agents complete, synthesize findings into:");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "1. **Architecture Map**: Top-level service → interface → implementation hierarchy"
    );
    let _ = writeln!(
        out,
        "2. **Health Report**: God objects, dead interfaces, incomplete impls, circular deps"
    );
    let _ = writeln!(
        out,
        "3. **API Surface**: Complete public API with method signatures"
    );
    let _ = writeln!(
        out,
        "4. **Recommendations**: Specific refactoring suggestions with file:line references"
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## Output Format");
    let _ = writeln!(out);
    let _ = writeln!(out, "Each agent should return structured JSON:");
    let _ = writeln!(out, "```json");
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "  \"agent_id\": 1,");
    let _ = writeln!(
        out,
        "  \"scope\": \"Interface & Type Boundary Audit\","
    );
    let _ = writeln!(out, "  \"findings\": [...],");
    let _ = writeln!(out, "  \"flags\": [...],");
    let _ = writeln!(out, "  \"file_count\": 42,");
    let _ = writeln!(out, "  \"entity_count\": 156");
    let _ = writeln!(out, "}}");
    let _ = writeln!(out, "```");
}

fn write_scan_embed(out: &mut String, index: &ScanIndex) -> Result<(), DomainScanError> {
    let _ = writeln!(out);
    let _ = writeln!(out, "## Full Scan Results");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "The following is the complete structural scan output. Use this as your starting map before reading source files."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "```json");
    let json = serde_json::to_string_pretty(index)?;
    let _ = writeln!(out, "{json}");
    let _ = writeln!(out, "```");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BuildStatus, Language};
    use std::collections::HashMap;

    fn make_ir_file(path: &str, lang: Language, status: BuildStatus) -> IrFile {
        IrFile::new(
            PathBuf::from(path),
            lang,
            format!("hash_{path}"),
            status,
        )
    }

    fn make_interface(name: &str, file: &str) -> crate::ir::InterfaceDef {
        crate::ir::InterfaceDef {
            name: name.to_string(),
            file: PathBuf::from(file),
            span: crate::ir::Span::default(),
            visibility: crate::ir::Visibility::Public,
            generics: Vec::new(),
            extends: Vec::new(),
            methods: Vec::new(),
            properties: Vec::new(),
            language_kind: crate::ir::InterfaceKind::Interface,
            decorators: Vec::new(),
        }
    }

    fn make_service(name: &str, file: &str) -> crate::ir::ServiceDef {
        crate::ir::ServiceDef {
            name: name.to_string(),
            file: PathBuf::from(file),
            span: crate::ir::Span::default(),
            kind: crate::ir::ServiceKind::HttpController,
            methods: Vec::new(),
            dependencies: Vec::new(),
            decorators: Vec::new(),
            routes: Vec::new(),
        }
    }

    fn make_test_index(files: Vec<IrFile>) -> ScanIndex {
        crate::index::build_index(PathBuf::from("/project"), files, 100, 0, 0)
    }

    // Make a deterministic index (with fixed timestamp) for snapshot tests.
    fn make_snapshot_index(files: Vec<IrFile>) -> ScanIndex {
        let mut index = make_test_index(files);
        // Fix the timestamp for deterministic snapshots
        index.scanned_at = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap_or_else(|_| chrono::DateTime::default())
            .with_timezone(&chrono::Utc);
        index
    }

    #[test]
    fn test_select_strategy_small() {
        assert_eq!(select_strategy(100, 5), PartitionStrategy::ByConcern);
        assert_eq!(select_strategy(499, 5), PartitionStrategy::ByConcern);
    }

    #[test]
    fn test_select_strategy_medium() {
        assert_eq!(select_strategy(500, 5), PartitionStrategy::Hybrid);
        assert_eq!(select_strategy(2000, 5), PartitionStrategy::Hybrid);
    }

    #[test]
    fn test_select_strategy_large() {
        assert_eq!(select_strategy(2001, 5), PartitionStrategy::ByDirectory);
        assert_eq!(select_strategy(10000, 5), PartitionStrategy::ByDirectory);
    }

    #[test]
    fn test_project_name_from_root() {
        assert_eq!(
            project_name_from_root(Path::new("/home/user/my-project")),
            "my-project"
        );
        assert_eq!(project_name_from_root(Path::new("/")), "project");
    }

    #[test]
    fn test_partition_by_build_status() {
        let built = IrFile::new(
            PathBuf::from("a.ts"),
            Language::TypeScript,
            "h1".into(),
            BuildStatus::Built,
        );
        let unbuilt = IrFile::new(
            PathBuf::from("b.ts"),
            Language::TypeScript,
            "h2".into(),
            BuildStatus::Unbuilt,
        );
        let error = IrFile::new(
            PathBuf::from("c.ts"),
            Language::TypeScript,
            "h3".into(),
            BuildStatus::Error,
        );

        let files: Vec<&IrFile> = vec![&built, &unbuilt, &error];
        let (b, nb) = partition_by_build_status(&files);
        assert_eq!(b.len(), 1);
        assert_eq!(nb.len(), 2);
    }

    #[test]
    fn test_filter_files_by_focus() {
        let mut f1 = make_ir_file("src/auth.ts", Language::TypeScript, BuildStatus::Built);
        f1.interfaces.push(make_interface("AuthService", "src/auth.ts"));

        let f2 = make_ir_file("src/utils.ts", Language::TypeScript, BuildStatus::Built);

        let files = vec![f1, f2];
        let filtered = filter_files_by_focus(&files, "auth");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, PathBuf::from("src/auth.ts"));
    }

    #[test]
    fn test_generate_prompt_empty_index() {
        let index = make_test_index(vec![]);
        let config = PromptConfig::default();
        let result = generate_prompt(&index, &config);
        assert!(result.is_ok());
        let prompt = result.unwrap_or_default();
        assert!(prompt.contains("# Codebase Structural Analysis: project"));
        assert!(prompt.contains("Launch 5 sub-agents"));
        assert!(prompt.contains("## Sub-Agent Assignments"));
        assert!(prompt.contains("## Synthesis"));
    }

    #[test]
    fn test_generate_prompt_with_files() {
        let mut f1 = make_ir_file("/project/src/types.ts", Language::TypeScript, BuildStatus::Built);
        f1.interfaces.push(make_interface("UserRepo", "/project/src/types.ts"));

        let index = make_test_index(vec![f1]);
        let config = PromptConfig::default();
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        assert!(prompt.contains("Agent 1: Interface & Type Boundary Audit"));
        assert!(prompt.contains("structural output is authoritative"));
    }

    #[test]
    fn test_generate_prompt_with_focus() {
        let mut f1 = make_ir_file("/project/src/auth.ts", Language::TypeScript, BuildStatus::Built);
        f1.interfaces.push(make_interface("AuthService", "/project/src/auth.ts"));

        let f2 = make_ir_file("/project/src/utils.ts", Language::TypeScript, BuildStatus::Built);

        let index = make_test_index(vec![f1, f2]);
        let config = PromptConfig {
            agents: 5,
            focus: Some("auth".to_string()),
            include_scan: false,
        };
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        // Should include auth file but not utils
        assert!(prompt.contains("auth.ts"));
    }

    #[test]
    fn test_generate_prompt_with_non_built_files() {
        let mut f1 = make_ir_file("/project/src/auth.ts", Language::TypeScript, BuildStatus::Unbuilt);
        f1.interfaces.push(make_interface("AuthService", "/project/src/auth.ts"));

        let index = make_test_index(vec![f1]);
        let config = PromptConfig::default();
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        assert!(prompt.contains("best-effort extraction"));
        assert!(prompt.contains("do not currently build"));
    }

    #[test]
    fn test_generate_prompt_include_scan() {
        let index = make_test_index(vec![]);
        let config = PromptConfig {
            agents: 3,
            focus: None,
            include_scan: true,
        };
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        assert!(prompt.contains("## Full Scan Results"));
        assert!(prompt.contains("```json"));
    }

    #[test]
    fn test_generate_prompt_agents_count() {
        let index = make_test_index(vec![]);
        let config = PromptConfig {
            agents: 3,
            focus: None,
            include_scan: false,
        };
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        assert!(prompt.contains("Launch 3 sub-agents"));
        // Should only have 3 agents, not 5
        assert!(prompt.contains("Agent 1:"));
        assert!(prompt.contains("Agent 2:"));
        assert!(prompt.contains("Agent 3:"));
        assert!(!prompt.contains("Agent 4:"));
    }

    #[test]
    fn test_format_language_stats() {
        let mut stats = ScanStats::default();
        let mut langs = HashMap::new();
        langs.insert(Language::TypeScript, 50);
        langs.insert(Language::Rust, 30);
        stats.files_by_language = langs;

        let formatted = format_language_stats(&stats);
        assert!(formatted.contains("TypeScript (50)"));
        assert!(formatted.contains("Rust (30)"));
    }

    // -----------------------------------------------------------------
    // Insta snapshot tests
    // -----------------------------------------------------------------

    #[test]
    fn snapshot_prompt_small_codebase() {
        let mut f1 = make_ir_file("/project/src/types.ts", Language::TypeScript, BuildStatus::Built);
        f1.interfaces.push(make_interface("UserRepository", "/project/src/types.ts"));
        f1.interfaces.push(make_interface("PostRepository", "/project/src/types.ts"));

        let mut f2 = make_ir_file("/project/src/services.ts", Language::TypeScript, BuildStatus::Built);
        f2.services.push(make_service("UserController", "/project/src/services.ts"));

        let mut f3 = make_ir_file("/project/src/legacy.ts", Language::TypeScript, BuildStatus::Unbuilt);
        f3.interfaces.push(make_interface("LegacyAuth", "/project/src/legacy.ts"));

        let index = make_snapshot_index(vec![f1, f2, f3]);
        let config = PromptConfig {
            agents: 5,
            focus: None,
            include_scan: false,
        };
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        insta::assert_snapshot!("prompt_small_codebase", prompt);
    }

    #[test]
    fn snapshot_prompt_with_focus() {
        let mut f1 = make_ir_file("/project/src/auth/handler.ts", Language::TypeScript, BuildStatus::Built);
        f1.interfaces.push(make_interface("AuthHandler", "/project/src/auth/handler.ts"));
        f1.services.push(make_service("AuthController", "/project/src/auth/handler.ts"));

        let mut f2 = make_ir_file("/project/src/users/service.ts", Language::TypeScript, BuildStatus::Built);
        f2.services.push(make_service("UserService", "/project/src/users/service.ts"));

        let index = make_snapshot_index(vec![f1, f2]);
        let config = PromptConfig {
            agents: 3,
            focus: Some("auth".to_string()),
            include_scan: false,
        };
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        insta::assert_snapshot!("prompt_with_focus_auth", prompt);
    }

    #[test]
    fn snapshot_prompt_mixed_build_status() {
        let mut f1 = make_ir_file("/project/src/built.ts", Language::TypeScript, BuildStatus::Built);
        f1.interfaces.push(make_interface("BuiltInterface", "/project/src/built.ts"));

        let mut f2 = make_ir_file("/project/src/unbuilt.ts", Language::TypeScript, BuildStatus::Unbuilt);
        f2.interfaces.push(make_interface("UnbuiltInterface", "/project/src/unbuilt.ts"));

        let mut f3 = make_ir_file("/project/src/error.ts", Language::TypeScript, BuildStatus::Error);
        f3.interfaces.push(make_interface("ErrorInterface", "/project/src/error.ts"));

        let mut f4 = make_ir_file("/project/src/rebuild.ts", Language::TypeScript, BuildStatus::Rebuild);
        f4.interfaces.push(make_interface("RebuildInterface", "/project/src/rebuild.ts"));

        let index = make_snapshot_index(vec![f1, f2, f3, f4]);
        let config = PromptConfig {
            agents: 5,
            focus: None,
            include_scan: false,
        };
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        insta::assert_snapshot!("prompt_mixed_build_status", prompt);
    }

    #[test]
    fn snapshot_prompt_three_agents() {
        let mut f1 = make_ir_file("/project/src/types.ts", Language::TypeScript, BuildStatus::Built);
        f1.interfaces.push(make_interface("Repo", "/project/src/types.ts"));

        let index = make_snapshot_index(vec![f1]);
        let config = PromptConfig {
            agents: 3,
            focus: None,
            include_scan: false,
        };
        let prompt = generate_prompt(&index, &config).unwrap_or_default();

        insta::assert_snapshot!("prompt_three_agents", prompt);
    }
}
