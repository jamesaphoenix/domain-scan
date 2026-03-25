//! Field mask filtering for JSON output.
//!
//! Supports dot-notation field paths (e.g. `name`, `files.path`, `files.interfaces.name`).
//! Post-processes serialized `serde_json::Value` by pruning fields not in the mask.

use std::collections::{BTreeMap, BTreeSet};

use crate::DomainScanError;

/// A parsed field mask tree.
///
/// Represents a set of allowed field paths. Leaf nodes mean "include this field entirely".
/// Interior nodes mean "include this field but filter its children".
#[derive(Debug, Clone)]
pub struct FieldMask {
    /// If true, this node is a leaf — include everything beneath it.
    pub include_all: bool,
    /// Children: field name -> sub-mask.
    pub children: BTreeMap<String, FieldMask>,
}

impl FieldMask {
    /// Parse a comma-separated list of dot-notation field paths into a `FieldMask`.
    ///
    /// Example: `"name,methods,files.path"` →
    /// - `name` (leaf)
    /// - `methods` (leaf)
    /// - `files` → `path` (leaf)
    pub fn parse(fields: &str) -> Result<Self, DomainScanError> {
        let mut root = FieldMask {
            include_all: false,
            children: BTreeMap::new(),
        };

        for field_path in fields.split(',') {
            let field_path = field_path.trim();
            if field_path.is_empty() {
                continue;
            }
            let parts: Vec<&str> = field_path.split('.').collect();
            root.insert(&parts);
        }

        if root.children.is_empty() {
            return Err(DomainScanError::FieldMask(
                "No valid field paths provided".to_string(),
            ));
        }

        Ok(root)
    }

    /// Insert a field path into the mask tree.
    fn insert(&mut self, parts: &[&str]) {
        if parts.is_empty() {
            self.include_all = true;
            return;
        }
        let child = self
            .children
            .entry(parts[0].to_string())
            .or_insert_with(|| FieldMask {
                include_all: false,
                children: BTreeMap::new(),
            });
        child.insert(&parts[1..]);
    }

    /// Apply this field mask to a JSON value, returning only the allowed fields.
    ///
    /// - For objects: keeps only keys in the mask, recursing into children.
    /// - For arrays: applies the mask to each element.
    /// - For scalars: returns as-is (the mask matched a leaf).
    pub fn apply(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut result = serde_json::Map::new();
                for (key, child_mask) in &self.children {
                    if let Some(val) = map.get(key) {
                        if child_mask.include_all && child_mask.children.is_empty() {
                            // Leaf: include entire value
                            result.insert(key.clone(), val.clone());
                        } else if child_mask.children.is_empty() {
                            // Leaf with include_all=false but no children — this is a leaf path
                            result.insert(key.clone(), val.clone());
                        } else {
                            // Interior node: recurse
                            result.insert(key.clone(), child_mask.apply(val));
                        }
                    }
                }
                serde_json::Value::Object(result)
            }
            serde_json::Value::Array(arr) => {
                let filtered: Vec<serde_json::Value> =
                    arr.iter().map(|elem| self.apply(elem)).collect();
                serde_json::Value::Array(filtered)
            }
            // Scalar — if the mask reaches here, include as-is
            other => other.clone(),
        }
    }

    /// Collect all top-level field names referenced in this mask.
    pub fn top_level_fields(&self) -> Vec<&str> {
        self.children.keys().map(|k| k.as_str()).collect()
    }
}

/// Validate that all field paths in the mask exist in the given JSON schema.
///
/// Returns a list of invalid field names. If the list is empty, all fields are valid.
pub fn validate_fields_against_schema(mask: &FieldMask, schema: &serde_json::Value) -> Vec<String> {
    let mut invalid = Vec::new();
    let valid_fields = extract_valid_fields_from_schema(schema);
    validate_mask_fields_recursive(mask, schema, &[], &valid_fields, &mut invalid);
    invalid
}

/// Extract valid top-level field names from a JSON Schema.
pub fn extract_valid_fields_from_schema(schema: &serde_json::Value) -> BTreeSet<String> {
    let mut fields = BTreeSet::new();

    // Direct properties
    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
        for key in props.keys() {
            fields.insert(key.clone());
        }
    }

    // For array schemas, look at items
    if let Some(items) = schema.get("items") {
        if let Some(props) = items.get("properties").and_then(|v| v.as_object()) {
            for key in props.keys() {
                fields.insert(key.clone());
            }
        }
    }

    // Follow $ref or definitions (schemars puts types in "definitions")
    // For schemars output, the main type is usually in definitions referenced by $ref
    if let Some(ref_path) = schema.get("$ref").and_then(|v| v.as_str()) {
        if let Some(def_name) = ref_path.strip_prefix("#/definitions/") {
            if let Some(def) = schema.get("definitions").and_then(|d| d.get(def_name)) {
                if let Some(props) = def.get("properties").and_then(|v| v.as_object()) {
                    for key in props.keys() {
                        fields.insert(key.clone());
                    }
                }
            }
        }
    }

    // For Vec<T>, schemars generates: { "type": "array", "items": { "$ref": "#/definitions/T" } }
    if let Some(items) = schema.get("items") {
        if let Some(ref_path) = items.get("$ref").and_then(|v| v.as_str()) {
            if let Some(def_name) = ref_path.strip_prefix("#/definitions/") {
                if let Some(def) = schema.get("definitions").and_then(|d| d.get(def_name)) {
                    if let Some(props) = def.get("properties").and_then(|v| v.as_object()) {
                        for key in props.keys() {
                            fields.insert(key.clone());
                        }
                    }
                }
            }
        }
    }

    fields
}

fn validate_mask_fields_recursive(
    mask: &FieldMask,
    _schema: &serde_json::Value,
    path: &[String],
    valid_fields: &BTreeSet<String>,
    invalid: &mut Vec<String>,
) {
    for key in mask.children.keys() {
        if path.is_empty() {
            // Top-level: validate against valid_fields
            if !valid_fields.contains(key) {
                invalid.push(key.clone());
            }
        }
        // Deep validation of nested paths would require following schema refs
        // For now, we validate top-level fields which covers the main use case
    }
}

/// Apply a field mask string to a JSON value, returning the filtered JSON string.
///
/// This is the main entry point for the CLI's `--fields` flag.
pub fn apply_field_mask(
    json_value: &serde_json::Value,
    fields: &str,
) -> Result<String, DomainScanError> {
    let mask = FieldMask::parse(fields)?;
    let filtered = mask.apply(json_value);
    serde_json::to_string_pretty(&filtered).map_err(DomainScanError::from)
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_fields() {
        let mask = FieldMask::parse("name,methods").unwrap_or_else(|_| panic!("parse failed"));
        assert!(mask.children.contains_key("name"));
        assert!(mask.children.contains_key("methods"));
        assert_eq!(mask.children.len(), 2);
    }

    #[test]
    fn test_parse_dot_notation() {
        let mask = FieldMask::parse("files.path,files.language,stats")
            .unwrap_or_else(|_| panic!("parse failed"));
        assert!(mask.children.contains_key("files"));
        assert!(mask.children.contains_key("stats"));
        let files = &mask.children["files"];
        assert!(files.children.contains_key("path"));
        assert!(files.children.contains_key("language"));
    }

    #[test]
    fn test_parse_empty_returns_error() {
        let result = FieldMask::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_simple_object() {
        let mask = FieldMask::parse("name,methods").unwrap_or_else(|_| panic!("parse failed"));
        let input = serde_json::json!({
            "name": "Foo",
            "methods": ["bar", "baz"],
            "extends": [],
            "file": "test.ts"
        });
        let output = mask.apply(&input);
        let obj = output.as_object().unwrap_or_else(|| panic!("not object"));
        assert_eq!(obj.len(), 2);
        assert_eq!(obj.get("name"), Some(&serde_json::json!("Foo")));
        assert!(obj.get("methods").is_some());
        assert!(obj.get("extends").is_none());
        assert!(obj.get("file").is_none());
    }

    #[test]
    fn test_apply_to_array() {
        let mask = FieldMask::parse("name").unwrap_or_else(|_| panic!("parse failed"));
        let input = serde_json::json!([
            {"name": "A", "kind": "interface"},
            {"name": "B", "kind": "trait"}
        ]);
        let output = mask.apply(&input);
        let arr = output.as_array().unwrap_or_else(|| panic!("not array"));
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], serde_json::json!({"name": "A"}));
        assert_eq!(arr[1], serde_json::json!({"name": "B"}));
    }

    #[test]
    fn test_apply_nested_dot_notation() {
        let mask = FieldMask::parse("files.path,stats").unwrap_or_else(|_| panic!("parse failed"));
        let input = serde_json::json!({
            "files": [
                {"path": "a.ts", "language": "TypeScript", "interfaces": []},
                {"path": "b.rs", "language": "Rust", "interfaces": []}
            ],
            "stats": {"total_files": 2},
            "version": "1.0"
        });
        let output = mask.apply(&input);
        let obj = output.as_object().unwrap_or_else(|| panic!("not object"));
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("files"));
        assert!(obj.contains_key("stats"));
        assert!(!obj.contains_key("version"));

        // files should only have "path" per element
        let files = obj
            .get("files")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("no files"));
        assert_eq!(files[0], serde_json::json!({"path": "a.ts"}));
        assert_eq!(files[1], serde_json::json!({"path": "b.rs"}));
    }

    #[test]
    fn test_apply_nonexistent_field_returns_empty() {
        let mask = FieldMask::parse("nonexistent").unwrap_or_else(|_| panic!("parse failed"));
        let input = serde_json::json!({
            "name": "Foo",
            "methods": []
        });
        let output = mask.apply(&input);
        let obj = output.as_object().unwrap_or_else(|| panic!("not object"));
        assert!(obj.is_empty());
    }

    #[test]
    fn test_validate_fields_against_schema() {
        let schema = serde_json::json!({
            "properties": {
                "name": {"type": "string"},
                "methods": {"type": "array"},
                "extends": {"type": "array"}
            }
        });
        let mask = FieldMask::parse("name,bogus").unwrap_or_else(|_| panic!("parse failed"));
        let invalid = validate_fields_against_schema(&mask, &schema);
        assert_eq!(invalid, vec!["bogus"]);
    }

    #[test]
    fn test_validate_all_valid() {
        let schema = serde_json::json!({
            "properties": {
                "name": {"type": "string"},
                "methods": {"type": "array"}
            }
        });
        let mask = FieldMask::parse("name,methods").unwrap_or_else(|_| panic!("parse failed"));
        let invalid = validate_fields_against_schema(&mask, &schema);
        assert!(invalid.is_empty());
    }

    #[test]
    fn test_apply_field_mask_convenience() {
        let value = serde_json::json!([
            {"name": "X", "kind": "interface", "file": "a.ts"},
            {"name": "Y", "kind": "trait", "file": "b.rs"}
        ]);
        let result = apply_field_mask(&value, "name,kind");
        assert!(result.is_ok());
        let parsed: serde_json::Value =
            serde_json::from_str(&result.unwrap_or_default()).unwrap_or_default();
        let arr = parsed.as_array().unwrap_or_else(|| panic!("not array"));
        assert_eq!(
            arr[0],
            serde_json::json!({"kind": "interface", "name": "X"})
        );
    }

    #[test]
    fn test_whitespace_in_fields_is_trimmed() {
        let mask = FieldMask::parse(" name , methods ").unwrap_or_else(|_| panic!("parse failed"));
        assert!(mask.children.contains_key("name"));
        assert!(mask.children.contains_key("methods"));
    }

    #[test]
    fn test_deeply_nested_dot_notation() {
        let mask = FieldMask::parse("a.b.c").unwrap_or_else(|_| panic!("parse failed"));
        let input = serde_json::json!({
            "a": {
                "b": {
                    "c": "deep",
                    "d": "hidden"
                },
                "e": "hidden"
            },
            "f": "hidden"
        });
        let output = mask.apply(&input);
        assert_eq!(
            output,
            serde_json::json!({
                "a": {
                    "b": {
                        "c": "deep"
                    }
                }
            })
        );
    }
}
