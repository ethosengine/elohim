//! CLI tool for exporting DNA schemas as JSON Schema
//!
//! Generates JSON Schema files from DNA entry type definitions for offline
//! validation of seed data before it hits the Holochain conductor.
//!
//! Usage:
//!   hc-rna-schema [OPTIONS] --integrity <PATH> --output <PATH>
//!
//! Examples:
//!   # Export all schemas to a directory
//!   hc-rna-schema -i integrity.rs -o schemas/
//!
//!   # Export a single combined schema
//!   hc-rna-schema -i integrity.rs -o schema.json --combined
//!
//!   # Export only Content schema
//!   hc-rna-schema -i integrity.rs -o content.json --entry Content
//!
//!   # Preview without writing
//!   hc-rna-schema -i integrity.rs -o schemas/ --dry-run -v

use std::fs;
use std::path::PathBuf;
use clap::Parser;
use std::time::Instant;
use regex::Regex;

use hc_rna::{DNAAnalyzer, generate_schemas, generate_combined_schema, export_schemas_to_json};

#[derive(Parser, Debug)]
#[command(name = "hc-rna-schema")]
#[command(about = "Export DNA schemas as JSON Schema for offline validation")]
#[command(long_about = "Analyzes Rust DNA structures and generates JSON Schema files.\n\nThese schemas can be used to validate seed data before running against Holochain, catching schema mismatches in seconds instead of minutes.")]
struct Args {
    /// Path to the integrity zome lib.rs file
    #[arg(short, long, value_name = "PATH")]
    integrity: PathBuf,

    /// Output path (directory for multiple schemas, file for combined/single)
    #[arg(short, long, value_name = "PATH")]
    output: PathBuf,

    /// Generate a single combined schema with all types as definitions
    #[arg(long)]
    combined: bool,

    /// Export only a specific entry type
    #[arg(long, value_name = "NAME")]
    entry: Option<String>,

    /// Export only enum definitions as JSON (for TypeScript generation)
    #[arg(long)]
    export_enums: bool,

    /// Additional file containing const arrays (e.g., healing.rs)
    /// Used with --export-enums to extract validation constants
    #[arg(long, value_name = "PATH")]
    constants_file: Option<PathBuf>,

    /// Preview output without writing to file
    #[arg(short, long)]
    dry_run: bool,

    /// Verbosity level (can be repeated: -v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start = Instant::now();

    if args.verbose >= 1 {
        println!("🔍 Analyzing DNA schema...");
        println!("   Integrity file: {}", args.integrity.display());
    }

    // Step 1: Read and analyze
    let integrity_source = fs::read_to_string(&args.integrity)
        .map_err(|e| format!("Failed to read {}: {}", args.integrity.display(), e))?;

    let mut analyzer = DNAAnalyzer::new();
    analyzer.parse_source(&integrity_source)?;

    let entry_types_count = analyzer.entry_types().len();

    if args.verbose >= 1 {
        println!("✓ Found {} entry types", entry_types_count);
        if args.verbose >= 2 {
            for entry_type in analyzer.entry_types() {
                let required = entry_type.required_fields().len();
                let optional = entry_type.optional_fields().len();
                println!("   - {} ({} required, {} optional fields)",
                    entry_type.name, required, optional);
            }
        }
    }

    // Step 2: Generate schemas
    if args.verbose >= 1 {
        println!("\n📋 Generating JSON Schema...");
    }

    // Handle --export-enums flag first (standalone output)
    if args.export_enums {
        if args.verbose >= 1 {
            println!("📦 Exporting validation constants...");
        }

        let mut const_output = serde_json::Map::new();

        // Extract const arrays from constants file if provided
        if let Some(constants_path) = &args.constants_file {
            if args.verbose >= 1 {
                println!("   Reading constants from: {}", constants_path.display());
            }
            let constants_source = fs::read_to_string(constants_path)
                .map_err(|e| format!("Failed to read {}: {}", constants_path.display(), e))?;

            let const_arrays = extract_const_arrays(&constants_source);
            if args.verbose >= 1 {
                println!("   Found {} const arrays", const_arrays.len());
            }

            for (name, values) in const_arrays {
                let json_values: Vec<serde_json::Value> = values
                    .iter()
                    .map(|v| serde_json::Value::String(v.clone()))
                    .collect();
                const_output.insert(name, serde_json::Value::Array(json_values));
            }
        }

        // Also include any pub enums from the integrity file (like DoorwayTier)
        for (name, enum_def) in analyzer.enums() {
            // Skip macro-generated enums (EntryTypes, LinkTypes)
            if name == "EntryTypes" || name == "LinkTypes" {
                continue;
            }

            let snake_name = to_snake_case(name);
            let variants: Vec<serde_json::Value> = enum_def.variants
                .iter()
                .filter_map(|v| {
                    // Extract just the variant name, ignoring comments and tuple contents
                    let trimmed = v.trim();
                    let name_part = trimmed.split(|c| c == ',' || c == '(' || c == '/').next()?;
                    let name_clean = name_part.trim();
                    if name_clean.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::String(name_clean.to_string()))
                    }
                })
                .collect();

            if !variants.is_empty() {
                const_output.insert(snake_name, serde_json::Value::Array(variants));
            }
        }

        let json_str = serde_json::to_string_pretty(&const_output)?;

        if args.dry_run {
            println!("\n✨ Validation constants (dry-run):\n");
            println!("{}", json_str);
        } else {
            fs::write(&args.output, &json_str)?;
            println!("✅ Written {} validation constants to: {}", const_output.len(), args.output.display());
        }

        let elapsed = start.elapsed().as_millis();
        if args.verbose >= 1 {
            println!("\n⏱️  Completed in {}ms", elapsed);
        }
        return Ok(());
    }

    if let Some(entry_name) = &args.entry {
        // Single entry type
        let schema = generate_combined_schema(&analyzer, entry_name);
        let json_str = serde_json::to_string_pretty(&schema)?;

        if args.dry_run {
            println!("\n✨ Generated schema for {} (dry-run):\n", entry_name);
            println!("{}", json_str);
        } else {
            fs::write(&args.output, &json_str)?;
            println!("✅ Written schema to: {}", args.output.display());
        }
    } else if args.combined {
        // Combined schema with all types
        let first_type = analyzer.entry_types().first()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Content".to_string());
        let schema = generate_combined_schema(&analyzer, &first_type);
        let json_str = serde_json::to_string_pretty(&schema)?;

        if args.dry_run {
            println!("\n✨ Generated combined schema (dry-run):\n");
            // Only show first 100 lines in dry-run
            for (i, line) in json_str.lines().take(100).enumerate() {
                println!("{}", line);
                if i == 99 {
                    println!("... ({} more lines)", json_str.lines().count() - 100);
                }
            }
        } else {
            fs::write(&args.output, &json_str)?;
            println!("✅ Written combined schema to: {}", args.output.display());
        }
    } else {
        // Multiple schemas in directory
        let schemas = generate_schemas(&analyzer);
        let json_schemas = export_schemas_to_json(&schemas);

        if args.dry_run {
            println!("\n✨ Would generate {} schema files (dry-run):", json_schemas.len());
            for (name, _) in &json_schemas {
                println!("   - {}.json", name.to_lowercase());
            }
        } else {
            // Create output directory
            fs::create_dir_all(&args.output)?;

            for (name, json_str) in &json_schemas {
                let file_path = args.output.join(format!("{}.json", name.to_lowercase()));
                fs::write(&file_path, json_str)?;
                if args.verbose >= 1 {
                    println!("   ✓ {}", file_path.display());
                }
            }
            println!("✅ Written {} schemas to: {}", json_schemas.len(), args.output.display());
        }
    }

    let elapsed = start.elapsed().as_millis();
    if args.verbose >= 1 {
        println!("\n⏱️  Completed in {}ms", elapsed);
    }

    Ok(())
}

/// Convert PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Extract const string array definitions from Rust source
///
/// Parses patterns like:
/// ```rust
/// pub const CONTENT_TYPES: &[&str] = &[
///     "epic",
///     "concept",
/// ];
/// // OR sized arrays:
/// pub const ENGAGEMENT_TYPES: [&str; 8] = [
///     "view",
///     "quiz",
/// ];
/// ```
fn extract_const_arrays(source: &str) -> Vec<(String, Vec<String>)> {
    let mut results = Vec::new();

    // Match: pub const NAME: &[&str] = &[ OR pub const NAME: [&str; N] = [
    let const_regex = Regex::new(r#"pub\s+const\s+(\w+)\s*:\s*(?:&\[&str\]|\[&str;\s*\d+\])\s*=\s*&?\["#)
        .expect("Invalid regex");

    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = const_regex.captures(line) {
            let const_name = caps.get(1).unwrap().as_str();
            let snake_name = const_name.to_lowercase();

            let mut values = Vec::new();
            let mut j = i;

            // Find all string values until the closing ];
            while j < lines.len() {
                let current_line = lines[j];

                // Extract quoted strings from this line
                let string_regex = Regex::new(r#""([^"]+)""#).expect("Invalid regex");
                for cap in string_regex.captures_iter(current_line) {
                    if let Some(value) = cap.get(1) {
                        values.push(value.as_str().to_string());
                    }
                }

                // Check if this line ends the array
                if current_line.contains("];") {
                    break;
                }

                j += 1;
            }

            if !values.is_empty() {
                results.push((snake_name, values));
            }
        }
    }

    results
}
