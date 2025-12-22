//! Provider Template Generator
//!
//! Generates provider implementations from analyzed DNA entry type schemas.

use crate::analyzer::{DNAAnalyzer, EntryTypeSchema, FieldType};
use std::fmt::Write as FmtWrite;

/// Generates provider template code
pub struct ProviderGenerator {
    pub analyzer: DNAAnalyzer,
}

impl ProviderGenerator {
    pub fn new(analyzer: DNAAnalyzer) -> Self {
        Self { analyzer }
    }

    /// Generate complete providers.rs file content
    pub fn generate_providers_file(&self) -> String {
        let mut output = String::new();

        // Header
        writeln!(output, "{}", self.generate_header()).unwrap();

        // Imports
        writeln!(output, "{}", self.generate_imports()).unwrap();

        // Validators section
        writeln!(output, "\n{}", self.generate_validators_section()).unwrap();

        // Transformers section
        writeln!(output, "\n{}", self.generate_transformers_section()).unwrap();

        // Reference resolvers section
        writeln!(output, "\n{}", self.generate_resolvers_section()).unwrap();

        // Degradation handlers section
        writeln!(output, "\n{}", self.generate_handlers_section()).unwrap();

        // Entry type providers section
        writeln!(output, "\n{}", self.generate_providers_section()).unwrap();

        // Tests section
        writeln!(output, "\n{}", self.generate_tests_section()).unwrap();

        output
    }

    fn generate_header(&self) -> String {
        format!(
            r#"//! Auto-generated Entry Type Providers
//!
//! This file was auto-generated from DNA schema analysis.
//! Review and customize the implementations as needed.
//!
//! Generated validators, transformers, resolvers, handlers, and providers
//! for all entry types in this DNA.
"#
        )
    }

    fn generate_imports(&self) -> String {
        r#"use hc_rna::{
    Validator, Transformer, ReferenceResolver, DegradationHandler, DegradationDecision,
    EntryTypeProvider,
};
use serde_json::Value;"#.to_string()
    }

    fn generate_validators_section(&self) -> String {
        let mut output = String::from(
            "// ============================================================================\n\
             // VALIDATORS - Schema and Business Logic Validation\n\
             // ============================================================================\n"
        );

        for entry_type in self.analyzer.entry_types() {
            output.push_str("\n");
            output.push_str(&self.generate_validator(entry_type));
        }

        output
    }

    fn generate_validator(&self, schema: &EntryTypeSchema) -> String {
        let validator_name = format!("{}Validator", schema.name);
        let struct_name = &schema.name;
        let required_fields = schema.required_fields();

        let mut field_checks = String::new();

        // Required field checks
        for field in &required_fields {
            let field_type_check = match &field.field_type {
                FieldType::String => {
                    format!(
                        "let {} = data[\"{}\"].as_str().ok_or(\"{} {} is required and must be string\")?;",
                        field.name, field.name, struct_name, field.name
                    )
                }
                FieldType::U32 | FieldType::U64 => {
                    format!(
                        "let {} = data[\"{}\"].as_u64().ok_or(\"{} {} is required and must be number\")?;",
                        field.name, field.name, struct_name, field.name
                    )
                }
                FieldType::F64 => {
                    format!(
                        "let {} = data[\"{}\"].as_f64().ok_or(\"{} {} is required and must be number\")?;",
                        field.name, field.name, struct_name, field.name
                    )
                }
                FieldType::Bool => {
                    format!(
                        "let {} = data[\"{}\"].as_bool().ok_or(\"{} {} is required and must be bool\")?;",
                        field.name, field.name, struct_name, field.name
                    )
                }
                _ => format!("// TODO: Handle {} field: {}", field.field_type.to_rust_string(), field.name),
            };
            writeln!(field_checks, "        {}", field_type_check).unwrap();
        }

        // Check for empty strings in required fields
        for field in &required_fields {
            if matches!(field.field_type, FieldType::String) {
                writeln!(
                    field_checks,
                    "        if {}.is_empty() {{\n            return Err(\"{} {} cannot be empty\".to_string());\n        }}",
                    field.name, struct_name, field.name
                )
                .unwrap();
            }
        }

        // Schema version check
        writeln!(
            field_checks,
            "        let schema_version = data[\"schema_version\"].as_u64().unwrap_or(0);\n        if schema_version != 2 {{\n            return Err(format!(\"Expected schema_version 2, got {{}}\", schema_version));\n        }}"
        ).unwrap();

        format!(
            r#"/// Validates {struct_name} entries according to schema and business rules
pub struct {validator_name};

impl Validator for {validator_name} {{
    fn validate_json(&self, data: &Value) -> Result<(), String> {{
        // TODO: Customize validation rules for {struct_name}
        // Required field checks:
{field_checks}

        // TODO: Add enum validation if needed
        // TODO: Add constraint validation if needed
        // TODO: Validate related field IDs

        Ok(())
    }}
}}
"#,
            struct_name = schema.name,
            validator_name = validator_name,
            field_checks = field_checks
        )
    }

    fn generate_transformers_section(&self) -> String {
        let mut output = String::from(
            "// ============================================================================\n\
             // TRANSFORMERS - V1 to V2 Schema Transformation\n\
             // ============================================================================\n"
        );

        for entry_type in self.analyzer.entry_types() {
            output.push_str("\n");
            output.push_str(&self.generate_transformer(entry_type));
        }

        output
    }

    fn generate_transformer(&self, schema: &EntryTypeSchema) -> String {
        let transformer_name = format!("{}Transformer", schema.name);
        let struct_name = &schema.name;

        // Generate field extraction code
        let mut extractions = String::new();
        for field in &schema.fields {
            let extract = match &field.field_type {
                FieldType::String => {
                    format!(
                        "let {} = v1_data[\"{}\"].as_str().unwrap_or(\"\");",
                        field.name, field.name
                    )
                }
                FieldType::U32 | FieldType::U64 => {
                    format!(
                        "let {} = v1_data[\"{}\"].as_u64().unwrap_or(0);",
                        field.name, field.name
                    )
                }
                FieldType::F64 => {
                    format!(
                        "let {} = v1_data[\"{}\"].as_f64().unwrap_or(0.0);",
                        field.name, field.name
                    )
                }
                FieldType::Vec(inner) if matches!(**inner, FieldType::String) => {
                    format!(
                        "let {}: Vec<String> = v1_data[\"{}\"].as_array()\n            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())\n            .unwrap_or_default();",
                        field.name, field.name
                    )
                }
                _ => format!("// TODO: Handle {} field: {}", field.field_type.to_rust_string(), field.name),
            };
            writeln!(extractions, "        {}", extract).unwrap();
        }

        // Generate JSON construction
        let mut json_fields = String::new();
        for field in &schema.fields {
            writeln!(
                json_fields,
                "            \"{}\": {},",
                field.name, field.name
            )
            .unwrap();
        }
        writeln!(json_fields, "            \"schema_version\": 2,").unwrap();
        writeln!(
            json_fields,
            "            \"validation_status\": \"Migrated\""
        )
        .unwrap();

        format!(
            r#"/// Transforms {struct_name} from v1 to v2 schema
pub struct {transformer_name};

impl Transformer for {transformer_name} {{
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {{
        // Extract v1 fields
{extractions}

        // Build v2 entry with current schema
        Ok(serde_json::json!({{
{json_fields}
        }}))
    }}

    fn description(&self) -> &str {{
        "Transform {struct_name} from v1 to v2 schema"
    }}
}}
"#,
            struct_name = schema.name,
            transformer_name = transformer_name,
            json_fields = json_fields
        )
    }

    fn generate_resolvers_section(&self) -> String {
        let mut output = String::from(
            "// ============================================================================\n\
             // REFERENCE RESOLVERS - Check if Referenced Entries Exist\n\
             // ============================================================================\n"
        );

        for entry_type in self.analyzer.entry_types() {
            output.push_str("\n");
            output.push_str(&self.generate_resolver(entry_type));
        }

        output
    }

    fn generate_resolver(&self, schema: &EntryTypeSchema) -> String {
        let resolver_name = format!("{}ReferenceResolver", schema.name);
        let struct_name = &schema.name;
        let ref_fields = schema.reference_fields();

        let mut resolve_match_arms = String::new();
        if !ref_fields.is_empty() {
            for field in &ref_fields {
                writeln!(
                    resolve_match_arms,
                    "            \"{}\" => {{\n                // TODO: Check if {} exists in DHT\n                Ok(true)\n            }},",
                    field.name, field.name
                )
                .unwrap();
            }
        }

        let match_section = if !ref_fields.is_empty() {
            format!(
                "        match entry_type {{\n{}\n            _ => Ok(true)\n        }}",
                resolve_match_arms
            )
        } else {
            "        Ok(true) // No specific references to check".to_string()
        };

        format!(
            r#"/// Resolves references in {struct_name} entries
pub struct {resolver_name};

impl ReferenceResolver for {resolver_name} {{
    fn resolve_reference(&self, entry_type: &str, _id: &str) -> Result<bool, String> {{
        // TODO: Implement DHT lookups for referenced entries
        // In a real implementation, this would check if referenced entries exist
{match_section}
    }}
}}
"#,
            struct_name = schema.name,
            resolver_name = resolver_name,
            match_section = match_section
        )
    }

    fn generate_handlers_section(&self) -> String {
        let mut output = String::from(
            "// ============================================================================\n\
             // DEGRADATION HANDLERS - Handle Validation/Reference Failures\n\
             // ============================================================================\n"
        );

        for entry_type in self.analyzer.entry_types() {
            output.push_str("\n");
            output.push_str(&self.generate_handler(entry_type));
        }

        output
    }

    fn generate_handler(&self, schema: &EntryTypeSchema) -> String {
        let handler_name = format!("{}DegradationHandler", schema.name);
        let struct_name = &schema.name;

        format!(
            r#"/// Determines what to do when {struct_name} healing encounters errors
pub struct {handler_name};

impl DegradationHandler for {handler_name} {{
    fn handle_validation_failure(
        &self,
        _entry_type: &str,
        _error: &str,
        _was_migrated: bool,
    ) -> DegradationDecision {{
        // TODO: Customize degradation policy for {struct_name}
        // Options: Degrade, Fail, Accept
        DegradationDecision::Degrade
    }}

    fn handle_missing_reference(
        &self,
        _entry_type: &str,
        _ref_type: &str,
        _ref_id: &str,
    ) -> DegradationDecision {{
        // TODO: Customize reference failure policy for {struct_name}
        DegradationDecision::Degrade
    }}
}}
"#,
            struct_name = struct_name,
            handler_name = handler_name
        )
    }

    fn generate_providers_section(&self) -> String {
        let mut output = String::from(
            "// ============================================================================\n\
             // ENTRY TYPE PROVIDERS - Compose All Components\n\
             // ============================================================================\n"
        );

        for entry_type in self.analyzer.entry_types() {
            output.push_str("\n");
            output.push_str(&self.generate_provider(entry_type));
        }

        output
    }

    fn generate_provider(&self, schema: &EntryTypeSchema) -> String {
        let provider_name = format!("{}Provider", schema.name);
        let validator_name = format!("{}Validator", schema.name);
        let transformer_name = format!("{}Transformer", schema.name);
        let resolver_name = format!("{}ReferenceResolver", schema.name);
        let handler_name = format!("{}DegradationHandler", schema.name);
        let struct_name = &schema.name;
        let entry_type_name = schema.to_entry_type_name();

        format!(
            r#"/// Complete provider for {} entry type
pub struct {provider_name};

impl EntryTypeProvider for {provider_name} {{
    fn entry_type(&self) -> &str {{
        "{entry_type_name}"
    }}

    fn validator(&self) -> &dyn Validator {{
        &{validator_name}
    }}

    fn transformer(&self) -> &dyn Transformer {{
        &{transformer_name}
    }}

    fn reference_resolver(&self) -> &dyn ReferenceResolver {{
        &{resolver_name}
    }}

    fn degradation_handler(&self) -> &dyn DegradationHandler {{
        &{handler_name}
    }}

    fn create_healing_instance(&self, _id: &str, v1_data: &Value) -> Result<Vec<u8>, String> {{
        // Transform v1 to v2 JSON
        let v2_json = self.transformer().transform_v1_to_v2(v1_data)?;

        // Validate the transformed entry
        self.validator().validate_json(&v2_json)?;

        // Serialize to bytes
        Ok(serde_json::to_vec(&v2_json)
            .map_err(|e| format!("Failed to serialize healed {}: {{}}", e))?)
    }}
}}
"#,
            struct_name = schema.name,
            provider_name = provider_name,
            validator_name = validator_name,
            transformer_name = transformer_name,
            resolver_name = resolver_name,
            handler_name = handler_name,
            entry_type_name = entry_type_name
        )
    }

    fn generate_tests_section(&self) -> String {
        let mut output = String::from(
            "#[cfg(test)]\nmod tests {\n    use super::*;\n\n"
        );

        for entry_type in self.analyzer.entry_types() {
            let provider_name = format!("{}Provider", entry_type.name);
            let validator_name = format!("{}Validator", entry_type.name);
            let transformer_name = format!("{}Transformer", entry_type.name);

            writeln!(
                output,
                "    #[test]\n    fn test_{}_provider_entry_type() {{\n        let provider = {};\n        assert_eq!(provider.entry_type(), \"{}\");\n    }}\n",
                entry_type.to_entry_type_name(),
                provider_name,
                entry_type.to_entry_type_name()
            )
            .unwrap();

            writeln!(
                output,
                "    #[test]\n    fn test_{}_validator_with_valid_data() {{\n        let validator = {};\n        let valid_data = serde_json::json!({{\n            \"id\": \"test-id\",\n            \"schema_version\": 2\n        }});\n        // TODO: Add more fields to valid_data\n        assert!(validator.validate_json(&valid_data).is_ok());\n    }}\n",
                entry_type.to_entry_type_name(),
                validator_name
            )
            .unwrap();

            writeln!(
                output,
                "    #[test]\n    fn test_{}_transformer() {{\n        let transformer = {};\n        let v1_data = serde_json::json!({{ }});\n        // TODO: Add v1 data fields\n        let result = transformer.transform_v1_to_v2(&v1_data);\n        // TODO: Assert transformation worked correctly\n    }}\n",
                entry_type.to_entry_type_name(),
                transformer_name
            )
            .unwrap();
        }

        output.push_str("}\n");
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::DNAAnalyzer;

    #[test]
    fn test_generate_providers_file() {
        let source = r#"
pub struct Content {
    pub id: String,
    pub title: String,
    pub schema_version: u32,
}
"#;

        let mut analyzer = DNAAnalyzer::new();
        analyzer.parse_source(source).unwrap();

        let generator = ProviderGenerator::new(analyzer);
        let output = generator.generate_providers_file();

        assert!(output.contains("ContentValidator"));
        assert!(output.contains("ContentTransformer"));
        assert!(output.contains("ContentProvider"));
        assert!(output.contains("impl Validator for ContentValidator"));
    }
}
