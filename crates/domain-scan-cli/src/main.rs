use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use domain_scan_core::ir::{
    BuildStatus, EntityKind, FilterParams, Language, ScanConfig,
};
use domain_scan_core::output::{self, OutputFormat};
use domain_scan_core::{cache, index, manifest, parser, query_engine, validate, walker};

mod tui;

// ---------------------------------------------------------------------------
// CLI argument definitions
// ---------------------------------------------------------------------------

/// domain-scan — structural code intelligence via tree-sitter.
///
/// Find every interface, service, method, trait, protocol, and type boundary
/// in any codebase. Fast, deterministic, language-agnostic.
#[derive(Parser)]
#[command(name = "domain-scan", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Root directory to scan (default: .)
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    /// Config file path (default: .domain-scan.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Output format: json | table | compact
    #[arg(long, global = true, default_value = "table", value_enum, conflicts_with = "interactive")]
    output: OutputFormatArg,

    /// Launch TUI mode (ratatui). Single-keypress tree navigation.
    #[arg(long, global = true, conflicts_with = "output")]
    interactive: bool,

    /// Write output to file (default: stdout)
    #[arg(short = 'o', long = "out", global = true)]
    out: Option<PathBuf>,

    /// Override build status detection
    #[arg(long, global = true, value_enum)]
    build_status: Option<BuildStatusArg>,

    /// Disable caching
    #[arg(long, global = true)]
    no_cache: bool,

    /// Only scan these languages (comma-separated)
    #[arg(long, global = true, value_delimiter = ',')]
    languages: Vec<LanguageArg>,

    /// Suppress progress output
    #[arg(short = 'q', long, global = true)]
    quiet: bool,

    /// Verbose output (timing, cache stats)
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a full structural scan of a directory
    Scan,

    /// List all interfaces / traits / protocols
    Interfaces {
        /// Filter by name pattern (substring match)
        #[arg(long)]
        name: Option<String>,
        /// Show methods inline
        #[arg(long)]
        show_methods: bool,
    },

    /// List all service definitions
    Services {
        /// Filter by service kind (comma-separated)
        #[arg(long)]
        kind: Option<String>,
        /// Filter by name pattern
        #[arg(long)]
        name: Option<String>,
        /// Show routes inline
        #[arg(long)]
        show_routes: bool,
        /// Show dependencies inline
        #[arg(long)]
        show_deps: bool,
    },

    /// List all methods (optionally filtered by owner)
    Methods {
        /// Filter by owner class/struct name
        #[arg(long)]
        owner: Option<String>,
        /// Only show async methods
        #[arg(long, name = "async")]
        is_async: bool,
        /// Filter by visibility
        #[arg(long)]
        visibility: Option<String>,
        /// Filter by name pattern
        #[arg(long)]
        name: Option<String>,
    },

    /// List all runtime schema definitions
    Schemas {
        /// Filter by framework (comma-separated, e.g. zod,drizzle)
        #[arg(long)]
        framework: Option<String>,
        /// Filter by schema kind (comma-separated)
        #[arg(long)]
        kind: Option<String>,
        /// Filter by name pattern
        #[arg(long)]
        name: Option<String>,
        /// Show fields inline
        #[arg(long)]
        show_fields: bool,
    },

    /// List implementations of a trait/interface
    Impls {
        /// Trait/interface name to look up
        name: Option<String>,
        /// Show all implementations
        #[arg(long)]
        all: bool,
        /// Show implemented methods
        #[arg(long)]
        show_methods: bool,
    },

    /// Search across all entity names
    Search {
        /// Search query (substring match)
        query: String,
        /// Filter by entity kind (comma-separated)
        #[arg(long)]
        kind: Option<String>,
        /// Use regex for search
        #[arg(long)]
        regex: bool,
    },

    /// Print scan statistics
    Stats,

    /// Run data quality checks on scan results
    Validate {
        /// Run only specific rules (comma-separated)
        #[arg(long)]
        rules: Option<String>,
        /// Validate against a subsystem manifest
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Treat warnings as failures (exit 1 on WARN)
        #[arg(long)]
        strict: bool,
    },

    /// Match entities to subsystems defined in a manifest
    Match {
        /// Path to the manifest file (e.g. system.json)
        #[arg(long)]
        manifest: PathBuf,
        /// Show only unmatched entities
        #[arg(long)]
        unmatched_only: bool,
        /// Generate LLM prompt for unmatched items
        #[arg(long)]
        prompt_unmatched: bool,
        /// Number of agents for prompt generation
        #[arg(long, default_value = "3")]
        agents: usize,
        /// Write matched entities back to manifest
        #[arg(long)]
        write_back: bool,
        /// Dry run (no writes)
        #[arg(long)]
        dry_run: bool,
        /// Exit 1 if unmatched items remain
        #[arg(long)]
        fail_on_unmatched: bool,
    },

    /// Cache management
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Generate an LLM prompt with sub-agent dispatch
    Prompt {
        /// Number of agents
        #[arg(long, default_value = "5")]
        agents: usize,
        /// Focus on entities matching this pattern
        #[arg(long)]
        focus: Option<String>,
        /// Include full scan results in prompt
        #[arg(long)]
        include_scan: bool,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Show cache statistics
    Stats,
    /// Clear all cached entries
    Clear,
    /// Remove entries for deleted files
    Prune,
}

// ---------------------------------------------------------------------------
// Value enums for clap
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormatArg {
    Json,
    Table,
    Compact,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Table => OutputFormat::Table,
            OutputFormatArg::Compact => OutputFormat::Compact,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum BuildStatusArg {
    Built,
    Unbuilt,
    Error,
    Rebuild,
}

impl From<BuildStatusArg> for BuildStatus {
    fn from(arg: BuildStatusArg) -> Self {
        match arg {
            BuildStatusArg::Built => BuildStatus::Built,
            BuildStatusArg::Unbuilt => BuildStatus::Unbuilt,
            BuildStatusArg::Error => BuildStatus::Error,
            BuildStatusArg::Rebuild => BuildStatus::Rebuild,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum LanguageArg {
    Typescript,
    Python,
    Rust,
    Go,
    Java,
    Kotlin,
    Csharp,
    Swift,
    Php,
    Ruby,
    Scala,
    Cpp,
}

impl From<LanguageArg> for Language {
    fn from(arg: LanguageArg) -> Self {
        match arg {
            LanguageArg::Typescript => Language::TypeScript,
            LanguageArg::Python => Language::Python,
            LanguageArg::Rust => Language::Rust,
            LanguageArg::Go => Language::Go,
            LanguageArg::Java => Language::Java,
            LanguageArg::Kotlin => Language::Kotlin,
            LanguageArg::Csharp => Language::CSharp,
            LanguageArg::Swift => Language::Swift,
            LanguageArg::Php => Language::PHP,
            LanguageArg::Ruby => Language::Ruby,
            LanguageArg::Scala => Language::Scala,
            LanguageArg::Cpp => Language::Cpp,
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else if !cli.quiet {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    if let Err(e) = run(cli) {
        let err = CliError {
            code: "CLI_ERROR",
            message: e.to_string(),
            suggestion: None,
        };
        // Structured error to stderr
        if let Ok(json) = serde_json::to_string_pretty(&err) {
            eprintln!("{json}");
        } else {
            eprintln!("Error: {e}");
        }
        process::exit(1);
    }
}

#[derive(serde::Serialize)]
struct CliError<'a> {
    code: &'a str,
    message: String,
    suggestion: Option<String>,
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let format: OutputFormat = cli.output.into();

    match &cli.command {
        Commands::Scan => {
            if cli.interactive {
                let scan_index = run_scan(&cli)?;
                run_tui(tui::TuiApp::from_entity_list(&scan_index, "Scan"))
            } else {
                cmd_scan(&cli, format)
            }
        }
        Commands::Interfaces { ref name, show_methods } => {
            if cli.interactive {
                let scan_index = run_scan(&cli)?;
                run_tui(tui::TuiApp::from_interfaces(&scan_index))
            } else {
                cmd_interfaces(&cli, format, name.clone(), *show_methods)
            }
        }
        Commands::Services {
            ref kind,
            ref name,
            show_routes,
            show_deps,
        } => {
            if cli.interactive {
                let scan_index = run_scan(&cli)?;
                run_tui(tui::TuiApp::from_services(&scan_index))
            } else {
                cmd_services(&cli, format, kind.clone(), name.clone(), *show_routes, *show_deps)
            }
        }
        Commands::Methods {
            ref owner,
            is_async,
            ref visibility,
            ref name,
        } => cmd_methods(&cli, format, owner.clone(), *is_async, visibility.clone(), name.clone()),
        Commands::Schemas {
            ref framework,
            ref kind,
            ref name,
            show_fields,
        } => {
            if cli.interactive {
                let scan_index = run_scan(&cli)?;
                run_tui(tui::TuiApp::from_schemas(&scan_index))
            } else {
                cmd_schemas(&cli, format, framework.clone(), kind.clone(), name.clone(), *show_fields)
            }
        }
        Commands::Impls {
            ref name,
            all,
            show_methods,
        } => cmd_impls(&cli, format, name.clone(), *all, *show_methods),
        Commands::Search {
            ref query,
            ref kind,
            regex: _,
        } => cmd_search(&cli, format, query.clone(), kind.clone()),
        Commands::Stats => cmd_stats(&cli, format),
        Commands::Validate {
            ref rules,
            manifest: ref manifest_path,
            strict,
        } => cmd_validate(&cli, format, rules.clone(), manifest_path.clone(), *strict),
        Commands::Match {
            manifest: ref manifest_path,
            unmatched_only,
            prompt_unmatched: _,
            agents: _,
            write_back: _,
            dry_run: _,
            fail_on_unmatched,
        } => cmd_match(&cli, format, manifest_path.clone(), *unmatched_only, *fail_on_unmatched),
        Commands::Cache { ref action } => cmd_cache(&cli, action),
        Commands::Prompt {
            agents: _,
            focus: _,
            include_scan: _,
        } => {
            eprintln!("Prompt generation is not yet implemented (Phase 7).");
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// TUI launch helper
// ---------------------------------------------------------------------------

fn run_tui(mut app: tui::TuiApp) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = tui::setup_terminal()?;
    let result = app.run(&mut terminal);
    tui::teardown_terminal(&mut terminal)?;
    result
}

// ---------------------------------------------------------------------------
// Build ScanConfig from CLI args
// ---------------------------------------------------------------------------

fn build_scan_config(cli: &Cli) -> ScanConfig {
    let root = cli.root.clone();
    let languages: Vec<Language> = cli.languages.iter().map(|l| Language::from(*l)).collect();
    let build_status_override = cli.build_status.map(BuildStatus::from);
    let cache_dir = root.join(".domain-scan-cache");

    ScanConfig {
        root,
        include: Vec::new(),
        exclude: Vec::new(),
        languages,
        build_status_override,
        cache_enabled: !cli.no_cache,
        cache_dir,
    }
}

// ---------------------------------------------------------------------------
// Run the scan pipeline: walk -> parse -> extract -> index
// ---------------------------------------------------------------------------

fn run_scan(cli: &Cli) -> Result<domain_scan_core::ir::ScanIndex, Box<dyn std::error::Error>> {
    let config = build_scan_config(cli);
    let start = std::time::Instant::now();

    // Step 1: Walk
    let walked = walker::walk_directory(&config)?;

    if walked.is_empty() {
        if !cli.quiet {
            eprintln!("No files found in {}", config.root.display());
        }
        return Ok(index::build_index(config.root, Vec::new(), 0, 0, 0));
    }

    if !cli.quiet {
        eprintln!("Found {} files", walked.len());
    }

    // Step 2: Optional cache
    let disk_cache = if config.cache_enabled {
        let c = cache::Cache::new(config.cache_dir.clone(), 100);
        let _ = c.load_from_disk();
        Some(c)
    } else {
        None
    };

    // Step 3: Parse + Extract
    let mut ir_files = Vec::new();
    let mut cache_hits: usize = 0;
    let mut cache_misses: usize = 0;

    for walked_file in &walked {
        let build_status = config
            .build_status_override
            .unwrap_or(BuildStatus::Built);

        // Try cache first
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

        let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)?;
        let ir = query_engine::extract(
            &tree,
            &source,
            &walked_file.path,
            walked_file.language,
            build_status,
        )?;

        // Store in cache
        if let Some(ref c) = disk_cache {
            let _ = c.insert(hash, ir.clone());
        }

        ir_files.push(ir);
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if cli.verbose {
        eprintln!(
            "Parsed {} files in {}ms ({} cached, {} parsed)",
            walked.len(),
            duration_ms,
            cache_hits,
            cache_misses,
        );
    }

    Ok(index::build_index(
        config.root,
        ir_files,
        duration_ms,
        cache_hits,
        cache_misses,
    ))
}

// ---------------------------------------------------------------------------
// Output helper
// ---------------------------------------------------------------------------

fn emit(cli: &Cli, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref path) = cli.out {
        output::write_to_file(content, path)?;
        if !cli.quiet {
            eprintln!("Output written to {}", path.display());
        }
    } else {
        print!("{content}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: scan
// ---------------------------------------------------------------------------

fn cmd_scan(cli: &Cli, format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;
    let content = output::format_scan_index(&scan_index, format)?;
    emit(cli, &content)
}

// ---------------------------------------------------------------------------
// Subcommand: interfaces
// ---------------------------------------------------------------------------

fn cmd_interfaces(
    cli: &Cli,
    format: OutputFormat,
    name: Option<String>,
    show_methods: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;
    let interfaces = scan_index.get_interfaces(name.as_deref());

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&interfaces)?;
            emit(cli, &json)?;
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.set_header(vec!["Language", "Kind", "Name", "Methods", "Extends", "File"]);
            for iface in &interfaces {
                let extends = iface.extends.join(", ");
                let file_loc = format!(
                    "{}:{}",
                    iface.file.display(),
                    iface.span.start_line
                );
                table.add_row(vec![
                    interface_kind_str(&iface.language_kind).to_string(),
                    interface_kind_str(&iface.language_kind).to_string(),
                    iface.name.clone(),
                    iface.methods.len().to_string(),
                    extends,
                    file_loc,
                ]);
            }
            let mut out = table.to_string();
            if show_methods {
                out.push('\n');
                for iface in &interfaces {
                    if !iface.methods.is_empty() {
                        out.push_str(&format!("\n{} methods:\n", iface.name));
                        for m in &iface.methods {
                            let async_str = if m.is_async { "async " } else { "" };
                            let ret = m.return_type.as_deref().unwrap_or("void");
                            out.push_str(&format!("  {async_str}{}(...) -> {ret}\n", m.name));
                        }
                    }
                }
            }
            emit(cli, &out)?;
        }
        OutputFormat::Compact => {
            let mut out = String::new();
            for iface in &interfaces {
                out.push_str(&format!(
                    "interface:{} [{}] {} methods\n",
                    iface.name,
                    iface.file.display(),
                    iface.methods.len(),
                ));
            }
            emit(cli, &out)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: services
// ---------------------------------------------------------------------------

fn cmd_services(
    cli: &Cli,
    format: OutputFormat,
    _kind: Option<String>,
    name: Option<String>,
    show_routes: bool,
    show_deps: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;
    let mut services: Vec<_> = scan_index.get_services(None);

    if let Some(ref n) = name {
        services.retain(|s| s.name.contains(n.as_str()));
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&services)?;
            emit(cli, &json)?;
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            let mut headers = vec!["Kind", "Name", "Methods", "Routes", "File"];
            if show_deps {
                headers.push("Dependencies");
            }
            table.set_header(headers);
            for svc in &services {
                let file_loc =
                    format!("{}:{}", svc.file.display(), svc.span.start_line);
                let mut row = vec![
                    format!("{:?}", svc.kind),
                    svc.name.clone(),
                    svc.methods.len().to_string(),
                    svc.routes.len().to_string(),
                    file_loc,
                ];
                if show_deps {
                    row.push(svc.dependencies.join(", "));
                }
                table.add_row(row);
            }
            let mut out = table.to_string();
            if show_routes {
                out.push('\n');
                for svc in &services {
                    if !svc.routes.is_empty() {
                        out.push_str(&format!("\n{} routes:\n", svc.name));
                        for r in &svc.routes {
                            out.push_str(&format!(
                                "  {:?} {} -> {}\n",
                                r.method, r.path, r.handler
                            ));
                        }
                    }
                }
            }
            emit(cli, &out)?;
        }
        OutputFormat::Compact => {
            let mut out = String::new();
            for svc in &services {
                out.push_str(&format!(
                    "service:{} [{:?}] {} methods\n",
                    svc.name,
                    svc.kind,
                    svc.methods.len(),
                ));
            }
            emit(cli, &out)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: methods
// ---------------------------------------------------------------------------

fn cmd_methods(
    cli: &Cli,
    format: OutputFormat,
    owner: Option<String>,
    is_async: bool,
    _visibility: Option<String>,
    name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;

    // Collect all methods
    let mut methods: Vec<&domain_scan_core::ir::MethodDef> = Vec::new();
    if let Some(ref owner_name) = owner {
        methods.extend(scan_index.get_methods_by_owner(owner_name));
    } else {
        for file in &scan_index.files {
            for class in &file.classes {
                methods.extend(class.methods.iter());
            }
            for svc in &file.services {
                methods.extend(svc.methods.iter());
            }
            for impl_def in &file.implementations {
                methods.extend(impl_def.methods.iter());
            }
        }
    }

    // Apply filters
    if is_async {
        methods.retain(|m| m.is_async);
    }
    if let Some(ref n) = name {
        methods.retain(|m| m.name.contains(n.as_str()));
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&methods)?;
            emit(cli, &json)?;
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.set_header(vec!["Name", "Owner", "Async", "Visibility", "Return", "File"]);
            for m in &methods {
                let owner_name = m.owner.as_deref().unwrap_or("-");
                let ret = m.return_type.as_deref().unwrap_or("-");
                let file_loc =
                    format!("{}:{}", m.file.display(), m.span.start_line);
                table.add_row(vec![
                    m.name.clone(),
                    owner_name.to_string(),
                    m.is_async.to_string(),
                    format!("{:?}", m.visibility),
                    ret.to_string(),
                    file_loc,
                ]);
            }
            emit(cli, &table.to_string())?;
        }
        OutputFormat::Compact => {
            let mut out = String::new();
            for m in &methods {
                let owner_name = m.owner.as_deref().unwrap_or("");
                let async_str = if m.is_async { "async " } else { "" };
                if owner_name.is_empty() {
                    out.push_str(&format!("method:{async_str}{}\n", m.name));
                } else {
                    out.push_str(&format!(
                        "method:{owner_name}.{async_str}{}\n",
                        m.name
                    ));
                }
            }
            emit(cli, &out)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: schemas
// ---------------------------------------------------------------------------

fn cmd_schemas(
    cli: &Cli,
    format: OutputFormat,
    framework: Option<String>,
    _kind: Option<String>,
    name: Option<String>,
    show_fields: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;
    let mut schemas: Vec<_> = scan_index.get_schemas(framework.as_deref());

    if let Some(ref n) = name {
        schemas.retain(|s| s.name.contains(n.as_str()));
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&schemas)?;
            emit(cli, &json)?;
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.set_header(vec!["Framework", "Kind", "Name", "Fields", "File"]);
            for s in &schemas {
                let file_loc =
                    format!("{}:{}", s.file.display(), s.span.start_line);
                table.add_row(vec![
                    s.source_framework.clone(),
                    format!("{:?}", s.kind),
                    s.name.clone(),
                    s.fields.len().to_string(),
                    file_loc,
                ]);
            }
            let mut out = table.to_string();
            if show_fields {
                out.push('\n');
                for s in &schemas {
                    if !s.fields.is_empty() {
                        out.push_str(&format!("\n{} fields:\n", s.name));
                        for f in &s.fields {
                            let ty = f.type_annotation.as_deref().unwrap_or("?");
                            let opt = if f.is_optional { "?" } else { "" };
                            out.push_str(&format!("  {}{opt}: {ty}\n", f.name));
                        }
                    }
                }
            }
            emit(cli, &out)?;
        }
        OutputFormat::Compact => {
            let mut out = String::new();
            for s in &schemas {
                out.push_str(&format!(
                    "schema:{} [{}] {} fields\n",
                    s.name,
                    s.source_framework,
                    s.fields.len(),
                ));
            }
            emit(cli, &out)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: impls
// ---------------------------------------------------------------------------

fn cmd_impls(
    cli: &Cli,
    format: OutputFormat,
    name: Option<String>,
    all: bool,
    show_methods: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;

    if all {
        // Show all implementations
        let mut all_impls: Vec<&domain_scan_core::ir::ImplDef> = Vec::new();
        for file in &scan_index.files {
            all_impls.extend(file.implementations.iter());
        }

        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&all_impls)?;
                emit(cli, &json)?;
            }
            OutputFormat::Table => {
                let mut table = comfy_table::Table::new();
                let mut headers = vec!["Target", "Trait", "Methods", "File"];
                if show_methods {
                    headers.push("Method Names");
                }
                table.set_header(headers);
                for imp in &all_impls {
                    let trait_name = imp.trait_name.as_deref().unwrap_or("-");
                    let file_loc = format!(
                        "{}:{}",
                        imp.file.display(),
                        imp.span.start_line
                    );
                    let mut row = vec![
                        imp.target.clone(),
                        trait_name.to_string(),
                        imp.methods.len().to_string(),
                        file_loc,
                    ];
                    if show_methods {
                        let names: Vec<_> = imp.methods.iter().map(|m| m.name.as_str()).collect();
                        row.push(names.join(", "));
                    }
                    table.add_row(row);
                }
                emit(cli, &table.to_string())?;
            }
            OutputFormat::Compact => {
                let mut out = String::new();
                for imp in &all_impls {
                    let trait_name = imp.trait_name.as_deref().unwrap_or("(inherent)");
                    out.push_str(&format!(
                        "impl:{} for {} ({} methods)\n",
                        trait_name,
                        imp.target,
                        imp.methods.len(),
                    ));
                }
                emit(cli, &out)?;
            }
        }
    } else if let Some(ref trait_name) = name {
        // Show implementations of a specific trait
        let impls = scan_index.get_implementations(trait_name);
        let implementors = scan_index.get_implementors(trait_name);

        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&impls)?;
                emit(cli, &json)?;
            }
            OutputFormat::Table => {
                let mut out = format!(
                    "Trait: {trait_name} ({} implementors)\n\n",
                    implementors.len()
                );
                let mut table = comfy_table::Table::new();
                let mut headers = vec!["Implementor", "Methods", "File"];
                if show_methods {
                    headers.push("Method Names");
                }
                table.set_header(headers);
                for imp in &impls {
                    let file_loc = format!(
                        "{}:{}",
                        imp.file.display(),
                        imp.span.start_line
                    );
                    let mut row = vec![
                        imp.target.clone(),
                        imp.methods.len().to_string(),
                        file_loc,
                    ];
                    if show_methods {
                        let names: Vec<_> = imp.methods.iter().map(|m| m.name.as_str()).collect();
                        row.push(names.join(", "));
                    }
                    table.add_row(row);
                }
                out.push_str(&table.to_string());
                emit(cli, &out)?;
            }
            OutputFormat::Compact => {
                let mut out = String::new();
                for name in &implementors {
                    out.push_str(&format!("impl:{trait_name} for {name}\n"));
                }
                emit(cli, &out)?;
            }
        }
    } else {
        eprintln!("Usage: domain-scan impls <NAME> or domain-scan impls --all");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: search
// ---------------------------------------------------------------------------

fn cmd_search(
    cli: &Cli,
    format: OutputFormat,
    query: String,
    kind: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;

    let mut filter = FilterParams {
        name_pattern: Some(query),
        ..FilterParams::default()
    };

    if let Some(ref k) = kind {
        let kinds: Vec<EntityKind> = k
            .split(',')
            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                "interface" => Some(EntityKind::Interface),
                "service" => Some(EntityKind::Service),
                "class" => Some(EntityKind::Class),
                "function" => Some(EntityKind::Function),
                "schema" => Some(EntityKind::Schema),
                "impl" => Some(EntityKind::Impl),
                "type_alias" | "typealias" => Some(EntityKind::TypeAlias),
                "method" => Some(EntityKind::Method),
                _ => None,
            })
            .collect();
        if !kinds.is_empty() {
            filter.kind = Some(kinds);
        }
    }

    let summaries = scan_index.get_entity_summaries(&filter);

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&summaries)?;
            emit(cli, &json)?;
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.set_header(vec!["Kind", "Name", "Language", "Status", "File"]);
            for s in &summaries {
                let file_loc = format!("{}:{}", s.file.display(), s.line);
                table.add_row(vec![
                    format!("{:?}", s.kind),
                    s.name.clone(),
                    format!("{}", s.language),
                    format!("{}", s.build_status),
                    file_loc,
                ]);
            }
            emit(cli, &table.to_string())?;
        }
        OutputFormat::Compact => {
            let mut out = String::new();
            for s in &summaries {
                out.push_str(&format!(
                    "{:?}:{} [{}]\n",
                    s.kind,
                    s.name,
                    s.file.display(),
                ));
            }
            emit(cli, &out)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: stats
// ---------------------------------------------------------------------------

fn cmd_stats(cli: &Cli, format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&scan_index.stats)?;
            emit(cli, &json)?;
        }
        OutputFormat::Table | OutputFormat::Compact => {
            let stats = &scan_index.stats;
            let mut out = String::new();
            out.push_str("Scan Statistics\n");
            out.push_str(&format!("Root:            {}\n", scan_index.root.display()));
            out.push_str(&format!("Files scanned:   {}\n", stats.total_files));

            let mut langs: Vec<_> = stats.files_by_language.iter().collect();
            langs.sort_by(|a, b| b.1.cmp(a.1));
            let lang_str: Vec<_> = langs
                .iter()
                .map(|(l, c)| format!("{l} ({c})"))
                .collect();
            out.push_str(&format!("Languages:       {}\n", lang_str.join(", ")));

            out.push('\n');
            out.push_str(&format!("Interfaces:      {}\n", stats.total_interfaces));
            out.push_str(&format!("Services:        {}\n", stats.total_services));
            out.push_str(&format!("Classes:         {}\n", stats.total_classes));
            out.push_str(&format!("Methods:         {}\n", stats.total_methods));
            out.push_str(&format!("Functions:       {}\n", stats.total_functions));
            out.push_str(&format!("Schemas:         {}\n", stats.total_schemas));
            out.push_str(&format!("Type aliases:    {}\n", stats.total_type_aliases));
            out.push_str(&format!("Implementations: {}\n", stats.total_implementations));

            if stats.parse_duration_ms > 0 || stats.cache_hits > 0 {
                out.push('\n');
                let duration_s = stats.parse_duration_ms as f64 / 1000.0;
                out.push_str(&format!(
                    "Parse time:      {duration_s:.1}s ({} files, {} cached)\n",
                    stats.total_files, stats.cache_hits,
                ));
                let total = stats.cache_hits + stats.cache_misses;
                if total > 0 {
                    let hit_rate = (stats.cache_hits as f64 / total as f64) * 100.0;
                    out.push_str(&format!("Cache hit rate:  {hit_rate:.0}%\n"));
                }
            }
            emit(cli, &out)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: validate
// ---------------------------------------------------------------------------

fn cmd_validate(
    cli: &Cli,
    format: OutputFormat,
    rules: Option<String>,
    _manifest_path: Option<PathBuf>,
    strict: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;

    let result = if let Some(ref rule_list) = rules {
        let rule_names: Vec<&str> = rule_list.split(',').map(|s| s.trim()).collect();
        validate::validate_rules(&scan_index, &rule_names)
    } else {
        validate::validate(&scan_index)
    };

    match format {
        OutputFormat::Json => {
            let json = output::format_validation_result(&result)?;
            emit(cli, &json)?;
        }
        OutputFormat::Table | OutputFormat::Compact => {
            let mut out = String::new();
            out.push_str(&format!(
                "Validation: {} rules checked, {} pass, {} warn, {} fail\n\n",
                result.rules_checked, result.pass_count, result.warn_count, result.fail_count,
            ));
            if result.violations.is_empty() {
                out.push_str("All checks passed.\n");
            } else {
                for v in &result.violations {
                    let severity = match v.severity {
                        domain_scan_core::ir::ViolationSeverity::Warn => "WARN",
                        domain_scan_core::ir::ViolationSeverity::Fail => "FAIL",
                    };
                    let entity = v.entity_name.as_deref().unwrap_or("-");
                    out.push_str(&format!(
                        "[{severity}] {}: {entity} - {}\n",
                        v.rule, v.message,
                    ));
                }
            }
            emit(cli, &out)?;
        }
    }

    // Exit codes
    if result.fail_count > 0 {
        process::exit(1);
    }
    if strict && result.warn_count > 0 {
        process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: match
// ---------------------------------------------------------------------------

fn cmd_match(
    cli: &Cli,
    format: OutputFormat,
    manifest_path: PathBuf,
    unmatched_only: bool,
    fail_on_unmatched: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;
    let manifest_data = manifest::parse_manifest_file(&manifest_path)?;
    let result = manifest::match_entities(&scan_index, &manifest_data);

    match format {
        OutputFormat::Json => {
            if unmatched_only {
                let json = serde_json::to_string_pretty(&result.unmatched)?;
                emit(cli, &json)?;
            } else {
                let json = output::format_match_result(&result)?;
                emit(cli, &json)?;
            }
        }
        OutputFormat::Table | OutputFormat::Compact => {
            let mut out = String::new();
            if !unmatched_only {
                out.push_str(&format!(
                    "Matched: {} | Unmatched: {} | Coverage: {:.0}%\n\n",
                    result.matched.len(),
                    result.unmatched.len(),
                    result.coverage_percent,
                ));
                for m in &result.matched {
                    out.push_str(&format!(
                        "  + {:?}:{} -> {} ({})\n",
                        m.entity.kind,
                        m.entity.name,
                        m.subsystem_name,
                        match m.match_strategy {
                            domain_scan_core::ir::MatchStrategy::FilePath => "file-path",
                            domain_scan_core::ir::MatchStrategy::ImportGraph => "import-graph",
                            domain_scan_core::ir::MatchStrategy::NameMatch => "name-match",
                        },
                    ));
                }
                if !result.unmatched.is_empty() {
                    out.push_str("\nUnmatched:\n");
                }
            }
            for u in &result.unmatched {
                out.push_str(&format!(
                    "  - {:?}:{} [{}:{}]\n",
                    u.entity.kind,
                    u.entity.name,
                    u.entity.file.display(),
                    u.entity.line,
                ));
            }
            emit(cli, &out)?;
        }
    }

    if fail_on_unmatched && !result.unmatched.is_empty() {
        process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: cache
// ---------------------------------------------------------------------------

fn cmd_cache(cli: &Cli, action: &CacheAction) -> Result<(), Box<dyn std::error::Error>> {
    let config = build_scan_config(cli);
    let c = cache::Cache::new(config.cache_dir, 100);
    let _ = c.load_from_disk();

    match *action {
        CacheAction::Stats => {
            let stats = c.stats();
            let size_mb = stats.disk_size_bytes as f64 / (1024.0 * 1024.0);
            let max_mb = stats.max_size_bytes as f64 / (1024.0 * 1024.0);
            let json = serde_json::to_string_pretty(&stats)?;
            eprintln!(
                "Cache: {} entries, {size_mb:.1} MB / {max_mb:.0} MB",
                stats.entries,
            );
            print!("{json}");
        }
        CacheAction::Clear => {
            c.clear()?;
            eprintln!("Cache cleared.");
        }
        CacheAction::Prune => {
            // Simple prune: clear for now (full impl would check file existence)
            eprintln!("Prune not yet fully implemented; clearing cache.");
            c.clear()?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn interface_kind_str(kind: &domain_scan_core::ir::InterfaceKind) -> &'static str {
    use domain_scan_core::ir::InterfaceKind;
    match kind {
        InterfaceKind::Interface => "interface",
        InterfaceKind::Trait => "trait",
        InterfaceKind::Protocol => "protocol",
        InterfaceKind::AbstractClass => "abstract",
        InterfaceKind::PureVirtual => "virtual",
        InterfaceKind::Module => "module",
    }
}
