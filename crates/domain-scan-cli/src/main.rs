use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::{Parser, Subcommand, ValueEnum};
use domain_scan_core::input_validation;
use domain_scan_core::ir::{BuildStatus, EntityKind, FilterParams, Language, ScanConfig};
use domain_scan_core::output::{self, OutputFormat};
use domain_scan_core::prompt::PromptConfig;
use domain_scan_core::{
    cache, index, manifest, manifest_builder, parser, prompt, query_engine, validate, walker,
};
use serde::Deserialize;

mod tui;

// ---------------------------------------------------------------------------
// CLI argument definitions
// ---------------------------------------------------------------------------

/// domain-scan — structural code intelligence via tree-sitter.
///
/// Find every interface, service, method, trait, protocol, and type boundary
/// in any codebase. Fast, deterministic, language-agnostic.
///
/// AGENT SKILLS: Run `domain-scan skills list` to discover embedded agent
/// skill files. Install them with `domain-scan skills install --claude-code`
/// (or --codex, --dir). Skills teach AI agents how to use domain-scan
/// effectively — scan workflows, manifest building, tube map interaction.
#[derive(Parser)]
#[command(name = "domain-scan", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Root directory to scan (default: .)
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    /// Root directory (positional alternative to --root).
    /// Use `domain-scan scan .` instead of `domain-scan scan --root .`.
    #[arg(global = true, value_name = "PATH")]
    path: Option<PathBuf>,

    /// Config file path (default: .domain-scan.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Output format: json | table | compact.
    /// Defaults to json when stdout is not a TTY (piped/redirected), table otherwise.
    #[arg(long, global = true, value_enum, conflicts_with = "interactive")]
    output: Option<OutputFormatArg>,

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

    /// Limit JSON output to specific fields (comma-separated, dot-notation).
    /// Only applies when --output json is active.
    /// Example: --fields name,methods or --fields files.path,stats
    #[arg(long, global = true)]
    fields: Option<String>,

    /// Emit one JSON object per entity (NDJSON / JSON Lines), one per line.
    /// Works with interfaces, services, methods, schemas, impls, search.
    /// Compatible with --fields (field mask applied per line).
    #[arg(long, global = true)]
    page_all: bool,

    /// Raw JSON payload input that replaces individual filter flags for the subcommand.
    /// Structure mirrors `domain-scan schema <command>` output.
    /// Mutually exclusive with individual filter flags (--name, --kind, etc.).
    /// Max size: 1 MB. Max nesting depth: 32.
    #[arg(long = "json", global = true)]
    json_input: Option<String>,
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
        /// Search query (substring match). Required unless --json is used.
        query: Option<String>,
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
        /// Validate domain-scan's own codebase (uses crate root as --root)
        #[arg(long)]
        self_test: bool,
    },

    /// Match entities to subsystems defined in a manifest
    Match {
        /// Path to the manifest file (e.g. system.json). Required unless --json is used.
        #[arg(long)]
        manifest: Option<PathBuf>,
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

    /// Initialize a system manifest by scanning the codebase and inferring domains/subsystems
    Init {
        /// Use heuristic bootstrapping to infer domains, subsystems, and connections
        #[arg(long)]
        bootstrap: bool,
        /// Apply (load + validate) an existing manifest file
        #[arg(long)]
        apply_manifest: Option<PathBuf>,
        /// Preview what would be written without actually writing
        #[arg(long)]
        dry_run: bool,
        /// Project name for the generated manifest
        #[arg(long)]
        name: Option<String>,
    },

    /// Dump the JSON schema for a subcommand's input/output types
    Schema {
        /// Subcommand name (e.g. scan, interfaces, services, methods, schemas, impls, search, stats, validate, match, prompt, init)
        command: Option<String>,
        /// Dump all schemas in a single JSON object keyed by subcommand name
        #[arg(long)]
        all: bool,
    },

    /// Manage embedded agent skill files
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Show cache statistics
    Stats,
    /// Clear all cached entries
    Clear {
        /// Preview what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove entries for deleted files
    Prune {
        /// Preview what would be pruned without actually deleting
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum SkillsAction {
    /// List all embedded skill names
    List,
    /// Print a specific skill to stdout
    Show {
        /// Skill name (e.g. domain-scan-init)
        name: String,
    },
    /// Print all skills concatenated (for context injection)
    Dump,
    /// Install skill files to a project directory
    Install {
        /// Install to .claude/skills/ in the project root (for Claude Code)
        #[arg(long)]
        claude_code: bool,
        /// Install to .codex/skills/ in the project root (for Codex)
        #[arg(long)]
        codex: bool,
        /// Install to a custom directory
        #[arg(long)]
        dir: Option<PathBuf>,
    },
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
// JSON input types for --json flag (one per subcommand)
// ---------------------------------------------------------------------------

const MAX_JSON_INPUT_SIZE: usize = 1_048_576; // 1 MB
const MAX_JSON_DEPTH: usize = 32;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InterfacesJsonInput {
    name: Option<String>,
    #[serde(default)]
    show_methods: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServicesJsonInput {
    kind: Option<String>,
    name: Option<String>,
    #[serde(default)]
    show_routes: bool,
    #[serde(default)]
    show_deps: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MethodsJsonInput {
    owner: Option<String>,
    #[serde(default, rename = "async")]
    is_async: bool,
    visibility: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemasJsonInput {
    framework: Option<String>,
    kind: Option<String>,
    name: Option<String>,
    #[serde(default)]
    show_fields: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImplsJsonInput {
    name: Option<String>,
    #[serde(default)]
    all: bool,
    #[serde(default)]
    show_methods: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchJsonInput {
    query: String,
    kind: Option<String>,
    /// Accepted for schema completeness; search regex support is handled upstream.
    #[serde(default)]
    #[allow(dead_code)]
    regex: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidateJsonInput {
    rules: Option<String>,
    manifest: Option<String>,
    #[serde(default)]
    strict: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchJsonInput {
    manifest: String,
    #[serde(default)]
    unmatched_only: bool,
    /// Accepted for schema completeness; prompt generation not yet wired through --json.
    #[serde(default)]
    #[allow(dead_code)]
    prompt_unmatched: bool,
    /// Accepted for schema completeness; agent count not yet wired through --json.
    #[serde(default = "default_match_agents")]
    #[allow(dead_code)]
    agents: usize,
    /// Write matched entities back to manifest.
    #[serde(default)]
    write_back: bool,
    /// Preview what --write-back would do without actually writing.
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    fail_on_unmatched: bool,
}

fn default_match_agents() -> usize {
    3
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptJsonInput {
    #[serde(default = "default_prompt_agents")]
    agents: usize,
    focus: Option<String>,
    #[serde(default)]
    include_scan: bool,
}

fn default_prompt_agents() -> usize {
    5
}

// ---------------------------------------------------------------------------
// JSON input validation & parsing
// ---------------------------------------------------------------------------

fn json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(arr) => 1 + arr.iter().map(json_depth).max().unwrap_or(0),
        serde_json::Value::Object(obj) => 1 + obj.values().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

fn parse_json_input<T: serde::de::DeserializeOwned>(
    raw: &str,
    command: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    if raw.len() > MAX_JSON_INPUT_SIZE {
        return Err(format!(
            "JSON input is {} bytes, exceeds maximum of {} bytes (1 MB)",
            raw.len(),
            MAX_JSON_INPUT_SIZE,
        )
        .into());
    }

    let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
        format!(
            "Invalid JSON syntax: {e}. Run `domain-scan schema {command}` to see expected structure"
        )
    })?;

    let depth = json_depth(&value);
    if depth > MAX_JSON_DEPTH {
        return Err(
            format!("JSON nesting depth is {depth}, exceeds maximum of {MAX_JSON_DEPTH}").into(),
        );
    }

    serde_json::from_value(value).map_err(|e| {
        format!(
            "JSON does not match expected schema for '{command}': {e}. \
             Run `domain-scan schema {command}` to see the expected input structure"
        )
        .into()
    })
}

fn check_json_conflicts(flags: &[(&str, bool)]) -> Result<(), Box<dyn std::error::Error>> {
    let conflicts: Vec<&str> = flags
        .iter()
        .filter(|(_, set)| *set)
        .map(|(name, _)| *name)
        .collect();
    if !conflicts.is_empty() {
        return Err(format!(
            "--json and individual filter flags are mutually exclusive. \
             Remove these flags when using --json: {}",
            conflicts.join(", ")
        )
        .into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_module("domain_scan", log::LevelFilter::Debug)
            .filter_level(log::LevelFilter::Warn)
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

/// Resolve the effective output format.
///
/// If `--output` was explicitly provided, use it. Otherwise, default to JSON
/// when stdout is not a TTY (piped or redirected to a file), and table when
/// stdout is a TTY (interactive terminal).
fn resolve_format(explicit: Option<OutputFormatArg>) -> OutputFormat {
    match explicit {
        Some(f) => f.into(),
        None => {
            if std::io::stdout().is_terminal() {
                OutputFormat::Table
            } else {
                OutputFormat::Json
            }
        }
    }
}

/// Validate that --root points to an existing directory.
fn validate_root_path(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    if !cli.root.exists() {
        let err = CliError {
            code: "PATH_NOT_FOUND",
            message: format!("Root directory does not exist: {}", cli.root.display()),
            suggestion: Some(
                "Check the path and ensure the directory exists. \
                 Use --root to specify a different directory."
                    .to_string(),
            ),
        };
        let json = serde_json::to_string_pretty(&err)?;
        eprintln!("{json}");
        process::exit(1);
    }
    if !cli.root.is_dir() {
        let err = CliError {
            code: "NOT_A_DIRECTORY",
            message: format!("Root path is not a directory: {}", cli.root.display()),
            suggestion: Some(
                "The --root flag must point to a directory, not a file. \
                 Provide the parent directory instead."
                    .to_string(),
            ),
        };
        let json = serde_json::to_string_pretty(&err)?;
        eprintln!("{json}");
        process::exit(1);
    }
    Ok(())
}

/// Validate that --config points to an existing file (if provided).
fn validate_config_path(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref config_path) = cli.config {
        if !config_path.exists() {
            let err = CliError {
                code: "CONFIG_NOT_FOUND",
                message: format!("Config file does not exist: {}", config_path.display()),
                suggestion: Some(
                    "Check the path to your .domain-scan.toml file. \
                     If you don't have a config, omit the --config flag \
                     to use defaults."
                        .to_string(),
                ),
            };
            let json = serde_json::to_string_pretty(&err)?;
            eprintln!("{json}");
            process::exit(1);
        }
        if !config_path.is_file() {
            let err = CliError {
                code: "CONFIG_NOT_FILE",
                message: format!("Config path is not a file: {}", config_path.display()),
                suggestion: Some(
                    "The --config flag must point to a .domain-scan.toml file, \
                     not a directory."
                        .to_string(),
                ),
            };
            let json = serde_json::to_string_pretty(&err)?;
            eprintln!("{json}");
            process::exit(1);
        }
    }
    Ok(())
}

/// Find the workspace root by searching upward for a Cargo.toml containing [workspace].
fn find_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut dir = std::env::current_dir()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.is_file() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            return Err(
                "Could not find workspace root (Cargo.toml with [workspace]). \
                        Run from within the domain-scan repository."
                    .into(),
            );
        }
    }
}

/// Validate all user-supplied string inputs (name filters, kinds, patterns, etc.)
/// through the input_validation module before any processing. Returns a structured
/// error on invalid input (control chars, null bytes, etc.).
fn validate_string_inputs(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Helper: validate an optional string, mapping DomainScanError to CliError
    let check = |value: &Option<String>, flag: &str| -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref v) = *value {
            input_validation::validate_string_input(v).map_err(|e| {
                let err = CliError {
                    code: "INVALID_INPUT",
                    message: format!("{flag}: {e}"),
                    suggestion: Some(
                        "Remove control characters and null bytes from the input value."
                            .to_string(),
                    ),
                };
                serde_json::to_string_pretty(&err).unwrap_or_else(|_| e.to_string())
            })?;
        }
        Ok(())
    };

    // Validate --fields
    check(&cli.fields, "--fields")?;

    // Validate --json input through core module validation
    if let Some(ref json) = cli.json_input {
        input_validation::validate_json_input(json).map_err(|e| {
            let err = CliError {
                code: "INVALID_JSON",
                message: e.to_string(),
                suggestion: Some(
                    "Check JSON syntax, ensure nesting depth < 32, and size < 1 MB.".to_string(),
                ),
            };
            serde_json::to_string_pretty(&err).unwrap_or_else(|_| e.to_string())
        })?;
    }

    // Validate per-subcommand string inputs
    match &cli.command {
        Commands::Interfaces { name, .. } => {
            check(name, "--name")?;
        }
        Commands::Services { kind, name, .. } => {
            check(kind, "--kind")?;
            check(name, "--name")?;
        }
        Commands::Methods {
            owner,
            visibility,
            name,
            ..
        } => {
            check(owner, "--owner")?;
            check(visibility, "--visibility")?;
            check(name, "--name")?;
        }
        Commands::Schemas {
            framework,
            kind,
            name,
            ..
        } => {
            check(framework, "--framework")?;
            check(kind, "--kind")?;
            check(name, "--name")?;
        }
        Commands::Impls { name, .. } => {
            check(name, "name")?;
        }
        Commands::Search { query, kind, .. } => {
            check(query, "query")?;
            check(kind, "--kind")?;
        }
        Commands::Validate { rules, .. } => {
            check(rules, "--rules")?;
        }
        Commands::Prompt { focus, .. } => {
            check(focus, "--focus")?;
        }
        Commands::Schema { command, .. } => {
            check(command, "command")?;
        }
        Commands::Scan
        | Commands::Stats
        | Commands::Match { .. }
        | Commands::Cache { .. }
        | Commands::Init { .. }
        | Commands::Skills { .. } => {}
    }

    Ok(())
}

fn run(mut cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Merge positional PATH with --root. Positional takes precedence when
    // the user writes `domain-scan scan /some/path` instead of `--root`.
    if let Some(ref path) = cli.path {
        // Only override if --root was left at its default value (".").
        // If both are explicitly set, the positional wins (it's more specific).
        cli.root = path.clone();
    }

    // Validate all string inputs through input_validation before any processing
    validate_string_inputs(&cli)?;

    // Early validation for common mistakes (skip for self-test which overrides root)
    let is_self_test = matches!(&cli.command, Commands::Validate { self_test, .. } if *self_test);
    if !is_self_test {
        validate_root_path(&cli)?;
    }
    validate_config_path(&cli)?;

    let format: OutputFormat = resolve_format(cli.output);

    match &cli.command {
        Commands::Scan => {
            if cli.interactive {
                let scan_index = run_scan(&cli)?;
                run_tui(tui::TuiApp::from_entity_list(&scan_index, "Scan"))
            } else {
                cmd_scan(&cli, format)
            }
        }
        Commands::Interfaces {
            ref name,
            show_methods,
        } => {
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("--name", name.is_some()),
                    ("--show-methods", *show_methods),
                ])?;
                let input: InterfacesJsonInput = parse_json_input(json, "interfaces")?;
                cmd_interfaces(&cli, format, input.name, input.show_methods)
            } else if cli.interactive {
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
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("--kind", kind.is_some()),
                    ("--name", name.is_some()),
                    ("--show-routes", *show_routes),
                    ("--show-deps", *show_deps),
                ])?;
                let input: ServicesJsonInput = parse_json_input(json, "services")?;
                cmd_services(
                    &cli,
                    format,
                    input.kind,
                    input.name,
                    input.show_routes,
                    input.show_deps,
                )
            } else if cli.interactive {
                let scan_index = run_scan(&cli)?;
                run_tui(tui::TuiApp::from_services(&scan_index))
            } else {
                cmd_services(
                    &cli,
                    format,
                    kind.clone(),
                    name.clone(),
                    *show_routes,
                    *show_deps,
                )
            }
        }
        Commands::Methods {
            ref owner,
            is_async,
            ref visibility,
            ref name,
        } => {
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("--owner", owner.is_some()),
                    ("--async", *is_async),
                    ("--visibility", visibility.is_some()),
                    ("--name", name.is_some()),
                ])?;
                let input: MethodsJsonInput = parse_json_input(json, "methods")?;
                cmd_methods(
                    &cli,
                    format,
                    input.owner,
                    input.is_async,
                    input.visibility,
                    input.name,
                )
            } else {
                cmd_methods(
                    &cli,
                    format,
                    owner.clone(),
                    *is_async,
                    visibility.clone(),
                    name.clone(),
                )
            }
        }
        Commands::Schemas {
            ref framework,
            ref kind,
            ref name,
            show_fields,
        } => {
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("--framework", framework.is_some()),
                    ("--kind", kind.is_some()),
                    ("--name", name.is_some()),
                    ("--show-fields", *show_fields),
                ])?;
                let input: SchemasJsonInput = parse_json_input(json, "schemas")?;
                cmd_schemas(
                    &cli,
                    format,
                    input.framework,
                    input.kind,
                    input.name,
                    input.show_fields,
                )
            } else if cli.interactive {
                let scan_index = run_scan(&cli)?;
                run_tui(tui::TuiApp::from_schemas(&scan_index))
            } else {
                cmd_schemas(
                    &cli,
                    format,
                    framework.clone(),
                    kind.clone(),
                    name.clone(),
                    *show_fields,
                )
            }
        }
        Commands::Impls {
            ref name,
            all,
            show_methods,
        } => {
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("name (positional)", name.is_some()),
                    ("--all", *all),
                    ("--show-methods", *show_methods),
                ])?;
                let input: ImplsJsonInput = parse_json_input(json, "impls")?;
                cmd_impls(&cli, format, input.name, input.all, input.show_methods)
            } else {
                cmd_impls(&cli, format, name.clone(), *all, *show_methods)
            }
        }
        Commands::Search {
            ref query,
            ref kind,
            regex: _,
        } => {
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("query (positional)", query.is_some()),
                    ("--kind", kind.is_some()),
                ])?;
                let input: SearchJsonInput = parse_json_input(json, "search")?;
                cmd_search(&cli, format, input.query, input.kind)
            } else {
                let q = query.clone().ok_or(
                    "Search requires a query. Provide it as a positional argument or via --json",
                )?;
                cmd_search(&cli, format, q, kind.clone())
            }
        }
        Commands::Stats => cmd_stats(&cli, format),
        Commands::Validate {
            ref rules,
            manifest: ref manifest_path,
            strict,
            self_test,
        } => {
            if *self_test {
                // Find the workspace root by looking for Cargo.toml with [workspace]
                cli.root = find_workspace_root()?;
                // Self-test scans only Rust production source
                cli.languages = vec![LanguageArg::Rust];
                return cmd_validate_self_test(&cli, format);
            }
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("--rules", rules.is_some()),
                    ("--manifest", manifest_path.is_some()),
                    ("--strict", *strict),
                ])?;
                let input: ValidateJsonInput = parse_json_input(json, "validate")?;
                cmd_validate(
                    &cli,
                    format,
                    input.rules,
                    input.manifest.map(PathBuf::from),
                    input.strict,
                )
            } else {
                cmd_validate(&cli, format, rules.clone(), manifest_path.clone(), *strict)
            }
        }
        Commands::Match {
            manifest: ref manifest_path,
            unmatched_only,
            prompt_unmatched: _,
            agents: _,
            write_back,
            dry_run,
            fail_on_unmatched,
        } => {
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("--manifest", manifest_path.is_some()),
                    ("--unmatched-only", *unmatched_only),
                    ("--fail-on-unmatched", *fail_on_unmatched),
                    ("--write-back", *write_back),
                    ("--dry-run", *dry_run),
                ])?;
                let input: MatchJsonInput = parse_json_input(json, "match")?;
                cmd_match(
                    &cli,
                    format,
                    PathBuf::from(input.manifest),
                    input.unmatched_only,
                    input.fail_on_unmatched,
                    input.write_back,
                    input.dry_run,
                )
            } else {
                let path = manifest_path.clone().ok_or(
                    "Match requires --manifest <path>. Provide it as a flag or via --json",
                )?;
                cmd_match(
                    &cli,
                    format,
                    path,
                    *unmatched_only,
                    *fail_on_unmatched,
                    *write_back,
                    *dry_run,
                )
            }
        }
        Commands::Cache { ref action } => cmd_cache(&cli, action),
        Commands::Init {
            bootstrap,
            ref apply_manifest,
            dry_run,
            ref name,
        } => cmd_init(
            &cli,
            format,
            *bootstrap,
            apply_manifest.clone(),
            *dry_run,
            name.clone(),
        ),
        Commands::Prompt {
            agents,
            ref focus,
            include_scan,
        } => {
            if let Some(ref json) = cli.json_input {
                check_json_conflicts(&[
                    ("--agents", *agents != 5),
                    ("--focus", focus.is_some()),
                    ("--include-scan", *include_scan),
                ])?;
                let input: PromptJsonInput = parse_json_input(json, "prompt")?;
                cmd_prompt(&cli, input.agents, input.focus, input.include_scan)
            } else {
                cmd_prompt(&cli, *agents, focus.clone(), *include_scan)
            }
        }
        Commands::Schema { ref command, all } => cmd_schema(&cli, command.clone(), *all),
        Commands::Skills { ref action } => cmd_skills(&cli, action),
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
    let total_start = std::time::Instant::now();

    // Step 1: Walk
    let walk_start = std::time::Instant::now();
    let walked = walker::walk_directory(&config)?;
    let walk_ms = walk_start.elapsed().as_millis();

    if walked.is_empty() {
        if !cli.quiet {
            let mut suggestion = String::from(
                "Ensure the directory contains files with supported extensions: \
                 .ts, .tsx, .js, .jsx, .rs, .go, .py, .java, .kt, .scala, .cs, \
                 .swift, .cpp, .hpp, .php, .rb",
            );
            if !config.languages.is_empty() {
                suggestion.push_str(&format!(
                    ". You're filtering to {:?} — try removing --languages to scan all.",
                    config.languages
                ));
            }
            let err = CliError {
                code: "NO_FILES_FOUND",
                message: format!(
                    "No recognized source files found in {}",
                    config.root.display()
                ),
                suggestion: Some(suggestion),
            };
            if let Ok(json) = serde_json::to_string_pretty(&err) {
                eprintln!("{json}");
            }
        }
        return Ok(index::build_index(config.root, Vec::new(), 0, 0, 0));
    }

    if !cli.quiet {
        eprintln!("Found {} files", walked.len());
    }

    if cli.verbose {
        // Per-language file counts
        let mut lang_counts: std::collections::HashMap<domain_scan_core::ir::Language, usize> =
            std::collections::HashMap::new();
        for wf in &walked {
            *lang_counts.entry(wf.language).or_insert(0) += 1;
        }
        let mut lang_parts: Vec<String> = lang_counts
            .iter()
            .map(|(lang, count)| format!("{lang:?}: {count}"))
            .collect();
        lang_parts.sort();
        eprintln!(
            "[verbose] walk: {}ms, files by language: {}",
            walk_ms,
            lang_parts.join(", ")
        );
    }

    // Step 2: Optional cache
    let cache_start = std::time::Instant::now();
    let disk_cache = if config.cache_enabled {
        let c = cache::Cache::new(config.cache_dir.clone(), 100);
        let _ = c.load_from_disk();
        Some(c)
    } else {
        None
    };
    let cache_load_ms = cache_start.elapsed().as_millis();

    if cli.verbose && config.cache_enabled {
        let entries = disk_cache.as_ref().map_or(0, |c| c.len());
        eprintln!(
            "[verbose] cache load: {}ms, {} entries",
            cache_load_ms, entries
        );
    }

    // Step 3: Parse + Extract (parallel via rayon)
    let parse_start = std::time::Instant::now();
    let cache_hits = AtomicUsize::new(0);
    let cache_misses = AtomicUsize::new(0);
    let build_status = config.build_status_override.unwrap_or(BuildStatus::Built);

    use rayon::prelude::*;
    let ir_results: Vec<
        Result<domain_scan_core::ir::IrFile, Box<dyn std::error::Error + Send + Sync>>,
    > = walked
        .par_iter()
        .map(|walked_file| {
            // Try cache first
            let source_bytes = std::fs::read(&walked_file.path)?;
            let hash = domain_scan_core::content_hash(&source_bytes);

            if let Some(ref c) = disk_cache {
                if let Some(cached_ir) = c.get(&hash) {
                    cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(cached_ir);
                }
            }

            cache_misses.fetch_add(1, Ordering::Relaxed);

            let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)?;
            let ir = query_engine::extract(
                &tree,
                &source,
                &walked_file.path,
                walked_file.language,
                build_status,
            )?;

            // Store in cache (DashMap is thread-safe)
            if let Some(ref c) = disk_cache {
                let _ = c.insert(hash, ir.clone());
            }

            Ok(ir)
        })
        .collect();

    // Collect results, propagate first error
    let mut ir_files = Vec::with_capacity(ir_results.len());
    for result in ir_results {
        ir_files.push(result.map_err(|e| -> Box<dyn std::error::Error> { e })?);
    }

    let parse_ms = parse_start.elapsed().as_millis();
    let cache_hits = cache_hits.load(Ordering::Relaxed);
    let cache_misses = cache_misses.load(Ordering::Relaxed);

    if cli.verbose {
        let files_per_sec = if parse_ms > 0 {
            (walked.len() as f64 / parse_ms as f64 * 1000.0) as u64
        } else {
            0
        };
        eprintln!(
            "[verbose] parse+extract: {}ms, {} files ({} cached, {} parsed), ~{} files/sec",
            parse_ms,
            walked.len(),
            cache_hits,
            cache_misses,
            files_per_sec,
        );
    }

    // Step 4: Build Index
    let index_start = std::time::Instant::now();
    let duration_ms = total_start.elapsed().as_millis() as u64;
    let scan_index =
        index::build_index(config.root, ir_files, duration_ms, cache_hits, cache_misses);
    let index_ms = index_start.elapsed().as_millis();

    if cli.verbose {
        eprintln!(
            "[verbose] index build: {}ms ({} interfaces, {} classes, {} services, {} schemas)",
            index_ms,
            scan_index.stats.total_interfaces,
            scan_index.stats.total_classes,
            scan_index.stats.total_services,
            scan_index.stats.total_schemas,
        );
        let total_ms = total_start.elapsed().as_millis();
        eprintln!("[verbose] total: {}ms", total_ms);
    }

    Ok(scan_index)
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

/// Emit JSON output, applying field mask if `--fields` is set.
///
/// If fields is `None` or output format is not JSON, this is equivalent to
/// serializing to pretty JSON and calling `emit`. When fields is set, the JSON
/// is post-processed to include only the requested fields.
fn emit_json<T: serde::Serialize>(
    cli: &Cli,
    value: &T,
    format: OutputFormat,
    schema_command: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if format != OutputFormat::Json {
        // --fields is only for JSON output; caller handles table/compact separately
        return Ok(());
    }

    let json_value = serde_json::to_value(value)?;

    let content = if let Some(ref fields) = cli.fields {
        // Validate fields against schema if we know the command
        if let Some(cmd) = schema_command {
            if let Some(cmd_schema) = domain_scan_core::schema::schema_for_command(cmd) {
                let mask = domain_scan_core::field_mask::FieldMask::parse(fields)?;
                let invalid = domain_scan_core::field_mask::validate_fields_against_schema(
                    &mask,
                    &cmd_schema.output,
                );
                if !invalid.is_empty() {
                    let valid = domain_scan_core::field_mask::extract_valid_fields_from_schema(
                        &cmd_schema.output,
                    );
                    let valid_list: Vec<&str> = valid.iter().map(|s| s.as_str()).collect();
                    let err = CliError {
                        code: "INVALID_FIELDS",
                        message: format!("Unknown field(s): {}", invalid.join(", ")),
                        suggestion: Some(format!(
                            "Valid fields for '{}': {}",
                            cmd,
                            valid_list.join(", ")
                        )),
                    };
                    let json = serde_json::to_string_pretty(&err)?;
                    eprintln!("{json}");
                    std::process::exit(1);
                }
            }
        }
        domain_scan_core::field_mask::apply_field_mask(&json_value, fields)?
    } else {
        serde_json::to_string_pretty(&json_value)?
    };

    emit(cli, &content)
}

/// Validate `--fields` against the schema for a command, returning the parsed mask if valid.
///
/// Returns `None` if `--fields` is not set. Exits with a structured error if fields are invalid.
fn validate_fields_mask(
    cli: &Cli,
    schema_command: Option<&str>,
) -> Result<Option<domain_scan_core::field_mask::FieldMask>, Box<dyn std::error::Error>> {
    let fields = match cli.fields {
        Some(ref f) => f,
        None => return Ok(None),
    };
    let parsed = domain_scan_core::field_mask::FieldMask::parse(fields)?;
    if let Some(cmd) = schema_command {
        if let Some(cmd_schema) = domain_scan_core::schema::schema_for_command(cmd) {
            let invalid = domain_scan_core::field_mask::validate_fields_against_schema(
                &parsed,
                &cmd_schema.output,
            );
            if !invalid.is_empty() {
                let valid = domain_scan_core::field_mask::extract_valid_fields_from_schema(
                    &cmd_schema.output,
                );
                let valid_list: Vec<&str> = valid.iter().map(|s| s.as_str()).collect();
                let err = CliError {
                    code: "INVALID_FIELDS",
                    message: format!("Unknown field(s): {}", invalid.join(", ")),
                    suggestion: Some(format!(
                        "Valid fields for '{}': {}",
                        cmd,
                        valid_list.join(", ")
                    )),
                };
                let json = serde_json::to_string_pretty(&err)?;
                eprintln!("{json}");
                std::process::exit(1);
            }
        }
    }
    Ok(Some(parsed))
}

/// Emit NDJSON output (one JSON object per line) for a slice of serializable items.
///
/// If `--fields` is set, the field mask is applied to each individual entity.
/// Each entity is serialized as compact JSON (no pretty-printing) on its own line.
fn emit_ndjson<T: serde::Serialize>(
    cli: &Cli,
    items: &[T],
    schema_command: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mask = validate_fields_mask(cli, schema_command)?;

    let mut output = String::new();
    for item in items {
        let json_value = serde_json::to_value(item)?;
        let filtered = if let Some(ref m) = mask {
            m.apply(&json_value)
        } else {
            json_value
        };
        let line = serde_json::to_string(&filtered)?;
        output.push_str(&line);
        output.push('\n');
    }

    emit(cli, &output)
}

/// Emit NDJSON for a slice of references.
fn emit_ndjson_refs<T: serde::Serialize>(
    cli: &Cli,
    items: &[&T],
    schema_command: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mask = validate_fields_mask(cli, schema_command)?;

    let mut output = String::new();
    for item in items {
        let json_value = serde_json::to_value(item)?;
        let filtered = if let Some(ref m) = mask {
            m.apply(&json_value)
        } else {
            json_value
        };
        let line = serde_json::to_string(&filtered)?;
        output.push_str(&line);
        output.push('\n');
    }

    emit(cli, &output)
}

// ---------------------------------------------------------------------------
// Subcommand: scan
// ---------------------------------------------------------------------------

fn cmd_scan(cli: &Cli, format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;
    if format == OutputFormat::Json {
        return emit_json(cli, &scan_index, format, Some("scan"));
    }
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

    if cli.page_all {
        return emit_ndjson(cli, &interfaces, Some("interfaces"));
    }

    match format {
        OutputFormat::Json => {
            return emit_json(cli, &interfaces, format, Some("interfaces"));
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.set_header(vec![
                "Language", "Kind", "Name", "Methods", "Extends", "File",
            ]);
            for iface in &interfaces {
                let extends = iface.extends.join(", ");
                let file_loc = format!("{}:{}", iface.file.display(), iface.span.start_line);
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

    if cli.page_all {
        return emit_ndjson(cli, &services, Some("services"));
    }

    match format {
        OutputFormat::Json => {
            return emit_json(cli, &services, format, Some("services"));
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            let mut headers = vec!["Kind", "Name", "Methods", "Routes", "File"];
            if show_deps {
                headers.push("Dependencies");
            }
            table.set_header(headers);
            for svc in &services {
                let file_loc = format!("{}:{}", svc.file.display(), svc.span.start_line);
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

    if cli.page_all {
        return emit_ndjson_refs(cli, &methods, Some("methods"));
    }

    match format {
        OutputFormat::Json => {
            return emit_json(cli, &methods, format, Some("methods"));
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.set_header(vec![
                "Name",
                "Owner",
                "Async",
                "Visibility",
                "Return",
                "File",
            ]);
            for m in &methods {
                let owner_name = m.owner.as_deref().unwrap_or("-");
                let ret = m.return_type.as_deref().unwrap_or("-");
                let file_loc = format!("{}:{}", m.file.display(), m.span.start_line);
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
                    out.push_str(&format!("method:{owner_name}.{async_str}{}\n", m.name));
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

    if cli.page_all {
        return emit_ndjson(cli, &schemas, Some("schemas"));
    }

    match format {
        OutputFormat::Json => {
            return emit_json(cli, &schemas, format, Some("schemas"));
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.set_header(vec!["Framework", "Kind", "Name", "Fields", "File"]);
            for s in &schemas {
                let file_loc = format!("{}:{}", s.file.display(), s.span.start_line);
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

        if cli.page_all {
            return emit_ndjson_refs(cli, &all_impls, Some("impls"));
        }

        match format {
            OutputFormat::Json => {
                return emit_json(cli, &all_impls, format, Some("impls"));
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
                    let file_loc = format!("{}:{}", imp.file.display(), imp.span.start_line);
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

        if cli.page_all {
            return emit_ndjson(cli, &impls, Some("impls"));
        }

        match format {
            OutputFormat::Json => {
                return emit_json(cli, &impls, format, Some("impls"));
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
                    let file_loc = format!("{}:{}", imp.file.display(), imp.span.start_line);
                    let mut row = vec![imp.target.clone(), imp.methods.len().to_string(), file_loc];
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

    if cli.page_all {
        return emit_ndjson(cli, &summaries, Some("search"));
    }

    match format {
        OutputFormat::Json => {
            return emit_json(cli, &summaries, format, Some("search"));
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
                out.push_str(&format!("{:?}:{} [{}]\n", s.kind, s.name, s.file.display(),));
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
            return emit_json(cli, &scan_index.stats, format, Some("stats"));
        }
        OutputFormat::Table | OutputFormat::Compact => {
            let stats = &scan_index.stats;
            let mut out = String::new();
            out.push_str("Scan Statistics\n");
            out.push_str(&format!("Root:            {}\n", scan_index.root.display()));
            out.push_str(&format!("Files scanned:   {}\n", stats.total_files));

            let mut langs: Vec<_> = stats.files_by_language.iter().collect();
            langs.sort_by(|a, b| b.1.cmp(a.1));
            let lang_str: Vec<_> = langs.iter().map(|(l, c)| format!("{l} ({c})")).collect();
            out.push_str(&format!("Languages:       {}\n", lang_str.join(", ")));

            out.push('\n');
            out.push_str(&format!("Interfaces:      {}\n", stats.total_interfaces));
            out.push_str(&format!("Services:        {}\n", stats.total_services));
            out.push_str(&format!("Classes:         {}\n", stats.total_classes));
            out.push_str(&format!("Methods:         {}\n", stats.total_methods));
            out.push_str(&format!("Functions:       {}\n", stats.total_functions));
            out.push_str(&format!("Schemas:         {}\n", stats.total_schemas));
            out.push_str(&format!("Type aliases:    {}\n", stats.total_type_aliases));
            out.push_str(&format!(
                "Implementations: {}\n",
                stats.total_implementations
            ));

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

/// Self-test: scan domain-scan's own Rust source (excluding test fixtures) and validate.
fn cmd_validate_self_test(
    cli: &Cli,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = build_scan_config(cli);
    // Exclude test fixtures and benches — scan only production source
    config.exclude = vec![
        "**/tests/fixtures/**".to_string(),
        "**/tests/adversarial/**".to_string(),
    ];

    if !cli.quiet {
        eprintln!(
            "Self-test: scanning domain-scan source at {}",
            config.root.display()
        );
    }

    let start = std::time::Instant::now();
    let walked = walker::walk_directory(&config)?;

    if walked.is_empty() {
        return Err("Self-test: no Rust source files found. \
                     Are you in the domain-scan repository?"
            .into());
    }

    if !cli.quiet {
        eprintln!("Found {} Rust source files", walked.len());
    }

    let mut ir_files = Vec::new();
    for walked_file in &walked {
        let build_status = config.build_status_override.unwrap_or(BuildStatus::Built);
        let (tree, source) = parser::parse_file(&walked_file.path, walked_file.language)?;
        let ir = query_engine::extract(
            &tree,
            &source,
            &walked_file.path,
            walked_file.language,
            build_status,
        )?;
        ir_files.push(ir);
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let scan_index = index::build_index(config.root, ir_files, duration_ms, 0, 0);
    let result = validate::validate(&scan_index);

    match format {
        OutputFormat::Json => {
            emit_json(cli, &result, format, Some("validate"))?;
        }
        OutputFormat::Table | OutputFormat::Compact => {
            let mut out = String::new();
            out.push_str(&format!(
                "Self-test: {} files scanned in {}ms\n",
                scan_index.stats.total_files, duration_ms,
            ));
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

    if result.fail_count > 0 {
        process::exit(1);
    }
    Ok(())
}

fn cmd_validate(
    cli: &Cli,
    format: OutputFormat,
    rules: Option<String>,
    manifest_path: Option<PathBuf>,
    strict: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref manifest_path) = manifest_path {
        if rules.is_some() {
            return Err(
                "validate --manifest cannot be combined with --rules. \
                 Use validate without --manifest for entity rules, or run match separately for coverage."
                    .into(),
            );
        }
        let system_manifest = manifest::parse_system_manifest_file(manifest_path)?;
        let scan_index = run_scan(cli)?;
        let violations = manifest::validate_system_manifest(&system_manifest);
        let match_result = manifest::match_entities(&scan_index, &system_manifest.as_manifest());
        let report = manifest::ManifestValidationReport {
            manifest_path: manifest_path.display().to_string(),
            domains: system_manifest.domains.len(),
            subsystems: system_manifest.subsystems.len(),
            connections: system_manifest.connections.len(),
            validation_errors: violations.len(),
            violations,
            coverage_percent: match_result.coverage_percent,
            matched: match_result.matched.len(),
            unmatched: match_result.unmatched.len(),
        };

        match format {
            OutputFormat::Json => {
                emit_json(cli, &report, format, None)?;
            }
            OutputFormat::Table | OutputFormat::Compact => {
                let mut out = format!(
                    "Manifest: {}\n\
                     Domains: {} | Subsystems: {} | Connections: {}\n\
                     Validation errors: {}\n\
                     Coverage: {:.1}% ({} matched, {} unmatched)\n",
                    report.manifest_path,
                    report.domains,
                    report.subsystems,
                    report.connections,
                    report.validation_errors,
                    report.coverage_percent,
                    report.matched,
                    report.unmatched,
                );
                if !report.violations.is_empty() {
                    out.push_str("\nValidation errors:\n");
                    for v in &report.violations {
                        out.push_str(&format!(
                            "  {} / {}: '{}' (expected {})\n",
                            v.subsystem_id, v.field, v.value, v.expected,
                        ));
                    }
                }
                emit(cli, &out)?;
            }
        }

        if report.validation_errors > 0 {
            process::exit(1);
        }
        return Ok(());
    }

    let scan_index = run_scan(cli)?;

    let result = if let Some(ref rule_list) = rules {
        let rule_names: Vec<&str> = rule_list.split(',').map(|s| s.trim()).collect();
        validate::validate_rules(&scan_index, &rule_names)
    } else {
        validate::validate(&scan_index)
    };

    match format {
        OutputFormat::Json => {
            emit_json(cli, &result, format, Some("validate"))?;
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
    write_back: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;
    let manifest_data = manifest::parse_manifest_file(&manifest_path)?;
    let result = manifest::match_entities(&scan_index, &manifest_data);

    // --write-back (with optional --dry-run)
    if write_back {
        // Try SystemManifest first (preserves meta/domains/connections)
        let serialized =
            if let Ok(mut sys_manifest) = manifest::parse_system_manifest_file(&manifest_path) {
                manifest::write_back_system(&mut sys_manifest, &result, &scan_index);
                manifest::serialize_system_manifest(&sys_manifest)?
            } else {
                // Fallback: plain Manifest (subsystems only)
                let mut updated_manifest = manifest_data.clone();
                manifest::write_back(&mut updated_manifest, &result, &scan_index);
                manifest::serialize_manifest(&updated_manifest)?
            };

        if dry_run {
            // Show what would be written as structured JSON
            let actions = vec![serde_json::json!({
                "action": "write",
                "target": manifest_path.display().to_string(),
                "reason": format!(
                    "write-back would add {} matched entities to manifest",
                    result.matched.len()
                ),
                "preview": serialized,
            })];
            let json = serde_json::to_string_pretty(&actions)?;
            eprintln!(
                "Dry run: would write back {} matched entities to {}",
                result.matched.len(),
                manifest_path.display()
            );
            print!("{json}");
            return Ok(());
        }

        // Actually write the manifest
        std::fs::write(&manifest_path, serialized.as_bytes())
            .map_err(|e| format!("failed to write manifest {}: {e}", manifest_path.display()))?;
        eprintln!(
            "Wrote back {} matched entities to {}",
            result.matched.len(),
            manifest_path.display()
        );
    }

    match format {
        OutputFormat::Json => {
            if unmatched_only {
                return emit_json(cli, &result.unmatched, format, Some("match"));
            } else {
                return emit_json(cli, &result, format, Some("match"));
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
        CacheAction::Clear { dry_run } => {
            if dry_run {
                let actions = c.dry_run_clear();
                let json = serde_json::to_string_pretty(&actions)?;
                eprintln!("Dry run: would delete {} cache entries.", actions.len());
                print!("{json}");
            } else {
                c.clear()?;
                eprintln!("Cache cleared.");
            }
        }
        CacheAction::Prune { dry_run } => {
            if dry_run {
                let actions = c.dry_run_prune();
                let json = serde_json::to_string_pretty(&actions)?;
                eprintln!(
                    "Dry run: would prune {} stale cache entries.",
                    actions.len()
                );
                print!("{json}");
            } else {
                let pruned = c.prune();
                eprintln!("Pruned {pruned} stale cache entries.");
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: prompt
// ---------------------------------------------------------------------------

fn cmd_prompt(
    cli: &Cli,
    agents: usize,
    focus: Option<String>,
    include_scan: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_index = run_scan(cli)?;

    let config = PromptConfig {
        agents,
        focus,
        include_scan,
    };

    let prompt_text = prompt::generate_prompt(&scan_index, &config)?;
    emit(cli, &prompt_text)
}

// ---------------------------------------------------------------------------
// Subcommand: schema
// ---------------------------------------------------------------------------

fn cmd_schema(
    cli: &Cli,
    command: Option<String>,
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use domain_scan_core::schema;

    if all {
        let all_schemas = schema::all_schemas();
        let json = serde_json::to_string_pretty(&all_schemas)?;
        emit(cli, &json)
    } else if let Some(ref cmd_name) = command {
        match schema::schema_for_command(cmd_name) {
            Some(cmd_schema) => {
                let json = serde_json::to_string_pretty(&cmd_schema)?;
                emit(cli, &json)
            }
            None => {
                let names = schema::all_command_names().join(", ");
                Err(format!("Unknown command: '{cmd_name}'. Valid commands: {names}").into())
            }
        }
    } else {
        // No command specified and --all not set: list available commands
        let names = schema::all_command_names();
        let json = serde_json::to_string_pretty(&names)?;
        emit(cli, &json)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Embedded agent skill files
// ---------------------------------------------------------------------------

const EMBEDDED_SKILLS: &[(&str, &str)] = &[
    (
        "domain-scan-cli",
        include_str!("../../../skills/domain-scan-cli.md"),
    ),
    (
        "domain-scan-scan",
        include_str!("../../../skills/domain-scan-scan.md"),
    ),
    (
        "domain-scan-query",
        include_str!("../../../skills/domain-scan-query.md"),
    ),
    (
        "domain-scan-validate",
        include_str!("../../../skills/domain-scan-validate.md"),
    ),
    (
        "domain-scan-match",
        include_str!("../../../skills/domain-scan-match.md"),
    ),
    (
        "domain-scan-prompt",
        include_str!("../../../skills/domain-scan-prompt.md"),
    ),
    (
        "domain-scan-cache",
        include_str!("../../../skills/domain-scan-cache.md"),
    ),
    (
        "domain-scan-safety",
        include_str!("../../../skills/domain-scan-safety.md"),
    ),
    (
        "domain-scan-schema",
        include_str!("../../../skills/domain-scan-schema.md"),
    ),
    (
        "domain-scan-init",
        include_str!("../../../skills/domain-scan-init.md"),
    ),
    (
        "domain-scan-tube-map",
        include_str!("../../../skills/domain-scan-tube-map.md"),
    ),
];

// ---------------------------------------------------------------------------
// Subcommand: skills
// ---------------------------------------------------------------------------

fn cmd_skills(cli: &Cli, action: &SkillsAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        SkillsAction::List => {
            let names: Vec<&str> = EMBEDDED_SKILLS.iter().map(|(name, _)| *name).collect();
            let json = serde_json::to_string_pretty(&names)?;
            emit(cli, &json)
        }
        SkillsAction::Show { name } => {
            let skill = EMBEDDED_SKILLS
                .iter()
                .find(|(n, _)| *n == name.as_str())
                .map(|(_, content)| *content);
            match skill {
                Some(content) => emit(cli, content),
                None => {
                    let names: Vec<&str> = EMBEDDED_SKILLS.iter().map(|(n, _)| *n).collect();
                    Err(format!(
                        "Unknown skill: '{}'. Available skills: {}",
                        name,
                        names.join(", ")
                    )
                    .into())
                }
            }
        }
        SkillsAction::Dump => {
            let mut output = String::new();
            for (name, content) in EMBEDDED_SKILLS {
                output.push_str(&format!("# === {} ===\n\n", name));
                output.push_str(content);
                output.push_str("\n\n");
            }
            emit(cli, &output)
        }
        SkillsAction::Install {
            claude_code,
            codex,
            dir,
        } => cmd_skills_install(cli, *claude_code, *codex, dir.clone()),
    }
}

fn cmd_skills_install(
    cli: &Cli,
    claude_code: bool,
    codex: bool,
    custom_dir: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut targets: Vec<PathBuf> = Vec::new();

    if claude_code {
        targets.push(cli.root.join(".claude").join("skills"));
    }
    if codex {
        targets.push(cli.root.join(".codex").join("skills"));
    }
    if let Some(ref dir) = custom_dir {
        targets.push(dir.clone());
    }

    if targets.is_empty() {
        return Err(
            "Specify at least one install target: --claude-code, --codex, or --dir <PATH>".into(),
        );
    }

    for target_dir in &targets {
        std::fs::create_dir_all(target_dir)
            .map_err(|e| format!("Failed to create directory {}: {e}", target_dir.display()))?;

        for (name, content) in EMBEDDED_SKILLS {
            let file_path = target_dir.join(format!("{name}.md"));
            std::fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write {}: {e}", file_path.display()))?;
        }

        if !cli.quiet {
            eprintln!(
                "Installed {} skills to {}",
                EMBEDDED_SKILLS.len(),
                target_dir.display()
            );
        }

        // Auto-add to .gitignore
        update_gitignore(&cli.root, target_dir)?;
    }

    Ok(())
}

/// Add the skills directory to .gitignore if not already present.
fn update_gitignore(root: &Path, skills_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = root.join(".gitignore");
    let relative = skills_dir.strip_prefix(root).unwrap_or(skills_dir);
    let entry = format!("{}/", relative.display());

    let existing = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    // Check if already present (exact line match)
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed == entry || trimmed == entry.trim_end_matches('/') {
            return Ok(());
        }
    }

    // Append to .gitignore
    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&entry);
    content.push('\n');
    std::fs::write(&gitignore_path, content)?;

    Ok(())
}

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

// ---------------------------------------------------------------------------
// Subcommand: init
// ---------------------------------------------------------------------------

fn cmd_init(
    cli: &Cli,
    format: OutputFormat,
    bootstrap: bool,
    apply_manifest: Option<PathBuf>,
    dry_run: bool,
    project_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref manifest_path) = apply_manifest {
        // --apply-manifest: load, validate, and optionally write
        return cmd_init_apply(cli, format, manifest_path, dry_run);
    }

    if !bootstrap {
        return Err("init requires --bootstrap or --apply-manifest. \
             Use --bootstrap to infer domains/subsystems from the codebase, \
             or --apply-manifest <path> to validate an existing manifest."
            .into());
    }

    // --bootstrap: scan codebase, infer manifest
    let scan_index = run_scan(cli)?;

    let options = manifest_builder::BootstrapOptions {
        project_name,
        ..Default::default()
    };

    let system_manifest = manifest_builder::bootstrap_manifest(&scan_index, &options);
    let json_output = manifest_builder::serialize_manifest(&system_manifest)?;

    if dry_run {
        // Show what would be written
        let output_path = cli
            .out
            .as_deref()
            .unwrap_or(std::path::Path::new("system.json"));
        if !cli.quiet {
            eprintln!(
                "Dry run: would write manifest with {} domains, {} subsystems, {} connections to {}",
                system_manifest.domains.len(),
                system_manifest.subsystems.len(),
                system_manifest.connections.len(),
                output_path.display(),
            );
        }

        // Optionally run match to show coverage
        let simple_manifest = system_manifest.as_manifest();
        let match_result = manifest::match_entities(&scan_index, &simple_manifest);
        if !cli.quiet {
            eprintln!(
                "Coverage: {:.1}% ({} matched, {} unmatched)",
                match_result.coverage_percent,
                match_result.matched.len(),
                match_result.unmatched.len(),
            );
        }

        // In dry-run mode, output the manifest to stdout for inspection
        match format {
            OutputFormat::Json => return emit_json(cli, &system_manifest, format, None),
            OutputFormat::Table | OutputFormat::Compact => {
                let summary = format!(
                    "Domains: {}\nSubsystems: {}\nConnections: {}\nCoverage: {:.1}%\n",
                    system_manifest.domains.len(),
                    system_manifest.subsystems.len(),
                    system_manifest.connections.len(),
                    match_result.coverage_percent,
                );
                emit(cli, &summary)?;
            }
        }
        return Ok(());
    }

    // Write the manifest to file
    if let Some(ref out_path) = cli.out {
        std::fs::write(out_path, json_output.as_bytes())
            .map_err(|e| format!("failed to write manifest to {}: {e}", out_path.display()))?;
        if !cli.quiet {
            eprintln!(
                "Wrote manifest with {} domains, {} subsystems, {} connections to {}",
                system_manifest.domains.len(),
                system_manifest.subsystems.len(),
                system_manifest.connections.len(),
                out_path.display(),
            );
        }
    } else {
        // No -o flag: write to stdout
        print!("{json_output}");
    }

    Ok(())
}

fn cmd_init_apply(
    cli: &Cli,
    format: OutputFormat,
    manifest_path: &Path,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the manifest
    let system_manifest = manifest::parse_system_manifest_file(manifest_path)?;

    // Validate
    let simple_manifest = system_manifest.as_manifest();
    let violations = manifest::validate_system_manifest(&system_manifest);

    // Run scan and match
    let scan_index = run_scan(cli)?;
    let match_result = manifest::match_entities(&scan_index, &simple_manifest);
    let report = manifest::ManifestValidationReport {
        manifest_path: manifest_path.display().to_string(),
        domains: system_manifest.domains.len(),
        subsystems: system_manifest.subsystems.len(),
        connections: system_manifest.connections.len(),
        validation_errors: violations.len(),
        violations,
        coverage_percent: match_result.coverage_percent,
        matched: match_result.matched.len(),
        unmatched: match_result.unmatched.len(),
    };

    if dry_run {
        // Show validation + coverage without writing
        match format {
            OutputFormat::Json => {
                emit_json(cli, &report, format, None)?;
            }
            OutputFormat::Table | OutputFormat::Compact => {
                let mut out = format!(
                    "Manifest: {}\n\
                     Domains: {} | Subsystems: {} | Connections: {}\n\
                     Validation errors: {}\n\
                     Coverage: {:.1}% ({} matched, {} unmatched)\n",
                    report.manifest_path,
                    report.domains,
                    report.subsystems,
                    report.connections,
                    report.validation_errors,
                    report.coverage_percent,
                    report.matched,
                    report.unmatched,
                );
                if !report.violations.is_empty() {
                    out.push_str("\nValidation errors:\n");
                    for v in &report.violations {
                        out.push_str(&format!(
                            "  {} / {}: '{}' (expected {})\n",
                            v.subsystem_id, v.field, v.value, v.expected,
                        ));
                    }
                }
                emit(cli, &out)?;
            }
        }
        return Ok(());
    }

    // --apply-manifest without --dry-run: write the file back (re-serialized)
    let serialized = serde_json::to_string_pretty(&system_manifest)?;
    if let Some(ref out_path) = cli.out {
        std::fs::write(out_path, serialized.as_bytes())
            .map_err(|e| format!("failed to write manifest to {}: {e}", out_path.display()))?;
    } else {
        // Overwrite original file
        std::fs::write(manifest_path, serialized.as_bytes()).map_err(|e| {
            format!(
                "failed to write manifest to {}: {e}",
                manifest_path.display()
            )
        })?;
    }

    if !cli.quiet {
        eprintln!(
            "Applied manifest: {} domains, {} subsystems, {} connections. Coverage: {:.1}%",
            system_manifest.domains.len(),
            system_manifest.subsystems.len(),
            system_manifest.connections.len(),
            match_result.coverage_percent,
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn resolve_format_explicit_json() {
        assert_eq!(
            resolve_format(Some(OutputFormatArg::Json)),
            OutputFormat::Json
        );
    }

    #[test]
    fn resolve_format_explicit_table() {
        assert_eq!(
            resolve_format(Some(OutputFormatArg::Table)),
            OutputFormat::Table
        );
    }

    #[test]
    fn resolve_format_explicit_compact() {
        assert_eq!(
            resolve_format(Some(OutputFormatArg::Compact)),
            OutputFormat::Compact
        );
    }

    #[test]
    fn resolve_format_none_uses_tty_detection() {
        // When no explicit format, the result depends on whether stdout is a TTY.
        // In test context, stdout is typically not a TTY (piped), so JSON is expected.
        let format = resolve_format(None);
        if std::io::stdout().is_terminal() {
            assert_eq!(format, OutputFormat::Table);
        } else {
            assert_eq!(format, OutputFormat::Json);
        }
    }

    // -----------------------------------------------------------------------
    // B.9: Positional PATH argument accepted
    // -----------------------------------------------------------------------

    #[test]
    fn positional_path_accepted() {
        // `domain-scan scan /some/path` should parse successfully
        let cli = Cli::try_parse_from(["domain-scan", "scan", "/some/path"]);
        assert!(cli.is_ok(), "Positional path should be accepted");
        let cli = cli.expect("parse should succeed");
        assert_eq!(cli.path, Some(PathBuf::from("/some/path")));
    }

    #[test]
    fn positional_dot_accepted() {
        // `domain-scan scan .` should parse successfully
        let cli = Cli::try_parse_from(["domain-scan", "scan", "."]);
        assert!(cli.is_ok(), "Positional '.' should be accepted");
        let cli = cli.expect("parse should succeed");
        assert_eq!(cli.path, Some(PathBuf::from(".")));
    }

    #[test]
    fn default_root_without_positional() {
        // `domain-scan scan` should still default root to "."
        let cli = Cli::try_parse_from(["domain-scan", "scan"]);
        assert!(cli.is_ok());
        let cli = cli.expect("parse should succeed");
        assert_eq!(cli.root, PathBuf::from("."));
        assert_eq!(cli.path, None);
    }

    #[test]
    fn explicit_root_flag_still_works() {
        // `domain-scan scan --root /some/path` should still work
        let cli = Cli::try_parse_from(["domain-scan", "scan", "--root", "/some/path"]);
        assert!(cli.is_ok());
        let cli = cli.expect("parse should succeed");
        assert_eq!(cli.root, PathBuf::from("/some/path"));
    }
}
