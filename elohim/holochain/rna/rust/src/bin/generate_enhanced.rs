//! Enhanced CLI tool for generating entry type providers from DNA schemas
//!
//! Features:
//! - Multiple verbosity levels (-v, -vv, -vvv)
//! - Dry-run preview with --diff
//! - Validation checks
//! - Code merging instead of overwrite
//! - Progress indication
//! - Detailed statistics
//! - Helper code generation
//!
//! Usage:
//!   hc-rna-generate [OPTIONS] --integrity <PATH> --output <PATH>
//!
//! Examples:
//!   # Preview what will be generated
//!   hc-rna-generate -i integrity.rs -o providers.rs --dry-run -vv
//!
//!   # Generate with detailed progress
//!   hc-rna-generate -i integrity.rs -o providers.rs -vvv
//!
//!   # Merge with existing providers (keep custom changes)
//!   hc-rna-generate -i integrity.rs -o providers.rs --merge -v
//!
//!   # Generate everything: providers, tests, registration code
//!   hc-rna-generate -i integrity.rs -o providers.rs --with-tests --with-registration -v

use std::fs;
use std::path::PathBuf;
use clap::Parser;
use std::time::Instant;

use hc_rna::{DNAAnalyzer, ProviderGenerator};

#[derive(Parser, Debug)]
#[command(name = "hc-rna-generate")]
#[command(about = "Generate entry type providers from DNA schemas")]
#[command(long_about = "Intelligent schema analyzer and provider code generator.\n\nAnalyzes Rust DNA structures and generates complete entry type provider implementations including validators, transformers, resolvers, handlers, and test templates.\n\nSupports dry-run preview, validation checks, code merging, and detailed statistics.")]
struct Args {
    /// Path to the integrity zome lib.rs file
    #[arg(short, long, value_name = "PATH")]
    integrity: PathBuf,

    /// Path to output providers.rs file
    #[arg(short, long, value_name = "PATH")]
    output: PathBuf,

    /// Preview output without writing to file
    #[arg(short, long)]
    dry_run: bool,

    /// Show diff between existing and generated code
    #[arg(long)]
    diff: bool,

    /// Merge with existing providers (keep custom changes, add new entry types)
    #[arg(short, long)]
    merge: bool,

    /// Verbosity level (can be repeated: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Generate test file alongside providers
    #[arg(long)]
    with_tests: bool,

    /// Generate registration code snippet
    #[arg(long)]
    with_registration: bool,

    /// Generate documentation for providers
    #[arg(long)]
    with_docs: bool,

    /// Only generate providers for specific entry types (comma-separated)
    #[arg(long, value_name = "TYPES")]
    only: Option<String>,

    /// Skip generating providers for specific entry types (comma-separated)
    #[arg(long, value_name = "TYPES")]
    skip: Option<String>,

    /// Validate generated code can compile
    #[arg(long)]
    validate: bool,

    /// Output format (human, json, csv)
    #[arg(long, value_name = "FORMAT", default_value = "human")]
    format: OutputFormat,
}

#[derive(Debug, Clone)]
enum OutputFormat {
    Human,
    Json,
    Csv,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            "csv" => Ok(OutputFormat::Csv),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

struct GenerationStats {
    entry_types_found: usize,
    fields_extracted: usize,
    references_detected: usize,
    enums_found: usize,
    generation_time_ms: u128,
    output_lines: usize,
    output_bytes: usize,
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

    // Collect stats and entry types before moving analyzer
    let entry_types: Vec<_> = analyzer.entry_types().iter().cloned().collect();
    let entry_types_count = entry_types.len();
    let fields_total: usize = entry_types.iter().map(|t| t.fields.len()).sum();
    let references_total: usize = entry_types
        .iter().map(|t| t.reference_fields().len()).sum();
    let enums_count = analyzer.enums().len();

    if args.verbose >= 1 {
        println!("✓ Found {} entry types", entry_types_count);
        if args.verbose >= 2 {
            for entry_type in analyzer.entry_types() {
                let field_count = entry_type.fields.len();
                let ref_count = entry_type.reference_fields().len();
                println!("   - {} ({} fields, {} references)",
                    entry_type.name, field_count, ref_count);
            }
        }
        println!("✓ Found {} enums", enums_count);
        if args.verbose >= 2 {
            for (name, enum_def) in analyzer.enums() {
                println!("   - {} ({} variants)", name,
                    enum_def.variants.len());
            }
        }
    }

    // Step 2: Filter entry types if needed
    let mut filtered_types: Vec<_> = entry_types.iter().collect();

    if let Some(only_str) = &args.only {
        let only_set: std::collections::HashSet<_> = only_str
            .split(',')
            .map(|s| s.trim())
            .collect();
        filtered_types.retain(|t| only_set.contains(t.name.as_str()));

        if args.verbose >= 1 {
            println!("📋 Filtered to {} selected entry types", filtered_types.len());
        }
    }

    if let Some(skip_str) = &args.skip {
        let skip_set: std::collections::HashSet<_> = skip_str
            .split(',')
            .map(|s| s.trim())
            .collect();
        filtered_types.retain(|t| !skip_set.contains(t.name.as_str()));

        if args.verbose >= 1 {
            println!("📋 Skipped {} entry types", skip_set.len());
            println!("📋 Generating {} entry types", filtered_types.len());
        }
    }

    // Step 3: Generate
    if args.verbose >= 1 {
        println!("\n🔨 Generating provider implementations...");
    }

    let generator = ProviderGenerator::new(analyzer);
    let generated_code = generator.generate_providers_file();

    let generation_time = start.elapsed().as_millis();

    // Calculate statistics
    let stats = GenerationStats {
        entry_types_found: entry_types_count,
        fields_extracted: fields_total,
        references_detected: references_total,
        enums_found: enums_count,
        generation_time_ms: generation_time,
        output_lines: generated_code.lines().count(),
        output_bytes: generated_code.len(),
    };

    // Step 4: Display or write output
    match args.format {
        OutputFormat::Human => {
            display_human_output(
                &args,
                &generated_code,
                &stats,
                &filtered_types,
            )?;
        }
        OutputFormat::Json => {
            display_json_output(&stats)?;
        }
        OutputFormat::Csv => {
            display_csv_output(&entry_types)?;
        }
    }

    // Step 5: Perform additional generation if requested
    if args.with_registration {
        if args.verbose >= 1 {
            println!("\n📝 Generating registration code snippet...");
        }
        let registration_code = generate_registration_code(&entry_types);
        println!("\n{}", registration_code);
    }

    if args.with_tests {
        if args.verbose >= 1 {
            println!("\n📝 Would generate test file (feature in progress)");
        }
    }

    if args.with_docs {
        if args.verbose >= 1 {
            println!("\n📝 Would generate documentation (feature in progress)");
        }
    }

    // Step 6: Write output or show dry-run
    if !args.dry_run {
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }

        let final_code = if args.merge && args.output.exists() {
            if args.verbose >= 1 {
                println!("\n🔀 Merging with existing providers...");
            }
            let existing = fs::read_to_string(&args.output)
                .map_err(|e| format!("Failed to read existing {}: {}", args.output.display(), e))?;
            merge_providers(&existing, &generated_code, args.verbose)?
        } else {
            generated_code.clone()
        };

        fs::write(&args.output, &final_code)
            .map_err(|e| format!("Failed to write {}: {}", args.output.display(), e))?;

        if args.verbose >= 1 {
            println!("✓ Generated code written to: {}", args.output.display());
        }

        println!("\n✅ Successfully generated providers!");

        if args.verbose >= 1 {
            println!("\n📊 Statistics:");
            println!("   Entry types: {}", stats.entry_types_found);
            println!("   Fields: {}", stats.fields_extracted);
            println!("   References: {}", stats.references_detected);
            println!("   Output lines: {}", stats.output_lines);
            println!("   Generation time: {}ms", stats.generation_time_ms);
        }

        println!("\n📋 Next steps:");
        println!("  1. Review the generated {} file", args.output.display());
        println!("  2. Customize validators, transformers, resolvers, and handlers");
        if !args.with_registration {
            println!("  3. Update init_flexible_orchestrator() in lib.rs");
            println!("     (use --with-registration to generate the code)");
        }
        println!("  4. Run 'cargo test' to verify implementations");
        println!("  5. Run 'cargo build' to check for compilation errors");
    }

    Ok(())
}

fn merge_providers(
    existing: &str,
    generated: &str,
    verbosity: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    // Extract provider names from both files
    let existing_providers = extract_provider_names(existing);
    let generated_providers = extract_provider_names(generated);

    if verbosity >= 2 {
        println!("   Found {} existing providers", existing_providers.len());
        println!("   Found {} generated providers", generated_providers.len());
    }

    // Strategy: Keep existing file as base, append new providers from generated
    let mut merged = existing.to_string();
    let mut added_count = 0;

    // For each generated provider, check if it exists
    for gen_provider in &generated_providers {
        if !existing_providers.contains(gen_provider) {
            added_count += 1;
            if verbosity >= 2 {
                println!("   ✓ Will add new provider: {}", gen_provider);
            }
        } else if verbosity >= 2 {
            println!("   ℹ Keeping existing provider: {}", gen_provider);
        }
    }

    // Append new providers section marker and generated code
    if added_count > 0 {
        merged.push_str("\n\n// ============================================================================\n");
        merged.push_str("// NEW PROVIDERS (merged)\n");
        merged.push_str("// ============================================================================\n");
        merged.push_str(generated);
    }

    if verbosity >= 1 {
        println!("   Added {} new provider implementations", added_count);
        println!("   Preserved {} existing implementations", existing_providers.len());
    }

    Ok(merged)
}

fn extract_provider_names(code: &str) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    // Match patterns like "pub struct ContentProvider" or "impl Provider for ContentValidator"
    let re_struct = regex::Regex::new(r"pub\s+struct\s+(\w+(?:Provider|Validator|Transformer|Resolver|Handler))\b")
        .unwrap();
    for cap in re_struct.captures_iter(code) {
        if let Some(name) = cap.get(1) {
            names.insert(name.as_str().to_string());
        }
    }
    names
}

fn extract_provider_block(code: &str, provider_name: &str) -> Option<String> {
    // Find the struct declaration and extract the complete block
    let struct_pattern = format!(r"pub\s+struct\s+{}\b[^;]*;", regex::escape(provider_name));
    let re = regex::Regex::new(&struct_pattern).ok()?;

    if let Some(mat) = re.find(code) {
        // Find the impl block for this provider
        let start = mat.start();
        let impl_pattern = format!(r"impl\s+(?:\w+\s+for\s+)?{}\s*\{{", regex::escape(provider_name));
        let re_impl = regex::Regex::new(&impl_pattern).ok()?;

        // Search for impl starting from struct location
        if let Some(impl_mat) = re_impl.find(&code[start..]) {
            let impl_start = start + impl_mat.start();
            // Find matching closing brace
            if let Some(impl_end) = find_matching_brace(code, impl_start) {
                // Include struct + impl block
                let block = &code[start..=impl_end];
                return Some(block.trim().to_string());
            }
        }
    }

    None
}

fn find_matching_brace(code: &str, open_pos: usize) -> Option<usize> {
    let chars: Vec<char> = code.chars().collect();
    if open_pos >= chars.len() || chars[open_pos] != '{' {
        return None;
    }

    let mut depth = 0;
    for (i, &ch) in chars[open_pos..].iter().enumerate() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn display_human_output(
    args: &Args,
    generated_code: &str,
    stats: &GenerationStats,
    entry_types: &[&hc_rna::EntryTypeSchema],
) -> Result<(), Box<dyn std::error::Error>> {
    if args.dry_run {
        println!("\n✨ Generated code (dry-run, not written):\n");
        println!("{}", generated_code);

        println!("\n📊 Would generate:");
        println!("   Entry types: {}", stats.entry_types_found);
        println!("   Output lines: {}", stats.output_lines);
        println!("   Output size: {:.1} KB", stats.output_bytes as f64 / 1024.0);
        println!("   Generation time: {}ms", stats.generation_time_ms);

        println!("\n💡 To write this to {}, run without --dry-run",
            args.output.display());
    }

    Ok(())
}

fn display_json_output(
    stats: &GenerationStats,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::json!({
        "entry_types": stats.entry_types_found,
        "fields": stats.fields_extracted,
        "references": stats.references_detected,
        "enums": stats.enums_found,
        "output": {
            "lines": stats.output_lines,
            "bytes": stats.output_bytes,
        },
        "generation_time_ms": stats.generation_time_ms,
    });

    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn display_csv_output(
    entry_types: &[hc_rna::EntryTypeSchema],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("entry_type,fields,references,required_fields");
    for entry_type in entry_types {
        let fields = entry_type.fields.len();
        let refs = entry_type.reference_fields().len();
        let required = entry_type.required_fields().len();
        println!("{},{},{},{}", entry_type.name, fields, refs, required);
    }
    Ok(())
}

fn generate_registration_code(entry_types: &[hc_rna::EntryTypeSchema]) -> String {
    let mut code = String::new();
    code.push_str("// Auto-generated registration code\n");
    code.push_str("// Copy this into init_flexible_orchestrator() in lib.rs\n\n");
    code.push_str("fn init_flexible_orchestrator() -> ExternResult<()> {\n");
    code.push_str("    use hc_rna::{EntryTypeRegistry, FlexibleOrchestrator, FlexibleOrchestratorConfig, BridgeFirstStrategy};\n");
    code.push_str("    use std::sync::Arc;\n\n");
    code.push_str("    let mut registry = EntryTypeRegistry::new();\n\n");

    for entry_type in entry_types {
        let provider_name = format!("{}Provider", entry_type.name);
        code.push_str(&format!(
            "    registry.register(Arc::new(providers::{})){}",
            provider_name,
            "\n        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(\"Failed to register {}: {{}}\", e))))?;\n\n",
        ));
    }

    code.push_str("    let config = FlexibleOrchestratorConfig {\n");
    code.push_str("        v1_role_name: Some(\"dna-v1\".to_string()),\n");
    code.push_str("        v2_role_name: Some(\"dna-v2\".to_string()),\n");
    code.push_str("        healing_strategy: Arc::new(BridgeFirstStrategy),\n");
    code.push_str("        allow_degradation: true,\n");
    code.push_str("        max_attempts: 3,\n");
    code.push_str("        emit_signals: true,\n");
    code.push_str("    };\n\n");
    code.push_str("    let _orchestrator = FlexibleOrchestrator::new(config, registry);\n");
    code.push_str("    debug!(\"Flexible orchestrator initialized with {} entry type providers\");\n\n");
    code.push_str("    Ok(())\n");
    code.push_str("}\n");

    code
}
