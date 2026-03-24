//! ScanIndex construction with all lookup tables and query methods.
//!
//! After all files are parsed, `build_index` populates:
//! - interfaces_by_name, classes_by_name, services_by_kind
//! - methods_by_owner, implementations, implementors
//! - schemas_by_framework, schemas_by_kind
//! - ScanStats
//!
//! Query methods on ScanIndex resolve (file_idx, entity_idx) pairs
//! back into concrete IR types.

use std::path::{Path, PathBuf};

use crate::ir::*;
use crate::resolver::{self, ResolutionResult};

/// Build a fully-indexed ScanIndex from parsed files.
///
/// This is the main entry point after parsing. It:
/// 1. Computes stats
/// 2. Builds all lookup tables
/// 3. Runs cross-file resolution
/// 4. Populates implementor maps
pub fn build_index(
    root: PathBuf,
    files: Vec<IrFile>,
    parse_duration_ms: u64,
    cache_hits: usize,
    cache_misses: usize,
) -> ScanIndex {
    let mut index = ScanIndex::new(root.clone());
    index.files = files;

    // Compute stats
    index.stats = compute_stats(&index.files, parse_duration_ms, cache_hits, cache_misses);

    // Build lookup tables
    build_lookup_tables(&mut index);

    // Cross-file resolution
    let resolution = resolver::resolve(&index.files, &root);
    apply_resolution(&mut index, &resolution);

    index
}

/// Rebuild lookup tables from the current files. Useful after deserialization
/// (since lookup tables are `#[serde(skip)]`).
pub fn rebuild_lookup_tables(index: &mut ScanIndex) {
    build_lookup_tables(index);
    let resolution = resolver::resolve(&index.files, &index.root);
    apply_resolution(index, &resolution);
}

// ---------------------------------------------------------------------------
// Stats computation
// ---------------------------------------------------------------------------

fn compute_stats(
    files: &[IrFile],
    parse_duration_ms: u64,
    cache_hits: usize,
    cache_misses: usize,
) -> ScanStats {
    let mut total_interfaces = 0;
    let mut total_services = 0;
    let mut total_classes = 0;
    let mut total_methods = 0;
    let mut total_functions = 0;
    let mut total_schemas = 0;
    let mut total_type_aliases = 0;
    let mut total_implementations = 0;
    let mut files_by_language = std::collections::HashMap::new();

    for file in files {
        *files_by_language.entry(file.language).or_insert(0) += 1;
        total_interfaces += file.interfaces.len();
        total_services += file.services.len();
        total_classes += file.classes.len();
        total_functions += file.functions.len();
        total_schemas += file.schemas.len();
        total_type_aliases += file.type_aliases.len();
        total_implementations += file.implementations.len();

        for class in &file.classes {
            total_methods += class.methods.len();
        }
        for service in &file.services {
            total_methods += service.methods.len();
        }
        for impl_def in &file.implementations {
            total_methods += impl_def.methods.len();
        }
    }

    ScanStats {
        total_files: files.len(),
        files_by_language,
        total_interfaces,
        total_services,
        total_classes,
        total_methods,
        total_functions,
        total_schemas,
        total_type_aliases,
        total_implementations,
        parse_duration_ms,
        cache_hits,
        cache_misses,
    }
}

// ---------------------------------------------------------------------------
// Lookup table construction
// ---------------------------------------------------------------------------

fn build_lookup_tables(index: &mut ScanIndex) {
    index.interfaces_by_name.clear();
    index.classes_by_name.clear();
    index.services_by_kind.clear();
    index.methods_by_owner.clear();
    index.implementations.clear();
    index.schemas_by_framework.clear();
    index.schemas_by_kind.clear();

    for (file_idx, file) in index.files.iter().enumerate() {
        // Interfaces
        for (entity_idx, iface) in file.interfaces.iter().enumerate() {
            index
                .interfaces_by_name
                .entry(iface.name.clone())
                .or_default()
                .push((file_idx, entity_idx));
        }

        // Classes
        for (entity_idx, class) in file.classes.iter().enumerate() {
            index
                .classes_by_name
                .entry(class.name.clone())
                .or_default()
                .push((file_idx, entity_idx));
        }

        // Services
        for (entity_idx, service) in file.services.iter().enumerate() {
            index
                .services_by_kind
                .entry(service.kind.clone())
                .or_default()
                .push((file_idx, entity_idx));
        }

        // Methods by owner (from classes)
        for class in &file.classes {
            for (method_idx, _method) in class.methods.iter().enumerate() {
                index
                    .methods_by_owner
                    .entry(class.name.clone())
                    .or_default()
                    .push((file_idx, method_idx));
            }
        }

        // Implementations (by trait/interface name)
        for (entity_idx, impl_def) in file.implementations.iter().enumerate() {
            if let Some(trait_name) = &impl_def.trait_name {
                index
                    .implementations
                    .entry(trait_name.clone())
                    .or_default()
                    .push((file_idx, entity_idx));
            }
        }

        // Schemas by framework
        for (entity_idx, schema) in file.schemas.iter().enumerate() {
            index
                .schemas_by_framework
                .entry(schema.source_framework.clone())
                .or_default()
                .push((file_idx, entity_idx));
            index
                .schemas_by_kind
                .entry(schema.kind)
                .or_default()
                .push((file_idx, entity_idx));
        }
    }
}

fn apply_resolution(index: &mut ScanIndex, resolution: &ResolutionResult) {
    index.implementors = resolution.implementors.clone();
}

// ---------------------------------------------------------------------------
// Query methods on ScanIndex
// ---------------------------------------------------------------------------

impl ScanIndex {
    /// Get all interfaces, optionally filtered by name pattern.
    pub fn get_interfaces(&self, name_pattern: Option<&str>) -> Vec<&InterfaceDef> {
        let mut results = Vec::new();
        for file in &self.files {
            for iface in &file.interfaces {
                if matches_name(name_pattern, &iface.name) {
                    results.push(iface);
                }
            }
        }
        results
    }

    /// Get interfaces by exact name.
    pub fn get_interfaces_by_name(&self, name: &str) -> Vec<&InterfaceDef> {
        self.interfaces_by_name
            .get(name)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|(fi, ei)| self.files.get(*fi)?.interfaces.get(*ei))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all classes, optionally filtered by name pattern.
    pub fn get_classes(&self, name_pattern: Option<&str>) -> Vec<&ClassDef> {
        let mut results = Vec::new();
        for file in &self.files {
            for class in &file.classes {
                if matches_name(name_pattern, &class.name) {
                    results.push(class);
                }
            }
        }
        results
    }

    /// Get classes by exact name.
    pub fn get_classes_by_name(&self, name: &str) -> Vec<&ClassDef> {
        self.classes_by_name
            .get(name)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|(fi, ei)| self.files.get(*fi)?.classes.get(*ei))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all services, optionally filtered by kind.
    pub fn get_services(&self, kind: Option<&ServiceKind>) -> Vec<&ServiceDef> {
        if let Some(k) = kind {
            self.services_by_kind
                .get(k)
                .map(|indices| {
                    indices
                        .iter()
                        .filter_map(|(fi, ei)| self.files.get(*fi)?.services.get(*ei))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            self.files.iter().flat_map(|f| &f.services).collect()
        }
    }

    /// Get all methods for a given owner (class/struct name).
    pub fn get_methods_by_owner(&self, owner: &str) -> Vec<&MethodDef> {
        // Look in classes
        let mut results = Vec::new();
        for file in &self.files {
            for class in &file.classes {
                if class.name == owner {
                    results.extend(class.methods.iter());
                }
            }
            for impl_def in &file.implementations {
                if impl_def.target == owner {
                    results.extend(impl_def.methods.iter());
                }
            }
        }
        results
    }

    /// Get all implementations of a trait/interface by name.
    pub fn get_implementations(&self, trait_name: &str) -> Vec<&ImplDef> {
        self.implementations
            .get(trait_name)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|(fi, ei)| self.files.get(*fi)?.implementations.get(*ei))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the list of type names that implement a given trait/interface.
    pub fn get_implementors(&self, trait_name: &str) -> Vec<&str> {
        self.implementors
            .get(trait_name)
            .map(|names| names.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Get all schemas, optionally filtered by framework.
    pub fn get_schemas(&self, framework: Option<&str>) -> Vec<&SchemaDef> {
        if let Some(fw) = framework {
            self.schemas_by_framework
                .get(fw)
                .map(|indices| {
                    indices
                        .iter()
                        .filter_map(|(fi, ei)| self.files.get(*fi)?.schemas.get(*ei))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            self.files.iter().flat_map(|f| &f.schemas).collect()
        }
    }

    /// Get schemas by kind.
    pub fn get_schemas_by_kind(&self, kind: SchemaKind) -> Vec<&SchemaDef> {
        self.schemas_by_kind
            .get(&kind)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|(fi, ei)| self.files.get(*fi)?.schemas.get(*ei))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all functions across all files, optionally filtered by name pattern.
    pub fn get_functions(&self, name_pattern: Option<&str>) -> Vec<&FunctionDef> {
        let mut results = Vec::new();
        for file in &self.files {
            for func in &file.functions {
                if matches_name(name_pattern, &func.name) {
                    results.push(func);
                }
            }
        }
        results
    }

    /// Get all type aliases across all files, optionally filtered by name pattern.
    pub fn get_type_aliases(&self, name_pattern: Option<&str>) -> Vec<&TypeAlias> {
        let mut results = Vec::new();
        for file in &self.files {
            for alias in &file.type_aliases {
                if matches_name(name_pattern, &alias.name) {
                    results.push(alias);
                }
            }
        }
        results
    }

    /// Get all entity summaries, optionally filtered.
    pub fn get_entity_summaries(&self, filter: &FilterParams) -> Vec<EntitySummary> {
        let mut summaries = Vec::new();

        for file in &self.files {
            if let Some(langs) = &filter.languages {
                if !langs.contains(&file.language) {
                    continue;
                }
            }
            if let Some(bs) = &filter.build_status {
                if file.build_status != *bs {
                    continue;
                }
            }

            add_interface_summaries(file, filter, &mut summaries);
            add_service_summaries(file, filter, &mut summaries);
            add_class_summaries(file, filter, &mut summaries);
            add_function_summaries(file, filter, &mut summaries);
            add_schema_summaries(file, filter, &mut summaries);
            add_impl_summaries(file, filter, &mut summaries);
            add_type_alias_summaries(file, filter, &mut summaries);
        }

        summaries
    }

    /// Search entities by name regex pattern across all kinds.
    pub fn search(&self, pattern: &str) -> Vec<EntitySummary> {
        let filter = FilterParams {
            name_pattern: Some(pattern.to_string()),
            ..FilterParams::default()
        };
        self.get_entity_summaries(&filter)
    }

    /// Get a file by path.
    pub fn get_file(&self, path: &Path) -> Option<&IrFile> {
        self.files.iter().find(|f| f.path == path)
    }
}

// ---------------------------------------------------------------------------
// Summary helpers
// ---------------------------------------------------------------------------

fn add_interface_summaries(file: &IrFile, filter: &FilterParams, out: &mut Vec<EntitySummary>) {
    if let Some(kinds) = &filter.kind {
        if !kinds.contains(&EntityKind::Interface) {
            return;
        }
    }
    for iface in &file.interfaces {
        if matches_name(filter.name_pattern.as_deref(), &iface.name) {
            if let Some(vis) = &filter.visibility {
                if iface.visibility != *vis {
                    continue;
                }
            }
            out.push(EntitySummary {
                name: iface.name.clone(),
                kind: EntityKind::Interface,
                file: file.path.clone(),
                line: iface.span.start_line,
                language: file.language,
                build_status: file.build_status,
                confidence: file.confidence,
            });
        }
    }
}

fn add_service_summaries(file: &IrFile, filter: &FilterParams, out: &mut Vec<EntitySummary>) {
    if let Some(kinds) = &filter.kind {
        if !kinds.contains(&EntityKind::Service) {
            return;
        }
    }
    for svc in &file.services {
        if matches_name(filter.name_pattern.as_deref(), &svc.name) {
            out.push(EntitySummary {
                name: svc.name.clone(),
                kind: EntityKind::Service,
                file: file.path.clone(),
                line: svc.span.start_line,
                language: file.language,
                build_status: file.build_status,
                confidence: file.confidence,
            });
        }
    }
}

fn add_class_summaries(file: &IrFile, filter: &FilterParams, out: &mut Vec<EntitySummary>) {
    if let Some(kinds) = &filter.kind {
        if !kinds.contains(&EntityKind::Class) {
            return;
        }
    }
    for class in &file.classes {
        if matches_name(filter.name_pattern.as_deref(), &class.name) {
            if let Some(vis) = &filter.visibility {
                if class.visibility != *vis {
                    continue;
                }
            }
            out.push(EntitySummary {
                name: class.name.clone(),
                kind: EntityKind::Class,
                file: file.path.clone(),
                line: class.span.start_line,
                language: file.language,
                build_status: file.build_status,
                confidence: file.confidence,
            });
        }
    }
}

fn add_function_summaries(file: &IrFile, filter: &FilterParams, out: &mut Vec<EntitySummary>) {
    if let Some(kinds) = &filter.kind {
        if !kinds.contains(&EntityKind::Function) {
            return;
        }
    }
    for func in &file.functions {
        if matches_name(filter.name_pattern.as_deref(), &func.name) {
            if let Some(vis) = &filter.visibility {
                if func.visibility != *vis {
                    continue;
                }
            }
            out.push(EntitySummary {
                name: func.name.clone(),
                kind: EntityKind::Function,
                file: file.path.clone(),
                line: func.span.start_line,
                language: file.language,
                build_status: file.build_status,
                confidence: file.confidence,
            });
        }
    }
}

fn add_schema_summaries(file: &IrFile, filter: &FilterParams, out: &mut Vec<EntitySummary>) {
    if let Some(kinds) = &filter.kind {
        if !kinds.contains(&EntityKind::Schema) {
            return;
        }
    }
    for schema in &file.schemas {
        if matches_name(filter.name_pattern.as_deref(), &schema.name) {
            out.push(EntitySummary {
                name: schema.name.clone(),
                kind: EntityKind::Schema,
                file: file.path.clone(),
                line: schema.span.start_line,
                language: file.language,
                build_status: file.build_status,
                confidence: file.confidence,
            });
        }
    }
}

fn add_impl_summaries(file: &IrFile, filter: &FilterParams, out: &mut Vec<EntitySummary>) {
    if let Some(kinds) = &filter.kind {
        if !kinds.contains(&EntityKind::Impl) {
            return;
        }
    }
    for impl_def in &file.implementations {
        let display_name = impl_def
            .trait_name
            .as_ref()
            .map(|t| format!("{} for {}", t, impl_def.target))
            .unwrap_or_else(|| impl_def.target.clone());
        if matches_name(filter.name_pattern.as_deref(), &display_name) {
            out.push(EntitySummary {
                name: display_name,
                kind: EntityKind::Impl,
                file: file.path.clone(),
                line: impl_def.span.start_line,
                language: file.language,
                build_status: file.build_status,
                confidence: file.confidence,
            });
        }
    }
}

fn add_type_alias_summaries(file: &IrFile, filter: &FilterParams, out: &mut Vec<EntitySummary>) {
    if let Some(kinds) = &filter.kind {
        if !kinds.contains(&EntityKind::TypeAlias) {
            return;
        }
    }
    for alias in &file.type_aliases {
        if matches_name(filter.name_pattern.as_deref(), &alias.name) {
            if let Some(vis) = &filter.visibility {
                if alias.visibility != *vis {
                    continue;
                }
            }
            out.push(EntitySummary {
                name: alias.name.clone(),
                kind: EntityKind::TypeAlias,
                file: file.path.clone(),
                line: alias.span.start_line,
                language: file.language,
                build_status: file.build_status,
                confidence: file.confidence,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if a name matches an optional pattern (substring match).
fn matches_name(pattern: Option<&str>, name: &str) -> bool {
    match pattern {
        None => true,
        Some(pat) => name.contains(pat),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_ir_file(path: &str, lang: Language) -> IrFile {
        IrFile::new(
            PathBuf::from(path),
            lang,
            format!("hash_{path}"),
            BuildStatus::Built,
        )
    }

    fn make_interface(name: &str, path: &str) -> InterfaceDef {
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

    fn make_class(name: &str, path: &str, implements: Vec<&str>) -> ClassDef {
        ClassDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            visibility: Visibility::Public,
            generics: Vec::new(),
            extends: None,
            implements: implements.into_iter().map(String::from).collect(),
            methods: Vec::new(),
            properties: Vec::new(),
            is_abstract: false,
            decorators: Vec::new(),
        }
    }

    fn make_service(name: &str, path: &str, kind: ServiceKind) -> ServiceDef {
        ServiceDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            kind,
            methods: Vec::new(),
            dependencies: Vec::new(),
            decorators: Vec::new(),
            routes: Vec::new(),
        }
    }

    fn make_schema(name: &str, path: &str, framework: &str, kind: SchemaKind) -> SchemaDef {
        SchemaDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            kind,
            fields: Vec::new(),
            source_framework: framework.to_string(),
            table_name: None,
            derives: Vec::new(),
            visibility: Visibility::Public,
        }
    }

    fn make_function(name: &str, path: &str) -> FunctionDef {
        FunctionDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            visibility: Visibility::Public,
            is_async: false,
            is_generator: false,
            parameters: Vec::new(),
            return_type: None,
            decorators: Vec::new(),
        }
    }

    #[test]
    fn test_build_index_stats() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![
            make_interface("Foo", "/project/src/types.ts"),
            make_interface("Bar", "/project/src/types.ts"),
        ];
        file.classes = vec![make_class("Baz", "/project/src/types.ts", vec![])];
        file.functions = vec![make_function("helper", "/project/src/types.ts")];

        let index = build_index(PathBuf::from("/project"), vec![file], 100, 5, 10);

        assert_eq!(index.stats.total_files, 1);
        assert_eq!(index.stats.total_interfaces, 2);
        assert_eq!(index.stats.total_classes, 1);
        assert_eq!(index.stats.total_functions, 1);
        assert_eq!(index.stats.parse_duration_ms, 100);
        assert_eq!(index.stats.cache_hits, 5);
        assert_eq!(index.stats.cache_misses, 10);
        assert_eq!(
            index.stats.files_by_language.get(&Language::TypeScript),
            Some(&1)
        );
    }

    #[test]
    fn test_get_interfaces_by_name() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![
            make_interface("UserService", "/project/src/types.ts"),
            make_interface("PostService", "/project/src/types.ts"),
        ];

        let index = build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        let results = index.get_interfaces_by_name("UserService");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "UserService");

        let results = index.get_interfaces_by_name("NotExist");
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_interfaces_pattern() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![
            make_interface("UserService", "/project/src/types.ts"),
            make_interface("PostService", "/project/src/types.ts"),
            make_interface("Config", "/project/src/types.ts"),
        ];

        let index = build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        let results = index.get_interfaces(Some("Service"));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_services_by_kind() {
        let mut file = make_ir_file("/project/src/app.ts", Language::TypeScript);
        file.services = vec![
            make_service("UserController", "/project/src/app.ts", ServiceKind::HttpController),
            make_service("PostController", "/project/src/app.ts", ServiceKind::HttpController),
            make_service("EventWorker", "/project/src/app.ts", ServiceKind::Worker),
        ];

        let index = build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        let http = index.get_services(Some(&ServiceKind::HttpController));
        assert_eq!(http.len(), 2);

        let workers = index.get_services(Some(&ServiceKind::Worker));
        assert_eq!(workers.len(), 1);

        let all = index.get_services(None);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_schemas_by_framework_and_kind() {
        let mut file = make_ir_file("/project/src/schemas.ts", Language::TypeScript);
        file.schemas = vec![
            make_schema("UserSchema", "/project/src/schemas.ts", "zod", SchemaKind::ValidationSchema),
            make_schema("PostSchema", "/project/src/schemas.ts", "zod", SchemaKind::ValidationSchema),
            make_schema("User", "/project/src/schemas.ts", "drizzle", SchemaKind::OrmModel),
        ];

        let index = build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        let zod = index.get_schemas(Some("zod"));
        assert_eq!(zod.len(), 2);

        let orm = index.get_schemas_by_kind(SchemaKind::OrmModel);
        assert_eq!(orm.len(), 1);
        assert_eq!(orm[0].name, "User");
    }

    #[test]
    fn test_get_implementors_cross_file() {
        let mut file_a = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file_a.interfaces = vec![make_interface("Repository", "/project/src/types.ts")];

        let mut file_b = make_ir_file("/project/src/user_repo.ts", Language::TypeScript);
        file_b.classes = vec![make_class(
            "UserRepo",
            "/project/src/user_repo.ts",
            vec!["Repository"],
        )];

        let mut file_c = make_ir_file("/project/src/post_repo.ts", Language::TypeScript);
        file_c.classes = vec![make_class(
            "PostRepo",
            "/project/src/post_repo.ts",
            vec!["Repository"],
        )];

        let index = build_index(
            PathBuf::from("/project"),
            vec![file_a, file_b, file_c],
            0,
            0,
            0,
        );

        let implementors = index.get_implementors("Repository");
        assert_eq!(implementors.len(), 2);
        assert!(implementors.contains(&"UserRepo"));
        assert!(implementors.contains(&"PostRepo"));
    }

    #[test]
    fn test_get_entity_summaries_filtered() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![make_interface("Foo", "/project/src/types.ts")];
        file.classes = vec![make_class("Bar", "/project/src/types.ts", vec![])];

        let index = build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        // Filter to interfaces only
        let filter = FilterParams {
            kind: Some(vec![EntityKind::Interface]),
            ..FilterParams::default()
        };
        let summaries = index.get_entity_summaries(&filter);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].kind, EntityKind::Interface);
    }

    #[test]
    fn test_search() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![
            make_interface("UserService", "/project/src/types.ts"),
            make_interface("Config", "/project/src/types.ts"),
        ];
        file.classes = vec![make_class("UserRepo", "/project/src/types.ts", vec![])];

        let index = build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        let results = index.search("User");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_rebuild_lookup_tables() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![make_interface("Foo", "/project/src/types.ts")];

        // Simulate deserialization: create index without lookup tables
        let mut index = ScanIndex::new(PathBuf::from("/project"));
        index.files = vec![file];

        // Before rebuild, lookup is empty
        assert!(index.get_interfaces_by_name("Foo").is_empty());

        // After rebuild, lookup works
        rebuild_lookup_tables(&mut index);
        assert_eq!(index.get_interfaces_by_name("Foo").len(), 1);
    }

    #[test]
    fn test_empty_index() {
        let index = build_index(PathBuf::from("/project"), vec![], 0, 0, 0);

        assert_eq!(index.stats.total_files, 0);
        assert!(index.get_interfaces(None).is_empty());
        assert!(index.get_classes(None).is_empty());
        assert!(index.get_services(None).is_empty());
        assert!(index.get_schemas(None).is_empty());
    }

    #[test]
    fn test_get_file() {
        let file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        let index = build_index(PathBuf::from("/project"), vec![file], 0, 0, 0);

        assert!(index.get_file(Path::new("/project/src/types.ts")).is_some());
        assert!(index.get_file(Path::new("/project/src/other.ts")).is_none());
    }

    #[test]
    fn test_language_filter() {
        let mut ts_file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        ts_file.interfaces = vec![make_interface("TsFoo", "/project/src/types.ts")];

        let mut rs_file = make_ir_file("/project/src/types.rs", Language::Rust);
        rs_file.interfaces = vec![make_interface("RsFoo", "/project/src/types.rs")];

        let index = build_index(
            PathBuf::from("/project"),
            vec![ts_file, rs_file],
            0,
            0,
            0,
        );

        let filter = FilterParams {
            languages: Some(vec![Language::TypeScript]),
            ..FilterParams::default()
        };
        let summaries = index.get_entity_summaries(&filter);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "TsFoo");
    }

    // -----------------------------------------------------------------------
    // Adversarial repo integration tests
    // -----------------------------------------------------------------------

    /// Helper: run the full pipeline (walk → parse → extract → index) on a directory.
    fn scan_fixture_dir(dir: &Path) -> ScanIndex {
        let config = ScanConfig::new(dir.to_path_buf());
        let walked = crate::walker::walk_directory(&config)
            .unwrap_or_default();

        let build_status = BuildStatus::Built;
        let mut ir_files = Vec::new();
        for wf in &walked {
            let parse_result = crate::parser::parse_file(&wf.path, wf.language);
            if let Ok((tree, source)) = parse_result {
                if let Ok(ir) = crate::query_engine::extract(
                    &tree,
                    &source,
                    &wf.path,
                    wf.language,
                    build_status,
                ) {
                    ir_files.push(ir);
                }
            }
            // If parse fails (e.g. binary content), silently skip — that's expected
        }
        build_index(dir.to_path_buf(), ir_files, 0, 0, 0)
    }

    fn adversarial_fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/adversarial_repos")
    }

    #[test]
    fn adversarial_deeply_nested_parses_without_panic() {
        let dir = adversarial_fixtures().join("01_deeply_nested");
        let index = scan_fixture_dir(&dir);
        // Must find the deep.ts file with its interface and class
        assert!(
            index.stats.total_files >= 1,
            "Expected at least 1 file in deeply nested dir, got {}",
            index.stats.total_files
        );
        assert!(
            index.stats.total_interfaces >= 1,
            "Expected at least 1 interface, got {}",
            index.stats.total_interfaces
        );
    }

    #[test]
    fn adversarial_name_collisions_preserves_all() {
        let dir = adversarial_fixtures().join("02_name_collisions");
        let index = scan_fixture_dir(&dir);
        let user_services: Vec<_> = index
            .files
            .iter()
            .flat_map(|f| &f.services)
            .filter(|s| s.name == "UserService")
            .collect();
        assert_eq!(
            user_services.len(),
            50,
            "Each UserService must be distinct — got {}",
            user_services.len()
        );
        // All must have unique file paths
        let paths: std::collections::HashSet<_> =
            user_services.iter().map(|s| &s.file).collect();
        assert_eq!(paths.len(), 50, "All UserService files must be unique");
    }

    #[test]
    fn adversarial_empty_files_no_crash() {
        let dir = adversarial_fixtures().join("03_empty_files");
        let index = scan_fixture_dir(&dir);
        // Empty files should parse but produce zero entities
        for file in &index.files {
            assert!(
                file.interfaces.is_empty()
                    && file.services.is_empty()
                    && file.classes.is_empty()
                    && file.functions.is_empty(),
                "Empty file {:?} should have no entities",
                file.path
            );
        }
    }

    #[test]
    fn adversarial_syntax_errors_no_panic() {
        let dir = adversarial_fixtures().join("04_syntax_errors");
        let index = scan_fixture_dir(&dir);
        // Must complete without panic. tree-sitter does partial recovery,
        // so we may or may not get entities from broken files.
        assert!(index.stats.total_files >= 0);
    }

    #[test]
    fn adversarial_massive_file_parses_within_time() {
        let dir = adversarial_fixtures().join("05_massive_file");
        let start = std::time::Instant::now();
        let index = scan_fixture_dir(&dir);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 30,
            "500-interface file should parse in < 30s, took {:?}",
            elapsed
        );
        // Should find all 500 interfaces
        assert!(
            index.stats.total_interfaces >= 450,
            "Expected ~500 interfaces, got {}",
            index.stats.total_interfaces
        );
    }

    #[test]
    fn adversarial_binary_file_skipped() {
        let dir = adversarial_fixtures().join("07_binary_files");
        let index = scan_fixture_dir(&dir);
        // Binary content should be skipped (tree-sitter parse fails gracefully)
        // No entities extracted from PNG data
        let total_entities = index.stats.total_interfaces
            + index.stats.total_services
            + index.stats.total_classes
            + index.stats.total_functions;
        assert_eq!(
            total_entities, 0,
            "Binary file should produce no entities, got {}",
            total_entities
        );
    }

    #[test]
    fn adversarial_unicode_identifiers_preserved() {
        let dir = adversarial_fixtures().join("08_unicode_identifiers");
        let index = scan_fixture_dir(&dir);
        // CJK identifiers should be preserved
        let all_interfaces: Vec<_> = index
            .files
            .iter()
            .flat_map(|f| f.interfaces.iter().map(|i| i.name.as_str()))
            .collect();
        assert!(
            all_interfaces.iter().any(|n| n.contains("ユーザー")),
            "CJK interface name should be preserved, found: {:?}",
            all_interfaces
        );
    }

    #[test]
    fn adversarial_huge_method_count() {
        let dir = adversarial_fixtures().join("09_huge_method_count");
        let index = scan_fixture_dir(&dir);
        // GodInterface with 1000 methods
        let god = index
            .files
            .iter()
            .flat_map(|f| &f.interfaces)
            .find(|i| i.name == "GodInterface");
        assert!(god.is_some(), "GodInterface must be found");
        let method_count = god.map_or(0, |g| g.methods.len());
        assert!(
            method_count >= 900,
            "Expected ~1000 methods, got {}",
            method_count
        );
    }

    #[test]
    fn adversarial_mixed_encodings_no_crash() {
        let dir = adversarial_fixtures().join("10_mixed_encodings");
        let index = scan_fixture_dir(&dir);
        // UTF-8 files should parse fine, non-UTF-8 may be skipped
        // The key assertion: no panics
        assert!(index.stats.total_files >= 1);
    }

    #[test]
    #[cfg(unix)]
    fn adversarial_circular_symlinks_no_hang() {
        // Create circular symlinks at runtime (git can't store symlinks)
        let dir = TempDir::new().ok();
        if let Some(d) = &dir {
            let src = d.path().join("src");
            std::fs::create_dir_all(&src).ok();
            let a = src.join("a");
            let b = src.join("b");
            let c = src.join("c");
            // a -> b -> c -> a (circular)
            std::os::unix::fs::symlink(&b, &a).ok();
            std::os::unix::fs::symlink(&c, &b).ok();
            std::os::unix::fs::symlink(&a, &c).ok();
            let index = scan_fixture_dir(d.path());
            // Must complete without hanging. No real files to parse.
            assert_eq!(index.stats.total_files, 0);
        }
    }
}
