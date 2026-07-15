pub mod execute;
pub mod ffc;
pub mod model;
pub mod recent;
pub mod report;
pub mod validate;

pub use execute::{preview_workflow, run_workflow, RunOptions};
pub use model::{Workflow, WorkflowStep};
pub use report::{ExecutionReport, ReportFormat};
pub use validate::validate_workflow;

#[derive(Debug, thiserror::Error)]
pub enum FpwError {
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, FpwError>;
