use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::Result;

#[derive(Debug, Clone, Copy)]
pub enum ReportFormat {
    Json,
    Txt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReport {
    pub fpw_version: String,
    pub workflow_path: String,
    pub workflow_sha256: String,
    pub command: Vec<String>,
    pub started_at_unix_ms: u128,
    pub ended_at_unix_ms: u128,
    pub duration_ms: u128,
    pub status: ReportStatus,
    pub steps: Vec<StepReport>,
    pub files: Vec<FileReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportStatus {
    Success,
    Failed,
}

impl ReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReport {
    pub id: String,
    pub kind: String,
    pub status: ReportStatus,
    pub duration_ms: u128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReport {
    pub role: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

impl ExecutionReport {
    pub fn write_all(&self, report_dir: &Path, stem: &str) -> Result<Vec<PathBuf>> {
        fs::create_dir_all(report_dir)?;
        let json_path = report_dir.join(format!("{stem}.json"));
        let txt_path = report_dir.join(format!("{stem}.txt"));
        fs::write(&json_path, serde_json::to_string_pretty(self)?)?;
        fs::write(&txt_path, self.to_text())?;
        Ok(vec![json_path, txt_path])
    }

    pub fn to_text(&self) -> String {
        let mut text = String::new();
        text.push_str("FPW Execution Report\n");
        text.push_str("====================\n\n");
        text.push_str(&format!("Status: {}\n", self.status.as_str()));
        text.push_str(&format!("FPW version: {}\n", self.fpw_version));
        text.push_str(&format!("Workflow: {}\n", self.workflow_path));
        text.push_str(&format!("Workflow SHA256: {}\n", self.workflow_sha256));
        text.push_str(&format!("Duration: {} ms\n", self.duration_ms));
        text.push_str(&format!("Command: {}\n\n", self.command.join(" ")));
        text.push_str("Steps:\n");
        for step in &self.steps {
            text.push_str(&format!(
                "- {} [{}] {} ({} ms)",
                step.id,
                step.kind,
                step.status.as_str(),
                step.duration_ms
            ));
            if let Some(message) = &step.message {
                text.push_str(&format!(": {message}"));
            }
            text.push('\n');
        }
        text.push_str("\nFiles:\n");
        for file in &self.files {
            text.push_str(&format!(
                "- {} {}: {} ({} bytes, sha256 {})\n",
                file.role, file.name, file.path, file.size_bytes, file.sha256
            ));
        }
        text
    }
}

pub fn unix_ms_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
