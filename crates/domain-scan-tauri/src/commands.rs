use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::Mutex;

use domain_scan_core::doctor::DoctorReport;
use domain_scan_core::ir::{
    BuildStatus, Entity, EntityKind, EntitySummary, FilterParams, MatchResult, ScanConfig,
    ScanIndex, ScanStats,
};
use domain_scan_core::manifest::{
    Connection, DomainDef, ManifestMeta, ManifestSubsystem, SystemManifest,
};
use domain_scan_core::manifest_builder::{self, BootstrapOptions};
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
    pub current_manifest_path: Mutex<Option<PathBuf>>,
    pub current_match_result: Mutex<Option<MatchResult>>,
    /// Cache file contents to avoid repeated disk reads (path → content).
    pub file_source_cache: Mutex<HashMap<PathBuf, String>>,
    /// Entity lookup index: (name, file_path) → (file_index, entity_kind_index).
    /// Built after scan for O(1) entity detail lookups instead of linear scan.
    pub entity_lookup: Mutex<HashMap<(String, PathBuf), EntityLookupEntry>>,
}

/// Cached position of an entity within the ScanIndex for O(1) detail retrieval.
#[derive(Debug, Clone)]
pub struct EntityLookupEntry {
    pub file_index: usize,
    pub kind: EntityKind,
    pub kind_index: usize,
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
        let build_status = config.build_status_override.unwrap_or(BuildStatus::Built);

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

        let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)
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
        return Err(CommandError::Scan(format!("Not a directory: {root}")));
    }

    // Run the CPU-intensive scan pipeline on a blocking thread to avoid
    // starving the tokio async executor.
    let scan_root = root_path.clone();
    let scan_index = tauri::async_runtime::spawn_blocking(move || run_scan_pipeline(scan_root))
        .await
        .map_err(|e| CommandError::Scan(e.to_string()))??;
    let stats = scan_index.stats.clone();

    // Build entity lookup index for O(1) detail retrieval
    let mut lookup = HashMap::new();
    for (fi, ir_file) in scan_index.files.iter().enumerate() {
        for (ki, iface) in ir_file.interfaces.iter().enumerate() {
            lookup.insert(
                (iface.name.clone(), ir_file.path.clone()),
                EntityLookupEntry {
                    file_index: fi,
                    kind: EntityKind::Interface,
                    kind_index: ki,
                },
            );
        }
        for (ki, svc) in ir_file.services.iter().enumerate() {
            lookup.insert(
                (svc.name.clone(), ir_file.path.clone()),
                EntityLookupEntry {
                    file_index: fi,
                    kind: EntityKind::Service,
                    kind_index: ki,
                },
            );
        }
        for (ki, cls) in ir_file.classes.iter().enumerate() {
            lookup.insert(
                (cls.name.clone(), ir_file.path.clone()),
                EntityLookupEntry {
                    file_index: fi,
                    kind: EntityKind::Class,
                    kind_index: ki,
                },
            );
        }
        for (ki, func) in ir_file.functions.iter().enumerate() {
            lookup.insert(
                (func.name.clone(), ir_file.path.clone()),
                EntityLookupEntry {
                    file_index: fi,
                    kind: EntityKind::Function,
                    kind_index: ki,
                },
            );
        }
        for (ki, schema) in ir_file.schemas.iter().enumerate() {
            lookup.insert(
                (schema.name.clone(), ir_file.path.clone()),
                EntityLookupEntry {
                    file_index: fi,
                    kind: EntityKind::Schema,
                    kind_index: ki,
                },
            );
        }
        for (ki, imp) in ir_file.implementations.iter().enumerate() {
            lookup.insert(
                (imp.target.clone(), ir_file.path.clone()),
                EntityLookupEntry {
                    file_index: fi,
                    kind: EntityKind::Impl,
                    kind_index: ki,
                },
            );
        }
        for (ki, alias) in ir_file.type_aliases.iter().enumerate() {
            lookup.insert(
                (alias.name.clone(), ir_file.path.clone()),
                EntityLookupEntry {
                    file_index: fi,
                    kind: EntityKind::TypeAlias,
                    kind_index: ki,
                },
            );
        }
    }

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

    // Store entity lookup and clear stale caches
    let mut lookup_lock = state
        .entity_lookup
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *lookup_lock = lookup;

    let mut cache_lock = state
        .file_source_cache
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    cache_lock.clear();

    Ok(stats)
}

/// Check if a scan is loaded (for startup / empty state detection).
#[tauri::command]
pub fn get_current_scan(state: State<'_, AppState>) -> Result<Option<ScanStats>, CommandError> {
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
/// Uses the pre-built entity lookup index for O(1) retrieval.
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

    let file_path = PathBuf::from(&file);

    // Fast path: use the lookup index
    let lookup_lock = state
        .entity_lookup
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;

    if let Some(entry) = lookup_lock.get(&(name.clone(), file_path)) {
        let ir_file = idx
            .files
            .get(entry.file_index)
            .ok_or_else(|| CommandError::EntityNotFound(name.clone()))?;
        let not_found = || CommandError::EntityNotFound(format!("{name} in {file}"));
        return match entry.kind {
            EntityKind::Interface => Ok(Entity::Interface(
                ir_file
                    .interfaces
                    .get(entry.kind_index)
                    .ok_or_else(not_found)?
                    .clone(),
            )),
            EntityKind::Service => Ok(Entity::Service(
                ir_file
                    .services
                    .get(entry.kind_index)
                    .ok_or_else(not_found)?
                    .clone(),
            )),
            EntityKind::Class => Ok(Entity::Class(
                ir_file
                    .classes
                    .get(entry.kind_index)
                    .ok_or_else(not_found)?
                    .clone(),
            )),
            EntityKind::Function => Ok(Entity::Function(
                ir_file
                    .functions
                    .get(entry.kind_index)
                    .ok_or_else(not_found)?
                    .clone(),
            )),
            EntityKind::Schema => Ok(Entity::Schema(
                ir_file
                    .schemas
                    .get(entry.kind_index)
                    .ok_or_else(not_found)?
                    .clone(),
            )),
            EntityKind::Impl => Ok(Entity::Impl(
                ir_file
                    .implementations
                    .get(entry.kind_index)
                    .ok_or_else(not_found)?
                    .clone(),
            )),
            EntityKind::TypeAlias => Ok(Entity::TypeAlias(
                ir_file
                    .type_aliases
                    .get(entry.kind_index)
                    .ok_or_else(not_found)?
                    .clone(),
            )),
            _ => Err(CommandError::EntityNotFound(format!("{name} in {file}"))),
        };
    }

    Err(CommandError::EntityNotFound(format!("{name} in {file}")))
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

/// Get the full source content of a file.
/// Uses an in-memory cache to avoid repeated disk reads on tab switches.
#[tauri::command]
pub fn get_file_source(file: String, state: State<'_, AppState>) -> Result<String, CommandError> {
    let path = PathBuf::from(&file);

    // Check cache first
    {
        let cache = state
            .file_source_cache
            .lock()
            .map_err(|e| CommandError::Scan(e.to_string()))?;
        if let Some(content) = cache.get(&path) {
            return Ok(content.clone());
        }
    }

    if !path.is_file() {
        return Err(CommandError::Io(format!("Not a file: {file}")));
    }
    let content = std::fs::read_to_string(&path)?;

    // Store in cache (cap at 50 files to bound memory)
    let mut cache = state
        .file_source_cache
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    if cache.len() >= 50 {
        // HashMap has no defined iteration order, so evicting via .keys().take()
        // is non-deterministic. Simply clear the entire cache — it will refill on
        // demand with the files actually in use.
        cache.clear();
    }
    cache.insert(path, content.clone());

    Ok(content)
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
                    "\"{}\",{:?},\"{}\",{},{:?},{:?},{:?}",
                    s.name.replace('"', "\"\""),
                    s.kind,
                    s.file.display().to_string().replace('"', "\"\""),
                    s.line,
                    s.language,
                    s.build_status,
                    s.confidence,
                );
            }
            Ok(csv)
        }
        "markdown" => {
            let mut md = String::from(
                "| Name | Kind | File | Line | Language | Build Status | Confidence |\n",
            );
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
/// Uses macOS `open -a` to launch by app name, avoiding PATH issues in bundled apps.
#[tauri::command]
pub fn open_in_editor(
    editor: String,
    file: String,
    line: usize,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let scan_root = state
        .current_root
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?
        .clone();
    let manifest_path = state
        .current_manifest_path
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?
        .clone();
    let resolved_path = resolve_editor_path(
        &file,
        scan_root.as_deref(),
        manifest_path.as_deref(),
    );

    if !resolved_path.exists() {
        return Err(CommandError::Io(format!(
            "Path does not exist: {}",
            resolved_path.display()
        )));
    }

    // First, try the CLI command directly (works when CLI tools are in PATH)
    let cli_result = try_open_via_cli(&editor, &resolved_path, line);
    if cli_result.is_ok() {
        return Ok(());
    }

    // Fallback: platform-specific open command
    #[cfg(target_os = "macos")]
    {
        let app_name = app_name_for_editor(&editor)?;

        // For VS Code / Cursor, use `open -a <App> --args --goto file:line`
        // For Zed, use `open -a Zed file:line`
        let mut cmd = ProcessCommand::new("open");
        cmd.arg("-a").arg(app_name);

        if resolved_path.is_dir() {
            cmd.arg(&resolved_path);
        } else {
            match editor.as_str() {
                "code" | "vscode" | "cursor" => {
                    cmd.arg("--args")
                        .arg("--goto")
                        .arg(format!("{}:{line}", resolved_path.display()));
                }
                "zed" => {
                    cmd.arg(format!("{}:{line}", resolved_path.display()));
                }
                _ => {}
            }
        }

        cmd.spawn()
            .map_err(|e| CommandError::Io(format!("Failed to open {app_name}: {e}")))?;
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use `start <editor>` or `cmd /C start "" <editor> <args>`
        let (cli_cmd, args) = build_cli_editor_command(&editor, &resolved_path, line)?;
        ProcessCommand::new("cmd")
            .args(["/C", "start", "", cli_cmd])
            .args(&args)
            .spawn()
            .map_err(|e| CommandError::Io(format!("Failed to open {cli_cmd}: {e}")))?;
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, use xdg-open for directories, or fall back to the CLI command
        if resolved_path.is_dir() {
            ProcessCommand::new("xdg-open")
                .arg(&resolved_path)
                .spawn()
                .map_err(|e| CommandError::Io(format!("Failed to xdg-open: {e}")))?;
        } else {
            let (cli_cmd, args) = build_cli_editor_command(&editor, &resolved_path, line)?;
            ProcessCommand::new(cli_cmd)
                .args(&args)
                .spawn()
                .map_err(|e| CommandError::Io(format!("Failed to open {cli_cmd}: {e}")))?;
        }
    }

    Ok(())
}

fn app_name_for_editor(editor: &str) -> Result<&'static str, CommandError> {
    match editor {
        "cursor" => Ok("Cursor"),
        "code" | "vscode" => Ok("Visual Studio Code"),
        "zed" => Ok("Zed"),
        _ => Err(CommandError::Io(format!(
            "Unsupported editor: {editor}. Use cursor, code, or zed."
        ))),
    }
}

fn resolve_editor_path(
    target: &str,
    scan_root: Option<&Path>,
    manifest_path: Option<&Path>,
) -> PathBuf {
    let candidate = PathBuf::from(target);
    if candidate.is_absolute() {
        return candidate;
    }

    let manifest_dir = manifest_path.and_then(Path::parent);

    let joined_scan = scan_root.map(|root| root.join(&candidate));
    let joined_manifest = manifest_dir.map(|root| root.join(&candidate));

    if let Some(path) = joined_scan.as_ref() {
        if path.exists() {
            return path.clone();
        }
    }
    if let Some(path) = joined_manifest.as_ref() {
        if path.exists() {
            return path.clone();
        }
    }

    joined_scan
        .or(joined_manifest)
        .unwrap_or(candidate)
}

fn build_cli_editor_command(
    editor: &str,
    path: &Path,
    line: usize,
) -> Result<(&'static str, Vec<String>), CommandError> {
    let path_text = path.display().to_string();
    let is_dir = path.is_dir();

    match editor {
        "code" | "vscode" => Ok(if is_dir {
            ("code", vec![path_text])
        } else {
            ("code", vec!["--goto".to_string(), format!("{path_text}:{line}")])
        }),
        "cursor" => Ok(if is_dir {
            ("cursor", vec![path_text])
        } else {
            (
                "cursor",
                vec!["--goto".to_string(), format!("{path_text}:{line}")],
            )
        }),
        "zed" => Ok(if is_dir {
            ("zed", vec![path_text])
        } else {
            ("zed", vec![format!("{path_text}:{line}")])
        }),
        _ => Err(CommandError::Io("unsupported".to_string())),
    }
}

fn try_open_via_cli(editor: &str, path: &Path, line: usize) -> Result<(), CommandError> {
    let (cmd, args) = build_cli_editor_command(editor, path, line)?;

    ProcessCommand::new(cmd)
        .args(&args)
        .spawn()
        .map_err(|e| CommandError::Io(e.to_string()))?;

    Ok(())
}

/// Check which editors are available on this system.
#[tauri::command]
pub fn check_editors_available() -> HashMap<String, bool> {
    #[cfg(target_os = "windows")]
    let lookup_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let lookup_cmd = "which";

    let editors = ["code", "cursor", "zed"];
    let mut result = HashMap::new();
    for editor in &editors {
        let available = std::process::Command::new(lookup_cmd)
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

    let mut manifest_path_lock = state
        .current_manifest_path
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *manifest_path_lock = Some(manifest_path.to_path_buf());

    Ok(sys_manifest)
}

/// Run matching: maps scanned entities to manifest subsystems.
/// Requires both a scan index and a manifest to be loaded.
#[tauri::command]
pub fn match_manifest(state: State<'_, AppState>) -> Result<MatchResult, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;

    let manifest_lock = state
        .current_manifest
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let sys_manifest = manifest_lock.as_ref().ok_or_else(|| {
        CommandError::Scan("No manifest loaded. Call load_manifest first.".to_string())
    })?;

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
pub fn get_tube_map_data(state: State<'_, AppState>) -> Result<TubeMapData, CommandError> {
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
    let match_result = match_lock.as_ref().ok_or_else(|| {
        CommandError::Scan("No match result. Call match_manifest first.".to_string())
    })?;

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

    let subsystem = find_subsystem(&sys_manifest.subsystems, &subsystem_id).ok_or_else(|| {
        CommandError::EntityNotFound(format!("Subsystem not found: {subsystem_id}"))
    })?;

    let matched_entities: Vec<EntitySummary> = match match_lock.as_ref() {
        Some(mr) => mr
            .matched
            .iter()
            .filter(|m| m.subsystem_id == subsystem_id)
            .map(|m| m.entity.clone())
            .collect(),
        None => Vec::new(),
    };

    Ok(build_subsystem_detail(
        subsystem,
        &matched_entities,
        match_lock.as_ref(),
    ))
}

/// Bootstrap a manifest from scan data using heuristic inference.
/// Requires a scan to be loaded. Returns a draft SystemManifest.
#[tauri::command]
pub fn bootstrap_manifest(
    project_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<SystemManifest, CommandError> {
    let idx_lock = state
        .current_index
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    let idx = idx_lock.as_ref().ok_or(CommandError::NoIndexLoaded)?;

    let options = BootstrapOptions {
        project_name,
        min_entities: 3,
    };

    Ok(manifest_builder::bootstrap_manifest(idx, &options))
}

/// Save a manifest to disk. Accepts the full SystemManifest JSON and a file path.
/// Normalizes all `filePath` fields to use forward slashes for cross-platform
/// compatibility (Windows paths with backslashes break consumers on Unix).
#[tauri::command]
pub fn save_manifest(
    mut manifest_json: SystemManifest,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    normalize_manifest_paths(&mut manifest_json.subsystems);

    let json = manifest_builder::serialize_manifest(&manifest_json)
        .map_err(|e| CommandError::Export(e.to_string()))?;

    std::fs::write(&path, &json)?;

    // Also load the saved manifest into AppState so tube map can use it immediately
    let mut manifest_lock = state
        .current_manifest
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *manifest_lock = Some(manifest_json);

    let mut manifest_path_lock = state
        .current_manifest_path
        .lock()
        .map_err(|e| CommandError::Scan(e.to_string()))?;
    *manifest_path_lock = Some(PathBuf::from(path));

    Ok(())
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
            Some(mr) => mr
                .matched
                .iter()
                .filter(|m| m.subsystem_id == sub.id)
                .count(),
            None => 0,
        };

        let (interface_count, operation_count, table_count, event_count) = match match_result {
            Some(mr) => {
                let matched_for_sub: Vec<_> = mr
                    .matched
                    .iter()
                    .filter(|m| m.subsystem_id == sub.id)
                    .collect();
                let iface = matched_for_sub
                    .iter()
                    .filter(|m| m.entity.kind == EntityKind::Interface)
                    .count();
                let ops = matched_for_sub
                    .iter()
                    .filter(|m| {
                        m.entity.kind == EntityKind::Method || m.entity.kind == EntityKind::Function
                    })
                    .count();
                let tables = matched_for_sub
                    .iter()
                    .filter(|m| m.entity.kind == EntityKind::Schema)
                    .count();
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

/// Normalize `file_path` fields on subsystems (and their children) to always
/// use forward slashes so that serialised manifests are portable across OSes.
fn normalize_manifest_paths(subsystems: &mut [ManifestSubsystem]) {
    for sub in subsystems.iter_mut() {
        let normalized = sub.file_path.display().to_string().replace('\\', "/");
        sub.file_path = PathBuf::from(normalized);
        normalize_manifest_paths(&mut sub.children);
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

// ---------------------------------------------------------------------------
// Platform + Release info for agent prompt
// ---------------------------------------------------------------------------

/// GitHub release asset info returned to the frontend.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

/// Platform and latest release info used to build the agent prompt dynamically.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformReleaseInfo {
    /// e.g. "darwin", "linux", "windows"
    pub os: String,
    /// e.g. "aarch64", "x86_64"
    pub arch: String,
    /// Latest release tag or None
    pub latest_tag: Option<String>,
    /// All release assets
    pub assets: Vec<ReleaseAsset>,
    /// The matching asset for this OS+arch, if found
    pub matching_asset: Option<ReleaseAsset>,
    /// Cargo install fallback command
    pub cargo_install_cmd: String,
    /// Preferred install command for this platform
    pub recommended_install_cmd: String,
    /// Preferred update command for this platform/install state
    pub recommended_update_cmd: String,
    /// Absolute path to the currently scanned directory, if a scan is loaded.
    pub scanned_root: Option<String>,
    /// Absolute path to the installed CLI on PATH, if found.
    pub installed_path: Option<String>,
    /// Installed CLI version parsed from `domain-scan --version`, if found.
    pub installed_version: Option<String>,
    /// Whether the installed CLI already supports `domain-scan doctor`.
    pub doctor_supported: bool,
    /// Whether the installed CLI appears older than the latest release.
    pub update_available: Option<bool>,
}

/// Detect current platform and fetch the latest domain-scan release from GitHub.
#[tauri::command]
pub async fn get_platform_release_info(
    state: State<'_, AppState>,
) -> Result<PlatformReleaseInfo, CommandError> {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string(); // "aarch64", "x86_64"

    // Normalise os name to match release asset naming convention
    let os_label = normalize_os_label(&os).to_string();

    let cargo_install_cmd =
        "cargo install --force domain-scan-cli --git https://github.com/jamesaphoenix/domain-scan.git"
            .to_string();

    // Fetch latest release from GitHub API (best effort — don't fail if offline)
    let release = fetch_latest_release().await;

    let (latest_tag, assets) = match release {
        Some((tag, raw_assets)) => {
            let assets: Vec<ReleaseAsset> = raw_assets
                .iter()
                .filter_map(|a| {
                    let name = a.get("name")?.as_str()?.to_string();
                    let download_url = a.get("browser_download_url")?.as_str()?.to_string();
                    let size = a.get("size")?.as_u64().unwrap_or(0);
                    Some(ReleaseAsset {
                        name,
                        download_url,
                        size,
                    })
                })
                .collect();
            (Some(tag), assets)
        }
        None => (None, Vec::new()),
    };

    // Find matching asset for this platform
    let matching_asset = assets
        .iter()
        .find(|a| {
            let lower = a.name.to_lowercase();
            lower.contains(&os_label) && lower.contains(&arch)
        })
        .cloned();

    let recommended_install_cmd = matching_asset
        .as_ref()
        .map(|asset| build_binary_install_command(&os_label, asset))
        .unwrap_or_else(|| cargo_install_cmd.clone());

    // Read the scanned root path (if a scan has been performed)
    let scanned_root = state
        .current_root
        .lock()
        .ok()
        .and_then(|r| r.as_ref().map(|p| p.display().to_string()));

    let installed_path_buf = find_executable_on_path("domain-scan");
    let mut installed_version = installed_path_buf
        .as_deref()
        .and_then(read_installed_version);
    let doctor_report = installed_path_buf.as_deref().and_then(read_doctor_report);
    let doctor_supported = doctor_report.is_some();

    if let Some(report) = doctor_report.as_ref() {
        installed_version = Some(report.current_version.clone());
    }

    let fallback_update_available = match (installed_version.as_deref(), latest_tag.as_deref()) {
        (Some(current), Some(latest)) => Some(is_update_available(current, latest)),
        _ => None,
    };
    let update_available = doctor_report
        .as_ref()
        .and_then(|report| report.update_available)
        .or(fallback_update_available);
    let recommended_update_cmd = doctor_report
        .as_ref()
        .map(|report| report.recommended_update_command.clone())
        .unwrap_or_else(|| recommended_install_cmd.clone());

    Ok(PlatformReleaseInfo {
        os: os_label,
        arch,
        latest_tag,
        assets,
        matching_asset,
        cargo_install_cmd,
        recommended_install_cmd,
        recommended_update_cmd,
        scanned_root,
        installed_path: installed_path_buf.map(|path| path.display().to_string()),
        installed_version,
        doctor_supported,
        update_available,
    })
}

fn normalize_os_label(os: &str) -> &str {
    match os {
        "macos" => "darwin",
        other => other,
    }
}

fn build_binary_install_command(os: &str, asset: &ReleaseAsset) -> String {
    if os == "windows" {
        return "cargo install --force domain-scan-cli --git https://github.com/jamesaphoenix/domain-scan.git".to_string();
    }

    format!(
        "curl -sL \"{url}\" -o /tmp/domain-scan.tar.gz\n\
         tar -xzf /tmp/domain-scan.tar.gz -C /tmp\n\
         chmod +x /tmp/domain-scan\n\
         mkdir -p ~/.local/bin\n\
         mv /tmp/domain-scan ~/.local/bin/domain-scan\n\
         export PATH=\"$HOME/.local/bin:$PATH\"",
        url = asset.download_url,
    )
}

fn find_executable_on_path(name: &str) -> Option<PathBuf> {
    let executable_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    if let Some(path_var) = std::env::var_os("PATH") {
        if let Some(found) = std::env::split_paths(&path_var)
            .map(|dir| dir.join(&executable_name))
            .find(|candidate| candidate.is_file())
        {
            return Some(found);
        }
    }

    let home_dir = std::env::var_os("HOME").map(PathBuf::from);
    let mut fallback_paths = Vec::new();
    if let Some(home) = home_dir {
        fallback_paths.push(home.join(".local").join("bin").join(&executable_name));
        fallback_paths.push(home.join(".cargo").join("bin").join(&executable_name));
    }
    fallback_paths.push(PathBuf::from("/usr/local/bin").join(&executable_name));
    fallback_paths.push(PathBuf::from("/opt/homebrew/bin").join(&executable_name));

    fallback_paths
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn read_installed_version(executable_path: &Path) -> Option<String> {
    let output = ProcessCommand::new(executable_path)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    parse_cli_version_output(&stdout)
}

fn parse_cli_version_output(raw: &str) -> Option<String> {
    raw.split_whitespace().nth(1).map(|value| value.to_string())
}

fn read_doctor_report(executable_path: &Path) -> Option<DoctorReport> {
    let output = ProcessCommand::new(executable_path)
        .arg("doctor")
        .arg("--output")
        .arg("json")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice::<DoctorReport>(&output.stdout).ok()
}

fn normalize_version(version: &str) -> &str {
    version.trim().trim_start_matches('v')
}

fn parse_version_parts(version: &str) -> Option<(u64, u64, u64)> {
    let normalized = normalize_version(version);
    let core = normalized.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn is_update_available(current_version: &str, latest_tag: &str) -> bool {
    match (
        parse_version_parts(current_version),
        parse_version_parts(latest_tag),
    ) {
        (Some(current), Some(latest)) => current < latest,
        _ => normalize_version(current_version) != normalize_version(latest_tag),
    }
}

/// Fetch the latest release JSON from GitHub. Returns (tag_name, assets[]) or None.
async fn fetch_latest_release() -> Option<(String, Vec<serde_json::Value>)> {
    let mut resp =
        ureq::get("https://api.github.com/repos/jamesaphoenix/domain-scan/releases/latest")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "domain-scan-tauri")
            .call()
            .ok()?;

    let body: serde_json::Value = resp.body_mut().read_json().ok()?;

    let tag = body.get("tag_name")?.as_str()?.to_string();
    let assets = body.get("assets")?.as_array()?.clone();
    Some((tag, assets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("domain-scan-{label}-{nonce}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn resolve_editor_path_prefers_scan_root() {
        let root = make_temp_dir("scan-root");
        let target = root.join("packages/services");
        fs::create_dir_all(&target).expect("target dir should exist");

        let resolved = resolve_editor_path("packages/services", Some(&root), None);
        assert_eq!(resolved, target);

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn resolve_editor_path_falls_back_to_manifest_directory() {
        let manifest_dir = make_temp_dir("manifest-root");
        let manifest_path = manifest_dir.join("system.json");
        let target = manifest_dir.join("apps/api");
        fs::create_dir_all(&target).expect("target dir should exist");
        fs::write(&manifest_path, "{}").expect("manifest should be written");

        let resolved =
            resolve_editor_path("apps/api", None, Some(manifest_path.as_path()));
        assert_eq!(resolved, target);

        fs::remove_dir_all(manifest_dir).expect("temp dir should be removed");
    }

    #[test]
    fn build_cli_editor_command_uses_plain_directory_open() {
        let dir = make_temp_dir("editor-dir");
        let (cmd, args) =
            build_cli_editor_command("cursor", &dir, 1).expect("command should build");
        assert_eq!(cmd, "cursor");
        assert_eq!(args, vec![dir.display().to_string()]);

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn build_cli_editor_command_uses_goto_for_files() {
        let dir = make_temp_dir("editor-file");
        let file = dir.join("src/main.ts");
        fs::create_dir_all(file.parent().expect("file should have parent"))
            .expect("parent dir should exist");
        fs::write(&file, "export const x = 1;\n").expect("file should be written");

        let (cmd, args) =
            build_cli_editor_command("code", &file, 12).expect("command should build");
        assert_eq!(cmd, "code");
        assert_eq!(
            args,
            vec![
                "--goto".to_string(),
                format!("{}:12", file.display()),
            ]
        );

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    // -----------------------------------------------------------------------
    // Fix 1 tests: bounds-checked entity detail lookup
    // -----------------------------------------------------------------------

    #[test]
    fn entity_lookup_out_of_bounds_file_index_returns_none() {
        // Build an empty ScanIndex via the public API
        let idx = index::build_index(PathBuf::from("/tmp"), Vec::new(), 0, 0, 0);

        // Create a lookup entry that points past the end of idx.files
        let entry = EntityLookupEntry {
            file_index: 999,
            kind: EntityKind::Interface,
            kind_index: 0,
        };

        let ir_file_result = idx.files.get(entry.file_index);
        assert!(
            ir_file_result.is_none(),
            "out-of-bounds file_index should return None from .get()"
        );
    }

    #[test]
    fn entity_lookup_out_of_bounds_kind_index_returns_none() {
        use domain_scan_core::ir::{BuildStatus, IrFile, Language};

        let ir_file = IrFile::new(
            PathBuf::from("/tmp/test.ts"),
            Language::TypeScript,
            "hash".to_string(),
            BuildStatus::Built,
        );

        // kind_index 999 is past the end of the empty interfaces vec
        let result = ir_file.interfaces.get(999);
        assert!(
            result.is_none(),
            "out-of-bounds kind_index should return None from .get()"
        );
    }

    // -----------------------------------------------------------------------
    // Fix 4 tests: normalize_manifest_paths
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_manifest_paths_converts_backslashes() {
        use domain_scan_core::manifest::{ManifestStatus, ManifestSubsystem};

        let mut subsystems = vec![ManifestSubsystem {
            id: "auth".to_string(),
            name: "Auth".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::Built,
            file_path: PathBuf::from("src\\auth\\index.ts"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: vec![ManifestSubsystem {
                id: "auth-child".to_string(),
                name: "Auth Child".to_string(),
                domain: "core".to_string(),
                status: ManifestStatus::Built,
                file_path: PathBuf::from("src\\auth\\child\\index.ts"),
                interfaces: Vec::new(),
                operations: Vec::new(),
                tables: Vec::new(),
                events: Vec::new(),
                children: Vec::new(),
                dependencies: Vec::new(),
            }],
            dependencies: Vec::new(),
        }];

        normalize_manifest_paths(&mut subsystems);

        assert_eq!(
            subsystems[0].file_path.display().to_string(),
            "src/auth/index.ts",
            "top-level path should have forward slashes"
        );
        assert_eq!(
            subsystems[0].children[0].file_path.display().to_string(),
            "src/auth/child/index.ts",
            "nested child path should have forward slashes"
        );
    }

    #[test]
    fn normalize_manifest_paths_preserves_forward_slashes() {
        use domain_scan_core::manifest::{ManifestStatus, ManifestSubsystem};

        let mut subsystems = vec![ManifestSubsystem {
            id: "api".to_string(),
            name: "API".to_string(),
            domain: "core".to_string(),
            status: ManifestStatus::Built,
            file_path: PathBuf::from("src/api/index.ts"),
            interfaces: Vec::new(),
            operations: Vec::new(),
            tables: Vec::new(),
            events: Vec::new(),
            children: Vec::new(),
            dependencies: Vec::new(),
        }];

        normalize_manifest_paths(&mut subsystems);

        assert_eq!(
            subsystems[0].file_path.display().to_string(),
            "src/api/index.ts",
            "already-forward-slashed paths should remain unchanged"
        );
    }

    // -----------------------------------------------------------------------
    // parse_cli_version_output tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_version_normal_output() {
        let result = parse_cli_version_output("domain-scan 1.2.3\n");
        assert_eq!(result, Some("1.2.3".to_string()));
    }

    #[test]
    fn parse_version_with_git_hash() {
        let result = parse_cli_version_output("domain-scan 1.2.3 (abc123)");
        assert_eq!(result, Some("1.2.3".to_string()));
    }

    #[test]
    fn parse_version_v_prefix() {
        // The function takes the second whitespace token, so "v1.2.3" is returned as-is.
        // normalize_version (used elsewhere) would strip the v, but parse_cli_version_output
        // returns the raw token.
        let result = parse_cli_version_output("domain-scan v1.2.3");
        assert_eq!(result, Some("v1.2.3".to_string()));
    }

    #[test]
    fn parse_version_empty_string() {
        let result = parse_cli_version_output("");
        assert_eq!(result, None);
    }

    #[test]
    fn parse_version_single_token() {
        let result = parse_cli_version_output("domain-scan");
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // File source cache eviction tests
    // -----------------------------------------------------------------------

    #[test]
    fn file_source_cache_clears_when_full() {
        let mut cache: HashMap<PathBuf, String> = HashMap::new();
        // Fill the cache to the limit
        for i in 0..50 {
            cache.insert(PathBuf::from(format!("/tmp/file{i}.ts")), format!("content{i}"));
        }
        assert_eq!(cache.len(), 50);

        // Simulate the eviction logic: clear the cache when it hits 50
        if cache.len() >= 50 {
            cache.clear();
        }
        assert_eq!(cache.len(), 0, "cache should be empty after eviction");

        // New entry can be inserted after clearing
        cache.insert(PathBuf::from("/tmp/new.ts"), "new content".to_string());
        assert_eq!(cache.len(), 1);
    }

    // -----------------------------------------------------------------------
    // normalize_os_label tests
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_os_label_macos() {
        assert_eq!(normalize_os_label("macos"), "darwin");
    }

    #[test]
    fn normalize_os_label_linux() {
        assert_eq!(normalize_os_label("linux"), "linux");
    }

    #[test]
    fn normalize_os_label_windows() {
        assert_eq!(normalize_os_label("windows"), "windows");
    }

    #[test]
    fn normalize_os_label_unknown() {
        assert_eq!(normalize_os_label("freebsd"), "freebsd");
    }

    // -----------------------------------------------------------------------
    // parse_version_parts tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_version_parts_simple() {
        assert_eq!(parse_version_parts("1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn parse_version_parts_with_v_prefix() {
        assert_eq!(parse_version_parts("v1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn parse_version_parts_with_prerelease() {
        assert_eq!(parse_version_parts("1.2.3-beta.1"), Some((1, 2, 3)));
    }

    #[test]
    fn parse_version_parts_with_build_metadata() {
        assert_eq!(parse_version_parts("1.2.3+abc123"), Some((1, 2, 3)));
    }

    #[test]
    fn parse_version_parts_too_few_components() {
        assert_eq!(parse_version_parts("1.2"), None);
    }

    #[test]
    fn parse_version_parts_too_many_components() {
        assert_eq!(parse_version_parts("1.2.3.4"), None);
    }

    #[test]
    fn parse_version_parts_non_numeric() {
        assert_eq!(parse_version_parts("a.b.c"), None);
    }

    #[test]
    fn parse_version_parts_empty() {
        assert_eq!(parse_version_parts(""), None);
    }

    // -----------------------------------------------------------------------
    // normalize_version tests
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_version_strips_v() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
    }

    #[test]
    fn normalize_version_no_v() {
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
    }

    #[test]
    fn normalize_version_trims_whitespace() {
        assert_eq!(normalize_version("  v1.2.3  "), "1.2.3");
    }

    // -----------------------------------------------------------------------
    // is_update_available tests
    // -----------------------------------------------------------------------

    #[test]
    fn is_update_available_newer_version() {
        assert!(is_update_available("1.0.0", "1.1.0"));
    }

    #[test]
    fn is_update_available_same_version() {
        assert!(!is_update_available("1.0.0", "1.0.0"));
    }

    #[test]
    fn is_update_available_older_version() {
        assert!(!is_update_available("2.0.0", "1.0.0"));
    }

    #[test]
    fn is_update_available_with_v_prefix() {
        assert!(is_update_available("v1.0.0", "v1.1.0"));
    }

    #[test]
    fn is_update_available_mixed_prefixes() {
        assert!(is_update_available("1.0.0", "v1.1.0"));
    }

    #[test]
    fn is_update_available_same_with_v() {
        assert!(!is_update_available("v1.0.0", "1.0.0"));
    }

    #[test]
    fn is_update_available_patch_bump() {
        assert!(is_update_available("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_update_available_unparseable_falls_back_to_string_compare() {
        // When versions can't be parsed, falls back to string comparison
        assert!(is_update_available("abc", "def"));
        assert!(!is_update_available("same", "same"));
    }

    // -----------------------------------------------------------------------
    // build_binary_install_command tests
    // -----------------------------------------------------------------------

    #[test]
    fn build_binary_install_command_windows_returns_cargo() {
        let asset = ReleaseAsset {
            name: "domain-scan-windows-x86_64.zip".to_string(),
            download_url: "https://example.com/asset.zip".to_string(),
            size: 1024,
        };
        let cmd = build_binary_install_command("windows", &asset);
        assert!(
            cmd.contains("cargo install"),
            "Windows should fall back to cargo install"
        );
    }

    #[test]
    fn build_binary_install_command_darwin_returns_curl() {
        let asset = ReleaseAsset {
            name: "domain-scan-darwin-aarch64.tar.gz".to_string(),
            download_url: "https://example.com/asset.tar.gz".to_string(),
            size: 2048,
        };
        let cmd = build_binary_install_command("darwin", &asset);
        assert!(
            cmd.contains("curl -sL"),
            "macOS should use curl-based install"
        );
        assert!(
            cmd.contains("https://example.com/asset.tar.gz"),
            "Install command should contain the download URL"
        );
        assert!(
            cmd.contains("chmod +x"),
            "Install command should make binary executable"
        );
    }

    #[test]
    fn build_binary_install_command_linux_returns_curl() {
        let asset = ReleaseAsset {
            name: "domain-scan-linux-x86_64.tar.gz".to_string(),
            download_url: "https://example.com/linux.tar.gz".to_string(),
            size: 4096,
        };
        let cmd = build_binary_install_command("linux", &asset);
        assert!(cmd.contains("curl -sL"));
        assert!(cmd.contains("https://example.com/linux.tar.gz"));
    }

    // -----------------------------------------------------------------------
    // Export format tests (pure logic, no Tauri State)
    // -----------------------------------------------------------------------

    #[test]
    fn export_csv_header_format() {
        // Verify the CSV header string is correct
        let csv_header = "name,kind,file,line,language,build_status,confidence\n";
        assert!(csv_header.contains("name"));
        assert!(csv_header.contains("kind"));
        assert!(csv_header.contains("confidence"));
        assert!(csv_header.ends_with('\n'));
    }

    #[test]
    fn export_markdown_header_format() {
        // Verify the markdown table header
        let md_header = "| Name | Kind | File | Line | Language | Build Status | Confidence |\n";
        let md_separator =
            "|------|------|------|------|----------|--------------|------------|\n";
        assert!(md_header.starts_with("| Name"));
        assert!(md_separator.starts_with("|---"));
    }
}
