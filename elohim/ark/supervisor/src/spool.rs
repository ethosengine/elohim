//! The amber-local disk projection of runtime evidence.

use std::{
    ffi::OsString,
    fs::{self, DirBuilder, File, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use ark_core::{
    DeathTally, DeathWitness, ExitClass, Incident, Intent, Passport, RestartVerdict, RuntimeScope,
    SinkError, WitnessSink,
};
use elohim_epr_rea::store::{FlowRecord, FlowStore, SidecarFlowStore};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const DIRECTORY_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;

/// Failure while opening, reading, or durably updating the amber-local spool.
#[derive(thiserror::Error, Debug)]
pub enum SpoolError {
    /// The requested spool root cannot be written by its owner.
    #[error("spool root is unwritable")]
    Unwritable,
    /// A filesystem operation failed.
    #[error("spool I/O at {path}: {message}")]
    Io {
        /// Path at which the operation failed.
        path: PathBuf,
        /// Underlying error text.
        message: String,
    },
    /// A record could not be encoded.
    #[error("spool encoding: {0}")]
    Encode(String),
    /// Stored data could not be decoded.
    #[error("spool data: {0}")]
    Data(String),
    /// A record could not be projected into the REA sidecar.
    #[error("spool flow: {0}")]
    Flow(String),
}

/// List projection for witness CLI output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WitnessSummary {
    /// Content-derived witness CID, taken from the file name.
    pub cid: String,
    /// Incident CID carrying this death.
    pub incident: String,
    /// Process that died.
    pub process: String,
    /// Operating-system process identifier that died.
    pub pid: u32,
    /// Wall-clock death time.
    pub died_at_epoch_ms: u64,
    /// Normalized termination cause.
    pub exit: ExitClass,
    /// Restart decision, absent on the write-ahead copy.
    pub verdict: Option<RestartVerdict>,
}

/// Durable S0 spool plus its inherited REA flow projection.
#[derive(Debug)]
pub struct Spool {
    root: PathBuf,
    flows: SidecarFlowStore,
    scope: RuntimeScope,
}

impl Spool {
    /// Opens `<data_root>/ark`, creating every spool directory with owner-only access.
    pub fn open(data_root: &Path, scope: RuntimeScope) -> Result<Self, SpoolError> {
        ensure_writable_directory(data_root)?;

        let root = data_root.join("ark");
        for directory in [
            root.clone(),
            root.join("witnesses"),
            root.join("incidents"),
            root.join(".eprfs"),
            root.join(".eprfs/status"),
        ] {
            create_private_directory(&directory)?;
        }
        ensure_writable_directory(&root)?;

        let flows = SidecarFlowStore::open(&root).map_err(flow_open_error)?;
        Ok(Self { root, flows, scope })
    }

    /// Lists witness JSON projections newest-first.
    pub fn list_witnesses(&self) -> Result<Vec<WitnessSummary>, SpoolError> {
        let mut summaries = Vec::new();
        for path in json_files(&self.root.join("witnesses"))? {
            let witness: DeathWitness = read_json(&path)?;
            let cid = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| {
                    SpoolError::Data(format!("invalid witness path: {}", path.display()))
                })?
                .to_string();
            summaries.push(WitnessSummary {
                cid,
                incident: witness.incident,
                process: witness.process,
                pid: witness.pid,
                died_at_epoch_ms: witness.died_at_epoch_ms,
                exit: witness.exit,
                verdict: witness.verdict,
            });
        }
        summaries.sort_by(|left, right| {
            right
                .died_at_epoch_ms
                .cmp(&left.died_at_epoch_ms)
                .then_with(|| right.cid.cmp(&left.cid))
        });
        Ok(summaries)
    }

    /// Reads one witness from its JSON sidecar.
    pub fn read_witness(&self, cid: &str) -> Result<DeathWitness, SpoolError> {
        if cid.is_empty()
            || cid
                .chars()
                .any(|character| matches!(character, '/' | '\\' | '.'))
        {
            return Err(SpoolError::Data(format!("invalid witness cid: {cid}")));
        }
        read_json(&self.root.join("witnesses").join(format!("{cid}.json")))
    }

    /// Lists incident projections newest-first.
    pub fn list_incidents(&self) -> Result<Vec<Incident>, SpoolError> {
        let mut incidents = json_files(&self.root.join("incidents"))?
            .into_iter()
            .map(|path| read_json::<Incident>(&path))
            .collect::<Result<Vec<_>, _>>()?;
        incidents.sort_by(|left, right| {
            right
                .opened_at_epoch_ms
                .cmp(&left.opened_at_epoch_ms)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(incidents)
    }

    fn tally_path(&self, process: &str) -> PathBuf {
        self.root
            .join(format!("tally-{}.json", hex::encode(process.as_bytes())))
    }

    fn append_intent(&self, intent: &Intent) -> Result<(), SpoolError> {
        let path = self.root.join("intents.log");
        let created = !path.exists();
        let mut bytes =
            serde_json::to_vec(intent).map_err(|error| SpoolError::Encode(error.to_string()))?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(FILE_MODE)
            .open(&path)
            .map_err(|error| io_error(&path, error))?;
        file.write_all(&bytes)
            .map_err(|error| io_error(&path, error))?;
        file.sync_all().map_err(|error| io_error(&path, error))?;
        if created {
            fsync_dir(&self.root)?;
        }
        Ok(())
    }
}

impl WitnessSink for Spool {
    fn intent(&mut self, intent: &Intent) -> Result<(), SinkError> {
        self.append_intent(intent).map_err(sink_error)?;
        self.flows
            .append(FlowRecord::Intent(intent.as_rea_intent(&self.scope)))
            .map_err(|error| sink_error(SpoolError::Flow(error.to_string())))?;
        Ok(())
    }

    fn witness(&mut self, witness: &DeathWitness) -> Result<String, SinkError> {
        let canonical = witness.canonical_bytes()?;
        let cid = witness.cid()?;
        let base = self.root.join("witnesses").join(&cid);

        // The JSON row becomes durable before the canonical blob it describes.
        let json =
            serde_json::to_vec(witness).map_err(|error| SinkError::Encode(error.to_string()))?;
        atomic_write(&base.with_extension("json"), &json).map_err(sink_error)?;
        atomic_write(&base.with_extension("cbor"), &canonical).map_err(sink_error)?;

        // The first write is the write-ahead fact. Only the verdict-filled address is the
        // one death event, so a two-write death never doubles the flow.
        if witness.verdict.is_some() {
            let event = witness
                .as_flow_event(&self.scope)
                .map_err(|error| SinkError::Encode(error.to_string()))?;
            self.flows
                .append(FlowRecord::Event(event))
                .map_err(|error| sink_error(SpoolError::Flow(error.to_string())))?;
        }
        Ok(cid)
    }

    fn incident(&mut self, incident: &Incident) -> Result<(), SinkError> {
        // Contract: call at most once after the incident is closed. Each call appends a fresh
        // Process snapshot; readers take the newest per Incident.id.
        let path = self
            .root
            .join("incidents")
            .join(format!("{}.json", incident.id));
        let already_closed = read_optional_json::<Incident>(&path)
            .map_err(sink_error)?
            .is_some_and(|stored| stored.closed.is_some());

        // Replacing this projection atomically keeps every witness CID reachable after a crash.
        atomic_json(&path, incident).map_err(sink_error)?;

        if !already_closed {
            if let Some(close) = incident
                .as_close_event(&self.scope)
                .map_err(|error| SinkError::Encode(error.to_string()))?
            {
                self.flows
                    .append(FlowRecord::Event(close))
                    .map_err(|error| sink_error(SpoolError::Flow(error.to_string())))?;
            }
        }
        let process = incident
            .as_rea_process(&self.scope)
            .map_err(|error| SinkError::Encode(error.to_string()))?;
        self.flows
            .append(FlowRecord::Process(process))
            .map_err(|error| sink_error(SpoolError::Flow(error.to_string())))?;
        Ok(())
    }

    fn passport(&mut self, passport: &Passport) -> Result<(), SinkError> {
        atomic_json(&self.root.join("passport.json"), passport).map_err(sink_error)
    }

    fn tally(&mut self, process: &str, tally: &DeathTally) -> Result<(), SinkError> {
        atomic_json(&self.tally_path(process), tally).map_err(sink_error)
    }

    fn load_tally(&self, process: &str) -> Result<Option<DeathTally>, SinkError> {
        read_optional_json(&self.tally_path(process)).map_err(sink_error)
    }

    fn load_passport(&self) -> Result<Option<Passport>, SinkError> {
        read_optional_json(&self.root.join("passport.json")).map_err(sink_error)
    }
}

fn create_private_directory(path: &Path) -> Result<(), SpoolError> {
    DirBuilder::new()
        .recursive(true)
        .mode(DIRECTORY_MODE)
        .create(path)
        .map_err(|error| io_error(path, error))?;
    fs::set_permissions(path, fs::Permissions::from_mode(DIRECTORY_MODE))
        .map_err(|error| io_error(path, error))
}

fn ensure_writable_directory(path: &Path) -> Result<(), SpoolError> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(path).map_err(|error| io_error(path, error))?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o222 == 0 {
        return Err(SpoolError::Unwritable);
    }
    Ok(())
}

fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), SpoolError> {
    let bytes = serde_json::to_vec(value).map_err(|error| SpoolError::Encode(error.to_string()))?;
    atomic_write(path, &bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SpoolError> {
    let temporary = temporary_path(path);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(FILE_MODE)
        .open(&temporary)
        .map_err(|error| io_error(&temporary, error))?;
    file.write_all(bytes)
        .map_err(|error| io_error(&temporary, error))?;
    file.sync_all()
        .map_err(|error| io_error(&temporary, error))?;
    fs::rename(&temporary, path).map_err(|error| io_error(path, error))?;
    let parent = path
        .parent()
        .ok_or_else(|| SpoolError::Data(format!("path has no parent: {}", path.display())))?;
    fsync_dir(parent)
}

fn fsync_dir(directory: &Path) -> Result<(), SpoolError> {
    let file = File::open(directory).map_err(|error| io_error(directory, error))?;
    file.sync_all().map_err(|error| io_error(directory, error))
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".tmp");
    PathBuf::from(name)
}

fn json_files(directory: &Path) -> Result<Vec<PathBuf>, SpoolError> {
    let entries = fs::read_dir(directory).map_err(|error| io_error(directory, error))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| io_error(directory, error))?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, SpoolError> {
    let bytes = fs::read(path).map_err(|error| io_error(path, error))?;
    serde_json::from_slice(&bytes).map_err(|error| SpoolError::Data(error.to_string()))
}

fn read_optional_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, SpoolError> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| SpoolError::Data(error.to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, error)),
    }
}

fn io_error(path: &Path, error: std::io::Error) -> SpoolError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        SpoolError::Unwritable
    } else {
        SpoolError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

fn flow_open_error(error: elohim_epr_rea::FabricError) -> SpoolError {
    match error {
        elohim_epr_rea::FabricError::Io(error)
            if error.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            SpoolError::Unwritable
        }
        other => SpoolError::Flow(other.to_string()),
    }
}

fn sink_error(error: SpoolError) -> SinkError {
    match error {
        SpoolError::Encode(message) => SinkError::Encode(message),
        SpoolError::Data(message) => SinkError::Data(message),
        other => SinkError::Io(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
    };

    use ark_core::{
        DeathTally, DeathWitness, EffectiveTier, ExitClass, Incident, IncidentClose, Intent,
        IntentAction, Passport, ProcessPassport, RestartVerdict, RuntimeScope, WitnessSink,
        PASSPORT_KIND, WITNESS_KIND,
    };
    use elohim_epr_rea::store::{FlowRecord, FlowStore, SidecarFlowStore};

    use super::*;

    fn scope() -> RuntimeScope {
        RuntimeScope::new(elohim_epr::cid::compute_cid(b"runtime-manifest"))
    }

    fn passport(updated_at_epoch_ms: u64) -> Passport {
        Passport {
            schema: 1,
            kind: PASSPORT_KIND.to_string(),
            manifest: elohim_epr::cid::compute_cid(b"runtime-manifest").to_string(),
            node: None,
            incarnation: 1,
            ark_version: "0.1.0".to_string(),
            processes: vec![ProcessPassport {
                name: "conductor".to_string(),
                artifact_sha256: "ab".repeat(32),
                artifact_path: "/bin/sh".to_string(),
                pid: Some(42),
                started_at_epoch_ms: Some(1_000),
                ready: false,
                effective_tier: EffectiveTier::None,
                deaths_in_window: 1,
            }],
            last_verdict: None,
            updated_at_epoch_ms,
        }
    }

    fn witness(died_at_epoch_ms: u64, verdict: Option<RestartVerdict>) -> DeathWitness {
        DeathWitness {
            schema: 1,
            kind: WITNESS_KIND.to_string(),
            incident: elohim_epr::cid::compute_cid(b"incident").to_string(),
            process: "conductor".to_string(),
            incarnation: 1,
            pid: 42,
            artifact_sha256: "ab".repeat(32),
            artifact_path: "/bin/sh".to_string(),
            started_at_epoch_ms: 1_000,
            died_at_epoch_ms,
            uptime_ms: died_at_epoch_ms - 1_000,
            exit: ExitClass::Signaled {
                signal: 9,
                core_dumped: false,
            },
            last_stderr: vec!["fatal".to_string()],
            last_stdout: vec!["Conductor ready.".to_string()],
            sample: None,
            last_intent: None,
            passport: passport(died_at_epoch_ms),
            verdict,
            refusal: None,
            bounded_by: None,
            pain: None,
        }
    }

    fn intent(at_epoch_ms: u64, reason: &str) -> Intent {
        Intent {
            at_epoch_ms,
            incarnation: 1,
            process: "conductor".to_string(),
            action: IntentAction::Spawn,
            reason: reason.to_string(),
        }
    }

    #[test]
    fn spool_writes_witness_row_before_blob_and_lists_newest_first() {
        let data_root = tempfile::tempdir().unwrap();
        let mut spool = Spool::open(data_root.path(), scope()).unwrap();
        let earlier = witness(2_000, Some(RestartVerdict::Stop));
        let later = witness(3_000, Some(RestartVerdict::Stop));

        let earlier_cid = spool.witness(&earlier).unwrap();
        let later_cid = spool.witness(&later).unwrap();

        let summaries = spool.list_witnesses().unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].cid, later_cid);
        assert_eq!(summaries[0].pid, 42);
        assert_eq!(summaries[0].died_at_epoch_ms, 3_000);
        assert_eq!(summaries[1].cid, earlier_cid);
        assert_eq!(spool.read_witness(&later_cid).unwrap(), later);
        assert_eq!(
            fs::read(
                data_root
                    .path()
                    .join("ark/witnesses")
                    .join(format!("{earlier_cid}.cbor")),
            )
            .unwrap(),
            earlier.canonical_bytes().unwrap()
        );
        assert!(fs::read_dir(data_root.path().join("ark/witnesses"))
            .unwrap()
            .all(|entry| entry.unwrap().path().extension().unwrap() != "tmp"));
        for directory in [
            "ark",
            "ark/witnesses",
            "ark/incidents",
            "ark/.eprfs",
            "ark/.eprfs/status",
        ] {
            assert_eq!(
                fs::metadata(data_root.path().join(directory))
                    .unwrap()
                    .mode()
                    & 0o777,
                0o700,
                "{directory}"
            );
        }
    }

    #[test]
    fn spool_reasserts_private_mode_on_existing_ark_directory() {
        if nix::unistd::geteuid().is_root() {
            return;
        }

        let data_root = tempfile::tempdir().unwrap();
        let ark = data_root.path().join("ark");
        fs::create_dir(&ark).unwrap();
        fs::set_permissions(&ark, fs::Permissions::from_mode(0o755)).unwrap();

        Spool::open(data_root.path(), scope()).unwrap();

        assert_eq!(fs::metadata(ark).unwrap().mode() & 0o777, 0o700);
    }

    #[test]
    fn read_witness_rejects_invalid_cid() {
        let data_root = tempfile::tempdir().unwrap();
        let spool = Spool::open(data_root.path(), scope()).unwrap();

        assert!(matches!(
            spool.read_witness("../passport"),
            Err(SpoolError::Data(_))
        ));
    }

    #[test]
    fn spool_intent_log_is_append_only_json_lines() {
        let data_root = tempfile::tempdir().unwrap();
        let mut spool = Spool::open(data_root.path(), scope()).unwrap();
        let first = intent(1_000, "first start");
        let second = intent(2_000, "restart");

        spool.intent(&first).unwrap();
        spool.intent(&second).unwrap();

        let contents = fs::read_to_string(data_root.path().join("ark/intents.log")).unwrap();
        let decoded: Vec<Intent> = contents
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(decoded, vec![first, second]);

        let flows = SidecarFlowStore::open(&data_root.path().join("ark")).unwrap();
        let projected: Vec<_> = flows
            .records()
            .unwrap()
            .into_iter()
            .filter_map(|(_, record)| match record {
                FlowRecord::Intent(intent) => Some(intent),
                _ => None,
            })
            .collect();
        assert_eq!(projected.len(), 2);
        assert!(projected.iter().all(|intent| {
            intent
                .resource_spec
                .classified_as
                .iter()
                .any(|tag| tag == "runtime:spawn")
        }));
    }

    #[test]
    fn two_witness_writes_emit_one_verdict_filled_death_event() {
        let data_root = tempfile::tempdir().unwrap();
        let mut spool = Spool::open(data_root.path(), scope()).unwrap();
        let write_ahead = witness(2_000, None);

        let write_ahead_cid = spool.witness(&write_ahead).unwrap();
        let mut decided = write_ahead;
        decided.verdict = Some(RestartVerdict::Stop);
        let decided_cid = spool.witness(&decided).unwrap();

        assert_ne!(write_ahead_cid, decided_cid);
        assert_eq!(spool.list_witnesses().unwrap().len(), 2);

        let flow_path = data_root.path().join("ark/.eprfs/status/flows.jsonl");
        assert!(flow_path.is_file());
        let flows = SidecarFlowStore::open(&data_root.path().join("ark")).unwrap();
        let events = flows.events().unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0]
            .1
            .classified_as
            .iter()
            .any(|tag| tag == "runtime:death"));
    }

    #[test]
    fn spool_round_trips_passport_and_tally() {
        let data_root = tempfile::tempdir().unwrap();
        let mut spool = Spool::open(data_root.path(), scope()).unwrap();
        let passport = passport(2_000);
        let tally = DeathTally::default();

        spool.passport(&passport).unwrap();
        spool.tally("conductor", &tally).unwrap();

        assert_eq!(spool.load_passport().unwrap(), Some(passport));
        assert_eq!(spool.load_tally("conductor").unwrap(), Some(tally));
    }

    #[test]
    fn spool_persists_every_incident_append_and_projects_processes() {
        let data_root = tempfile::tempdir().unwrap();
        let mut spool = Spool::open(data_root.path(), scope()).unwrap();
        let mut incident = Incident::open("conductor", 1_000, 1);
        incident
            .witnesses
            .push(elohim_epr::cid::compute_cid(b"write-ahead witness").to_string());

        spool.incident(&incident).unwrap();
        assert_eq!(spool.list_incidents().unwrap()[0].witnesses.len(), 1);

        incident
            .witnesses
            .push(elohim_epr::cid::compute_cid(b"decided witness").to_string());
        spool.incident(&incident).unwrap();
        assert_eq!(spool.list_incidents().unwrap()[0].witnesses.len(), 2);

        let flows = SidecarFlowStore::open(&data_root.path().join("ark")).unwrap();
        assert_eq!(flows.processes().unwrap().len(), 2);
    }

    #[test]
    fn repeated_closed_incident_appends_one_close_event() {
        let data_root = tempfile::tempdir().unwrap();
        let mut spool = Spool::open(data_root.path(), scope()).unwrap();
        let mut incident = Incident::open("conductor", 1_000, 1);
        incident.closed = Some(IncidentClose::Stopped { at_epoch_ms: 2_000 });

        spool.incident(&incident).unwrap();
        spool.incident(&incident).unwrap();

        let flows = SidecarFlowStore::open(&data_root.path().join("ark")).unwrap();
        let close_events = flows
            .events()
            .unwrap()
            .into_iter()
            .filter(|(_, event)| {
                event
                    .classified_as
                    .iter()
                    .any(|tag| tag == "runtime:incident-closed")
            })
            .count();
        assert_eq!(close_events, 1);
    }

    #[test]
    fn spool_refuses_unwritable_root() {
        if nix::unistd::geteuid().is_root() {
            return;
        }

        let data_root = tempfile::tempdir().unwrap();
        fs::set_permissions(data_root.path(), fs::Permissions::from_mode(0o500)).unwrap();

        let result = Spool::open(data_root.path(), scope());

        fs::set_permissions(data_root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(result, Err(SpoolError::Unwritable)));
    }
}
