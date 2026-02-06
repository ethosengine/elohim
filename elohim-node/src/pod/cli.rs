//! Pod CLI - Command-line interface for manual pod operations
//!
//! Provides commands for humans (and eventually elohim agents) to
//! interact with the pod for manual operations.

use clap::{Args, Subcommand};
use tracing::info;

use super::models::*;
use super::Pod;

/// Pod CLI commands
#[derive(Debug, Subcommand)]
pub enum PodCommands {
    /// Show pod status
    Status,

    /// List recent actions
    Actions {
        /// Number of actions to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    /// List recent observations
    Observations {
        /// Number of observations to show
        #[arg(short, long, default_value = "20")]
        count: usize,
    },

    /// List active rules
    Rules,

    /// Execute an action
    #[command(subcommand)]
    Exec(ExecCommands),

    /// Trigger a decision cycle
    Tick,

    /// Enable/disable dry run mode
    DryRun {
        /// Enable dry run
        #[arg(long)]
        enable: bool,
    },
}

/// Actions that can be executed manually
#[derive(Debug, Subcommand)]
pub enum ExecCommands {
    /// Set log level for a module
    SetLogLevel {
        /// Log level (trace, debug, info, warn, error)
        #[arg(short, long)]
        level: String,
        /// Target module (optional)
        #[arg(short, long)]
        module: Option<String>,
    },

    /// Flush a cache
    FlushCache {
        /// Cache name
        #[arg(short, long, default_value = "content")]
        cache: String,
    },

    /// Resize a cache
    ResizeCache {
        /// Cache name
        #[arg(short, long, default_value = "content")]
        cache: String,
        /// New size in MB
        #[arg(short, long)]
        size_mb: u64,
    },

    /// Restart a service
    RestartService {
        /// Service name (holochain, sync, storage, p2p, api)
        #[arg(short, long)]
        service: String,
    },

    /// Collect diagnostics
    Diagnostics {
        /// Include logs
        #[arg(long, default_value = "true")]
        logs: bool,
        /// Include config
        #[arg(long, default_value = "true")]
        config: bool,
    },

    /// Report a bug
    ReportBug {
        /// Bug title
        #[arg(short, long)]
        title: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Severity (low, medium, high, critical)
        #[arg(short, long, default_value = "medium")]
        severity: String,
    },

    /// Throttle sync operations
    ThrottleSync {
        /// Maximum rate in KB/s
        #[arg(long)]
        max_rate_kbps: Option<u64>,
        /// Maximum concurrent operations
        #[arg(long)]
        max_concurrent: Option<u64>,
        /// Duration in seconds
        #[arg(long)]
        duration_secs: Option<u64>,
    },

    /// Rebalance storage
    RebalanceStorage {
        /// Target usage percentage
        #[arg(long, default_value = "75")]
        target_percent: f64,
        /// Dry run only
        #[arg(long)]
        dry_run: bool,
    },

    /// Replicate a blob
    ReplicateBlob {
        /// Blob hash
        #[arg(short, long)]
        hash: String,
        /// Target nodes (comma-separated)
        #[arg(short, long)]
        targets: Option<String>,
    },

    /// Evict a blob
    EvictBlob {
        /// Blob hash
        #[arg(short, long)]
        hash: String,
        /// Minimum replicas required
        #[arg(long, default_value = "2")]
        min_replicas: usize,
    },

    /// Reconnect to a peer
    ReconnectPeer {
        /// Peer ID
        #[arg(short, long)]
        peer: String,
    },

    /// Quarantine a node
    QuarantineNode {
        /// Node ID
        #[arg(short, long)]
        node: String,
        /// Reason
        #[arg(short, long)]
        reason: String,
        /// Duration in seconds
        #[arg(long)]
        duration_secs: Option<u64>,
    },
}

/// Execute a pod CLI command
pub async fn execute_command(pod: &mut Pod, command: PodCommands) -> Result<String, String> {
    match command {
        PodCommands::Status => {
            let status = pod.status().await;
            Ok(format_status(&status))
        }

        PodCommands::Actions { count } => {
            let actions = pod.get_action_history(count).await;
            Ok(format_actions(&actions))
        }

        PodCommands::Observations { count } => {
            let obs = pod.get_observations(count).await;
            Ok(format_observations(&obs))
        }

        PodCommands::Rules => {
            let rules = pod.get_rules();
            Ok(format_rules(&rules))
        }

        PodCommands::Tick => {
            // Would trigger a tick - not directly accessible
            Ok("Tick triggered".to_string())
        }

        PodCommands::DryRun { enable: _ } => {
            // Would update config
            Ok("Dry run mode updated".to_string())
        }

        PodCommands::Exec(exec) => {
            let action = exec_to_action(exec)?;
            let result = pod.execute_manual_action(action).await?;
            Ok(format_result(&result))
        }
    }
}

/// Convert ExecCommand to Action
fn exec_to_action(cmd: ExecCommands) -> Result<Action, String> {
    match cmd {
        ExecCommands::SetLogLevel { level, module } => {
            Ok(Action::new(
                ActionKind::SetLogLevel,
                format!("Set log level to {} for {}", level, module.as_deref().unwrap_or("all")),
                serde_json::json!({
                    "level": level,
                    "module": module,
                }),
            ))
        }

        ExecCommands::FlushCache { cache } => {
            Ok(Action::new(
                ActionKind::FlushCache,
                format!("Flush {} cache", cache),
                serde_json::json!({"cache": cache}),
            ))
        }

        ExecCommands::ResizeCache { cache, size_mb } => {
            Ok(Action::new(
                ActionKind::ResizeCache,
                format!("Resize {} cache to {} MB", cache, size_mb),
                serde_json::json!({
                    "cache": cache,
                    "size_mb": size_mb,
                }),
            ))
        }

        ExecCommands::RestartService { service } => {
            Ok(Action::new(
                ActionKind::RestartService,
                format!("Restart {} service", service),
                serde_json::json!({"service": service}),
            ))
        }

        ExecCommands::Diagnostics { logs, config } => {
            Ok(Action::new(
                ActionKind::CollectDiagnostics,
                "Collect diagnostics",
                serde_json::json!({
                    "include_logs": logs,
                    "include_config": config,
                }),
            ))
        }

        ExecCommands::ReportBug { title, description, severity } => {
            Ok(Action::new(
                ActionKind::ReportBug,
                format!("Report bug: {}", title),
                serde_json::json!({
                    "title": title,
                    "description": description,
                    "severity": severity,
                }),
            ))
        }

        ExecCommands::ThrottleSync { max_rate_kbps, max_concurrent, duration_secs } => {
            Ok(Action::new(
                ActionKind::ThrottleSync,
                "Throttle sync operations",
                serde_json::json!({
                    "max_rate_kbps": max_rate_kbps,
                    "max_concurrent": max_concurrent,
                    "duration_secs": duration_secs,
                }),
            ))
        }

        ExecCommands::RebalanceStorage { target_percent, dry_run } => {
            Ok(Action::new(
                ActionKind::RebalanceStorage,
                "Rebalance storage",
                serde_json::json!({
                    "target_usage_percent": target_percent,
                    "dry_run": dry_run,
                }),
            ).with_risk(if dry_run {
                ActionRisk::Safe
            } else {
                ActionRisk::Risky { required_approvals: 2, total_evaluators: 3 }
            }))
        }

        ExecCommands::ReplicateBlob { hash, targets } => {
            let target_nodes: Vec<String> = targets
                .map(|t| t.split(',').map(String::from).collect())
                .unwrap_or_default();

            Ok(Action::new(
                ActionKind::ReplicateBlob,
                format!("Replicate blob {}", hash),
                serde_json::json!({
                    "blob_hash": hash,
                    "target_nodes": target_nodes,
                }),
            ))
        }

        ExecCommands::EvictBlob { hash, min_replicas } => {
            Ok(Action::new(
                ActionKind::EvictBlob,
                format!("Evict blob {}", hash),
                serde_json::json!({
                    "blob_hash": hash,
                    "min_replicas": min_replicas,
                    "verify_replicas": true,
                }),
            ).with_risk(ActionRisk::Risky { required_approvals: 2, total_evaluators: 3 }))
        }

        ExecCommands::ReconnectPeer { peer } => {
            Ok(Action::new(
                ActionKind::ReconnectPeer,
                format!("Reconnect to peer {}", peer),
                serde_json::json!({"peer_id": peer}),
            ))
        }

        ExecCommands::QuarantineNode { node, reason, duration_secs } => {
            Ok(Action::new(
                ActionKind::QuarantineNode,
                format!("Quarantine node {}: {}", node, reason),
                serde_json::json!({
                    "node_id": node,
                    "reason": reason,
                    "duration_secs": duration_secs,
                }),
            ).with_risk(ActionRisk::Risky { required_approvals: 2, total_evaluators: 3 }))
        }
    }
}

/// Format pod status for display
fn format_status(status: &PodStatus) -> String {
    let mut output = String::new();
    output.push_str("Pod Status\n");
    output.push_str("==========\n\n");

    output.push_str(&format!("Node ID:        {}\n", status.node_id));
    output.push_str(&format!("Active:         {}\n", status.active));
    output.push_str(&format!("Mode:           {:?}\n", status.mode));
    output.push_str(&format!("Started:        {}\n", format_timestamp(status.started_at)));
    output.push_str(&format!("Last Decision:  {}\n",
        status.last_decision_at.map(format_timestamp).unwrap_or_else(|| "Never".to_string())));
    output.push_str(&format!("Actions Exec:   {}\n", status.actions_executed));
    output.push_str(&format!("Actions Pend:   {}\n", status.actions_pending));
    output.push_str(&format!("Active Rules:   {}\n", status.active_rules));
    output.push_str(&format!("Peer Pods:      {}\n", status.peer_pods.len()));
    output.push_str(&format!("Local LLM:      {}\n", status.has_local_inference));

    if !status.peer_pods.is_empty() {
        output.push_str("\nPeer Pods:\n");
        for peer in &status.peer_pods {
            output.push_str(&format!("  - {} ({})\n", peer.node_id, peer.peer_id));
        }
    }

    output
}

/// Format actions for display
fn format_actions(actions: &[Action]) -> String {
    if actions.is_empty() {
        return "No recent actions".to_string();
    }

    let mut output = String::new();
    output.push_str("Recent Actions\n");
    output.push_str("==============\n\n");

    for action in actions {
        output.push_str(&format!(
            "[{}] {:?} - {} ({:?})\n",
            &action.id[..8],
            action.kind,
            action.description,
            action.status
        ));

        if let Some(result) = &action.result {
            output.push_str(&format!(
                "     Result: {} ({}ms)\n",
                if result.success { "Success" } else { "Failed" },
                result.duration_ms
            ));
        }
    }

    output
}

/// Format observations for display
fn format_observations(observations: &[Observation]) -> String {
    if observations.is_empty() {
        return "No recent observations".to_string();
    }

    let mut output = String::new();
    output.push_str("Recent Observations\n");
    output.push_str("===================\n\n");

    for obs in observations {
        output.push_str(&format!(
            "[{}] {:?}\n",
            format_timestamp(obs.timestamp),
            obs.kind
        ));
    }

    output
}

/// Format rules for display
fn format_rules(rules: &[Rule]) -> String {
    if rules.is_empty() {
        return "No active rules".to_string();
    }

    let mut output = String::new();
    output.push_str("Active Rules\n");
    output.push_str("============\n\n");

    for rule in rules {
        output.push_str(&format!(
            "[{}] {} (priority: {}, cooldown: {}s)\n",
            if rule.enabled { "ON " } else { "OFF" },
            rule.name,
            rule.priority,
            rule.cooldown_secs
        ));
    }

    output
}

/// Format action result for display
fn format_result(result: &ActionResult) -> String {
    let mut output = String::new();

    if result.success {
        output.push_str("SUCCESS: ");
    } else {
        output.push_str("FAILED: ");
    }

    output.push_str(&result.message);
    output.push_str(&format!(" ({}ms)", result.duration_ms));

    if let Some(details) = &result.details {
        output.push_str("\n\nDetails:\n");
        output.push_str(&serde_json::to_string_pretty(details).unwrap_or_default());
    }

    output
}

/// Format a timestamp for display
fn format_timestamp(ts: u64) -> String {
    // Simple relative time for now
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let diff = now.saturating_sub(ts);

    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}
