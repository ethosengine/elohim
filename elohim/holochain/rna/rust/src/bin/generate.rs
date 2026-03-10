//! CLI tool for generating entry type providers from DNA schemas
//!
//! Usage:
//!   hc-rna-generate --integrity <path> --output <path> [--dry-run]
//!
//! Example:
//!   hc-rna-generate \
//!     --integrity holochain/dna/lamad/zomes/content_store_integrity/src/lib.rs \
//!     --output holochain/dna/lamad/zomes/content_store/src/providers.rs

use std::fs;
use std::path::PathBuf;
use clap::Parser;

// Re-export from hc-rna library
use hc_rna::{DNAAnalyzer, ProviderGenerator};

#[derive(Parser, Debug)]
#[command(name = "hc-rna-generate")]
#[command(about = "Generate entry type providers from DNA schemas", long_about = None)]
struct Args {
    /// Path to the integrity zome lib.rs file
    #[arg(short, long)]
    integrity: PathBuf,

    /// Path to output providers.rs file
    #[arg(short, long)]
    output: PathBuf,

    /// Preview output without writing to file
    #[arg(short, long)]
    dry_run: bool,

    /// Print verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.verbose {
        println!("🔍 Analyzing DNA schema from: {}", args.integrity.display());
    }

    // Read integrity zome source
    let integrity_source = fs::read_to_string(&args.integrity)
        .map_err(|e| format!("Failed to read {}: {}", args.integrity.display(), e))?;

    // Analyze the schema
    let mut analyzer = DNAAnalyzer::new();
    analyzer.parse_source(&integrity_source)?;

    if args.verbose {
        println!("✓ Found {} entry types:", analyzer.entry_types().len());
        for entry_type in analyzer.entry_types() {
            println!(
                "  - {} (entry_type: \"{}\")",
                entry_type.name,
                entry_type.to_entry_type_name()
            );
        }
    }

    // Generate provider code
    if args.verbose {
        println!("\n🔨 Generating provider implementations...");
    }

    let generator = ProviderGenerator::new(analyzer);
    let entry_type_count = generator.analyzer.entry_types().len();
    let generated_code = generator.generate_providers_file();

    // Display or write output
    if args.dry_run {
        println!("\n✨ Generated code (dry-run, not written):\n");
        println!("{}", generated_code);
        println!(
            "\n💡 To write this to {}, run without --dry-run",
            args.output.display()
        );
    } else {
        // Create parent directories if needed
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write output file
        fs::write(&args.output, &generated_code)
            .map_err(|e| format!("Failed to write {}: {}", args.output.display(), e))?;

        if args.verbose {
            println!("✓ Generated code written to: {}", args.output.display());
        }

        println!(
            "✅ Successfully generated providers for {} entry types!",
            entry_type_count
        );
        println!("\n📋 Next steps:");
        println!("  1. Review the generated {} file", args.output.display());
        println!("  2. Customize validators, transformers, resolvers, and handlers");
        println!("  3. Update init_flexible_orchestrator() in lib.rs to register providers");
        println!("  4. Run tests to verify implementations");
    }

    Ok(())
}
