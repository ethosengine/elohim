//! `ark` — the launchable unit of the elohim compute envelope (tevah).
//!
//! Spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §11 S0.
//! The surface is declared here so the shape is reviewable before it is wired;
//! every subcommand refuses with exit 64 until Task 10 hands it a supervisor.
//!
//! Exit codes (spec §11 S0): 0 clean stop; 3 every process reached GiveUp;
//! 64 usage; 65 manifest or berth invalid; 66 artifact hash mismatch;
//! 67 spool unwritable.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Exit code for "usage" — also the S0 placeholder for an unwired subcommand.
const EXIT_USAGE: i32 = 64;

#[derive(Parser, Debug)]
#[command(
    name = "ark",
    about = "Run a RuntimeManifest in a Berth and witness what dies",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Supervise the manifest's processes in the given berth until they stop.
    Run {
        /// Path to the RuntimeManifest JSON.
        manifest: PathBuf,
        /// Path to the Berth JSON.
        berth: PathBuf,
    },
    /// Print the berth's passport as JSON.
    Describe {
        /// Path to the Berth JSON.
        berth: PathBuf,
    },
    /// Read the berth's death witnesses out of the amber-local spool.
    Witness {
        #[command(subcommand)]
        cmd: WitnessCmd,
    },
    /// Print the sha256 of a file — the same hash the driver checks before spawn.
    Hash {
        /// Path to the artifact file.
        file: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum WitnessCmd {
    /// List the witnesses in the berth's spool, newest first.
    Ls {
        /// Path to the Berth JSON.
        berth: PathBuf,
    },
    /// Show one witness by its CID.
    Show {
        /// Path to the Berth JSON.
        berth: PathBuf,
        /// The witness CID string.
        cid: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let what = match &cli.command {
        Command::Run { .. } => "ark run",
        Command::Describe { .. } => "ark describe",
        Command::Witness { cmd } => match cmd {
            WitnessCmd::Ls { .. } => "ark witness ls",
            WitnessCmd::Show { .. } => "ark witness show",
        },
        Command::Hash { .. } => "ark hash",
    };
    eprintln!("{what}: not yet wired (Task 10)");
    std::process::exit(EXIT_USAGE);
}
