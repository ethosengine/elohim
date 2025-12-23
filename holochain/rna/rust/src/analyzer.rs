//! DNA Schema Analyzer
//!
//! Analyzes Rust DNA structures to extract entry type information needed for
//! provider template generation.

use std::collections::HashMap;
use regex::Regex;

/// Represents a field in an entry type
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub is_required: bool,
    pub is_reference: bool,
}

/// Different types of fields
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    U32,
    U64,
    F64,
    Bool,
    Vec(Box<FieldType>),
    Option(Box<FieldType>),
    Custom(String),
}

impl FieldType {
    pub fn to_rust_string(&self) -> String {
        match self {
            FieldType::String => "String".to_string(),
            FieldType::U32 => "u32".to_string(),
            FieldType::U64 => "u64".to_string(),
            FieldType::F64 => "f64".to_string(),
            FieldType::Bool => "bool".to_string(),
            FieldType::Vec(inner) => format!("Vec<{}>", inner.to_rust_string()),
            FieldType::Option(inner) => format!("Option<{}>", inner.to_rust_string()),
            FieldType::Custom(name) => name.clone(),
        }
    }

    pub fn is_string_like(&self) -> bool {
        matches!(self, FieldType::String | FieldType::Custom(_))
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, FieldType::Option(_))
    }
}

/// Represents an enum's possible values
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

/// Represents an entry type in the DNA
#[derive(Debug, Clone)]
pub struct EntryTypeSchema {
    pub name: String,
    pub fields: Vec<Field>,
    pub is_public: bool,
}

impl EntryTypeSchema {
    pub fn to_entry_type_name(&self) -> String {
        let mut name = self.name.clone();
        if name.ends_with("Entry") {
            name.truncate(name.len() - 5);
        }
        // Convert CamelCase to snake_case
        let mut result = String::new();
        for (i, c) in name.chars().enumerate() {
            if i > 0 && c.is_uppercase() {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
        result
    }

    pub fn required_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.is_required).collect()
    }

    pub fn optional_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| !f.is_required).collect()
    }

    pub fn reference_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.is_reference).collect()
    }
}

/// Analyzes Rust code to extract DNA structure information
pub struct DNAAnalyzer {
    entry_types: Vec<EntryTypeSchema>,
    enums: HashMap<String, EnumDef>,
}

impl DNAAnalyzer {
    pub fn new() -> Self {
        Self {
            entry_types: Vec::new(),
            enums: HashMap::new(),
        }
    }

    /// Parse a Rust source file and extract entry type definitions
    pub fn parse_source(&mut self, source: &str) -> Result<(), String> {
        // Split into logical sections
        let lines: Vec<&str> = source.lines().collect();

        // First pass: extract enum definitions
        self.extract_enums(&lines)?;

        // Second pass: extract struct definitions
        self.extract_structs(&lines)?;

        Ok(())
    }

    /// Extract enum definitions from source
    fn extract_enums(&mut self, lines: &[&str]) -> Result<(), String> {
        let enum_regex = Regex::new(r"pub\s+enum\s+(\w+)\s*\{")
            .map_err(|e| format!("Regex error: {}", e))?;

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = enum_regex.captures(line) {
                let enum_name = caps.get(1).unwrap().as_str().to_string();

                // Find closing brace
                let mut variants = Vec::new();
                let mut j = i + 1;
                while j < lines.len() {
                    let variant_line = lines[j];

                    if variant_line.trim() == "}" {
                        break;
                    }

                    // Extract variant names
                    if let Some(variant) = extract_enum_variant(variant_line) {
                        variants.push(variant);
                    }

                    j += 1;
                }

                self.enums.insert(enum_name.clone(), EnumDef {
                    name: enum_name,
                    variants,
                });
            }
        }

        Ok(())
    }

    /// Extract struct definitions from source
    fn extract_structs(&mut self, lines: &[&str]) -> Result<(), String> {
        let struct_regex = Regex::new(r"pub\s+struct\s+(\w+)\s*\{")
            .map_err(|e| format!("Regex error: {}", e))?;

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = struct_regex.captures(line) {
                let struct_name = caps.get(1).unwrap().as_str().to_string();

                // Find struct fields until closing brace
                let mut fields = Vec::new();
                let mut j = i + 1;
                while j < lines.len() {
                    let field_line = lines[j];

                    if field_line.trim() == "}" {
                        break;
                    }

                    // Parse field definition
                    if let Some(field) = self.parse_field(field_line) {
                        fields.push(field);
                    }

                    j += 1;
                }

                // Determine if this is likely an entry type
                // (has an id field, is public)
                if self.is_likely_entry_type(&struct_name, &fields) {
                    self.entry_types.push(EntryTypeSchema {
                        name: struct_name,
                        fields,
                        is_public: true,
                    });
                }
            }
        }

        Ok(())
    }

    /// Parse a single field definition
    fn parse_field(&self, line: &str) -> Option<Field> {
        // Match: pub name: Type,
        let field_regex = Regex::new(r"pub\s+(\w+)\s*:\s*(.*),?\s*$").ok()?;

        if let Some(caps) = field_regex.captures(line.trim()) {
            let name = caps.get(1)?.as_str().to_string();
            let type_str = caps.get(2)?.as_str().trim().to_string();

            let field_type = self.parse_type(&type_str);
            let is_required = !matches!(field_type, FieldType::Option(_));
            let is_reference = self.is_reference_field(&name, &type_str);

            Some(Field {
                name,
                field_type,
                is_required,
                is_reference,
            })
        } else {
            None
        }
    }

    /// Parse a type string into FieldType
    fn parse_type(&self, type_str: &str) -> FieldType {
        let trimmed = type_str.trim();

        match trimmed {
            "String" => FieldType::String,
            "u32" => FieldType::U32,
            "u64" => FieldType::U64,
            "f64" => FieldType::F64,
            "bool" => FieldType::Bool,
            _ => {
                // Check for Vec<T>
                if trimmed.starts_with("Vec<") && trimmed.ends_with(">") {
                    let inner_type = &trimmed[4..trimmed.len() - 1];
                    FieldType::Vec(Box::new(self.parse_type(inner_type)))
                }
                // Check for Option<T>
                else if trimmed.starts_with("Option<") && trimmed.ends_with(">") {
                    let inner_type = &trimmed[7..trimmed.len() - 1];
                    FieldType::Option(Box::new(self.parse_type(inner_type)))
                } else {
                    // Custom type
                    FieldType::Custom(trimmed.to_string())
                }
            }
        }
    }

    /// Check if a field is likely a reference to another entry
    fn is_reference_field(&self, name: &str, type_str: &str) -> bool {
        // Heuristics:
        // - Ends with _id, _ids, _hash
        // - Contains "id" in the name and is a String
        let name_lower = name.to_lowercase();
        let is_id_field = name_lower.ends_with("_id") || name_lower.ends_with("_ids") ||
                          name_lower.ends_with("_hash") || name == "id";

        is_id_field && (type_str.contains("String") || type_str.contains("Vec"))
    }

    /// Determine if a struct is likely an entry type
    fn is_likely_entry_type(&self, _name: &str, fields: &[Field]) -> bool {
        // Heuristics:
        // - Has an 'id' field
        // - Is public
        // - Has reasonable number of fields (2-100)
        let has_id = fields.iter().any(|f| f.name == "id");
        let reasonable_field_count = fields.len() > 1 && fields.len() < 100;

        has_id && reasonable_field_count
    }

    /// Get all extracted entry types
    pub fn entry_types(&self) -> &[EntryTypeSchema] {
        &self.entry_types
    }

    /// Get all extracted enums
    pub fn enums(&self) -> &HashMap<String, EnumDef> {
        &self.enums
    }

    /// Get enum for a field type (if it's an enum)
    pub fn get_enum_for_type(&self, type_name: &str) -> Option<&EnumDef> {
        self.enums.get(type_name)
    }
}

impl Default for DNAAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract enum variant name from a line
fn extract_enum_variant(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Handle simple variant: VariantName,
    if !trimmed.is_empty() && !trimmed.starts_with("//") {
        let variant = trimmed.trim_end_matches(',').trim_end_matches('(').trim();

        if !variant.is_empty() && variant.chars().next().unwrap().is_uppercase() {
            return Some(variant.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_string_type() {
        let analyzer = DNAAnalyzer::new();
        let field_type = analyzer.parse_type("String");
        assert_eq!(field_type, FieldType::String);
    }

    #[test]
    fn test_parse_vec_type() {
        let analyzer = DNAAnalyzer::new();
        let field_type = analyzer.parse_type("Vec<String>");
        match field_type {
            FieldType::Vec(inner) => assert_eq!(*inner, FieldType::String),
            _ => panic!("Expected Vec type"),
        }
    }

    #[test]
    fn test_parse_option_type() {
        let analyzer = DNAAnalyzer::new();
        let field_type = analyzer.parse_type("Option<String>");
        match field_type {
            FieldType::Option(inner) => assert_eq!(*inner, FieldType::String),
            _ => panic!("Expected Option type"),
        }
    }

    #[test]
    fn test_detect_reference_field() {
        let analyzer = DNAAnalyzer::new();
        assert!(analyzer.is_reference_field("content_id", "String"));
        assert!(analyzer.is_reference_field("related_ids", "Vec<String>"));
        assert!(analyzer.is_reference_field("id", "String"));
        assert!(!analyzer.is_reference_field("title", "String"));
    }

    #[test]
    fn test_entry_type_name_conversion() {
        let schema = EntryTypeSchema {
            name: "Content".to_string(),
            fields: vec![],
            is_public: true,
        };
        assert_eq!(schema.to_entry_type_name(), "content");

        let schema = EntryTypeSchema {
            name: "LearningPath".to_string(),
            fields: vec![],
            is_public: true,
        };
        assert_eq!(schema.to_entry_type_name(), "learning_path");
    }

    #[test]
    fn test_parse_source_with_struct() {
        let source = r#"
pub struct Content {
    pub id: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub schema_version: u32,
}
"#;

        let mut analyzer = DNAAnalyzer::new();
        assert!(analyzer.parse_source(source).is_ok());

        assert_eq!(analyzer.entry_types().len(), 1);
        let content = &analyzer.entry_types()[0];
        assert_eq!(content.name, "Content");
        assert_eq!(content.fields.len(), 5);
    }

    #[test]
    fn test_parse_source_with_enum() {
        let source = r#"
pub enum ValidationStatus {
    Valid,
    Migrated,
    Degraded,
    Healing,
}
"#;

        let mut analyzer = DNAAnalyzer::new();
        assert!(analyzer.parse_source(source).is_ok());

        assert!(analyzer.enums().contains_key("ValidationStatus"));
        let enum_def = &analyzer.enums()["ValidationStatus"];
        assert_eq!(enum_def.variants.len(), 4);
    }
}
