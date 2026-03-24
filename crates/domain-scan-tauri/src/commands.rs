use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use domain_scan_core::ir::{
    BuildStatus, Entity, EntityKind, EntitySummary, FilterParams, MatchResult, ScanConfig,
    ScanIndex, ScanStats,
};
use domain_scan_core::manifest::{
    Connection, DomainDef, ManifestMeta, ManifestSubsystem, SystemManifest,
};
use domain_scan_core::prompt::PromptConfig;
use domain_scan_core::{cache, index, manifest, parser, prompt, query_engine, walker};
use serde::{Deserialize, Serialize};
use tauri::State;

// ---------------------------------------------------------------------------
// Application State
// ---------------------------------------------------------------------------

pub struct AppState {
    pub current_index: Mutex<Option<ScanIndex>>,
    pub current_root: Mutex<Option<PathBuf>>,
    pub current_manifest: Mutex<Option<SystemManifest>>,
    pub current_match_result: Mutex<Option<MatchResult>>,
}

// ---------------------------------------------------------------------------
// Tube Map Data Types (IPC DTOs)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TubeMapData {
    pub meta: ManifestMeta,
    pub domains: HashMap<String, DomainDef>,
    pub subsystems: Vec<TubeMapSubsystem>,
    pub connections: Vec<Connection>,
    pub coverage_percent: f64,
    pub unmatched_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TubeMapSubsystem {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub status: String,
    pub description: String,
    pub file_path: String,
    pub matched_entity_count: usize,
    pub interface_count: usize,
    pub operation_count: usize,
    pub table_count: usize,
    pub event_count: usize,
    pub has_children: bool,
    pub child_count: usize,
    pub dependency_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubsystemDetail {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub status: String,
    pub file_path: String,
    pub interfaces: Vec<String>,
    pub operations: Vec<String>,
    pub tables: Vec<String>,
    pub events: Vec<String>,
    pub dependencies: Vec<String>,
    pub children: Vec<SubsystemDetail>,
    pub matched_entities: Vec<EntitySummary>,
}

// ---------------------------------------------------------------------------
// Error Type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Scan error: {0}")]
    Scan(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("No index loaded. Call scan_directory first.")]
    NoIndexLoaded,
    #[error("Entity not found: {0}")]
    EntityNotFound(String),
    #[error("Export error: {0}")]
    Export(String),
}

impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<domain_scan_core::DomainScanError> for CommandError {
    fn from(e: domain_scan_core::DomainScanError) -> Self {
        Self::Scan(e.to_string())
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Helper: run scan pipeline (mirrors CLI's run_scan)
// ---------------------------------------------------------------------------

fn run_scan_pipeline(root: PathBuf) -> Result<ScanIndex, CommandError> {
    let config = ScanConfig::new(root);
    let start = std::time::Instant::now();

    let walked = walker::walk_directory(&config).map_err(|e| CommandError::Scan(e.to_string()))?;

    if walked.is_empty() {
        return Ok(index::build_index(config.root, Vec::new(), 0, 0, 0));
    }

    let disk_cache = if config.cache_enabled {
        let c = cache::Cache::new(config.cache_dir.clone(), 100);
        let _ = c.load_from_disk();
        Some(c)
    } else {
        None
    };

    let mut ir_files = Vec::new();
    let mut cache_hits: usize = 0;
    let mut cache_misses: usize = 0;

    for walked_file in &walked {
        let build_status = config
            .build_status_override
            .unwrap_or(BuildStatus::Built);

        let source_bytes = std::fs::read(&walked_file.path)?;
        let hash = domain_scan_core::content_hash(&source_bytes);

        if let Some(ref c) = disk_cache {
            if let Some(cached_ir) = c.get(&hash) {
                ir_files.push(cached_ir);
                cache_hits += 1;
                continue;
            }
        }

        cache_misses += 1;

        let (tree, source) =
            parser::parse_file(&walked_file.path, walked_file.language)
                .map_err(|e| CommandError::Scan(e.to_string()))?;
        let ir = query_engine::extract(
            &tree,
            &source,
            &walked_file.path,
            walked_file.language,
            build_status,
        )
        .map_err(|e| CommandError::Scan(e.to_string()))?;

        if let Some(ref c) = disk_cache {
            let _ = c.insert(hash, ir.clone());
        }

        ir_files.push(ir);
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(index::build_index(
        config.root,
        ir_files,
        duration_ms,
        cache_hits,
        cache_misses,
    ))
}

// ---------------------------------------------------------------------------
// IPC Commands
// ---------------------------------------------------------------------------

/// Scan a directory, populate AppState. Returns stats only (not the full index).
#[tauri::command]
pub async fn scan_directory(
    root: String,
    state: State<'_, AppState>,
) -> Result<ScanStats, CommandError> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(CommandError::Scan(format!(
            "Not a directory: {root}"
        )));
    }

    let scan_index = run_scan_pipeline(root_path.clone())?;
    let stats = scan_index.stats.clone();

    let mut idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *idx_lock = Some(scan_index);

    let mut root_lock = state
        .current_root
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *root_lock = Some(root_path);

    Ok(stats)
}

/// Check if a scan is loaded (for startup / empty state detection).
#[tauri::command]
pub fn get_current_scan(
    state: State<'_, AppState>,
) -> Result<Option<ScanStats>, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    Ok(idx_lock.as_ref().map(|idx| idx.stats.clone()))
}

/// Filter entities from the loaded index. Reads from AppState.
#[tauri::command]
pub fn filter_entities(
    filters: FilterParams,
    state: State<'_, AppState>,
) -> Result<Vec<EntitySummary>, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;
    Ok(idx.get_entity_summaries(&filters))
}

/// Get full details for a specific entity.
#[tauri::command]
pub fn get_entity_detail(
    name: String,
    file: String,
    state: State<'_, AppState>,
) -> Result<Entity, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;

    let file_path = Path::new(&file);

    // Search through files to find the entity by name and file path
    for ir_file in &idx.files {
        if ir_file.path != file_path {
            continue;
        }
        // Check interfaces
        for iface in &ir_file.interfaces {
            if iface.name == name {
                return Ok(Entity::Interface(iface.clone()));
            }
        }
        // Check services
        for svc in &ir_file.services {
            if svc.name == name {
                return Ok(Entity::Service(svc.clone()));
            }
        }
        // Check classes
        for cls in &ir_file.classes {
            if cls.name == name {
                return Ok(Entity::Class(cls.clone()));
            }
        }
        // Check functions
        for func in &ir_file.functions {
            if func.name == name {
                return Ok(Entity::Function(func.clone()));
            }
        }
        // Check schemas
        for schema in &ir_file.schemas {
            if schema.name == name {
                return Ok(Entity::Schema(schema.clone()));
            }
        }
        // Check implementations
        for imp in &ir_file.implementations {
            if imp.target == name {
                return Ok(Entity::Impl(imp.clone()));
            }
        }
        // Check type aliases
        for alias in &ir_file.type_aliases {
            if alias.name == name {
                return Ok(Entity::TypeAlias(alias.clone()));
            }
        }
    }

    Err(CommandError::EntityNotFound(format!(
        "{name} in {file}"
    )))
}

/// Get source code for a specific span.
#[tauri::command]
pub fn get_entity_source(
    file: String,
    start_byte: usize,
    end_byte: usize,
) -> Result<String, CommandError> {
    let source = std::fs::read_to_string(&file)?;
    let bytes = source.as_bytes();
    if end_byte > bytes.len() || start_byte > end_byte {
        return Err(CommandError::Scan(format!(
            "Invalid byte range {start_byte}..{end_byte} for file of {} bytes",
            bytes.len()
        )));
    }
    String::from_utf8(bytes[start_byte..end_byte].to_vec())
        .map_err(|e| CommandError::Scan(e.to_string()))
}

/// Search entities by name (fuzzy).
#[tauri::command]
pub fn search_entities(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<EntitySummary>, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;
    Ok(idx.search(&query))
}

/// Generate LLM sub-agent prompt scoped to selected entities.
#[tauri::command]
pub fn generate_prompt(
    entity_ids: Vec<String>,
    agents: u8,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;

    let focus = if entity_ids.is_empty() {
        None
    } else {
        // Join entity names into a regex alternation pattern
        Some(entity_ids.join("|"))
    };

    let config = PromptConfig {
        agents: usize::from(agents),
        focus,
        include_scan: false,
    };

    prompt::generate_prompt(idx, &config).map_err(|e| CommandError::Scan(e.to_string()))
}

/// Export current view as JSON, CSV, or Markdown.
#[tauri::command]
pub fn export_entities(
    format: String,
    filters: FilterParams,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;

    let summaries = idx.get_entity_summaries(&filters);

    match format.as_str() {
        "json" => serde_json::to_string_pretty(&summaries)
            .map_err(|e| CommandError::Export(e.to_string())),
        "csv" => {
            let mut csv = String::from("name,kind,file,line,language,build_status,confidence\n");
            for s in &summaries {
                use std::fmt::Write;
                let _ = writeln!(
                    csv,
                    "{},{:?},{},{},{:?},{:?},{:?}",
                    s.name,
                    s.kind,
                    s.file.display(),
                    s.line,
                    s.language,
                    s.build_status,
                    s.confidence,
                );
            }
            Ok(csv)
        }
        "markdown" => {
            let mut md =
                String::from("| Name | Kind | File | Line | Language | Build Status | Confidence |\n");
            md.push_str("|------|------|------|------|----------|--------------|------------|\n");
            for s in &summaries {
                use std::fmt::Write;
                let _ = writeln!(
                    md,
                    "| {} | {:?} | {} | {} | {:?} | {:?} | {:?} |",
                    s.name,
                    s.kind,
                    s.file.display(),
                    s.line,
                    s.language,
                    s.build_status,
                    s.confidence,
                );
            }
            Ok(md)
        }
        _ => Err(CommandError::Export(format!(
            "Unknown format: {format}. Use json, csv, or markdown."
        ))),
    }
}

/// Get build status for all modules.
#[tauri::command]
pub fn get_build_status(
    state: State<'_, AppState>,
) -> Result<HashMap<String, BuildStatus>, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;

    let mut result = HashMap::new();
    for file in &idx.files {
        result.insert(file.path.display().to_string(), file.build_status);
    }
    Ok(result)
}

/// Open a file in the user's editor.
#[tauri::command]
pub fn open_in_editor(
    editor: String,
    file: String,
    line: usize,
) -> Result<(), CommandError> {
    let args: Vec<String> = match editor.as_str() {
        "code" | "vscode" => vec![
            "--goto".to_string(),
            format!("{file}:{line}"),
        ],
        "cursor" => vec![
            "--goto".to_string(),
            format!("{file}:{line}"),
        ],
        "zed" => vec![
            format!("{file}:{line}"),
        ],
        _ => {
            return Err(CommandError::Scan(format!(
                "Unsupported editor: {editor}. Use code, cursor, or zed."
            )));
        }
    };

    let cmd = match editor.as_str() {
        "code" | "vscode" => "code",
        "cursor" => "cursor",
        "zed" => "zed",
        other => other,
    };

    std::process::Command::new(cmd)
        .args(&args)
        .spawn()
        .map_err(|e| CommandError::Io(format!("Failed to open {editor}: {e}")))?;

    Ok(())
}

/// Check which editors are available on this system.
#[tauri::command]
pub fn check_editors_available() -> HashMap<String, bool> {
    let editors = ["code", "cursor", "zed"];
    let mut result = HashMap::new();
    for editor in &editors {
        let available = std::process::Command::new("which")
            .arg(editor)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        result.insert((*editor).to_string(), available);
    }
    result
}

// ---------------------------------------------------------------------------
// Tube Map IPC Commands
// ---------------------------------------------------------------------------

/// Load a system manifest from a file path, storing it in AppState.
#[tauri::command]
pub fn load_manifest(
    path: String,
    state: State<'_, AppState>,
) -> Result<SystemManifest, CommandError> {
    let manifest_path = Path::new(&path);
    if !manifest_path.is_file() {
        return Err(CommandError::Io(format!("Not a file: {path}")));
    }

    let sys_manifest = manifest::parse_system_manifest_file(manifest_path)?;

    let mut manifest_lock = state
        .current_manifest
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *manifest_lock = Some(sys_manifest.clone());

    Ok(sys_manifest)
}

/// Run matching: maps scanned entities to manifest subsystems.
/// Requires both a scan index and a manifest to be loaded.
#[tauri::command]
pub fn match_manifest(
    state: State<'_, AppState>,
) -> Result<MatchResult, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;

    let manifest_lock = state
        .current_manifest
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let sys_manifest = manifest_lock
        .as_ref()
        .ok_or_else(|| CommandError::Scan("No manifest loaded. Call load_manifest first.".to_string()))?;

    let simple_manifest = sys_manifest.as_manifest();
    let result = manifest::match_entities(idx, &simple_manifest);

    // Store match result
    drop(manifest_lock);
    drop(idx_lock);
    let mut match_lock = state
        .current_match_result
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *match_lock = Some(result.clone());

    Ok(result)
}

/// Get composite tube map data: subsystems with match counts, domains, connections.
#[tauri::command]
pub fn get_tube_map_data(
    state: State<'_, AppState>,
) -> Result<TubeMapData, CommandError> {
    let manifest_lock = state
        .current_manifest
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let sys_manifest = manifest_lock
        .as_ref()
        .ok_or_else(|| CommandError::Scan("No manifest loaded.".to_string()))?;

    let match_lock = state
        .current_match_result
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;

    let subsystems = build_tube_map_subsystems(&sys_manifest.subsystems, match_lock.as_ref());

    let (coverage_percent, unmatched_count) = match match_lock.as_ref() {
        Some(mr) => (mr.coverage_percent, mr.unmatched.len()),
        None => (0.0, 0),
    };

    Ok(TubeMapData {
        meta: sys_manifest.meta.clone(),
        domains: sys_manifest.domains.clone(),
        subsystems,
        connections: sys_manifest.connections.clone(),
        coverage_percent,
        unmatched_count,
    })
}

/// Get entities matched to a specific subsystem.
#[tauri::command]
pub fn get_subsystem_entities(
    subsystem_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<EntitySummary>, CommandError> {
    let match_lock = state
        .current_match_result
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let match_result = match_lock
        .as_ref()
        .ok_or_else(|| CommandError::Scan("No match result. Call match_manifest first.".to_string()))?;

    let entities: Vec<EntitySummary> = match_result
        .matched
        .iter()
        .filter(|m| m.subsystem_id == subsystem_id)
        .map(|m| m.entity.clone())
        .collect();

    Ok(entities)
}

/// Get full detail for a subsystem: metadata + matched entities.
#[tauri::command]
pub fn get_subsystem_detail(
    subsystem_id: String,
    state: State<'_, AppState>,
) -> Result<SubsystemDetail, CommandError> {
    let manifest_lock = state
        .current_manifest
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let sys_manifest = manifest_lock
        .as_ref()
        .ok_or_else(|| CommandError::Scan("No manifest loaded.".to_string()))?;

    let match_lock = state
        .current_match_result
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;

    let subsystem = find_subsystem(&sys_manifest.subsystems, &subsystem_id)
        .ok_or_else(|| CommandError::EntityNotFound(format!("Subsystem not found: {subsystem_id}")))?;

    let matched_entities: Vec<EntitySummary> = match match_lock.as_ref() {
        Some(mr) => mr
            .matched
            .iter()
            .filter(|m| m.subsystem_id == subsystem_id)
            .map(|m| m.entity.clone())
            .collect(),
        None => Vec::new(),
    };

    Ok(build_subsystem_detail(subsystem, &matched_entities, match_lock.as_ref()))
}

// ---------------------------------------------------------------------------
// Helpers for tube map commands
// ---------------------------------------------------------------------------

fn build_tube_map_subsystems(
    subsystems: &[ManifestSubsystem],
    match_result: Option<&MatchResult>,
) -> Vec<TubeMapSubsystem> {
    let mut result = Vec::new();
    collect_tube_map_subsystems(subsystems, match_result, &mut result);
    result
}

fn collect_tube_map_subsystems(
    subsystems: &[ManifestSubsystem],
    match_result: Option<&MatchResult>,
    out: &mut Vec<TubeMapSubsystem>,
) {
    for sub in subsystems {
        let matched_entity_count = match match_result {
            Some(mr) => mr.matched.iter().filter(|m| m.subsystem_id == sub.id).count(),
            None => 0,
        };

        let (interface_count, operation_count, table_count, event_count) = match match_result {
            Some(mr) => {
                let matched_for_sub: Vec<_> = mr.matched.iter().filter(|m| m.subsystem_id == sub.id).collect();
                let iface = matched_for_sub.iter().filter(|m| m.entity.kind == EntityKind::Interface).count();
                let ops = matched_for_sub.iter().filter(|m| m.entity.kind == EntityKind::Method || m.entity.kind == EntityKind::Function).count();
                let tables = matched_for_sub.iter().filter(|m| m.entity.kind == EntityKind::Schema).count();
                let events = 0usize; // No event entity kind yet
                (iface, ops, tables, events)
            }
            None => (0, 0, 0, 0),
        };

        out.push(TubeMapSubsystem {
            id: sub.id.clone(),
            name: sub.name.clone(),
            domain: sub.domain.clone(),
            status: format!("{:?}", sub.status).to_lowercase(),
            description: String::new(),
            file_path: sub.file_path.display().to_string(),
            matched_entity_count,
            interface_count,
            operation_count,
            table_count,
            event_count,
            has_children: !sub.children.is_empty(),
            child_count: sub.children.len(),
            dependency_count: sub.dependencies.len(),
        });

        collect_tube_map_subsystems(&sub.children, match_result, out);
    }
}

fn find_subsystem<'a>(
    subsystems: &'a [ManifestSubsystem],
    id: &str,
) -> Option<&'a ManifestSubsystem> {
    for sub in subsystems {
        if sub.id == id {
            return Some(sub);
        }
        if let Some(found) = find_subsystem(&sub.children, id) {
            return Some(found);
        }
    }
    None
}

fn build_subsystem_detail(
    sub: &ManifestSubsystem,
    matched_entities: &[EntitySummary],
    match_result: Option<&MatchResult>,
) -> SubsystemDetail {
    SubsystemDetail {
        id: sub.id.clone(),
        name: sub.name.clone(),
        domain: sub.domain.clone(),
        status: format!("{:?}", sub.status).to_lowercase(),
        file_path: sub.file_path.display().to_string(),
        interfaces: sub.interfaces.clone(),
        operations: sub.operations.clone(),
        tables: sub.tables.clone(),
        events: sub.events.clone(),
        dependencies: sub.dependencies.clone(),
        children: sub
            .children
            .iter()
            .map(|child| {
                let child_entities: Vec<EntitySummary> = match match_result {
                    Some(mr) => mr
                        .matched
                        .iter()
                        .filter(|m| m.subsystem_id == child.id)
                        .map(|m| m.entity.clone())
                        .collect(),
                    None => Vec::new(),
                };
                build_subsystem_detail(child, &child_entities, match_result)
            })
            .collect(),
        matched_entities: matched_entities.to_vec(),
    }
}
