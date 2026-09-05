//! The `hc-dbtool` operator CLI. See the crate docs in `lib.rs`.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use hc_dbtool::apps;
use hc_dbtool::blocks::{self, CellSelector};
use hc_dbtool::db::{self, Access, Databases};
use hc_dbtool::rejected;

#[derive(Parser, Debug)]
#[command(
    name = "hc-dbtool",
    about = "See, explain and lift Holochain 0.7 cell blocks",
    long_about = "Holochain 0.7 blocks an agent's cell forever when one op they authored \
                  integrates as invalid, and ships no way to lift it. This tool reads the \
                  BlockSpan rows, reads the rejected ops that caused them, and — only with \
                  --yes, and only while the conductor is stopped — deletes them."
)]
struct Cli {
    /// The conductor's `databases/` directory (holds conductor.db, dht-*.db, db.key).
    #[arg(long, value_name = "DIR")]
    databases: std::path::PathBuf,

    /// Passphrase for `db.key`. The local household mesh uses the literal `test`
    /// (`app/elohim-app/scripts/hc-mesh.sh`).
    #[arg(long, default_value = "test", value_name = "PASSPHRASE")]
    passphrase: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List this conductor's installed apps: its own agent key, and each role's DNA.
    Apps,
    /// List every BlockSpan row in conductor.db, decoded.
    Blocks,
    /// List rejected ops and warrants in one DNA's DHT database.
    Rejected {
        /// DNA hash in `uhC0k…` form (the `dht-<hash>.db` filename).
        #[arg(long, value_name = "HASH")]
        dna: String,
    },
    /// Delete the BlockSpan rows for a cell. Requires a stopped conductor.
    Unblock {
        /// `<dna>:<agent>`, or `<dna>:*` for every agent blocked in that DNA.
        #[arg(long, value_name = "CELL")]
        cell: String,
        /// Confirm the write. Without it the matching rows are only printed.
        #[arg(long)]
        yes: bool,
    },
}

fn main() {
    // Print the failure as a sentence an operator can act on, not as a Rust
    // backtrace. Every error this tool raises is already phrased as advice.
    if let Err(err) = run() {
        eprintln!("hc-dbtool: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let dbs = Databases::new(&cli.databases);
    if !dbs.root().is_dir() {
        bail!("not a databases directory: {}", dbs.root().display());
    }
    let passphrase = cli.passphrase.clone().into_bytes();

    match &cli.command {
        Command::Apps => cmd_apps(&dbs, &passphrase),
        Command::Blocks => cmd_blocks(&dbs, &passphrase),
        Command::Rejected { dna } => cmd_rejected(&dbs, &passphrase, dna),
        Command::Unblock { cell, yes } => cmd_unblock(&dbs, &passphrase, cell, *yes),
    }
}

fn cmd_apps(dbs: &Databases, passphrase: &[u8]) -> Result<()> {
    let mut key = dbs.load_key(passphrase)?;
    let conn = db::open(&dbs.conductor_db(), &mut key, Access::ReadOnly)?;
    println!("APPS — {}", dbs.conductor_db().display());
    let apps = apps::list(&conn)?;
    if apps.is_empty() {
        println!("  (none installed)");
    }
    for app in &apps {
        println!("{}\n", app.render());
    }
    Ok(())
}

fn cmd_blocks(dbs: &Databases, passphrase: &[u8]) -> Result<()> {
    let mut key = dbs.load_key(passphrase)?;
    let conn = db::open(&dbs.conductor_db(), &mut key, Access::ReadOnly)?;
    let rows = blocks::list(&conn)?;

    // Name this conductor first, so every row below reads as "this agent
    // refuses that agent" rather than as an unattributed pair of hashes.
    match apps::own_agent_keys(&conn) {
        Ok(keys) if !keys.is_empty() => {
            println!("THIS CONDUCTOR — {}", keys.join(", "));
        }
        Ok(_) => println!("THIS CONDUCTOR — (no installed app; agent key unknown)"),
        Err(e) => println!("THIS CONDUCTOR — (could not read InstalledApp: {e})"),
    }

    println!("BLOCKS — {}", dbs.conductor_db().display());
    if rows.is_empty() {
        println!("  (none)");
        return Ok(());
    }
    let permanent = rows.iter().filter(|r| r.is_permanent()).count();
    println!(
        "  {} row(s), {permanent} of them permanent (end_us = Timestamp::max)\n",
        rows.len()
    );
    for row in &rows {
        println!("{}", row.render());
        if let Some(op) = &row.invalid_op {
            let dna = row.target.split(':').next().unwrap_or_default();
            println!(
                "        explain: hc-dbtool --databases {} rejected --dna {dna}   # look for {op}",
                dbs.root().display()
            );
        }
        println!();
    }
    Ok(())
}

fn cmd_rejected(dbs: &Databases, passphrase: &[u8], dna: &str) -> Result<()> {
    let mut key = dbs.load_key(passphrase)?;
    let path = dbs.dht_db(dna);
    let conn = db::open(&path, &mut key, Access::ReadOnly)?;

    let ops = rejected::list(&conn)?;
    println!("REJECTED OPS — {}", path.display());
    if ops.is_empty() {
        println!("  (none)");
    } else {
        println!("  {} row(s)\n", ops.len());
        for op in &ops {
            println!("{}\n", op.render());
        }
    }

    let warrants = rejected::warrants(&conn)?;
    println!("WARRANTS — {}", path.display());
    if warrants.is_empty() {
        println!("  (none)");
    } else {
        println!("  {} row(s)\n", warrants.len());
        for w in &warrants {
            println!("{}\n", w.render());
        }
    }
    Ok(())
}

fn cmd_unblock(dbs: &Databases, passphrase: &[u8], cell: &str, yes: bool) -> Result<()> {
    let selector = CellSelector::parse(cell)?;
    let conductor_db = dbs.conductor_db();

    // Refuse while the database is live. The stop is the operator's move, not
    // ours — this tool never starts or stops a conductor.
    let holders = db::lock_holders(&conductor_db)
        .context("could not establish whether the conductor is stopped")?;
    if !holders.is_empty() {
        let who = holders
            .iter()
            .map(|(pid, exe)| format!("pid {pid} ({exe})"))
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "refusing to touch a live database: {} is open by {who}.\n\
             Stop the conductor, re-run this command, then start it again.\n\
             `blocks` reads the same rows without stopping anything.",
            conductor_db.display()
        );
    }

    let mut key = dbs.load_key(passphrase)?;

    if !yes {
        let conn = db::open(&conductor_db, &mut key, Access::ReadOnly)?;
        let matched: Vec<_> = blocks::list(&conn)?
            .into_iter()
            .filter(|r| selector.matches(r))
            .collect();
        println!(
            "DRY RUN — {} row(s) would be deleted from BlockSpan",
            matched.len()
        );
        for row in &matched {
            println!("{}\n", row.render());
        }
        println!("Re-run with --yes to remove them.");
        return Ok(());
    }

    let conn = db::open(&conductor_db, &mut key, Access::ReadWrite)?;
    let removed = blocks::delete_matching(&conn, &selector)?;
    if removed.is_empty() {
        println!("No BlockSpan row matched {cell}; nothing was changed.");
        return Ok(());
    }
    println!("LIFTED — {} BlockSpan row(s) deleted", removed.len());
    for row in &removed {
        println!("{}\n", row.render());
    }
    println!("Start the conductor again for the peer store to re-admit the cell.");
    Ok(())
}
