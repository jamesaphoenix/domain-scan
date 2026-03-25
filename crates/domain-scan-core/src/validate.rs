//! Validation rules for scan results.
//!
//! 10 built-in rules enforcing naming conventions, structural completeness,
//! and god-object detection. Each rule produces zero or more `Violation`s.

use std::collections::{HashMap, HashSet};

use crate::ir::*;

type RuleFn = fn(&ScanIndex) -> Vec<Violation>;

/// Run all 10 validation rules and return a `ValidationResult`.
pub fn validate(index: &ScanIndex) -> ValidationResult {
    let mut violations = Vec::new();

    // Run all rules
    let rules: Vec<(&str, RuleFn)> = vec![
        ("interfaces-pascal-case", rule_interfaces_pascal_case),
        ("methods-naming-convention", rule_methods_naming_convention),
        (
            "no-duplicate-interface-names",
            rule_no_duplicate_interface_names,
        ),
        ("no-duplicate-method-names", rule_no_duplicate_method_names),
        ("interfaces-have-methods", rule_interfaces_have_methods),
        ("services-have-methods", rule_services_have_methods),
        ("schema-fields-have-types", rule_schema_fields_have_types),
        ("no-god-interfaces", rule_no_god_interfaces),
        ("no-god-services", rule_no_god_services),
        (
            "interfaces-have-implementors",
            rule_interfaces_have_implementors,
        ),
    ];

    let rules_checked = rules.len();

    for (_name, rule_fn) in &rules {
        violations.extend(rule_fn(index));
    }

    let fail_count = violations
        .iter()
        .filter(|v| v.severity == ViolationSeverity::Fail)
        .count();
    let warn_count = violations
        .iter()
        .filter(|v| v.severity == ViolationSeverity::Warn)
        .count();
    let pass_count = rules_checked
        .saturating_sub(if fail_count > 0 { 1 } else { 0 })
        .saturating_sub(if warn_count > 0 { 1 } else { 0 });

    ValidationResult {
        violations,
        rules_checked,
        pass_count,
        warn_count,
        fail_count,
    }
}

/// Run only the specified rules (by name).
pub fn validate_rules(index: &ScanIndex, rule_names: &[&str]) -> ValidationResult {
    let all_rules: HashMap<&str, fn(&ScanIndex) -> Vec<Violation>> = HashMap::from([
        (
            "interfaces-pascal-case",
            rule_interfaces_pascal_case as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "methods-naming-convention",
            rule_methods_naming_convention as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "no-duplicate-interface-names",
            rule_no_duplicate_interface_names as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "no-duplicate-method-names",
            rule_no_duplicate_method_names as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "interfaces-have-methods",
            rule_interfaces_have_methods as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "services-have-methods",
            rule_services_have_methods as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "schema-fields-have-types",
            rule_schema_fields_have_types as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "no-god-interfaces",
            rule_no_god_interfaces as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "no-god-services",
            rule_no_god_services as fn(&ScanIndex) -> Vec<Violation>,
        ),
        (
            "interfaces-have-implementors",
            rule_interfaces_have_implementors as fn(&ScanIndex) -> Vec<Violation>,
        ),
    ]);

    let mut violations = Vec::new();
    let mut rules_checked: usize = 0;

    for name in rule_names {
        if let Some(rule_fn) = all_rules.get(name) {
            violations.extend(rule_fn(index));
            rules_checked += 1;
        }
    }

    let fail_count = violations
        .iter()
        .filter(|v| v.severity == ViolationSeverity::Fail)
        .count();
    let warn_count = violations
        .iter()
        .filter(|v| v.severity == ViolationSeverity::Warn)
        .count();
    let pass_count = rules_checked
        .saturating_sub(if fail_count > 0 { 1 } else { 0 })
        .saturating_sub(if warn_count > 0 { 1 } else { 0 });

    ValidationResult {
        violations,
        rules_checked,
        pass_count,
        warn_count,
        fail_count,
    }
}

// ---------------------------------------------------------------------------
// Rule 1: Interfaces are PascalCase
// ---------------------------------------------------------------------------

fn rule_interfaces_pascal_case(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for iface in &file.interfaces {
            if !is_pascal_case(&iface.name) {
                violations.push(Violation {
                    rule: "interfaces-pascal-case".to_string(),
                    severity: ViolationSeverity::Warn,
                    message: format!("Interface '{}' is not PascalCase", iface.name),
                    entity_name: Some(iface.name.clone()),
                    file: Some(file.path.clone()),
                    line: Some(iface.span.start_line),
                });
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 2: Methods follow language naming conventions
// ---------------------------------------------------------------------------

fn rule_methods_naming_convention(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        let check = method_name_checker(file.language);
        for class in &file.classes {
            for method in &class.methods {
                if !check(&method.name) {
                    violations.push(Violation {
                        rule: "methods-naming-convention".to_string(),
                        severity: ViolationSeverity::Warn,
                        message: format!(
                            "Method '{}' in class '{}' doesn't follow {} naming convention",
                            method.name, class.name, file.language
                        ),
                        entity_name: Some(method.name.clone()),
                        file: Some(file.path.clone()),
                        line: Some(method.span.start_line),
                    });
                }
            }
        }
        for iface in &file.interfaces {
            for method in &iface.methods {
                if !check(&method.name) {
                    violations.push(Violation {
                        rule: "methods-naming-convention".to_string(),
                        severity: ViolationSeverity::Warn,
                        message: format!(
                            "Method '{}' in interface '{}' doesn't follow {} naming convention",
                            method.name, iface.name, file.language
                        ),
                        entity_name: Some(method.name.clone()),
                        file: Some(file.path.clone()),
                        line: Some(method.span.start_line),
                    });
                }
            }
        }
    }
    violations
}

/// Returns a checker function appropriate for the language.
/// Rust/Go/Python/Ruby/PHP use snake_case; TS/Java/Kotlin/C#/Swift/Scala use camelCase.
fn method_name_checker(lang: Language) -> fn(&str) -> bool {
    match lang {
        Language::Rust | Language::Go | Language::Python | Language::Ruby | Language::PHP => {
            is_snake_case_or_special
        }
        _ => is_camel_case_or_special,
    }
}

// ---------------------------------------------------------------------------
// Rule 3: No duplicate interface names within a module
// ---------------------------------------------------------------------------

fn rule_no_duplicate_interface_names(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        let mut seen: HashSet<&str> = HashSet::new();
        for iface in &file.interfaces {
            if !seen.insert(&iface.name) {
                violations.push(Violation {
                    rule: "no-duplicate-interface-names".to_string(),
                    severity: ViolationSeverity::Fail,
                    message: format!("Duplicate interface name '{}' in same file", iface.name),
                    entity_name: Some(iface.name.clone()),
                    file: Some(file.path.clone()),
                    line: Some(iface.span.start_line),
                });
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 4: No duplicate method names within an interface
// ---------------------------------------------------------------------------

fn rule_no_duplicate_method_names(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for iface in &file.interfaces {
            let mut seen: HashSet<&str> = HashSet::new();
            for method in &iface.methods {
                if !seen.insert(&method.name) {
                    violations.push(Violation {
                        rule: "no-duplicate-method-names".to_string(),
                        severity: ViolationSeverity::Fail,
                        message: format!(
                            "Duplicate method '{}' in interface '{}'",
                            method.name, iface.name
                        ),
                        entity_name: Some(method.name.clone()),
                        file: Some(file.path.clone()),
                        line: Some(method.span.start_line),
                    });
                }
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 5: Every interface has at least 1 method
// ---------------------------------------------------------------------------

fn rule_interfaces_have_methods(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for iface in &file.interfaces {
            if iface.methods.is_empty() && iface.properties.is_empty() {
                violations.push(Violation {
                    rule: "interfaces-have-methods".to_string(),
                    severity: ViolationSeverity::Warn,
                    message: format!("Interface '{}' has no methods or properties", iface.name),
                    entity_name: Some(iface.name.clone()),
                    file: Some(file.path.clone()),
                    line: Some(iface.span.start_line),
                });
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 6: Every service has at least 1 route/method
// ---------------------------------------------------------------------------

fn rule_services_have_methods(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for svc in &file.services {
            if svc.methods.is_empty() && svc.routes.is_empty() {
                violations.push(Violation {
                    rule: "services-have-methods".to_string(),
                    severity: ViolationSeverity::Warn,
                    message: format!("Service '{}' has no methods or routes", svc.name),
                    entity_name: Some(svc.name.clone()),
                    file: Some(file.path.clone()),
                    line: Some(svc.span.start_line),
                });
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 7: Schema fields have type annotations
// ---------------------------------------------------------------------------

fn rule_schema_fields_have_types(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for schema in &file.schemas {
            for field in &schema.fields {
                if field.type_annotation.is_none() {
                    violations.push(Violation {
                        rule: "schema-fields-have-types".to_string(),
                        severity: ViolationSeverity::Warn,
                        message: format!(
                            "Field '{}' in schema '{}' has no type annotation",
                            field.name, schema.name
                        ),
                        entity_name: Some(schema.name.clone()),
                        file: Some(file.path.clone()),
                        line: Some(schema.span.start_line),
                    });
                }
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 8: No god-interfaces (>10 methods)
// ---------------------------------------------------------------------------

const GOD_INTERFACE_THRESHOLD: usize = 10;

fn rule_no_god_interfaces(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for iface in &file.interfaces {
            if iface.methods.len() > GOD_INTERFACE_THRESHOLD {
                violations.push(Violation {
                    rule: "no-god-interfaces".to_string(),
                    severity: ViolationSeverity::Warn,
                    message: format!(
                        "Interface '{}' has {} methods (max {})",
                        iface.name,
                        iface.methods.len(),
                        GOD_INTERFACE_THRESHOLD
                    ),
                    entity_name: Some(iface.name.clone()),
                    file: Some(file.path.clone()),
                    line: Some(iface.span.start_line),
                });
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 9: No god-services (>15 methods)
// ---------------------------------------------------------------------------

const GOD_SERVICE_THRESHOLD: usize = 15;

fn rule_no_god_services(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for svc in &file.services {
            if svc.methods.len() > GOD_SERVICE_THRESHOLD {
                violations.push(Violation {
                    rule: "no-god-services".to_string(),
                    severity: ViolationSeverity::Warn,
                    message: format!(
                        "Service '{}' has {} methods (max {})",
                        svc.name,
                        svc.methods.len(),
                        GOD_SERVICE_THRESHOLD
                    ),
                    entity_name: Some(svc.name.clone()),
                    file: Some(file.path.clone()),
                    line: Some(svc.span.start_line),
                });
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Rule 10: Every public interface has at least 1 implementor
// ---------------------------------------------------------------------------

fn rule_interfaces_have_implementors(index: &ScanIndex) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in &index.files {
        for iface in &file.interfaces {
            if iface.visibility != Visibility::Public {
                continue;
            }
            let implementors = index.get_implementors(&iface.name);
            if implementors.is_empty() {
                // Also check classes that implement this interface
                let has_class_impl = index
                    .files
                    .iter()
                    .any(|f| f.classes.iter().any(|c| c.implements.contains(&iface.name)));
                if !has_class_impl {
                    violations.push(Violation {
                        rule: "interfaces-have-implementors".to_string(),
                        severity: ViolationSeverity::Fail,
                        message: format!("Public interface '{}' has no implementors", iface.name),
                        entity_name: Some(iface.name.clone()),
                        file: Some(file.path.clone()),
                        line: Some(iface.span.start_line),
                    });
                }
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// Naming convention helpers
// ---------------------------------------------------------------------------

fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// Allow snake_case and special names (constructors, operators, etc.)
fn is_snake_case_or_special(s: &str) -> bool {
    if s.starts_with("__") || s.starts_with("test_") {
        return true;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Allow camelCase and special names (constructors, etc.)
fn is_camel_case_or_special(s: &str) -> bool {
    // Allow constructors and special methods
    if s == "constructor" || s.starts_with("$") || s.starts_with("#") {
        return true;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    fn make_method_sig(name: &str) -> MethodSignature {
        MethodSignature {
            name: name.to_string(),
            span: Span::default(),
            is_async: false,
            parameters: Vec::new(),
            return_type: None,
            has_default: false,
        }
    }

    fn build_test_index(files: Vec<IrFile>) -> ScanIndex {
        crate::index::build_index(PathBuf::from("/project"), files, 0, 0, 0)
    }

    #[test]
    fn test_rule_pascal_case_pass() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![make_interface(
            "UserService",
            "/project/src/types.ts",
            vec![make_method_sig("getUser")],
        )];

        let index = build_test_index(vec![file]);
        let result = validate(&index);
        let pascal_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "interfaces-pascal-case")
            .collect();
        assert!(pascal_violations.is_empty());
    }

    #[test]
    fn test_rule_pascal_case_fail() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![make_interface(
            "userService",
            "/project/src/types.ts",
            vec![make_method_sig("getUser")],
        )];

        let index = build_test_index(vec![file]);
        let result = validate(&index);
        let pascal_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "interfaces-pascal-case")
            .collect();
        assert_eq!(pascal_violations.len(), 1);
    }

    #[test]
    fn test_rule_no_duplicate_interfaces() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![
            make_interface("Foo", "/project/src/types.ts", vec![make_method_sig("a")]),
            make_interface("Foo", "/project/src/types.ts", vec![make_method_sig("b")]),
        ];

        let index = build_test_index(vec![file]);
        let result = validate(&index);
        let dup_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "no-duplicate-interface-names")
            .collect();
        assert_eq!(dup_violations.len(), 1);
        assert_eq!(dup_violations[0].severity, ViolationSeverity::Fail);
    }

    #[test]
    fn test_rule_no_god_interfaces() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        let methods: Vec<MethodSignature> = (0..12)
            .map(|i| make_method_sig(&format!("method{i}")))
            .collect();
        file.interfaces = vec![make_interface(
            "GodInterface",
            "/project/src/types.ts",
            methods,
        )];

        let index = build_test_index(vec![file]);
        let result = validate(&index);
        let god_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "no-god-interfaces")
            .collect();
        assert_eq!(god_violations.len(), 1);
    }

    #[test]
    fn test_rule_interfaces_have_implementors() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![make_interface(
            "Orphan",
            "/project/src/types.ts",
            vec![make_method_sig("doStuff")],
        )];
        // No classes implement it

        let index = build_test_index(vec![file]);
        let result = validate(&index);
        let impl_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "interfaces-have-implementors")
            .collect();
        assert_eq!(impl_violations.len(), 1);
    }

    #[test]
    fn test_rule_interfaces_have_implementors_pass() {
        let mut file_a = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file_a.interfaces = vec![make_interface(
            "Repository",
            "/project/src/types.ts",
            vec![make_method_sig("find")],
        )];

        let mut file_b = make_ir_file("/project/src/repo.ts", Language::TypeScript);
        file_b.classes = vec![ClassDef {
            name: "UserRepo".to_string(),
            file: PathBuf::from("/project/src/repo.ts"),
            span: Span::default(),
            visibility: Visibility::Public,
            generics: Vec::new(),
            extends: None,
            implements: vec!["Repository".to_string()],
            methods: Vec::new(),
            properties: Vec::new(),
            is_abstract: false,
            decorators: Vec::new(),
        }];

        let index = build_test_index(vec![file_a, file_b]);
        let result = validate(&index);
        let impl_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "interfaces-have-implementors")
            .collect();
        assert!(impl_violations.is_empty());
    }

    #[test]
    fn test_rule_empty_interface_warning() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![make_interface("Empty", "/project/src/types.ts", vec![])];

        let index = build_test_index(vec![file]);
        let result = validate(&index);
        let empty_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "interfaces-have-methods")
            .collect();
        assert_eq!(empty_violations.len(), 1);
    }

    #[test]
    fn test_rule_schema_fields_without_types() {
        let mut file = make_ir_file("/project/src/schemas.ts", Language::TypeScript);
        file.schemas = vec![SchemaDef {
            name: "UserSchema".to_string(),
            file: PathBuf::from("/project/src/schemas.ts"),
            span: Span::default(),
            kind: SchemaKind::ValidationSchema,
            fields: vec![
                SchemaField {
                    name: "name".to_string(),
                    type_annotation: Some("string".to_string()),
                    is_optional: false,
                    is_primary_key: false,
                    constraints: Vec::new(),
                },
                SchemaField {
                    name: "age".to_string(),
                    type_annotation: None, // missing type
                    is_optional: false,
                    is_primary_key: false,
                    constraints: Vec::new(),
                },
            ],
            source_framework: "zod".to_string(),
            table_name: None,
            derives: Vec::new(),
            visibility: Visibility::Public,
        }];

        let index = build_test_index(vec![file]);
        let result = validate(&index);
        let type_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "schema-fields-have-types")
            .collect();
        assert_eq!(type_violations.len(), 1);
    }

    #[test]
    fn test_validate_clean_codebase() {
        let mut file_a = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file_a.interfaces = vec![make_interface(
            "UserService",
            "/project/src/types.ts",
            vec![make_method_sig("getUser")],
        )];

        let mut file_b = make_ir_file("/project/src/impl.ts", Language::TypeScript);
        file_b.classes = vec![ClassDef {
            name: "UserServiceImpl".to_string(),
            file: PathBuf::from("/project/src/impl.ts"),
            span: Span::default(),
            visibility: Visibility::Public,
            generics: Vec::new(),
            extends: None,
            implements: vec!["UserService".to_string()],
            methods: vec![MethodDef {
                name: "getUser".to_string(),
                file: PathBuf::from("/project/src/impl.ts"),
                span: Span::default(),
                visibility: Visibility::Public,
                is_async: false,
                is_static: false,
                is_generator: false,
                parameters: Vec::new(),
                return_type: None,
                decorators: Vec::new(),
                owner: Some("UserServiceImpl".to_string()),
                implements: None,
            }],
            properties: Vec::new(),
            is_abstract: false,
            decorators: Vec::new(),
        }];

        let index = build_test_index(vec![file_a, file_b]);
        let result = validate(&index);
        assert_eq!(result.rules_checked, 10);
        // Only expect possible naming convention warnings, no failures
        let fails: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Fail)
            .collect();
        assert!(fails.is_empty(), "Unexpected failures: {fails:?}");
    }

    #[test]
    fn test_validate_rules_subset() {
        let mut file = make_ir_file("/project/src/types.ts", Language::TypeScript);
        file.interfaces = vec![make_interface("bad_name", "/project/src/types.ts", vec![])];

        let index = build_test_index(vec![file]);
        let result = validate_rules(&index, &["interfaces-pascal-case"]);
        assert_eq!(result.rules_checked, 1);
        assert_eq!(result.violations.len(), 1);
    }

    #[test]
    fn test_naming_helpers() {
        assert!(is_pascal_case("Foo"));
        assert!(is_pascal_case("FooBar"));
        assert!(!is_pascal_case("fooBar"));
        assert!(!is_pascal_case(""));

        assert!(is_snake_case_or_special("get_user"));
        assert!(is_snake_case_or_special("__init__"));
        assert!(is_snake_case_or_special("test_foo"));
        assert!(!is_snake_case_or_special("getUser"));

        assert!(is_camel_case_or_special("getUser"));
        assert!(is_camel_case_or_special("constructor"));
        assert!(!is_camel_case_or_special("GetUser"));
    }
}
