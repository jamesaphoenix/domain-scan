//! Manifest parsing, matching, validation, and write-back.
//!
//! A manifest describes the expected subsystem structure of a codebase.
//! Compatible with octospark-visualizer's `system.json` format.
//!
//! The matching algorithm maps extracted entities to subsystems by:
//! 1. File path prefix (deepest match wins)
//! 2. Name matching against interfaces/operations/tables/events
//! 3. Unmatched bucket for human review or LLM enrichment

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use globset::Glob;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::ir::{
    BuildStatus, EntityKind, EntitySummary, MatchResult, MatchStrategy, MatchedEntity, ScanIndex,
    UnmatchedEntity,
};
use crate::DomainScanError;

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// A manifest describing the expected subsystem structure.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct Manifest {
    pub subsystems: Vec<ManifestSubsystem>,
}

/// Extended manifest with meta, domains, and connections (system.json format).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ManifestMeta {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
}

/// A domain definition with label and color.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct DomainDef {
    pub label: String,
    pub color: String,
}

/// A connection between two subsystems.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub label: String,
    #[serde(rename = "type")]
    pub connection_type: ConnectionType,
}

/// The type of connection between subsystems.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    DependsOn,
    Uses,
    Triggers,
}

/// A subsystem in the manifest (recursive via `children`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
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
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
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

/// A flattened subsystem for matching.
#[derive(Debug, Clone)]
struct FlatSubsystem {
    id: String,
    name: String,
    file_path: PathBuf,
    interfaces: Vec<String>,
    operations: Vec<String>,
    tables: Vec<String>,
    events: Vec<String>,
}

/// Flatten the manifest tree depth-first.
fn flatten_manifest(manifest: &Manifest) -> Vec<FlatSubsystem> {
    let mut result = Vec::new();
    for subsystem in &manifest.subsystems {
        flatten_recursive(subsystem, &mut result);
    }
    result
}

fn flatten_recursive(subsystem: &ManifestSubsystem, out: &mut Vec<FlatSubsystem>) {
    out.push(FlatSubsystem {
        id: subsystem.id.clone(),
        name: subsystem.name.clone(),
        file_path: subsystem.file_path.clone(),
        interfaces: subsystem.interfaces.clone(),
        operations: subsystem.operations.clone(),
        tables: subsystem.tables.clone(),
        events: subsystem.events.clone(),
    });
    for child in &subsystem.children {
        flatten_recursive(child, out);
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a manifest from a JSON string.
pub fn parse_manifest(json: &str) -> Result<Manifest, DomainScanError> {
    serde_json::from_str(json)
        .map_err(|e| DomainScanError::Config(format!("manifest parse error: {e}")))
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
/// 3. Select the most specific match (path component count, not tree depth)
/// 4. If no filePath match, fall back to name matching
/// 5. If still unmatched, place in unmatched bucket
pub fn match_entities(index: &ScanIndex, manifest: &Manifest) -> MatchResult {
    let flat = flatten_manifest(manifest);
    let scan_root = &index.root;
    let summaries = index.get_entity_summaries(&Default::default());
    let total_entities = summaries.len();
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();

    for summary in summaries {
        if let Some((sub_id, sub_name, strategy)) = find_match(&summary, &flat, scan_root) {
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

/// Check whether a path string contains glob metacharacters (`*`, `?`, `[`, `{`).
pub fn is_glob_pattern(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}

/// Validate that a glob pattern compiles successfully.
/// Returns `Err(DomainScanError::InvalidInput)` for malformed patterns.
pub fn validate_glob_pattern(pattern: &str) -> Result<(), DomainScanError> {
    Glob::new(pattern).map_err(|e| {
        DomainScanError::InvalidInput(format!("Invalid glob pattern '{pattern}': {e}"))
    })?;
    Ok(())
}

/// Validate all glob patterns in a manifest's subsystem `filePath` fields.
/// Returns the first invalid glob as a `DomainScanError`.
pub fn validate_manifest_globs(manifest: &Manifest) -> Result<(), DomainScanError> {
    for sub in &manifest.subsystems {
        if is_glob_pattern(&sub.file_path) {
            validate_glob_pattern(&sub.file_path.to_string_lossy())?;
        }
        for child in &sub.children {
            if is_glob_pattern(&child.file_path) {
                validate_glob_pattern(&child.file_path.to_string_lossy())?;
            }
        }
    }
    Ok(())
}

fn find_match(
    entity: &EntitySummary,
    flat: &[FlatSubsystem],
    scan_root: &Path,
) -> Option<(String, String, MatchStrategy)> {
    // Strategy 1: File path prefix match (most specific path wins)
    // Supports both prefix matching and glob patterns in filePath.
    let mut best_match: Option<(&FlatSubsystem, usize)> = None;
    for sub in flat {
        let resolved = if sub.file_path.is_relative() {
            scan_root.join(&sub.file_path)
        } else {
            sub.file_path.clone()
        };

        let matches = if is_glob_pattern(&resolved) {
            // Glob matching: compile the pattern and test against the entity path
            Glob::new(&resolved.to_string_lossy())
                .ok()
                .map(|g| g.compile_matcher())
                .is_some_and(|m| m.is_match(&entity.file))
        } else {
            entity.file.starts_with(&resolved)
        };

        if matches {
            let specificity = resolved.components().count();
            let is_more_specific = best_match.as_ref().is_none_or(|(_, s)| specificity > *s);
            if is_more_specific {
                best_match = Some((sub, specificity));
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
        if sub
            .operations
            .iter()
            .any(|o| o == &entity_name_with_parens || o == entity_name)
        {
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

/// Validate a full SystemManifest, including naming rules and semantic
/// cross-reference integrity between domains, dependencies, and connections.
pub fn validate_system_manifest(system_manifest: &SystemManifest) -> Vec<ManifestViolation> {
    let mut violations = validate_manifest(&system_manifest.as_manifest());
    let mut seen_ids = HashSet::new();
    let mut all_ids = HashSet::new();

    collect_subsystem_ids(
        &system_manifest.subsystems,
        &mut seen_ids,
        &mut all_ids,
        &mut violations,
    );
    validate_subsystem_semantics(
        &system_manifest.subsystems,
        &system_manifest.domains,
        &all_ids,
        None,
        &mut violations,
    );
    validate_connections(&system_manifest.connections, &all_ids, &mut violations);

    violations
}

/// A naming convention violation in a manifest.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ManifestViolation {
    pub subsystem_id: String,
    pub field: String,
    pub value: String,
    pub expected: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct ManifestValidationReport {
    pub manifest_path: String,
    pub domains: usize,
    pub subsystems: usize,
    pub connections: usize,
    pub validation_errors: usize,
    pub violations: Vec<ManifestViolation>,
    pub coverage_percent: f64,
    pub matched: usize,
    pub unmatched: usize,
}

fn collect_subsystem_ids(
    subsystems: &[ManifestSubsystem],
    seen_ids: &mut HashSet<String>,
    all_ids: &mut HashSet<String>,
    violations: &mut Vec<ManifestViolation>,
) {
    for sub in subsystems {
        if !seen_ids.insert(sub.id.clone()) {
            violations.push(ManifestViolation {
                subsystem_id: sub.id.clone(),
                field: "id".to_string(),
                value: sub.id.clone(),
                expected: "unique subsystem id".to_string(),
            });
        }
        all_ids.insert(sub.id.clone());
        collect_subsystem_ids(&sub.children, seen_ids, all_ids, violations);
    }
}

fn validate_subsystem_semantics(
    subsystems: &[ManifestSubsystem],
    domains: &HashMap<String, DomainDef>,
    subsystem_ids: &HashSet<String>,
    parent_domain: Option<&str>,
    violations: &mut Vec<ManifestViolation>,
) {
    for sub in subsystems {
        if sub.file_path.as_os_str().is_empty() {
            violations.push(ManifestViolation {
                subsystem_id: sub.id.clone(),
                field: "filePath".to_string(),
                value: String::new(),
                expected: "non-empty directory path".to_string(),
            });
        }

        if !domains.is_empty() && !sub.domain.is_empty() && !domains.contains_key(&sub.domain) {
            violations.push(ManifestViolation {
                subsystem_id: sub.id.clone(),
                field: "domain".to_string(),
                value: sub.domain.clone(),
                expected: "existing domain id".to_string(),
            });
        }

        if let Some(parent) = parent_domain {
            if !parent.is_empty() && !sub.domain.is_empty() && sub.domain != parent {
                violations.push(ManifestViolation {
                    subsystem_id: sub.id.clone(),
                    field: "domain".to_string(),
                    value: sub.domain.clone(),
                    expected: format!("same domain as parent ({parent})"),
                });
            }
        }

        for dependency in &sub.dependencies {
            if !subsystem_ids.contains(dependency) {
                violations.push(ManifestViolation {
                    subsystem_id: sub.id.clone(),
                    field: "dependencies".to_string(),
                    value: dependency.clone(),
                    expected: "existing subsystem id".to_string(),
                });
            }
        }

        validate_subsystem_semantics(
            &sub.children,
            domains,
            subsystem_ids,
            Some(&sub.domain),
            violations,
        );
    }
}

fn validate_connections(
    connections: &[Connection],
    subsystem_ids: &HashSet<String>,
    violations: &mut Vec<ManifestViolation>,
) {
    for conn in connections {
        if !subsystem_ids.contains(&conn.from) {
            violations.push(ManifestViolation {
                subsystem_id: conn.from.clone(),
                field: "connections.from".to_string(),
                value: conn.from.clone(),
                expected: "existing subsystem id".to_string(),
            });
        }
        if !subsystem_ids.contains(&conn.to) {
            violations.push(ManifestViolation {
                subsystem_id: conn.to.clone(),
                field: "connections.to".to_string(),
                value: conn.to.clone(),
                expected: "existing subsystem id".to_string(),
            });
        }
    }
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
/// - Status is never changed (human-controlled).
pub fn write_back(manifest: &mut Manifest, match_result: &MatchResult, _index: &ScanIndex) {
    for m in &match_result.matched {
        write_back_to_subsystem(&mut manifest.subsystems, m);
    }
}

fn write_back_to_subsystem(subsystems: &mut [ManifestSubsystem], matched: &MatchedEntity) {
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

            // Status is human-controlled: never auto-upgrade.
            // Only the user can promote a subsystem to "built".
            return;
        }
        // Recurse into children
        write_back_to_subsystem(&mut sub.children, matched);
    }
}

/// Serialize the manifest back to pretty-printed JSON.
pub fn serialize_manifest(manifest: &Manifest) -> Result<String, DomainScanError> {
    serde_json::to_string_pretty(manifest).map_err(DomainScanError::Serialization)
}

/// Serialize a full SystemManifest back to pretty-printed JSON.
/// Preserves meta, domains, connections alongside subsystems.
pub fn serialize_system_manifest(manifest: &SystemManifest) -> Result<String, DomainScanError> {
    serde_json::to_string_pretty(manifest).map_err(DomainScanError::Serialization)
}

/// Write-back into a full SystemManifest. Preserves meta, domains, connections.
pub fn write_back_system(
    manifest: &mut SystemManifest,
    match_result: &MatchResult,
    _index: &ScanIndex,
) {
    for m in &match_result.matched {
        write_back_to_subsystem(&mut manifest.subsystems, m);
    }
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
        assert_eq!(flat[0].id, "auth");
        assert_eq!(flat[1].id, "auth-jwt");
        assert_eq!(flat[2].id, "billing");
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
        file.interfaces = vec![make_iface(
            "AuthPrincipal",
            "/project/src/other/principal.ts",
        )];

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
        assert!(
            violations.is_empty(),
            "Expected no violations, got: {violations:?}"
        );
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

        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));
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
        assert!(manifest.subsystems[0]
            .interfaces
            .contains(&"NewInterface".to_string()));
        assert!(manifest.subsystems[0]
            .interfaces
            .contains(&"AuthPrincipal".to_string()));
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
        let json =
            serialize_manifest(&manifest).unwrap_or_else(|e| panic!("Failed to serialize: {e}"));
        let reparsed = parse_manifest(&json).unwrap_or_else(|e| panic!("Failed to reparse: {e}"));
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn test_empty_manifest() {
        let json = r#"{"subsystems": []}"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

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
        assert_eq!(
            manifest.connections[0].connection_type,
            ConnectionType::DependsOn
        );
        assert_eq!(
            manifest.connections[1].connection_type,
            ConnectionType::Uses
        );
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
        let manifest =
            parse_system_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        assert!(manifest.domains.is_empty());
        assert!(manifest.subsystems.is_empty());
        assert!(manifest.connections.is_empty());
    }

    #[test]
    fn test_validate_system_manifest_clean() {
        let manifest = parse_system_manifest(sample_system_manifest_json())
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        let violations = validate_system_manifest(&manifest);
        assert!(
            violations.is_empty(),
            "Expected no violations, got: {violations:?}"
        );
    }

    #[test]
    fn test_validate_system_manifest_semantic_violations() {
        let json = r##"{
            "meta": { "name": "Bad", "version": "1.0", "description": "" },
            "domains": {
                "platform": { "label": "Platform", "color": "#3b82f6" }
            },
            "subsystems": [
                {
                    "id": "auth",
                    "name": "Auth",
                    "domain": "missing-domain",
                    "status": "new",
                    "filePath": "/project/src/auth/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": ["ghost-subsystem"]
                },
                {
                    "id": "auth",
                    "name": "Duplicate Auth",
                    "domain": "platform",
                    "status": "new",
                    "filePath": "/project/src/auth-2/",
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
                    "to": "missing-target",
                    "label": "calls",
                    "type": "depends_on"
                }
            ]
        }"##;

        let manifest =
            parse_system_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        let violations = validate_system_manifest(&manifest);

        assert!(violations
            .iter()
            .any(|v| v.field == "domain" && v.value == "missing-domain"));
        assert!(violations
            .iter()
            .any(|v| v.field == "dependencies" && v.value == "ghost-subsystem"));
        assert!(violations
            .iter()
            .any(|v| v.field == "connections.to" && v.value == "missing-target"));
        assert!(violations
            .iter()
            .any(|v| v.field == "id" && v.value == "auth"));
    }

    #[test]
    fn test_connection_type_serde() {
        let conn = Connection {
            from: "a".to_string(),
            to: "b".to_string(),
            label: "test".to_string(),
            connection_type: ConnectionType::Triggers,
        };
        let json =
            serde_json::to_string(&conn).unwrap_or_else(|e| panic!("Failed to serialize: {e}"));
        assert!(json.contains("\"type\":\"triggers\""));
        let deserialized: Connection =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to deserialize: {e}"));
        assert_eq!(deserialized.connection_type, ConnectionType::Triggers);
    }

    // -----------------------------------------------------------------------
    // A.7: SystemManifest write-back round-trip preserves meta/domains/connections
    // -----------------------------------------------------------------------

    #[test]
    fn test_system_manifest_write_back_roundtrip() {
        let original = parse_system_manifest(sample_system_manifest_json())
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        let mut sys_manifest = original.clone();

        // Create an entity that matches the "auth" subsystem
        let mut file = IrFile::new(
            PathBuf::from("/project/src/auth/login.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("NewAuthInterface", "/project/src/auth/login.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let manifest_for_match = sys_manifest.as_manifest();
        let result = match_entities(&index, &manifest_for_match);

        // Write back into the SystemManifest
        write_back_system(&mut sys_manifest, &result, &index);

        // Serialize and re-parse
        let serialized = serialize_system_manifest(&sys_manifest)
            .unwrap_or_else(|e| panic!("Failed to serialize: {e}"));
        let reparsed =
            parse_system_manifest(&serialized).unwrap_or_else(|e| panic!("Failed to reparse: {e}"));

        // meta, domains, connections must survive the round-trip
        assert_eq!(reparsed.meta, original.meta);
        assert_eq!(reparsed.domains, original.domains);
        assert_eq!(reparsed.connections, original.connections);

        // The new interface should be present
        assert!(reparsed.subsystems[0]
            .interfaces
            .contains(&"NewAuthInterface".to_string()));
        // Original interfaces still present
        assert!(reparsed.subsystems[0]
            .interfaces
            .contains(&"AuthPrincipal".to_string()));
    }

    #[test]
    fn test_system_manifest_write_back_dry_run_preview_includes_all_sections() {
        let sys_manifest = parse_system_manifest(sample_system_manifest_json())
            .unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        // Serialize the full manifest (as dry-run preview would)
        let serialized = serialize_system_manifest(&sys_manifest)
            .unwrap_or_else(|e| panic!("Failed to serialize: {e}"));

        // Parse as generic JSON to verify structure
        let value: serde_json::Value = serde_json::from_str(&serialized)
            .unwrap_or_else(|e| panic!("Failed to parse JSON: {e}"));
        let obj = value
            .as_object()
            .unwrap_or_else(|| panic!("Expected JSON object"));

        assert!(obj.contains_key("meta"), "Preview must contain 'meta'");
        assert!(
            obj.contains_key("domains"),
            "Preview must contain 'domains'"
        );
        assert!(
            obj.contains_key("subsystems"),
            "Preview must contain 'subsystems'"
        );
        assert!(
            obj.contains_key("connections"),
            "Preview must contain 'connections'"
        );
    }

    #[test]
    fn test_fallback_manifest_only_subsystems() {
        // A manifest with only subsystems (no meta/domains/connections)
        let manifest = sample_manifest();
        let serialized =
            serialize_manifest(&manifest).unwrap_or_else(|e| panic!("Failed to serialize: {e}"));
        let reparsed =
            parse_manifest(&serialized).unwrap_or_else(|e| panic!("Failed to reparse: {e}"));
        assert_eq!(manifest, reparsed);
    }

    // -----------------------------------------------------------------------
    // A.8: Relative filePaths match against absolute entity paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_relative_filepath_matching() {
        let json = r#"{
            "subsystems": [
                {
                    "id": "auth",
                    "name": "Auth",
                    "domain": "core",
                    "status": "new",
                    "filePath": "packages/auth/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": []
                }
            ]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        let mut file = IrFile::new(
            PathBuf::from("/abs/project/packages/auth/handler.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface(
            "AuthHandler",
            "/abs/project/packages/auth/handler.ts",
        )];

        let index = crate::index::build_index(PathBuf::from("/abs/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert_eq!(
            result.matched.len(),
            1,
            "Relative filePath should match absolute entity path"
        );
        assert_eq!(result.matched[0].subsystem_id, "auth");
        assert_eq!(result.matched[0].match_strategy, MatchStrategy::FilePath);
        assert!(result.coverage_percent > 0.0);
    }

    #[test]
    fn test_absolute_filepath_still_works() {
        // Absolute paths should continue to work as before
        let manifest = sample_manifest(); // uses absolute /project/src/auth/
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
    fn test_mixed_relative_absolute_paths() {
        let json = r#"{
            "subsystems": [
                {
                    "id": "auth",
                    "name": "Auth",
                    "domain": "core",
                    "status": "new",
                    "filePath": "packages/auth/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": []
                },
                {
                    "id": "billing",
                    "name": "Billing",
                    "domain": "core",
                    "status": "new",
                    "filePath": "/abs/project/packages/billing/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": []
                }
            ]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        let mut file1 = IrFile::new(
            PathBuf::from("/abs/project/packages/auth/handler.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file1.interfaces = vec![make_iface(
            "AuthHandler",
            "/abs/project/packages/auth/handler.ts",
        )];

        let mut file2 = IrFile::new(
            PathBuf::from("/abs/project/packages/billing/invoice.ts"),
            Language::TypeScript,
            "hash2".to_string(),
            BuildStatus::Built,
        );
        file2.interfaces = vec![make_iface(
            "InvoiceService",
            "/abs/project/packages/billing/invoice.ts",
        )];

        let index =
            crate::index::build_index(PathBuf::from("/abs/project"), vec![file1, file2], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert_eq!(
            result.matched.len(),
            2,
            "Both relative and absolute paths should match"
        );
        let auth_match = result.matched.iter().find(|m| m.subsystem_id == "auth");
        let billing_match = result.matched.iter().find(|m| m.subsystem_id == "billing");
        assert!(
            auth_match.is_some(),
            "Relative path 'packages/auth/' should match"
        );
        assert!(billing_match.is_some(), "Absolute path should match");
    }

    // -----------------------------------------------------------------------
    // A.9: Sibling subsystems resolve by path specificity, not array order
    // -----------------------------------------------------------------------

    #[test]
    fn test_path_specificity_sibling_subsystems() {
        let json = r#"{
            "subsystems": [
                {
                    "id": "platform-core",
                    "name": "Platform Core",
                    "domain": "platform",
                    "status": "new",
                    "filePath": "/project/packages/platform/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": []
                },
                {
                    "id": "test-utils",
                    "name": "Test Utils",
                    "domain": "platform",
                    "status": "new",
                    "filePath": "/project/packages/platform/test-utils/",
                    "interfaces": [],
                    "operations": [],
                    "tables": [],
                    "events": [],
                    "children": [],
                    "dependencies": []
                }
            ]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        // Entity under test-utils/ should match test-utils, NOT platform-core
        let mut file = IrFile::new(
            PathBuf::from("/project/packages/platform/test-utils/helpers.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface(
            "TestHelper",
            "/project/packages/platform/test-utils/helpers.ts",
        )];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert_eq!(result.matched.len(), 1);
        assert_eq!(
            result.matched[0].subsystem_id, "test-utils",
            "Entity under test-utils/ should match test-utils (more specific), not platform-core"
        );
    }

    #[test]
    fn test_path_specificity_parent_child_hierarchy() {
        // Existing behavior: child subsystem (auth-jwt) should win over parent (auth)
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

        assert_eq!(result.matched.len(), 1);
        assert_eq!(
            result.matched[0].subsystem_id, "auth-jwt",
            "Child subsystem auth-jwt should win over parent auth"
        );
    }

    // -----------------------------------------------------------------------
    // B.7: Write-back does not change subsystem status
    // -----------------------------------------------------------------------

    #[test]
    fn test_write_back_no_status_upgrade_new() {
        // status: "new" must stay "new" even when all entities are Built
        let json = r#"{
            "subsystems": [{
                "id": "auth",
                "name": "Auth",
                "domain": "core",
                "status": "new",
                "filePath": "/project/src/auth/",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let mut manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

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
                    name: "LoginService".to_string(),
                    kind: EntityKind::Interface,
                    file: PathBuf::from("/project/src/auth/login.ts"),
                    line: 1,
                    language: Language::TypeScript,
                    build_status: BuildStatus::Built,
                    confidence: Confidence::High,
                },
                subsystem_id: "auth".to_string(),
                subsystem_name: "Auth".to_string(),
                match_strategy: MatchStrategy::FilePath,
            }],
            unmatched: vec![],
            total_entities: 1,
            coverage_percent: 100.0,
        };

        write_back(&mut manifest, &match_result, &index);
        assert_eq!(
            manifest.subsystems[0].status,
            ManifestStatus::New,
            "Write-back must not upgrade status from 'new'"
        );
    }

    #[test]
    fn test_write_back_no_status_upgrade_boilerplate() {
        let json = r#"{
            "subsystems": [{
                "id": "auth",
                "name": "Auth",
                "domain": "core",
                "status": "boilerplate",
                "filePath": "/project/src/auth/",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let mut manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

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
                    name: "LoginService".to_string(),
                    kind: EntityKind::Interface,
                    file: PathBuf::from("/project/src/auth/login.ts"),
                    line: 1,
                    language: Language::TypeScript,
                    build_status: BuildStatus::Built,
                    confidence: Confidence::High,
                },
                subsystem_id: "auth".to_string(),
                subsystem_name: "Auth".to_string(),
                match_strategy: MatchStrategy::FilePath,
            }],
            unmatched: vec![],
            total_entities: 1,
            coverage_percent: 100.0,
        };

        write_back(&mut manifest, &match_result, &index);
        assert_eq!(
            manifest.subsystems[0].status,
            ManifestStatus::Boilerplate,
            "Write-back must not upgrade status from 'boilerplate'"
        );
    }

    #[test]
    fn test_write_back_no_status_downgrade_built() {
        let json = r#"{
            "subsystems": [{
                "id": "auth",
                "name": "Auth",
                "domain": "core",
                "status": "built",
                "filePath": "/project/src/auth/",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let mut manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

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
                    name: "LoginService".to_string(),
                    kind: EntityKind::Interface,
                    file: PathBuf::from("/project/src/auth/login.ts"),
                    line: 1,
                    language: Language::TypeScript,
                    build_status: BuildStatus::Built,
                    confidence: Confidence::High,
                },
                subsystem_id: "auth".to_string(),
                subsystem_name: "Auth".to_string(),
                match_strategy: MatchStrategy::FilePath,
            }],
            unmatched: vec![],
            total_entities: 1,
            coverage_percent: 100.0,
        };

        write_back(&mut manifest, &match_result, &index);
        assert_eq!(
            manifest.subsystems[0].status,
            ManifestStatus::Built,
            "Write-back must not downgrade status from 'built'"
        );
    }

    // -----------------------------------------------------------------------
    // D.5: Glob filePath matches individual files
    // -----------------------------------------------------------------------

    #[test]
    fn test_glob_filepath_matches_individual_files() {
        let json = r#"{
            "subsystems": [{
                "id": "scheduling",
                "name": "Scheduling",
                "domain": "services",
                "status": "new",
                "filePath": "/project/src/services/scheduling*",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        let mut file1 = IrFile::new(
            PathBuf::from("/project/src/services/scheduling.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file1.interfaces = vec![make_iface(
            "Scheduler",
            "/project/src/services/scheduling.ts",
        )];

        let mut file2 = IrFile::new(
            PathBuf::from("/project/src/services/schedulingUtils.ts"),
            Language::TypeScript,
            "hash2".to_string(),
            BuildStatus::Built,
        );
        file2.interfaces = vec![make_iface(
            "SchedulerUtils",
            "/project/src/services/schedulingUtils.ts",
        )];

        // An unrelated file that should NOT match
        let mut file3 = IrFile::new(
            PathBuf::from("/project/src/services/billing.ts"),
            Language::TypeScript,
            "hash3".to_string(),
            BuildStatus::Built,
        );
        file3.interfaces = vec![make_iface(
            "BillingService",
            "/project/src/services/billing.ts",
        )];

        let index = crate::index::build_index(
            PathBuf::from("/project"),
            vec![file1, file2, file3],
            0,
            0,
            0,
        );
        let result = match_entities(&index, &manifest);

        assert_eq!(
            result.matched.len(),
            2,
            "Glob scheduling* should match scheduling.ts and schedulingUtils.ts"
        );
        assert_eq!(result.unmatched.len(), 1, "billing.ts should not match");
        for m in &result.matched {
            assert_eq!(m.subsystem_id, "scheduling");
            assert_eq!(m.match_strategy, MatchStrategy::FilePath);
        }
    }

    // -----------------------------------------------------------------------
    // D.6: Glob filePath matches wildcard patterns
    // -----------------------------------------------------------------------

    #[test]
    fn test_glob_wildcard_matches_all_packages() {
        let json = r#"{
            "subsystems": [{
                "id": "all-packages",
                "name": "All Packages",
                "domain": "mono",
                "status": "new",
                "filePath": "/project/packages/*/src/**",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        let mut file_a = IrFile::new(
            PathBuf::from("/project/packages/auth/src/handler.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file_a.interfaces = vec![make_iface(
            "AuthHandler",
            "/project/packages/auth/src/handler.ts",
        )];

        let mut file_b = IrFile::new(
            PathBuf::from("/project/packages/billing/src/stripe.ts"),
            Language::TypeScript,
            "hash2".to_string(),
            BuildStatus::Built,
        );
        file_b.interfaces = vec![make_iface(
            "StripeClient",
            "/project/packages/billing/src/stripe.ts",
        )];

        // File outside packages/ — should NOT match
        let mut file_c = IrFile::new(
            PathBuf::from("/project/apps/web/src/index.ts"),
            Language::TypeScript,
            "hash3".to_string(),
            BuildStatus::Built,
        );
        file_c.interfaces = vec![make_iface("WebApp", "/project/apps/web/src/index.ts")];

        let index = crate::index::build_index(
            PathBuf::from("/project"),
            vec![file_a, file_b, file_c],
            0,
            0,
            0,
        );
        let result = match_entities(&index, &manifest);

        assert_eq!(
            result.matched.len(),
            2,
            "Glob packages/*/src/** should match both auth and billing"
        );
        assert_eq!(
            result.unmatched.len(),
            1,
            "apps/web/src/index.ts should not match"
        );
    }

    #[test]
    fn test_glob_does_not_regress_non_glob_paths() {
        // Non-glob paths must continue to use prefix matching
        let manifest = sample_manifest(); // uses /project/src/auth/ (no globs)
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

    // -----------------------------------------------------------------------
    // D.7: Invalid glob produces structured error
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_glob_pattern_detection() {
        assert!(is_glob_pattern(Path::new("src/services/scheduling*")));
        assert!(is_glob_pattern(Path::new("packages/*/src/")));
        assert!(is_glob_pattern(Path::new("src/[abc]/")));
        assert!(is_glob_pattern(Path::new("src/{a,b}/")));
        assert!(!is_glob_pattern(Path::new("src/services/")));
        assert!(!is_glob_pattern(Path::new("/project/src/auth/")));
    }

    #[test]
    fn test_validate_glob_pattern_valid() {
        assert!(validate_glob_pattern("packages/*/src/**").is_ok());
        assert!(validate_glob_pattern("src/services/scheduling*").is_ok());
    }

    #[test]
    fn test_validate_glob_pattern_invalid() {
        let result = validate_glob_pattern("[unclosed");
        assert!(result.is_err());
        if let Err(err) = result {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("Invalid glob pattern"),
                "Error should mention 'Invalid glob pattern', got: {err_msg}"
            );
        }
    }

    #[test]
    fn test_validate_manifest_globs_valid() {
        let json = r#"{
            "subsystems": [{
                "id": "sched",
                "name": "Scheduling",
                "domain": "services",
                "status": "new",
                "filePath": "src/services/scheduling*",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        assert!(validate_manifest_globs(&manifest).is_ok());
    }

    // -----------------------------------------------------------------------
    // Child subsystem matching: deepest path prefix wins
    // -----------------------------------------------------------------------

    #[test]
    fn test_child_subsystem_takes_precedence_over_parent() {
        let manifest = sample_manifest(); // auth has child auth-jwt

        // Entity in auth/jwt/ should match auth-jwt (child), not auth (parent)
        let mut file = IrFile::new(
            PathBuf::from("/project/src/auth/jwt/claims.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("JwtClaims", "/project/src/auth/jwt/claims.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert_eq!(result.matched.len(), 1);
        assert_eq!(
            result.matched[0].subsystem_id, "auth-jwt",
            "Entity in auth/jwt/ should match the child subsystem, not the parent"
        );
        assert_eq!(result.matched[0].match_strategy, MatchStrategy::FilePath);
    }

    #[test]
    fn test_parent_subsystem_matches_when_not_in_child_path() {
        let manifest = sample_manifest(); // auth has child auth-jwt

        // Entity in auth/ (but not auth/jwt/) should match auth (parent)
        let mut file = IrFile::new(
            PathBuf::from("/project/src/auth/session.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface("SessionManager", "/project/src/auth/session.ts")];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert_eq!(result.matched.len(), 1);
        assert_eq!(
            result.matched[0].subsystem_id, "auth",
            "Entity in auth/ (not in jwt/) should match the parent subsystem"
        );
    }

    #[test]
    fn test_multiple_entities_distributed_across_parent_and_child() {
        let manifest = sample_manifest();

        let mut file_parent = IrFile::new(
            PathBuf::from("/project/src/auth/login.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file_parent.interfaces =
            vec![make_iface("LoginService", "/project/src/auth/login.ts")];

        let mut file_child = IrFile::new(
            PathBuf::from("/project/src/auth/jwt/verify.ts"),
            Language::TypeScript,
            "hash2".to_string(),
            BuildStatus::Built,
        );
        file_child.interfaces =
            vec![make_iface("TokenVerifier", "/project/src/auth/jwt/verify.ts")];

        let mut file_unmatched = IrFile::new(
            PathBuf::from("/project/src/unknown/thing.ts"),
            Language::TypeScript,
            "hash3".to_string(),
            BuildStatus::Built,
        );
        file_unmatched.interfaces =
            vec![make_iface("UnknownThing", "/project/src/unknown/thing.ts")];

        let index = crate::index::build_index(
            PathBuf::from("/project"),
            vec![file_parent, file_child, file_unmatched],
            0,
            0,
            0,
        );
        let result = match_entities(&index, &manifest);

        assert_eq!(result.matched.len(), 2);
        assert_eq!(result.unmatched.len(), 1);

        let parent_match = result.matched.iter().find(|m| m.entity.name == "LoginService");
        let child_match = result.matched.iter().find(|m| m.entity.name == "TokenVerifier");

        assert_eq!(
            parent_match.map(|m| m.subsystem_id.as_str()),
            Some("auth")
        );
        assert_eq!(
            child_match.map(|m| m.subsystem_id.as_str()),
            Some("auth-jwt")
        );
    }

    // -----------------------------------------------------------------------
    // Glob pattern with relative paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_glob_relative_path_resolved_against_scan_root() {
        let json = r#"{
            "subsystems": [{
                "id": "services",
                "name": "Services",
                "domain": "core",
                "status": "built",
                "filePath": "src/services/**",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));

        let mut file = IrFile::new(
            PathBuf::from("/project/src/services/auth/handler.ts"),
            Language::TypeScript,
            "hash1".to_string(),
            BuildStatus::Built,
        );
        file.interfaces = vec![make_iface(
            "AuthHandler",
            "/project/src/services/auth/handler.ts",
        )];

        let index = crate::index::build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);
        let result = match_entities(&index, &manifest);

        assert_eq!(result.matched.len(), 1, "Relative glob should match when resolved against scan root");
        assert_eq!(result.matched[0].subsystem_id, "services");
    }

    #[test]
    fn test_validate_manifest_globs_invalid() {
        let json = r#"{
            "subsystems": [{
                "id": "bad",
                "name": "Bad Glob",
                "domain": "services",
                "status": "new",
                "filePath": "[unclosed",
                "interfaces": [],
                "operations": [],
                "tables": [],
                "events": [],
                "children": [],
                "dependencies": []
            }]
        }"#;
        let manifest = parse_manifest(json).unwrap_or_else(|e| panic!("Failed to parse: {e}"));
        let result = validate_manifest_globs(&manifest);
        assert!(result.is_err());
        if let Err(err) = result {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("Invalid glob pattern"),
                "Error should be structured DomainScanError, got: {err_msg}"
            );
        }
    }
}
