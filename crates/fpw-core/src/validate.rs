use std::collections::BTreeSet;

use crate::{
    model::{Workflow, WorkflowStep},
    FpwError, Result,
};

pub fn validate_workflow(workflow: &Workflow) -> Result<()> {
    if workflow.schema_version != 1 {
        return Err(FpwError::Message(format!(
            "unsupported schemaVersion {}, expected 1",
            workflow.schema_version
        )));
    }
    if workflow.name.trim().is_empty() {
        return Err(FpwError::Message("workflow name is required".to_string()));
    }
    if workflow.steps.is_empty() {
        return Err(FpwError::Message(
            "workflow must contain at least one step".to_string(),
        ));
    }

    let mut step_ids = BTreeSet::new();
    let mut artifacts = BTreeSet::new();

    for step in &workflow.steps {
        if !step_ids.insert(step.id().to_string()) {
            return Err(FpwError::Message(format!(
                "duplicate step id: {}",
                step.id()
            )));
        }

        match step {
            WorkflowStep::Input(input) => {
                if input.name.trim().is_empty() {
                    return Err(FpwError::Message(format!("{} requires name", input.id)));
                }
                artifacts.insert(input.name.clone());
            }
            WorkflowStep::Output(output) => {
                require_artifact(&artifacts, &output.input, &output.id)?;
                if output.name.trim().is_empty() {
                    return Err(FpwError::Message(format!("{} requires name", output.id)));
                }
            }
            WorkflowStep::Fill(fill) => {
                require_artifact(&artifacts, &fill.input, &fill.id)?;
                validate_byte(fill.value.parse_u64()?, &fill.id, "value")?;
                artifacts.insert(fill.output.clone());
            }
            WorkflowStep::Insert(insert) => {
                require_artifact(&artifacts, &insert.base, &insert.id)?;
                require_artifact(&artifacts, &insert.insert, &insert.id)?;
                artifacts.insert(insert.output.clone());
            }
            WorkflowStep::Merge(merge) => {
                if merge.parts.is_empty() {
                    return Err(FpwError::Message(format!("{} requires parts", merge.id)));
                }
                for part in &merge.parts {
                    require_artifact(&artifacts, &part.input, &merge.id)?;
                }
                artifacts.insert(merge.output.clone());
            }
            WorkflowStep::Crc32(crc) => {
                require_artifact(&artifacts, &crc.input, &crc.id)?;
                artifacts.insert(crc.output.clone());
            }
            WorkflowStep::Sha256(sha) => {
                require_artifact(&artifacts, &sha.input, &sha.id)?;
                artifacts.insert(sha.output.clone());
            }
        }
    }

    Ok(())
}

fn require_artifact(artifacts: &BTreeSet<String>, name: &str, step_id: &str) -> Result<()> {
    if artifacts.contains(name) {
        Ok(())
    } else {
        Err(FpwError::Message(format!(
            "{step_id} references unknown artifact: {name}"
        )))
    }
}

fn validate_byte(value: u64, step_id: &str, field: &str) -> Result<()> {
    if value <= u8::MAX as u64 {
        Ok(())
    } else {
        Err(FpwError::Message(format!(
            "{step_id} field {field} must be a byte, got {value}"
        )))
    }
}
