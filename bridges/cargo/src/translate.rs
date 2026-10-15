//! A workspace's resolve graph → reliance records. Pure: no clock, no network, no filesystem.
//!
//! What one translation yields, in append order:
//!
//! 1. **The offer** — the collective's governed offer to `service:cargo`, a two-sided disclosure
//!    whose `disclosure.inflows` names ONE unclaimed presence per upstream *project* (identity:
//!    the package's declared repository URL, falling back to the crate name). Rendered as an
//!    offer document and minted as its Intent, exactly the Jenkins offer's shape; it starts
//!    Proposed and only a Steward who is not its author makes it Active.
//! 2. **One `Cite` event per resolved package** (node 1): provider the upstream presence (or the
//!    repository itself for in-repo path packages), receiver the collective, resource the CID
//!    that wraps the lockfile's own checksum (node 2,
//!    [`elohim_epr::cid::raw_cid_from_sha256_hex`]). A path or git package has no checksum: its
//!    resource is the dag-cbor CID of its *coordinate* and the event says
//!    `identity:coordinate-only` — no CID is ever invented for bytes the lockfile does not pin.
//!    Each event carries the package's declared `license:` SPDX string.
//! 3. **One `Process`** whose `inputs` are those events' CIDs — the build rests on them.
//! 4. **One `DepEdge{Compiler(<workspace>)}`** per direct edge from a workspace member to a
//!    registry package, naming the package's bytes through `to_resource`.
//!
//! Every event and the Process carry the shared external-claim envelope (`service:cargo`, the
//! lockfile as provenance, unattested, the lowest reach, `in-scope-of` the offer), so the records
//! read as cargo's claim, evaluated on entry. Nothing here values a dependency or names a payee.

use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr_cli::flow::memory::offer::{offer_cid, offer_intent, parse_offer};
use elohim_epr_rea::{
    AgentRef, DepEdge, Disclosure, ExternalClaim, Externalities, FlowEvent, FlowRecord, Governor,
    HeldCapability, Inflow, Magnitude, OfferDeclaration, OfferDocument, PinnedRef, Process,
    ReaVerb, RecipePin, SignatureStatus, Terms,
};
use serde::Serialize;

use crate::metadata::{Checksums, Metadata, Package};

/// The provenance participant every record from this bridge names.
pub const SOURCE: &str = "service:cargo";
/// The recipe the build Process pins. Not yet declared in `.claude/epr-meta/recipes.yaml`:
/// projecting it is the gate step's concern (spec §7 step 6).
pub const RECIPE_ID: &str = "cargo-workspace-build";
pub const RECIPE_VERSION: u32 = 1;
/// One package drawn on, counted.
pub const PACKAGE_UNIT: &str = "package";
const CRATES_IO_INDEXES: [&str; 2] = [
    "registry+https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
];
/// Forges whose repository URLs name a project in their first two path segments.
const FORGES: [&str; 5] = [
    "github.com",
    "gitlab.com",
    "codeberg.org",
    "bitbucket.org",
    "git.sr.ht",
];

/// A refusal: malformed or unknown input, refused before anything is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub String);
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Refusal {}

/// Where a translation runs and who it names. Every field is an input, so the output is a pure
/// function of (metadata, lockfile, config).
#[derive(Debug, Clone)]
pub struct Config {
    /// The workspace's name in records (`Compiler(<workspace>)`, the offer id).
    pub workspace: String,
    /// The repository root (absolute), against which every recorded path is made relative.
    pub repo_root: String,
    /// This repository's own location (`github.com/ethosengine/elohim`): a package declaring it
    /// is the network's own work and gets no presence held in trust.
    pub self_repository: String,
    /// RFC3339 time of the observation: by default the lockfile's last commit, so unchanged
    /// metadata always yields the same records.
    pub occurred_at: String,
    /// Who drafted the offer; the approving Steward must be someone else.
    pub author: String,
    /// The governing collective (`collective:<id>`) and its declaration path.
    pub collective: String,
    pub collective_path: String,
    /// The repository agent that seals the edges and provides in-repo path packages.
    pub repo_agent: String,
}

impl Config {
    pub fn for_repository(workspace: &str, repo_root: &str, occurred_at: &str) -> Self {
        Self {
            workspace: workspace.to_string(),
            repo_root: repo_root.to_string(),
            self_repository: "github.com/ethosengine/elohim".into(),
            occurred_at: occurred_at.to_string(),
            author: "agent:implementer@claude-opus-5-5".into(),
            collective: "collective:ethosengine/elohim".into(),
            collective_path: ".epr-meta/collective.json".into(),
            repo_agent: elohim_epr_cli::flow::REPO_AGENT.into(),
        }
    }
}

/// Where a package came from, per its cargo `source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    Registry,
    Git,
    Path,
}

impl Origin {
    pub fn word(self) -> &'static str {
        match self {
            Origin::Registry => "registry",
            Origin::Git => "git",
            Origin::Path => "path",
        }
    }
}

/// One resolved package, as cited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cited {
    pub coordinate: String,
    pub origin: Origin,
    /// `checksum` when the resource wraps the lockfile's checksum; `coordinate-only` otherwise.
    pub identity: &'static str,
    pub resource: String,
    pub provider: String,
    pub license: String,
}

/// The counts a run reports.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub workspace: String,
    pub packages_resolved: usize,
    pub workspace_members: usize,
    pub cited: usize,
    pub registry_with_checksum: usize,
    pub registry_without_checksum: usize,
    pub git: usize,
    pub path: usize,
    pub presences: usize,
    pub edges: usize,
}

/// One workspace, translated.
#[derive(Debug, Clone)]
pub struct Reliance {
    pub offer: OfferDocument,
    pub offer_cid: Cid,
    pub events: Vec<FlowEvent>,
    pub process: Process,
    pub process_cid: Cid,
    pub edges: Vec<DepEdge>,
    pub cited: Vec<Cited>,
    pub report: Report,
}

impl Reliance {
    /// Every record, in append order: the offer's Intent, the cite events, the Process, the edges.
    pub fn records(&self) -> Vec<FlowRecord> {
        let mut records = vec![FlowRecord::Intent(offer_intent(&self.offer))];
        records.extend(self.events.iter().cloned().map(FlowRecord::Event));
        records.push(FlowRecord::Process(self.process.clone()));
        records.extend(self.edges.iter().cloned().map(FlowRecord::Edge));
        records
    }
}

/// The repository-relative path of the offer document a workspace's offer is rendered to.
pub fn offer_path(workspace: &str) -> String {
    format!(
        "bridges/.epr-meta/offers/cargo-{}.offer.md",
        slugify(workspace)
    )
}

/// Lowercase and keep only `[a-z0-9/-]`, every other run becoming one `-`; trims `-` and `/`.
fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.to_lowercase().chars() {
        let keep = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '/';
        if keep {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches(|c| c == '-' || c == '/').to_string()
}

/// A repository URL's location, scheme and noise removed: `https://github.com/serde-rs/serde.git`
/// and `git+ssh://git@github.com/serde-rs/serde/tree/main/serde_derive` are both
/// `github.com/serde-rs/serde` (a forge URL keeps only owner/repo, so the crates of one project
/// share one presence).
pub fn repository_location(url: &str) -> String {
    let mut s = url.trim().to_lowercase();
    for prefix in ["git+", "https://", "http://", "ssh://", "git://", "git@"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
        }
    }
    if let Some(rest) = s.strip_prefix("git@") {
        s = rest.to_string();
    }
    s = s.replacen(':', "/", 1);
    let s = s.split(['?', '#']).next().unwrap_or("").to_string();
    let s = s.trim_end_matches('/').trim_end_matches(".git").to_string();
    let mut segments: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    if let Some(host) = segments.first() {
        let host = host.trim_start_matches("www.");
        if FORGES.contains(&host) && segments.len() > 3 {
            segments.truncate(3);
        }
        let mut rebuilt = vec![host.to_string()];
        rebuilt.extend(
            segments
                .iter()
                .skip(1)
                .map(|p| p.trim_end_matches(".git").to_string()),
        );
        return rebuilt.join("/");
    }
    s
}

/// The presence slug for a repository URL: its location, slugged (`github-com/serde-rs/serde`).
pub fn presence_slug(url: &str) -> String {
    slugify(&repository_location(url))
}

/// Lexical relative path from `root` to `path` (both absolute, `/`-separated).
fn relative(root: &str, path: &str) -> String {
    let root: Vec<&str> = root.split('/').filter(|p| !p.is_empty()).collect();
    let path: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let common = root
        .iter()
        .zip(path.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<&str> = std::iter::repeat_n("..", root.len() - common).collect();
    parts.extend(&path[common..]);
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

fn manifest_dir(package: &Package) -> &str {
    package
        .manifest_path
        .rsplit_once('/')
        .map_or(package.manifest_path.as_str(), |(dir, _)| dir)
}

fn origin_of(package: &Package) -> Origin {
    match package.source.as_deref() {
        None => Origin::Path,
        Some(s) if s.starts_with("git+") => Origin::Git,
        Some(_) => Origin::Registry,
    }
}

fn registry_name(source: &str) -> String {
    if CRATES_IO_INDEXES.contains(&source) {
        "crates.io".into()
    } else {
        format!(
            "registry:{}",
            repository_location(source.split_once('+').map_or(source, |(_, u)| u))
        )
    }
}

/// The coordinate-only resource body: what is cited when no checksum pins the bytes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoordinateBody<'a> {
    source: &'a str,
    origin: Origin,
    coordinate: &'a str,
}

fn coordinate_cid(origin: Origin, coordinate: &str) -> Result<Cid, Refusal> {
    let body = CoordinateBody {
        source: SOURCE,
        origin,
        coordinate,
    };
    let bytes = serde_ipld_dagcbor::to_vec(&body).map_err(|e| Refusal(e.to_string()))?;
    Ok(elohim_epr::cid::compute_cid(&bytes))
}

fn epoch_seconds(rfc3339: &str) -> Result<i64, Refusal> {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| t.timestamp())
        .map_err(|e| Refusal(format!("occurred-at `{rfc3339}` is not RFC3339: {e}")))
}

struct Upstream {
    /// The URL as the first package (in coordinate order) declares it, for the inflow's text.
    url: Option<String>,
    packages: BTreeSet<String>,
}

/// Translate one workspace. `lockfile_checksums` is [`crate::metadata::parse_lockfile`] over the
/// workspace's `Cargo.lock`; `lockfile_bytes` are that file's bytes (their raw CID names what was
/// read).
pub fn translate(
    metadata: &Metadata,
    lockfile_checksums: &Checksums,
    lockfile_bytes: &[u8],
    config: &Config,
) -> Result<Reliance, Refusal> {
    let sealed_at = epoch_seconds(&config.occurred_at)?;
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or_else(|| Refusal("cargo metadata has no resolve graph".into()))?;
    let packages: BTreeMap<&str, &Package> = metadata
        .packages
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let members: BTreeSet<&str> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();
    let resolved: BTreeSet<&str> = resolve.nodes.iter().map(|n| n.id.as_str()).collect();
    let self_location = slugify(&config.self_repository);
    let lockfile_rel = format!(
        "{}/Cargo.lock",
        relative(&config.repo_root, &metadata.workspace_root)
    );
    let lockfile_cid = elohim_epr::cid::compute_raw_cid(lockfile_bytes);

    let mut report = Report {
        workspace: config.workspace.clone(),
        packages_resolved: resolved.len(),
        workspace_members: members.len(),
        ..Report::default()
    };

    // 1. Classify every resolved, non-member package: coordinate, resource, provider.
    let mut cited: Vec<Cited> = Vec::new();
    let mut upstreams: BTreeMap<String, Upstream> = BTreeMap::new();
    let mut registry_resource: BTreeMap<&str, (String, Cid)> = BTreeMap::new();
    for id in &resolved {
        if members.contains(id) {
            continue;
        }
        let package = packages.get(id).ok_or_else(|| {
            Refusal(format!(
                "resolve names `{id}`, which packages does not list"
            ))
        })?;
        let origin = origin_of(package);
        let versioned = format!("{}@{}", package.name, package.version);
        let (coordinate, checksum) = match origin {
            Origin::Registry => {
                let source = package.source.as_deref().unwrap_or_default();
                let key = (
                    package.name.clone(),
                    package.version.clone(),
                    source.to_string(),
                );
                (
                    format!("{}:{versioned}", registry_name(source)),
                    lockfile_checksums.get(&key).cloned(),
                )
            }
            Origin::Git => {
                let source = package.source.as_deref().unwrap_or_default();
                let rev = source.rsplit_once('#').map_or("", |(_, r)| r);
                (format!("git:{versioned}#{rev}"), None)
            }
            Origin::Path => (
                format!(
                    "path:{}",
                    relative(&config.repo_root, manifest_dir(package))
                ),
                None,
            ),
        };
        let (identity, resource) = match &checksum {
            Some(hex) => (
                "checksum",
                elohim_epr::cid::raw_cid_from_sha256_hex(hex)
                    .map_err(|e| Refusal(format!("Cargo.lock checksum of {coordinate}: {e}")))?,
            ),
            None => ("coordinate-only", coordinate_cid(origin, &coordinate)?),
        };
        match (origin, checksum.is_some()) {
            (Origin::Registry, true) => report.registry_with_checksum += 1,
            (Origin::Registry, false) => report.registry_without_checksum += 1,
            (Origin::Git, _) => report.git += 1,
            (Origin::Path, _) => report.path += 1,
        }
        if origin == Origin::Registry && checksum.is_some() {
            registry_resource.insert(id, (coordinate.clone(), resource));
        }

        // The upstream project: the declared repository, else the crate (registry), the git URL
        // (git), or — for a path package with no repository — this repository when it lives
        // inside it.
        let declared = package
            .repository
            .as_deref()
            .filter(|u| !u.trim().is_empty());
        let location = match (declared, origin) {
            (Some(url), _) => Some((presence_slug(url), Some(url.to_string()))),
            (None, Origin::Registry) => {
                Some((slugify(&format!("crates-io/{}", package.name)), None))
            }
            (None, Origin::Git) => {
                let source = package.source.as_deref().unwrap_or_default();
                let url = source.trim_start_matches("git+");
                Some((
                    presence_slug(url),
                    Some(url.split(['?', '#']).next().unwrap_or(url).to_string()),
                ))
            }
            (None, Origin::Path) => {
                let rel = relative(&config.repo_root, manifest_dir(package));
                if rel.starts_with("..") {
                    Some((slugify(&format!("path/{}", package.name)), None))
                } else {
                    None
                }
            }
        };
        let provider = match location {
            Some((slug, _)) if slug == self_location => config.repo_agent.clone(),
            None => config.repo_agent.clone(),
            Some((slug, url)) => {
                let upstream = upstreams.entry(slug.clone()).or_insert(Upstream {
                    url: None,
                    packages: BTreeSet::new(),
                });
                upstream.packages.insert(versioned.clone());
                if upstream.url.is_none() {
                    upstream.url = url;
                }
                format!("presence:{slug}")
            }
        };
        cited.push(Cited {
            coordinate,
            origin,
            identity,
            resource: resource.to_string(),
            provider,
            license: package
                .license
                .clone()
                .filter(|l| !l.trim().is_empty())
                .unwrap_or_else(|| "undeclared".into()),
        });
    }
    cited.sort_by(|a, b| a.coordinate.cmp(&b.coordinate));
    report.cited = cited.len();
    report.presences = upstreams.len();

    // 2. The offer: a two-sided disclosure naming every upstream project.
    let offer = render_offer(config, &upstreams)?;
    let offer_cid = offer_cid(&offer).map_err(|e| Refusal(e.to_string()))?;
    let claim = ExternalClaim::new(
        SOURCE,
        &lockfile_rel,
        SignatureStatus::Unattested,
        offer_cid,
    )
    .map_err(|e| Refusal(e.to_string()))?;
    let workspace_slot = format!("workspace:{}", config.workspace);

    // 3. The cite events, then the Process that rests on them.
    let mut events = Vec::with_capacity(cited.len());
    let mut inputs = Vec::with_capacity(cited.len());
    for c in &cited {
        let event = FlowEvent {
            action: ReaVerb::Cite,
            provider: AgentRef(c.provider.clone()),
            receiver: AgentRef(config.collective.clone()),
            resource: c.resource.parse().map_err(|e| Refusal(format!("{e}")))?,
            quantity: Magnitude::Count {
                value: 1.0,
                unit: PACKAGE_UNIT.to_string(),
            },
            process: None,
            in_scope_of: offer_cid,
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            occurred_at: config.occurred_at.clone(),
            classified_as: claim.classify(
                "reliance:cite",
                c.coordinate.clone(),
                [
                    format!("identity:{}", c.identity),
                    format!("origin:{}", c.origin.word()),
                    format!("license:{}", c.license),
                    workspace_slot.clone(),
                ],
            ),
        };
        inputs.push(
            FlowRecord::Event(event.clone())
                .cid()
                .map_err(|e| Refusal(e.to_string()))?,
        );
        events.push(event);
    }
    let process = Process {
        spec: PinnedRef {
            id: RECIPE_ID.into(),
            version: RECIPE_VERSION,
        },
        in_scope_of: offer_cid,
        inputs,
        outputs: Vec::new(),
        classified_as: claim.classify(
            "reliance:resolve",
            format!("cargo:{}", config.workspace),
            [format!("lockfile:{lockfile_cid}"), workspace_slot],
        ),
    };
    let process_cid = FlowRecord::Process(process.clone())
        .cid()
        .map_err(|e| Refusal(e.to_string()))?;

    // 4. Direct edges from a member to a registry package the lockfile pins.
    let mut slots: BTreeMap<(String, String), Cid> = BTreeMap::new();
    for node in &resolve.nodes {
        if !members.contains(node.id.as_str()) {
            continue;
        }
        let member = packages
            .get(node.id.as_str())
            .ok_or_else(|| Refusal(format!("member `{}` is not in packages", node.id)))?;
        let from = relative(&config.repo_root, manifest_dir(member));
        for dep in &node.deps {
            if let Some((coordinate, resource)) = registry_resource.get(dep.pkg.as_str()) {
                slots.insert((from.clone(), coordinate.clone()), *resource);
            }
        }
    }
    let mut edges = Vec::with_capacity(slots.len());
    for ((from, to), resource) in slots {
        let edge = DepEdge::new(
            from,
            to,
            None,
            Governor::Compiler(config.workspace.clone()),
            None,
            AgentRef(config.repo_agent.clone()),
            sealed_at,
            None,
        )
        .map_err(|e| Refusal(e.to_string()))?
        .with_to_resource(resource);
        edges.push(edge);
    }
    report.edges = edges.len();

    Ok(Reliance {
        offer,
        offer_cid,
        events,
        process,
        process_cid,
        edges,
        cited,
        report,
    })
}

/// Render the workspace's offer document deterministically and parse it back through the one
/// offer reader, so the document is validated exactly as a hand-authored one would be.
fn render_offer(
    config: &Config,
    upstreams: &BTreeMap<String, Upstream>,
) -> Result<OfferDocument, Refusal> {
    let inflows = upstreams
        .iter()
        .map(|(slug, upstream)| Inflow {
            presence: format!("presence:{slug}"),
            brings: format!(
                "{} — Rust packages the {} workspace builds on, made by the project at {}, \
                 outside this network",
                upstream
                    .packages
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
                config.workspace,
                upstream
                    .url
                    .as_deref()
                    .unwrap_or("its registry listing (no repository declared)"),
            ),
        })
        .collect();
    let declaration = OfferDeclaration {
        offer_version: elohim_epr_rea::external::OFFER_VERSION,
        id: slugify(&format!("cargo-{}", config.workspace)),
        collective: config.collective_path.clone(),
        provider: config.collective.clone(),
        receiver: SOURCE.into(),
        author: config.author.clone(),
        recipe: RecipePin {
            id: RECIPE_ID.into(),
            version: RECIPE_VERSION,
        },
        offered: vec![format!(
            "reliance-record: the {} workspace's resolved packages, each a cite input on its build \
             Process, refreshed from Cargo.lock",
            config.workspace
        )],
        terms: Terms {
            optional: true,
            exclusive: false,
            withdrawable: true,
            claims: "none".into(),
            settlement: "none".into(),
        },
        crosses_network: false,
        disclosure: Disclosure {
            inflows,
            held_capabilities: vec![
                HeldCapability {
                    id: "registry:package-fetch".into(),
                    what:
                        "downloads the source of every package Cargo.lock names from its registry \
                           or git remote"
                            .into(),
                },
                HeldCapability {
                    id: "build:run-dependency-code".into(),
                    what: "runs each dependency's build script and procedural macros with the \
                           builder's own privileges"
                        .into(),
                },
            ],
            externalities: Externalities {
                positive: vec![
                    "every build compiles exactly the versions Cargo.lock pins, each registry \
                     download checked against its recorded checksum"
                        .into(),
                ],
                negative: vec![
                    "dependency build scripts and procedural macros run as code no Steward \
                     reviewed, on the machine that builds"
                        .into(),
                ],
            },
        },
    };
    let yaml = serde_yaml::to_string(&declaration).map_err(|e| Refusal(e.to_string()))?;
    let path = offer_path(&config.workspace);
    let text = format!(
        "---\n{yaml}---\n# Offer: the {ws} workspace's reliance records, given to cargo\n\n\
         Generated by `cargo-bridge` from `cargo metadata` and Cargo.lock; regenerating it from \
         unchanged inputs yields the same bytes and so the same offer address.\n\n\
         The collective `{collective}` records what the `{ws}` Rust workspace relies on: one \
         `cite` input per resolved package on the build's Process, and one presence held in trust \
         for each upstream project, named by its repository. The record states the facts of \
         reliance and defers valuation. The inflows name unclaimed presences, never payees; \
         nothing here creates a claim or a settlement.\n",
        ws = config.workspace,
        collective = config.collective,
    );
    parse_offer(&path, &text).map_err(|e| Refusal(e.to_string()))
}
