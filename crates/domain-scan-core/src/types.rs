// Public type re-exports for convenience.
// Users can `use domain_scan_core::types::*` to get all IR types.

pub use crate::config::{
    CacheSection, ConfigFile, CustomServiceDef, GlobFilter, OutputSection, ProjectSection,
    ScanSection, ServicesSection,
};
pub use crate::ir::{
    BuildStatus, ClassDef, Confidence, Entity, EntityKind, EntitySummary, ExportDef, ExportKind,
    FilterParams, FunctionDef, HttpMethod, ImplDef, ImportDef, ImportedSymbol, InterfaceDef,
    InterfaceKind, IrFile, Language, MatchResult, MatchStrategy, MatchedEntity, MethodDef,
    MethodSignature, Parameter, PropertyDef, RouteDef, ScanConfig, ScanIndex, ScanStats,
    SchemaDef, SchemaField, SchemaKind, ServiceDef, ServiceKind, Span, TypeAlias,
    UnmatchedEntity, ValidationResult, Violation, ViolationSeverity, Visibility,
};
pub use crate::prompt::PromptConfig;
pub use crate::DomainScanError;
