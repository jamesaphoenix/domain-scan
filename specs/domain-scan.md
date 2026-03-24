# domain-scan -- structural code intelligence via tree-sitter

> Find every interface, service, method, trait, protocol, and type boundary in any codebase. Fast, deterministic, language-agnostic.

---

## 1. Product Overview

### 1.1 Problem

Large codebases have thousands of interfaces, services, and methods scattered across hundreds of files. Developers joining a project, auditing an API surface, or preparing for a refactor need answers to:

- "What services exist and what methods do they expose?"
- "Which interfaces are implemented by which structs/classes?"
- "What is the full public API surface of this module?"
- "Where are the RPC/HTTP/gRPC service definitions?"
- "Which methods are async? Which take callbacks? Which return streams?"

Today this requires grepping, manual navigation, or language-specific tooling that breaks across polyglot repos. LSP servers answer point queries but cannot produce a full structural census.

### 1.2 Solution

**domain-scan** is a Rust CLI that uses tree-sitter with declarative `.scm` query files to extract a complete structural census of interfaces, services, methods, types, and their relationships from any supported codebase. It produces structured JSON output suitable for human review, LLM consumption, or downstream tooling.

### 1.3 Core Principles

1. **Declarative language support.** Adding a new language = writing `.scm` query files. Zero Rust code per language.
2. **Deterministic output.** Same input, same output. No LLM in the critical path.
3. **Parallel by default.** rayon for file parsing. Thread-local parser pools.
4. **Structured output.** JSON schema for all output. Pipe to jq, feed to LLMs, ingest into databases.
5. **Incremental.** Content-addressed caching (SHA-256) so re-scans skip unchanged files.
6. **Build-status-aware.** Source code is only authoritative for modules that actually build. Everything else is a best-guess, enriched by LLM agents.

### 1.4 Module Build Status Model

Every scanned module has a `BuildStatus` that determines how its extracted data is treated downstream:

| Status | Source Code Is... | Tree-Sitter Output | LLM Enrichment |
|--------|-------------------|--------------------|-----------------|
| `Built` | **Source of truth.** Module compiles/runs. | High confidence. Pull interfaces, services, methods directly. | Not needed for interface extraction. Still needed for higher-level enrichment (e.g. domain classification, subsystem mapping, intent inference). |
| `Unbuilt` | **Best guess.** Module has never been built or has stale artifacts. | Low confidence. Extraction may miss dynamic patterns. | Recommended. Agents infer intent from naming, comments, partial defs. |
| `Error` | **Partial truth.** Module fails to build (compiler errors). | Mixed confidence. Valid syntax parses fine, broken code may be incomplete. | Recommended. Agents flag broken contracts vs intentional WIP. |
| `Rebuild` | **Unreliable.** Module is being actively refactored. | Do not treat as authoritative. Old and new definitions may conflict. | Required. Agents reconcile old vs new, flag conflicts, infer intended state. |

**Detection heuristics:**
- `Built`: has recent build artifacts (e.g. `target/` with fresh timestamps for Rust, `node_modules/.cache` for TS, `__pycache__` for Python, `.class` files for Java)
- `Unbuilt`: no build artifacts exist
- `Error`: build artifacts exist but are older than source changes, or `Cargo.lock` / `package-lock.json` has unresolved conflicts
- `Rebuild`: detected via git status (many uncommitted changes in the module) or explicit `--build-status rebuild` flag

**CLI integration:**
```bash
# Auto-detect build status per module
domain-scan scan

# Override: treat everything as built (trust source code)
domain-scan scan --build-status built

# Override: treat everything as rebuild (LLM enrichment for all)
domain-scan scan --build-status rebuild
```

**JSON output includes per-file status:**
```json
{
  "path": "src/auth/service.ts",
  "build_status": "built",
  "confidence": "high",
  "interfaces": [...]
}
```

**LLM prompt generation respects build status.** When `domain-scan prompt` generates sub-agent assignments, it partitions files by build status:
- `Built` files get a "verify and catalog" instruction (trust the scan, just confirm)
- `Unbuilt`/`Error`/`Rebuild` files get an "analyze and infer" instruction (read the code, infer intent, flag gaps)

### 1.5 Non-Goals

- Not a Language Server Protocol implementation (no hover, no go-to-definition)
- Not a linter or formatter
- Not a code generator
- No LLM in the analysis pipeline (LLM prompt generation is output, not input)

---

## 2. Architecture

### 2.1 High-Level Pipeline

```
Filesystem walker (ignore .gitignore, configurable globs)
      |
      v
Language detection (by extension + shebang)
      |
      v
Tree-sitter parsing (parallel via rayon, thread-local parsers)
      |
      v
.scm query dispatch (per-language query sets, lazy compiled)
      |
      v
Language-agnostic IR (IrFile -> definitions, methods, implementations, services)
      |
      v
Cross-file resolution (import/export tracking, implementation matching)
      |
      v
Index construction (in-memory + optional disk cache)
      |
      v
Query engine (CLI subcommands filter/search the index)
      |
      v
Output (JSON / table / LLM prompt)
```

### 2.2 Crate Layout

```
domain-scan/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── domain-scan-core/          # Library: all analysis logic
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API + strict clippy deny wall
│   │   │   ├── walker.rs       # Filesystem traversal (ignore crate)
│   │   │   ├── lang.rs         # Language detection + registry
│   │   │   ├── parser.rs       # Tree-sitter parsing + thread-local pool
│   │   │   ├── query_engine.rs # .scm query loading, compilation, dispatch
│   │   │   ├── ir.rs           # Language-agnostic intermediate representation
│   │   │   ├── build_status.rs # Build status detection heuristics
│   │   │   ├── resolver.rs     # Cross-file import/export + implementation matching
│   │   │   ├── index.rs        # In-memory index with query methods
│   │   │   ├── cache.rs        # SHA-256 content-addressed caching
│   │   │   ├── config.rs       # .domain-scan.toml parsing
│   │   │   ├── output.rs       # JSON + table + LLM prompt serialization
│   │   │   └── types.rs        # Public types (InterfaceDef, ServiceDef, MethodDef, etc.)
│   │   ├── queries/            # Tree-sitter .scm files (one dir per language)
│   │   │   ├── typescript/
│   │   │   │   ├── interfaces.scm
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── functions.scm
│   │   │   │   ├── types.scm
│   │   │   │   ├── services.scm      # Framework-specific (NestJS, tRPC, etc.)
│   │   │   │   ├── effect.scm        # Effect.ts: Service, Layer, Schema, Config, pipe/flow, error channels
│   │   │   │   ├── imports.scm
│   │   │   │   └── exports.scm
│   │   │   ├── python/
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── functions.scm
│   │   │   │   ├── protocols.scm      # typing.Protocol (Python's interfaces)
│   │   │   │   ├── abstract.scm       # ABC classes
│   │   │   │   ├── services.scm       # FastAPI, Flask, Django
│   │   │   │   ├── imports.scm
│   │   │   │   └── decorators.scm
│   │   │   ├── rust/
│   │   │   │   ├── traits.scm         # Rust's interfaces
│   │   │   │   ├── impls.scm          # impl blocks (trait impls + inherent)
│   │   │   │   ├── methods.scm
│   │   │   │   ├── functions.scm
│   │   │   │   ├── types.scm          # struct, enum, type alias
│   │   │   │   ├── services.scm       # actix, axum, tonic (gRPC)
│   │   │   │   └── imports.scm        # use statements
│   │   │   ├── go/
│   │   │   │   ├── interfaces.scm
│   │   │   │   ├── structs.scm
│   │   │   │   ├── methods.scm        # Receiver methods
│   │   │   │   ├── functions.scm
│   │   │   │   ├── services.scm       # net/http, gRPC
│   │   │   │   └── imports.scm
│   │   │   ├── java/
│   │   │   │   ├── interfaces.scm
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── annotations.scm    # @Service, @Controller, @RestController
│   │   │   │   ├── services.scm       # Spring Boot service detection
│   │   │   │   └── imports.scm
│   │   │   ├── kotlin/
│   │   │   │   ├── interfaces.scm
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── services.scm
│   │   │   │   └── imports.scm
│   │   │   ├── csharp/
│   │   │   │   ├── interfaces.scm
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── services.scm       # ASP.NET, gRPC
│   │   │   │   └── imports.scm        # using statements
│   │   │   ├── swift/
│   │   │   │   ├── protocols.scm      # Swift's interfaces
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── extensions.scm
│   │   │   │   └── imports.scm
│   │   │   ├── php/
│   │   │   │   ├── interfaces.scm
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── traits.scm         # PHP traits
│   │   │   │   └── imports.scm        # use statements
│   │   │   ├── ruby/
│   │   │   │   ├── modules.scm
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   └── imports.scm        # require/include
│   │   │   ├── scala/
│   │   │   │   ├── traits.scm
│   │   │   │   ├── classes.scm
│   │   │   │   ├── methods.scm
│   │   │   │   ├── objects.scm        # Companion objects
│   │   │   │   └── imports.scm
│   │   │   └── cpp/
│   │   │       ├── classes.scm
│   │   │       ├── methods.scm
│   │   │       ├── functions.scm
│   │   │       ├── templates.scm
│   │   │       ├── virtual.scm        # Pure virtual = interface
│   │   │       └── imports.scm        # #include
│   │   └── tests/
│   │       ├── integration/
│   │       │   ├── treesitter_real.rs  # Real tree-sitter parsing tests
│   │       │   ├── cross_file.rs      # Cross-file resolution tests
│   │       │   ├── query_engine.rs    # Query dispatch tests
│   │       │   └── full_pipeline.rs   # End-to-end scan tests
│   │       ├── fixtures/              # Real code snippets per language
│   │       │   ├── typescript/
│   │       │   ├── python/
│   │       │   ├── rust/
│   │       │   ├── go/
│   │       │   ├── java/
│   │       │   └── ...
│   │       └── helpers/
│   │           └── mod.rs
│   ├── domain-scan-cli/               # Binary: CLI entry point
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs             # clap subcommands
│   └── domain-scan-tauri/             # Tauri 2 desktop app
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs             # Tauri setup, window config
│       │   ├── lib.rs
│       │   └── commands.rs         # Tauri IPC commands
│       ├── ui/                     # React frontend (bundled by Tauri)
│       │   ├── src/
│       │   │   ├── App.tsx
│       │   │   ├── components/     # EntityTree, SourcePreview, DetailsPanel
│       │   │   ├── hooks/
│       │   │   ├── types.ts
│       │   │   └── styles.css
│       │   ├── package.json
│       │   └── vite.config.ts
│       ├── tauri.conf.json
│       └── icons/
├── specs/
│   ├── readme.md                   # Spec index
│   └── domain-scan.md                 # This file
├── loop.sh                         # Claude loop harness
├── prompt.md                       # Development prompt for Ralph
├── CLAUDE.md                       # Project context
└── README.md
```

### 2.3 Clippy Deny Wall (lib.rs)

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
```

All errors via `thiserror`. All propagation via `?`. No `.unwrap()` anywhere.

---

## 3. Intermediate Representation (IR)

### 3.1 Core Types

```rust
/// Build status of the module this file belongs to
pub enum BuildStatus {
    Built,      // Compiles/runs. Source code is truth.
    Unbuilt,    // Never built or stale artifacts. Source is best guess.
    Error,      // Build fails. Partial truth.
    Rebuild,    // Active refactor. Source unreliable, needs LLM reconciliation.
}

/// Confidence level for extracted entities
pub enum Confidence {
    High,       // From a Built module. Tree-sitter extraction is authoritative.
    Medium,     // From an Error module. Syntax parsed but semantics may be incomplete.
    Low,        // From Unbuilt/Rebuild module. Best guess, needs LLM enrichment.
}

/// A parsed file's complete structural census
pub struct IrFile {
    pub path: PathBuf,
    pub language: Language,
    pub content_hash: String,           // SHA-256 for caching
    pub build_status: BuildStatus,      // Detected or overridden build status
    pub confidence: Confidence,         // Derived from build_status
    pub interfaces: Vec<InterfaceDef>,
    pub services: Vec<ServiceDef>,
    pub classes: Vec<ClassDef>,
    pub functions: Vec<FunctionDef>,
    pub type_aliases: Vec<TypeAlias>,
    pub imports: Vec<ImportDef>,
    pub exports: Vec<ExportDef>,
    pub implementations: Vec<ImplDef>,  // impl Trait for Struct, class implements Interface
    pub schemas: Vec<SchemaDef>,        // Runtime schemas: Zod, Effect, Pydantic, Drizzle, serde, etc.
    pub effect_layers: Vec<EffectLayerDef>,          // Effect.ts Layer definitions
    pub effect_errors: Vec<EffectErrorChannelDef>,   // Effect.ts tagged error types
    pub effect_pipelines: Vec<EffectPipelineDef>,    // Effect.ts pipe/flow compositions
}

/// An interface / trait / protocol definition
pub struct InterfaceDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub visibility: Visibility,
    pub generics: Vec<String>,
    pub extends: Vec<String>,           // Parent interfaces/traits
    pub methods: Vec<MethodSignature>,
    pub properties: Vec<PropertyDef>,   // TS interface properties, trait associated types
    pub language_kind: InterfaceKind,   // Interface | Trait | Protocol | ABC | PureVirtual
    pub decorators: Vec<String>,
}

pub enum InterfaceKind {
    Interface,      // TS/Java/Go/C#/Kotlin/PHP
    Trait,          // Rust/Scala/PHP
    Protocol,       // Swift/Python (typing.Protocol)
    AbstractClass,  // Python ABC, Java abstract class
    PureVirtual,    // C++ class with pure virtual methods
    Module,         // Ruby module used as mixin
}

/// A service definition (framework-specific)
pub struct ServiceDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub kind: ServiceKind,
    pub methods: Vec<MethodDef>,
    pub dependencies: Vec<String>,      // Injected dependencies
    pub decorators: Vec<String>,
    pub routes: Vec<RouteDef>,          // HTTP routes if applicable
    pub effect_tag: Option<String>,     // Effect.ts Context.Tag string identifier (e.g. "UserRepo")
}

pub enum ServiceKind {
    HttpController,     // Express router, FastAPI router, Spring @Controller
    GrpcService,        // tonic, gRPC service impl
    GraphqlResolver,    // GraphQL resolver class
    Worker,             // Queue consumer, background job
    Microservice,       // NestJS @Injectable, Spring @Service
    CliCommand,         // CLI command handler
    EventHandler,       // Event/message handler
    Middleware,         // Express/Koa middleware, Django middleware
    Repository,         // Data access layer
    EffectService,      // Effect.ts Context.Tag service (Effect.Service, Context.GenericTag)
    Custom(String),     // User-defined via config
}

/// A concrete method (with body)
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
    pub owner: Option<String>,          // Class/struct/impl this belongs to
    pub implements: Option<String>,     // Which interface method this implements
}

/// A method signature (no body, in interface/trait)
pub struct MethodSignature {
    pub name: String,
    pub span: Span,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub has_default: bool,              // Default impl in trait/interface
}

/// A class / struct definition
pub struct ClassDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub visibility: Visibility,
    pub generics: Vec<String>,
    pub extends: Option<String>,        // Superclass
    pub implements: Vec<String>,        // Interfaces/traits implemented
    pub methods: Vec<MethodDef>,
    pub properties: Vec<PropertyDef>,
    pub is_abstract: bool,
    pub decorators: Vec<String>,
}

/// An implementation block (Rust impl, Go method set, Swift extension)
pub struct ImplDef {
    pub target: String,                 // The type being implemented
    pub trait_name: Option<String>,     // The trait/interface (None = inherent impl)
    pub file: PathBuf,
    pub span: Span,
    pub methods: Vec<MethodDef>,
}

/// A standalone function (not a method)
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

pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub has_default: bool,
    pub is_rest: bool,                  // ...args, *args, variadic
}

pub struct PropertyDef {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub is_readonly: bool,
    pub visibility: Visibility,
}

pub struct RouteDef {
    pub method: HttpMethod,
    pub path: String,
    pub handler: String,
}

pub struct TypeAlias {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub target: String,                 // The type it aliases
    pub generics: Vec<String>,
    pub visibility: Visibility,
}

pub struct ImportDef {
    pub source: String,                 // Module path
    pub symbols: Vec<ImportedSymbol>,
    pub is_wildcard: bool,
    pub span: Span,
}

pub struct ImportedSymbol {
    pub name: String,
    pub alias: Option<String>,          // import { Foo as Bar } -> alias = "Bar"
    pub is_default: bool,              // import Foo from '...' -> is_default = true
    pub is_namespace: bool,            // import * as ns from '...' -> is_namespace = true
}

pub struct ExportDef {
    pub name: String,
    pub kind: ExportKind,
    pub source: Option<String>,         // Re-export source
    pub span: Span,
}

pub enum ExportKind {
    Named,          // export { Foo }
    Default,        // export default class Foo
    ReExport,       // export { Foo } from './bar'
}

// --- Effect.ts-specific IR types ---

/// An Effect.ts Layer definition (dependency injection / composition)
pub struct EffectLayerDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub kind: EffectLayerKind,
    pub provides: Vec<String>,          // Services this layer provides
    pub requires: Vec<String>,          // Services this layer depends on (the R channel)
    pub composition: Vec<String>,       // Other layers merged/composed into this one
}

pub enum EffectLayerKind {
    EffectLayer,        // Layer.effect — effectful construction
    SyncLayer,          // Layer.sync — synchronous construction
    SucceedLayer,       // Layer.succeed — constant value
    ScopedLayer,        // Layer.scoped — scoped resource
    FreshLayer,         // Layer.fresh — fresh instance per use
    MergeComposition,   // Layer.merge / Layer.provideMerge
    PipeComposition,    // pipe(LayerA, Layer.provide(LayerB))
}

/// An Effect.ts error channel type (the E in Effect<A, E, R>)
pub struct EffectErrorChannelDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub tag: String,                    // The _tag discriminant value
    pub extends: Option<String>,        // Parent error class (Data.TaggedError, Schema.TaggedError)
    pub fields: Vec<SchemaField>,       // Structured error payload fields
}

/// An Effect.ts pipe/flow composition chain
pub struct EffectPipelineDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub steps: Vec<String>,            // Ordered list of Effect combinators in the pipe
    pub input_type: Option<String>,     // Inferred or annotated input
    pub output_type: Option<String>,    // The A channel
    pub error_type: Option<String>,     // The E channel
    pub requirements: Option<String>,   // The R channel (dependencies)
}

pub enum HttpMethod {
    Get, Post, Put, Patch, Delete, Head, Options,
}

pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_range: (usize, usize),
}

pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,       // C# internal, Kotlin internal
    Crate,          // Rust pub(crate)
    Unknown,        // Language doesn't have visibility modifiers (Go, Python)
}

/// TypeScript and JavaScript share the same tree-sitter grammar and queries.
/// JS files are detected by extension and parsed with the TypeScript grammar.
pub enum Language {
    TypeScript,     // Also covers JavaScript (.js, .jsx, .mjs, .cjs)
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

/// Union type for any extracted entity. Used by Tauri IPC and CLI.
pub enum Entity {
    Interface(InterfaceDef),
    Service(ServiceDef),
    Class(ClassDef),
    Function(FunctionDef),
    Schema(SchemaDef),
    Impl(ImplDef),
    TypeAlias(TypeAlias),
    EffectLayer(EffectLayerDef),
    EffectError(EffectErrorChannelDef),
    EffectPipeline(EffectPipelineDef),
}

/// Lightweight summary for list views (tree panel, search results).
pub struct EntitySummary {
    pub name: String,
    pub kind: EntityKind,               // "interface", "service", "class", etc.
    pub file: PathBuf,
    pub line: u32,
    pub language: Language,
    pub build_status: BuildStatus,
    pub confidence: Confidence,
}

pub enum EntityKind {
    Interface, Service, Class, Function, Schema, Impl, TypeAlias, Method,
    EffectLayer, EffectError, EffectPipeline,
}

/// Filter parameters for querying the index. Used by CLI and Tauri IPC.
pub struct FilterParams {
    pub languages: Option<Vec<Language>>,
    pub name_pattern: Option<String>,       // Regex
    pub kind: Option<Vec<EntityKind>>,
    pub build_status: Option<BuildStatus>,  // Filter results to this status
    pub visibility: Option<Visibility>,
}

/// Scan configuration (parsed from .domain-scan.toml or CLI flags).
pub struct ScanConfig {
    pub root: PathBuf,
    pub include: Vec<String>,               // Glob patterns
    pub exclude: Vec<String>,               // Glob patterns
    pub languages: Vec<Language>,            // Empty = all
    pub build_status_override: Option<BuildStatus>,
    pub cache_enabled: bool,
    pub cache_dir: PathBuf,
}

/// Result of `domain-scan validate`. Each violation references a specific rule and entity.
pub struct ValidationResult {
    pub violations: Vec<Violation>,
    pub rules_checked: usize,
    pub pass_count: usize,
    pub warn_count: usize,
    pub fail_count: usize,
}

pub struct Violation {
    pub rule: String,                       // "naming_pascal_case", "no_god_interfaces", etc.
    pub severity: ViolationSeverity,
    pub message: String,
    pub entity_name: Option<String>,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

pub enum ViolationSeverity { Warn, Fail }

/// Result of `domain-scan match`. Maps entities to subsystems with an unmatched bucket.
pub struct MatchResult {
    pub matched: Vec<MatchedEntity>,
    pub unmatched: Vec<UnmatchedEntity>,
    pub total_entities: usize,
    pub coverage_percent: f64,
}

pub struct MatchedEntity {
    pub entity: EntitySummary,
    pub subsystem_id: String,
    pub subsystem_name: String,
    pub match_strategy: MatchStrategy,      // How it was matched
}

pub enum MatchStrategy {
    FilePath,       // Entity file falls under subsystem filePath
    ImportGraph,    // Entity imports from subsystem's files
    NameMatch,      // Entity name matches subsystem's interfaces/operations/tables/events
}

pub struct UnmatchedEntity {
    pub entity: EntitySummary,
    pub candidate_subsystems: Vec<String>,  // Best-guess subsystem IDs (for LLM prompt)
}
```

### 3.2 Index

```rust
/// The complete scan result: everything found in the codebase.
/// Lookup tables use indices (file_idx, entity_idx) into the `files` vec
/// to avoid lifetime parameters. Query methods on ScanIndex resolve indices.
pub struct ScanIndex {
    pub root: PathBuf,
    pub version: String,                // "0.1.0"
    pub scanned_at: chrono::DateTime<chrono::Utc>,
    pub files: Vec<IrFile>,
    pub stats: ScanStats,

    // Pre-built lookup tables (populated after all files parsed)
    // Keys are names/kinds; values are (file_index, entity_index) pairs.
    interfaces_by_name: HashMap<String, Vec<(usize, usize)>>,
    classes_by_name: HashMap<String, Vec<(usize, usize)>>,
    services_by_kind: HashMap<ServiceKind, Vec<(usize, usize)>>,
    methods_by_owner: HashMap<String, Vec<(usize, usize)>>,
    implementations: HashMap<String, Vec<(usize, usize)>>,  // trait/interface name -> impls
    implementors: HashMap<String, Vec<String>>,              // trait/interface name -> implementing types
    schemas_by_framework: HashMap<String, Vec<(usize, usize)>>,
    schemas_by_kind: HashMap<SchemaKind, Vec<(usize, usize)>>,
}

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
```

---

## 4. Tree-Sitter Query Files (.scm)

### 4.1 Query Architecture

Each language gets a directory under `queries/<language>/`. Each `.scm` file targets one structural category. The query engine loads all `.scm` files for a language, compiles them lazily on first use, and dispatches captures to IR constructors.

**Capture naming convention:**

```
@interface.name          -> InterfaceDef.name
@interface.extends       -> InterfaceDef.extends
@method.name             -> MethodDef.name / MethodSignature.name
@method.async            -> MethodDef.is_async (presence = true)
@method.static           -> MethodDef.is_static
@method.params           -> MethodDef.parameters (needs sub-parsing)
@method.return_type      -> MethodDef.return_type
@class.name              -> ClassDef.name
@class.extends           -> ClassDef.extends
@class.implements        -> ClassDef.implements
@service.name            -> ServiceDef.name
@service.decorator       -> ServiceDef.decorators
@function.name           -> FunctionDef.name
@import.source           -> ImportDef.source
@import.symbol           -> ImportDef.symbols
@impl.target             -> ImplDef.target
@impl.trait              -> ImplDef.trait_name
@type.name               -> TypeAlias.name
@type.target             -> TypeAlias.target
@visibility              -> Visibility
@decorator               -> decorators vec
```

### 4.2 Example: TypeScript interfaces.scm

```scheme
;; Named interface declarations
;; NOTE: Use name: (_) instead of name: (property_identifier) to also catch
;; computed method names ([Symbol.iterator]) and string-named methods.
(interface_declaration
  name: (type_identifier) @interface.name
  type_parameters: (type_parameters)? @interface.generics
  (extends_type_clause
    (type_identifier) @interface.extends)?
  body: (interface_body
    (method_signature
      name: (_) @method.name
      parameters: (formal_parameters) @method.params
      return_type: (type_annotation)? @method.return_type) @interface.method
    (property_signature
      name: (_) @property.name
      type: (type_annotation)? @property.type) @interface.property))
```

### 4.3 Example: Rust traits.scm

```scheme
;; Trait definitions
;; NOTE: Use (_) wildcards for return_type, trait, and type fields to catch
;; generic types (Vec<T>), scoped types (serde::Serialize), etc.
(trait_item
  name: (type_identifier) @interface.name
  type_parameters: (type_parameters)? @interface.generics
  bounds: (trait_bounds
    (_) @interface.extends)?
  body: (declaration_list
    (function_signature_item
      name: (identifier) @method.name
      parameters: (parameters) @method.params
      return_type: (_)? @method.return_type) @interface.method))

;; Trait implementations
(impl_item
  trait: (_) @impl.trait
  type: (_) @impl.target
  body: (declaration_list) @impl.body)

;; Inherent implementations
(impl_item
  !trait
  type: (_) @impl.target
  body: (declaration_list) @impl.body)
```

### 4.4 Example: Go interfaces.scm

```scheme
;; Interface type declarations
;; NOTE: tree-sitter-go renamed method_spec to method_elem in recent versions.
(type_declaration
  (type_spec
    name: (type_identifier) @interface.name
    type: (interface_type
      (method_elem
        name: (field_identifier) @method.name
        parameters: (parameter_list) @method.params
        result: (_)? @method.return_type) @interface.method
      ;; Embedded interfaces
      (type_elem)? @interface.extends)))
```

### 4.5 Example: Python protocols.scm

```scheme
;; typing.Protocol classes
(class_definition
  name: (identifier) @interface.name
  superclasses: (argument_list
    (attribute
      object: (identifier) @_module
      attribute: (identifier) @_protocol)
    (#eq? @_protocol "Protocol"))
  body: (block
    (function_definition
      name: (identifier) @method.name
      parameters: (parameters) @method.params
      return_type: (type)? @method.return_type)* @interface.method))

;; ABC abstract classes
(class_definition
  name: (identifier) @interface.name
  superclasses: (argument_list
    (identifier) @_abc
    (#match? @_abc "^ABC$"))
  body: (block
    (decorated_definition
      (decorator
        (attribute
          attribute: (identifier) @_dec
          (#eq? @_dec "abstractmethod")))
      definition: (function_definition
        name: (identifier) @method.name
        parameters: (parameters) @method.params
        return_type: (type)? @method.return_type))* @interface.method))
```

### 4.6 Runtime Schema Extraction (schemas.scm)

Plain `interface`/`type` declarations only capture compile-time type boundaries. Many codebases define structural boundaries at runtime via schema libraries. These are equally important for understanding the API surface and data contracts.

Each language gets a `schemas.scm` query file targeting framework-specific runtime type definitions:

#### TypeScript schemas.scm

```scheme
;; Effect.ts Schema.Struct
;; const User = Schema.Struct({ name: Schema.String, age: Schema.Number })
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_obj
      property: (property_identifier) @_method
      (#any-of? @_obj "Schema" "S")
      (#eq? @_method "Struct"))
    arguments: (arguments) @schema.fields)) @schema.node

;; Zod z.object
;; const UserSchema = z.object({ name: z.string(), age: z.number() })
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_z
      property: (property_identifier) @_method
      (#eq? @_z "z")
      (#eq? @_method "object"))
    arguments: (arguments) @schema.fields)) @schema.node

;; Drizzle pgTable / mysqlTable / sqliteTable
;; export const users = pgTable('users', { id: serial('id').primaryKey(), ... })
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (identifier) @_fn
    (#any-of? @_fn "pgTable" "mysqlTable" "sqliteTable")
    arguments: (arguments
      (string) @schema.table_name
      (_) @schema.fields))) @schema.node
```

#### Python schemas.scm

```scheme
;; Pydantic BaseModel
;; class User(BaseModel): name: str; age: int
(class_definition
  name: (identifier) @schema.name
  superclasses: (argument_list
    (identifier) @_base
    (#any-of? @_base "BaseModel" "BaseSettings"))
  body: (block) @schema.fields) @schema.node

;; dataclass
;; @dataclass class User: name: str; age: int
(decorated_definition
  (decorator
    (identifier) @_dec
    (#any-of? @_dec "dataclass" "dataclasses.dataclass"))
  definition: (class_definition
    name: (identifier) @schema.name
    body: (block) @schema.fields)) @schema.node

;; TypedDict
;; class UserDict(TypedDict): name: str; age: int
(class_definition
  name: (identifier) @schema.name
  superclasses: (argument_list
    (identifier) @_base
    (#eq? @_base "TypedDict"))
  body: (block) @schema.fields) @schema.node

;; SQLAlchemy declarative model
;; class User(Base): __tablename__ = 'users'; id = Column(Integer, primary_key=True)
(class_definition
  name: (identifier) @schema.name
  superclasses: (argument_list
    (identifier) @_base
    (#any-of? @_base "Base" "DeclarativeBase"))
  body: (block) @schema.fields) @schema.node
```

#### Rust schemas.scm

```scheme
;; #[derive(Serialize, Deserialize)] struct
(attribute_item
  (attribute
    (identifier) @_derive
    (#eq? @_derive "derive")
    arguments: (token_tree) @schema.derives))
.
(struct_item
  name: (type_identifier) @schema.name
  body: (_) @schema.fields) @schema.node

;; #[derive(Validate)] structs (validator crate)
;; Same pattern, detected by checking derives for Serialize/Deserialize/Validate
```

#### Go schemas.scm

```scheme
;; Struct with json/db tags (indicates serialization boundary)
(type_declaration
  (type_spec
    name: (type_identifier) @schema.name
    type: (struct_type
      (field_declaration_list
        (field_declaration
          tag: (raw_string_literal) @schema.tag)*)))) @schema.node
```

#### Java schemas.scm

```scheme
;; JPA @Entity
(class_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @_ann
      (#eq? @_ann "Entity")))
  name: (identifier) @schema.name
  body: (class_body) @schema.fields) @schema.node

;; Java record (Java 16+)
(record_declaration
  name: (identifier) @schema.name
  parameters: (formal_parameters) @schema.fields) @schema.node
```

#### Kotlin schemas.scm

```scheme
;; data class
;; NOTE: Kotlin uses (identifier) for class names, not (type_identifier).
;; Confirmed by flowdiff's kotlin/definitions.scm.
(class_declaration
  (modifiers
    (modifier) @_mod
    (#eq? @_mod "data"))
  (identifier) @schema.name
  (primary_constructor
    (class_parameter)* @schema.fields)) @schema.node
```

**Schema capture conventions:**
```
@schema.name         -> SchemaDef.name
@schema.node         -> SchemaDef (full node for span)
@schema.fields       -> SchemaDef.fields_source (raw text, sub-parsed in Rust)
@schema.table_name   -> SchemaDef.table_name (for DB schemas)
@schema.tag          -> SchemaDef.tags (Go struct tags)
@schema.derives      -> SchemaDef.derives (Rust derive macros)
```

### 4.7 Effect.ts Deep Extraction (effect.scm)

Effect.ts is a first-class framework for domain-scan. It defines structural boundaries through a distinctive pattern system: services via `Context.Tag`, dependency injection via `Layer`, typed error channels via `Data.TaggedError`, schemas via `Schema.Struct`/`Schema.Class`, and composition via `pipe`/`flow`. These map to specific IR types beyond what `schemas.scm` captures.

The dedicated `queries/typescript/effect.scm` file extracts all Effect structural patterns in one pass.

#### 4.7.1 Effect Services (Context.Tag)

Effect services are defined via `Context.Tag` or the `Effect.Service` pattern — not as classes or decorators. They represent dependency injection boundaries.

```scheme
;; Context.Tag pattern
;; class UserRepo extends Context.Tag("UserRepo")<UserRepo, { findById(id: string): Effect<User, NotFound> }>() {}
(class_declaration
  name: (type_identifier) @service.name
  (class_heritage
    (extends_clause
      value: (call_expression
        function: (member_expression
          object: (call_expression
            function: (member_expression
              object: (identifier) @_ctx
              property: (property_identifier) @_tag
              (#eq? @_ctx "Context")
              (#eq? @_tag "Tag"))
            arguments: (arguments
              (string) @service.effect_tag))
          property: (property_identifier))
        arguments: (arguments) @service.shape)))) @service.node

;; Effect.Service pattern (newer API)
;; class UserRepo extends Effect.Service<UserRepo>()("UserRepo", { ... }) {}
(class_declaration
  name: (type_identifier) @service.name
  (class_heritage
    (extends_clause
      value: (call_expression
        function: (call_expression
          function: (member_expression
            object: (identifier) @_eff
            property: (property_identifier) @_svc
            (#eq? @_eff "Effect")
            (#eq? @_svc "Service")))
        arguments: (arguments
          (string) @service.effect_tag
          (_) @service.shape))))) @service.node

;; Context.GenericTag pattern
;; const Cache = Context.GenericTag<CacheService>("Cache")
(variable_declarator
  name: (identifier) @service.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_ctx
      property: (property_identifier) @_tag
      (#eq? @_ctx "Context")
      (#any-of? @_tag "GenericTag" "Tag"))
    arguments: (arguments
      (string) @service.effect_tag))) @service.node
```

Service captures produce a `ServiceDef` with `kind: EffectService`. The `@service.shape` capture is sub-parsed to extract method signatures (the service interface contract). The `@service.effect_tag` is stored as `ServiceDef.effect_tag`.

#### 4.7.2 Effect Layers (Dependency Injection Graph)

Layers are Effect's dependency injection and composition mechanism. They define how services are constructed and what they depend on. Extracting the layer graph reveals the full DI wiring.

```scheme
;; Layer.effect — effectful construction
;; const UserRepoLive = Layer.effect(UserRepo, Effect.gen(function*() { ... }))
(variable_declarator
  name: (identifier) @layer.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_layer
      property: (property_identifier) @_method
      (#eq? @_layer "Layer")
      (#any-of? @_method "effect" "sync" "succeed" "scoped" "fresh" "suspend"))
    arguments: (arguments
      (identifier) @layer.provides
      (_) @layer.body))) @layer.node

;; Layer.merge composition
;; const AppLive = Layer.merge(UserRepoLive, AuthLive)
(variable_declarator
  name: (identifier) @layer.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_layer
      property: (property_identifier) @_method
      (#eq? @_layer "Layer")
      (#any-of? @_method "merge" "mergeAll" "provideMerge"))
    arguments: (arguments) @layer.composed_layers)) @layer.node

;; pipe-based layer composition
;; const AppLive = pipe(UserRepoLive, Layer.provide(DbLive))
;; const AppLive = UserRepoLive.pipe(Layer.provide(DbLive))
(variable_declarator
  name: (identifier) @layer.name
  value: (call_expression
    function: (identifier) @_pipe
    (#eq? @_pipe "pipe")
    arguments: (arguments
      (identifier) @layer.base
      (call_expression
        function: (member_expression
          object: (identifier) @_layer2
          property: (property_identifier) @_provide
          (#eq? @_layer2 "Layer")
          (#any-of? @_provide "provide" "provideMerge" "use" "passthrough"))
        arguments: (arguments) @layer.deps)*))) @layer.node
```

Layer captures produce `EffectLayerDef`. The `@layer.provides` identifies which service tag the layer satisfies. The `@layer.composed_layers` and `@layer.deps` are sub-parsed to extract dependency relationships, building the full DI graph.

#### 4.7.3 Effect Schemas (Schema.Struct, Schema.Class, Schema.TaggedStruct, Schema.Union)

Beyond `Schema.Struct` (already in `schemas.scm`), Effect's schema module defines several structural patterns that represent domain types:

```scheme
;; Schema.Class — branded class schema with methods
;; class User extends Schema.Class<User>("User")({ name: Schema.String, age: Schema.Number }) {}
(class_declaration
  name: (type_identifier) @schema.name
  (class_heritage
    (extends_clause
      value: (call_expression
        function: (call_expression
          function: (member_expression
            object: (identifier) @_schema
            property: (property_identifier) @_class
            (#eq? @_schema "Schema")
            (#eq? @_class "Class")))
        arguments: (arguments
          (_) @schema.fields))))) @schema.node

;; Schema.TaggedStruct — discriminated union member
;; const UserCreated = Schema.TaggedStruct("UserCreated")({ userId: Schema.String })
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (call_expression
      function: (member_expression
        object: (identifier) @_schema
        property: (property_identifier) @_method
        (#eq? @_schema "Schema")
        (#any-of? @_method "TaggedStruct" "TaggedClass"))
      arguments: (arguments
        (string) @schema.effect_tag))
    arguments: (arguments
      (_) @schema.fields))) @schema.node

;; Schema.Union — discriminated union type
;; const Event = Schema.Union(UserCreated, UserDeleted, UserUpdated)
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_schema
      property: (property_identifier) @_method
      (#eq? @_schema "Schema")
      (#eq? @_method "Union"))
    arguments: (arguments) @schema.members)) @schema.node

;; Schema.Struct (already in schemas.scm, but also captured here for completeness)
;; Also catches Schema.partial, Schema.required, Schema.pick, Schema.omit applied to structs
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_schema
      property: (property_identifier) @_method
      (#any-of? @_schema "Schema" "S")
      (#any-of? @_method "Struct" "partial" "required" "pick" "omit"))
    arguments: (arguments) @schema.fields)) @schema.node
```

Schema captures produce `SchemaDef` with `kind: EffectSchema` and `source_framework: "effect-schema"`. The `@schema.effect_tag` is stored as `SchemaDef.effect_tag`.

#### 4.7.4 Effect Error Channels (Data.TaggedError, Schema.TaggedError)

Effect's typed error channel is a core structural boundary. Errors in Effect are not strings — they are tagged, structured types that appear in the `E` channel of `Effect<A, E, R>`. Extracting these reveals the full error domain model.

```scheme
;; Data.TaggedError — simple tagged error
;; class NotFound extends Data.TaggedError("NotFound")<{ id: string }> {}
(class_declaration
  name: (type_identifier) @error.name
  (class_heritage
    (extends_clause
      value: (call_expression
        function: (call_expression
          function: (member_expression
            object: (identifier) @_data
            property: (property_identifier) @_tagged
            (#any-of? @_data "Data" "Schema")
            (#eq? @_tagged "TaggedError"))
          arguments: (arguments
            (string) @error.tag))
        arguments: (arguments) @error.fields)))) @error.node

;; Data.TaggedError without type params (no payload)
;; class Unauthorized extends Data.TaggedError("Unauthorized") {}
(class_declaration
  name: (type_identifier) @error.name
  (class_heritage
    (extends_clause
      value: (call_expression
        function: (member_expression
          object: (identifier) @_data
          property: (property_identifier) @_tagged
          (#any-of? @_data "Data" "Schema")
          (#eq? @_tagged "TaggedError"))
        arguments: (arguments
          (string) @error.tag))))) @error.node

;; Schema.TaggedError — schema-validated error (has encode/decode)
;; class ValidationError extends Schema.TaggedError<ValidationError>("ValidationError")({
;;   field: Schema.String,
;;   message: Schema.String
;; }) {}
(class_declaration
  name: (type_identifier) @error.name
  (class_heritage
    (extends_clause
      value: (call_expression
        function: (call_expression
          function: (call_expression
            function: (member_expression
              object: (identifier) @_schema
              property: (property_identifier) @_tagged
              (#eq? @_schema "Schema")
              (#eq? @_tagged "TaggedError")))
          arguments: (arguments
            (string) @error.tag))
        arguments: (arguments
          (_) @error.fields))))) @error.node
```

Error captures produce `EffectErrorChannelDef`. The `@error.tag` is the discriminant (`_tag` field at runtime). The `@error.fields` are sub-parsed into `Vec<SchemaField>` for the structured error payload.

#### 4.7.5 Effect Pipelines (pipe / flow / Effect.gen)

`pipe` and `flow` are the primary composition patterns in Effect. Extracting these reveals how effects are chained, what services they require, and what errors they can produce.

```scheme
;; Named pipe composition
;; const getUser = pipe(readId, Effect.flatMap(fetchUser), Effect.catchTag("NotFound", handleNotFound))
(variable_declarator
  name: (identifier) @pipeline.name
  value: (call_expression
    function: (identifier) @_pipe
    (#any-of? @_pipe "pipe" "flow")
    arguments: (arguments) @pipeline.steps)) @pipeline.node

;; Effect.gen — generator-based composition
;; const getUser = Effect.gen(function*() { const repo = yield* UserRepo; ... })
(variable_declarator
  name: (identifier) @pipeline.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @_effect
      property: (property_identifier) @_gen
      (#eq? @_effect "Effect")
      (#eq? @_gen "gen"))
    arguments: (arguments
      (function_expression) @pipeline.body))) @pipeline.node

;; Method-chain pipe (fluent API)
;; const getUser = readId.pipe(Effect.flatMap(fetchUser), Effect.catchTag("NotFound", handleNotFound))
(variable_declarator
  name: (identifier) @pipeline.name
  value: (call_expression
    function: (member_expression
      property: (property_identifier) @_pipe
      (#eq? @_pipe "pipe"))
    arguments: (arguments) @pipeline.steps)) @pipeline.node
```

Pipeline captures produce `EffectPipelineDef`. The `@pipeline.steps` are sub-parsed to extract combinator names (`Effect.flatMap`, `Effect.catchTag`, `Effect.provide`, etc.), which reveal error handling, dependency provision, and data flow.

#### 4.7.6 Effect.ts Capture Conventions

```
@service.name        -> ServiceDef.name (with kind: EffectService)
@service.effect_tag  -> ServiceDef.effect_tag (the Context.Tag string identifier)
@service.shape       -> ServiceDef.methods (sub-parsed from the shape type literal)
@service.node        -> ServiceDef (full node for span)
@layer.name          -> EffectLayerDef.name
@layer.provides      -> EffectLayerDef.provides
@layer.body          -> EffectLayerDef (sub-parsed for yield* dependencies)
@layer.composed_layers -> EffectLayerDef.composition
@layer.deps          -> EffectLayerDef.requires
@layer.node          -> EffectLayerDef (full node for span)
@error.name          -> EffectErrorChannelDef.name
@error.tag           -> EffectErrorChannelDef.tag (_tag discriminant)
@error.fields        -> EffectErrorChannelDef.fields (sub-parsed)
@error.node          -> EffectErrorChannelDef (full node for span)
@pipeline.name       -> EffectPipelineDef.name
@pipeline.steps      -> EffectPipelineDef.steps (sub-parsed combinator list)
@pipeline.body       -> EffectPipelineDef (sub-parsed for yield* and dependency usage)
@pipeline.node       -> EffectPipelineDef (full node for span)
```

#### 4.7.7 Effect.ts Test Fixtures

```
tests/fixtures/typescript/
├── effect_service.ts           # Context.Tag, Effect.Service, Context.GenericTag patterns
├── effect_layer.ts             # Layer.effect, Layer.sync, Layer.succeed, Layer.merge, pipe composition
├── effect_schema.ts            # Schema.Struct, Schema.Class, Schema.TaggedStruct, Schema.Union
├── effect_errors.ts            # Data.TaggedError, Schema.TaggedError with/without payloads
├── effect_pipelines.ts         # pipe, flow, Effect.gen, method-chain .pipe
├── effect_full_app.ts          # Realistic Effect app combining all patterns (services + layers + errors + schemas)
├── expected/
│   ├── effect_service.json
│   ├── effect_layer.json
│   ├── effect_schema.json
│   ├── effect_errors.json
│   ├── effect_pipelines.json
│   └── effect_full_app.json
```

**`effect_full_app.ts` fixture** — a realistic mini-application:

```typescript
import { Context, Data, Effect, Layer, Schema } from "effect"

// --- Error channel ---
class NotFound extends Data.TaggedError("NotFound")<{ readonly id: string }> {}
class Unauthorized extends Data.TaggedError("Unauthorized") {}
class ValidationError extends Schema.TaggedError<ValidationError>("ValidationError")({
  field: Schema.String,
  message: Schema.String,
}) {}

// --- Schemas ---
const UserId = Schema.String.pipe(Schema.brand("UserId"))
const User = Schema.Struct({
  id: UserId,
  name: Schema.String,
  email: Schema.String,
})
class CreateUserRequest extends Schema.Class<CreateUserRequest>("CreateUserRequest")({
  name: Schema.String,
  email: Schema.String,
}) {}
const UserEvent = Schema.Union(
  Schema.TaggedStruct("UserCreated")({ userId: UserId }),
  Schema.TaggedStruct("UserDeleted")({ userId: UserId }),
)

// --- Services ---
class UserRepo extends Context.Tag("UserRepo")<
  UserRepo,
  {
    readonly findById: (id: string) => Effect.Effect<typeof User.Type, NotFound>
    readonly create: (data: typeof CreateUserRequest.Type) => Effect.Effect<typeof User.Type, ValidationError>
  }
>() {}

class EventBus extends Context.Tag("EventBus")<
  EventBus,
  { readonly publish: (event: typeof UserEvent.Type) => Effect.Effect<void> }
>() {}

// --- Layers ---
const UserRepoLive = Layer.effect(
  UserRepo,
  Effect.gen(function* () {
    // ... implementation
    return UserRepo.of({ findById: (id) => Effect.succeed({} as any), create: (data) => Effect.succeed({} as any) })
  })
)

const EventBusLive = Layer.succeed(EventBus, { publish: () => Effect.void })

const AppLive = Layer.merge(UserRepoLive, EventBusLive)

// --- Pipelines ---
const getUser = (id: string) =>
  Effect.gen(function* () {
    const repo = yield* UserRepo
    return yield* repo.findById(id)
  })

const createUser = (data: typeof CreateUserRequest.Type) =>
  pipe(
    Effect.gen(function* () {
      const repo = yield* UserRepo
      const bus = yield* EventBus
      const user = yield* repo.create(data)
      yield* bus.publish({ _tag: "UserCreated", userId: user.id })
      return user
    }),
    Effect.catchTag("ValidationError", (e) => Effect.fail(e)),
  )
```

**Expected extraction from `effect_full_app.ts`:**
- 3 `EffectErrorChannelDef`: `NotFound` (tag: "NotFound", 1 field), `Unauthorized` (tag: "Unauthorized", 0 fields), `ValidationError` (tag: "ValidationError", 2 fields)
- 4 `SchemaDef` (kind: EffectSchema): `UserId`, `User`, `CreateUserRequest`, `UserEvent`
- 2 `ServiceDef` (kind: EffectService): `UserRepo` (2 methods), `EventBus` (1 method)
- 3 `EffectLayerDef`: `UserRepoLive` (provides: UserRepo), `EventBusLive` (provides: EventBus), `AppLive` (composition: [UserRepoLive, EventBusLive])
- 2 `EffectPipelineDef`: `getUser` (gen, requires: UserRepo), `createUser` (pipe+gen, requires: UserRepo+EventBus, catches: ValidationError)

**IR type for schemas:**

```rust
pub struct SchemaDef {
    pub name: String,
    pub file: PathBuf,
    pub span: Span,
    pub kind: SchemaKind,
    pub fields: Vec<SchemaField>,       // Sub-parsed from fields_source
    pub source_framework: String,       // "zod", "effect-schema", "pydantic", "drizzle", etc.
    pub table_name: Option<String>,     // For DB schema definitions
    pub derives: Vec<String>,           // Rust derives
    pub visibility: Visibility,
    pub effect_tag: Option<String>,     // Effect.ts Schema.TaggedStruct/TaggedClass discriminant tag
    pub union_members: Vec<String>,     // Effect.ts Schema.Union member names
}

pub enum SchemaKind {
    ValidationSchema,   // Zod, Effect Schema, io-ts, Yup
    OrmModel,           // Pydantic, SQLAlchemy, TypeORM, Prisma, Drizzle
    DataTransfer,       // Rust serde structs, Go tagged structs, Java records, Kotlin data classes
    DomainEvent,        // Event schema definitions
    EffectSchema,       // Effect.ts Schema.Struct, Schema.Class, Schema.TaggedStruct, Schema.Union
}

pub struct SchemaField {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub is_primary_key: bool,           // For DB schemas
    pub constraints: Vec<String>,       // "unique", "nullable", "default(...)"
}
```

### 4.7 Language Coverage Matrix

| Language   | Interfaces | Classes | Methods | Functions | Services | Schemas | Imports | Impls |
|------------|:----------:|:-------:|:-------:|:---------:|:--------:|:-------:|:-------:|:-----:|
| TypeScript |     x      |    x    |    x    |     x     |    x     |    x    |    x    |   -   |
| Python     |     x      |    x    |    x    |     x     |    x     |    x    |    x    |   -   |
| Rust       |     x      |    -    |    x    |     x     |    x     |    x    |    x    |   x   |
| Go         |     x      |    x    |    x    |     x     |    x     |    x    |    x    |   -   |
| Java       |     x      |    x    |    x    |     -     |    x     |    x    |    x    |   -   |
| Kotlin     |     x      |    x    |    x    |     x     |    x     |    x    |    x    |   -   |
| C#         |     x      |    x    |    x    |     -     |    x     |    -    |    x    |   -   |
| Swift      |     x      |    x    |    x    |     x     |    -     |    -    |    x    |   x   |
| PHP        |     x      |    x    |    x    |     x     |    -     |    -    |    x    |   -   |
| Ruby       |     x      |    x    |    x    |     -     |    -     |    -    |    x    |   -   |
| Scala      |     x      |    x    |    x    |     x     |    -     |    -    |    x    |   -   |
| C++        |     x      |    x    |    x    |     x     |    -     |    -    |    x    |   -   |

"x" = query file exists. "-" = not applicable or deferred.

---

## 5. CLI Interface

### 5.0 Tree Navigation UX

When the CLI outputs hierarchical data (interfaces with methods, classes with properties, services with routes), parent-to-child navigation must work on a **single action**. No double-click or triple-click to drill into children.

**Rules:**
1. **Table mode** (`--output table`): Hierarchy is pre-expanded. Parent rows show the entity name, child rows are indented below. No interaction needed.
2. **TUI mode** (`--interactive`): Tree nodes expand on a single `Enter` keypress. Arrow keys navigate. `q` quits. Parent nodes show a `>` indicator when collapsed, `v` when expanded. First `Enter` on a collapsed parent expands it. Second `Enter` on the same parent collapses it. Children are immediately visible after one keypress.
3. **JSON mode** (`--output json`): Children are nested inline (methods inside interfaces, routes inside services). No separate expansion step.
4. **Compact mode** (`--output compact`): One line per entity. `interface:UserRepo.findById` style dotted paths. No hierarchy, just flat searchable output.

**TUI interaction model:**
```
> UserRepository (5 methods)         # Collapsed. Press Enter once.
v UserRepository (5 methods)         # Expanded after single Enter.
    findById(id: string): Promise<User>
    findAll(): Promise<User[]>
    create(data: CreateUserDto): Promise<User>
    update(id: string, data: UpdateUserDto): Promise<User>
    delete(id: string): Promise<void>
> EventHandler (3 methods)           # Next parent, still collapsed.
```

**Implementation:** Use `ratatui` for TUI mode. Tree state is a `Vec<(entity, expanded: bool)>`. Enter toggles `expanded`. Rendering walks the tree and skips children of collapsed nodes.

### 5.1 Top-Level Commands

```
domain-scan <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    scan        Run a full structural scan of a directory
    interfaces  List all interfaces / traits / protocols
    services    List all service definitions
    methods     List all methods (optionally filtered by owner)
    schemas     List all runtime schema definitions (Zod, Effect, Pydantic, Drizzle, etc.)
    impls       List implementations of a trait/interface
    effect      Effect.ts structural queries (layers, errors, pipelines, services, schemas)
    search      Full-text search across names and types
    stats       Print scan statistics
    validate    Run data quality checks on scan results (naming, completeness, duplicates)
    match       Match extracted entities to subsystems and report unmatched items
    prompt      Generate an LLM prompt with sub-agent dispatch instructions
    cache       Cache management (clear, stats)

GLOBAL OPTIONS:
    --root <PATH>           Root directory to scan (default: .)
    --config <PATH>         Config file path (default: .domain-scan.toml)
    --output <FORMAT>       Output format: json | table | compact (default: table)
    -o, --out <FILE>        Write output to file (default: stdout)
    --interactive           Launch TUI mode (ratatui). Single-keypress tree navigation.
    --build-status <STATUS> Override build status detection: built | unbuilt | error | rebuild
    --no-cache              Disable caching
    --include <GLOB>        Include only matching paths (repeatable)
    --exclude <GLOB>        Exclude matching paths (repeatable)
    --languages <LANG,...>  Only scan these languages
    -q, --quiet             Suppress progress output
    -v, --verbose           Verbose output (timing, cache stats)
```

### 5.2 Subcommand Details

#### `domain-scan scan`

Full structural scan. Produces a `ScanIndex` as JSON.

```bash
# Scan current directory, output JSON
domain-scan scan --output json -o scan.json

# Scan specific directory, only TypeScript and Rust
domain-scan scan --root ./src --languages typescript,rust

# Scan with custom config
domain-scan scan --config my-config.toml
```

#### `domain-scan interfaces`

List all interface-like definitions (interfaces, traits, protocols, ABCs).

```bash
# All interfaces
domain-scan interfaces

# Filter by language
domain-scan interfaces --languages rust,go

# Filter by name pattern (regex)
domain-scan interfaces --name ".*Repository"

# Show methods inline
domain-scan interfaces --show-methods

# JSON output for piping
domain-scan interfaces --output json | jq '.[] | select(.methods | length > 5)'
```

**Table output:**
```
 Language   | Kind      | Name                | Methods | Extends        | File
------------+-----------+---------------------+---------+----------------+----------------------------
 TypeScript | interface | UserRepository      |       5 | BaseRepository | src/repos/user.ts:12
 Rust       | trait     | EventHandler        |       3 | Send + Sync    | src/events/handler.rs:8
 Go         | interface | Storage             |       4 |                | pkg/storage/storage.go:15
 Python     | protocol  | Serializable        |       2 |                | core/types.py:44
 Java       | interface | PaymentGateway      |       7 | Gateway        | src/.../PaymentGateway.java:3
```

#### `domain-scan services`

List service definitions (HTTP controllers, gRPC services, workers, etc.).

```bash
# All services
domain-scan services

# Filter by kind
domain-scan services --kind http-controller,grpc-service

# Show routes
domain-scan services --show-routes

# Show injected dependencies
domain-scan services --show-deps
```

**Table output:**
```
 Language   | Kind            | Name              | Methods | Routes | File
------------+-----------------+-------------------+---------+--------+---------------------------
 TypeScript | http-controller | UserController    |       8 |      6 | src/controllers/user.ts:5
 Python     | http-controller | AuthRouter        |       4 |      4 | api/auth.py:12
 Rust       | grpc-service    | OrderService      |       5 |      - | src/grpc/orders.rs:22
 Java       | microservice    | PaymentService    |      12 |      8 | src/.../PaymentService.java:10
```

#### `domain-scan methods`

List methods, optionally filtered.

```bash
# All public async methods
domain-scan methods --async --visibility public

# Methods on a specific class/trait
domain-scan methods --owner UserRepository

# Methods matching a pattern
domain-scan methods --name "get.*" --output json

# Methods with specific parameter types
domain-scan methods --param-type "Request"
```

#### `domain-scan impls`

Show which types implement a given interface/trait.

```bash
# Who implements EventHandler?
domain-scan impls EventHandler

# All trait implementations in the project
domain-scan impls --all

# Show method coverage (which methods are implemented)
domain-scan impls EventHandler --show-methods
```

**Table output:**
```
 Trait: EventHandler (src/events/handler.rs:8)
 Methods: on_event, on_error, cleanup

 Implementor          | File                        | Methods Implemented
----------------------+-----------------------------+--------------------
 LogEventHandler      | src/events/log.rs:15        | 3/3 (complete)
 MetricsHandler       | src/events/metrics.rs:8     | 2/3 (missing: cleanup)
 AuditEventHandler    | src/events/audit.rs:22      | 3/3 (complete)
```

#### `domain-scan search`

Full-text search across all names and type annotations.

```bash
# Find anything named "auth"
domain-scan search auth

# Search with kind filter
domain-scan search --kind interface,service "payment"

# Regex search
domain-scan search --regex "handle.*Event"
```

#### `domain-scan stats`

Print scan statistics.

```bash
domain-scan stats
```

```
 Scan Statistics
 ───────────────────────────────
 Root:           /home/user/project
 Files scanned:  342
 Languages:      TypeScript (180), Rust (95), Python (67)

 Interfaces:     48
 Services:       12
 Classes:        156
 Methods:        1,847
 Functions:      423
 Type aliases:   89
 Implementations: 67

 Parse time:     1.2s (342 files, 284 cached)
 Cache hit rate: 83%
```

#### `domain-scan prompt`

Generate an LLM prompt with sub-agent dispatch instructions. This is the key command for LLM-assisted codebase exploration.

```bash
# Generate a prompt for exploring this codebase
domain-scan prompt --root ./src --agents 5

# Generate prompt targeting specific concerns
domain-scan prompt --focus "authentication,authorization" --agents 3

# Generate prompt with scan results embedded
domain-scan prompt --include-scan --agents 4 -o prompt-output.md
```

See Section 7 for full prompt generation spec.

#### `domain-scan schemas`

List runtime schema definitions (Zod, Effect.ts Schema, Pydantic, Drizzle, serde, data classes, etc.).

```bash
# All schemas
domain-scan schemas

# Filter by framework
domain-scan schemas --framework zod,effect-schema,drizzle

# Filter by kind
domain-scan schemas --kind orm-model,validation-schema

# Show fields inline
domain-scan schemas --show-fields

# JSON for piping
domain-scan schemas --output json
```

**Table output:**
```
 Language   | Framework      | Kind             | Name           | Fields | File
------------+----------------+------------------+----------------+--------+---------------------------
 TypeScript | effect-schema  | validation       | UserSchema     |      5 | src/schemas/user.ts:12
 TypeScript | zod            | validation       | LoginInput     |      3 | src/schemas/auth.ts:8
 TypeScript | drizzle        | orm-model        | users          |      7 | src/db/schema.ts:15
 Python     | pydantic       | validation       | CreateUserDto  |      4 | api/dto.py:22
 Rust       | serde          | data-transfer    | ApiResponse    |      3 | src/types.rs:44
 Kotlin     | data-class     | data-transfer    | UserEvent      |      5 | src/events/User.kt:10
```

#### `domain-scan effect`

Query Effect.ts-specific structural entities: services, layers, errors, schemas, and pipelines. This is a convenience subcommand that filters across multiple IR types to show the full Effect.ts dependency graph and domain model.

```bash
# All Effect.ts entities
domain-scan effect

# Effect services (Context.Tag)
domain-scan effect services

# Effect layers — shows provides/requires/composition graph
domain-scan effect layers
domain-scan effect layers --show-graph      # ASCII dependency graph

# Effect errors — tagged error types with payloads
domain-scan effect errors

# Effect schemas — Schema.Struct, Schema.Class, Schema.Union, Schema.TaggedStruct
domain-scan effect schemas

# Effect pipelines — pipe/flow/Effect.gen chains with combinator steps
domain-scan effect pipelines

# Full dependency graph: which layers provide which services, which pipelines require which services
domain-scan effect graph --output json

# Filter by name
domain-scan effect services --name ".*Repo"
domain-scan effect errors --name ".*Error"
```

**Table output (`domain-scan effect layers`):**
```
 Name               | Kind       | Provides       | Requires           | Composition
--------------------+------------+----------------+--------------------+-----------------------------
 UserRepoLive       | effect     | UserRepo       | Db                 |
 AuthLive           | effect     | Auth           | UserRepo, Config   |
 EventBusLive       | succeed    | EventBus       |                    |
 AppLive            | merge      |                |                    | UserRepoLive, AuthLive, EventBusLive
```

**Table output (`domain-scan effect errors`):**
```
 Name               | Tag              | Fields                          | File
--------------------+------------------+---------------------------------+---------------------------
 NotFound           | NotFound         | id: string                      | src/errors.ts:3
 Unauthorized       | Unauthorized     |                                 | src/errors.ts:5
 ValidationError    | ValidationError  | field: string, message: string  | src/errors.ts:7
```

**Graph output (`domain-scan effect graph`):**
```
EventBus ──→ EventBusLive (succeed)
UserRepo ──→ UserRepoLive (effect) ──requires──→ Db
Auth ──→ AuthLive (effect) ──requires──→ UserRepo, Config
AppLive (merge) ══► UserRepoLive + AuthLive + EventBusLive

Pipelines:
  getUser ──uses──→ UserRepo ──errors──→ NotFound
  createUser ──uses──→ UserRepo, EventBus ──errors──→ ValidationError
```

#### `domain-scan validate`

Run data quality checks on scan results. Inspired by octospark-visualizer's `system-invariants.test.ts` pattern. Each language has built-in validation rules that enforce naming conventions and structural completeness.

```bash
# Run all validations
domain-scan validate

# Validate specific rules
domain-scan validate --rules naming,completeness,duplicates

# Validate against a subsystem manifest (system.json style)
domain-scan validate --manifest system.json

# JSON output for CI
domain-scan validate --output json
```

**Built-in validation rules per language:**

| Rule | TypeScript | Python | Rust | Go | Java |
|------|:----------:|:------:|:----:|:--:|:----:|
| Interfaces are PascalCase | x | x | x | x | x |
| Methods are camelCase/snake_case (per language convention) | x | x | x | x | x |
| No duplicate interface names within a module | x | x | x | x | x |
| No duplicate method names within an interface | x | x | x | x | x |
| Every interface has at least 1 method | x | x | x | x | x |
| Every service has at least 1 route/method | x | x | x | x | x |
| Schema fields have type annotations | x | x | x | x | x |
| No god-interfaces (>10 methods) | x | x | x | x | x |
| No god-services (>15 methods) | x | x | x | x | x |
| Every public interface has at least 1 implementor | x | x | x | x | x |
| **Effect.ts-specific rules** | | | | | |
| Every Effect service has a matching Layer | x | | | | |
| Every Layer.provides references a real service tag | x | | | | |
| No orphan layers (layer exists but service tag is unused) | x | | | | |
| Every `yield*` in Effect.gen resolves to a known service | x | | | | |
| Error tags are unique (no duplicate `_tag` values) | x | | | | |
| Schema.Union members all exist as defined schemas | x | | | | |

**Table output:**
```
 Rule                          | Status | Violations
-------------------------------+--------+------------------------------------------
 Interfaces are PascalCase     | PASS   | 0
 No duplicate interface names  | PASS   | 0
 No god-interfaces (>10)       | WARN   | 1: AdminController (14 methods)
 Every interface has impls     | FAIL   | 3: Cacheable, Auditable, Retryable
 Schema fields have types      | WARN   | 2 fields missing types in UserSchema
```

#### `domain-scan match`

Match extracted entities (interfaces, schemas, services, methods) to subsystems defined in a manifest file (like octospark-visualizer's `system.json`). The goal is to **reduce unmatched items to zero**. Unmatched items are flagged for human review or LLM enrichment.

```bash
# Match against a manifest
domain-scan match --manifest system.json

# Show only unmatched
domain-scan match --manifest system.json --unmatched-only

# Generate LLM prompt to resolve unmatched items
domain-scan match --manifest system.json --prompt-unmatched --agents 3

# JSON output for downstream processing
domain-scan match --manifest system.json --output json
```

**Matching strategy:**

1. **File path matching**: If an entity's file path falls under a subsystem's `filePath`, it belongs to that subsystem.
2. **Import graph matching**: If entity A imports from subsystem B's files, A relates to B.
3. **Name matching**: Schema/interface names that match a subsystem's known `interfaces[]` list.
4. **Unmatched bucket**: Everything that doesn't match goes into an "unmatched" report.

**The workflow:**

1. `domain-scan scan` extracts all entities from source code (deterministic, fast)
2. `domain-scan match --manifest system.json` maps entities to subsystems, reports unmatched
3. Human reviews unmatched items and either:
   - Updates the manifest to include them
   - Marks them as intentionally untracked
4. `domain-scan match --manifest system.json --prompt-unmatched` generates an LLM prompt for agents to propose where unmatched items belong
5. Repeat until unmatched count is zero

**Table output:**
```
 Subsystem: auth (5 matched, 0 unmatched)
 ─────────────────────────────────────────
  ✓ AuthPrincipal       interface    src/auth/types.ts:12
  ✓ SessionToken        interface    src/auth/session.ts:5
  ✓ users               schema/drizzle  src/db/schema.ts:15
  ✓ auth_sessions       schema/drizzle  src/db/schema.ts:28
  ✓ AuthService         service      src/auth/service.ts:8

 Subsystem: billing (3 matched, 2 unmatched)
 ─────────────────────────────────────────
  ✓ Invoice             interface    src/billing/types.ts:3
  ✓ Subscription        interface    src/billing/types.ts:18
  ✓ BillingService      service      src/billing/service.ts:5
  ✗ PaymentRetry        interface    src/billing/retry.ts:10    ← UNMATCHED
  ✗ stripe_events       schema/drizzle  src/db/schema.ts:44    ← UNMATCHED

 UNMATCHED (no subsystem):
  ✗ HealthCheck         interface    src/health.ts:3
  ✗ MetricsCollector    service      src/metrics/collector.ts:8

 Summary: 48 matched, 4 unmatched (92% coverage)
```

#### `domain-scan cache`

Cache management.

```bash
domain-scan cache stats     # Show cache size, hit rate, entries
domain-scan cache clear     # Clear all cached entries
domain-scan cache prune     # Remove entries for deleted files
```

---

## 6. Configuration (.domain-scan.toml)

```toml
[project]
name = "my-project"
root = "."                              # Scan root (relative to config file)

[scan]
include = ["src/**", "lib/**"]          # Only scan these paths
exclude = [
    "**/node_modules/**",
    "**/target/**",
    "**/.git/**",
    "**/vendor/**",
    "**/__pycache__/**",
    "**/dist/**",
    "**/build/**",
    "**/*.test.*",
    "**/*.spec.*",
    "**/*_test.go",
]
languages = []                          # Empty = all detected languages
follow_symlinks = false

[cache]
enabled = true
dir = ".domain-scan/cache"                 # Relative to project root
max_size_mb = 100

[services]
# Custom service detection patterns
# These supplement the built-in framework detection
[[services.custom]]
name = "DomainService"
pattern = "**/*Service.ts"              # File glob
decorator = "@DomainService"            # Required decorator (optional)
kind = "microservice"

[[services.custom]]
name = "EventProcessor"
pattern = "src/processors/**/*.rs"
trait_name = "EventProcessor"           # Required trait impl (optional)
kind = "event-handler"

[output]
default_format = "table"                # json | table | compact
show_file_paths = true                  # Show full paths or relative
sort_by = "name"                        # name | file | kind | methods
```

---

## 7. LLM Sub-Agent Prompt Generation

### 7.1 Purpose

The `domain-scan prompt` command generates a structured LLM prompt that instructs an LLM orchestrator to launch 3-5 sub-agents, each responsible for scanning a partition of the codebase. This is for codebases too large to fit in a single context window, or when parallel analysis is faster.

### 7.2 Prompt Output Structure

```markdown
# Codebase Structural Analysis: {project_name}

## Context

You are analyzing the codebase at `{root_path}`.
A structural scan has identified the following high-level statistics:

{embedded scan stats}

## Your Task

Launch {n_agents} sub-agents to perform a deep structural analysis of this codebase.
Each sub-agent should scan its assigned partition and report back with findings.

## Sub-Agent Assignments

### Agent 1: Interface & Type Boundary Audit
**Scope:** All interface, trait, and protocol definitions
**Directory focus:** {auto-partitioned dirs}
**Instructions:**
1. Read every interface/trait/protocol definition in your assigned files
2. For each, document: name, methods, extends chain, which types implement it
3. Flag any interface with >10 methods (possible god-interface)
4. Flag any interface with 0 implementors (dead interface)
5. Flag any partial implementations (missing methods)

**Files to scan:**
```
{list of files containing interfaces, from scan results}
```

### Agent 2: Service Architecture Map
**Scope:** All service definitions (HTTP controllers, gRPC, workers, etc.)
**Directory focus:** {auto-partitioned dirs}
**Instructions:**
1. Read every service definition in your assigned files
2. Document: name, kind, routes/methods, injected dependencies
3. Map the dependency graph between services
4. Flag any service with >15 methods (possible god-service)
5. Flag circular dependencies between services

**Files to scan:**
```
{list of files containing services, from scan results}
```

### Agent 3: Method Signature Census
**Scope:** All public methods across classes, structs, impls
**Directory focus:** {auto-partitioned dirs}
**Instructions:**
1. Catalog all public methods with their full signatures
2. Group by owner (class/struct/trait)
3. Flag inconsistent naming patterns (mixedCase vs snake_case in same module)
4. Flag methods with >5 parameters (possible refactor target)
5. Identify async/sync boundary crossings

**Files to scan:**
```
{list of files containing methods, from scan results}
```

### Agent 4: Cross-Cutting Concerns
**Scope:** Decorators, middleware, annotations, generic constraints
**Directory focus:** {auto-partitioned dirs}
**Instructions:**
1. Catalog all decorator/annotation usage patterns
2. Identify middleware chains and their ordering
3. Map generic type parameter constraints
4. Flag unused or redundant decorators
5. Document the authentication/authorization boundary

**Files to scan:**
```
{list of files with decorators/annotations, from scan results}
```

### Agent 5: Implementation Completeness Audit
**Scope:** All impl blocks, class implementations, protocol conformances
**Directory focus:** {auto-partitioned dirs}
**Instructions:**
1. For every interface/trait, verify all implementations are complete
2. Document which methods have default implementations vs required
3. Flag orphaned implementations (impl for trait that doesn't exist)
4. Map the inheritance/composition hierarchy
5. Identify diamond inheritance or conflicting implementations

**Files to scan:**
```
{list of files with impl blocks, from scan results}
```

## Synthesis

After all agents complete, synthesize findings into:

1. **Architecture Map**: Top-level service → interface → implementation hierarchy
2. **Health Report**: God objects, dead interfaces, incomplete impls, circular deps
3. **API Surface**: Complete public API with method signatures
4. **Recommendations**: Specific refactoring suggestions with file:line references

## Output Format

Each agent should return structured JSON:
```json
{
  "agent_id": 1,
  "scope": "Interface & Type Boundary Audit",
  "findings": [...],
  "flags": [...],
  "file_count": 42,
  "entity_count": 156
}
```
```

### 7.3 Partitioning Strategy

The prompt generator partitions the codebase intelligently:

1. **By concern** (default): Each agent gets a structural category (interfaces, services, methods, cross-cutting, impls)
2. **By directory**: For very large codebases, split by top-level directories
3. **By language**: For polyglot repos, one agent per language
4. **By build status**: Separate built (verify) from unbuilt/rebuild (analyze and infer)
5. **Hybrid**: Combine concern + directory + build status for the largest codebases

The partition strategy is chosen automatically based on scan results:
- < 500 files: by concern (5 agents)
- 500-2000 files: hybrid (3-5 agents)
- \> 2000 files: by directory with concern sub-partitions (5 agents)

**Build-status-aware agent instructions:**

For `Built` files, agents get:
> "These files are from modules that compile successfully. The domain-scan structural output is authoritative. Verify the scan results are complete, catalog any patterns the static analysis missed (e.g. runtime registration, reflection-based DI), and document the architecture."

For `Unbuilt` / `Error` / `Rebuild` files, agents get:
> "These files are from modules that do not currently build. The domain-scan output is a best-effort extraction. Read each file carefully. Infer the intended interfaces and services from naming patterns, comments, and partial definitions. Flag conflicts between old and new definitions. Mark your findings with confidence levels."

### 7.4 Scan Result Embedding

When `--include-scan` is passed, the prompt includes the full `domain-scan scan` JSON output. This gives the LLM agents a structural map before they start reading files, reducing wasted token spend on discovery.

---

## 8. Tauri Desktop App

### 8.1 Overview

domain-scan ships a Tauri 2 desktop app (`domain-scan-tauri`) that provides a visual structural explorer. The app wraps `domain-scan-core` directly (no CLI subprocess), giving instant access to all scan, filter, and prompt generation features with a native GUI.

### 8.2 Layout

**Three-panel layout:**

```
┌─────────────────┬──────────────────────────┬─────────────────────┐
│  Entity Tree    │     Source Preview        │   Details Panel     │
│                 │                           │                     │
│ v UserRepo      │  interface UserRepository │ Build Status: Built │
│   findById      │    findById(id: string)   │ Confidence: High    │
│   findAll       │    findAll(): User[]      │ File: src/repos/... │
│   create        │    create(data): User     │ Extends: BaseRepo   │
│   update        │    ...                    │ Implementors: 3     │
│   delete        │                           │  - PgUserRepo       │
│ > EventHandler  │                           │  - MockUserRepo     │
│ > OrderService  │                           │  - CachedUserRepo   │
│                 │                           │                     │
│ [Filter: ___]   │                           │ [Generate Prompt]   │
└─────────────────┴──────────────────────────┴─────────────────────┘
```

1. **Left: Entity Tree.** Hierarchical tree of all scanned entities (interfaces, services, classes, functions). Single click expands parent to show children (methods, properties, routes). Filter bar at bottom. Color-coded by build status (green = Built, yellow = Unbuilt, red = Error, orange = Rebuild).
2. **Center: Source Preview.** Syntax-highlighted source code for the selected entity. Scrolls to the exact span. Read-only.
3. **Right: Details Panel.** Metadata for the selected entity: build status, confidence, file path, extends/implements chain, implementors list, method signatures, decorators. "Generate Prompt" button creates LLM sub-agent prompt for the selected scope.

### 8.3 Tree Navigation UX

**Critical requirement:** Parent nodes expand on a single click. No double-click or triple-click.

- Single click on collapsed `>` node: expands to show children, selects the parent
- Single click on expanded `v` node: collapses children, keeps parent selected
- Single click on a child (method/property): selects it, scrolls source preview to its span
- Arrow keys: up/down navigate the visible tree. Right arrow expands, left arrow collapses.
- `/` or `Ctrl+F`: focus filter bar
- `Enter` on a node: open file in external editor (VS Code, Cursor, terminal)

### 8.4 Build Status Indicators

Every entity in the tree shows its build status visually:
- **Green dot** or no indicator: `Built` (high confidence, source is truth)
- **Yellow dot**: `Unbuilt` (low confidence, needs LLM enrichment)
- **Red dot**: `Error` (mixed confidence, module has build errors)
- **Orange dot**: `Rebuild` (unreliable, active refactor)

The details panel shows a prominent banner for non-Built entities:
> "This module does not currently build. Extracted interfaces are best-effort. Use 'Generate Prompt' to dispatch LLM agents for enrichment."

### 8.5 Features

- **Scan on open.** When a directory is opened, domain-scan runs automatically. Progress bar in the status bar.
- **Filter by kind.** Toggle buttons: Interfaces | Services | Classes | Functions | All.
- **Filter by build status.** Toggle: Built | Unbuilt | Error | Rebuild | All.
- **Filter by language.** Dropdown or toggle buttons for detected languages.
- **Search.** Fuzzy search across all entity names. Updates tree in real-time.
- **Prompt generation.** Select entities in the tree, click "Generate Prompt", get a ready-to-paste LLM sub-agent prompt scoped to the selection.
- **Export.** Export current view as JSON, CSV, or Markdown.
- **Keyboard-driven.** All actions accessible via keyboard. `j`/`k` for navigation, `/` for search, `Enter` to open, `p` for prompt, `e` for export.

### 8.6 Crate Structure

```
crates/domain-scan-tauri/
├── Cargo.toml
├── src/
│   ├── main.rs           # Tauri setup, window config
│   ├── lib.rs
│   └── commands.rs       # Tauri IPC commands (scan, filter, search, prompt)
├── ui/                   # Frontend (built with Tauri's webview)
│   ├── src/
│   │   ├── App.tsx       # Main three-panel layout
│   │   ├── components/
│   │   │   ├── EntityTree.tsx      # Left panel: tree with single-click expand
│   │   │   ├── SourcePreview.tsx   # Center panel: syntax-highlighted code
│   │   │   ├── DetailsPanel.tsx    # Right panel: metadata + prompt gen
│   │   │   ├── FilterBar.tsx       # Kind/status/language filters
│   │   │   └── SearchBar.tsx       # Fuzzy entity search
│   │   ├── hooks/
│   │   │   ├── useScan.ts          # IPC wrapper for scan commands
│   │   │   └── useTreeState.ts     # Tree expand/collapse state management
│   │   ├── types.ts
│   │   └── styles.css
│   ├── package.json
│   ├── tsconfig.json
│   └── vite.config.ts
├── tauri.conf.json
└── icons/
```

### 8.7 Application State and Error Type

Following flowdiff's pattern: state is held server-side in `Mutex`, commands read from state. The `ScanIndex` is never passed over IPC (it can be megabytes).

```rust
pub struct AppState {
    pub current_index: Mutex<Option<ScanIndex>>,
    pub current_root: Mutex<Option<PathBuf>>,
}

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

// Serialize as plain string for Tauri IPC (same pattern as flowdiff)
impl serde::Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
```

### 8.8 IPC Commands

All commands read from `state: tauri::State<'_, AppState>`. No command accepts `ScanIndex` as a parameter.

```rust
// Scan a directory, populate AppState. Returns stats only (not the full index).
#[tauri::command]
async fn scan_directory(root: String, state: State<'_, AppState>) -> Result<ScanStats, CommandError>;

// Check if a scan is loaded (for startup / empty state detection).
#[tauri::command]
fn get_current_scan(state: State<'_, AppState>) -> Result<Option<ScanStats>, CommandError>;

// Filter entities from the loaded index. Reads from AppState.
#[tauri::command]
fn filter_entities(filters: FilterParams, state: State<'_, AppState>) -> Result<Vec<EntitySummary>, CommandError>;

// Get full details for a specific entity.
#[tauri::command]
fn get_entity_detail(name: String, file: String, state: State<'_, AppState>) -> Result<Entity, CommandError>;

// Get source code for a specific span.
#[tauri::command]
fn get_entity_source(file: String, start_byte: usize, end_byte: usize) -> Result<String, CommandError>;

// Search entities by name (fuzzy).
#[tauri::command]
fn search_entities(query: String, state: State<'_, AppState>) -> Result<Vec<EntitySummary>, CommandError>;

// Generate LLM sub-agent prompt scoped to selected entities.
#[tauri::command]
fn generate_prompt(entity_ids: Vec<String>, agents: u8, state: State<'_, AppState>) -> Result<String, CommandError>;

// Export current view as JSON, CSV, or Markdown.
#[tauri::command]
fn export_entities(format: String, filters: FilterParams, state: State<'_, AppState>) -> Result<String, CommandError>;

// Get build status for all modules.
#[tauri::command]
fn get_build_status(state: State<'_, AppState>) -> Result<HashMap<PathBuf, BuildStatus>, CommandError>;

// Open a file in the user's editor.
#[tauri::command]
fn open_in_editor(editor: String, file: String, line: usize) -> Result<(), CommandError>;

// Check which editors are available on this system.
#[tauri::command]
fn check_editors_available() -> HashMap<String, bool>;
```

### 8.9 tauri.conf.json

```json
{
  "productName": "domain-scan",
  "identifier": "com.domain-scan.app",
  "build": {
    "beforeDevCommand": { "script": "npm run dev", "cwd": "ui" },
    "beforeBuildCommand": { "script": "npm run build", "cwd": "ui" },
    "devUrl": "http://localhost:5173",
    "frontendDist": "ui/dist"
  },
  "app": {
    "security": {
      "csp": "default-src 'self' tauri: asset: ipc: http://ipc.localhost; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: asset:; connect-src ipc: http://ipc.localhost tauri:; font-src 'self' data:;"
    },
    "windows": [{
      "label": "main",
      "title": "domain-scan",
      "width": 1440,
      "height": 900,
      "minWidth": 1200,
      "minHeight": 700,
      "resizable": true,
      "fullscreen": false
    }]
  },
  "bundle": { "active": true, "targets": "all" },
  "plugins": {}
}
```

### 8.10 Shared Core Pattern

**Critical architectural rule:** The Tauri IPC layer and the CLI are both thin wrappers over `domain-scan-core`. No business logic (filtering, matching, validation, prompt generation) lives in either wrapper. All query and analysis logic lives in `domain-scan-core::index` methods. Both wrappers:

1. Hold state (`AppState` for Tauri, parsed CLI args for the CLI)
2. Deserialize incoming parameters
3. Call `domain-scan-core` functions
4. Serialize the result

This prevents diverging implementations between Tauri and CLI.

---

## 10. Caching

### 10.1 Content-Addressed Cache

```rust
pub struct Cache {
    entries: DashMap<String, CachedFile>,  // key = SHA-256(path + content)
    dir: PathBuf,
    max_size: usize,
}

pub struct CachedFile {
    pub ir: IrFile,
    pub hash: String,
    pub last_accessed: Instant,
}
```

Cache key = `SHA-256(file_path + file_content)`. If the file content hasn't changed, the cached IR is returned without re-parsing.

### 10.2 Disk Persistence

- Cache is written to `.domain-scan/cache/` as individual `.bincode` files
- LRU eviction when `max_size_mb` is exceeded
- `domain-scan cache prune` removes entries for files that no longer exist

### 10.3 Thread Safety

- `DashMap` for concurrent reads/writes during parallel parsing
- No global locks. Each file's cache entry is independent.

---

## 10a. Manifest Schema (for `domain-scan match` and `domain-scan validate --manifest`)

### 10a.1 Compatibility with octospark-visualizer

The manifest format is a strict subset of octospark-visualizer's `system.json`. Any valid `system.json` is a valid domain-scan manifest with no conversion required.

### 10a.2 Minimal Manifest Schema

```json
{
  "subsystems": [
    {
      "id": "auth",
      "name": "Auth & Identity",
      "domain": "platform-core",
      "status": "built",
      "filePath": "/absolute/path/to/src/auth/",
      "interfaces": [],
      "operations": [],
      "tables": [],
      "events": [],
      "children": [
        {
          "id": "auth-jwt",
          "name": "JWT Provider",
          "domain": "platform-core",
          "status": "built",
          "filePath": "/absolute/path/to/src/auth/jwt/",
          "interfaces": ["AuthPrincipal", "JWTClaims"],
          "operations": ["signToken()", "verifyToken()"],
          "tables": ["users"],
          "events": ["session.created"],
          "children": [],
          "dependencies": []
        }
      ],
      "dependencies": ["billing"]
    }
  ]
}
```

Optional fields from `system.json` that domain-scan ignores: `meta`, `editor`, `domains`, `connections`, `description`, `designDocPath`.

### 10a.3 Rust Types (manifest.rs)

```rust
pub struct Manifest {
    pub subsystems: Vec<ManifestSubsystem>,
}

#[derive(Deserialize)]
pub struct ManifestSubsystem {
    pub id: String,
    pub name: String,
    pub domain: String,
    #[serde(deserialize_with = "deserialize_status")]
    pub status: ManifestStatus,
    #[serde(rename = "filePath")]
    pub file_path: PathBuf,
    #[serde(default)]
    pub interfaces: Vec<String>,        // PascalCase
    #[serde(default)]
    pub operations: Vec<String>,        // camelCase with "()" suffix
    #[serde(default)]
    pub tables: Vec<String>,            // snake_case
    #[serde(default)]
    pub events: Vec<String>,            // dot.notation
    #[serde(default)]
    pub children: Vec<ManifestSubsystem>,
    #[serde(default)]
    pub dependencies: Vec<String>,      // IDs of other subsystems
}

pub enum ManifestStatus {
    Built,          // -> BuildStatus::Built
    Rebuild,        // -> BuildStatus::Rebuild
    New,            // -> BuildStatus::Unbuilt
    Boilerplate,    // -> BuildStatus::Unbuilt
}
```

### 10a.4 Hierarchical Matching Algorithm

Match against the most specific (deepest) subsystem whose `filePath` covers the entity's file:

1. Flatten manifest tree depth-first into `Vec<(subsystem, depth)>`.
2. For each extracted entity, collect all subsystems whose `filePath` is a prefix of the entity's file path.
3. Select the deepest match. A child at depth 2 wins over its parent at depth 1.
4. If no filePath match, fall back to name matching against `interfaces[]`, `operations[]`, `tables[]`, `events[]`.
5. If still unmatched, place in the unmatched bucket.

Parent subsystems intentionally have empty entity arrays. Only children carry interfaces/operations/tables/events.

### 10a.5 Naming Convention Validation Rules

| Category | Convention | Regex | Example |
|----------|-----------|-------|---------|
| `interfaces[]` | PascalCase | `^[A-Z][A-Za-z0-9]*$` | `AuthPrincipal` |
| `operations[]` | camelCase with `()` | `^[a-z][A-Za-z0-9]*\(\)$` | `signToken()` |
| `tables[]` | snake_case | `^[a-z][a-z0-9_]*$` | `auth_login_sessions` |
| `events[]` | dot.notation (at least one dot) | `^[a-z][a-z0-9]*(\.[a-z][a-z0-9]*)+$` | `session.created` |

### 10a.6 Write-Back

`domain-scan match --manifest system.json --write-back` updates the manifest with newly discovered entities. Rules:
- Additive only. Never removes entries.
- Human-authored fields (`name`, `description`, `domain`, `designDocPath`, `children`, connections) are never touched.
- Duplicates are never introduced.
- `status` is upgraded from `new`/`boilerplate` to `built` if `BuildStatus::Built` is detected.
- Does not create new subsystem entries or modify `children[]` hierarchy.

### 10a.7 Exit Codes for CI

- `domain-scan validate`: exit 0 on all PASS/WARN, exit 1 on any FAIL. With `--strict`: exit 1 on any WARN or FAIL.
- `domain-scan match`: exit 0 always (unmatched items are informational). With `--fail-on-unmatched`: exit 1 if any unmatched items remain.

---

## 11. Build Phases

### Phase 1: Foundation
- [x] Workspace setup (Cargo.toml with `[profile.test] opt-level = 1`, crate layout for core/cli/mcp)
- [x] `walker.rs`: filesystem traversal with `ignore` crate (.gitignore-aware)
- [x] `lang.rs`: language detection by extension (TypeScript also covers JS)
- [x] `parser.rs`: tree-sitter parsing with thread-local pool (`thread_local!`)
- [x] `ir.rs`: all IR types defined (BuildStatus, Confidence, IrFile, all Def types, Entity, EntitySummary, FilterParams, ScanConfig)
- [x] `build_status.rs`: build status detection heuristics (artifact timestamps, git status)
- [x] `output.rs`: JSON + table + compact serialization module (skeleton)
- [x] `types.rs`: public type re-exports
- [x] Basic `lib.rs` with clippy deny wall
- [x] Unit tests for walker, lang detection, IR construction
- [x] Build status tests: Built (fresh artifacts), Unbuilt (no artifacts), Error (stale artifacts), Rebuild (many uncommitted changes)
- [x] Build status tests for multiple languages: Rust (target/), TypeScript (node_modules/.cache), Python (__pycache__)

**Acceptance criteria:**
- `cargo test -p domain-scan-core` passes
- Can walk a directory, respect .gitignore, and identify all 12 languages
- Can parse a single TypeScript file into a tree-sitter tree
- Build status detection correctly identifies all 4 states (Built/Unbuilt/Error/Rebuild)
- `IrFile.confidence` is correctly derived from `BuildStatus`

### Phase 2: Query Engine + First Language (TypeScript)
- [x] `query_engine.rs`: load .scm files via `include_str!`, lazy compile per language, dispatch captures to IR
- [x] `queries/typescript/interfaces.scm`
- [x] `queries/typescript/classes.scm`
- [x] `queries/typescript/methods.scm`
- [x] `queries/typescript/functions.scm`
- [x] `queries/typescript/types.scm`
- [x] `queries/typescript/imports.scm`
- [x] `queries/typescript/exports.scm`
- [x] `queries/typescript/services.scm` (Express, NestJS, tRPC)
- [x] `queries/typescript/schemas.scm` (Effect.ts Schema.Struct, Zod z.object, Drizzle pgTable)
- [x] `queries/typescript/effect.scm` (Effect.ts deep extraction — see Section 4.7):
  - [x] Effect services: `Context.Tag`, `Effect.Service`, `Context.GenericTag`
  - [x] Effect layers: `Layer.effect`, `Layer.sync`, `Layer.succeed`, `Layer.scoped`, `Layer.fresh`, `Layer.merge`, pipe-based composition
  - [x] Effect schemas: `Schema.Class`, `Schema.TaggedStruct`, `Schema.TaggedClass`, `Schema.Union`, `Schema.partial`/`required`/`pick`/`omit`
  - [x] Effect errors: `Data.TaggedError`, `Schema.TaggedError` (with and without payloads)
  - [x] Effect pipelines: `pipe`, `flow`, `Effect.gen`, method-chain `.pipe`
- [x] Effect sub-parsing: `@service.shape` → method signatures, `@layer.body` → `yield*` dependency extraction, `@pipeline.steps` → combinator list
- [x] Schema field sub-parsing logic (`fields_source` raw text -> `Vec<SchemaField>`)
- [x] 13+ fixture files in `tests/fixtures/typescript/` with expected JSON (including 6 Effect-specific fixtures + 1 full-app integration fixture)
- [x] Integration tests: each .scm file has at least one test parsing a real fixture
- [x] Property-based tests: IR roundtrip serialization (NOT source code generation)

**Acceptance criteria:**
- Parse real TypeScript files and extract all interfaces, classes, methods, schemas
- Every `.scm` file has at least one integration test against a real fixture
- `proptest` tests verify `IrFile` serde roundtrip, not tree-sitter parsing
- Schema extraction works for Zod, Effect Schema, and Drizzle patterns
- Effect.ts: `Context.Tag` services extracted with method signatures from shape type
- Effect.ts: `Layer.effect`/`Layer.merge`/pipe composition produces correct provides/requires/composition fields
- Effect.ts: `Data.TaggedError` and `Schema.TaggedError` extracted with tag + fields
- Effect.ts: `pipe`/`flow`/`Effect.gen` pipelines extracted with combinator steps
- Effect.ts: full-app fixture (`effect_full_app.ts`) produces correct counts: 3 errors, 4 schemas, 2 services, 3 layers, 2 pipelines

### Phase 3: Rust + Go + Python Queries
- [x] `queries/rust/traits.scm`, `impls.scm`, `methods.scm`, `functions.scm`, `types.scm`, `imports.scm`, `services.scm`, `schemas.scm` (serde derive structs)
- [x] `queries/go/interfaces.scm` (uses `method_elem` not `method_spec`), `structs.scm`, `methods.scm`, `functions.scm`, `imports.scm`, `services.scm`, `schemas.scm` (tagged structs)
- [x] `queries/python/classes.scm`, `methods.scm`, `functions.scm`, `protocols.scm`, `abstract.scm`, `imports.scm`, `decorators.scm`, `services.scm`, `schemas.scm` (Pydantic, dataclass, TypedDict, SQLAlchemy)
- [x] 5-7 fixtures per language with expected JSON
- [x] Integration tests for all three languages
- [x] Property-based tests: IR roundtrip, ScanIndex invariants

**Acceptance criteria:**
- Parse real Rust/Go/Python files and extract correct structural census
- Cross-language IR types are consistent (a Rust trait and a TS interface both produce InterfaceDef with same field structure)
- Go interfaces.scm uses `method_elem` (not `method_spec`) and captures methods correctly
- Rust traits.scm uses `(_)` wildcards for return_type/trait/type fields

### Phase 4a: JVM Languages (Java, Kotlin, Scala)
- [x] Java queries: interfaces, classes, methods, annotations, services, imports, `schemas.scm` (@Entity, records)
- [x] Kotlin queries: interfaces, classes, methods, services, imports, `schemas.scm` (data class, uses `(identifier)` not `(type_identifier)` for names)
- [x] Scala queries: traits, classes, methods, objects, imports
- [x] 4-5 fixtures per language with expected JSON

**Acceptance criteria:**
- Java @Entity and records detected as schemas
- Kotlin data classes detected as schemas with `(identifier)` capture
- Each language has at least 4 fixture files

### Phase 4b: Systems/Scripting Languages (C#, Swift, C++, PHP, Ruby)
- [x] C# queries: interfaces, classes, methods, services (ASP.NET), imports
- [x] Swift queries: protocols, classes, methods, extensions, imports
- [x] C++ queries: classes, methods, functions, templates, virtual, imports
- [x] PHP queries: interfaces, classes, methods, traits, imports
- [x] Ruby queries: modules, classes, methods, imports
- [x] 3+ fixtures per language with expected JSON

**Acceptance criteria:**
- All 12 languages parse correctly (combined with Phase 4a)
- Each language has at least 3 fixture files

### Phase 5: Cross-File Resolution + Index + Config + Cache
- [x] `config.rs`: `.domain-scan.toml` parsing, custom service definitions, include/exclude globs
- [ ] `cache.rs`: content-addressed cache with DashMap, disk persistence with bincode, LRU eviction
- [ ] `resolver.rs`: import/export tracking, implementation matching
- [ ] `index.rs`: ScanIndex construction with all lookup tables (interfaces, classes, services, methods, impls, schemas)
- [ ] `manifest.rs`: Manifest struct, deserialization, flatten, validate, match algorithm, write-back
- [ ] `validate.rs`: 10 validation rules (naming, completeness, god-objects, impls)
- [ ] Cross-file tests: interface in file A, implementation in file B
- [ ] Implementation completeness checking
- [ ] Full pipeline integration test: walk -> parse -> index -> query -> output
- [ ] `--no-cache` and `-o, --out <FILE>` plumbing

**Acceptance criteria:**
- `domain-scan impls EventHandler` correctly finds all implementors across files
- Import chains resolve (A imports B which re-exports from C)
- `.domain-scan.toml` is read and respected
- Cache correctly invalidates when file content changes
- `domain-scan validate` detects PascalCase violations, god-interfaces, missing impls
- `domain-scan match --manifest test.json` maps entities to subsystems with >0 matched

### Phase 6a: CLI Core
- [ ] `domain-scan-cli/src/main.rs`: clap subcommands with global flags (`--root`, `--config`, `--output`, `--languages`, `--build-status`, `--no-cache`, `-o`, `-v`, `-q`)
- [ ] `scan` subcommand (with `--build-status` override)
- [ ] `interfaces` subcommand with filters (`--name`, `--languages`, `--build-status`)
- [ ] `services` subcommand with filters (`--kind`, `--name`, `--show-routes`, `--show-deps`)
- [ ] `methods` subcommand with filters (`--owner`, `--async`, `--visibility`, `--name`)
- [ ] `schemas` subcommand with filters (`--framework`, `--kind`, `--name`, `--show-fields`)
- [ ] `impls` subcommand (`--all`, `--show-methods`)
- [ ] `search` subcommand (`--kind`, `--regex`)
- [ ] `stats` subcommand
- [ ] `validate` subcommand (`--rules`, `--manifest`, `--strict`)
- [ ] `match` subcommand (`--manifest`, `--unmatched-only`, `--prompt-unmatched --agents N`, `--write-back`, `--dry-run`, `--fail-on-unmatched`)
- [ ] `cache` subcommand (stats, clear, prune)
- [ ] `prompt` subcommand (`--agents`, `--focus`, `--include-scan`)
- [ ] Table, JSON, compact output formatters
- [ ] Golden-file snapshot tests (insta) for all output formats
- [ ] CLI integration tests with assert_cmd

**Acceptance criteria:**
- All subcommands work with `--output json`, `--output table`, `--output compact`
- `domain-scan scan --output json | jq .` produces valid JSON with `build_status` fields
- `domain-scan validate` exits 1 on FAIL, 0 on PASS/WARN; exits 1 on WARN with `--strict`
- `domain-scan match --manifest test.json --fail-on-unmatched` exits 1 if unmatched items remain
- CLI arg parsing tested (clap's test macros)
- Snapshot tests pass for table/json/compact output of `interfaces`, `services`, `schemas`

### Phase 6b: TUI Interactive Mode
- [ ] `--interactive` flag (mutually exclusive with `--output`)
- [ ] ratatui TUI with `crossterm` backend
- [ ] TuiApp struct with `handle_event` and `render` methods (testable without terminal)
- [ ] Tree state: `Vec<(entity, expanded: bool)>`, Enter toggles expanded
- [ ] Arrow keys navigate, right expands, left collapses, `/` focuses search, `q` quits
- [ ] TUI tests using `ratatui::backend::TestBackend` (single Enter expands, second Enter collapses)

**Acceptance criteria:**
- `domain-scan interfaces --interactive` launches a TUI
- Single Enter keypress expands a parent node to show children
- Tests verify expand/collapse via `TestBackend` (no real terminal needed)
- `--interactive` and `--output` are mutually exclusive (clap conflict)

### Phase 7: LLM Prompt Generation
- [ ] `prompt` subcommand (already scaffolded in Phase 6a)
- [ ] Partitioning strategy: auto-select by file count (< 500: by concern, 500-2000: hybrid, > 2000: by directory)
- [ ] Build-status-aware partitioning: different agent instructions for Built vs Unbuilt/Rebuild files
- [ ] Prompt template with embedded scan results
- [ ] Agent assignment generation with file lists from index
- [ ] `--focus` flag: filter scan index by entity name regex, scope prompt to matching files only
- [ ] `--include-scan` flag for full scan embedding
- [ ] Snapshot tests (insta) for prompt output

**Acceptance criteria:**
- `domain-scan prompt --agents 5` produces a valid prompt with 5 agent sections
- Prompt adapts partitioning to codebase size (tested with small/medium fixture dirs)
- `--focus "auth"` scopes prompt to only auth-related files
- Built files get "verify and catalog" instructions, Unbuilt files get "analyze and infer" instructions

### Phase 8: Tauri Desktop App - Backend
- [ ] `domain-scan-tauri` crate setup (Tauri 2) with `tauri-plugin-shell` and `tauri-plugin-dialog`
- [ ] `AppState` struct with `Mutex<Option<ScanIndex>>` and `Mutex<Option<PathBuf>>`
- [ ] `CommandError` enum with thiserror + serde::Serialize
- [ ] All IPC commands from Section 8.8 (scan_directory, get_current_scan, filter_entities, get_entity_detail, get_entity_source, search_entities, generate_prompt, export_entities, get_build_status, open_in_editor, check_editors_available)
- [ ] `tauri.conf.json` with CSP, window config (1440x900, min 1200x700)
- [ ] React scaffold with Vite + TypeScript + Tailwind

**Acceptance criteria:**
- `cargo tauri dev` launches the app window
- `scan_directory` IPC command populates AppState and returns ScanStats
- `filter_entities` reads from AppState (does not accept ScanIndex as parameter)
- `open_in_editor` works for VS Code, Cursor, Zed

### Phase 9: Tauri Desktop App - Frontend
- [ ] Three-panel layout: Entity Tree | Source Preview | Details Panel
- [ ] EntityTree component with single-click expand/collapse
- [ ] Source preview with syntax highlighting (scrolls to entity span)
- [ ] Details panel with build status, confidence, metadata, warning banner for non-Built
- [ ] Filter bar: by kind, build status, language
- [ ] Fuzzy search with real-time tree update
- [ ] Build status color indicators (green/yellow/red/orange dots)
- [ ] Keyboard navigation: j/k, arrow keys, Enter expand/collapse, / search, p prompt, e export, q quit
- [ ] "Generate Prompt" button scoped to selected entities
- [ ] Export: JSON, CSV, Markdown
- [ ] Scan-on-open with progress bar in status bar
- [ ] `useScan.ts` and `useTreeState.ts` hooks
- [ ] `useKeyboard.ts` hook with input-focus guards

**Acceptance criteria:**
- Single click on a parent node expands to show children. No double-click needed.
- Build status visually indicated per entity in the tree
- `Rebuild` entities show warning banner recommending LLM enrichment
- All keyboard shortcuts work (j/k/Enter///p/e/q/Escape)
- `cargo tauri build` produces a working app

### Phase 10: Polish + Performance
- [ ] Benchmark: parse throughput (target: >500 files/sec on 8 cores)
- [ ] Benchmark: cached re-scan (target: >5000 files/sec)
- [ ] Benchmark: CLI startup (target: <100ms for <50 files)
- [ ] `--verbose` output with timing details
- [ ] Error messages for common mistakes (wrong path, no files found, bad config)
- [ ] `domain-scan validate --self-test` (validates domain-scan's own codebase)
- [ ] README.md with usage examples
- [ ] rayon parallelism tuning
- [ ] No deadlocks under parallel load

**Acceptance criteria:**
- Performance targets from Section 15 are met
- `domain-scan validate --self-test` exits 0 on domain-scan's own codebase
- README covers all subcommands with examples

---

## 12. Testing Strategy

### 12.1 Test Hierarchy

1. **Unit tests** (co-located `#[cfg(test)]`): IR construction, language detection, cache logic, config parsing
2. **Integration tests** (`tests/integration/`): Real tree-sitter parsing against fixture files
3. **Property-based tests** (proptest): Query capture → IR mapping invariants, roundtrip serialization
4. **Shared test fixtures** (`tests/fixtures/`): Real code snippets with expected structural output
5. **CLI integration tests**: Spawn CLI binary, check JSON output against expected schema

### 12.2 Fixture Design

Each language fixture directory contains:
```
tests/fixtures/typescript/
├── basic_interface.ts          # Simple interface with methods
├── generic_interface.ts        # Generics + extends
├── class_implements.ts         # Class implementing interface
├── service_express.ts          # Express router (service detection)
├── service_nestjs.ts           # NestJS controller
├── complex_methods.ts          # Async, generators, overloads
├── imports_exports.ts          # Re-exports, barrel files
├── expected/
│   ├── basic_interface.json    # Expected IR output
│   ├── generic_interface.json
│   └── ...
```

### 12.3 Property-Based Test Examples

```rust
// Every parsed interface must have a name
proptest! {
    #[test]
    fn interface_always_has_name(code in arbitrary_ts_interface()) {
        let ir = parse_typescript(&code)?;
        for iface in &ir.interfaces {
            prop_assert!(!iface.name.is_empty());
        }
    }
}

// Parse → JSON → Parse roundtrip preserves all fields
proptest! {
    #[test]
    fn ir_roundtrip(ir_file in arbitrary_ir_file()) {
        let json = serde_json::to_string(&ir_file)?;
        let roundtrip: IrFile = serde_json::from_str(&json)?;
        prop_assert_eq!(ir_file, roundtrip);
    }
}

// Every method in an impl must reference a real trait method (when trait is known)
proptest! {
    #[test]
    fn impl_methods_match_trait(
        trait_def in arbitrary_interface_def(),
        impl_def in arbitrary_impl_for(&trait_def)
    ) {
        for method in &impl_def.methods {
            prop_assert!(
                trait_def.methods.iter().any(|m| m.name == method.name),
                "Impl method {} not found in trait", method.name
            );
        }
    }
}
```

### 12.4 Real Tree-Sitter Integration Tests

```rust
#[test]
fn test_typescript_interface_parsing() {
    let source = include_str!("fixtures/typescript/basic_interface.ts");
    let ir = parse_file(source, Language::TypeScript).unwrap();

    assert_eq!(ir.interfaces.len(), 1);
    let iface = &ir.interfaces[0];
    assert_eq!(iface.name, "UserRepository");
    assert_eq!(iface.methods.len(), 4);
    assert_eq!(iface.methods[0].name, "findById");
    assert!(iface.methods[0].is_async);
    assert_eq!(iface.methods[0].parameters.len(), 1);
    assert_eq!(iface.methods[0].parameters[0].name, "id");
    assert_eq!(iface.methods[0].parameters[0].type_annotation.as_deref(), Some("string"));
    assert_eq!(iface.methods[0].return_type.as_deref(), Some("Promise<User | null>"));
}

#[test]
fn test_rust_trait_parsing() {
    let source = include_str!("fixtures/rust/event_handler.rs");
    let ir = parse_file(source, Language::Rust).unwrap();

    assert_eq!(ir.interfaces.len(), 1);
    let trait_def = &ir.interfaces[0];
    assert_eq!(trait_def.name, "EventHandler");
    assert_eq!(trait_def.language_kind, InterfaceKind::Trait);
    assert_eq!(trait_def.methods.len(), 3);
}
```

---

## 13. Output Schemas

### 13.1 JSON Output (domain-scan scan --output json)

```json
{
  "version": "0.1.0",
  "root": "/path/to/project",
  "scanned_at": "2026-03-23T14:00:00Z",
  "stats": {
    "total_files": 342,
    "files_by_language": { "TypeScript": 180, "Rust": 95, "Python": 67 },
    "total_interfaces": 48,
    "total_services": 12,
    "total_classes": 156,
    "total_methods": 1847,
    "total_functions": 423,
    "total_type_aliases": 89,
    "total_implementations": 67,
    "parse_duration_ms": 1200,
    "cache_hits": 284,
    "cache_misses": 58
  },
  "files": [
    {
      "path": "src/repos/user.ts",
      "language": "TypeScript",
      "content_hash": "a1b2c3...",
      "build_status": "built",
      "confidence": "high",
      "interfaces": [
        {
          "name": "UserRepository",
          "span": { "start_line": 12, "start_col": 0, "end_line": 28, "end_col": 1 },
          "visibility": "public",
          "generics": [],
          "extends": ["BaseRepository"],
          "methods": [
            {
              "name": "findById",
              "is_async": true,
              "parameters": [
                { "name": "id", "type_annotation": "string", "is_optional": false }
              ],
              "return_type": "Promise<User | null>",
              "has_default": false
            }
          ],
          "properties": [],
          "language_kind": "Interface",
          "decorators": []
        }
      ],
      "services": [],
      "classes": [],
      "functions": [],
      "type_aliases": [],
      "imports": [],
      "exports": [],
      "implementations": []
    }
  ]
}
```

---

## 14. Dependencies

```toml
[workspace.dependencies]
# Tree-sitter — pin to 0.24.7 to stay ABI-compatible with all grammar crates.
# Grammar crates requiring ^0.25 are pinned to their last ^0.24-compatible release.
# WARNING: Mixing tree-sitter 0.24 and 0.25 grammars causes silent ABI split.
tree-sitter = "0.24.7"
tree-sitter-typescript = "0.23"         # Also covers JavaScript; requires ^0.24
tree-sitter-python = "0.23"             # Pin to last ^0.24-compatible release
tree-sitter-rust = "0.23"              # Pin to last ^0.24-compatible release
tree-sitter-go = "0.23"               # Pin to last ^0.24-compatible release
tree-sitter-java = "0.23"             # Latest 0.23.5, requires ^0.24
tree-sitter-kotlin-ng = "1.1"         # NOT tree-sitter-kotlin (0.1 is from 2021, requires ^0.19)
tree-sitter-c-sharp = "0.23"          # Latest 0.23.1, requires ^0.24
tree-sitter-swift = "0.7"             # Latest 0.7.1, requires ^0.23
tree-sitter-php = "0.23"              # Pin to last ^0.24-compatible release
tree-sitter-ruby = "0.23"             # Latest 0.23.1, requires ^0.24
tree-sitter-scala = "0.23"            # Pin to last ^0.24-compatible release
tree-sitter-cpp = "0.23"              # Latest 0.23.4, requires ^0.24
tree-sitter-c = "0.23"                # Pin to last ^0.24-compatible release

# Core
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rayon = "1.10"
dashmap = "6"
sha2 = "0.10"
bincode = "1"                          # v1 API (serde-based); NOT compatible with v2/v3
thiserror = "2"
ignore = "0.4"                         # .gitignore-aware walking (from ripgrep project)
toml = "0.8"
regex = "1"
log = "0.4"
env_logger = "0.11"
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }

# CLI
clap = { version = "4", features = ["derive"] }
comfy-table = "7"                      # Table output
indicatif = "0.18"                     # Progress bars (0.17 is outdated)
ratatui = "0.30"                       # TUI mode (0.29 does not exist; latest is 0.30)
crossterm = "0.29"                     # Terminal backend for ratatui (ratatui 0.30 requires 0.29)

# Tauri — tauri-build goes in [build-dependencies] at the crate level, not [dependencies]
tauri = "2"
tauri-build = "2"
tauri-plugin-shell = "2"               # For open-in-editor
tauri-plugin-dialog = "2"              # For folder picker

# Testing — these must be [dev-dependencies] in consuming crates, not [dependencies]
proptest = "1"
insta = "1"                            # Snapshot testing for JSON/prompt output
rstest = "0.18"                        # Parameterized tests (Rust equivalent of it.each)
assert_cmd = "2"                       # CLI subprocess testing
tempfile = "3"                         # TempDir for build status and pipeline tests

[profile.test]
opt-level = 1                          # Critical for tree-sitter test performance (5-10x faster)
```

---

## 15. Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Parse throughput | > 500 files/sec | With rayon on 8-core machine |
| Cached re-scan | > 5000 files/sec | Content-addressed cache hits |
| Memory per file | < 50 KB IR | Average across languages |
| CLI startup | < 100 ms | For small projects (< 50 files) |
| Full scan 1K files | < 3 seconds | First scan, no cache |
| Full scan 1K files | < 500 ms | Cached re-scan |

---

## 16. Future Extensions (Not In Scope)

- VS Code extension (tree view of interfaces/services)
- Watch mode (re-scan on file change)
- GraphQL schema extraction
- Protobuf/gRPC .proto file parsing
- OpenAPI spec generation from scanned services
- Dependency injection graph visualization

---

## 17. Input Hardening & Adversarial Testing

The CLI is an agent-first tool. AI agents are not trusted operators — they hallucinate paths, embed query params in IDs, generate control characters, and pre-encode strings that get double-encoded. Every input boundary must be validated.

### 17.1 Input Validation Rules

All input validation lives in `domain-scan-core` (not the CLI crate) so it protects every surface (CLI, Tauri IPC, future integrations).

**Path inputs** (`--root`, `--config`, `--manifest`, `--out`):
- Canonicalize via `std::fs::canonicalize` and reject if the resolved path escapes the expected base directory
- Reject paths containing `..` segments before canonicalization (defense-in-depth)
- Reject null bytes (`\0`)
- Reject paths longer than 4096 bytes

**String inputs** (names, patterns, filter values):
- Reject control characters below ASCII 0x20 (except `\n` and `\t` in multi-line JSON inputs)
- Reject strings containing null bytes
- Reject resource identifiers containing `?`, `#`, or `%` (signs of embedded query params or pre-encoding)

**JSON inputs** (`--json`):
- Parse with `serde_json` — reject on any parse error with a structured error including the byte offset
- Enforce max depth of 32 levels (prevent stack overflow from deeply nested input)
- Enforce max input size of 1 MB

**Regex inputs** (`--name`, `--pattern`, filter patterns):
- Compile with `regex::Regex::new` and reject on error
- Enforce a max pattern length of 1024 chars (prevent ReDoS)
- Set a match timeout (10 ms per file) to prevent catastrophic backtracking

### 17.2 Structured Error Format

All validation failures return structured JSON errors:

```json
{
  "error": {
    "code": "INVALID_PATH",
    "message": "Path contains traversal segment: ../../etc/passwd",
    "suggestion": "Use an absolute path or a path relative to the project root without '..' segments",
    "input": "../../etc/passwd",
    "field": "--root"
  }
}
```

Error codes:
| Code | Trigger |
|------|---------|
| `INVALID_PATH` | Path traversal, null bytes, too long |
| `INVALID_INPUT` | Control characters, null bytes in strings |
| `INVALID_RESOURCE_ID` | Embedded `?`, `#`, `%` in identifiers |
| `INVALID_JSON` | Malformed JSON, too deep, too large |
| `INVALID_REGEX` | Regex parse failure or pattern too long |
| `NO_SCAN` | Command requires a prior scan but none exists |
| `FILE_NOT_FOUND` | Specified file/directory does not exist |
| `PERMISSION_DENIED` | Cannot read the target path |

### 17.3 Adversarial Test Fixtures

Adversarial tests live in `crates/domain-scan-core/tests/adversarial/`. Each test exercises a specific hallucination pattern that agents produce in practice.

```
tests/adversarial/
├── path_traversal.rs          # ../../.ssh, /etc/passwd, symlink escapes
├── control_chars.rs           # Null bytes, bell, backspace, escape sequences in names
├── embedded_query_params.rs   # fileId?fields=name, path#fragment, pre-encoded %2e%2e
├── json_bombs.rs              # Deeply nested JSON, 100MB payloads, duplicate keys
├── regex_dos.rs               # Catastrophic backtracking patterns, 10KB regexes
├── unicode_edge_cases.rs      # Homoglyphs, zero-width joiners, RTL overrides in identifiers
├── symlink_escapes.rs         # Symlinks pointing outside project root
└── concurrent_scans.rs        # Race conditions: scan while another scan replaces the index
```

**Path traversal tests:**
```rust
#[test]
fn rejects_dot_dot_traversal() {
    let err = validate_path("../../.ssh/id_rsa", &project_root).unwrap_err();
    assert_eq!(err.code(), "INVALID_PATH");
}

#[test]
fn rejects_null_byte_in_path() {
    let err = validate_path("src/main\0.rs", &project_root).unwrap_err();
    assert_eq!(err.code(), "INVALID_PATH");
}

#[test]
fn rejects_symlink_escape() {
    // Create a symlink inside project root pointing to /etc
    let dir = tempdir()?;
    std::os::unix::fs::symlink("/etc", dir.path().join("escape"))?;
    let err = validate_path("escape/passwd", dir.path()).unwrap_err();
    assert_eq!(err.code(), "INVALID_PATH");
}
```

**Embedded query param tests:**
```rust
#[test]
fn rejects_query_params_in_resource_id() {
    let err = validate_resource_id("abc123?fields=name").unwrap_err();
    assert_eq!(err.code(), "INVALID_RESOURCE_ID");
}

#[test]
fn rejects_pre_encoded_traversal() {
    let err = validate_resource_id("%2e%2e%2f%2e%2e%2fetc%2fpasswd").unwrap_err();
    assert_eq!(err.code(), "INVALID_RESOURCE_ID");
}

#[test]
fn rejects_fragment_in_resource_id() {
    let err = validate_resource_id("abc123#section").unwrap_err();
    assert_eq!(err.code(), "INVALID_RESOURCE_ID");
}
```

**JSON bomb tests:**
```rust
#[test]
fn rejects_deeply_nested_json() {
    let json = (0..100).fold(String::from("null"), |acc, _| format!("[{}]", acc));
    let err = parse_json_input(&json).unwrap_err();
    assert_eq!(err.code(), "INVALID_JSON");
}

#[test]
fn rejects_oversized_json() {
    let json = format!(r#"{{"data": "{}"}}"#, "x".repeat(2_000_000));
    let err = parse_json_input(&json).unwrap_err();
    assert_eq!(err.code(), "INVALID_JSON");
}
```

**Regex DoS tests:**
```rust
#[test]
fn rejects_catastrophic_backtracking_pattern() {
    let err = validate_regex("(a+)+$").unwrap_err();
    assert_eq!(err.code(), "INVALID_REGEX");
}

#[test]
fn rejects_oversized_regex() {
    let pattern = "a".repeat(2000);
    let err = validate_regex(&pattern).unwrap_err();
    assert_eq!(err.code(), "INVALID_REGEX");
}
```

**Unicode edge case tests:**
```rust
#[test]
fn rejects_null_byte_in_name_filter() {
    let err = validate_string_input("User\0Repository").unwrap_err();
    assert_eq!(err.code(), "INVALID_INPUT");
}

#[test]
fn rejects_control_chars_in_name_filter() {
    let err = validate_string_input("User\x07Repository").unwrap_err(); // bell character
    assert_eq!(err.code(), "INVALID_INPUT");
}

#[test]
fn allows_valid_unicode_identifiers() {
    // CJK, accented, emoji in identifiers are valid (some languages allow them)
    validate_string_input("ユーザーRepository").unwrap();
    validate_string_input("café_service").unwrap();
}
```

### 17.4 Real Codebase Test Suite

domain-scan is tested against real open-source codebases to catch edge cases synthetic fixtures can't anticipate. These repos are cloned at test time (cached in CI) and scanned end-to-end.

| Repo | Language | Why this repo |
|------|----------|---------------|
| `tokio-rs/tokio` | Rust | Deep trait hierarchies, async traits, proc macros, huge crate graph |
| `denoland/deno` | Rust + TypeScript | Polyglot. Rust core with TS API surface. Tests cross-language scanning. |
| `nestjs/nest` | TypeScript | Decorator-heavy. Controller/service/module pattern. Tests service detection. |
| `pallets/flask` | Python | Class-based views, blueprints, extension interfaces. Tests Python extraction. |
| `spring-projects/spring-boot` | Java | Annotation-heavy. Interface + impl pattern at scale. |

```rust
/// Integration tests against real repos (run with `cargo test -- --ignored real_codebase`)
/// These are #[ignore]'d by default — CI runs them on a schedule, not on every push.

#[test]
#[ignore]
fn real_codebase_tokio() {
    let dir = clone_or_cache("https://github.com/tokio-rs/tokio", "tokio");
    let index = scan_directory(&dir, &ScanConfig::default()).unwrap();

    // Smoke: tokio has hundreds of traits and impls
    assert!(index.stats.interfaces > 50, "Expected 50+ traits, got {}", index.stats.interfaces);
    assert!(index.stats.impls > 100, "Expected 100+ impls, got {}", index.stats.impls);

    // No panics, no empty names, no invalid spans
    for file in &index.files {
        for iface in &file.interfaces {
            assert!(!iface.name.is_empty(), "Empty interface name in {:?}", file.path);
            assert!(iface.span.start < iface.span.end, "Invalid span for {}", iface.name);
        }
    }
}

#[test]
#[ignore]
fn real_codebase_deno_polyglot() {
    let dir = clone_or_cache("https://github.com/denoland/deno", "deno");
    let index = scan_directory(&dir, &ScanConfig::default()).unwrap();

    // Must detect both Rust and TypeScript files
    let languages: HashSet<_> = index.files.iter().map(|f| f.language).collect();
    assert!(languages.contains(&Language::Rust), "Missing Rust files");
    assert!(languages.contains(&Language::TypeScript), "Missing TypeScript files");
}
```

### 17.5 Synthetic Adversarial Codebases

5-10 fake codebases in `tests/fixtures/adversarial_repos/` that stress-test specific parsing edge cases real repos may not cover. Each is a minimal self-contained project.

```
tests/fixtures/adversarial_repos/
├── 01_deeply_nested/           # 20+ levels of nested modules/classes/namespaces
│   └── src/
│       └── a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/deep.ts
├── 02_name_collisions/         # Same interface name in 50 different files
│   └── src/
│       ├── auth/UserService.ts
│       ├── billing/UserService.ts
│       └── ... (50 modules, all exporting UserService)
├── 03_empty_files/             # Valid source files with zero content
│   └── src/
│       ├── empty.ts
│       ├── empty.rs
│       ├── empty.py
│       └── empty.go
├── 04_syntax_errors/           # Files with deliberate parse errors (build_status: Error)
│   └── src/
│       ├── missing_brace.ts    # Unclosed interface
│       ├── bad_generic.rs      # Invalid lifetime syntax
│       └── indent_error.py     # Mixed tabs/spaces
├── 05_massive_file/            # Single 50K-line generated file with 500 interfaces
│   └── src/
│       └── generated.ts
├── 06_circular_symlinks/       # Symlinks creating a cycle: a -> b -> c -> a
│   └── src/
│       ├── a -> b
│       ├── b -> c
│       └── c -> a
├── 07_binary_files/            # .ts extension but binary content (images, compiled output)
│   └── src/
│       └── not_really_code.ts  # Contains PNG header bytes
├── 08_unicode_identifiers/     # Interfaces/methods with CJK, emoji, RTL, homoglyphs
│   └── src/
│       ├── cjk_interface.ts    # interface ユーザー { 取得(): Promise<void> }
│       ├── rtl_names.ts        # Right-to-left override characters in names
│       └── homoglyphs.rs       # trait Usеr (Cyrillic е) vs trait User (Latin e)
├── 09_huge_method_count/       # Single interface with 1000 methods
│   └── src/
│       └── god_interface.ts
└── 10_mixed_encodings/         # UTF-8, UTF-16 BOM, Latin-1 in same project
    └── src/
        ├── utf8.ts
        ├── utf16_bom.ts        # UTF-16 with BOM header
        └── latin1.py           # ISO-8859-1 encoded
```

**Assertions per adversarial repo:**

| Repo | Must not | Must |
|------|----------|------|
| `01_deeply_nested` | Panic or stack overflow | Parse all 20 levels, entities have correct fully-qualified paths |
| `02_name_collisions` | Merge or deduplicate entities silently | Report all 50 distinct `UserService` with unique `file` paths |
| `03_empty_files` | Error or crash | Produce valid `IrFile` with zero entities per file |
| `04_syntax_errors` | Panic on parse error | Set `build_status: Error`, extract what tree-sitter can parse (partial recovery) |
| `05_massive_file` | Run OOM or exceed 10s | Parse within 5s, produce all 500 interfaces |
| `06_circular_symlinks` | Infinite loop | Detect cycle and skip with a warning log entry |
| `07_binary_files` | Crash on invalid UTF-8 | Detect non-text content and skip with a warning |
| `08_unicode_identifiers` | Corrupt or drop non-ASCII names | Preserve exact Unicode names in output |
| `09_huge_method_count` | Truncate methods silently | Include all 1000 methods in IR |
| `10_mixed_encodings` | Produce mojibake | Parse UTF-8 files, skip or warn on non-UTF-8 |

```rust
#[test]
fn adversarial_repo_circular_symlinks() {
    let dir = Path::new("tests/fixtures/adversarial_repos/06_circular_symlinks");
    let index = scan_directory(dir, &ScanConfig::default()).unwrap();
    // Must complete without hanging. File count should be 0 (all symlinks, no real files).
    assert_eq!(index.stats.total_files, 0);
}

#[test]
fn adversarial_repo_binary_files() {
    let dir = Path::new("tests/fixtures/adversarial_repos/07_binary_files");
    let index = scan_directory(dir, &ScanConfig::default()).unwrap();
    // Binary file should be skipped, not parsed
    assert_eq!(index.stats.total_files, 0);
    assert!(index.stats.skipped_files > 0);
}

#[test]
fn adversarial_repo_name_collisions() {
    let dir = Path::new("tests/fixtures/adversarial_repos/02_name_collisions");
    let index = scan_directory(dir, &ScanConfig::default()).unwrap();
    let user_services: Vec<_> = index.files.iter()
        .flat_map(|f| &f.services)
        .filter(|s| s.name == "UserService")
        .collect();
    assert_eq!(user_services.len(), 50, "Each UserService must be distinct");
    // All must have unique file paths
    let paths: HashSet<_> = user_services.iter().map(|s| &s.file).collect();
    assert_eq!(paths.len(), 50);
}

#[test]
fn adversarial_repo_massive_file() {
    let dir = Path::new("tests/fixtures/adversarial_repos/05_massive_file");
    let start = std::time::Instant::now();
    let index = scan_directory(dir, &ScanConfig::default()).unwrap();
    assert!(start.elapsed().as_secs() < 10, "50K-line file must parse in under 10s");
    assert_eq!(index.files[0].interfaces.len(), 500);
}
```

### 17.6 Build Phase

Add to **Phase 10** (Polish + Performance):
- [ ] `input_validation.rs`: `validate_path`, `validate_string_input`, `validate_resource_id`, `validate_regex`, `parse_json_input` — all return structured `DomainScanError`
- [ ] All CLI commands route inputs through validation before any processing
- [ ] Adversarial test suite: all 8 test files in `tests/adversarial/`
- [ ] Adversarial repo fixtures: all 10 synthetic repos generated and committed
- [ ] Real codebase tests: `#[ignore]`'d tests for 5 real repos, CI scheduled job
- [ ] `--dry-run` flag on all mutating commands (`match --apply`, `validate --fix`, `cache clear`)

**Acceptance criteria:**
- All adversarial tests pass
- All 10 synthetic repos scan without panic, hang, or OOM
- Real codebase tests pass on latest tagged versions of all 5 repos
- `domain-scan scan ../../.ssh` returns `INVALID_PATH` error, not a stack trace
- `domain-scan interfaces --name "User\0Repo"` returns `INVALID_INPUT` error
- `--dry-run` on mutating commands shows what would happen without side effects
