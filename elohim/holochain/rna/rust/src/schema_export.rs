//! JSON Schema Export for DNA Entry Types
//!
//! Generates JSON Schema from DNA entry type definitions for offline validation
//! of seed data before it hits the Holochain conductor.

use crate::analyzer::{DNAAnalyzer, EntryTypeSchema, FieldType};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// Generate JSON Schema for all entry types
pub fn generate_schemas(analyzer: &DNAAnalyzer) -> HashMap<String, Value> {
    let mut schemas = HashMap::new();

    for entry_type in analyzer.entry_types() {
        let schema = generate_entry_type_schema(entry_type);
        schemas.insert(entry_type.name.clone(), schema);
    }

    schemas
}

/// Generate JSON Schema for a single entry type
fn generate_entry_type_schema(entry_type: &EntryTypeSchema) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for field in &entry_type.fields {
        let field_schema = field_type_to_schema(&field.field_type);
        properties.insert(field.name.clone(), field_schema);

        if field.is_required {
            required.push(json!(field.name));
        }
    }

    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": format!("elohim://dna/content_store/{}", entry_type.name),
        "title": entry_type.name,
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

/// Convert FieldType to JSON Schema
fn field_type_to_schema(field_type: &FieldType) -> Value {
    match field_type {
        FieldType::String => json!({ "type": "string" }),
        FieldType::U32 => json!({ "type": "integer", "minimum": 0, "maximum": 4294967295_u64 }),
        FieldType::U64 => json!({ "type": "integer", "minimum": 0 }),
        FieldType::F64 => json!({ "type": "number" }),
        FieldType::Bool => json!({ "type": "boolean" }),
        FieldType::Vec(inner) => json!({
            "type": "array",
            "items": field_type_to_schema(inner)
        }),
        FieldType::Option(inner) => {
            // For optional fields, allow null or the actual type
            let inner_schema = field_type_to_schema(inner);
            json!({
                "oneOf": [
                    { "type": "null" },
                    inner_schema
                ]
            })
        },
        FieldType::Custom(name) => {
            // For custom types, reference them
            json!({
                "$ref": format!("#/definitions/{}", name)
            })
        }
    }
}

/// Generate a combined schema with all entry types as definitions
pub fn generate_combined_schema(analyzer: &DNAAnalyzer, entry_type_name: &str) -> Value {
    let mut definitions = Map::new();
    let mut main_schema = json!({});

    for entry_type in analyzer.entry_types() {
        if entry_type.name == entry_type_name {
            main_schema = generate_entry_type_schema(entry_type);
        }

        // Add all types as definitions for cross-references
        let def_schema = generate_entry_type_schema(entry_type);
        definitions.insert(entry_type.name.clone(), def_schema);
    }

    // Add definitions to main schema
    if let Value::Object(ref mut map) = main_schema {
        map.insert("definitions".to_string(), json!(definitions));
    }

    main_schema
}

/// Export schemas to JSON files
pub fn export_schemas_to_json(schemas: &HashMap<String, Value>) -> HashMap<String, String> {
    schemas.iter()
        .map(|(name, schema)| {
            let json_str = serde_json::to_string_pretty(schema)
                .unwrap_or_else(|_| "{}".to_string());
            (name.clone(), json_str)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_to_schema() {
        let string_schema = field_type_to_schema(&FieldType::String);
        assert_eq!(string_schema["type"], "string");

        let vec_schema = field_type_to_schema(&FieldType::Vec(Box::new(FieldType::String)));
        assert_eq!(vec_schema["type"], "array");
        assert_eq!(vec_schema["items"]["type"], "string");

        let option_schema = field_type_to_schema(&FieldType::Option(Box::new(FieldType::U32)));
        assert!(option_schema["oneOf"].is_array());
    }
}
