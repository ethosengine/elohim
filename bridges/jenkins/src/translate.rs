//! Jenkins' own archive → external-claim observations. Pure: no clock, no network, no filesystem.
//!
//! Two sources, both Jenkins' own records of what it did:
//! - `wfapi/describe` of one `elohim-edge` build (per-stage status, start and duration) — always
//!   required;
//! - the dispatching ORCHESTRATOR run's archived `actual-build-graph.json` (commit SHA, the build's
//!   absolute URL, the orchestrator run) — optional. When it is absent the observation is made from
//!   wfapi alone and says so: no `sha:` slot, and the URL is derived from wfapi's own link.
//!
//! **The output can only express observations.** [`Observation`] has exactly two variants, a
//! `Process` and an `Event`; there is no way to hand a Commitment, Spec, Intent or Edge to the
//! sidecar through this module. Events never discharge anything (`fulfills`/`satisfies` empty)
//! and follow ark's shape: `process: None` on the event, the Process listing the event CIDs as its
//! outputs (no CID cycle).
//!
//! **The Process carries its own envelope.** Both record kinds classify the same way — tag first,
//! subject second, then the [`ExternalClaim`] slots, then the build's own (`job:`, `build:`,
//! `url:`, `sha:`, `orchestrator-run:`) — so the run reads as a claim without walking its events:
//! the Process is tagged with the BUILD's result and names `elohim-edge#<n>` as its subject, each
//! event with its STAGE's result and the stage name.

use std::collections::BTreeMap;

use cid::Cid;
use elohim_epr_rea::{
    AgentRef, ExternalClaim, FlowEvent, FlowRecord, Magnitude, PinnedRef, Process, ReaVerb,
    SignatureStatus,
};
use serde::{Deserialize, Serialize};

/// The provenance participant every record from this bridge names.
pub const SOURCE: &str = "service:jenkins";
/// The Jenkins job whose builds are the edge pipeline.
pub const EDGE_JOB: &str = "elohim-edge";
/// A fold key of its own, so a stage duration is never counted as any other unit.
pub const STAGE_UNIT: &str = "ci-stage-ms";
/// Jenkins' synthetic post-actions stage: machinery, not a stage of the chain.
pub const SYNTHETIC_PREFIX: &str = "Declarative:";
/// The public Jenkins every URL in the edge archive lives under.
pub const DEFAULT_JENKINS: &str = "https://jenkins.ethosengine.com";

/// A refusal: malformed or unknown input, refused before anything is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub String);
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Refusal {}
fn refuse(message: impl Into<String>) -> Refusal {
    Refusal(message.into())
}

// ── Jenkins' input shapes (only the fields read; unknown fields ignored) ──────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfapiRun {
    pub id: String,
    pub status: String,
    pub start_time_millis: i64,
    pub duration_millis: i64,
    #[serde(rename = "_links")]
    pub links: WfapiLinks,
    pub stages: Vec<WfapiStage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WfapiLinks {
    #[serde(rename = "self")]
    pub self_link: WfapiHref,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WfapiHref {
    pub href: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfapiStage {
    pub name: String,
    pub status: String,
    pub start_time_millis: i64,
    pub duration_millis: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGraph {
    pub schema_version: String,
    pub commit_sha: String,
    pub build_number: String,
    pub results: BTreeMap<String, GraphResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResult {
    pub result: String,
    pub url: String,
    pub build_number: u64,
}

pub fn parse_wfapi(text: &str) -> Result<WfapiRun, Refusal> {
    let run: WfapiRun = serde_json::from_str(text)
        .map_err(|e| refuse(format!("wfapi describe is malformed: {e}")))?;
    if run.id.parse::<u64>().is_err() {
        return Err(refuse(format!(
            "wfapi id `{}` is not a build number",
            run.id
        )));
    }
    if run.stages.is_empty() {
        return Err(refuse("wfapi describe names no stages"));
    }
    if matches!(
        run.status.as_str(),
        "IN_PROGRESS" | "PAUSED_PENDING_INPUT" | "NOT_EXECUTED"
    ) {
        return Err(refuse(format!(
            "build #{} is {} — observe a finished build",
            run.id, run.status
        )));
    }
    StageResult::parse(&run.status).map_err(|e| refuse(format!("build #{}: {}", run.id, e.0)))?;
    for stage in &run.stages {
        if stage.name.trim().is_empty() || stage.start_time_millis < 0 || stage.duration_millis < 0
        {
            return Err(refuse(format!(
                "wfapi stage `{}` has an empty name or a negative time",
                stage.name
            )));
        }
        StageResult::parse(&stage.status)?;
    }
    Ok(run)
}

pub fn parse_graph(text: &str) -> Result<BuildGraph, Refusal> {
    let graph: BuildGraph = serde_json::from_str(text)
        .map_err(|e| refuse(format!("actual-build-graph.json is malformed: {e}")))?;
    if graph.schema_version != "1" {
        return Err(refuse(format!(
            "actual-build-graph schemaVersion `{}` is unknown (this bridge reads 1)",
            graph.schema_version
        )));
    }
    if graph.commit_sha.len() != 40 || !graph.commit_sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(refuse(format!(
            "actual-build-graph commitSha `{}` is not a full git SHA",
            graph.commit_sha
        )));
    }
    Ok(graph)
}

/// A Jenkins stage outcome, closed. `NotExecuted` produces no event (honest absence);
/// an in-flight stage refuses the whole build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageResult {
    Success,
    Unstable,
    Failed,
    Aborted,
    NotExecuted,
}

impl StageResult {
    pub fn parse(status: &str) -> Result<Self, Refusal> {
        match status {
            "SUCCESS" => Ok(Self::Success),
            "UNSTABLE" => Ok(Self::Unstable),
            "FAILED" | "FAILURE" => Ok(Self::Failed),
            "ABORTED" => Ok(Self::Aborted),
            "NOT_EXECUTED" => Ok(Self::NotExecuted),
            other => Err(refuse(format!(
                "unknown or in-flight stage status `{other}` — observe a finished build"
            ))),
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Unstable => "unstable",
            Self::Failed => "failure",
            Self::Aborted => "aborted",
            Self::NotExecuted => "not-executed",
        }
    }

    /// SUCCESS produces; everything else that ran is a dismiss carrying its result.
    pub fn verb(self) -> Option<ReaVerb> {
        match self {
            Self::Success => Some(ReaVerb::Produce),
            Self::Unstable | Self::Failed | Self::Aborted => Some(ReaVerb::Dismiss),
            Self::NotExecuted => None,
        }
    }
}

/// One observed stage, as drift reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObservedStage {
    pub name: String,
    pub result: StageResult,
    pub start_millis: i64,
    pub duration_millis: i64,
}

/// The non-synthetic stages of a run, in the order they started.
pub fn observed_stages(run: &WfapiRun) -> Vec<ObservedStage> {
    let mut stages: Vec<ObservedStage> = run
        .stages
        .iter()
        .filter(|s| !s.name.starts_with(SYNTHETIC_PREFIX))
        .map(|s| ObservedStage {
            name: s.name.clone(),
            result: StageResult::parse(&s.status).expect("validated in parse_wfapi"),
            start_millis: s.start_time_millis,
            duration_millis: s.duration_millis,
        })
        .collect();
    stages.sort_by_key(|s| s.start_millis);
    stages
}

/// What this module can hand the sidecar. Exactly two shapes, on purpose.
// Unboxed like `FlowRecord` itself: an observation is moved straight into the sidecar record.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Observation {
    Process(Process),
    Event(FlowEvent),
}

impl Observation {
    pub fn into_record(self) -> FlowRecord {
        match self {
            Observation::Process(p) => FlowRecord::Process(p),
            Observation::Event(e) => FlowRecord::Event(e),
        }
    }

    pub fn cid(&self) -> Result<Cid, Refusal> {
        self.clone()
            .into_record()
            .cid()
            .map_err(|e| refuse(e.to_string()))
    }
}

/// One build, translated: its stage events then the Process that groups them (append order).
#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    pub build: u64,
    pub url: String,
    pub sha: Option<String>,
    pub orchestrator_run: Option<String>,
    pub process_cid: Cid,
    pub observations: Vec<Observation>,
    pub observed: Vec<ObservedStage>,
    /// `graph` or `wfapi-only` — which archive the claim was made from.
    pub basis: &'static str,
}

/// The content address of one stage claim's body: what Jenkins says this stage did.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StageClaimBody<'a> {
    source: &'a str,
    job: &'a str,
    build: u64,
    stage: &'a str,
    result: StageResult,
    start_millis: i64,
    duration_millis: i64,
}

fn rfc3339_ms(millis: i64) -> Result<String, Refusal> {
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .ok_or_else(|| refuse(format!("{millis} is not a representable instant")))
}

/// Translate one finished edge build into observations scoped to `offer`, pinned to `spec`.
///
/// `graph` is the dispatching orchestrator run's archive; it must describe THIS build, or it is
/// refused rather than mixed in.
pub fn translate(
    run: &WfapiRun,
    graph: Option<&BuildGraph>,
    spec: &PinnedRef,
    offer: Cid,
    jenkins_base: &str,
) -> Result<Translation, Refusal> {
    let build: u64 = run
        .id
        .parse()
        .map_err(|_| refuse(format!("wfapi id `{}` is not a build number", run.id)))?;
    let (url, sha, orchestrator_run, basis) = match graph {
        Some(graph) => {
            let edge = graph.results.get(EDGE_JOB).ok_or_else(|| {
                refuse(format!(
                    "orchestrator run {} dispatched no {EDGE_JOB} build",
                    graph.build_number
                ))
            })?;
            if edge.build_number != build {
                return Err(refuse(format!(
                    "orchestrator run {} dispatched {EDGE_JOB} #{}, not #{build}",
                    graph.build_number, edge.build_number
                )));
            }
            (
                edge.url.clone(),
                Some(graph.commit_sha.clone()),
                Some(graph.build_number.clone()),
                "graph",
            )
        }
        None => {
            let href = run
                .links
                .self_link
                .href
                .strip_suffix("wfapi/describe")
                .ok_or_else(|| refuse("wfapi self link is not a describe link"))?;
            (
                format!("{}{href}", jenkins_base.trim_end_matches('/')),
                None,
                None,
                "wfapi-only",
            )
        }
    };
    let claim = ExternalClaim::new(SOURCE, &url, SignatureStatus::Unattested, offer)
        .map_err(|e| refuse(e.to_string()))?;
    let observed = observed_stages(run);
    // The build's own slots, after the envelope — on every event and on the Process alike.
    let mut build_slots = vec![
        format!("job:{EDGE_JOB}"),
        format!("build:{build}"),
        format!("url:{url}"),
    ];
    if let Some(sha) = &sha {
        build_slots.push(format!("sha:{sha}"));
    }
    if let Some(run) = &orchestrator_run {
        build_slots.push(format!("orchestrator-run:{run}"));
    }

    let mut observations = Vec::new();
    let mut outputs = Vec::new();
    for stage in &observed {
        let Some(action) = stage.result.verb() else {
            continue;
        };
        let body = StageClaimBody {
            source: SOURCE,
            job: EDGE_JOB,
            build,
            stage: &stage.name,
            result: stage.result,
            start_millis: stage.start_millis,
            duration_millis: stage.duration_millis,
        };
        let resource = elohim_epr::cid::compute_cid(
            &serde_ipld_dagcbor::to_vec(&body).map_err(|e| refuse(e.to_string()))?,
        );
        // Tag first, subject second, then the envelope and the build's own slots, prefixed.
        let classified_as = claim.classify(
            format!("result:{}", stage.result.word()),
            stage.name.clone(),
            build_slots.iter().cloned(),
        );
        let event = FlowEvent {
            action,
            provider: AgentRef(SOURCE.to_string()),
            receiver: AgentRef(elohim_epr_cli::flow::REPO_AGENT.to_string()),
            resource,
            quantity: Magnitude::Count {
                value: stage.duration_millis as f64,
                unit: STAGE_UNIT.to_string(),
            },
            process: None,
            in_scope_of: offer,
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            occurred_at: rfc3339_ms(stage.start_millis + stage.duration_millis)?,
            classified_as,
        };
        let observation = Observation::Event(event);
        outputs.push(observation.cid()?);
        observations.push(observation);
    }
    let process = Observation::Process(Process {
        spec: spec.clone(),
        in_scope_of: offer,
        inputs: Vec::new(),
        outputs,
        classified_as: claim.classify(
            format!("result:{}", StageResult::parse(&run.status)?.word()),
            format!("{EDGE_JOB}#{build}"),
            build_slots,
        ),
    });
    let process_cid = process.cid()?;
    observations.push(process);
    Ok(Translation {
        build,
        url,
        sha,
        orchestrator_run,
        process_cid,
        observations,
        observed,
        basis,
    })
}
