//! Offline feedback authoring and contextual reading. Dispatched before repository/session
//! discovery: recording never needs HEAD, Git, network, or a conductor.

mod capacity;
mod genesis;
mod qualify;
mod repair;
mod response;

use cid::Cid;
use elohim_epr_rea::{
    feedback::{experiences, Feedback, Occurrence, Outcome, Policy, Relation, Source},
    feedback_store::FeedbackStore,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const INPUT_LIMIT: u64 = 2 * 1024 * 1024;
const HELP: &str = "epr feedback <qualify|capacity|reclaim|migrate|policy|capture|record|list|show|group|evidence|triage|propose|link|repair|ingest-genesis> [--root DIR] [--json]
  qualify --brit PATH          Qualify this recorder and an explicit Brit in an isolated store
  qualify --recorder-only      Check initial recorder arrival; never qualifies the tool pair
  capacity                     Effective policy, file accounting, eligibility and waiting reasons
  reclaim [--dry-run]           Plan or execute policy-authorized local reclamation
  migrate --legacy-writers-quiesced
                               Explicit offline v2 cutover; stop all legacy writers first
  policy                       Explain the nearest contextual capture/retention/response policy
  capture --producer brit|brit-guard --operation NAME --outcome STATUS [--invocation ID] [--elapsed-ms N]
                               Structured automatic capture; no arguments or diagnostics
  record [--protect]           Deliberately author a Feedback JSON atom on stdin
  list                         Retained experiences, distinct from rebuildable interpretations
  show --id CID                One immutable record and its governing policy
  group --ids CID,CID --as ACTOR --summary TEXT [--objective TEXT]
                               Author an interpretation; never rewrite an occurrence
  evidence --id CID --evidence CID --relation attempt|workaround|verified-fix|available-release|experienced-recovery|routine-success|protected --as ACTOR --outcome STATUS
                               Link evidence; recovery requires a retained participant experience
  triage [--response report|propose|repair] [--as ACTOR]
                               Context, evidence and waiting reasons; never executes work
  propose --id CID --as ACTOR --objective TEXT
                               Create an existing REA Intent, not an admitted commitment
  link --id CID --commitment CID
                               Link an existing REA commitment; never fulfils it
  repair --id CID --contract FILE --runtime codex|claude
                               Hand an explicit local-delivery contract to the existing shift launcher
  ingest-genesis               Explicit single-report ingestion from stdin; no publication
Private evidence stays local. Sharing requires separate authorization. Repair executes only
through existing claim, authority, isolation, budget and gate admission (just dev shift).";

pub fn run(args: &[String]) -> Result<()> {
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let command = &args[0];
    let (options, machine) = options(&args[1..])?;
    if command == "qualify" {
        return qualify::run(&options, machine);
    }
    if options.contains_key("brit") || options.contains_key("recorder-only") {
        return Err("--brit and --recorder-only apply only to qualify".into());
    }
    let root = policy_root(options.get("root").map(String::as_str))?;
    let policy = load_policy(&root)?;
    let store = FeedbackStore::new(&root, policy.clone())?;
    let value = match command.as_str() {
        "measure" => {
            let report = serde_json::to_value(store.capacity()?)?;
            let key = match required(&options, "measure")? {
                "feedback-retained-bytes@1" => "retainedBytes",
                "feedback-managed-bytes@1" => "totalManagedFileBytes",
                "feedback-reclaimable-bytes@1" => "reclaimableBytes",
                "feedback-protected-bytes@1" => "protectedBytes",
                "feedback-rejected-capture@1" => "rejectedCapture",
                _ => return Err("unknown feedback capacity measure version".into()),
            };
            json!({"value":report[key],"env":{"policyCid":report["policy"]["policyCid"]},
                "detail":["Private local file-byte accounting; retained history only."]})
        }
        "capacity" => match store.capacity() {
            Ok(report) => serde_json::to_value(report)?,
            Err(error) => {
                json!({"status":"unknown", "policy":policy.capacity, "retainedBytes":null, "totalManagedFileBytes":null,
                "reclaimableBytes":null,"protectedBytes":null,"rejectedCapture":null,"waiting":[error.to_string()]})
            }
        },
        "reclaim" => serde_json::to_value(store.reclaim(options.contains_key("dry-run"))?)?,
        "migrate" => {
            serde_json::to_value(store.migrate(options.contains_key("legacy-writers-quiesced"))?)?
        }
        "policy" => json!({"policy":policy,"source":root.join(".epr-meta/feedback-policy.json"),
            "retention":"private retained evidence; summaries rebuildable; only policy-eligible groups may be reclaimed",
            "coverage":"native adapters only; unsupported shell adapters are unmeasured"}),
        "capture" => {
            let record = capture(&options, &policy)?;
            json!({"cid":store.append(Feedback::Occurrence(record), false)?.to_string()})
        }
        "record" => {
            let record: Feedback = serde_json::from_slice(&input()?)?;
            // Commitment links have a stronger boundary than generic authored evidence.
            if let Feedback::CommitmentLink { commitment, .. } = &record {
                response::check_commitment(&root, commitment)?;
            }
            json!({"cid":store.append_protected(record, true, options.contains_key("protect"))?.to_string()})
        }
        "ingest-genesis" => genesis::ingest(&store, &policy, &serde_json::from_slice(&input()?)?)?,
        "list" => json!({"experiences":experiences(&store.read()?),"policy":policy.id}),
        "show" => {
            let id: Cid = required(&options, "id")?.parse()?;
            serde_json::to_value(
                store
                    .read()?
                    .into_iter()
                    .find(|e| e.cid == id)
                    .ok_or("feedback not found")?,
            )?
        }
        "triage" => response::triage(&root, &store, &policy, &options)?,
        "group" => {
            let occurrences = required(&options, "ids")?
                .split(',')
                .map(str::parse)
                .collect::<std::result::Result<Vec<Cid>, _>>()?;
            let record = Feedback::Interpretation {
                occurrences,
                author: required(&options, "as")?.into(),
                at: now(),
                summary: required(&options, "summary")?.into(),
                objective: options.get("objective").cloned(),
            };
            json!({"cid":store.append(record,true)?.to_string()})
        }
        "evidence" => {
            let relation: Relation =
                serde_json::from_value(json!(required(&options, "relation")?))?;
            let outcome: Outcome = serde_json::from_value(json!(required(&options, "outcome")?))?;
            let record = Feedback::EvidenceLink {
                occurrence: required(&options, "id")?.parse()?,
                evidence: required(&options, "evidence")?.parse()?,
                relation,
                observer: required(&options, "as")?.into(),
                at: now(),
                outcome,
            };
            json!({"cid":store.append(record,true)?.to_string()})
        }
        "propose" => response::propose(&root, &store, &policy, &options)?,
        "repair" => repair::handoff(&root, &store, &policy, &options)?,
        "link" => {
            let occurrence = required(&options, "id")?.parse()?;
            let commitment = required(&options, "commitment")?.parse()?;
            response::check_commitment(&root, &commitment)?;
            json!({"cid":store.append(Feedback::CommitmentLink {occurrence,commitment},true)?.to_string()})
        }
        _ => return Err(format!("unknown feedback command: {command}").into()),
    };
    if !machine && ["capacity", "reclaim", "migrate"].contains(&command.as_str()) {
        capacity::print(&value);
    } else if machine {
        println!("{}", serde_json::to_string(&value)?);
    } else if let Some(rows) = value["experiences"].as_array() {
        println!(
            "Private feedback · policy {}",
            value["policy"].as_str().unwrap_or("unknown")
        );
        for row in rows {
            let o = &row["occurrence"];
            println!(
                "{} · {} · {} reported · {} source runs · cost {} ms · recovery {}",
                o["subject"].as_str().unwrap_or("unknown"),
                o["outcome"].as_str().unwrap_or("unknown"),
                o["count"],
                row["observedRuns"],
                o["durationMs"]
                    .as_u64()
                    .map_or("unknown".into(), |n| n.to_string()),
                if row["experiencedRecovery"] == true {
                    "observed"
                } else {
                    "not observed"
                }
            );
            if let Some(message) = o["message"].as_str() {
                println!("  {message}");
            }
        }
        println!(
            "{} retained occurrence records; use --json for addresses and evidence links",
            rows.len()
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}

fn options(args: &[String]) -> Result<(BTreeMap<String, String>, bool)> {
    let mut result = BTreeMap::new();
    let mut machine = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--json" {
            machine = true;
            i += 1;
            continue;
        }
        if [
            "--dry-run",
            "--legacy-writers-quiesced",
            "--protect",
            "--recorder-only",
        ]
        .contains(&args[i].as_str())
        {
            if result
                .insert(args[i][2..].to_string(), "true".into())
                .is_some()
            {
                return Err("duplicate option".into());
            }
            i += 1;
            continue;
        }
        let key = args[i].strip_prefix("--").ok_or("expected named option")?;
        if ![
            "root",
            "brit",
            "measure",
            "producer",
            "operation",
            "outcome",
            "invocation",
            "elapsed-ms",
            "id",
            "response",
            "as",
            "objective",
            "commitment",
            "ids",
            "summary",
            "evidence",
            "relation",
            "contract",
            "runtime",
        ]
        .contains(&key)
        {
            return Err("unknown feedback option".into());
        }
        let value = args.get(i + 1).ok_or("option requires value")?;
        if result.insert(key.to_owned(), value.clone()).is_some() {
            return Err("duplicate option".into());
        }
        i += 2;
    }
    Ok((result, machine))
}

fn required<'a>(options: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    options
        .get(key)
        .map(String::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("--{key} required").into())
}

/// Read the explicitly selected workspace's contextual policy without ambient root discovery.
pub(crate) fn load_policy(root: &Path) -> Result<Policy> {
    let mut policy: Policy =
        serde_json::from_slice(&std::fs::read(root.join(".epr-meta/feedback-policy.json"))?)?;
    capacity::resolve(root, &mut policy)?;
    policy.validate()?;
    Ok(policy)
}

fn policy_root(explicit: Option<&str>) -> Result<PathBuf> {
    if let Some(root) = explicit {
        return Ok(PathBuf::from(root));
    }
    if let Some(root) = std::env::var_os("EPR_FEEDBACK_ROOT") {
        return Ok(root.into());
    }
    let cwd = std::env::current_dir()?;
    cwd.ancestors()
        .find(|p| p.join(".epr-meta/feedback-policy.json").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| "no contextual feedback policy; capture is unmeasured".into())
}

fn input() -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(INPUT_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > INPUT_LIMIT {
        return Err("feedback input limit exceeded".into());
    }
    Ok(bytes)
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn capture(options: &BTreeMap<String, String>, policy: &Policy) -> Result<Occurrence> {
    let producer = required(options, "producer")?;
    if !["brit", "brit-guard"].contains(&producer) {
        return Err("automatic producer requires a registered adapter".into());
    }
    let operation = required(options, "operation")?;
    // Adapter sends parser-owned names, never the raw first argument. This closed floor also
    // stops a mismatched adapter accidentally storing a path/URL/secret in an operation field.
    if ![
        "status",
        "log",
        "help",
        "version",
        "surface",
        "parse",
        "bridge",
        "guard",
        "build",
        "snapshot",
        "context",
        "integrate",
        "init",
        "clone",
        "fetch",
        "push",
        "pull",
        "add",
        "commit",
        "diff",
        "branch",
        "switch",
        "merge",
        "rebase",
        "restore",
        "tag",
        "remote",
        "submodule",
        "other",
    ]
    .contains(&operation)
    {
        return Err("unknown structured operation".into());
    }
    let outcome: Outcome = serde_json::from_value(json!(required(options, "outcome")?))?;
    let invocation = options.get("invocation").cloned().unwrap_or_else(|| {
        format!(
            "{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    });
    if invocation.len() > 128
        || !invocation
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("invocation must be an opaque alphanumeric token".into());
    }
    Ok(Occurrence {
        source: Source {
            producer: producer.into(),
            run: invocation,
            item: if producer == "brit-guard" {
                format!("{operation}:{}", required(options, "outcome")?)
            } else {
                operation.into()
            },
        },
        observed_at: now(),
        participant: policy.participant.clone(),
        subject: format!("{producer}:{operation}"),
        outcome,
        count: 1,
        duration_ms: options.get("elapsed-ms").map(|s| s.parse()).transpose()?,
        impact: None,
        message: None,
        detail: Value::Null,
    })
}
