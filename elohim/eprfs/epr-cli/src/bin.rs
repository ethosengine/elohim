//! Thin command adapter for the device-runtime library.
use elohim_device_runtime::{
    bin_store::{self, IngestOptions, PeerServing, Refusal, Registry, Store, Transition},
    manifest_store::ManifestStore,
    session,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

fn argument(detail: impl Into<String>) -> Refusal {
    Refusal {
        kind: "arguments".into(),
        detail: detail.into(),
    }
}
fn land_transition(
    flags: &BTreeSet<&str>,
    options: &BTreeMap<&str, &str>,
    expected: &str,
) -> Result<Transition, Refusal> {
    let rebase = flags.contains("--rebase-lineage");
    let revert = flags.contains("--revert");
    if rebase && revert {
        return Err(argument(
            "--rebase-lineage and --revert are mutually exclusive",
        ));
    }
    if rebase {
        if expected == "none" {
            return Err(argument(
                "--rebase-lineage requires --expect <current-cid>, not none",
            ));
        }
        let reason = options
            .get("--reason")
            .filter(|reason| !reason.trim().is_empty())
            .ok_or_else(|| argument("--rebase-lineage requires --reason <text>"))?;
        return Ok(Transition::RebaseLineage {
            reason: (*reason).to_owned(),
        });
    }
    Ok(if revert {
        Transition::Revert
    } else {
        Transition::Forward
    })
}
/// Parse, call the device library, and render one bin command. Returns a process exit code.
pub fn run(args: &[String]) -> i32 {
    // Stdout is this verb's JSON; a restart after a landing speaks on stderr.
    crate::device::supervise::say_on_stderr();
    match execute_with(args, true) {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            2
        }
    }
}
/// Run one bin verb without the restart after a landing: arrival asks the device ark itself
/// (see `device::arrival_work::after_moves`).
pub(crate) fn execute(args: &[String]) -> Result<String, Refusal> {
    execute_with(args, false)
}

/// `restart`: a `follow` or forward `land` that moved current runs the hand command `epr device
/// supervise` for the person (row 35; see [`restart_after_landing`]).
fn execute_with(args: &[String], restart: bool) -> Result<String, Refusal> {
    let mut options = BTreeMap::new();
    let mut flags = BTreeSet::new();
    let mut positional = Vec::new();
    let mut rest = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--" {
            rest.extend(iter.cloned());
            break;
        }
        match arg.as_str() {
            "--repair"|"--json"|"--revert"|"--rebase-lineage"|"--current"|"--pin"|"--provisional"=>{ flags.insert(arg.as_str()); },
            "--tool"|"--registry"|"--store"|"--repo"|"--source-commit"|"--gate-run"|"--tree-clean"|"--commit"|"--expect"|"--reason"|"--invoked"|"--cid"|"--closure"|"--launcher"|"--destination"=>{ options.insert(arg.as_str(),iter.next().ok_or_else(|| argument(format!("{arg} needs a value")))?.as_str()); },
            "--help"|"-h"=>return Ok("epr bin <ingest|land|pin|resolve|status|gc|bootstrap|follow|shadows|source-state|launch|install-launcher|migrate> --tool <name> [--registry PATH] [--store PATH] [--json]; land requires --expect <cid|none> [--revert | --rebase-lineage --reason <text>]".into()),
            _ if arg.starts_with('-')=>return Err(argument(format!("unknown flag {arg}"))),
            _=>positional.push(arg.as_str()),
        }
    }
    let verb = *positional
        .first()
        .ok_or_else(|| argument("missing bin verb"))?;
    let registry = if let Some(path) = options.get("--registry") {
        Registry::load(Path::new(path))?
    } else {
        Registry::discover()?
    };
    if verb == "launch" {
        bin_store::launch(
            &registry,
            Path::new(
                options
                    .get("--invoked")
                    .ok_or_else(|| argument("launch requires --invoked"))?,
            ),
            &rest,
        )?;
        return Ok(String::new());
    }
    let tool = options
        .get("--tool")
        .ok_or_else(|| argument("--tool is required"))?;
    let store = if let Some(path) = options.get("--store") {
        Store::new(path, tool, &registry)?
    } else {
        Store::from_environment(tool, &registry)?
    };
    let store = store.with_runner(std::sync::Arc::new(crate::git::DeviceRunner));
    // Only the verbs that land read the device manifest for the landing policy.
    let store = if matches!(verb, "land" | "follow" | "bootstrap") {
        store.with_peer_serving(peer_serving_policy(
            crate::device::declaration::store(None, None, None),
            tool,
        ))
    } else {
        store
    };
    let repo = || store.repo_root(options.get("--repo").map(Path::new));
    let identity = session::resolve();
    let closure = options.get("--closure").map(Path::new);
    let value: Value = match verb {
        "ingest" => {
            let source = closure
                .or_else(|| positional.get(1).map(Path::new))
                .ok_or_else(|| argument("ingest requires binary or --closure PATH"))?;
            let clean = options
                .get("--tree-clean")
                .map(|v| match v.to_ascii_lowercase().as_str() {
                    "true" | "1" | "yes" => Ok(true),
                    "false" | "0" | "no" => Ok(false),
                    _ => Err(argument("--tree-clean expects true or false")),
                })
                .transpose()?;
            json!(store.ingest(
                source,
                &IngestOptions {
                    repo: options.get("--repo").map(PathBuf::from),
                    source_commit: options.get("--source-commit").map(|s| s.to_string()),
                    gate_run: options.get("--gate-run").map(|s| s.to_string()),
                    tree_clean: clean,
                    closure: closure.is_some()
                }
            )?)
        }
        "land" => {
            let expected = options
                .get("--expect")
                .copied()
                .ok_or_else(|| argument("land requires --expect <cid|none>"))?;
            let transition = land_transition(&flags, &options, expected)?;
            let cid = positional
                .get(1)
                .or_else(|| options.get("--cid"))
                .ok_or_else(|| argument("land requires cid"))?;
            judge_held(&store, cid)?;
            let repo = repo()?;
            // Only a forward landing restarts the device; a revert or a lineage rebase never.
            let restarts = restart && transition == Transition::Forward;
            let before = current_cid(&store);
            let mut value = json!(store.land_expected(
                cid,
                options
                    .get("--commit")
                    .ok_or_else(|| argument("land requires --commit"))?,
                &repo,
                transition,
                (expected != "none").then_some(expected)
            )?);
            if restarts {
                with_restart(
                    &mut value,
                    restart_after_landing(&store, tool, "land", before, &repo),
                );
            }
            value
        }
        "pin" => {
            if let Some(cid) = options.get("--cid") {
                judge_held(&store, cid)?;
            }
            json!(store.pin(
                &identity,
                options.get("--cid").copied(),
                flags.contains("--provisional"),
                options.get("--repo").map(Path::new)
            )?)
        }
        "resolve" => {
            let result = if flags.contains("--current") {
                store.resolve_current(None)?
            } else {
                store.resolve(None, Some(&identity), &|k| std::env::var(k).ok(), false)?
            };
            if !flags.contains("--json") {
                return Ok(result.path.display().to_string());
            }
            json!(result)
        }
        "status" => {
            if flags.contains("--repair") {
                store.repair_heads()?;
            }
            let status = store.status(
                &identity,
                options.get("--commit").copied(),
                flags.contains("--pin"),
                &|k| std::env::var(k).ok(),
            )?;
            if !flags.contains("--json") {
                return Ok(format!(
                    "session: {} cid={}\nhead: {}\nstate: {}\nresolved: {}",
                    status.session.as_deref().unwrap_or("none"),
                    status.session_cid.as_deref().unwrap_or("none"),
                    status.head.as_deref().unwrap_or("none"),
                    if status.behind {
                        "behind"
                    } else {
                        &status.source
                    },
                    status
                        .resolved
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "none".into())
                ));
            }
            json!(status)
        }
        "gc" => store.gc(&session::proc_root(), None)?,
        "shadows" => json!(store.shadows(&std::env::var("PATH").unwrap_or_default())),
        "source-state" => store.source_state(&repo()?)?,
        "bootstrap" => {
            let repo = repo()?;
            // The first arrival of a device that cannot build is the case the draw is for.
            let drawn = if closure.is_none() {
                sibling::before_landing(&store, tool, &repo)?
            } else {
                sibling::Drawn::default()
            };
            match store.bootstrap(&repo, closure) {
                Ok(release) => {
                    let mut value = json!(release);
                    if let (Some(record), Some(map)) = (drawn.record, value.as_object_mut()) {
                        map.insert("sibling".into(), record);
                    }
                    value
                }
                Err(refusal) => return Err(drawn.explain(refusal)),
            }
        }
        "follow" => {
            let head = crate::device::declaration::store(None, None, None)
                .map_err(argument)?
                .read()?;
            match head
                .follows
                .iter()
                .find(|f| f.channel == format!("bin:{tool}"))
            {
                Some(follow) => {
                    let repo = repo()?;
                    let before = current_cid(&store);
                    // A closure the caller supplied is the bytes to follow; nothing is drawn.
                    let drawn = if follow.mode.applies() && closure.is_none() {
                        sibling::before_landing(&store, tool, &repo)?
                    } else {
                        sibling::Drawn::default()
                    };
                    let mut value = match store.follow_declared(&repo, closure, follow) {
                        Ok(mut value) => {
                            if let (Some(record), Some(map)) = (drawn.record, value.as_object_mut())
                            {
                                map.insert("sibling".into(), record);
                            }
                            value
                        }
                        Err(refusal) => return Err(drawn.explain(refusal)),
                    };
                    if restart {
                        with_restart(
                            &mut value,
                            restart_after_landing(&store, tool, "follow", before, &repo),
                        );
                    }
                    value
                }
                None => {
                    serde_json::json!({"followed":false,"why":"device does not follow this tool"})
                }
            }
        }
        "migrate" => store.migrate()?,
        "install-launcher" => {
            if !options.contains_key("--registry") {
                let bins = store
                    .root()
                    .parent()
                    .map(|root| root.join("bin/bin-tools.json"));
                crate::device::declaration::store_with_bins(None, None, None, bins)
                    .map_err(argument)?
                    .apply()?;
            }
            let native = match options.get("--launcher") {
                Some(path) => PathBuf::from(path),
                None => store.landed_launcher()?,
            };
            let destination = options
                .get("--destination")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var("CARGO_HOME").unwrap_or_else(|_| {
                        format!("{}/.cargo", std::env::var("HOME").unwrap_or_default())
                    }))
                    .join("bin")
                });
            store.install_launcher(&native, &destination, &registry, &repo()?)?
        }
        _ => return Err(argument(format!("unknown bin verb {verb}"))),
    };
    serde_json::to_string(&value).map_err(|e| argument(e.to_string()))
}

/// The committed current the store records, if any.
fn current_cid(store: &Store) -> Option<String> {
    std::fs::read(store.root().join("current.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|record| record["cid"].as_str().map(str::to_owned))
}

/// After a follow or forward land: when it moved current, run the hand command for the person,
/// and return what the restart did for the landing's record. A landing's exit status stays the
/// landing's (row 35, F1): a caller that parses the record keeps the landing, whatever the
/// daemon did, and a degraded device is said on stderr with its cure, written to the receipt
/// and read by `epr device status`.
fn restart_after_landing(
    store: &Store,
    tool: &str,
    trigger: &str,
    before: Option<String>,
    repo: &Path,
) -> Option<Value> {
    let after = current_cid(store);
    if after.is_none() || after == before {
        return None;
    }
    use crate::device::supervise::{after_landed, AfterLanding};
    match after_landed(repo, trigger, tool, after.as_deref()) {
        AfterLanding::Untouched => None,
        AfterLanding::Reported(report) => Some(report.json()),
    }
}

/// Put the restart's report into a landing's record as `restart`.
fn with_restart(record: &mut Value, restart: Option<Value>) {
    if let (Some(map), Some(restart)) = (record.as_object_mut(), restart) {
        map.insert("restart".into(), restart);
    }
}

/// The cure for a sibling's release whose C this device cannot vouch for.
const ISA_C_CURE: &str =
    "the producer rebuilds under the declared target, or build for this device";

/// Before an explicit `land <cid>` or `pin --cid <cid>`: a held release drawn from a sibling
/// runs its own `epr doctor --exercise`, and a signal death, a timeout or any non-zero exit
/// refuses it. Executing the held bytes needs no sibling record, so the judgement survives a
/// restart.
///
/// Nothing is executed before the store's own verification (`Store::verified`, which hashes
/// every bin against the record and the record against its release CID) accepts the release: a
/// damaged or replaced file is left for the store to refuse. Provenance is read from the typed
/// record (`bin_store::Release`), and the release is judged when its original ingest or any later
/// one came from a sibling (`gateRun` `sibling:<device>`). A release only this device built is
/// not judged here.
fn judge_held(store: &Store, cid: &str) -> Result<(), Refusal> {
    if !store.verified(cid) {
        return Ok(());
    }
    let dir = store.root().join(cid);
    let Some(release) = std::fs::read(dir.join("record.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<bin_store::Release>(&bytes).ok())
    else {
        return Ok(());
    };
    let Some(gate) = std::iter::once(&release.provenance)
        .chain(&release.sources)
        .filter_map(|source| source.gate_run.as_deref())
        .find(|gate| gate.starts_with("sibling:"))
    else {
        return Ok(());
    };
    let refused = |why: String| Refusal {
        kind: "sibling-exercise-failed".into(),
        detail: format!(
            "release {cid} was drawn from a sibling ({gate}) and {why}; nothing was landed or \
             pinned; cure: {ISA_C_CURE}"
        ),
    };
    if !release.bins.contains_key("epr") {
        return Err(refused("holds no epr to exercise".into()));
    }
    crate::exercise::judge(&dir.join("epr"), crate::exercise::TIMEOUT)
        .map_err(|verdict| refused(format!("its own {verdict}")))
}

/// Whether `manifest` declares a supervised `peer serve` running `tool`'s landed release: a
/// process on the tool's channel whose arguments begin `peer serve` (not `--check`).
pub(crate) fn declares_peer_serve(manifest: &ark_core::RuntimeManifest, tool: &str) -> bool {
    let channel = format!("bin:{tool}");
    manifest.processes.iter().any(|process| {
        matches!(&process.artifact, ark_core::ArtifactRef::Channel { channel_id } if *channel_id == channel)
            && process.argv.get(1..3) == Some(&["peer".to_owned(), "serve".to_owned()][..])
            && !process.argv.iter().any(|arg| arg == "--check")
    })
}

/// The landing policy the device's current manifest head sets for `tool`. An unreadable head
/// keeps the tolerance and says so: landing never fails on the manifest it cannot read.
pub(crate) fn peer_serving_policy(store: Result<ManifestStore, String>, tool: &str) -> PeerServing {
    match store.and_then(|store| store.read().map_err(|error| error.to_string())) {
        Ok(head) if declares_peer_serve(&head, tool) => PeerServing::Required,
        Ok(_) => PeerServing::Tolerated,
        Err(detail) => {
            eprintln!(
                "{tool} landing: device manifest unreadable ({detail}); a candidate whose peer \
                 check is unsupported or finds no device still lands with a note"
            );
            PeerServing::Tolerated
        }
    }
}

/// Open the epr custody store shared by daemon and device status.
pub(crate) fn daemon_store(
) -> Result<elohim_device_runtime::bin_store::Store, elohim_device_runtime::bin_store::Refusal> {
    elohim_device_runtime::bin_store::Store::from_environment(
        "epr",
        &elohim_device_runtime::bin_store::Registry::discover()?,
    )
}

/// Process table used to observe daemon owner lifetimes.
pub(crate) fn daemon_proc_root() -> PathBuf {
    session::proc_root()
}

/// Follow-from-sibling: before `bin follow` compiles epr, take the release an admitted sibling
/// landed for the same commit, drawn by exact output CID through this device's `epr peer serve`.
/// The sibling's word is never the authority: the drawn bytes must make the release CID its
/// Produce named before anything is ingested, and ingest then verifies them like any other bytes.
pub(crate) mod sibling {
    #[cfg(feature = "peer")]
    use super::ISA_C_CURE;
    use super::{Refusal, Store};
    use serde_json::Value;
    use std::path::Path;
    #[cfg(feature = "peer")]
    use std::time::{Duration, Instant};

    /// The knob that makes a host that can build draw from a sibling first.
    #[cfg(feature = "peer")]
    pub(crate) const FOLLOW_SIBLING_ENV: &str = "EPR_FOLLOW_SIBLING";
    /// The bins one sibling must hold for one release before it is taken.
    #[cfg(feature = "peer")]
    pub(crate) const BINS: [&str; 2] = ["epr", "bin-launch"];
    /// The cure for an offer whose statement cannot be read here.
    #[cfg(feature = "peer")]
    const RE_PUT: &str = "the sibling re-puts the release with an epr that writes env:requires-doc";

    /// The instruction-set baseline a release's C is built at, by Rust target triple (row 29.2:
    /// C compiles at the baseline the Rust target already has). A sibling's bin is drawn only
    /// when its Produce's `env:isa-c` equals the entry for this epr's own target; a target with
    /// no entry draws nothing from a sibling and builds for itself.
    #[cfg(feature = "peer")]
    const C_BASELINES: [(&str, &str); 1] = [("x86_64-unknown-linux-gnu", "x86-64")];

    /// The target triple this epr was compiled for, exactly as cargo named it to `build.rs`.
    /// Never synthesised from `cfg` predicates: GNU x32 (`x86_64-unknown-linux-gnux32`) also
    /// satisfies x86_64, linux and gnu, and must not read as the x86_64 baseline.
    #[cfg(feature = "peer")]
    pub(crate) const OWN_TARGET: &str = env!("EPR_CLI_TARGET");

    /// The baseline [`C_BASELINES`] holds for `target`, with the triple it was found under.
    #[cfg(feature = "peer")]
    pub(crate) fn c_baseline(target: Option<&str>) -> Option<(&'static str, &'static str)> {
        C_BASELINES
            .into_iter()
            .find(|(triple, _)| Some(*triple) == target)
    }

    /// Whether `stated`, one bin's `env:isa-c`, is the baseline this device knows for `target`.
    /// Missing, unknown and different values are each an unmeasured requirement: no CPU level
    /// is computed or compared, only the one declared value.
    #[cfg(feature = "peer")]
    pub(crate) fn isa_c_admits(
        bin: &str,
        offer: &Offer,
        stated: Option<&str>,
        target: Option<&str>,
    ) -> Result<(), String> {
        let whose = format!("{bin} of release {} from {}", offer.release, offer.device);
        let Some((triple, baseline)) = c_baseline(target) else {
            return Err(format!(
                "unmeasured-requirement: {whose} cannot be judged: this epr knows no C baseline \
                 for its target ({}), so it reads no env:isa-c as measured; cure: {ISA_C_CURE}",
                target.unwrap_or("unnamed")
            ));
        };
        match stated {
            Some(value) if value == baseline => Ok(()),
            Some(value) => Err(format!(
                "unmeasured-requirement: {whose} records env:isa-c={value}, not {baseline}, the \
                 C baseline this epr knows for {triple}, so the instruction set its C needs is \
                 unmeasured here; cure: {ISA_C_CURE}"
            )),
            None => Err(format!(
                "unmeasured-requirement: {whose} records no env:isa-c, so the instruction set \
                 its C was compiled at is unmeasured (this device draws only env:isa-c=\
                 {baseline}, the C baseline of {triple}); cure: {ISA_C_CURE}"
            )),
        }
    }

    /// What the draw left for the follow that runs after it.
    #[derive(Debug, Default)]
    pub(crate) struct Drawn {
        /// The draw, reported on the follow's result as `sibling`.
        pub record: Option<Value>,
        /// Why nothing was drawn, appended to a follow that then could not build.
        pub gap: Option<String>,
    }

    impl Drawn {
        /// `refusal`, with why nothing was drawn when that is part of why it could not land.
        pub(crate) fn explain(self, mut refusal: Refusal) -> Refusal {
            if let Some(gap) = self.gap {
                refusal.detail = format!("{}; sibling: {gap}", refusal.detail);
            }
            refusal
        }
    }

    #[cfg(feature = "peer")]
    pub(crate) fn refused(kind: &str, detail: impl Into<String>) -> Refusal {
        Refusal {
            kind: kind.into(),
            detail: detail.into(),
        }
    }

    /// Whether `bin follow --tool <tool>` draws from a sibling before building, with the one line
    /// that says which path and why; `None` for a tool with no sibling path.
    #[cfg(feature = "peer")]
    pub(crate) fn path(tool: &str, can_build: bool, knob: Option<&str>) -> Option<(bool, String)> {
        if tool != "epr" {
            return None;
        }
        Some(match (can_build, knob == Some("1")) {
            (false, _) => (
                true,
                "follow: drawing epr from a sibling first: this host resolves no cargo pool".into(),
            ),
            (true, true) => (
                true,
                format!("follow: drawing epr from a sibling first: {FOLLOW_SIBLING_ENV}=1"),
            ),
            (true, false) => (
                false,
                format!(
                    "follow: building epr locally: this host resolves a cargo pool \
                     ({FOLLOW_SIBLING_ENV}=1 draws from a sibling first)"
                ),
            ),
        })
    }

    /// One sibling's landed release for a commit: every bin of [`BINS`] by output CID.
    #[cfg(feature = "peer")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct Offer {
        pub device: String,
        pub release: String,
        pub outputs: std::collections::BTreeMap<String, String>,
        /// What each bin's Produce stated it needs, read before anything is fetched.
        pub requires: std::collections::BTreeMap<String, Stated>,
        pub at: String,
    }

    /// One bin's statement of what it needs: the `env:requires` CID and the document itself
    /// as it crossed inline (`env:requires-doc`). Both are the sibling's word until checked.
    #[cfg(feature = "peer")]
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub(crate) struct Stated {
        pub cid: Option<String>,
        pub doc: Option<String>,
        /// The Produce's `env:gaps`: what its recorder could not measure.
        pub gaps: Vec<String>,
        /// The Produce's `env:isa-c`: the C baseline its producer checked the build against.
        pub isa_c: Option<String>,
    }

    /// The requirement axes a recorder's gap can leave unmeasured: a gap named `<axis>` or
    /// `<axis>.<detail>` (such as `isa.x86.partial-decode`) means the statement says nothing
    /// trustworthy about that axis, however well its other clauses read.
    #[cfg(all(test, feature = "peer"))]
    pub(crate) fn unmeasured_axis_for_test(gap: &str) -> Option<&'static str> {
        unmeasured_axis(gap)
    }

    #[cfg(feature = "peer")]
    fn unmeasured_axis(gap: &str) -> Option<&'static str> {
        use host_contract::names::axis;
        [
            axis::ARCH,
            axis::OS,
            axis::ISA_X86,
            axis::KERNEL_LINUX,
            axis::ABI_LIBC,
            axis::ABI_LOADER,
            axis::LIB,
        ]
        .into_iter()
        .find(|name| {
            gap.strip_prefix(name).is_some_and(|rest| {
                // `<axis>.observed.*` is a gap in what the bytes were seen to use, beside a
                // declared requirement that stays measured (extract_declared.rs).
                (rest.is_empty() || rest.starts_with('.')) && !rest.starts_with(".observed")
            })
        })
    }

    /// Whether `release` is one CID-shaped token: non-empty ASCII letters and digits only, so it
    /// can name a directory without leaving the one it is joined to.
    #[cfg(feature = "peer")]
    pub(crate) fn names_one_release(release: &str) -> bool {
        !release.is_empty() && release.bytes().all(|b| b.is_ascii_alphanumeric())
    }

    /// The newest offer [`offers`] lists.
    #[cfg(all(test, feature = "peer"))]
    pub(crate) fn select(
        rows: &[crate::peer::SiblingArtifact],
        me: &str,
        commit: &str,
    ) -> Option<Offer> {
        offers(rows, me, commit).into_iter().next()
    }

    /// The releases that could be drawn for `commit`, newest first: among admitted siblings'
    /// crossed Produces stamped with `env:release` and `env:release-bin`, every
    /// (device, release) that holds every bin. `me`'s own records are never offers. Admission
    /// is judged per offer afterwards, so a newer offer this host cannot run never hides an
    /// older one it can.
    #[cfg(feature = "peer")]
    pub(crate) fn offers(
        rows: &[crate::peer::SiblingArtifact],
        me: &str,
        commit: &str,
    ) -> Vec<Offer> {
        let mut offers = std::collections::BTreeMap::<(String, String), Offer>::new();
        for row in rows {
            let (Some(release), Some(bin)) = (&row.release, &row.release_bin) else {
                continue;
            };
            if row.device == me || row.commit != commit || !BINS.contains(&bin.as_str()) {
                continue;
            }
            // The release names a directory under `incoming`. It comes from a sibling's record,
            // so anything that is not one CID-shaped token is no offer.
            if !names_one_release(release) {
                continue;
            }
            let offer = offers
                .entry((row.device.clone(), release.clone()))
                .or_insert_with(|| Offer {
                    device: row.device.clone(),
                    release: release.clone(),
                    outputs: std::collections::BTreeMap::new(),
                    requires: std::collections::BTreeMap::new(),
                    at: String::new(),
                });
            // Rows arrive in flow order, so a later record of the same bin stands, and its
            // statement with it.
            offer.outputs.insert(bin.clone(), row.output.clone());
            offer.requires.insert(
                bin.clone(),
                Stated {
                    cid: row.requires.clone(),
                    doc: row.requires_doc.clone(),
                    gaps: row.gaps.clone(),
                    isa_c: row.isa_c.clone(),
                },
            );
            if row.at > offer.at {
                offer.at.clone_from(&row.at);
            }
        }
        let mut complete: Vec<Offer> = offers
            .into_values()
            .filter(|offer| BINS.iter().all(|bin| offer.outputs.contains_key(*bin)))
            .collect();
        // Newest first; among equally new, the lower release CID first, as before.
        complete.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.release.cmp(&b.release)));
        complete
    }

    /// Before `follow` or `bootstrap` lands: decide the path, say it, and draw when the path is
    /// sibling-first and no stored release already serves the checkout's HEAD. A gap falls back to the follow's own build; a sibling
    /// whose bytes do not make the release it named refuses the whole follow.
    #[cfg(feature = "peer")]
    pub(crate) fn before_landing(store: &Store, tool: &str, repo: &Path) -> Result<Drawn, Refusal> {
        let layout = elohim_device_runtime::layout::resolve();
        let can_build = layout.as_ref().is_ok_and(|l| l.cargo_pool().is_some());
        let knob = std::env::var(FOLLOW_SIBLING_ENV).ok();
        let Some((first, why)) = path(tool, can_build, knob.as_deref()) else {
            return Ok(Drawn::default());
        };
        // A host that builds and did not ask for a draw takes the path it always took: nothing
        // is read here that could refuse before the follow's own checks.
        if !first {
            eprintln!("{why}");
            return Ok(Drawn::default());
        }
        let commit = store.source_state(repo)?["head"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| refused("source-mismatch", "no source HEAD"))?;
        if store.holds_verified_for(&commit) {
            eprintln!("follow: epr for {commit} is already held; nothing to draw or build");
            return Ok(Drawn::default());
        }
        // Row 14: the follow lands only a published HEAD, so a draw for one that is not (or for
        // one a stale remote-tracking ref hides) is bytes paid for a refusal. Nothing is fetched.
        let published = match store.published_on(repo, &commit) {
            Ok(reference) => reference,
            Err(detail) => {
                let gap = format!("unpublished: {detail}");
                eprintln!("follow: nothing drawn from a sibling ({gap})");
                return Ok(Drawn {
                    record: None,
                    gap: Some(gap),
                });
            }
        };
        // Finding 5: say which ref made HEAD published, and when it is only a local branch.
        let local = if elohim_device_runtime::bin_store::is_local_ref(&published) {
            " (a local branch only: not on any remote-tracking publishedRef as of the last fetch)"
        } else {
            ""
        };
        eprintln!("{why}; {commit} is published on {published}{local}");
        let incoming = layout
            .ok()
            .and_then(|l| l.incoming().ok())
            .or_else(|| {
                store
                    .root()
                    .parent()
                    .map(|p| p.join(elohim_device_runtime::layout::INCOMING_DIR))
            })
            .ok_or_else(|| refused("layout-no-data-home", "no incoming directory to draw into"))?;
        let then = if can_build {
            "building locally"
        } else {
            "trying a local build"
        };
        match draw(store, repo, &commit, &incoming, then) {
            Ok(record) => Ok(Drawn {
                record: Some(record),
                gap: None,
            }),
            Err(Draw::Gap(gap)) => {
                eprintln!("follow: nothing drawn from a sibling ({gap}); {then}");
                Ok(Drawn {
                    record: None,
                    gap: Some(gap),
                })
            }
            // The wait already said who did not answer and for how long.
            Err(Draw::Unanswered(gap)) => Ok(Drawn {
                record: None,
                gap: Some(gap),
            }),
            Err(Draw::Refused(refusal)) => Err(refusal),
        }
    }

    #[cfg(not(feature = "peer"))]
    pub(crate) fn before_landing(
        _store: &Store,
        _tool: &str,
        _repo: &Path,
    ) -> Result<Drawn, Refusal> {
        Ok(Drawn::default())
    }

    #[cfg(feature = "peer")]
    pub(crate) enum Draw {
        /// Nothing was drawn and nothing was ingested; the follow may still build.
        Gap(String),
        /// Row 35 B: an admitted offer's holder did not answer for as long as this device waits
        /// ([`Patience`]). The waiting line has been said; this is the gap the follow carries.
        /// Nothing was ingested, and the follow may still build.
        Unanswered(String),
        /// The sibling's bytes are not what it promised, or the store refused them.
        Refused(Refusal),
    }

    /// Where the draw's patience is declared, read at the checkout's HEAD.
    #[cfg(feature = "peer")]
    const PATIENCE_POLICY: &str = "genesis/agentic/pool-policy.json";

    /// The patience when [`PATIENCE_POLICY`] declares none: about three times the slowest
    /// restart-to-ready measured when it was set (30 s and 33 s on shem, ~25 s on ethosengine).
    #[cfg(feature = "peer")]
    pub(crate) const DEFAULT_PATIENCE: Duration = Duration::from_secs(90);

    /// The first pause after a holder did not answer; each later pause doubles, up to
    /// [`PATIENCE_PAUSE_CAP`].
    #[cfg(feature = "peer")]
    const PATIENCE_PAUSE_FIRST: Duration = Duration::from_secs(2);

    #[cfg(feature = "peer")]
    const PATIENCE_PAUSE_CAP: Duration = Duration::from_secs(16);

    /// Row 35 B1: how long a draw waits for the holder of an offer this device already admitted
    /// (host admission passed, release named, statement verified) when the only failure is that
    /// the holder did not answer: a producer's peer restarts onto each landed release minutes
    /// before its siblings come to draw from it. `now` and `sleep` are the clock (tests inject
    /// their own so nothing really waits); `say` is where the waiting line goes; `then` is what
    /// the follow does when the wait runs out, as the fallback line says it.
    #[cfg(feature = "peer")]
    pub(crate) struct Patience<'a> {
        pub bound: Duration,
        pub now: &'a dyn Fn() -> Instant,
        pub sleep: &'a dyn Fn(Duration),
        pub say: &'a dyn Fn(&str),
        pub then: &'a str,
    }

    #[cfg(all(test, feature = "peer"))]
    impl Patience<'static> {
        /// No patience: an unanswered holder falls back at once.
        pub(crate) fn none() -> Patience<'static> {
            Patience {
                bound: Duration::ZERO,
                now: &Instant::now,
                sleep: &std::thread::sleep,
                say: &say_stderr,
                then: "building locally",
            }
        }
    }

    #[cfg(feature = "peer")]
    fn say_stderr(line: &str) {
        eprintln!("{line}");
    }

    /// `device_restart.draw_patience_seconds` in [`PATIENCE_POLICY`] at `repo`'s HEAD, read the
    /// way `declared_targets` is (`crate::git::command`, with this device's declared checkout
    /// trust); [`DEFAULT_PATIENCE`] when the record, the file or the key does not read.
    #[cfg(feature = "peer")]
    pub(crate) fn draw_patience(repo: &Path) -> Duration {
        crate::git::command(repo)
            .args(["show", &format!("HEAD:{PATIENCE_POLICY}")])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| serde_json::from_slice::<Value>(&output.stdout).ok())
            .and_then(|policy| policy["device_restart"]["draw_patience_seconds"].as_u64())
            .map_or(DEFAULT_PATIENCE, Duration::from_secs)
    }

    /// Whether a fetch error says only that the holder did not answer, so the same admitted
    /// offer may be asked again once it is back. That is the daemon's `no-route` gap, as
    /// `epr artifact fetch` prints it (`fetch gap: no-route: ...`): the holder refused the dial,
    /// could not be reached, did not connect within the dial's own bound, or its direct stream
    /// failed (a holder that restarts mid-draw hangs up). Every other kind is not a silent
    /// holder: `refused` and `manifest-rejected` are answers, `transport-cannot-deliver` is a
    /// relay too slow or a holder's error, `deadline-exceeded` is the fetch's own deadline or
    /// this device's daemon not answering the queue, `daemon-down` is this device's daemon,
    /// `local-store` this device's store, and the rest are about the record, not the holder.
    #[cfg(feature = "peer")]
    pub(crate) fn holder_unanswered(error: &str) -> bool {
        let silent = eprfs_peer::FetchGap::NoRoute {
            device: String::new(),
        };
        error
            .strip_prefix("fetch gap: ")
            .and_then(|rest| rest.split_once(": "))
            .is_some_and(|(kind, _)| kind == silent.kind())
    }

    /// One attempt's failure: a holder that did not answer, or the draw's own answer.
    #[cfg(feature = "peer")]
    enum Attempt {
        Unanswered(String),
        Draw(Draw),
    }

    #[cfg(feature = "peer")]
    impl From<Draw> for Attempt {
        fn from(draw: Draw) -> Attempt {
            Attempt::Draw(draw)
        }
    }

    #[cfg(feature = "peer")]
    fn draw(
        store: &Store,
        repo: &Path,
        commit: &str,
        incoming: &Path,
        then: &str,
    ) -> Result<Value, Draw> {
        let found = crate::peer::custody_artifacts(repo, None)
            .map_err(|e| Draw::Gap(format!("no-record: {e}")))?;
        let offers = offers(&found.rows, &found.device, commit);
        if offers.is_empty() {
            return Err(Draw::Gap(format!(
                "no-offer: no admitted sibling recorded a crossed Produce of epr and bin-launch \
                 stamped with a release for {commit}"
            )));
        }
        let host = ark_supervisor::host::observe_host();
        // Read before any wait; nothing the wait holds is a lock (the store takes and releases
        // its own around each ingest and release read, inside `take`).
        let patience = Patience {
            bound: draw_patience(repo),
            now: &Instant::now,
            sleep: &std::thread::sleep,
            say: &say_stderr,
            then,
        };
        draw_first(
            store,
            repo,
            commit,
            incoming,
            &offers,
            &Judges {
                admit: &|_bin, document| admit_on(&host, document),
                fetch: &|output, dest| {
                    crate::artifact::fetch_output(repo, output, dest).map_err(|e| e.to_string())
                },
                staged: &|bin, bytes| staged_admit(&host, bin, bytes),
                patience: &patience,
            },
        )
    }

    /// How a draw judges and moves bytes: `admit` judges an offer's verified statement on this
    /// host before anything is fetched, `fetch` copies an output CID to a destination, and
    /// `staged` judges what the fetched bytes themselves require before either is executed.
    /// `patience` is how long an admitted offer's silent holder is waited for.
    #[cfg(feature = "peer")]
    pub(crate) struct Judges<'a> {
        pub admit: &'a Admit<'a>,
        pub fetch: &'a dyn Fn(&str, &Path) -> Result<(), String>,
        pub staged: &'a Admit<'a>,
        pub patience: &'a Patience<'a>,
    }

    /// The residual's cheap half: the statement that admitted an offer is the producing
    /// sibling's own word, so before either drawn binary is executed (ingest runs
    /// `epr --version`), this device re-extracts what the staged bytes require and judges that
    /// on this host: libc, loader, kernel, libraries and architecture, which the bytes alone
    /// establish. isa.x86 is left out: what a host must have there is what the build declared
    /// (extract_declared.rs), already judged from the stated document, and the bytes'
    /// instruction union includes code behind runtime CPUID checks.
    #[cfg(feature = "peer")]
    fn staged_admit(host: &host_contract::Provides, bin: &str, bytes: &[u8]) -> Result<(), String> {
        let (document, _partial) = crate::artifact::staged_requires(Path::new(bin), bytes)?;
        let mut clauses: Vec<serde_json::Value> = serde_json::from_slice(&document)
            .map_err(|e| format!("re-extracted requirement is not a clause array: {e}"))?;
        clauses.retain(|clause| clause["axis"] != host_contract::names::axis::ISA_X86);
        let document = serde_json::to_vec(&clauses).map_err(|e| e.to_string())?;
        admit_on(host, &document)
    }

    /// Finding 3: judge `offers` (newest first) in turn and draw the first this host admits;
    /// nothing is fetched for an offer it does not. With none admissible, the gap is the newest
    /// offer's, followed by every older one's.
    #[cfg(feature = "peer")]
    pub(crate) fn draw_first(
        store: &Store,
        repo: &Path,
        commit: &str,
        incoming: &Path,
        offers: &[Offer],
        judges: &Judges<'_>,
    ) -> Result<Value, Draw> {
        let mut gaps = Vec::new();
        for offer in offers {
            match admissible(offer, judges.admit) {
                Ok(()) => return take_patiently(store, repo, commit, incoming, offer, judges),
                Err(gap) => gaps.push(gap),
            }
        }
        let mut gaps = gaps.into_iter();
        let newest = gaps
            .next()
            .unwrap_or_else(|| format!("no-offer: no release offered for {commit}"));
        let older: Vec<String> = gaps.collect();
        Err(Draw::Gap(if older.is_empty() {
            newest
        } else {
            format!(
                "{newest}; and {} older offer(s): {}",
                older.len(),
                older.join("; ")
            )
        }))
    }

    /// Judge one verified requirement document against this host's observation, with the
    /// envelope's own admission (what `ark host admit --requires` runs). Only the failed
    /// requirement leaves: the host's inventory is never printed or recorded.
    #[cfg(feature = "peer")]
    pub(crate) fn admit_on(host: &host_contract::Provides, document: &[u8]) -> Result<(), String> {
        use elohim_epr::verdict::{CheckOutcome, Decision};
        let requires: host_contract::Requires = serde_json::from_slice(document)
            .map_err(|e| format!("the requirement is not a clause array: {e}"))?;
        let verdict = ark_supervisor::admission::admit_recorded_requirements(Some(&requires), host);
        if verdict.decision == Decision::Permit {
            return Ok(());
        }
        let failed: Vec<String> = verdict
            .witness
            .checks
            .iter()
            .filter(|check| check.outcome == CheckOutcome::Failed)
            .map(|check| {
                let axis = check
                    .check_id
                    .strip_prefix("host-compatibility:")
                    .unwrap_or(&check.check_id);
                match check
                    .observed
                    .as_ref()
                    .and_then(|o| o.get("required"))
                    .map(|r| r.get("predicate").unwrap_or(r))
                {
                    Some(required) => format!("{axis} {} (requires {required})", check.summary),
                    // Admission demands some axes of every statement (arch, os).
                    None => format!(
                        "{axis} {}: the statement records no {axis} clause",
                        check.summary
                    ),
                }
            })
            .collect();
        Err(if failed.is_empty() {
            "admission declined without a failed check".into()
        } else {
            failed.join("; ")
        })
    }

    /// Judges one bin's verified requirement document on this host: `(bin, document)` to
    /// `Ok`, or the unmet requirement.
    #[cfg(feature = "peer")]
    pub(crate) type Admit<'a> = dyn Fn(&str, &[u8]) -> Result<(), String> + 'a;

    /// Row 12: whether this device can run every bin of `offer`, judged from what each bin's
    /// Produce stated, before a byte is fetched. `admit` judges one verified document on this
    /// host. A bin that stated nothing, whose statement did not cross, or whose statement does
    /// not make the CID its Produce named is not drawn: an honest draw never pays for bytes on
    /// an unread statement. Every answer here is a gap, so the follow may still build.
    #[cfg(feature = "peer")]
    pub(crate) fn admissible(offer: &Offer, admit: &Admit<'_>) -> Result<(), String> {
        for bin in BINS {
            let stated = offer.requires.get(bin).cloned().unwrap_or_default();
            let Some(cid) = stated.cid else {
                return Err(format!(
                    "unmeasured-requirement: {bin} of release {} from {} records no \
                     env:requires; cure: {RE_PUT}",
                    offer.release, offer.device
                ));
            };
            let Some(doc) = stated.doc else {
                return Err(format!(
                    "unmeasured-requirement: {bin} requirement {cid} did not cross with its \
                     Produce (offer predates requires-doc, or the document is over the inline \
                     cap); cure: {RE_PUT}"
                ));
            };
            let document = crate::artifact::requires_inline_doc(&cid, &doc).map_err(|why| {
                format!(
                    "unverifiable-requirement: {bin} of release {} from {}: {why}; cure: {RE_PUT}",
                    offer.release, offer.device
                )
            })?;
            // An honest recorder that could not measure an axis says so in env:gaps and leaves
            // the axis out of its statement; the remaining clauses passing proves nothing
            // about it (a partial ISA decode can hide an instruction this host lacks).
            if let Some((gap, axis)) = stated
                .gaps
                .iter()
                .find_map(|gap| unmeasured_axis(gap).map(|axis| (gap, axis)))
            {
                return Err(format!(
                    "unmeasured-requirement: {bin} of release {} from {} records gap {gap}, so \
                     its {axis} requirement is unmeasured; cure: build for this device",
                    offer.release, offer.device
                ));
            }
            // Row 29.2: the statement is read from the Rust-visible bytes, and the C a release
            // links can still need a newer CPU than its clauses say. Only the producer's checked
            // marker vouches for that, so a bin without it is never fetched.
            isa_c_admits(bin, offer, stated.isa_c.as_deref(), Some(OWN_TARGET))?;
            admit(bin, &document).map_err(|unmet| {
                format!(
                    "requirement-unmet: {bin} of release {} from {} cannot run here: {unmet}; \
                     cure: build for this device, or draw an offer built for it",
                    offer.release, offer.device
                )
            })?;
        }
        Ok(())
    }

    /// [`admissible`], then [`take`]: the offer's statement is read and judged on this host
    /// before `fetch` is called once. Test fixtures stage shell scripts, which no extraction
    /// reads, so this form accepts the staged bytes as fetched; [`draw_first`] takes the judge.
    #[cfg(all(test, feature = "peer"))]
    pub(crate) fn draw_offer(
        store: &Store,
        repo: &Path,
        commit: &str,
        incoming: &Path,
        offer: &Offer,
        admit: &Admit<'_>,
        fetch: &dyn Fn(&str, &Path) -> Result<(), String>,
    ) -> Result<Value, Draw> {
        draw_first(
            store,
            repo,
            commit,
            incoming,
            std::slice::from_ref(offer),
            &Judges {
                admit,
                fetch,
                staged: &|_, _| Ok(()),
                patience: &Patience::none(),
            },
        )
    }

    /// [`take`], asked again while `offer`'s holder does not answer, for as long as
    /// `judges.patience` allows: pauses of 2, 4, 8, then 16 s, the last cut to the bound. Only a
    /// fetch error [`holder_unanswered`] reads as a silent holder is waited for; every other gap,
    /// and a mismatch, is the answer at once. Nothing is held across a pause: `take` has removed
    /// what it staged, and the store's locks are taken and released inside each attempt.
    #[cfg(feature = "peer")]
    fn take_patiently(
        store: &Store,
        repo: &Path,
        commit: &str,
        incoming: &Path,
        offer: &Offer,
        judges: &Judges<'_>,
    ) -> Result<Value, Draw> {
        let patience = judges.patience;
        let mut pause = PATIENCE_PAUSE_FIRST;
        // When the holder was first found silent: the seconds waited are counted from there.
        let mut since: Option<Instant> = None;
        loop {
            let failed = match attempt(
                store,
                repo,
                commit,
                incoming,
                offer,
                judges.fetch,
                judges.staged,
            ) {
                Ok(value) => {
                    if let Some(since) = since {
                        (patience.say)(&format!(
                            "follow: holder {} not answering for release {}; waited {} s; drew it",
                            offer.device,
                            offer.release,
                            (patience.now)().saturating_duration_since(since).as_secs()
                        ));
                    }
                    return Ok(value);
                }
                Err(Attempt::Draw(draw)) => return Err(draw),
                Err(Attempt::Unanswered(gap)) => gap,
            };
            let since = *since.get_or_insert_with(|| (patience.now)());
            let waited = (patience.now)().saturating_duration_since(since);
            if waited >= patience.bound {
                let seconds = waited.as_secs();
                (patience.say)(&format!(
                    "follow: holder {} not answering for release {}; waited {seconds} s; {}",
                    offer.device, offer.release, patience.then
                ));
                return Err(Draw::Unanswered(format!(
                    "holder-not-answering: holder {} not answering for release {}; waited \
                     {seconds} s (device_restart.draw_patience_seconds {}); last: {failed}",
                    offer.device,
                    offer.release,
                    patience.bound.as_secs()
                )));
            }
            (patience.sleep)(pause.min(patience.bound - waited));
            pause = (pause * 2).min(PATIENCE_PAUSE_CAP);
        }
    }

    /// Draw `offer`'s bins with `fetch` (exact output CID to destination), refuse bytes that do
    /// not make the release it named, judge what the staged bytes require with `staged` before
    /// either is executed, and ingest them as the sibling's. One attempt, with no patience: the
    /// draw itself goes through [`take_patiently`]; tests take one attempt directly.
    #[cfg(all(test, feature = "peer"))]
    pub(crate) fn take(
        store: &Store,
        repo: &Path,
        commit: &str,
        incoming: &Path,
        offer: &Offer,
        fetch: &dyn Fn(&str, &Path) -> Result<(), String>,
        staged: &Admit<'_>,
    ) -> Result<Value, Draw> {
        attempt(store, repo, commit, incoming, offer, fetch, staged).map_err(
            |failed| match failed {
                Attempt::Unanswered(gap) => Draw::Gap(gap),
                Attempt::Draw(draw) => draw,
            },
        )
    }

    /// One [`take`], telling a holder that did not answer apart from every other gap.
    #[cfg(feature = "peer")]
    fn attempt(
        store: &Store,
        repo: &Path,
        commit: &str,
        incoming: &Path,
        offer: &Offer,
        fetch: &dyn Fn(&str, &Path) -> Result<(), String>,
        staged: &Admit<'_>,
    ) -> Result<Value, Attempt> {
        use std::os::unix::fs::PermissionsExt;
        if !names_one_release(&offer.release) {
            return Err(Draw::Gap(format!(
                "no-offer: {} named a release that is not one CID token",
                offer.device
            ))
            .into());
        }
        let dir = incoming.join(format!("sibling-{}", offer.release));
        // A previous partial draw is replaced, never trusted.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .map_err(|e| Draw::Gap(format!("local-store: {}: {e}", dir.display())))?;
        let started = std::time::Instant::now();
        let result = (|| -> Result<_, Attempt> {
            for (bin, output) in &offer.outputs {
                let dest = dir.join(bin);
                fetch(output, &dest).map_err(|error| {
                    if holder_unanswered(&error) {
                        Attempt::Unanswered(error)
                    } else {
                        Attempt::Draw(Draw::Gap(error))
                    }
                })?;
                std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))
                    .map_err(|e| Draw::Gap(format!("local-store: {}: {e}", dest.display())))?;
            }
            let fetched = store.release_cid_of(&dir).map_err(Draw::Refused)?;
            let mismatch = |made: &str| {
                Draw::Refused(refused(
                    "sibling-release-mismatch",
                    format!(
                        "{} offered release {} for {commit}; the drawn bytes make {made}",
                        offer.device, offer.release
                    ),
                ))
            };
            if fetched != offer.release {
                return Err(mismatch(&fetched).into());
            }
            for bin in offer.outputs.keys() {
                let path = dir.join(bin);
                let bytes = std::fs::read(&path)
                    .map_err(|e| Draw::Gap(format!("local-store: {}: {e}", path.display())))?;
                staged(bin, &bytes).map_err(|why| {
                    Draw::Gap(format!(
                        "statement-understated: {bin} of release {} from {}: what its drawn \
                         bytes require, re-extracted here before running either, is unmet: \
                         {why}; nothing was ingested",
                        offer.release, offer.device
                    ))
                })?;
            }
            // Row 29.2: before ingest executes either bin, the staged epr runs its own bundled C.
            // A signal death (an instruction this CPU lacks), a timeout or any non-zero exit (a
            // release from before the exercise has no such verb) leaves nothing ingested.
            let program = dir.join(BINS[0]);
            crate::exercise::judge(&program, crate::exercise::TIMEOUT).map_err(|verdict| {
                Draw::Gap(format!(
                    "exercise-failed: {} of release {} from {}: the staged bytes' {verdict}; \
                     nothing was ingested; cure: {ISA_C_CURE}",
                    BINS[0], offer.release, offer.device
                ))
            })?;
            let release = store
                .ingest(
                    &dir.join(BINS[0]),
                    &super::IngestOptions {
                        repo: Some(repo.into()),
                        source_commit: Some(commit.into()),
                        gate_run: Some(format!("sibling:{}", offer.device)),
                        tree_clean: Some(true),
                        closure: false,
                    },
                )
                // The bytes made the release the sibling named and this device's store still
                // would not take them (a sibling of another architecture cannot be probed here).
                // Nothing was ingested, so the follow may build.
                .map_err(|refusal| {
                    Draw::Gap(format!(
                        "store-refused: {}: {}",
                        refusal.kind, refusal.detail
                    ))
                })?;
            if release.cid != offer.release {
                return Err(mismatch(&release.cid).into());
            }
            Ok(release.provenance.gate_run)
        })();
        let _ = std::fs::remove_dir_all(&dir);
        let gate_run = result?;
        let seconds = started.elapsed().as_secs_f64();
        eprintln!(
            "follow: drew epr release {} from {} in {seconds:.1} s",
            offer.release, offer.device
        );
        Ok(serde_json::json!({
            "device": offer.device,
            "release": offer.release,
            "outputs": offer.outputs,
            "gateRun": gate_run,
            "seconds": seconds,
        }))
    }
}

#[cfg(all(test, feature = "peer"))]
#[path = "bin_draw_patience_tests.rs"]
mod draw_patience_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(values: &[&'static str]) -> BTreeSet<&'static str> {
        values.iter().copied().collect()
    }

    fn options(values: &[(&'static str, &'static str)]) -> BTreeMap<&'static str, &'static str> {
        values.iter().copied().collect()
    }

    #[test]
    fn land_arguments_refuse_rebase_lineage_with_revert() {
        let error = land_transition(
            &flags(&["--rebase-lineage", "--revert"]),
            &options(&[("--reason", "history was rewritten")]),
            "bafk-current",
        )
        .unwrap_err();

        assert_eq!(error.kind, "arguments");
        assert!(error.detail.contains("mutually exclusive"));
    }

    #[test]
    fn land_arguments_refuse_rebase_lineage_without_prior() {
        let error = land_transition(
            &flags(&["--rebase-lineage"]),
            &options(&[("--reason", "history was rewritten")]),
            "none",
        )
        .unwrap_err();

        assert_eq!(error.kind, "arguments");
        assert!(error.detail.contains("--expect"));
    }

    #[test]
    fn land_arguments_refuse_rebase_lineage_without_reason() {
        let error = land_transition(&flags(&["--rebase-lineage"]), &options(&[]), "bafk-current")
            .unwrap_err();

        assert_eq!(error.kind, "arguments");
        assert!(error.detail.contains("--reason"));
    }

    #[test]
    fn land_arguments_build_rebase_lineage_transition() {
        assert_eq!(
            land_transition(
                &flags(&["--rebase-lineage"]),
                &options(&[("--reason", "history was rewritten")]),
                "bafk-current",
            )
            .unwrap(),
            Transition::RebaseLineage {
                reason: "history was rewritten".into()
            }
        );
    }

    fn device_manifest(root: &Path) -> ManifestStore {
        crate::device::declaration::store(Some(root.to_path_buf()), None, None).unwrap()
    }

    fn run_epr(store: &ManifestStore, name: &str, args: &[&str]) {
        store
            .act(ark_core::declaration::ManifestAct::Run {
                tool: "epr".into(),
                stop: false,
                launch: ark_core::declaration::RunLaunch {
                    name: Some(name.into()),
                    args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                    ..Default::default()
                },
            })
            .unwrap();
    }

    #[test]
    fn landing_requires_peer_serving_only_when_the_head_declares_a_peer_process() {
        let tmp = tempfile::tempdir().unwrap();
        let store = device_manifest(tmp.path());
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "epr"),
            PeerServing::Tolerated,
            "a head with no processes keeps the tolerance"
        );

        run_epr(&store, "flow-report", &["flow", "report", "scope"]);
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "epr"),
            PeerServing::Tolerated,
            "an epr process that does not serve keeps the tolerance"
        );

        run_epr(&store, "peer", &["peer", "serve", "--listen", "0.0.0.0:0"]);
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "epr"),
            PeerServing::Required
        );
        // The policy is per tool: brit's landing does not judge epr's peer process.
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "brit"),
            PeerServing::Tolerated
        );
    }

    #[test]
    fn only_a_peer_serve_on_the_tools_own_channel_requires_peer_serving() {
        let tmp = tempfile::tempdir().unwrap();
        let base = device_manifest(tmp.path()).read().unwrap();
        let channel = |tool: &str| ark_core::ArtifactRef::Channel {
            channel_id: format!("bin:{tool}"),
        };
        let child = |artifact: ark_core::ArtifactRef, argv: &[&str]| ark_core::ChildSpec {
            name: "peer".into(),
            artifact,
            argv: argv.iter().map(|arg| (*arg).to_owned()).collect(),
            ..Default::default()
        };
        let declares = |spec: ark_core::ChildSpec| {
            let mut manifest = base.clone();
            manifest.processes = vec![spec];
            declares_peer_serve(&manifest, "epr")
        };

        assert!(declares(child(
            channel("epr"),
            &["{artifact}", "peer", "serve", "--root", "/r"]
        )));
        assert!(
            !declares(child(channel("brit"), &["{artifact}", "peer", "serve"])),
            "another tool's channel"
        );
        assert!(
            !declares(child(
                ark_core::ArtifactRef::Pinned {
                    cid: None,
                    sha256: "0".repeat(64),
                    bytes: None,
                },
                &["{artifact}", "peer", "serve"],
            )),
            "a pinned artifact does not follow the landed release"
        );
        assert!(
            !declares(child(
                channel("epr"),
                &["{artifact}", "peer", "serve", "--check"]
            )),
            "a check is not a daemon"
        );
        assert!(
            !declares(child(channel("epr"), &["peer", "serve"])),
            "`peer` at argv[0] is the program, not its first argument"
        );
    }

    #[test]
    fn an_unreadable_manifest_keeps_the_landing_tolerance() {
        let tmp = tempfile::tempdir().unwrap();
        let store = device_manifest(tmp.path());
        run_epr(&store, "peer", &["peer", "serve"]);
        std::fs::write(tmp.path().join("manifest/head"), b"not a head").unwrap();

        assert_eq!(
            peer_serving_policy(
                crate::device::declaration::store(Some(tmp.path().to_path_buf()), None, None),
                "epr"
            ),
            PeerServing::Tolerated
        );
        assert_eq!(
            peer_serving_policy(Err("no device key".into()), "epr"),
            PeerServing::Tolerated
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_host_draws_from_a_sibling_first_only_without_a_pool_or_with_the_knob() {
        use sibling::path;
        let (first, why) = path("epr", false, None).unwrap();
        assert!(first);
        assert!(why.contains("resolves no cargo pool"), "{why}");
        let (first, why) = path("epr", true, Some("1")).unwrap();
        assert!(first);
        assert!(why.contains("EPR_FOLLOW_SIBLING=1"), "{why}");
        for knob in [None, Some("0"), Some("yes"), Some("")] {
            let (first, why) = path("epr", true, knob).unwrap();
            assert!(!first, "knob {knob:?} must not draw on a host that builds");
            assert!(why.starts_with("follow: building epr locally"), "{why}");
        }
        assert_eq!(path("brit", false, Some("1")), None, "only epr has a path");
    }

    #[cfg(feature = "peer")]
    fn row(
        device: &str,
        commit: &str,
        release: &str,
        bin: &str,
        at: &str,
    ) -> crate::peer::SiblingArtifact {
        crate::peer::SiblingArtifact {
            record: format!("record-{device}-{release}-{bin}-{at}"),
            key: "key".into(),
            method: None,
            input: None,
            family: None,
            requires: None,
            requires_doc: None,
            device: device.into(),
            output: format!("output-{release}-{bin}"),
            manifest: "manifest".into(),
            bytes: 1,
            commit: commit.into(),
            release: (!release.is_empty()).then(|| release.into()),
            release_bin: (!bin.is_empty()).then(|| bin.into()),
            agent_id: None,
            gaps: Vec::new(),
            isa_c: None,
            at: at.into(),
            head_seq: None,
        }
    }

    /// The store's own identity probe answers like an epr release (as the launcher tests do).
    #[cfg(feature = "peer")]
    struct Identity;
    #[cfg(feature = "peer")]
    impl bin_store::Runner for Identity {
        fn run(&self, spec: &bin_store::CommandSpec) -> Result<bin_store::CommandOutput, Refusal> {
            Ok(bin_store::CommandOutput {
                success: true,
                stdout: json!({
                    "cid": bin_store::file_cid(Path::new(&spec.program))?,
                    "version": "fixture"
                })
                .to_string(),
                stderr: String::new(),
            })
        }
    }

    #[cfg(feature = "peer")]
    pub(super) fn two_bin_store(root: &Path) -> Store {
        let registry: Registry = serde_json::from_value(json!({"version":1,"tools":{"epr":{
            "bins":["epr","bin-launch"],"aliases":{},"longRunning":[],
            "candidateEnv":{"epr":["EPR_BIN"]}}}}))
        .unwrap();
        Store::new(root.join("store/epr"), "epr", &registry)
            .unwrap()
            .with_runner(std::sync::Arc::new(Identity))
    }

    #[cfg(feature = "peer")]
    pub(super) fn sibling_bins(dir: &Path, launcher: &str) {
        sibling_bins_with(dir, "#!/bin/sh\n# sibling epr\n", launcher);
    }

    /// A sibling's two bins, the epr one being `epr` (a script `doctor --exercise` runs).
    #[cfg(feature = "peer")]
    fn sibling_bins_with(dir: &Path, epr: &str, launcher: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(dir).unwrap();
        for (bin, body) in [("epr", epr), ("bin-launch", launcher)] {
            std::fs::write(dir.join(bin), body).unwrap();
            std::fs::set_permissions(dir.join(bin), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
    }

    #[cfg(feature = "peer")]
    pub(super) fn offer_for(release: &str) -> sibling::Offer {
        sibling::Offer {
            device: "did:key:z6MkSibling".into(),
            release: release.into(),
            outputs: [("bin-launch", "out-launch"), ("epr", "out-epr")]
                .into_iter()
                .map(|(b, o)| (b.to_string(), o.to_string()))
                .collect(),
            requires: std::collections::BTreeMap::new(),
            at: "2026-11-17T07:00:00Z".into(),
        }
    }

    /// A requirement document asking for glibc at least `min`, as `put` would record it: the
    /// `env:requires` CID and the inline `env:requires-doc` slot value.
    #[cfg(feature = "peer")]
    pub(super) fn glibc_at_least(min: &[u32]) -> sibling::Stated {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        use host_contract::{names::axis, AxisId, Clause, ClauseSource, Predicate, Requires};
        // As `put` records it: arch and os always (admission demands both), then the floor.
        let mut requires = Requires::default();
        for (name, predicate) in [
            (axis::ARCH, Predicate::Equal("x86_64".into())),
            (axis::OS, Predicate::Equal("linux".into())),
            (
                axis::ABI_LIBC,
                Predicate::AtLeast(host_contract::Version {
                    line: "glibc".into(),
                    components: min.to_vec(),
                }),
            ),
        ] {
            requires
                .insert(Clause {
                    axis: AxisId(name.into()),
                    predicate,
                    source: ClauseSource::Extracted,
                    narrowing: None,
                })
                .unwrap();
        }
        let bytes = serde_json::to_vec(&requires).unwrap();
        sibling::Stated {
            cid: Some(eprfs_core::BlobCid::compute_bytes(&bytes).to_string()),
            doc: Some(URL_SAFE_NO_PAD.encode(&bytes)),
            gaps: Vec::new(),
            // As a producer that checked its C at the x86_64 baseline records it.
            isa_c: Some("x86-64".into()),
        }
    }

    /// An x86_64 Linux host that observed nothing else.
    #[cfg(feature = "peer")]
    fn linux_host() -> host_contract::Provides {
        use host_contract::{names::axis, AxisId, AxisValue, Provides};
        let mut host = Provides::default();
        host.observe(
            AxisId(axis::ARCH.into()),
            AxisValue::Scalar("x86_64".into()),
        );
        host.observe(AxisId(axis::OS.into()), AxisValue::Scalar("linux".into()));
        host
    }

    /// An x86_64 Linux host that observed glibc `version`.
    #[cfg(feature = "peer")]
    pub(super) fn host_with_glibc(version: &[u32]) -> host_contract::Provides {
        use host_contract::{names::axis, AxisId, AxisValue, Version};
        let mut host = linux_host();
        host.observe(
            AxisId(axis::ABI_LIBC.into()),
            AxisValue::Version(Version {
                line: "glibc".into(),
                components: version.to_vec(),
            }),
        );
        host
    }

    /// `offer` with every bin stating `stated`.
    #[cfg(feature = "peer")]
    pub(super) fn stating(mut offer: sibling::Offer, stated: &sibling::Stated) -> sibling::Offer {
        offer.requires = sibling::BINS
            .iter()
            .map(|bin| ((*bin).to_owned(), stated.clone()))
            .collect();
        offer
    }

    #[cfg(feature = "peer")]
    #[test]
    fn an_offer_whose_statement_this_host_cannot_meet_is_refused_before_any_fetch() {
        // Row 12 probe: shem's epr needs glibc 2.39; a host on 2.31 fetches nothing.
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let landed = tmp.path().join("sibling");
        sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
        let release = store.release_cid_of(&landed).unwrap();
        let commit = "e".repeat(40);
        let incoming = tmp.path().join("incoming");
        let offer = stating(offer_for(&release), &glibc_at_least(&[2, 39]));
        let fetched = std::cell::Cell::new(0);
        let old = host_with_glibc(&[2, 31]);
        let result = sibling::draw_offer(
            &store,
            tmp.path(),
            &commit,
            &incoming,
            &offer,
            &|_, document| sibling::admit_on(&old, document),
            &|_, _| {
                fetched.set(fetched.get() + 1);
                Err("must not fetch".into())
            },
        );
        match result {
            Err(sibling::Draw::Gap(gap)) => {
                assert!(gap.starts_with("requirement-unmet: "), "{gap}");
                assert!(
                    gap.contains("abi.libc"),
                    "names the failed requirement: {gap}"
                );
                assert!(gap.contains(&release), "{gap}");
            }
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(r)) => panic!("a gap, not a refusal: {}", r.detail),
            Ok(value) => panic!("an offer this host cannot run was taken: {value}"),
        }
        assert_eq!(fetched.get(), 0, "nothing is fetched");
        assert!(!store.holds_verified_for(&commit));
        assert!(!incoming.exists(), "nothing is staged");

        // The same statement on a host that meets it is drawn and ingested.
        let new = host_with_glibc(&[2, 40]);
        let serve = serving(&landed);
        let drawn = match sibling::draw_offer(
            &store,
            tmp.path(),
            &commit,
            &incoming,
            &offer,
            &|_, document| sibling::admit_on(&new, document),
            &|output, dest| {
                fetched.set(fetched.get() + 1);
                serve(output, dest)
            },
        ) {
            Ok(value) => value,
            Err(sibling::Draw::Gap(gap)) => panic!("gap: {gap}"),
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(r)) => panic!("refused {}: {}", r.kind, r.detail),
        };
        assert_eq!(drawn["release"], release);
        assert_eq!(fetched.get(), 2, "both bins are fetched once admitted");
        assert!(store.holds_verified_for(&commit));
    }

    #[cfg(feature = "peer")]
    #[test]
    fn an_offer_whose_statement_did_not_cross_or_does_not_verify_is_not_fetched() {
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let incoming = tmp.path().join("incoming");
        let host = host_with_glibc(&[2, 99]);
        let good = glibc_at_least(&[2, 17]);
        let other = glibc_at_least(&[2, 18]);
        let cases = [
            (
                sibling::Stated::default(),
                "unmeasured-requirement: ",
                "records no env:requires",
            ),
            (
                sibling::Stated {
                    cid: good.cid.clone(),
                    doc: None,
                    gaps: Vec::new(),
                    isa_c: good.isa_c.clone(),
                },
                "unmeasured-requirement: ",
                "offer predates requires-doc",
            ),
            (
                // A document that makes another CID than the one its Produce named.
                sibling::Stated {
                    cid: good.cid.clone(),
                    doc: other.doc.clone(),
                    gaps: Vec::new(),
                    isa_c: good.isa_c.clone(),
                },
                "unverifiable-requirement: ",
                "not the env:requires",
            ),
            (
                sibling::Stated {
                    cid: good.cid.clone(),
                    doc: Some("not base64!".into()),
                    gaps: Vec::new(),
                    isa_c: good.isa_c.clone(),
                },
                "unverifiable-requirement: ",
                "base64url",
            ),
        ];
        for (stated, kind, why) in cases {
            let offer = stating(offer_for("R1"), &stated);
            let fetched = std::cell::Cell::new(0);
            match sibling::draw_offer(
                &store,
                tmp.path(),
                &"f".repeat(40),
                &incoming,
                &offer,
                &|_, document| sibling::admit_on(&host, document),
                &|_, _| {
                    fetched.set(fetched.get() + 1);
                    Err("must not fetch".into())
                },
            ) {
                Err(sibling::Draw::Gap(gap)) => {
                    assert!(
                        gap.starts_with(kind) && gap.contains(why),
                        "{stated:?}: {gap}"
                    );
                }
                Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
                Err(sibling::Draw::Refused(r)) => panic!("{stated:?}: refused {}", r.detail),
                Ok(value) => panic!("{stated:?}: taken {value}"),
            }
            assert_eq!(fetched.get(), 0, "{stated:?}: nothing is fetched");
        }
        // One bin stating its need is not enough: every bin must.
        let mut half = stating(offer_for("R1"), &good);
        half.requires.remove("bin-launch");
        let gap = sibling::admissible(&half, &|_, d| sibling::admit_on(&host, d)).unwrap_err();
        assert!(
            gap.contains("bin-launch") && gap.contains("records no env:requires"),
            "{gap}"
        );
        assert_eq!(
            sibling::admissible(&stating(offer_for("R1"), &good), &|_, d| {
                sibling::admit_on(&host, d)
            }),
            Ok(())
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn an_unobserved_axis_is_refused_not_assumed() {
        // A host that did not observe glibc cannot claim to meet a glibc floor.
        let document = {
            use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
            URL_SAFE_NO_PAD
                .decode(glibc_at_least(&[2, 17]).doc.unwrap())
                .unwrap()
        };
        let unmet = sibling::admit_on(&linux_host(), &document).unwrap_err();
        assert!(unmet.contains("abi.libc"), "{unmet}");
        assert!(
            !unmet.contains("arch") && !unmet.contains("os "),
            "only glibc fails: {unmet}"
        );
        // A statement missing an axis admission demands names it.
        let bare = br#"[]"#;
        let missing = sibling::admit_on(&host_with_glibc(&[2, 40]), bare).unwrap_err();
        assert!(missing.contains("arch missing-requirement"), "{missing}");
        assert!(sibling::admit_on(&host_with_glibc(&[2, 17]), &document).is_ok());
    }

    /// Serves `dir`'s bins by output CID, as a sibling's daemon would.
    #[cfg(feature = "peer")]
    pub(super) fn serving(dir: &Path) -> impl Fn(&str, &Path) -> Result<(), String> + '_ {
        move |output: &str, dest: &Path| {
            let bin = if output == "out-epr" {
                "epr"
            } else {
                "bin-launch"
            };
            std::fs::copy(dir.join(bin), dest)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_drawn_release_that_makes_its_promised_cid_is_ingested_as_the_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let landed = tmp.path().join("sibling");
        sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
        let release = store.release_cid_of(&landed).unwrap();
        let commit = "c".repeat(40);
        let incoming = tmp.path().join("incoming");
        assert!(!store.holds_verified_for(&commit));
        let serve = serving(&landed);
        let drawn = match sibling::take(
            &store,
            tmp.path(),
            &commit,
            &incoming,
            &offer_for(&release),
            &serve,
            &|_, _| Ok(()),
        ) {
            Ok(value) => value,
            Err(sibling::Draw::Gap(gap)) => panic!("gap: {gap}"),
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(r)) => panic!("refused {}: {}", r.kind, r.detail),
        };
        assert_eq!(drawn["release"], release);
        assert_eq!(drawn["gateRun"], "sibling:did:key:z6MkSibling");
        // The predicate `follow` and `bootstrap` land by without building.
        assert!(store.holds_verified_for(&commit));
        assert!(
            !incoming.join(format!("sibling-{release}")).exists(),
            "the draw leaves nothing in incoming"
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_drawn_release_whose_bytes_make_another_cid_is_refused_and_nothing_is_ingested() {
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let landed = tmp.path().join("sibling");
        sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
        let promised = store.release_cid_of(&landed).unwrap();
        let tampered = tmp.path().join("tampered");
        sibling_bins(&tampered, "#!/bin/sh\n# not the launcher it landed\n");
        let commit = "d".repeat(40);
        let incoming = tmp.path().join("incoming");
        let serve = serving(&tampered);
        match sibling::take(
            &store,
            tmp.path(),
            &commit,
            &incoming,
            &offer_for(&promised),
            &serve,
            &|_, _| Ok(()),
        ) {
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(refusal)) => {
                assert_eq!(refusal.kind, "sibling-release-mismatch");
                assert!(refusal.detail.contains(&promised), "{}", refusal.detail);
            }
            Err(sibling::Draw::Gap(gap)) => panic!("a mismatch is a refusal, not a gap: {gap}"),
            Ok(value) => panic!("tampered bytes were taken: {value}"),
        }
        assert!(!store.holds_verified_for(&commit), "nothing is ingested");
        let made = store.release_cid_of(&tampered).unwrap();
        assert!(!store.verified(&made) && !store.verified(&promised));
        assert!(!incoming.join(format!("sibling-{promised}")).exists());
    }

    #[cfg(feature = "peer")]
    #[test]
    fn the_offer_is_the_newest_complete_release_a_sibling_stamped_for_the_commit() {
        use sibling::select;
        const ME: &str = "did:key:me";
        const A: &str = "did:key:a";
        const B: &str = "did:key:b";
        const HEAD: &str = "c0ffee";
        let t1 = "2026-11-17T01:00:00Z";
        let t2 = "2026-11-17T02:00:00Z";
        let t3 = "2026-11-17T03:00:00Z";
        let rows = vec![
            // A's complete release R1 at HEAD.
            row(A, HEAD, "R1", "epr", t1),
            row(A, HEAD, "R1", "bin-launch", t1),
            // B's newer release R2 lacks its launcher: not an offer.
            row(B, HEAD, "R2", "epr", t3),
            // My own newest release is never an offer.
            row(ME, HEAD, "R3", "epr", t3),
            row(ME, HEAD, "R3", "bin-launch", t3),
            // Another commit, and an unstamped Produce, are not offers for HEAD.
            row(A, "other", "R4", "epr", t3),
            row(A, "other", "R4", "bin-launch", t3),
            row(A, HEAD, "", "", t3),
        ];
        let offer = select(&rows, ME, HEAD).expect("R1 is complete");
        assert_eq!((offer.device.as_str(), offer.release.as_str()), (A, "R1"));
        assert_eq!(offer.outputs["epr"], "output-R1-epr");
        assert_eq!(offer.outputs["bin-launch"], "output-R1-bin-launch");

        // Once B completes R2 later, the newest complete release wins.
        let mut more = rows.clone();
        more.push(row(B, HEAD, "R2", "bin-launch", t2));
        let offer = select(&more, ME, HEAD).unwrap();
        assert_eq!((offer.device.as_str(), offer.release.as_str()), (B, "R2"));

        // A later record of the same bin stands (flow order).
        let mut relabelled = rows.clone();
        let mut later = row(A, HEAD, "R1", "epr", t2);
        later.output = "output-R1-epr-again".into();
        later.requires = Some("req-cid".into());
        later.requires_doc = Some("req-doc".into());
        relabelled.push(later);
        let offer = select(&relabelled, ME, HEAD).unwrap();
        assert_eq!(offer.outputs["epr"], "output-R1-epr-again");
        // The statement travels with the output it was recorded beside.
        assert_eq!(
            offer.requires["epr"],
            sibling::Stated {
                cid: Some("req-cid".into()),
                doc: Some("req-doc".into()),
                gaps: Vec::new(),
                isa_c: None,
            }
        );
        assert_eq!(offer.requires["bin-launch"], sibling::Stated::default());

        assert_eq!(
            select(&rows, ME, "unknown"),
            None,
            "nothing for another commit"
        );
        assert_eq!(
            select(&rows[2..3], ME, HEAD),
            None,
            "an incomplete release is no offer"
        );

        // A release is a directory name under `incoming`: one that could leave it is no offer.
        for escaping in ["a/../../x", "..", "R1/x", "R 1", "R1\u{0}"] {
            let rows = vec![
                row(A, HEAD, escaping, "epr", t1),
                row(A, HEAD, escaping, "bin-launch", t1),
            ];
            assert_eq!(select(&rows, ME, HEAD), None, "{escaping:?} is no offer");
            assert!(!sibling::names_one_release(escaping));
        }
        assert!(sibling::names_one_release("bafyreiabc234"));
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_recorded_gap_on_a_requirement_axis_leaves_the_offer_undrawn() {
        // Finding 1: an honest producer whose ISA decode was partial says so in env:gaps and
        // records no ISA clause. A host meeting every remaining clause must still not fetch.
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let host = host_with_glibc(&[2, 99]);
        let mut stated = glibc_at_least(&[2, 17]);
        stated.gaps = vec![
            "build-inputs.build-scripts".into(),
            "isa.x86.partial-decode".into(),
        ];
        let offer = stating(offer_for("R1"), &stated);
        let fetched = std::cell::Cell::new(0);
        match sibling::draw_offer(
            &store,
            tmp.path(),
            &"a".repeat(40),
            &tmp.path().join("incoming"),
            &offer,
            &|_, document| sibling::admit_on(&host, document),
            &|_, _| {
                fetched.set(fetched.get() + 1);
                Err("must not fetch".into())
            },
        ) {
            Err(sibling::Draw::Gap(gap)) => {
                assert!(gap.starts_with("unmeasured-requirement: "), "{gap}");
                assert!(gap.contains("isa.x86.partial-decode"), "{gap}");
                assert!(gap.contains("isa.x86 requirement is unmeasured"), "{gap}");
            }
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(r)) => panic!("a gap, not a refusal: {}", r.detail),
            Ok(value) => panic!("a partial statement was drawn: {value}"),
        }
        assert_eq!(fetched.get(), 0, "nothing is fetched");
        // A gap about how the output was built, not what it needs, does not block the draw.
        let mut reach_only = glibc_at_least(&[2, 17]);
        reach_only.gaps = vec!["build-inputs.build-scripts".into()];
        assert_eq!(
            sibling::admissible(&stating(offer_for("R1"), &reach_only), &|_, d| {
                sibling::admit_on(&host, d)
            }),
            Ok(())
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_newer_offer_this_host_cannot_run_does_not_hide_an_older_one_it_can() {
        // Finding 3: the newest complete offer is judged first, but its refusal is not the end.
        const ME: &str = "did:key:me";
        const A: &str = "did:key:a";
        const B: &str = "did:key:b";
        const HEAD: &str = "c0ffee";
        let mut rows = vec![
            row(A, HEAD, "OLD", "epr", "2026-11-17T01:00:00Z"),
            row(A, HEAD, "OLD", "bin-launch", "2026-11-17T01:00:00Z"),
            row(B, HEAD, "NEW", "epr", "2026-11-17T02:00:00Z"),
            row(B, HEAD, "NEW", "bin-launch", "2026-11-17T02:00:00Z"),
        ];
        let compatible = glibc_at_least(&[2, 17]);
        let too_new = glibc_at_least(&[2, 39]);
        for row in &mut rows {
            let stated = if row.release.as_deref() == Some("OLD") {
                &compatible
            } else {
                &too_new
            };
            row.requires.clone_from(&stated.cid);
            row.requires_doc.clone_from(&stated.doc);
            row.isa_c.clone_from(&stated.isa_c);
        }
        let offers = sibling::offers(&rows, ME, HEAD);
        assert_eq!(
            offers
                .iter()
                .map(|o| o.release.as_str())
                .collect::<Vec<_>>(),
            ["NEW", "OLD"],
            "newest first"
        );

        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let landed = tmp.path().join("sibling");
        sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
        let release = store.release_cid_of(&landed).unwrap();
        // OLD is the release whose bytes the fixture serves.
        let mut offers = offers;
        offers[1].release.clone_from(&release);
        offers[1].outputs = offer_for(&release).outputs;
        let host = host_with_glibc(&[2, 31]);
        let serve = serving(&landed);
        let fetched = std::cell::RefCell::new(Vec::new());
        let commit = "b".repeat(40);
        let drawn = match sibling::draw_first(
            &store,
            tmp.path(),
            &commit,
            &tmp.path().join("incoming"),
            &offers,
            &sibling::Judges {
                admit: &|_, document| sibling::admit_on(&host, document),
                fetch: &|output, dest| {
                    fetched.borrow_mut().push(output.to_string());
                    serve(output, dest)
                },
                staged: &|_, _| Ok(()),
                patience: &sibling::Patience::none(),
            },
        ) {
            Ok(value) => value,
            Err(sibling::Draw::Gap(gap)) => panic!("gap: {gap}"),
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(r)) => panic!("refused {}: {}", r.kind, r.detail),
        };
        assert_eq!(drawn["release"], release);
        assert_eq!(drawn["device"], A);
        assert_eq!(
            fetched.borrow().len(),
            2,
            "only the admitted offer's two bins are fetched: {:?}",
            fetched.borrow()
        );

        // With nothing admissible, the gap starts with the newest offer's and names the rest.
        let ancient = host_with_glibc(&[2, 12]);
        match sibling::draw_first(
            &store,
            tmp.path(),
            &"9".repeat(40),
            &tmp.path().join("incoming"),
            &offers,
            &sibling::Judges {
                admit: &|_, document| sibling::admit_on(&ancient, document),
                fetch: &|_, _| Err("must not fetch".into()),
                staged: &|_, _| panic!("nothing is staged"),
                patience: &sibling::Patience::none(),
            },
        ) {
            Err(sibling::Draw::Gap(gap)) => {
                assert!(gap.starts_with("requirement-unmet: "), "{gap}");
                assert!(gap.contains("of release NEW"), "{gap}");
                assert!(gap.contains("and 1 older offer(s)"), "{gap}");
                assert!(gap.contains(&release), "{gap}");
            }
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(r)) => panic!("refused {}", r.detail),
            Ok(value) => panic!("taken {value}"),
        }
    }

    #[cfg(feature = "peer")]
    #[test]
    fn staged_bytes_whose_own_requirement_is_unmet_are_never_ingested_or_run() {
        // The residual's cheap half: a statement that admits the offer does not excuse bytes
        // that need more. The staged judge sees each drawn bin's bytes before ingest executes
        // either one, and its refusal leaves nothing in the store.
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let landed = tmp.path().join("sibling");
        sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
        let release = store.release_cid_of(&landed).unwrap();
        let commit = "7".repeat(40);
        let incoming = tmp.path().join("incoming");
        let serve = serving(&landed);
        let judged = std::cell::RefCell::new(Vec::new());
        let result = sibling::take(
            &store,
            tmp.path(),
            &commit,
            &incoming,
            &offer_for(&release),
            &serve,
            &|bin, bytes| {
                judged.borrow_mut().push((bin.to_string(), bytes.len()));
                if bin == "epr" {
                    Err("isa.x86 missing-feature avx512f".into())
                } else {
                    Ok(())
                }
            },
        );
        match result {
            Err(sibling::Draw::Gap(gap)) => {
                assert!(gap.starts_with("statement-understated: epr "), "{gap}");
                assert!(gap.contains("avx512f"), "{gap}");
                assert!(gap.contains("nothing was ingested"), "{gap}");
            }
            Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
            Err(sibling::Draw::Refused(r)) => panic!("a gap, not a refusal: {}", r.detail),
            Ok(value) => panic!("understated bytes were taken: {value}"),
        }
        // The launcher (first in name order) and epr were each judged on their full bytes.
        let judged = judged.into_inner();
        assert_eq!(judged[0].0, "bin-launch");
        assert_eq!(judged[1].0, "epr");
        assert_eq!(
            judged[1].1,
            std::fs::metadata(landed.join("epr")).unwrap().len() as usize
        );
        assert!(!store.holds_verified_for(&commit), "nothing is ingested");
        assert!(!incoming.join(format!("sibling-{release}")).exists());
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_gap_names_an_axis_only_when_it_is_that_axis_or_under_it() {
        for (gap, axis) in [
            ("isa.x86.partial-decode", Some("isa.x86")),
            ("abi.libc", Some("abi.libc")),
            ("lib.unresolved", Some("lib")),
            ("library", None),
            ("build-inputs.build-scripts", None),
            ("isa.x86x", None),
            // A gap in the observation beside a declared requirement leaves it measured.
            ("isa.x86.observed.partial-decode", None),
        ] {
            assert_eq!(sibling::unmeasured_axis_for_test(gap), axis, "{gap}");
        }
    }

    #[cfg(feature = "peer")]
    #[test]
    fn an_offer_without_this_devices_c_baseline_is_refused_before_any_fetch() {
        // Row 29.2: shem drew a release whose bundled C needed x86-64-v3. A statement that
        // admits on every clause still does not vouch for the C; only the producer's checked
        // `env:isa-c` does, and only when it is the baseline this epr knows for its target.
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let host = host_with_glibc(&[2, 99]);
        for (marker, why) in [
            (None, "records no env:isa-c"),
            (Some("x86-64-v3"), "records env:isa-c=x86-64-v3, not x86-64"),
            (Some("x86-64-v2"), "records env:isa-c=x86-64-v2, not x86-64"),
            (Some(""), "records env:isa-c=, not x86-64"),
        ] {
            let mut stated = glibc_at_least(&[2, 17]);
            stated.isa_c = marker.map(str::to_owned);
            let offer = stating(offer_for("R1"), &stated);
            let fetched = std::cell::Cell::new(0);
            match sibling::draw_offer(
                &store,
                tmp.path(),
                &"6".repeat(40),
                &tmp.path().join("incoming"),
                &offer,
                &|_, document| sibling::admit_on(&host, document),
                &|_, _| {
                    fetched.set(fetched.get() + 1);
                    Err("must not fetch".into())
                },
            ) {
                Err(sibling::Draw::Gap(gap)) => {
                    assert!(gap.starts_with("unmeasured-requirement: "), "{gap}");
                    assert!(gap.contains(why), "{marker:?}: {gap}");
                    assert!(
                        gap.contains(
                            "cure: the producer rebuilds under the declared target, or \
                             build for this device"
                        ),
                        "{gap}"
                    );
                    assert!(!gap.contains("re-put"), "{gap}");
                }
                Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
                Err(sibling::Draw::Refused(r)) => panic!("a gap, not a refusal: {}", r.detail),
                Ok(value) => panic!("{marker:?}: an unmarked release was drawn: {value}"),
            }
            assert_eq!(fetched.get(), 0, "{marker:?}: nothing is fetched");
        }
        // Every bin must carry it: a marked epr beside an unmarked launcher is not drawn.
        let mut half = stating(offer_for("R1"), &glibc_at_least(&[2, 17]));
        half.requires.get_mut("bin-launch").unwrap().isa_c = None;
        let gap = sibling::admissible(&half, &|_, d| sibling::admit_on(&host, d)).unwrap_err();
        assert!(gap.contains("bin-launch of release R1"), "{gap}");
        assert!(gap.contains("records no env:isa-c"), "{gap}");
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_target_with_no_c_baseline_draws_no_sibling_offer() {
        assert_eq!(
            sibling::c_baseline(Some("x86_64-unknown-linux-gnu")),
            Some(("x86_64-unknown-linux-gnu", "x86-64"))
        );
        let offer = offer_for("R1");
        assert_eq!(
            sibling::isa_c_admits(
                "epr",
                &offer,
                Some("x86-64"),
                Some("x86_64-unknown-linux-gnu")
            ),
            Ok(())
        );
        for target in [
            None,
            Some("aarch64-unknown-linux-gnu"),
            Some("x86_64-unknown-linux-musl"),
        ] {
            let gap = sibling::isa_c_admits("epr", &offer, Some("x86-64"), target).unwrap_err();
            assert!(gap.starts_with("unmeasured-requirement: "), "{gap}");
            assert!(gap.contains("knows no C baseline for its target"), "{gap}");
        }
    }

    #[cfg(feature = "peer")]
    #[test]
    fn staged_bytes_whose_exercise_fails_are_never_ingested() {
        // Before ingest runs either bin, the staged epr runs `doctor --exercise`. A signal death
        // and any non-zero exit (a release from before the verb) each leave the store as it was.
        for (body, kind) in [
            ("kill -ILL $$", "the staged bytes' signal: "),
            ("exit 2", "the staged bytes' exit: "),
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let store = two_bin_store(tmp.path());
            let landed = tmp.path().join("sibling");
            sibling_bins_with(
                &landed,
                &format!("#!/bin/sh\n{body}\n"),
                "#!/bin/sh\n# sibling launcher\n",
            );
            let release = store.release_cid_of(&landed).unwrap();
            let commit = "5".repeat(40);
            let incoming = tmp.path().join("incoming");
            let serve = serving(&landed);
            match sibling::take(
                &store,
                tmp.path(),
                &commit,
                &incoming,
                &offer_for(&release),
                &serve,
                &|_, _| Ok(()),
            ) {
                Err(sibling::Draw::Gap(gap)) => {
                    assert!(gap.starts_with("exercise-failed: epr of release "), "{gap}");
                    assert!(gap.contains(kind), "{body}: {gap}");
                    assert!(gap.contains("nothing was ingested"), "{gap}");
                }
                Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
                Err(sibling::Draw::Refused(r)) => panic!("a gap, not a refusal: {}", r.detail),
                Ok(value) => panic!("{body}: bytes whose exercise failed were taken: {value}"),
            }
            assert!(
                !store.holds_verified_for(&commit),
                "{body}: nothing is ingested"
            );
            assert!(!incoming.join(format!("sibling-{release}")).exists());
        }
    }

    /// Ingests the two bins in `dir` as a release of `commit` supplied by `gate_run`, as the draw
    /// or a gate ingests them, and returns its CID.
    #[cfg(feature = "peer")]
    fn held_release(store: &Store, dir: &Path, commit: &str, gate_run: &str) -> String {
        let release = store
            .ingest(
                &dir.join("epr"),
                &IngestOptions {
                    repo: dir.parent().map(Path::to_path_buf),
                    source_commit: Some(commit.into()),
                    gate_run: Some(gate_run.into()),
                    tree_clean: Some(true),
                    closure: false,
                },
            )
            .unwrap();
        assert!(store.verified(&release.cid), "the fixture release verifies");
        release.cid
    }

    /// `epr bin <verb…>` against the store [`two_bin_store`] opens under `root`.
    #[cfg(feature = "peer")]
    fn bin_command(root: &Path, verb: &[&str]) -> Result<String, Refusal> {
        let registry = root.join("bin-tools.json");
        std::fs::write(
            &registry,
            json!({"version":1,"tools":{"epr":{"bins":["epr","bin-launch"],"aliases":{},
                "longRunning":[],"candidateEnv":{"epr":["EPR_BIN"]}}}})
            .to_string(),
        )
        .unwrap();
        let mut args: Vec<String> = verb.iter().map(|arg| (*arg).to_owned()).collect();
        for (flag, value) in [
            ("--registry", registry.to_str().unwrap()),
            ("--store", root.join("store/epr").to_str().unwrap()),
            ("--tool", "epr"),
        ] {
            args.push(flag.into());
            args.push(value.into());
        }
        execute(&args)
    }

    #[cfg(feature = "peer")]
    #[test]
    fn an_explicit_land_or_pin_of_a_verified_siblings_release_runs_its_exercise_first() {
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let launcher = "#!/bin/sh\n# sibling launcher\n";
        for (name, body, kind) in [
            ("crashed", "kill -ILL $$", "its own signal: "),
            ("old", "exit 2", "its own exit: "),
        ] {
            let dir = tmp.path().join(name);
            sibling_bins_with(&dir, &format!("#!/bin/sh\n{body}\n"), launcher);
            let cid = held_release(&store, &dir, "c0ffee", "sibling:did:key:z6MkA");
            for verb in [
                vec!["land", &cid, "--expect", "none", "--commit", "c0ffee"],
                vec!["pin", "--cid", &cid],
            ] {
                let refusal = bin_command(tmp.path(), &verb).unwrap_err();
                assert_eq!(refusal.kind, "sibling-exercise-failed", "{verb:?}");
                assert!(
                    refusal.detail.contains(kind),
                    "{verb:?}: {}",
                    refusal.detail
                );
                assert!(
                    refusal.detail.contains("sibling:did:key:z6MkA"),
                    "{}",
                    refusal.detail
                );
                assert!(!refusal.detail.contains("  "), "{}", refusal.detail);
            }
        }
        // A release only this device built is not judged; a sibling's that passes reaches the
        // store. Either way the refusal, if any, is the store's own.
        for (name, body, gate) in [
            ("mine", "kill -ILL $$ # built here", "gate:local"),
            ("good", "exit 0", "sibling:did:key:z6MkA"),
        ] {
            let dir = tmp.path().join(name);
            sibling_bins_with(&dir, &format!("#!/bin/sh\n{body}\n"), launcher);
            let cid = held_release(&store, &dir, "c0ffee", gate);
            if let Err(refusal) = bin_command(tmp.path(), &["pin", "--cid", &cid]) {
                assert_ne!(
                    refusal.kind, "sibling-exercise-failed",
                    "{}",
                    refusal.detail
                );
            }
        }
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_held_release_whose_bytes_do_not_make_its_cid_is_never_executed() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let store = two_bin_store(tmp.path());
        let marker = tmp.path().join("executed");
        let dir = tmp.path().join("drawn");
        sibling_bins_with(
            &dir,
            &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
            "#!/bin/sh\n# sibling launcher\n",
        );
        let cid = held_release(&store, &dir, "c0ffee", "sibling:did:key:z6MkA");
        // The held epr is replaced after it was verified: its bytes no longer make the CID.
        let held = store.root().join(&cid).join("epr");
        std::fs::set_permissions(&held, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(
            &held,
            format!("#!/bin/sh\ntouch '{}' # replaced\n", marker.display()),
        )
        .unwrap();
        assert!(!store.verified(&cid));
        for verb in [
            vec!["land", &cid, "--expect", "none", "--commit", "c0ffee"],
            vec!["pin", "--cid", &cid],
        ] {
            let refusal = bin_command(tmp.path(), &verb).unwrap_err();
            assert_ne!(refusal.kind, "sibling-exercise-failed", "{verb:?}");
            assert!(
                !marker.exists(),
                "{verb:?}: the unverified bytes were executed"
            );
        }
    }

    #[cfg(feature = "peer")]
    #[test]
    fn the_own_target_is_the_triple_cargo_compiled_for() {
        // The build-time TARGET is the fact: it holds for a cross-target build too, and needs no
        // rustc where the tests run.
        assert_eq!(sibling::OWN_TARGET, env!("EPR_CLI_TARGET"));
        // GNU x32 satisfies x86_64, linux and gnu; it, like any triple without an entry, has no
        // baseline.
        for target in [
            "x86_64-unknown-linux-gnux32",
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-gnu",
        ] {
            assert_eq!(sibling::c_baseline(Some(target)), None, "{target}");
        }
    }
}
