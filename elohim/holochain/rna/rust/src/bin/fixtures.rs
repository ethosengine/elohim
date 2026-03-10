//! CLI tool for analyzing and generating Rust fixtures from JSON seed data
//!
//! Validates JSON files with metadata-focused approach and generates
//! a Rust module with embedded, typed fixture data.
//!
//! # Content Architecture
//!
//! DNA validates METADATA, not content:
//! - Required: id, title
//! - Hints: content_format, content_type (any value accepted)
//! - Blob refs: blob_hash, entry_point (for cache resolution)
//!
//! Usage:
//!   hc-rna-fixtures [OPTIONS] --fixtures <PATH>
//!
//! Examples:
//!   # Analyze fixtures (metadata validation)
//!   hc-rna-fixtures -f fixtures/ --analyze
//!
//!   # Generate fixtures module
//!   hc-rna-fixtures -f fixtures/ -o src/fixtures.rs
//!
//!   # Strict validation (all DNA fields)
//!   hc-rna-fixtures -f fixtures/ --mode strict -i integrity.rs

use std::fs;
use std::path::PathBuf;
use clap::{Parser, ValueEnum};
use std::time::Instant;

use hc_rna::{
    DNAAnalyzer, generate_fixtures_module,
    ValidationMode, analyze_fixtures_directory,
};

#[derive(Debug, Clone, ValueEnum)]
enum CliValidationMode {
    /// Only validate truly required fields (id, title)
    Metadata,
    /// Validate all DNA schema fields
    Strict,
    /// Accept anything with an id field
    Loose,
}

impl From<CliValidationMode> for ValidationMode {
    fn from(mode: CliValidationMode) -> Self {
        match mode {
            CliValidationMode::Metadata => ValidationMode::Metadata,
            CliValidationMode::Strict => ValidationMode::Strict,
            CliValidationMode::Loose => ValidationMode::Loose,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "hc-rna-fixtures")]
#[command(about = "Analyze and generate Rust fixtures from JSON seed data")]
#[command(long_about = "Analyzes JSON fixture files with metadata-focused validation.\n\nDNA validates METADATA (id, title, provenance), not CONTENT.\nContent formats are hints for clients, not DNA validation rules.")]
struct Args {
    /// Path to fixtures directory (contains JSON files)
    #[arg(short, long, value_name = "PATH")]
    fixtures: PathBuf,

    /// Path to the integrity zome lib.rs file (for strict mode)
    #[arg(short, long, value_name = "PATH")]
    integrity: Option<PathBuf>,

    /// Output path for generated fixtures.rs module
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Validation mode
    #[arg(long, value_enum, default_value = "metadata")]
    mode: CliValidationMode,

    /// Only analyze fixtures, don't generate code
    #[arg(long)]
    analyze: bool,

    /// Preview output without writing to file
    #[arg(short, long)]
    dry_run: bool,

    /// Verbosity level (can be repeated: -v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Module name for generated code
    #[arg(long, default_value = "fixtures")]
    module_name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start = Instant::now();

    let mode: ValidationMode = args.mode.clone().into();

    if args.verbose >= 1 {
        println!("🔍 Analyzing fixtures...");
        println!("   Fixtures dir: {}", args.fixtures.display());
        println!("   Mode: {:?}", mode);
    }

    // Step 1: Analyze fixtures with metadata-focused validation
    let analysis = analyze_fixtures_directory(&args.fixtures, mode)?;

    // Step 2: Print analysis results
    println!("\n{}", "=".repeat(70));
    println!("FIXTURE ANALYSIS (Metadata-Focused)");
    println!("{}", "=".repeat(70));
    println!("\nTotal files:         {}", analysis.total_files);
    println!("Valid metadata:      {} ✅", analysis.valid_metadata);
    println!("With blob refs:      {} 📦", analysis.with_blob_refs);
    println!("Missing required:    {} ❌", analysis.missing_required.len());

    // Format distribution
    if !analysis.format_distribution.is_empty() {
        println!("\n--- Content Formats (client hints) ---");
        let mut formats: Vec<_> = analysis.format_distribution.iter().collect();
        formats.sort_by(|a, b| b.1.cmp(a.1));
        for (format, count) in formats {
            println!("   {}: {} files", format, count);
        }
    }

    // Type distribution
    if !analysis.type_distribution.is_empty() {
        println!("\n--- Content Types (client hints) ---");
        let mut types: Vec<_> = analysis.type_distribution.iter().collect();
        types.sort_by(|a, b| b.1.cmp(a.1));
        for (ctype, count) in types.iter().take(10) {
            println!("   {}: {} files", ctype, count);
        }
        if types.len() > 10 {
            println!("   ... and {} more types", types.len() - 10);
        }
    }

    // Missing required
    if !analysis.missing_required.is_empty() {
        println!("\n--- Missing Required Fields ---");
        for (file, fields) in analysis.missing_required.iter().take(10) {
            println!("   {}: {}", file, fields.join(", "));
        }
        if analysis.missing_required.len() > 10 {
            println!("   ... and {} more files", analysis.missing_required.len() - 10);
        }
    }

    println!("{}", "=".repeat(70));

    // Early exit if just analyzing
    if args.analyze {
        let elapsed = start.elapsed().as_millis();
        println!("\n⏱️  Analysis completed in {}ms", elapsed);
        return Ok(());
    }

    // Step 3: Generate fixtures module
    if args.verbose >= 1 {
        println!("\n🔨 Generating fixtures module...");
    }

    // Parse DNA schema if provided
    let entry_types = if let Some(integrity_path) = &args.integrity {
        let integrity_source = fs::read_to_string(integrity_path)
            .map_err(|e| format!("Failed to read {}: {}", integrity_path.display(), e))?;
        let mut analyzer = DNAAnalyzer::new();
        analyzer.parse_source(&integrity_source)?;
        analyzer.entry_types().iter().cloned().collect()
    } else {
        Vec::new()
    };

    let generated_code = generate_fixtures_module(
        &args.fixtures,
        &entry_types,
        &args.module_name,
    )?;

    // Step 4: Output
    if args.dry_run {
        println!("\n✨ Generated code (dry-run, not written):\n");
        for (i, line) in generated_code.lines().take(50).enumerate() {
            println!("{}", line);
            if i == 49 && generated_code.lines().count() > 50 {
                println!("... ({} more lines)", generated_code.lines().count() - 50);
            }
        }
        println!("\n💡 To write this file, run without --dry-run");
    } else if let Some(output_path) = args.output {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, &generated_code)?;
        println!("\n✅ Generated fixtures module: {}", output_path.display());
        println!("   {} lines, {} bytes",
            generated_code.lines().count(),
            generated_code.len());
    } else {
        println!("\n⚠️  No output path specified. Use -o to write the generated code.");
    }

    let elapsed = start.elapsed().as_millis();
    if args.verbose >= 1 {
        println!("\n⏱️  Completed in {}ms", elapsed);
    }

    // Print next steps
    println!("\n📋 Next steps:");
    println!("   1. Add generated module to your coordinator zome:");
    println!("      mod {};", args.module_name);
    println!("   2. Add dependencies to Cargo.toml:");
    println!("      once_cell = \"1.19\"");
    println!("   3. Call seed_fixtures() from init or first-agent logic");
    println!("   4. Build and test: cargo build --release");

    Ok(())
}
