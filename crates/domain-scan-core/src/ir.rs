use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Build Status & Confidence
// ---------------------------------------------------------------------------

/// Build status of the module this file belongs to.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    /// Compiles/runs. Source code is truth.
    Built,
    /// Never built or stale artifacts. Source is best guess.
    Unbuilt,
    /// Build fails. Partial truth.
    Error,
    /// Active refactor. Source unreliable, needs LLM reconciliation.
    Rebuild,
}

impl BuildStatus {
    /// Derive confidence from build status.
    pub fn confidence(&self) -> Confidence {
        match self {
            Self::Built => Confidence::High,
            Self::Error => Confidence::Medium,
            Self::Unbuilt | Self::Rebuild => Confidence::Low,
        }
    }
}

impl std::fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Built => write!(f, "Built"),
            Self::Unbuilt => write!(f, "Unbuilt"),
            Self::Error => write!(f, "Error"),
            Self::Rebuild => write!(f, "Rebuild"),
        }
    }
}

/// Confidence level for extracted entities.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// From a Built module. Tree-sitter extraction is authoritative.
    High,
    /// From an Error module. Syntax parsed but semantics may be incomplete.
    Medium,
    /// From Unbuilt/Rebuild module. Best guess, needs LLM enrichment.
    Low,
}

// ---------------------------------------------------------------------------
// Language
// ---------------------------------------------------------------------------

/// Supported programming languages.
/// TypeScript also covers JavaScript (.js, .jsx, .mjs, .cjs).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
pub enum Language {
    TypeScript,
    Python,
    Rust,
    Go,
    Java,
    Kotlin,
    CSharp,
    Swift,
    PHP,
    Ruby,
    Scala,
    Cpp,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeScript => write!(f, "TypeScript"),
            Self::Python => write!(f, "Python"),
            Self::Rust => write!(f, "Rust"),
            Self::Go => write!(f, "Go"),
            Self::Java => write!(f, "Java"),
            Self::Kotlin => write!(f, "Kotlin"),
            Self::CSharp => write!(f, "C#"),
            Self::Swift => write!(f, "Swift"),
            Self::PHP => write!(f, "PHP"),
            Self::Ruby => write!(f, "Ruby"),
            Self::Scala => write!(f, "Scala"),
            Self::Cpp => write!(f, "C++"),
        }
    }
}

// ---------------------------------------------------------------------------
// Span & Visibility
// ---------------------------------------------------------------------------

/// Source location span.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_range: (usize, usize),
}

/// Visibility modifier.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    /// C# internal, Kotlin internal.
    Internal,
    /// Rust pub(crate).
    Crate,
    /// Language doesn't have visibility modifiers (Go, Python).
    Unknown,
}

// ---------------------------------------------------------------------------
// Interface / Trait / Protocol
// ---------------------------------------------------------------------------

/// An interface / trait / protocol definition.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct InterfaceDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub visibility: Visibility,
    pub generics: Vec<String>,
    pub extends: Vec<String>,
    pub methods: Vec<MethodSignature>,
    pub properties: Vec<PropertyDef>,
    pub language_kind: InterfaceKind,
    pub decorators: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    /// TS/Java/Go/C#/Kotlin/PHP
    Interface,
    /// Rust/Scala/PHP
    Trait,
    /// Swift/Python (typing.Protocol)
    Protocol,
    /// Python ABC, Java abstract class
    AbstractClass,
    /// C++ class with pure virtual methods
    PureVirtual,
    /// Ruby module used as mixin
    Module,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// A service definition (framework-specific).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ServiceDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub kind: ServiceKind,
    pub methods: Vec<MethodDef>,
    pub dependencies: Vec<String>,
    pub decorators: Vec<String>,
    pub routes: Vec<RouteDef>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    HttpController,
    GrpcService,
    GraphqlResolver,
    Worker,
    Microservice,
    CliCommand,
    EventHandler,
    Middleware,
    Repository,
    Custom(String),
}

// ---------------------------------------------------------------------------
// Method & Function
// ---------------------------------------------------------------------------

/// A concrete method (with body).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct MethodDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub visibility: Visibility,
    pub is_async: bool,
    pub is_static: bool,
    pub is_generator: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub decorators: Vec<String>,
    /// Class/struct/impl this belongs to.
    pub owner: Option<String>,
    /// Which interface method this implements.
    pub implements: Option<String>,
}

/// A method signature (no body, in interface/trait).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct MethodSignature {
    pub name: String,
    pub span: Span,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    /// Default impl in trait/interface.
    pub has_default: bool,
}

/// A standalone function (not a method).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct FunctionDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub visibility: Visibility,
    pub is_async: bool,
    pub is_generator: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub decorators: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub has_default: bool,
    /// ...args, *args, variadic
    pub is_rest: bool,
}

// ---------------------------------------------------------------------------
// Class & Implementation
// ---------------------------------------------------------------------------

/// A class / struct definition.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ClassDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub visibility: Visibility,
    pub generics: Vec<String>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub methods: Vec<MethodDef>,
    pub properties: Vec<PropertyDef>,
    pub is_abstract: bool,
    pub decorators: Vec<String>,
}

/// An implementation block (Rust impl, Go method set, Swift extension).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ImplDef {
    /// The type being implemented.
    pub target: String,
    /// The trait/interface (None = inherent impl).
    pub trait_name: Option<String>,
    pub file: PathBuf,
    pub span: Span,
    pub methods: Vec<MethodDef>,
}

/// A property / field definition.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct PropertyDef {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub is_readonly: bool,
    pub visibility: Visibility,
}

// ---------------------------------------------------------------------------
// Route & HTTP
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct RouteDef {
    pub method: HttpMethod,
    pub path: String,
    pub handler: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

// ---------------------------------------------------------------------------
// Type Alias
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct TypeAlias {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    /// The type it aliases.
    pub target: String,
    pub generics: Vec<String>,
    pub visibility: Visibility,
}

// ---------------------------------------------------------------------------
// Imports & Exports
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ImportDef {
    /// Module path.
    pub source: String,
    pub symbols: Vec<ImportedSymbol>,
    pub is_wildcard: bool,
    pub span: Span,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ImportedSymbol {
    pub name: String,
    /// import { Foo as Bar } -> alias = Some("Bar")
    pub alias: Option<String>,
    pub is_default: bool,
    pub is_namespace: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ExportDef {
    pub name: String,
    pub kind: ExportKind,
    /// Re-export source.
    pub source: Option<String>,
    pub span: Span,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportKind {
    Named,
    Default,
    ReExport,
}

// ---------------------------------------------------------------------------
// Schema (Runtime type boundaries)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct SchemaDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub kind: SchemaKind,
    /// Sub-parsed from fields_source.
    pub fields: Vec<SchemaField>,
    /// e.g. "zod", "effect-schema", "pydantic", "drizzle"
    pub source_framework: String,
    /// For DB schema definitions.
    pub table_name: Option<String>,
    /// Rust derives.
    pub derives: Vec<String>,
    pub visibility: Visibility,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SchemaKind {
    /// Zod, Effect Schema, io-ts, Yup
    ValidationSchema,
    /// Pydantic, SQLAlchemy, TypeORM, Prisma, Drizzle
    OrmModel,
    /// Rust serde structs, Go tagged structs, Java records, Kotlin data classes
    DataTransfer,
    /// Event schema definitions
    DomainEvent,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct SchemaField {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub is_primary_key: bool,
    pub constraints: Vec<String>,
}

// ---------------------------------------------------------------------------
// IrFile (per-file structural census)
// ---------------------------------------------------------------------------

/// A parsed file's complete structural census.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct IrFile {
    pub path: PathBuf,
    pub language: Language,
    pub content_hash: String,
    pub build_status: BuildStatus,
    pub confidence: Confidence,
    pub interfaces: Vec<InterfaceDef>,
    pub services: Vec<ServiceDef>,
    pub classes: Vec<ClassDef>,
    pub functions: Vec<FunctionDef>,
    pub type_aliases: Vec<TypeAlias>,
    pub imports: Vec<ImportDef>,
    pub exports: Vec<ExportDef>,
    pub implementations: Vec<ImplDef>,
    pub schemas: Vec<SchemaDef>,
}

impl IrFile {
    pub fn new(
        path: PathBuf,
        language: Language,
        content_hash: String,
        build_status: BuildStatus,
    ) -> Self {
        let confidence = build_status.confidence();
        Self {
            path,
            language,
            content_hash,
            build_status,
            confidence,
            interfaces: Vec::new(),
            services: Vec::new(),
            classes: Vec::new(),
            functions: Vec::new(),
            type_aliases: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            implementations: Vec::new(),
            schemas: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Entity (union type for any extracted entity)
// ---------------------------------------------------------------------------

/// Union type for any extracted entity. Used by Tauri IPC and MCP tools.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(tag = "kind")]
pub enum Entity {
    Interface(InterfaceDef),
    Service(ServiceDef),
    Class(ClassDef),
    Function(FunctionDef),
    Schema(SchemaDef),
    Impl(ImplDef),
    TypeAlias(TypeAlias),
}

/// Lightweight summary for list views.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct EntitySummary {
    pub name: String,
    pub kind: EntityKind,
    pub file: PathBuf,
    pub line: u32,
    pub language: Language,
    pub build_status: BuildStatus,
    pub confidence: Confidence,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Interface,
    Service,
    Class,
    Function,
    Schema,
    Impl,
    TypeAlias,
    Method,
}

// ---------------------------------------------------------------------------
// Filter & Config
// ---------------------------------------------------------------------------

/// Filter parameters for querying the index.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
pub struct FilterParams {
    pub languages: Option<Vec<Language>>,
    /// Regex pattern.
    pub name_pattern: Option<String>,
    pub kind: Option<Vec<EntityKind>>,
    pub build_status: Option<BuildStatus>,
    pub visibility: Option<Visibility>,
}

/// Scan configuration (parsed from .domain-scan.toml or CLI flags).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ScanConfig {
    pub root: PathBuf,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Empty = all languages.
    pub languages: Vec<Language>,
    pub build_status_override: Option<BuildStatus>,
    pub cache_enabled: bool,
    pub cache_dir: PathBuf,
}

impl ScanConfig {
    pub fn new(root: PathBuf) -> Self {
        let cache_dir = root.join(".domain-scan-cache");
        Self {
            root,
            include: Vec::new(),
            exclude: Vec::new(),
            languages: Vec::new(),
            build_status_override: None,
            cache_enabled: true,
            cache_dir,
        }
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Result of `domain-scan validate`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ValidationResult {
    pub violations: Vec<Violation>,
    pub rules_checked: usize,
    pub pass_count: usize,
    pub warn_count: usize,
    pub fail_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct Violation {
    pub rule: String,
    pub severity: ViolationSeverity,
    pub message: String,
    pub entity_name: Option<String>,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSeverity {
    Warn,
    Fail,
}

// ---------------------------------------------------------------------------
// Scan Index & Stats
// ---------------------------------------------------------------------------

/// The complete scan result.
/// Lookup tables use indices (file_idx, entity_idx) into the `files` vec
/// to avoid lifetime parameters. Query methods on ScanIndex resolve indices.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ScanIndex {
    pub root: PathBuf,
    pub version: String,
    pub scanned_at: DateTime<Utc>,
    pub files: Vec<IrFile>,
    pub stats: ScanStats,

    // Pre-built lookup tables (populated after all files parsed)
    #[serde(skip)]
    pub(crate) interfaces_by_name: HashMap<String, Vec<(usize, usize)>>,
    #[serde(skip)]
    pub(crate) classes_by_name: HashMap<String, Vec<(usize, usize)>>,
    #[serde(skip)]
    pub(crate) services_by_kind: HashMap<ServiceKind, Vec<(usize, usize)>>,
    #[serde(skip)]
    pub(crate) methods_by_owner: HashMap<String, Vec<(usize, usize)>>,
    #[serde(skip)]
    pub(crate) implementations: HashMap<String, Vec<(usize, usize)>>,
    #[serde(skip)]
    pub(crate) implementors: HashMap<String, Vec<String>>,
    #[serde(skip)]
    pub(crate) schemas_by_framework: HashMap<String, Vec<(usize, usize)>>,
    #[serde(skip)]
    pub(crate) schemas_by_kind: HashMap<SchemaKind, Vec<(usize, usize)>>,
}

impl ScanIndex {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            version: env!("CARGO_PKG_VERSION").to_string(),
            scanned_at: Utc::now(),
            files: Vec::new(),
            stats: ScanStats::default(),
            interfaces_by_name: HashMap::new(),
            classes_by_name: HashMap::new(),
            services_by_kind: HashMap::new(),
            methods_by_owner: HashMap::new(),
            implementations: HashMap::new(),
            implementors: HashMap::new(),
            schemas_by_framework: HashMap::new(),
            schemas_by_kind: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
pub struct ScanStats {
    pub total_files: usize,
    pub files_by_language: HashMap<Language, usize>,
    pub total_interfaces: usize,
    pub total_services: usize,
    pub total_classes: usize,
    pub total_methods: usize,
    pub total_functions: usize,
    pub total_schemas: usize,
    pub total_type_aliases: usize,
    pub total_implementations: usize,
    pub parse_duration_ms: u64,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

// ---------------------------------------------------------------------------
// Manifest Matching
// ---------------------------------------------------------------------------

/// Result of `domain-scan match`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct MatchResult {
    pub matched: Vec<MatchedEntity>,
    pub unmatched: Vec<UnmatchedEntity>,
    pub total_entities: usize,
    pub coverage_percent: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct MatchedEntity {
    pub entity: EntitySummary,
    pub subsystem_id: String,
    pub subsystem_name: String,
    pub match_strategy: MatchStrategy,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrategy {
    /// Entity file falls under subsystem filePath.
    FilePath,
    /// Entity imports from subsystem's files.
    ImportGraph,
    /// Entity name matches subsystem's interfaces/operations/tables/events.
    NameMatch,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct UnmatchedEntity {
    pub entity: EntitySummary,
    /// Best-guess subsystem IDs (for LLM prompt).
    pub candidate_subsystems: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_status_confidence() {
        assert_eq!(BuildStatus::Built.confidence(), Confidence::High);
        assert_eq!(BuildStatus::Error.confidence(), Confidence::Medium);
        assert_eq!(BuildStatus::Unbuilt.confidence(), Confidence::Low);
        assert_eq!(BuildStatus::Rebuild.confidence(), Confidence::Low);
    }

    #[test]
    fn test_ir_file_new_sets_confidence() {
        let file = IrFile::new(
            PathBuf::from("test.ts"),
            Language::TypeScript,
            "abc123".to_string(),
            BuildStatus::Built,
        );
        assert_eq!(file.confidence, Confidence::High);
        assert!(file.interfaces.is_empty());
        assert!(file.services.is_empty());
    }

    #[test]
    fn test_ir_file_unbuilt_confidence() {
        let file = IrFile::new(
            PathBuf::from("test.py"),
            Language::Python,
            "def456".to_string(),
            BuildStatus::Unbuilt,
        );
        assert_eq!(file.confidence, Confidence::Low);
    }

    #[test]
    fn test_ir_file_serde_roundtrip() -> Result<(), serde_json::Error> {
        let file = IrFile::new(
            PathBuf::from("src/main.rs"),
            Language::Rust,
            "hash123".to_string(),
            BuildStatus::Built,
        );
        let json = serde_json::to_string(&file)?;
        let deserialized: IrFile = serde_json::from_str(&json)?;
        assert_eq!(file, deserialized);
        Ok(())
    }

    #[test]
    fn test_scan_stats_default() {
        let stats = ScanStats::default();
        assert_eq!(stats.total_files, 0);
        assert!(stats.files_by_language.is_empty());
    }

    #[test]
    fn test_scan_config_new() {
        let config = ScanConfig::new(PathBuf::from("/tmp/project"));
        assert_eq!(config.root, PathBuf::from("/tmp/project"));
        assert!(config.languages.is_empty());
        assert!(config.cache_enabled);
    }

    #[test]
    fn test_language_display() {
        assert_eq!(Language::TypeScript.to_string(), "TypeScript");
        assert_eq!(Language::CSharp.to_string(), "C#");
        assert_eq!(Language::Cpp.to_string(), "C++");
    }

    #[test]
    fn test_scan_index_new() {
        let index = ScanIndex::new(PathBuf::from("/tmp/project"));
        assert_eq!(index.version, env!("CARGO_PKG_VERSION"));
        assert!(index.files.is_empty());
    }

    #[test]
    fn test_entity_summary_serde() -> Result<(), serde_json::Error> {
        let summary = EntitySummary {
            name: "MyInterface".to_string(),
            kind: EntityKind::Interface,
            file: PathBuf::from("src/types.ts"),
            line: 10,
            language: Language::TypeScript,
            build_status: BuildStatus::Built,
            confidence: Confidence::High,
        };
        let json = serde_json::to_string(&summary)?;
        let deserialized: EntitySummary = serde_json::from_str(&json)?;
        assert_eq!(summary, deserialized);
        Ok(())
    }

    #[test]
    fn test_scan_stats_serde_with_language_keys() -> Result<(), serde_json::Error> {
        let mut stats = ScanStats::default();
        stats.files_by_language.insert(Language::TypeScript, 5);
        stats.files_by_language.insert(Language::Python, 3);
        let json = serde_json::to_string(&stats)?;
        let deserialized: ScanStats = serde_json::from_str(&json)?;
        assert_eq!(stats, deserialized);
        Ok(())
    }
}
