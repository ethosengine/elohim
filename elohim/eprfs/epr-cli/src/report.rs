use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingStatus {
    Pass,
    Info,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub code: String,
    pub status: FindingStatus,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Whether this finding prevents a positive reach-readiness result. Local
    /// `check` remains advisory regardless; only `ready` consumes this bit.
    pub blocks_reach: bool,
}

impl Finding {
    pub fn new(code: impl Into<String>, status: FindingStatus, summary: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            status,
            summary: summary.into(),
            detail: None,
            remediation: None,
            path: None,
            blocks_reach: false,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_string_lossy().replace('\\', "/"));
        self
    }

    pub fn blocking(mut self) -> Self {
        self.blocks_reach = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub command: String,
    pub scope: String,
    pub repository: String,
    pub local_agency: String,
    pub ready: bool,
    pub findings: Vec<Finding>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
}

impl Report {
    pub fn new(command: impl Into<String>, repository: &Path) -> Self {
        Self {
            command: command.into(),
            scope: "governance-floor".into(),
            repository: repository.to_string_lossy().replace('\\', "/"),
            local_agency:
                "working-tree authoring is local; readiness is evaluated only for requested reach"
                    .into(),
            ready: true,
            findings: Vec::new(),
            data: Value::Null,
        }
    }

    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
        self.recompute();
    }

    pub fn extend(&mut self, findings: impl IntoIterator<Item = Finding>) {
        self.findings.extend(findings);
        self.recompute();
    }

    pub fn remove_code(&mut self, code: &str) {
        self.findings.retain(|finding| finding.code != code);
        self.recompute();
    }

    pub fn recompute(&mut self) {
        self.ready = !self
            .findings
            .iter()
            .any(|finding| finding.blocks_reach || finding.status == FindingStatus::Fail);
    }
}
