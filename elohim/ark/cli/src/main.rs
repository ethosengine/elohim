//! `ark` — run a RuntimeManifest in a Berth and inspect its local evidence.

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc, Mutex,
    },
};

use ark_core::{Berth, RuntimeManifest, RuntimeScope, WitnessSink};
use ark_supervisor::{
    sha256_file,
    spool::{Spool, SpoolError, WitnessSummary},
    supervisor::{StateObserver, Supervisor, SupervisorError, SystemClock},
    DriverError, NativeDriver,
};
use cid::Cid;
use clap::{error::ErrorKind, Parser, Subcommand};
use nix::{
    libc,
    sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
};
use serde_json::{json, Value};

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 64;
const EXIT_DATA: u8 = 65;
const EXIT_ARTIFACT: u8 = 66;
const EXIT_SPOOL: u8 = 67;

static ACTIVE_SHUTDOWN: AtomicPtr<AtomicBool> = AtomicPtr::new(ptr::null_mut());

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
        #[arg(long)]
        manifest: PathBuf,
        /// Path to the Berth JSON.
        #[arg(long)]
        berth: PathBuf,
    },
    /// Print the berth's passport as JSON.
    Describe {
        /// Path to the Berth JSON.
        #[arg(long)]
        berth: PathBuf,
    },
    /// Read the berth's death witnesses out of the amber-local spool.
    Witness {
        #[command(subcommand)]
        command: WitnessCommand,
    },
    /// Print the SHA-256 of a file.
    Hash {
        /// Path to the artifact file.
        file: PathBuf,
    },
    /// Inspect a runtime manifest.
    Manifest {
        #[command(subcommand)]
        command: ManifestCommand,
    },
}

#[derive(Subcommand, Debug)]
enum WitnessCommand {
    /// List deaths in the berth's spool, newest first.
    Ls {
        /// Path to the Berth JSON.
        #[arg(long)]
        berth: PathBuf,
    },
    /// Show one witness by its CID.
    Show {
        /// Path to the Berth JSON.
        #[arg(long)]
        berth: PathBuf,
        /// The witness CID string.
        cid: String,
    },
}

#[derive(Subcommand, Debug)]
enum ManifestCommand {
    /// Print the manifest's canonical CID.
    Cid {
        /// Path to the RuntimeManifest JSON.
        #[arg(long)]
        manifest: PathBuf,
    },
}

#[derive(Debug)]
struct Failure {
    code: u8,
    message: String,
}

impl Failure {
    fn data(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_DATA,
            message: message.into(),
        }
    }

    fn spool(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_SPOOL,
            message: message.into(),
        }
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = match error.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => EXIT_OK,
                _ => EXIT_USAGE,
            };
            let _ = error.print();
            return ExitCode::from(code);
        }
    };

    match execute(cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("ark: {}", error.message);
            ExitCode::from(error.code)
        }
    }
}

fn execute(command: Command) -> Result<u8, Failure> {
    match command {
        Command::Run { manifest, berth } => run(&manifest, &berth),
        Command::Describe { berth } => describe(&berth),
        Command::Witness { command } => match command {
            WitnessCommand::Ls { berth } => witness_ls(&berth),
            WitnessCommand::Show { berth, cid } => witness_show(&berth, &cid),
        },
        Command::Hash { file } => hash(&file),
        Command::Manifest { command } => match command {
            ManifestCommand::Cid { manifest } => manifest_cid(&manifest),
        },
    }
}

fn run(manifest_path: &Path, berth_path: &Path) -> Result<u8, Failure> {
    let manifest = read_manifest(manifest_path)?;
    let berth = read_berth(berth_path)?;
    let scope = Supervisor::scope_for(&manifest, &berth).map_err(supervisor_validation)?;
    let spool = Spool::open(&berth.data_root, scope).map_err(spool_failure)?;
    let supervisor = Supervisor::new(
        manifest,
        berth,
        Box::new(NativeDriver),
        Box::new(spool),
        Box::new(SystemClock),
    )
    .map_err(supervisor_validation)?
    .with_observer(Arc::new(JsonStateObserver::default()));

    let shutdown = supervisor.shutdown_flag();
    install_signal_handlers(&shutdown)?;
    let outcome = supervisor.run().map_err(supervisor_run_failure);
    ACTIVE_SHUTDOWN.store(ptr::null_mut(), Ordering::SeqCst);
    let outcome = outcome?;
    Ok(outcome.exit_code as u8)
}

fn describe(berth_path: &Path) -> Result<u8, Failure> {
    let berth = read_berth(berth_path)?;
    let spool = open_berth_spool(&berth)?;
    let passport = WitnessSink::load_passport(&spool)
        .map_err(|error| Failure::spool(error.to_string()))?
        .ok_or_else(|| Failure::data("the berth has no passport"))?;
    print_json(serde_json::to_value(passport).map_err(json_failure)?)?;
    Ok(EXIT_OK)
}

fn witness_ls(berth_path: &Path) -> Result<u8, Failure> {
    let berth = read_berth(berth_path)?;
    let spool = open_berth_spool(&berth)?;
    let summaries = spool.list_witnesses().map_err(spool_read_failure)?;
    print_json(Value::Array(pair_witnesses(summaries)?))?;
    Ok(EXIT_OK)
}

fn witness_show(berth_path: &Path, witness_cid: &str) -> Result<u8, Failure> {
    let berth = read_berth(berth_path)?;
    let spool = open_berth_spool(&berth)?;
    let witness = spool
        .read_witness(witness_cid)
        .map_err(spool_read_failure)?;
    print_json(serde_json::to_value(witness).map_err(json_failure)?)?;
    Ok(EXIT_OK)
}

fn hash(file: &Path) -> Result<u8, Failure> {
    let digest = sha256_file(file)
        .map_err(|error| Failure::data(format!("hashing {}: {error}", file.display())))?;
    println!("{digest}");
    Ok(EXIT_OK)
}

fn manifest_cid(path: &Path) -> Result<u8, Failure> {
    let manifest = read_manifest(path)?;
    let cid = manifest
        .cid()
        .map_err(|error| Failure::data(error.to_string()))?;
    println!("{cid}");
    Ok(EXIT_OK)
}

fn read_manifest(path: &Path) -> Result<RuntimeManifest, Failure> {
    let json = read_text(path, "manifest")?;
    RuntimeManifest::from_json(&json).map_err(|error| Failure::data(error.to_string()))
}

fn read_berth(path: &Path) -> Result<Berth, Failure> {
    let json = read_text(path, "berth")?;
    Berth::from_json(&json).map_err(|error| Failure::data(error.to_string()))
}

fn read_text(path: &Path, kind: &str) -> Result<String, Failure> {
    fs::read_to_string(path)
        .map_err(|error| Failure::data(format!("reading {kind} {}: {error}", path.display())))
}

fn open_berth_spool(berth: &Berth) -> Result<Spool, Failure> {
    if berth.data_root.as_os_str().is_empty() {
        return Err(Failure::data(
            "berth declares no data_root: the spool has nowhere to live",
        ));
    }
    let manifest_cid = berth.manifest.parse::<Cid>().map_err(|error| {
        Failure::data(format!(
            "berth manifest {} is not a CID: {error}",
            berth.manifest
        ))
    })?;
    let scope = if berth.node.is_some() {
        RuntimeScope::from_berth(berth).map_err(|error| Failure::data(error.to_string()))?
    } else {
        RuntimeScope::new(manifest_cid)
    };
    Spool::open(&berth.data_root, scope).map_err(spool_failure)
}

fn pair_witnesses(summaries: Vec<WitnessSummary>) -> Result<Vec<Value>, Failure> {
    type DeathKey = (String, u32, u64);
    type Pair = (Option<WitnessSummary>, Option<WitnessSummary>);
    let mut deaths: BTreeMap<DeathKey, Pair> = BTreeMap::new();

    for summary in summaries {
        let key = (
            summary.incident.clone(),
            summary.pid,
            summary.died_at_epoch_ms,
        );
        let pair = deaths.entry(key).or_default();
        let slot = if summary.verdict.is_some() {
            &mut pair.1
        } else {
            &mut pair.0
        };
        if slot
            .as_ref()
            .is_none_or(|existing| summary.cid < existing.cid)
        {
            *slot = Some(summary);
        }
    }

    let mut rows = deaths
        .into_values()
        .map(|(write_ahead, decided)| {
            let representative = decided
                .as_ref()
                .or(write_ahead.as_ref())
                .expect("a pair is created from at least one summary");
            Ok(json!({
                "cid": representative.cid,
                "write_ahead_cid": write_ahead.as_ref().map(|summary| &summary.cid),
                "verdict_cid": decided.as_ref().map(|summary| &summary.cid),
                "incident": representative.incident,
                "process": representative.process,
                "pid": representative.pid,
                "died_at_epoch_ms": representative.died_at_epoch_ms,
                "exit": serde_json::to_value(representative.exit).map_err(json_failure)?,
                "verdict": decided.as_ref().and_then(|summary| summary.verdict.as_ref()),
            }))
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    rows.sort_by(|left, right| {
        right["died_at_epoch_ms"]
            .as_u64()
            .cmp(&left["died_at_epoch_ms"].as_u64())
            .then_with(|| right["cid"].as_str().cmp(&left["cid"].as_str()))
    });
    Ok(rows)
}

fn print_json(value: Value) -> Result<(), Failure> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &value).map_err(json_failure)?;
    writeln!(output).map_err(|error| Failure::spool(format!("writing JSON: {error}")))
}

fn json_failure(error: serde_json::Error) -> Failure {
    Failure::spool(format!("JSON: {error}"))
}

fn spool_failure(error: SpoolError) -> Failure {
    Failure::spool(error.to_string())
}

fn spool_read_failure(error: SpoolError) -> Failure {
    match error {
        SpoolError::Data(message) => Failure::data(message),
        other => Failure::spool(other.to_string()),
    }
}

fn supervisor_validation(error: SupervisorError) -> Failure {
    Failure::data(error.to_string())
}

fn supervisor_run_failure(error: SupervisorError) -> Failure {
    let code = match &error {
        SupervisorError::Driver {
            source: DriverError::ArtifactHashMismatch { .. },
            ..
        } => EXIT_ARTIFACT,
        SupervisorError::Sink(_) => EXIT_SPOOL,
        _ => EXIT_DATA,
    };
    Failure {
        code,
        message: error.to_string(),
    }
}

extern "C" fn request_shutdown(_signal: libc::c_int) {
    let shutdown = ACTIVE_SHUTDOWN.load(Ordering::SeqCst);
    if !shutdown.is_null() {
        // SAFETY: `install_signal_handlers` stores the pointee from an `Arc<AtomicBool>`
        // retained by both that function's caller and the supervisor until `run` returns.
        unsafe { &*shutdown }.store(true, Ordering::SeqCst);
    }
}

fn install_signal_handlers(shutdown: &Arc<AtomicBool>) -> Result<(), Failure> {
    ACTIVE_SHUTDOWN.store(Arc::as_ptr(shutdown).cast_mut(), Ordering::SeqCst);
    let action = SigAction::new(
        SigHandler::Handler(request_shutdown),
        SaFlags::empty(),
        SigSet::empty(),
    );
    // SAFETY: the installed handler has C linkage and performs only atomic operations.
    let installed = unsafe {
        sigaction(Signal::SIGTERM, &action).and_then(|_| sigaction(Signal::SIGINT, &action))
    };
    if let Err(error) = installed {
        ACTIVE_SHUTDOWN.store(ptr::null_mut(), Ordering::SeqCst);
        return Err(Failure::data(format!(
            "installing signal handlers: {error}"
        )));
    }
    Ok(())
}

#[derive(Default)]
struct JsonStateObserver {
    states: Mutex<BTreeMap<String, String>>,
}

impl StateObserver for JsonStateObserver {
    fn on_state(&self, process: &str, state: &str, pid: Option<u32>, incarnation: u64) {
        let mut states = self.states.lock().expect("state observer lock");
        if states
            .get(process)
            .is_some_and(|previous| previous == state)
        {
            return;
        }
        states.insert(process.to_string(), state.to_string());
        eprintln!(
            "{}",
            json!({
                "ark": "state",
                "process": process,
                "state": state,
                "pid": pid,
                "incarnation": incarnation,
            })
        );
    }
}
