//! The last `epr bin follow` attempt per followed tool, as `epr peer status` reports it.
//!
//! Read from each tool store's private `follow.json`: device operational state, never
//! notarized or synced. The installed head stays `current.json`'s to name.
use elohim_device_runtime::bin_store::{
    FollowOutcome, FollowRecord, LastFollow, Refusal, Registry, Store,
};
use serde::Serialize;
use std::path::Path;

/// One tool's last follow attempt. `state` says why `record` or `detail` is absent, so no
/// absence is ambiguous (views CONVENTIONS rule 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub(super) enum ToolFollow {
    /// The store holds no follow record: no attempt was ever recorded.
    Never { tool: String },
    /// A terminal record.
    Recorded { tool: String, record: FollowRecord },
    /// A pending record whose owner process was observed running.
    InProgress { tool: String, record: FollowRecord },
    /// A pending record whose owner could not be observed; `detail` says why.
    Unknown {
        tool: String,
        record: FollowRecord,
        detail: String,
    },
    /// A pending record that will never finish; `detail` says what ended it.
    Interrupted {
        tool: String,
        record: FollowRecord,
        detail: String,
    },
    /// The record exists but cannot be read or fails validation.
    Corrupt { tool: String, detail: String },
    /// The tool's store (or, as tool `*`, the tool registry) could not be opened.
    Unavailable { tool: String, detail: String },
}

/// Each registered tool's last follow attempt, in registry order.
pub(super) fn follow_outcomes(proc_root: &Path) -> Vec<ToolFollow> {
    let registry = match Registry::discover() {
        Ok(registry) => registry,
        Err(error) => {
            return vec![ToolFollow::Unavailable {
                tool: "*".into(),
                detail: format!("tool registry: {error}"),
            }]
        }
    };
    registry
        .tools
        .keys()
        .map(|tool| tool_follow(tool, Store::from_environment(tool, &registry), proc_root))
        .collect()
}

/// One tool's last follow attempt from its (possibly unopenable) store.
pub(super) fn tool_follow(
    tool: &str,
    store: Result<Store, Refusal>,
    proc_root: &Path,
) -> ToolFollow {
    let tool = tool.to_owned();
    let store = match store {
        Ok(store) => store,
        Err(error) => {
            return ToolFollow::Unavailable {
                tool,
                detail: error.to_string(),
            }
        }
    };
    match store.last_follow(proc_root) {
        LastFollow::Never => ToolFollow::Never { tool },
        LastFollow::Recorded(record) => ToolFollow::Recorded { tool, record },
        LastFollow::InProgress(record) => ToolFollow::InProgress { tool, record },
        LastFollow::Unknown { record, detail } => ToolFollow::Unknown {
            tool,
            record,
            detail,
        },
        LastFollow::Interrupted { record, detail } => ToolFollow::Interrupted {
            tool,
            record,
            detail,
        },
        LastFollow::Corrupt(detail) => ToolFollow::Corrupt { tool, detail },
    }
}

/// Character-safe shortening: never slices inside a code point.
fn shorten(text: &str, head: usize, tail: usize) -> String {
    let count = text.chars().count();
    if count <= head + tail + 1 {
        return text.to_owned();
    }
    let start: String = text.chars().take(head).collect();
    let end: String = text.chars().skip(count - tail).collect();
    format!("{start}…{end}")
}

fn commit(record: &FollowRecord) -> String {
    record.commit.as_deref().map_or_else(
        || "commit unknown".into(),
        |c| format!("commit {}", c.chars().take(12).collect::<String>()),
    )
}

fn cid(cid: Option<&String>) -> String {
    cid.map_or_else(|| "none".into(), |cid| shorten(cid, 16, 4))
}

fn pending(tool: &str, label: &str, record: &FollowRecord) -> String {
    format!(
        "follow {tool}: {label} — {} attempt started {} by pid {} ({}, source {})",
        record.mode,
        record.started_at,
        record.owner_pid,
        commit(record),
        record.source
    )
}

/// One text line per tool, for `epr peer status`.
pub(super) fn follow_lines(outcomes: &[ToolFollow]) -> Vec<String> {
    outcomes
        .iter()
        .map(|outcome| match outcome {
            ToolFollow::Never { tool } => format!("follow {tool}: never attempted"),
            ToolFollow::Corrupt { tool, detail } => {
                format!("follow {tool}: CORRUPT record — {detail}")
            }
            ToolFollow::Unavailable { tool, detail } => {
                format!("follow {tool}: store unavailable — {detail}")
            }
            ToolFollow::InProgress { tool, record } => pending(tool, "in progress", record),
            ToolFollow::Unknown {
                tool,
                record,
                detail,
            } => format!(
                "{}; owner liveness unknown: {detail}",
                pending(tool, "pending", record)
            ),
            ToolFollow::Interrupted {
                tool,
                record,
                detail,
            } => format!(
                "{}; never finished: {detail}",
                pending(tool, "INTERRUPTED", record)
            ),
            ToolFollow::Recorded { tool, record } => {
                let at = record.ended_at.as_deref().unwrap_or(&record.started_at);
                let note = match (&record.kind, &record.reason) {
                    (Some(kind), Some(reason)) => format!("{kind}: {reason}"),
                    (Some(kind), None) => kind.clone(),
                    (None, Some(reason)) => reason.clone(),
                    (None, None) => "no reason recorded".into(),
                };
                match record.outcome {
                    FollowOutcome::Pending => pending(tool, "pending", record),
                    FollowOutcome::Landed => {
                        let line = format!(
                            "follow {tool}: landed {} (was {}) at {at} ({}, {})",
                            cid(record.result_cid.as_ref()),
                            cid(record.prior_cid.as_ref()),
                            commit(record),
                            record.mode
                        );
                        if record.kind.is_some() {
                            format!("{line} — {note}")
                        } else {
                            line
                        }
                    }
                    FollowOutcome::Observed => format!(
                        "follow {tool}: observed {} at {at} ({}, head stays {})",
                        cid(record.candidate_cid.as_ref()),
                        commit(record),
                        cid(record.result_cid.as_ref())
                    ),
                    FollowOutcome::Unchanged | FollowOutcome::Refused => format!(
                        "follow {tool}: {} at {at} ({}, {}, head stays {}) — {note}",
                        record.outcome.as_str(),
                        commit(record),
                        record.mode,
                        cid(record.result_cid.as_ref()),
                    ),
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn store(root: &Path, tool: &str) -> Store {
        let registry = Registry {
            version: 1,
            tools: BTreeMap::from([(tool.to_owned(), serde_json::json!({"bins": [tool]}))]),
        };
        Store::new(root, tool, &registry).expect("store")
    }

    fn record(outcome: FollowOutcome) -> FollowRecord {
        let landed = eprfs_core::BlobCid::compute_bytes(b"landed").to_string();
        let prior = eprfs_core::BlobCid::compute_bytes(b"prior").to_string();
        FollowRecord {
            version: 1,
            attempt: "1-2-3".into(),
            started_at: "2026-11-15T10:00:00Z".into(),
            ended_at: (outcome != FollowOutcome::Pending).then(|| "2026-11-15T10:00:05Z".into()),
            owner_pid: u32::MAX,
            owner_start_time: Some("1".into()),
            source: "/projects/elohim".into(),
            commit: Some("e239f3393".repeat(5)),
            mode: "apply".into(),
            prior_cid: Some(prior.clone()),
            candidate_cid: None,
            result_cid: Some(if outcome == FollowOutcome::Landed {
                landed
            } else {
                prior
            }),
            outcome,
            kind: (outcome == FollowOutcome::Refused).then(|| "non-descendant".into()),
            reason: (outcome == FollowOutcome::Refused)
                .then(|| "HEAD does not descend from the landed head".into()),
        }
    }

    fn write(root: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(root).expect("store dir");
        std::fs::write(root.join("follow.json"), bytes).expect("follow.json");
    }

    #[test]
    fn follow_status_distinguishes_never_corrupt_interrupted_and_recorded() {
        let tmp = tempfile::tempdir().expect("tmp");
        let proc_root = Path::new("/proc");
        let root = tmp.path().join("epr");
        let read = || tool_follow("epr", Ok(store(&root, "epr")), proc_root);
        assert_eq!(read(), ToolFollow::Never { tool: "epr".into() });

        write(&root, b"{truncated");
        assert!(matches!(read(), ToolFollow::Corrupt { .. }));

        let pending = record(FollowOutcome::Pending);
        write(&root, &serde_json::to_vec(&pending).expect("json"));
        assert!(matches!(read(), ToolFollow::Interrupted { .. }));

        let refused = record(FollowOutcome::Refused);
        write(&root, &serde_json::to_vec(&refused).expect("json"));
        assert_eq!(
            read(),
            ToolFollow::Recorded {
                tool: "epr".into(),
                record: refused
            }
        );

        let unavailable = tool_follow(
            "brit",
            Err(Refusal {
                kind: "registry-invalid".into(),
                detail: "fixture".into(),
            }),
            proc_root,
        );
        assert!(matches!(unavailable, ToolFollow::Unavailable { .. }));
    }

    // Review item 5: a valid-JSON record with a multibyte commit or a bad mode never panics
    // status and never reaches the wire as anything but corrupt.
    #[test]
    fn follow_status_survives_a_syntactically_valid_wrong_record() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("epr");
        let mut odd = record(FollowOutcome::Refused);
        odd.commit = Some("aaaaaaaaaaaé".into());
        odd.source = "é".repeat(40);
        let line = follow_lines(&[ToolFollow::Recorded {
            tool: "epr".into(),
            record: odd.clone(),
        }]);
        assert!(line[0].contains("commit aaaaaaaaaaaé"));
        odd.mode = "sideways".into();
        write(&root, &serde_json::to_vec(&odd).expect("json"));
        assert!(matches!(
            tool_follow("epr", Ok(store(&root, "epr")), Path::new("/proc")),
            ToolFollow::Corrupt { .. }
        ));
        assert_eq!(
            shorten("ééééééééééééééééééééééééééé", 16, 4)
                .chars()
                .count(),
            21
        );
    }

    #[test]
    fn follow_status_names_index_warming() {
        let mut warming = record(FollowOutcome::Refused);
        warming.kind = Some("index-warming".into());
        warming.reason = Some("the sibling is still indexing".into());

        let lines = follow_lines(&[ToolFollow::Recorded {
            tool: "epr".into(),
            record: warming,
        }]);

        assert!(
            lines[0].ends_with("— index-warming: the sibling is still indexing"),
            "{}",
            lines[0]
        );
        assert!(!lines[0].contains("no record"), "{}", lines[0]);
    }

    #[test]
    fn follow_status_renders_one_line_per_state() {
        let pending = record(FollowOutcome::Pending);
        let mut landed_late = record(FollowOutcome::Landed);
        landed_late.kind = Some("io".into());
        landed_late.reason = Some("head committed, then: Is a directory".into());
        let lines = follow_lines(&[
            ToolFollow::Never { tool: "a".into() },
            ToolFollow::Recorded {
                tool: "b".into(),
                record: record(FollowOutcome::Refused),
            },
            ToolFollow::Recorded {
                tool: "c".into(),
                record: record(FollowOutcome::Landed),
            },
            ToolFollow::Recorded {
                tool: "d".into(),
                record: record(FollowOutcome::Unchanged),
            },
            ToolFollow::Recorded {
                tool: "e".into(),
                record: record(FollowOutcome::Observed),
            },
            ToolFollow::InProgress {
                tool: "f".into(),
                record: pending.clone(),
            },
            ToolFollow::Interrupted {
                tool: "g".into(),
                record: pending.clone(),
                detail: "owner process 7 is a zombie".into(),
            },
            ToolFollow::Unknown {
                tool: "u".into(),
                record: pending,
                detail: "process table /proc is unavailable".into(),
            },
            ToolFollow::Corrupt {
                tool: "h".into(),
                detail: "expected value".into(),
            },
            ToolFollow::Unavailable {
                tool: "*".into(),
                detail: "tool registry: missing".into(),
            },
            ToolFollow::Recorded {
                tool: "l".into(),
                record: landed_late,
            },
        ]);
        assert_eq!(lines.len(), 11);
        assert_eq!(lines[0], "follow a: never attempted");
        assert!(lines[1].starts_with("follow b: refused at 2026-11-15T10:00:05Z"));
        assert!(lines[1].ends_with("non-descendant: HEAD does not descend from the landed head"));
        assert!(lines[2].starts_with("follow c: landed "));
        assert!(lines[3].starts_with("follow d: unchanged "));
        assert!(lines[4].starts_with("follow e: observed "));
        assert!(lines[5].starts_with("follow f: in progress"));
        assert!(lines[6].starts_with("follow g: INTERRUPTED"));
        assert!(lines[6].ends_with("never finished: owner process 7 is a zombie"));
        assert!(lines[7].starts_with("follow u: pending"));
        assert!(lines[7].contains("owner liveness unknown"));
        assert!(lines[8].starts_with("follow h: CORRUPT record"));
        assert!(lines[9].starts_with("follow *: store unavailable"));
        assert!(lines[10].ends_with("— io: head committed, then: Is a directory"));
        for line in &lines {
            println!("{line}");
        }
    }
}
