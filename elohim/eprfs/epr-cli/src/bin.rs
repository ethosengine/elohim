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
    match execute(args) {
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
pub(crate) fn execute(args: &[String]) -> Result<String, Refusal> {
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
            json!(store.land_expected(
                positional
                    .get(1)
                    .or_else(|| options.get("--cid"))
                    .ok_or_else(|| argument("land requires cid"))?,
                options
                    .get("--commit")
                    .ok_or_else(|| argument("land requires --commit"))?,
                &repo()?,
                transition,
                (expected != "none").then_some(expected)
            )?)
        }
        "pin" => json!(store.pin(
            &identity,
            options.get("--cid").copied(),
            flags.contains("--provisional"),
            options.get("--repo").map(Path::new)
        )?),
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
                    // A closure the caller supplied is the bytes to follow; nothing is drawn.
                    let drawn = if follow.mode.applies() && closure.is_none() {
                        sibling::before_landing(&store, tool, &repo)?
                    } else {
                        sibling::Drawn::default()
                    };
                    match store.follow_declared(&repo, closure, follow) {
                        Ok(mut value) => {
                            if let (Some(record), Some(map)) = (drawn.record, value.as_object_mut())
                            {
                                map.insert("sibling".into(), record);
                            }
                            value
                        }
                        Err(refusal) => return Err(drawn.explain(refusal)),
                    }
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
    use super::{Refusal, Store};
    use serde_json::Value;
    use std::path::Path;

    /// The knob that makes a host that can build draw from a sibling first.
    #[cfg(feature = "peer")]
    pub(crate) const FOLLOW_SIBLING_ENV: &str = "EPR_FOLLOW_SIBLING";
    /// The bins one sibling must hold for one release before it is taken.
    #[cfg(feature = "peer")]
    pub(crate) const BINS: [&str; 2] = ["epr", "bin-launch"];

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
        pub at: String,
    }

    /// Whether `release` is one CID-shaped token: non-empty ASCII letters and digits only, so it
    /// can name a directory without leaving the one it is joined to.
    #[cfg(feature = "peer")]
    pub(crate) fn names_one_release(release: &str) -> bool {
        !release.is_empty() && release.bytes().all(|b| b.is_ascii_alphanumeric())
    }

    /// The release to draw for `commit`: among admitted siblings' crossed Produces stamped with
    /// `env:release` and `env:release-bin`, the newest (device, release) that holds every bin.
    /// `me`'s own records are never offers.
    #[cfg(feature = "peer")]
    pub(crate) fn select(
        rows: &[crate::peer::SiblingArtifact],
        me: &str,
        commit: &str,
    ) -> Option<Offer> {
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
                    at: String::new(),
                });
            // Rows arrive in flow order, so a later record of the same bin stands.
            offer.outputs.insert(bin.clone(), row.output.clone());
            if row.at > offer.at {
                offer.at.clone_from(&row.at);
            }
        }
        offers
            .into_values()
            .filter(|offer| BINS.iter().all(|bin| offer.outputs.contains_key(*bin)))
            .max_by(|a, b| a.at.cmp(&b.at).then_with(|| b.release.cmp(&a.release)))
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
        eprintln!("{why}");
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
        match draw(store, repo, &commit, &incoming) {
            Ok(record) => Ok(Drawn {
                record: Some(record),
                gap: None,
            }),
            Err(Draw::Gap(gap)) => {
                eprintln!(
                    "follow: nothing drawn from a sibling ({gap}); {}",
                    if can_build {
                        "building locally"
                    } else {
                        "trying a local build"
                    }
                );
                Ok(Drawn {
                    record: None,
                    gap: Some(gap),
                })
            }
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
        /// The sibling's bytes are not what it promised, or the store refused them.
        Refused(Refusal),
    }

    #[cfg(feature = "peer")]
    fn draw(store: &Store, repo: &Path, commit: &str, incoming: &Path) -> Result<Value, Draw> {
        let found = crate::peer::custody_artifacts(repo, None)
            .map_err(|e| Draw::Gap(format!("no-record: {e}")))?;
        let offer = select(&found.rows, &found.device, commit).ok_or_else(|| {
            Draw::Gap(format!(
                "no-offer: no admitted sibling recorded a crossed Produce of epr and bin-launch \
                 stamped with a release for {commit}"
            ))
        })?;
        take(store, repo, commit, incoming, &offer, &|output, dest| {
            crate::artifact::fetch_output(repo, output, dest).map_err(|e| e.to_string())
        })
    }

    /// Draw `offer`'s bins with `fetch` (exact output CID to destination), refuse bytes that do
    /// not make the release it named, and ingest them as the sibling's.
    #[cfg(feature = "peer")]
    pub(crate) fn take(
        store: &Store,
        repo: &Path,
        commit: &str,
        incoming: &Path,
        offer: &Offer,
        fetch: &dyn Fn(&str, &Path) -> Result<(), String>,
    ) -> Result<Value, Draw> {
        use std::os::unix::fs::PermissionsExt;
        if !names_one_release(&offer.release) {
            return Err(Draw::Gap(format!(
                "no-offer: {} named a release that is not one CID token",
                offer.device
            )));
        }
        let dir = incoming.join(format!("sibling-{}", offer.release));
        // A previous partial draw is replaced, never trusted.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .map_err(|e| Draw::Gap(format!("local-store: {}: {e}", dir.display())))?;
        let started = std::time::Instant::now();
        let result = (|| {
            for (bin, output) in &offer.outputs {
                let dest = dir.join(bin);
                fetch(output, &dest).map_err(Draw::Gap)?;
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
                return Err(mismatch(&fetched));
            }
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
                return Err(mismatch(&release.cid));
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
            device: device.into(),
            output: format!("output-{release}-{bin}"),
            manifest: "manifest".into(),
            bytes: 1,
            commit: commit.into(),
            release: (!release.is_empty()).then(|| release.into()),
            release_bin: (!bin.is_empty()).then(|| bin.into()),
            agent_id: None,
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
    fn two_bin_store(root: &Path) -> Store {
        let registry: Registry = serde_json::from_value(json!({"version":1,"tools":{"epr":{
            "bins":["epr","bin-launch"],"aliases":{},"longRunning":[],
            "candidateEnv":{"epr":["EPR_BIN"]}}}}))
        .unwrap();
        Store::new(root.join("store/epr"), "epr", &registry)
            .unwrap()
            .with_runner(std::sync::Arc::new(Identity))
    }

    #[cfg(feature = "peer")]
    fn sibling_bins(dir: &Path, launcher: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(dir).unwrap();
        for (bin, body) in [
            ("epr", "#!/bin/sh\n# sibling epr\n"),
            ("bin-launch", launcher),
        ] {
            std::fs::write(dir.join(bin), body).unwrap();
            std::fs::set_permissions(dir.join(bin), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
    }

    #[cfg(feature = "peer")]
    fn offer_for(release: &str) -> sibling::Offer {
        sibling::Offer {
            device: "did:key:z6MkSibling".into(),
            release: release.into(),
            outputs: [("bin-launch", "out-launch"), ("epr", "out-epr")]
                .into_iter()
                .map(|(b, o)| (b.to_string(), o.to_string()))
                .collect(),
            at: "2026-11-17T07:00:00Z".into(),
        }
    }

    /// Serves `dir`'s bins by output CID, as a sibling's daemon would.
    #[cfg(feature = "peer")]
    fn serving(dir: &Path) -> impl Fn(&str, &Path) -> Result<(), String> + '_ {
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
        ) {
            Ok(value) => value,
            Err(sibling::Draw::Gap(gap)) => panic!("gap: {gap}"),
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
        ) {
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
        relabelled.push(later);
        assert_eq!(
            select(&relabelled, ME, HEAD).unwrap().outputs["epr"],
            "output-R1-epr-again"
        );

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
}
