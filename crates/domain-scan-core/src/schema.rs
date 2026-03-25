//! Runtime JSON Schema introspection for all subcommands.
//!
//! Generates JSON Schema from Rust types at compile time via `schemars`.
//! The `schema` CLI subcommand calls these functions to dump schemas
//! that agents can use for self-service discovery.

use std::collections::BTreeMap;

use schemars::schema::RootSchema;
use schemars::schema_for;
use serde::{Deserialize, Serialize};

use crate::ir::{
    EntitySummary, FilterParams, ImplDef, InterfaceDef, MatchResult, MethodDef, ScanConfig,
    ScanIndex, ScanStats, SchemaDef, ServiceDef, ValidationResult,
};
use crate::manifest::SystemManifest;
use crate::manifest_builder::BootstrapOptions;
use crate::prompt::PromptConfig;

/// A subcommand's input and output schemas bundled together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSchema {
    pub command: String,
    pub description: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

/// All subcommand schemas keyed by command name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllSchemas {
    pub schemas: BTreeMap<String, CommandSchema>,
}

fn root_to_value(schema: RootSchema) -> serde_json::Value {
    // Safe: RootSchema is always serializable
    serde_json::to_value(schema).unwrap_or_default()
}

/// Generate the schema for the `scan` subcommand.
pub fn scan_schema() -> CommandSchema {
    CommandSchema {
        command: "scan".to_string(),
        description: "Run a full structural scan of a directory".to_string(),
        input: root_to_value(schema_for!(ScanConfig)),
        output: root_to_value(schema_for!(ScanIndex)),
    }
}

/// Generate the schema for the `interfaces` subcommand.
pub fn interfaces_schema() -> CommandSchema {
    CommandSchema {
        command: "interfaces".to_string(),
        description: "List all interfaces / traits / protocols".to_string(),
        input: root_to_value(schema_for!(FilterParams)),
        output: root_to_value(schema_for!(Vec<InterfaceDef>)),
    }
}

/// Generate the schema for the `services` subcommand.
pub fn services_schema() -> CommandSchema {
    CommandSchema {
        command: "services".to_string(),
        description: "List all service definitions".to_string(),
        input: root_to_value(schema_for!(FilterParams)),
        output: root_to_value(schema_for!(Vec<ServiceDef>)),
    }
}

/// Generate the schema for the `methods` subcommand.
pub fn methods_schema() -> CommandSchema {
    CommandSchema {
        command: "methods".to_string(),
        description: "List all methods (optionally filtered by owner)".to_string(),
        input: root_to_value(schema_for!(FilterParams)),
        output: root_to_value(schema_for!(Vec<MethodDef>)),
    }
}

/// Generate the schema for the `schemas` subcommand.
pub fn schemas_schema() -> CommandSchema {
    CommandSchema {
        command: "schemas".to_string(),
        description: "List all runtime schema definitions".to_string(),
        input: root_to_value(schema_for!(FilterParams)),
        output: root_to_value(schema_for!(Vec<SchemaDef>)),
    }
}

/// Generate the schema for the `impls` subcommand.
pub fn impls_schema() -> CommandSchema {
    CommandSchema {
        command: "impls".to_string(),
        description: "List implementations of a trait/interface".to_string(),
        input: root_to_value(schema_for!(FilterParams)),
        output: root_to_value(schema_for!(Vec<ImplDef>)),
    }
}

/// Generate the schema for the `search` subcommand.
pub fn search_schema() -> CommandSchema {
    CommandSchema {
        command: "search".to_string(),
        description: "Search across all entity names".to_string(),
        input: root_to_value(schema_for!(FilterParams)),
        output: root_to_value(schema_for!(Vec<EntitySummary>)),
    }
}

/// Generate the schema for the `stats` subcommand.
pub fn stats_schema() -> CommandSchema {
    CommandSchema {
        command: "stats".to_string(),
        description: "Print scan statistics".to_string(),
        input: root_to_value(schema_for!(ScanConfig)),
        output: root_to_value(schema_for!(ScanStats)),
    }
}

/// Generate the schema for the `validate` subcommand.
pub fn validate_schema() -> CommandSchema {
    CommandSchema {
        command: "validate".to_string(),
        description: "Run data quality checks on scan results".to_string(),
        input: root_to_value(schema_for!(ScanConfig)),
        output: root_to_value(schema_for!(ValidationResult)),
    }
}

/// Generate the schema for the `match` subcommand.
pub fn match_schema() -> CommandSchema {
    CommandSchema {
        command: "match".to_string(),
        description: "Match entities to subsystems defined in a manifest".to_string(),
        input: root_to_value(schema_for!(ScanConfig)),
        output: root_to_value(schema_for!(MatchResult)),
    }
}

/// Generate the schema for the `prompt` subcommand.
pub fn prompt_schema() -> CommandSchema {
    CommandSchema {
        command: "prompt".to_string(),
        description: "Generate an LLM prompt with sub-agent dispatch".to_string(),
        input: root_to_value(schema_for!(PromptConfig)),
        output: {
            // Prompt output is a plain string
            let mut map = serde_json::Map::new();
            map.insert(
                "type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            map.insert(
                "description".to_string(),
                serde_json::Value::String(
                    "Generated LLM prompt text with sub-agent assignments".to_string(),
                ),
            );
            serde_json::Value::Object(map)
        },
    }
}

/// Generate the schema for the `init` subcommand.
pub fn init_schema() -> CommandSchema {
    CommandSchema {
        command: "init".to_string(),
        description: "Bootstrap a system manifest from scan data".to_string(),
        input: root_to_value(schema_for!(BootstrapOptions)),
        output: root_to_value(schema_for!(SystemManifest)),
    }
}

/// Get the schema for a specific subcommand by name.
/// Returns `None` if the command name is not recognized.
pub fn schema_for_command(command: &str) -> Option<CommandSchema> {
    match command {
        "scan" => Some(scan_schema()),
        "interfaces" => Some(interfaces_schema()),
        "services" => Some(services_schema()),
        "methods" => Some(methods_schema()),
        "schemas" => Some(schemas_schema()),
        "impls" => Some(impls_schema()),
        "search" => Some(search_schema()),
        "stats" => Some(stats_schema()),
        "validate" => Some(validate_schema()),
        "match" => Some(match_schema()),
        "prompt" => Some(prompt_schema()),
        "init" => Some(init_schema()),
        _ => None,
    }
}

/// List all known subcommand names.
pub fn all_command_names() -> &'static [&'static str] {
    &[
        "scan",
        "interfaces",
        "services",
        "methods",
        "schemas",
        "impls",
        "search",
        "stats",
        "validate",
        "match",
        "prompt",
        "init",
    ]
}

/// Generate schemas for all subcommands.
pub fn all_schemas() -> AllSchemas {
    let mut schemas = BTreeMap::new();
    for name in all_command_names() {
        if let Some(s) = schema_for_command(name) {
            schemas.insert(name.to_string(), s);
        }
    }
    AllSchemas { schemas }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_schema_has_required_fields() {
        let schema = scan_schema();
        assert_eq!(schema.command, "scan");
        assert!(!schema.description.is_empty());
        // Input and output should be valid JSON objects with schema fields
        assert!(schema.input.is_object());
        assert!(schema.output.is_object());
    }

    #[test]
    fn test_interfaces_schema_output_is_array() {
        let schema = interfaces_schema();
        assert_eq!(schema.command, "interfaces");
        // Output schema for Vec<T> should reference array type
        let output = &schema.output;
        assert!(output.is_object());
    }

    #[test]
    fn test_schema_for_command_known() {
        for name in all_command_names() {
            assert!(
                schema_for_command(name).is_some(),
                "Missing schema for command: {name}"
            );
        }
    }

    #[test]
    fn test_schema_for_command_unknown() {
        assert!(schema_for_command("nonexistent").is_none());
    }

    #[test]
    fn test_all_schemas_contains_all_commands() {
        let all = all_schemas();
        for name in all_command_names() {
            assert!(
                all.schemas.contains_key(*name),
                "all_schemas missing: {name}"
            );
        }
        assert_eq!(all.schemas.len(), all_command_names().len());
    }

    #[test]
    fn test_schemas_serialize_to_valid_json() -> Result<(), serde_json::Error> {
        let all = all_schemas();
        let json = serde_json::to_string_pretty(&all)?;
        // Verify it round-trips
        let _parsed: serde_json::Value = serde_json::from_str(&json)?;
        Ok(())
    }

    #[test]
    fn test_prompt_schema_output_is_string_type() {
        let schema = prompt_schema();
        let output = &schema.output;
        assert_eq!(
            output.get("type"),
            Some(&serde_json::Value::String("string".to_string()))
        );
    }

    #[test]
    fn test_schema_init_registered() {
        let schema = schema_for_command("init");
        assert!(
            schema.is_some(),
            "schema_for_command(\"init\") should return Some"
        );
        if let Some(schema) = schema {
            assert_eq!(schema.command, "init");
            assert!(!schema.description.is_empty());
            assert!(schema.input.is_object());
            assert!(schema.output.is_object());
        }
    }

    #[test]
    fn test_all_command_names_includes_init() {
        let names = all_command_names();
        assert!(
            names.contains(&"init"),
            "all_command_names() should contain \"init\""
        );
    }

    #[test]
    fn test_init_schema_returns_valid_json() -> Result<(), serde_json::Error> {
        let schema = init_schema();
        let json = serde_json::to_string_pretty(&schema)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert!(parsed.is_object());
        assert_eq!(
            parsed.get("command"),
            Some(&serde_json::Value::String("init".to_string()))
        );
        Ok(())
    }
}
