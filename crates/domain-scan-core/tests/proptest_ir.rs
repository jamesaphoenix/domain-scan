//! Property-based tests: IR roundtrip serialization.
//! Verifies that IrFile and all entity types survive JSON serialization/deserialization.
//! NOTE: These test IR roundtrip, NOT source code generation (per spec).

use proptest::prelude::*;
use std::path::PathBuf;

use domain_scan_core::ir::*;

// ---------------------------------------------------------------------------
// Strategies for generating IR types
// ---------------------------------------------------------------------------

fn arb_visibility() -> impl Strategy<Value = Visibility> {
    prop_oneof![
        Just(Visibility::Public),
        Just(Visibility::Private),
        Just(Visibility::Protected),
        Just(Visibility::Internal),
        Just(Visibility::Crate),
        Just(Visibility::Unknown),
    ]
}

fn arb_build_status() -> impl Strategy<Value = BuildStatus> {
    prop_oneof![
        Just(BuildStatus::Built),
        Just(BuildStatus::Unbuilt),
        Just(BuildStatus::Error),
        Just(BuildStatus::Rebuild),
    ]
}

fn arb_language() -> impl Strategy<Value = Language> {
    prop_oneof![
        Just(Language::TypeScript),
        Just(Language::Python),
        Just(Language::Rust),
        Just(Language::Go),
        Just(Language::Java),
        Just(Language::Kotlin),
        Just(Language::CSharp),
        Just(Language::Swift),
        Just(Language::PHP),
        Just(Language::Ruby),
        Just(Language::Scala),
        Just(Language::Cpp),
    ]
}

fn arb_span() -> impl Strategy<Value = Span> {
    (0u32..1000, 0u32..200, 0u32..1000, 0u32..200, 0usize..100000).prop_map(
        |(start_line, start_col, end_line, end_col, start_byte)| Span {
            start_line,
            start_col,
            end_line: start_line + end_line,
            end_col,
            byte_range: (start_byte, start_byte + 100),
        },
    )
}

fn arb_parameter() -> impl Strategy<Value = Parameter> {
    (
        "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        proptest::option::of("[a-zA-Z][a-zA-Z0-9<>\\[\\], ]{0,30}"),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(name, type_annotation, is_optional, has_default, is_rest)| Parameter {
                name,
                type_annotation,
                is_optional,
                has_default,
                is_rest,
            },
        )
}

fn arb_method_signature() -> impl Strategy<Value = MethodSignature> {
    (
        "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        arb_span(),
        any::<bool>(),
        proptest::collection::vec(arb_parameter(), 0..4),
        proptest::option::of("[a-zA-Z][a-zA-Z0-9<>]{0,20}"),
        any::<bool>(),
    )
        .prop_map(
            |(name, span, is_async, parameters, return_type, has_default)| MethodSignature {
                name,
                span,
                is_async,
                parameters,
                return_type,
                has_default,
            },
        )
}

fn arb_property_def() -> impl Strategy<Value = PropertyDef> {
    (
        "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        proptest::option::of("[a-zA-Z][a-zA-Z0-9<>]{0,20}"),
        any::<bool>(),
        any::<bool>(),
        arb_visibility(),
    )
        .prop_map(
            |(name, type_annotation, is_optional, is_readonly, visibility)| PropertyDef {
                name,
                type_annotation,
                is_optional,
                is_readonly,
                visibility,
            },
        )
}

fn arb_interface_kind() -> impl Strategy<Value = InterfaceKind> {
    prop_oneof![
        Just(InterfaceKind::Interface),
        Just(InterfaceKind::Trait),
        Just(InterfaceKind::Protocol),
        Just(InterfaceKind::AbstractClass),
        Just(InterfaceKind::PureVirtual),
        Just(InterfaceKind::Module),
    ]
}

fn arb_interface_def() -> impl Strategy<Value = InterfaceDef> {
    (
        "[A-Z][a-zA-Z0-9]{0,20}",
        arb_span(),
        arb_visibility(),
        proptest::collection::vec("[A-Z][a-zA-Z0-9]{0,10}", 0..3),
        proptest::collection::vec("[A-Z][a-zA-Z0-9]{0,10}", 0..3),
        proptest::collection::vec(arb_method_signature(), 0..4),
        proptest::collection::vec(arb_property_def(), 0..4),
        arb_interface_kind(),
        proptest::collection::vec("[A-Z][a-zA-Z0-9]{0,10}", 0..2),
    )
        .prop_map(
            |(name, span, visibility, generics, extends, methods, properties, kind, decorators)| {
                InterfaceDef {
                    name,
                    file: PathBuf::from("test.ts"),
                    span,
                    visibility,
                    generics,
                    extends,
                    methods,
                    properties,
                    language_kind: kind,
                    decorators,
                }
            },
        )
}

fn arb_schema_kind() -> impl Strategy<Value = SchemaKind> {
    prop_oneof![
        Just(SchemaKind::ValidationSchema),
        Just(SchemaKind::OrmModel),
        Just(SchemaKind::DataTransfer),
        Just(SchemaKind::DomainEvent),
    ]
}

fn arb_schema_field() -> impl Strategy<Value = SchemaField> {
    (
        "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        proptest::option::of("[a-zA-Z][a-zA-Z0-9]{0,20}"),
        any::<bool>(),
        any::<bool>(),
        proptest::collection::vec("[a-zA-Z]{3,10}", 0..3),
    )
        .prop_map(
            |(name, type_annotation, is_optional, is_primary_key, constraints)| SchemaField {
                name,
                type_annotation,
                is_optional,
                is_primary_key,
                constraints,
            },
        )
}

fn arb_ir_file() -> impl Strategy<Value = IrFile> {
    (
        arb_language(),
        arb_build_status(),
        proptest::collection::vec(arb_interface_def(), 0..3),
    )
        .prop_map(|(language, build_status, interfaces)| {
            let mut ir = IrFile::new(
                PathBuf::from("test.ts"),
                language,
                "hash123".to_string(),
                build_status,
            );
            ir.interfaces = interfaces;
            ir
        })
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn irfile_serde_roundtrip(ir in arb_ir_file()) {
        let json = serde_json::to_string(&ir).unwrap();
        let deserialized: IrFile = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(ir, deserialized);
    }

    #[test]
    fn interface_def_serde_roundtrip(iface in arb_interface_def()) {
        let json = serde_json::to_string(&iface).unwrap();
        let deserialized: InterfaceDef = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(iface, deserialized);
    }

    #[test]
    fn parameter_serde_roundtrip(param in arb_parameter()) {
        let json = serde_json::to_string(&param).unwrap();
        let deserialized: Parameter = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(param, deserialized);
    }

    #[test]
    fn method_signature_serde_roundtrip(method in arb_method_signature()) {
        let json = serde_json::to_string(&method).unwrap();
        let deserialized: MethodSignature = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(method, deserialized);
    }

    #[test]
    fn schema_field_serde_roundtrip(field in arb_schema_field()) {
        let json = serde_json::to_string(&field).unwrap();
        let deserialized: SchemaField = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(field, deserialized);
    }

    #[test]
    fn build_status_confidence_is_deterministic(status in arb_build_status()) {
        let c1 = status.confidence();
        let c2 = status.confidence();
        prop_assert_eq!(c1, c2);
    }

    #[test]
    fn entity_summary_serde_roundtrip(
        name in "[A-Z][a-zA-Z0-9]{0,20}",
        language in arb_language(),
        build_status in arb_build_status(),
    ) {
        let summary = EntitySummary {
            name,
            kind: EntityKind::Interface,
            file: PathBuf::from("test.ts"),
            line: 42,
            language,
            build_status,
            confidence: build_status.confidence(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: EntitySummary = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(summary, deserialized);
    }

    #[test]
    fn schema_kind_serde_roundtrip(kind in arb_schema_kind()) {
        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: SchemaKind = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(kind, deserialized);
    }

    #[test]
    fn scan_config_serde_roundtrip(
        languages in proptest::collection::vec(arb_language(), 0..4),
        cache_enabled in any::<bool>(),
    ) {
        let mut config = ScanConfig::new(PathBuf::from("/tmp/project"));
        config.languages = languages;
        config.cache_enabled = cache_enabled;
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ScanConfig = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(config, deserialized);
    }

    // -----------------------------------------------------------------------
    // ScanIndex invariant tests
    // -----------------------------------------------------------------------

    #[test]
    fn scan_index_serde_roundtrip(
        files in proptest::collection::vec(arb_ir_file(), 0..5),
    ) {
        let mut index = ScanIndex::new(PathBuf::from("/tmp/project"));
        index.files = files;
        let json = serde_json::to_string(&index).unwrap();
        let deserialized: ScanIndex = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(index, deserialized);
    }

    #[test]
    fn scan_index_confidence_matches_build_status(
        build_status in arb_build_status(),
        language in arb_language(),
    ) {
        let ir = IrFile::new(
            PathBuf::from("test.rs"),
            language,
            "hash".to_string(),
            build_status,
        );
        prop_assert_eq!(ir.confidence, build_status.confidence());
    }

    #[test]
    fn ir_file_new_always_empty_collections(
        language in arb_language(),
        build_status in arb_build_status(),
    ) {
        let ir = IrFile::new(
            PathBuf::from("test.rs"),
            language,
            "hash".to_string(),
            build_status,
        );
        prop_assert!(ir.interfaces.is_empty());
        prop_assert!(ir.services.is_empty());
        prop_assert!(ir.classes.is_empty());
        prop_assert!(ir.functions.is_empty());
        prop_assert!(ir.type_aliases.is_empty());
        prop_assert!(ir.imports.is_empty());
        prop_assert!(ir.exports.is_empty());
        prop_assert!(ir.implementations.is_empty());
        prop_assert!(ir.schemas.is_empty());
    }
}
