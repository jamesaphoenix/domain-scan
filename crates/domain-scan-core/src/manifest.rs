//! Manifest parsing, matching, validation, and write-back.
//!
//! A manifest describes the expected subsystem structure of a codebase.
//! Compatible with octospark-visualizer's `system.json` format.
//!
//! The matching algorithm maps extracted entities to subsystems by:
//! 1. File path prefix (deepest match wins)
//! 2. Name matching against interfaces/operations/tables/events
//! 3. Unmatched bucket for human review or LLM enrichment

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};

use crate::ir::{
    BuildStatus, EntityKind, EntitySummary, MatchResult, MatchStrategy, MatchedEntity,
    ScanIndex, UnmatchedEntity,
};
use crate::DomainScanError;

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// A manifest describing the expected subsystem structure.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Manifest {
    pub subsystems: Vec<ManifestSubsystem>,
}

/// Extended manifest with meta, domains, and connections (system.json format).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SystemManifest {
    pub meta: ManifestMeta,
    #[serde(default)]
    pub domains: HashMap<String, DomainDef>,
    pub subsystems: Vec<ManifestSubsystem>,
    #[serde(default)]
    pub connections: Vec<Connection>,
}

impl SystemManifest {
    /// Convert to the simpler `Manifest` (for matching).
    pub fn as_manifest(&self) -> Manifest {
        Manifest {
            subsystems: self.subsystems.clone(),
        }
    }
}

/// Metadata about the system.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ManifestMeta {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
}

/// A domain definition with label and color.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DomainDef {
    pub label: String,
    pub color: String,
}

/// A connection between two subsystems.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub label: String,
    #[serde(rename = "type")]
    pub connection_type: ConnectionType,
}

/// The type of connection between subsystems.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    DependsOn,
    Uses,
    Triggers,
}

/// A subsystem in the manifest (recursive via `children`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ManifestSubsystem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub domain: String,
    #[serde(deserialize_with = "deserialize_status")]
    pub status: ManifestStatus,
    #[serde(rename = "filePath")]
    pub file_path: PathBuf,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub tables: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub children: Vec<ManifestSubsystem>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Status values from the manifest, mapped to BuildStatus.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ManifestStatus {
    Built,
    Rebuild,
    New,
    Boilerplate,
}

impl ManifestStatus {
    pub fn to_build_status(self) -> BuildStatus {
        match self {
            Self::Built => BuildStatus::Built,
            Self::Rebuild => BuildStatus::Rebuild,
            Self::New | Self::Boilerplate => BuildStatus::Unbuilt,
        }
    }
}

fn deserialize_status<'de, D>(deserializer: D) -> Result<ManifestStatus, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "built" => Ok(ManifestStatus::Built),
        "rebuild" => Ok(ManifestStatus::Rebuild),
        "new" => Ok(ManifestStatus::New),
        "boilerplate" => Ok(ManifestStatus::Boilerplate),
        other => Err(serde::de::Error::custom(format!(
            "unknown manifest status: {other}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Flattened subsystem for matching
// ---------------------------------------------------------------------------

/// A flattened subsystem with its depth in the hierarchy.
#[derive(Debug, Clone)]
struct FlatSubsystem {
    id: String,
    name: String,
    file_path: PathBuf,
    interfaces: Vec<String>,
    operations: Vec<String>,
    tables: Vec<String>,
    events: Vec<String>,
    depth: usize,
}

/// Flatten the manifest tree depth-first.
fn flatten_manifest(manifest: &Manifest) -> Vec<FlatSubsystem> {
    let mut result = Vec::new();
    for subsystem in &manifest.subsystems {
        flatten_recursive(subsystem, 0, &mut result);
    }
    result
}

fn flatten_recursive(subsystem: &ManifestSubsystem, depth: usize, out: &mut Vec<FlatSubsystem>) {
    out.push(FlatSubsystem {
        id: subsystem.id.clone(),
        name: subsystem.name.clone(),
        file_path: subsystem.file_path.clone(),
        interfaces: subsystem.interfaces.clone(),
        operations: subsystem.operations.clone(),
        tables: subsystem.tables.clone(),
        events: subsystem.events.clone(),
        depth,
    });
    for child in &subsystem.children {
        flatten_recursive(child, depth + 1, out);
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a manifest from a JSON string.
pub fn parse_manifest(json: &str) -> Result<Manifest, DomainScanError> {
    serde_json::from_str(json).map_err(|e| DomainScanError::Config(format!("manifest parse error: {e}")))
}

/// Parse a manifest from a file path.
pub fn parse_manifest_file(path: &Path) -> Result<Manifest, DomainScanError> {
    let content = std::fs::read_to_string(path)?;
    parse_manifest(&content)
}

/// Parse a system manifest (extended format with meta, domains, connections).
pub fn parse_system_manifest(json: &str) -> Result<SystemManifest, DomainScanError> {
    serde_json::from_str(json)
        .map_err(|e| DomainScanError::Config(format!("system manifest parse error: {e}")))
}

/// Parse a system manifest from a file path.
pub fn parse_system_manifest_file(path: &Path) -> Result<SystemManifest, DomainScanError> {
    let content = std::fs::read_to_string(path)?;
    parse_system_manifest(&content)
}

// ---------------------------------------------------------------------------
// Matching algorithm
// ---------------------------------------------------------------------------

/// Match all entities from a ScanIndex against a manifest.
///
/// Algorithm (from spec 10a.4):
/// 1. Flatten manifest tree depth-first
/// 2. For each entity, collect all subsystems whose filePath is a prefix
/// 3. Select the deepest match (child at depth 2 wins over parent at depth 1)
/// 4. If no filePath match, fall back to name matching
/// 5. If still unmatched, place in unmatched bucket
pub fn match_entities(index: &ScanIndex, manifest: &Manifest) -> MatchResult {
    let flat = flatten_manifest(manifest);
    let summaries = index.get_entity_summaries(&Default::default());
    let total_entities = summaries.len();
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();

    for summary in summaries {
        if let Some((sub_id, sub_name, strategy)) = find_match(&summary, &flat) {
            matched.push(MatchedEntity {
                entity: summary,
                subsystem_id: sub_id,
                subsystem_name: sub_name,
                match_strategy: strategy,
            });
        } else {
            let candidates = find_candidate_subsystems(&summary, &flat);
            unmatched.push(UnmatchedEntity {
                entity: summary,
                candidate_subsystems: candidates,
            });
        }
    }

    let coverage_percent = if total_entities == 0 {
        100.0
    } else {
        (matched.len() as f64 / total_entities as f64) * 100.0
    };

    MatchResult {
        matched,
        unmatched,
        total_entities,
        coverage_percent,
    }
}

fn find_match(
    entity: &EntitySummary,
    flat: &[FlatSubsystem],
) -> Option<(String, String, MatchStrategy)> {
    // Strategy 1: File path prefix match (deepest wins)
    let mut best_match: Option<(&FlatSubsystem, usize)> = None;
    for sub in flat {
        if entity.file.starts_with(&sub.file_path) {
            let is_deeper = best_match
                .as_ref()
                .is_none_or(|(_, depth)| sub.depth > *depth);
            if is_deeper {
                best_match = Some((sub, sub.depth));
            }
        }
    }
    if let Some((sub, _)) = best_match {
        return Some((sub.id.clone(), sub.name.clone(), MatchStrategy::FilePath));
    }

    // Strategy 2: Name matching against interfaces/operations/tables/events
    let entity_name = &entity.name;
    let entity_name_with_parens = format!("{entity_name}()");

    for sub in flat {
        // Check interfaces
        if sub.interfaces.iter().any(|i| i == entity_name) {
            return Some((sub.id.clone(), sub.name.clone(), MatchStrategy::NameMatch));
        }
        // Check operations
        if sub.operations.iter().any(|o| o == &entity_name_with_parens || o == entity_name) {
            return Some((sub.id.clone(), sub.name.clone(), MatchStrategy::NameMatch));
        }
        // Check tables (for schema entities)
        if entity.kind == EntityKind::Schema && sub.tables.iter().any(|t| t == entity_name) {
            return Some((sub.id.clone(), sub.name.clone(), MatchStrategy::NameMatch));
        }
        // Check events
        if sub.events.iter().any(|e| e == entity_name) {
            return Some((sub.id.clone(), sub.name.clone(), MatchStrategy::NameMatch));
        }
    }

    None
}

/// Find candidate subsystems for an unmatched entity (best guesses).
fn find_candidate_subsystems(entity: &EntitySummary, flat: &[FlatSubsystem]) -> Vec<String> {
    let entity_name_lower = entity.name.to_lowercase();
    let mut candidates = Vec::new();

    for sub in flat {
        // Simple heuristic: does the entity name contain the subsystem id or name?
        let sub_id_lower = sub.id.to_lowercase();
        let sub_name_lower = sub.name.to_lowercase();

        if entity_name_lower.contains(&sub_id_lower)
            || sub_id_lower.contains(&entity_name_lower)
            || entity_name_lower.contains(&sub_name_lower)
            || sub_name_lower.contains(&entity_name_lower)
        {
            candidates.push(sub.id.clone());
        }
    }

    // Deduplicate
    candidates.sort();
    candidates.dedup();
    candidates
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate a manifest's naming conventions.
/// Returns a list of (subsystem_id, field, value, expected_pattern) violations.
pub fn validate_manifest(manifest: &Manifest) -> Vec<ManifestViolation> {
    let flat = flatten_manifest(manifest);
    let mut violations = Vec::new();

    for sub in &flat {
        // interfaces: PascalCase
        for iface in &sub.interfaces {
            if !is_pascal_case(iface) {
                violations.push(ManifestViolation {
                    subsystem_id: sub.id.clone(),
                    field: "interfaces".to_string(),
                    value: iface.clone(),
                    expected: "PascalCase (^[A-Z][A-Za-z0-9]*$)".to_string(),
                });
            }
        }

        // operations: camelCase with ()
        for op in &sub.operations {
            if !is_operation_format(op) {
                violations.push(ManifestViolation {
                    subsystem_id: sub.id.clone(),
                    field: "operations".to_string(),
                    value: op.clone(),
                    expected: "camelCase with () (^[a-z][A-Za-z0-9]*\\(\\)$)".to_string(),
                });
            }
        }

        // tables: snake_case
        for table in &sub.tables {
            if !is_snake_case(table) {
                violations.push(ManifestViolation {
                    subsystem_id: sub.id.clone(),
                    field: "tables".to_string(),
                    value: table.clone(),
                    expected: "snake_case (^[a-z][a-z0-9_]*$)".to_string(),
                });
            }
        }

        // events: dot.notation
        for event in &sub.events {
            if !is_dot_notation(event) {
                violations.push(ManifestViolation {
                    subsystem_id: sub.id.clone(),
                    field: "events".to_string(),
                    value: event.clone(),
                    expected: "dot.notation (^[a-z][a-z0-9]*(\\.[a-z][a-z0-9]*)+$)".to_string(),
                });
            }
        }
    }

    violations
}

/// A naming convention violation in a manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestViolation {
    pub subsystem_id: String,
    pub field: String,
    pub value: String,
    pub expected: String,
}

fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

fn is_operation_format(s: &str) -> bool {
    if !s.ends_with("()") {
        return false;
    }
    let name = &s[..s.len() - 2];
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

fn is_snake_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_dot_notation(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|part| {
        let mut chars = part.chars();
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() => {}
            _ => return false,
        }
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

// ---------------------------------------------------------------------------
// Write-back
// ---------------------------------------------------------------------------

/// Write-back: update a manifest with newly discovered entities.
///
/// Rules:
/// - Additive only. Never removes entries.
/// - Human-authored fields are never touched.
/// - Duplicates are never introduced.
/// - Status upgraded from new/boilerplate to built if BuildStatus::Built detected.
pub fn write_back(
    manifest: &mut Manifest,
    match_result: &MatchResult,
    index: &ScanIndex,
) {
    for m in &match_result.matched {
        write_back_to_subsystem(&mut manifest.subsystems, m, index);
    }
}

fn write_back_to_subsystem(
    subsystems: &mut [ManifestSubsystem],
    matched: &MatchedEntity,
    index: &ScanIndex,
) {
    for sub in subsystems.iter_mut() {
        if sub.id == matched.subsystem_id {
            // Add entity to appropriate list
            match matched.entity.kind {
                EntityKind::Interface => {
                    if !sub.interfaces.contains(&matched.entity.name) {
                        sub.interfaces.push(matched.entity.name.clone());
                    }
                }
                EntityKind::Method | EntityKind::Function => {
                    let op = format!("{}()", matched.entity.name);
                    if !sub.operations.contains(&op) {
                        sub.operations.push(op);
                    }
                }
                EntityKind::Schema => {
                    if !sub.tables.contains(&matched.entity.name) {
                        sub.tables.push(matched.entity.name.clone());
                    }
                }
                _ => {}
            }

            // Upgrade status if built
            if matched.entity.build_status == BuildStatus::Built
                && (sub.status == ManifestStatus::New || sub.status == ManifestStatus::Boilerplate)
            {
                // Check if at least one file in this subsystem is Built
                let has_built = index.files.iter().any(|f| {
                    f.path.starts_with(&sub.file_path) && f.build_status == BuildStatus::Built
                });
                if has_built {
                    sub.status = ManifestStatus::Built;
                }
            }
            return;
        }
        // Recurse into children
        write_back_to_subsystem(&mut sub.children, matched, index);
    }
}

/// Serialize the manifest back to pretty-printed JSON.
pub fn serialize_manifest(manifest: &Manifest) -> Result<String, DomainScanError> {
    serde_json::to_string_pretty(manifest).map_err(DomainScanError::Serialization)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn sample_manifest_json() -> &'static str {
        r#"{
            "subsystems": [
                {
                    "id": "auth",
                    "name": "Auth & Identity",
                    "domain": "platform-core",
                    "status": "built",
                    "filePath": "/project/src/auth/",
                    "interfaces": ["AuthPrincipal"],
                    "operations": ["signToken()", "verifyToken()"],
                    "tables": ["users"],
                    "events": ["session.created"],
                    "children": [
                        {
                            "id": "auth-jwt",
                            "name": "JWT Provider",
                            "domain": "platform-core",
                            "status": "built",
                            "filePath": "/project/src/auth/jwt/",
                            "interfaces": ["JWTClaims"],
                            "operations": [],
                            "tables": [],
                            "events": [],
                            "children": [],
                            "dependencies": []
                        }
                    ],
                    "dependencies": ["billing"]
                },
                {
                    "id": "billing",
                    "name": "Billing",
                    "domain": "platform-core",
                    "status": "new",
                    "filePath": "/project/src/billing/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": []
                }
            ]
        }"#
    }

    /// Helper: parse the sample manifest, panicking on failure (test-only).
    fn sample_manifest() -> Manifest {
        parse_manifest(sample_manifest_json())
            .unwrap_or_else(|e| panic!("Failed to parse sample manifest: {e}"))
    }

    fn make_iface(name: &str, path: &str) -> InterfaceDef {
        InterfaceDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            visibility: Visibility::Public,
            generics: Vec::new(),
            extends: Vec::new(),
            methods: Vec::new(),
            properties: Vec::new(),
            language_kind: InterfaceKind::Interface,
            decorators: Vec::new(),
        }
    }

    #[test]
    fn test_parse_manifest() {
        let manifest = sample_manifest();
        assert_eq!(manifest.subsystems.len(), 2);
        assert_eq!(manifest.subsystems[0].id, "auth");
        assert_eq!(manifest.subsystems[0].children.len(), 1);
    }

    #[test]
    fn test_flatten_manifest() {
        let manifest = sample_manifest();
        let flat = flatten_manifest(&manifest);
        assert_eq!(flat.len(), 3); // auth, auth-jwt, billing
        assert_eq!(flat[0].depth, 0);
        assert_eq!(flat[1].depth, 1); // auth-jwt is a child
        assert_eq!(flat[2].depth, 0);
    }

    #[test]
    fn test_match_by_file_path() {
        let manifest = sample_manifest();

        let mut file = IrFile::new(
            PathBuf::from("/project/src/auth/jwt/token.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("TokenService", "/project/src/auth/jwt/token.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        // Should match to auth-jwt (deeper) not auth
        assert_eq!(result.matched.len(), 1);
        assert_eq!(result.matched[0].subsystem_id, "auth-jwt");
        assert_eq!(result.matched[0].match_strategy, MatchStrategy::FilePath);
        assert!(result.unmatched.is_empty());
    }

    #[test]
    fn test_match_by_name() {
        let manifest = sample_manifest();

        let mut file = IrFile::new(
            PathBuf::from("/project/src/other/principal.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("AuthPrincipal", "/project/src/other/principal.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert_eq!(result.matched.len(), 1);
        assert_eq!(result.matched[0].subsystem_id, "auth");
        assert_eq!(result.matched[0].match_strategy, MatchStrategy::NameMatch);
    }

    #[test]
    fn test_unmatched_entity() {
        let manifest = sample_manifest();

        let mut file = IrFile::new(
            PathBuf::from("/project/src/random/foo.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("UnknownThing", "/project/src/random/foo.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert!(result.matched.is_empty());
        assert_eq!(result.unmatched.len(), 1);
    }

    #[test]
    fn test_validate_manifest_clean() {
        let manifest = sample_manifest();
        let violations = validate_manifest(&manifest);
        assert!(violations.is_empty(), "Expected no violations, got: {violations:?}");
    }

    #[test]
    fn test_validate_manifest_violations() {
        let json = r#"{
            "subsystems": [{
                "id": "bad",
                "name": "Bad Names",
                "domain": "test",
                "status": "built",
                "filePath": "/project/src/",
                "interfaces": ["lowercase_bad"],
                "operations": ["MissingParens"],
                "tables": ["CamelCase"],
                "events": ["no_dots"],
                "children": [],
                "dependencies": []
            }]
        }"#;

        let manifest = parse_manifest(json)
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        let violations = validate_manifest(&manifest);
        assert_eq!(violations.len(), 4);
    }

    #[test]
    fn test_naming_conventions() {
        assert!(is_pascal_case("AuthPrincipal"));
        assert!(is_pascal_case("A"));
        assert!(!is_pascal_case("authPrincipal"));
        assert!(!is_pascal_case("auth_principal"));
        assert!(!is_pascal_case(""));

        assert!(is_operation_format("signToken()"));
        assert!(is_operation_format("a()"));
        assert!(!is_operation_format("SignToken()"));
        assert!(!is_operation_format("signToken"));
        assert!(!is_operation_format("()"));

        assert!(is_snake_case("users"));
        assert!(is_snake_case("auth_login_sessions"));
        assert!(!is_snake_case("Users"));
        assert!(!is_snake_case("authLogin"));

        assert!(is_dot_notation("session.created"));
        assert!(is_dot_notation("auth.session.created"));
        assert!(!is_dot_notation("session"));
        assert!(!is_dot_notation("Session.created"));
    }

    #[test]
    fn test_coverage_percent() {
        let manifest = sample_manifest();

        let mut file = IrFile::new(
            PathBuf::from("/project/src/auth/login.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("LoginService", "/project/src/auth/login.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);
        assert!((result.coverage_percent - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_write_back_adds_interface() {
        let mut manifest = sample_manifest();

        let file = IrFile::new(
            PathBuf::from("/project/src/auth/login.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        let match_result = MatchResult {
            matched: vec![MatchedEntity {
                entity: EntitySummary {
                    name: "NewInterface".to_string(),
                    kind: EntityKind::Interface,
                    file: PathBuf::from("/project/src/auth/login.ts"),
                    line: 1,
                    language: Language::TypeScript,
                    build_status: BuildStatus::Built,
                    confidence: Confidence::High,
                },
                subsystem_id: "auth".to_string(),
                subsystem_name: "Auth & Identity".to_string(),
                match_strategy: MatchStrategy::FilePath,
            }],
            unmatched: vec![],
            total_entities: 1,
            coverage_percent: 100.0,
        };

        write_back(&mut manifest, &match_result, &index);
        assert!(manifest.subsystems[0].interfaces.contains(&"NewInterface".to_string()));
        assert!(manifest.subsystems[0].interfaces.contains(&"AuthPrincipal".to_string()));
    }

    #[test]
    fn test_write_back_no_duplicates() {
        let mut manifest = sample_manifest();

        let file = IrFile::new(
            PathBuf::from("/project/src/auth/login.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        let match_result = MatchResult {
            matched: vec![MatchedEntity {
                entity: EntitySummary {
                    name: "AuthPrincipal".to_string(),
                    kind: EntityKind::Interface,
                    file: PathBuf::from("/project/src/auth/login.ts"),
                    line: 1,
                    language: Language::TypeScript,
                    build_status: BuildStatus::Built,
                    confidence: Confidence::High,
                },
                subsystem_id: "auth".to_string(),
                subsystem_name: "Auth & Identity".to_string(),
                match_strategy: MatchStrategy::FilePath,
            }],
            unmatched: vec![],
            total_entities: 1,
            coverage_percent: 100.0,
        };

        let before_count = manifest.subsystems[0].interfaces.len();
        write_back(&mut manifest, &match_result, &index);
        assert_eq!(manifest.subsystems[0].interfaces.len(), before_count);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let manifest = sample_manifest();
        let json = serialize_manifest(&manifest)
            .unwrap_or_else(|e| panic!("Failed to serialize: {e}"));
        let reparsed = parse_manifest(&json)
            .unwrap_or_else(|e| panic!("Failed to reparse: {e}"));
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn test_empty_manifest() {
        let json = r#"{"subsystems": []}"#;
        let manifest = parse_manifest(json)
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        let index = crate::index::build_index(PathBuf::from("/project"), vec![], 0, 0, 0);
        let result = match_entities(&index, &manifest);
        assert_eq!(result.total_entities, 0);
        assert!((result.coverage_percent - 100.0).abs() < f64::EPSILON);
    }

    fn sample_system_manifest_json() -> &'static str {
        r##"{
            "meta": {
                "name": "TestSystem",
                "version": "1.0",
                "description": "A test system"
            },
            "domains": {
                "platform-core": { "label": "Platform Core", "color": "#3b82f6" },
                "services": { "label": "Services", "color": "#f97316" }
            },
            "subsystems": [
                {
                    "id": "auth",
                    "name": "Auth & Identity",
                    "domain": "platform-core",
                    "status": "built",
                    "filePath": "/project/src/auth/",
                    "interfaces": ["AuthPrincipal"],
                    "operations": [],
                    "tables": ["users"],
                    "events": [],
                    "children": [],
                    "dependencies": ["billing"]
                },
                {
                    "id": "billing",
                    "name": "Billing",
                    "domain": "services",
                    "status": "new",
                    "filePath": "/project/src/billing/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": []
                }
            ],
            "connections": [
                {
                    "from": "auth",
                    "to": "billing",
                    "label": "checks subscription",
                    "type": "depends_on"
                },
                {
                    "from": "billing",
                    "to": "auth",
                    "label": "validates identity",
                    "type": "uses"
                }
            ]
        }"##
    }

    #[test]
    fn test_parse_system_manifest() {
        let manifest = parse_system_manifest(sample_system_manifest_json())
            .unwrap_or_else(|e| panic!("Failed to parse system manifest: {e}"));

        assert_eq!(manifest.meta.name, "TestSystem");
        assert_eq!(manifest.meta.version, "1.0");
        assert_eq!(manifest.domains.len(), 2);
        assert_eq!(manifest.domains["platform-core"].color, "#3b82f6");
        assert_eq!(manifest.subsystems.len(), 2);
        assert_eq!(manifest.connections.len(), 2);
        assert_eq!(manifest.connections[0].connection_type, ConnectionType::DependsOn);
        assert_eq!(manifest.connections[1].connection_type, ConnectionType::Uses);
    }

    #[test]
    fn test_system_manifest_as_manifest() {
        let sys = parse_system_manifest(sample_system_manifest_json())
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        let manifest = sys.as_manifest();
        assert_eq!(manifest.subsystems.len(), 2);
        assert_eq!(manifest.subsystems[0].id, "auth");
    }

    #[test]
    fn test_system_manifest_matching() {
        let sys = parse_system_manifest(sample_system_manifest_json())
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        let manifest = sys.as_manifest();

        let mut file = IrFile::new(
            PathBuf::from("/project/src/auth/login.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("LoginService", "/project/src/auth/login.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);
        assert_eq!(result.matched.len(), 1);
        assert_eq!(result.matched[0].subsystem_id, "auth");
    }

    #[test]
    fn test_system_manifest_empty_domains() {
        let json = r#"{
            "meta": { "name": "Minimal", "version": "0.1", "description": "" },
            "subsystems": [],
            "connections": []
        }"#;
        let manifest = parse_system_manifest(json)
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        assert!(manifest.domains.is_empty());
        assert!(manifest.subsystems.is_empty());
        assert!(manifest.connections.is_empty());
    }

    #[test]
    fn test_connection_type_serde() {
        let conn = Connection {
            from: "a".to_string(),
            to: "b".to_string(),
            label: "test".to_string(),
            connection_type: ConnectionType::Triggers,
        };
        let json = serde_json::to_string(&conn)
            .unwrap_or_else(|e| panic!("Failed to serialize: {e}"));
        assert!(json.contains("\"type\":\"triggers\""));
        let deserialized: Connection = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("Failed to deserialize: {e}"));
        assert_eq!(deserialized.connection_type, ConnectionType::Triggers);
    }
}
