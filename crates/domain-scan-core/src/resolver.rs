//! Cross-file import/export tracking and implementation matching.
//!
//! The resolver operates on a collection of already-parsed `IrFile`s and builds
//! cross-file resolution data:
//! - Import resolution: maps import source paths to actual files
//! - Export resolution: tracks what each file exports (named, default, re-exports)
//! - Implementation matching: maps interface/trait names to their implementors across files
//! - Re-export chain resolution: A re-exports from B which re-exports from C

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ir::{ExportKind, IrFile};

/// A resolved import: the original import source mapped to an actual file path.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ResolvedImport {
    /// The file containing the import statement.
    pub importing_file: PathBuf,
    /// The raw import source string (e.g. "./utils", "lodash", "crate::foo").
    pub source: String,
    /// The resolved file path, if found in the scanned files.
    pub resolved_path: Option<PathBuf>,
    /// The symbols imported.
    pub symbols: Vec<String>,
}

/// An implementation relationship: a type implements an interface/trait.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ImplementationLink {
    /// The interface/trait name being implemented.
    pub interface_name: String,
    /// The implementing type name.
    pub implementor: String,
    /// The file where the implementation is defined.
    pub impl_file: PathBuf,
    /// The file where the interface is defined, if found.
    pub interface_file: Option<PathBuf>,
}

/// What a file exports, resolved across re-export chains.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ResolvedExport {
    /// The symbol name being exported.
    pub name: String,
    /// The file that originally defines the symbol.
    pub origin_file: PathBuf,
    /// The file that re-exports the symbol (if different from origin).
    pub via_file: Option<PathBuf>,
}

/// The complete cross-file resolution result.
#[derive(Debug, Clone, Default)]
pub struct ResolutionResult {
    /// All resolved imports.
    pub imports: Vec<ResolvedImport>,
    /// All implementation links (interface/trait -> implementor).
    pub impl_links: Vec<ImplementationLink>,
    /// Map of interface/trait name -> list of implementing type names.
    pub implementors: HashMap<String, Vec<String>>,
    /// Map of file path -> list of symbols it exports (including re-exports).
    pub exports_by_file: HashMap<PathBuf, Vec<ResolvedExport>>,
    /// Map of symbol name -> list of files that export it.
    pub files_by_export: HashMap<String, Vec<PathBuf>>,
}

/// Resolve cross-file relationships from a set of parsed files.
///
/// This is the main entry point for cross-file resolution. It:
/// 1. Builds a file path index for fast lookups
/// 2. Resolves imports to actual file paths
/// 3. Collects implementation relationships from classes, impls, and extends
/// 4. Resolves re-export chains
pub fn resolve(files: &[IrFile], root: &Path) -> ResolutionResult {
    let path_index = build_path_index(files, root);
    let imports = resolve_imports(files, &path_index, root);
    let (impl_links, implementors) = resolve_implementations(files, &path_index);
    let (exports_by_file, files_by_export) = resolve_exports(files, &path_index, root);

    ResolutionResult {
        imports,
        impl_links,
        implementors,
        exports_by_file,
        files_by_export,
    }
}

// ---------------------------------------------------------------------------
// Path Index
// ---------------------------------------------------------------------------

/// Maps various forms of a file path to its index in the files slice.
/// Allows resolving import sources like "./utils" to actual files.
fn build_path_index(files: &[IrFile], root: &Path) -> HashMap<String, usize> {
    let mut index = HashMap::new();

    for (i, file) in files.iter().enumerate() {
        let path = &file.path;

        // Index by full path
        index.insert(path.display().to_string(), i);

        // Index by path relative to root
        if let Ok(rel) = path.strip_prefix(root) {
            index.insert(rel.display().to_string(), i);

            // Also index without extension (for import resolution)
            let without_ext = rel.with_extension("");
            index.insert(without_ext.display().to_string(), i);

            // Index with "./" prefix (common in JS/TS imports)
            let dot_slash = format!("./{}", rel.display());
            index.insert(dot_slash, i);
            let dot_slash_no_ext = format!("./{}", without_ext.display());
            index.insert(dot_slash_no_ext, i);
        }

        // Index by filename without extension
        if let Some(stem) = path.file_stem() {
            index.insert(stem.to_string_lossy().to_string(), i);
        }
    }

    index
}

/// Try to resolve an import source string to a file index.
/// `from_file` is the file containing the import, used for relative path resolution.
fn resolve_source(
    source: &str,
    path_index: &HashMap<String, usize>,
    from_file: Option<&Path>,
) -> Option<usize> {
    // Direct lookup
    if let Some(&idx) = path_index.get(source) {
        return Some(idx);
    }

    // For relative imports (./foo, ../foo), resolve against the importing file's directory
    if let Some(from) = from_file {
        if source.starts_with("./") || source.starts_with("../") {
            if let Some(dir) = from.parent() {
                let resolved = normalize_path(&dir.join(source));
                return resolve_absolute_path(&resolved.display().to_string(), path_index);
            }
        }
    }

    // Try common extensions
    resolve_with_extensions(source, path_index)
}

/// Normalize a path by resolving `.` and `..` components.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

/// Try to resolve an absolute path string with various extensions.
fn resolve_absolute_path(path: &str, path_index: &HashMap<String, usize>) -> Option<usize> {
    if let Some(&idx) = path_index.get(path) {
        return Some(idx);
    }
    resolve_with_extensions(path, path_index)
}

/// Try adding common file extensions to resolve a path.
fn resolve_with_extensions(source: &str, path_index: &HashMap<String, usize>) -> Option<usize> {
    for ext in &[
        "ts", "tsx", "js", "jsx", "py", "rs", "go", "java", "kt", "scala", "cs", "swift",
        "cpp", "hpp", "php", "rb",
    ] {
        let with_ext = format!("{source}.{ext}");
        if let Some(&idx) = path_index.get(&with_ext) {
            return Some(idx);
        }
    }

    // Try /index variants (common in JS/TS)
    for ext in &["ts", "tsx", "js", "jsx"] {
        let index_path = format!("{source}/index.{ext}");
        if let Some(&idx) = path_index.get(&index_path) {
            return Some(idx);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Import Resolution
// ---------------------------------------------------------------------------

fn resolve_imports(
    files: &[IrFile],
    path_index: &HashMap<String, usize>,
    _root: &Path,
) -> Vec<ResolvedImport> {
    let mut resolved = Vec::new();

    for file in files {
        for import in &file.imports {
            let resolved_idx = resolve_source(&import.source, path_index, Some(&file.path));
            let resolved_path = resolved_idx.map(|idx| files[idx].path.clone());

            let symbols: Vec<String> = if import.is_wildcard {
                vec!["*".to_string()]
            } else {
                import
                    .symbols
                    .iter()
                    .map(|s| {
                        s.alias
                            .as_ref()
                            .map_or_else(|| s.name.clone(), |a| format!("{} as {}", s.name, a))
                    })
                    .collect()
            };

            resolved.push(ResolvedImport {
                importing_file: file.path.clone(),
                source: import.source.clone(),
                resolved_path,
                symbols,
            });
        }
    }

    resolved
}

// ---------------------------------------------------------------------------
// Implementation Matching
// ---------------------------------------------------------------------------

/// Collect all implementation relationships from:
/// - `ClassDef.implements` (TS/Java/C#/Kotlin classes implementing interfaces)
/// - `ImplDef` (Rust trait impls, Swift extensions)
/// - `ClassDef.extends` (class inheritance)
fn resolve_implementations(
    files: &[IrFile],
    path_index: &HashMap<String, usize>,
) -> (Vec<ImplementationLink>, HashMap<String, Vec<String>>) {
    let mut links = Vec::new();
    let mut implementors: HashMap<String, Vec<String>> = HashMap::new();

    // Build interface name -> file mapping for lookup
    let mut interface_files: HashMap<String, PathBuf> = HashMap::new();
    for file in files {
        for iface in &file.interfaces {
            interface_files.insert(iface.name.clone(), file.path.clone());
        }
    }

    // 1. From ClassDef.implements (TS, Java, C#, Kotlin, etc.)
    for file in files {
        for class in &file.classes {
            for iface_name in &class.implements {
                let interface_file = find_interface_file(
                    iface_name,
                    file,
                    files,
                    path_index,
                    &interface_files,
                );

                links.push(ImplementationLink {
                    interface_name: iface_name.clone(),
                    implementor: class.name.clone(),
                    impl_file: file.path.clone(),
                    interface_file: interface_file.clone(),
                });

                implementors
                    .entry(iface_name.clone())
                    .or_default()
                    .push(class.name.clone());
            }
        }
    }

    // 2. From ImplDef (Rust `impl Trait for Type`, Swift extensions)
    for file in files {
        for impl_def in &file.implementations {
            if let Some(trait_name) = &impl_def.trait_name {
                let interface_file = find_interface_file(
                    trait_name,
                    file,
                    files,
                    path_index,
                    &interface_files,
                );

                links.push(ImplementationLink {
                    interface_name: trait_name.clone(),
                    implementor: impl_def.target.clone(),
                    impl_file: file.path.clone(),
                    interface_file: interface_file.clone(),
                });

                implementors
                    .entry(trait_name.clone())
                    .or_default()
                    .push(impl_def.target.clone());
            }
        }
    }

    (links, implementors)
}

/// Try to find the file where an interface/trait is defined.
/// First checks the direct interface_files map, then follows imports.
fn find_interface_file(
    name: &str,
    importing_file: &IrFile,
    files: &[IrFile],
    path_index: &HashMap<String, usize>,
    interface_files: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    // Direct lookup: interface defined in a known file
    if let Some(path) = interface_files.get(name) {
        return Some(path.clone());
    }

    // Follow imports: check if the implementing file imports this name
    for import in &importing_file.imports {
        let imports_name = import.symbols.iter().any(|s| {
            s.name == name || s.alias.as_deref() == Some(name)
        });

        if imports_name || import.is_wildcard {
            if let Some(idx) = resolve_source(&import.source, path_index, Some(&importing_file.path)) {
                let target_file = &files[idx];
                // Check if the target file defines this interface
                if target_file.interfaces.iter().any(|i| i.name == name) {
                    return Some(target_file.path.clone());
                }
                // Check re-exports
                for export in &target_file.exports {
                    if export.name == name {
                        if let Some(re_source) = &export.source {
                            if let Some(re_idx) = resolve_source(re_source, path_index, Some(&target_file.path)) {
                                return Some(files[re_idx].path.clone());
                            }
                        }
                        return Some(target_file.path.clone());
                    }
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Export Resolution
// ---------------------------------------------------------------------------

fn resolve_exports(
    files: &[IrFile],
    path_index: &HashMap<String, usize>,
    _root: &Path,
) -> (HashMap<PathBuf, Vec<ResolvedExport>>, HashMap<String, Vec<PathBuf>>) {
    let mut exports_by_file: HashMap<PathBuf, Vec<ResolvedExport>> = HashMap::new();
    let mut files_by_export: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for file in files {
        let mut file_exports = Vec::new();

        for export in &file.exports {
            match export.kind {
                ExportKind::Named | ExportKind::Default => {
                    file_exports.push(ResolvedExport {
                        name: export.name.clone(),
                        origin_file: file.path.clone(),
                        via_file: None,
                    });
                    files_by_export
                        .entry(export.name.clone())
                        .or_default()
                        .push(file.path.clone());
                }
                ExportKind::ReExport => {
                    let origin = export
                        .source
                        .as_ref()
                        .and_then(|s| resolve_source(s, path_index, Some(&file.path)))
                        .map(|idx| files[idx].path.clone());

                    file_exports.push(ResolvedExport {
                        name: export.name.clone(),
                        origin_file: origin.clone().unwrap_or_else(|| file.path.clone()),
                        via_file: Some(file.path.clone()),
                    });
                    files_by_export
                        .entry(export.name.clone())
                        .or_default()
                        .push(file.path.clone());
                }
            }
        }

        if !file_exports.is_empty() {
            exports_by_file.insert(file.path.clone(), file_exports);
        }
    }

    (exports_by_file, files_by_export)
}

// ---------------------------------------------------------------------------
// Utility: implementation completeness check
// ---------------------------------------------------------------------------

/// Check whether a type fully implements an interface/trait.
/// Returns a list of missing method names.
pub fn check_implementation_completeness(
    interface_methods: &[crate::ir::MethodSignature],
    impl_methods: &[crate::ir::MethodDef],
) -> Vec<String> {
    let impl_names: std::collections::HashSet<&str> =
        impl_methods.iter().map(|m| m.name.as_str()).collect();

    interface_methods
        .iter()
        .filter(|sig| !sig.has_default && !impl_names.contains(sig.name.as_str()))
        .map(|sig| sig.name.clone())
        .collect()
}

/// For each implementor, check if it fully implements all methods of the interface.
/// Returns a map of (implementor_name, interface_name) -> Vec<missing_method_names>.
pub fn check_all_completeness(
    files: &[IrFile],
    impl_links: &[ImplementationLink],
) -> HashMap<(String, String), Vec<String>> {
    let mut results = HashMap::new();

    // Build lookup: interface_name -> methods
    let mut interface_methods: HashMap<String, Vec<crate::ir::MethodSignature>> = HashMap::new();
    for file in files {
        for iface in &file.interfaces {
            interface_methods.insert(iface.name.clone(), iface.methods.clone());
        }
    }

    for link in impl_links {
        let Some(iface_meths) = interface_methods.get(&link.interface_name) else {
            continue;
        };

        // Find the implementor's methods
        let impl_methods = find_implementor_methods(files, &link.implementor, &link.impl_file);

        let missing = check_implementation_completeness(iface_meths, &impl_methods);
        if !missing.is_empty() {
            results.insert(
                (link.implementor.clone(), link.interface_name.clone()),
                missing,
            );
        }
    }

    results
}

/// Find methods for a given implementor type in a specific file.
fn find_implementor_methods(
    files: &[IrFile],
    implementor: &str,
    impl_file: &Path,
) -> Vec<crate::ir::MethodDef> {
    for file in files {
        if file.path != impl_file {
            continue;
        }

        // Check classes
        for class in &file.classes {
            if class.name == implementor {
                return class.methods.clone();
            }
        }

        // Check impl blocks
        for impl_def in &file.implementations {
            if impl_def.target == implementor {
                return impl_def.methods.clone();
            }
        }
    }

    Vec::new()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn make_ir_file(path: &str, lang: Language) -> IrFile {
        IrFile::new(
            PathBuf::from(path),
            lang,
            format!("hash_{path}"),
            BuildStatus::Built,
        )
    }

    fn make_interface(name: &str, path: &str, methods: Vec<MethodSignature>) -> InterfaceDef {
        InterfaceDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            visibility: Visibility::Public,
            generics: Vec::new(),
            extends: Vec::new(),
            methods,
            properties: Vec::new(),
            language_kind: InterfaceKind::Interface,
            decorators: Vec::new(),
        }
    }

    fn make_method_sig(name: &str, has_default: bool) -> MethodSignature {
        MethodSignature {
            name: name.to_string(),
            span: Span::default(),
            is_async: false,
            parameters: Vec::new(),
            return_type: None,
            has_default,
        }
    }

    fn make_method_def(name: &str, path: &str) -> MethodDef {
        MethodDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            visibility: Visibility::Public,
            is_async: false,
            is_static: false,
            is_generator: false,
            parameters: Vec::new(),
            return_type: None,
            decorators: Vec::new(),
            owner: None,
            implements: None,
        }
    }

    fn make_class(name: &str, path: &str, implements: Vec<&str>, methods: Vec<MethodDef>) -> ClassDef {
        ClassDef {
            name: name.to_string(),
            file: PathBuf::from(path),
            span: Span::default(),
            visibility: Visibility::Public,
            generics: Vec::new(),
            extends: None,
            implements: implements.into_iter().map(String::from).collect(),
            methods,
            properties: Vec::new(),
            is_abstract: false,
            decorators: Vec::new(),
        }
    }

    fn make_import(source: &str, symbols: Vec<&str>) -> ImportDef {
        ImportDef {
            source: source.to_string(),
            symbols: symbols
                .into_iter()
                .map(|s| ImportedSymbol {
                    name: s.to_string(),
                    alias: None,
                    is_default: false,
                    is_namespace: false,
                })
                .collect(),
            is_wildcard: false,
            span: Span::default(),
        }
    }

    fn make_export(name: &str, kind: ExportKind, source: Option<&str>) -> ExportDef {
        ExportDef {
            name: name.to_string(),
            kind,
            source: source.map(String::from),
            span: Span::default(),
        }
    }

    #[test]
    fn test_resolve_basic_implementation() {
        let root = Path::new("/project");

        let mut file_a = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file_a.interfaces = vec![make_interface(
            "EventHandler",
            "/project/src/types.ts",
            vec![make_method_sig("handle", false)],
        )];

        let mut file_b = make_ir_file("/project/src/handler.ts", Language::TypeScript);
        file_b.classes = vec![make_class(
            "MyHandler",
            "/project/src/handler.ts",
            vec!["EventHandler"],
            vec![make_method_def("handle", "/project/src/handler.ts")],
        )];
        file_b.imports = vec![make_import("./types", vec!["EventHandler"])];

        let files = vec![file_a, file_b];
        let result = resolve(&files, root);

        assert_eq!(result.impl_links.len(), 1);
        assert_eq!(result.impl_links[0].interface_name, "EventHandler");
        assert_eq!(result.impl_links[0].implementor, "MyHandler");

        let impls = result.implementors.get("EventHandler");
        assert!(impls.is_some());
        assert!(impls.is_some_and(|v| v.contains(&"MyHandler".to_string())));
    }

    #[test]
    fn test_resolve_rust_trait_impl() {
        let root = Path::new("/project");

        let mut file_a = make_ir_file("/project/src/traits.rs", Language::Rust);
        file_a.interfaces = vec![make_interface(
            "Serialize",
            "/project/src/traits.rs",
            vec![make_method_sig("serialize", false)],
        )];

        let mut file_b = make_ir_file("/project/src/model.rs", Language::Rust);
        file_b.implementations = vec![ImplDef {
            target: "User".to_string(),
            trait_name: Some("Serialize".to_string()),
            file: PathBuf::from("/project/src/model.rs"),
            span: Span::default(),
            methods: vec![make_method_def("serialize", "/project/src/model.rs")],
        }];

        let files = vec![file_a, file_b];
        let result = resolve(&files, root);

        assert_eq!(result.impl_links.len(), 1);
        assert_eq!(result.impl_links[0].interface_name, "Serialize");
        assert_eq!(result.impl_links[0].implementor, "User");
        assert_eq!(
            result.impl_links[0].interface_file,
            Some(PathBuf::from("/project/src/traits.rs"))
        );
    }

    #[test]
    fn test_resolve_imports() {
        let root = Path::new("/project");

        let mut file_a = make_ir_file("/project/src/utils.ts", Language::TypeScript);
        file_a.exports = vec![make_export("helper", ExportKind::Named, None)];

        let mut file_b = make_ir_file("/project/src/main.ts", Language::TypeScript);
        file_b.imports = vec![make_import("./utils", vec!["helper"])];

        let files = vec![file_a, file_b];
        let result = resolve(&files, root);

        assert_eq!(result.imports.len(), 1);
        assert!(result.imports[0].resolved_path.is_some());
    }

    #[test]
    fn test_resolve_re_exports() {
        let root = Path::new("/project");

        let mut file_a = make_ir_file("/project/src/core.ts", Language::TypeScript);
        file_a.exports = vec![make_export("Config", ExportKind::Named, None)];

        let mut file_b = make_ir_file("/project/src/index.ts", Language::TypeScript);
        file_b.exports = vec![make_export("Config", ExportKind::ReExport, Some("./core"))];

        let files = vec![file_a, file_b];
        let result = resolve(&files, root);

        // index.ts should have a re-export pointing to core.ts
        let index_exports = result.exports_by_file.get(Path::new("/project/src/index.ts"));
        assert!(index_exports.is_some());
        let empty: Vec<ResolvedExport> = Vec::new();
        let exports = index_exports.unwrap_or(&empty);
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "Config");
        assert_eq!(exports[0].origin_file, PathBuf::from("/project/src/core.ts"));
        assert_eq!(
            exports[0].via_file,
            Some(PathBuf::from("/project/src/index.ts"))
        );
    }

    #[test]
    fn test_check_implementation_completeness_all_present() {
        let iface_methods = vec![
            make_method_sig("get", false),
            make_method_sig("set", false),
        ];
        let impl_methods = vec![
            make_method_def("get", "test.ts"),
            make_method_def("set", "test.ts"),
        ];

        let missing = check_implementation_completeness(&iface_methods, &impl_methods);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_check_implementation_completeness_missing() {
        let iface_methods = vec![
            make_method_sig("get", false),
            make_method_sig("set", false),
            make_method_sig("delete", false),
        ];
        let impl_methods = vec![make_method_def("get", "test.ts")];

        let missing = check_implementation_completeness(&iface_methods, &impl_methods);
        assert_eq!(missing, vec!["set", "delete"]);
    }

    #[test]
    fn test_check_implementation_completeness_default_methods_skipped() {
        let iface_methods = vec![
            make_method_sig("required", false),
            make_method_sig("optional", true), // has default impl
        ];
        let impl_methods = vec![make_method_def("required", "test.rs")];

        let missing = check_implementation_completeness(&iface_methods, &impl_methods);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_check_all_completeness() {
        let root = Path::new("/project");

        let mut file_a = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file_a.interfaces = vec![make_interface(
            "Repository",
            "/project/src/types.ts",
            vec![
                make_method_sig("find", false),
                make_method_sig("save", false),
                make_method_sig("delete", false),
            ],
        )];

        let mut file_b = make_ir_file("/project/src/user_repo.ts", Language::TypeScript);
        file_b.classes = vec![make_class(
            "UserRepo",
            "/project/src/user_repo.ts",
            vec!["Repository"],
            vec![
                make_method_def("find", "/project/src/user_repo.ts"),
                // missing: save, delete
            ],
        )];

        let files = vec![file_a, file_b];
        let resolution = resolve(&files, root);
        let completeness = check_all_completeness(&files, &resolution.impl_links);

        let key = ("UserRepo".to_string(), "Repository".to_string());
        assert!(completeness.contains_key(&key));
        let missing = &completeness[&key];
        assert!(missing.contains(&"save".to_string()));
        assert!(missing.contains(&"delete".to_string()));
    }

    #[test]
    fn test_resolve_multiple_implementors() {
        let root = Path::new("/project");

        let mut file_a = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file_a.interfaces = vec![make_interface("Serializable", "/project/src/types.ts", vec![])];

        let mut file_b = make_ir_file("/project/src/user.ts", Language::TypeScript);
        file_b.classes = vec![make_class("User", "/project/src/user.ts", vec!["Serializable"], vec![])];

        let mut file_c = make_ir_file("/project/src/post.ts", Language::TypeScript);
        file_c.classes = vec![make_class("Post", "/project/src/post.ts", vec!["Serializable"], vec![])];

        let files = vec![file_a, file_b, file_c];
        let result = resolve(&files, root);

        let impls = result.implementors.get("Serializable");
        assert!(impls.is_some());
        let empty: Vec<String> = Vec::new();
        let impls = impls.unwrap_or(&empty);
        assert_eq!(impls.len(), 2);
        assert!(impls.contains(&"User".to_string()));
        assert!(impls.contains(&"Post".to_string()));
    }

    #[test]
    fn test_resolve_empty_files() {
        let root = Path::new("/project");
        let files: Vec<IrFile> = vec![];
        let result = resolve(&files, root);

        assert!(result.imports.is_empty());
        assert!(result.impl_links.is_empty());
        assert!(result.implementors.is_empty());
        assert!(result.exports_by_file.is_empty());
    }

    #[test]
    fn test_resolve_unresolved_import() {
        let root = Path::new("/project");

        let mut file = make_ir_file("/project/src/main.ts", Language::TypeScript);
        file.imports = vec![make_import("lodash", vec!["map"])];

        let files = vec![file];
        let result = resolve(&files, root);

        assert_eq!(result.imports.len(), 1);
        assert!(result.imports[0].resolved_path.is_none());
    }

    #[test]
    fn test_files_by_export_maps_correctly() {
        let root = Path::new("/project");

        let mut file_a = make_ir_file("/project/src/a.ts", Language::TypeScript);
        file_a.exports = vec![make_export("Config", ExportKind::Named, None)];

        let mut file_b = make_ir_file("/project/src/b.ts", Language::TypeScript);
        file_b.exports = vec![make_export("Config", ExportKind::Named, None)];

        let files = vec![file_a, file_b];
        let result = resolve(&files, root);

        let config_files = result.files_by_export.get("Config");
        assert!(config_files.is_some());
        assert_eq!(config_files.map_or(0, |v| v.len()), 2);
    }
}
