use crc32fast::Hasher as Crc32Hasher;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    model::{Endian, Workflow, WorkflowStep},
    report::{unix_ms_now, ExecutionReport, FileReport, ReportStatus, StepReport},
    validate_workflow, FpwError, Result,
};

#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub inputs: BTreeMap<String, String>,
    pub outputs: BTreeMap<String, String>,
    pub report_dir: Option<PathBuf>,
    pub command: Vec<String>,
}

pub fn preview_workflow(workflow: &Workflow) -> Result<Vec<String>> {
    validate_workflow(workflow)?;
    Ok(workflow
        .steps
        .iter()
        .map(|step| match step {
            WorkflowStep::Input(step) => format!("input {} <- {:?}", step.name, step.path),
            WorkflowStep::Output(step) => format!("output {} -> {:?}", step.name, step.path),
            WorkflowStep::Fill(step) => format!("fill {} -> {}", step.input, step.output),
            WorkflowStep::Insert(step) => format!(
                "insert {} into {} -> {}",
                step.insert, step.base, step.output
            ),
            WorkflowStep::Merge(step) => {
                format!("merge {} parts -> {}", step.parts.len(), step.output)
            }
            WorkflowStep::Crc32(step) => format!("crc32 {} -> {}", step.input, step.output),
            WorkflowStep::Sha256(step) => format!("sha256 {} -> {}", step.input, step.output),
        })
        .collect())
}

pub fn run_workflow(
    workflow_path: &Path,
    workflow: &Workflow,
    options: &RunOptions,
) -> Result<ExecutionReport> {
    validate_workflow(workflow)?;

    let workflow_bytes = fs::read(workflow_path)?;
    run_validated_workflow(workflow_path, &workflow_bytes, workflow, options)
}

pub fn run_workflow_source(
    workflow_path: &Path,
    workflow_source: &[u8],
    workflow: &Workflow,
    options: &RunOptions,
) -> Result<ExecutionReport> {
    validate_workflow(workflow)?;
    run_validated_workflow(workflow_path, workflow_source, workflow, options)
}

fn run_validated_workflow(
    workflow_path: &Path,
    workflow_bytes: &[u8],
    workflow: &Workflow,
    options: &RunOptions,
) -> Result<ExecutionReport> {
    let base_dir = workflow_path.parent().unwrap_or_else(|| Path::new("."));
    let started_at = unix_ms_now();
    let mut artifacts: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut step_reports = Vec::new();
    let mut file_reports = Vec::new();
    let workflow_sha256 = sha256_hex(workflow_bytes);
    let mut status = ReportStatus::Success;

    for step in &workflow.steps {
        let step_started = unix_ms_now();
        let result = execute_step(step, base_dir, &mut artifacts, options, &mut file_reports);
        let step_ended = unix_ms_now();
        match result {
            Ok(()) => step_reports.push(StepReport {
                id: step.id().to_string(),
                kind: step_kind(step).to_string(),
                status: ReportStatus::Success,
                duration_ms: step_ended.saturating_sub(step_started),
                message: None,
            }),
            Err(error) => {
                status = ReportStatus::Failed;
                step_reports.push(StepReport {
                    id: step.id().to_string(),
                    kind: step_kind(step).to_string(),
                    status: ReportStatus::Failed,
                    duration_ms: step_ended.saturating_sub(step_started),
                    message: Some(error.to_string()),
                });
                break;
            }
        }
    }

    let ended_at = unix_ms_now();
    Ok(ExecutionReport {
        fpw_version: env!("CARGO_PKG_VERSION").to_string(),
        workflow_path: workflow_path.to_string_lossy().to_string(),
        workflow_sha256,
        command: options.command.clone(),
        started_at_unix_ms: started_at,
        ended_at_unix_ms: ended_at,
        duration_ms: ended_at.saturating_sub(started_at),
        status,
        steps: step_reports,
        files: file_reports,
    })
}

fn execute_step(
    step: &WorkflowStep,
    base_dir: &Path,
    artifacts: &mut BTreeMap<String, Vec<u8>>,
    options: &RunOptions,
    file_reports: &mut Vec<FileReport>,
) -> Result<()> {
    match step {
        WorkflowStep::Input(step) => {
            let resolved = if let Some(path) = options.inputs.get(&step.name) {
                resolve_path(Path::new("."), path)
            } else {
                let path = step.path.clone().ok_or_else(|| {
                    FpwError::Message(format!("input {} requires a path", step.name))
                })?;
                resolve_path(base_dir, &path)
            };
            let bytes = fs::read(&resolved)?;
            file_reports.push(file_report("input", &step.name, &resolved, &bytes));
            artifacts.insert(step.name.clone(), bytes);
        }
        WorkflowStep::Output(step) => {
            let bytes = artifact(artifacts, &step.input)?.clone();
            let resolved = if let Some(path) = options.outputs.get(&step.name) {
                resolve_path(Path::new("."), path)
            } else {
                let path = step.path.clone().ok_or_else(|| {
                    FpwError::Message(format!("output {} requires a path", step.name))
                })?;
                resolve_path(base_dir, &path)
            };
            if let Some(parent) = resolved.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&resolved, &bytes)?;
            file_reports.push(file_report("output", &step.name, &resolved, &bytes));
        }
        WorkflowStep::Fill(step) => {
            let mut bytes = artifact(artifacts, &step.input)?.clone();
            let offset = step.offset.parse_usize()?;
            let length = step.length.parse_usize()?;
            let value = step.value.parse_u64()?;
            if value > u8::MAX as u64 {
                return Err(FpwError::Message(format!(
                    "{} value must fit in one byte",
                    step.id
                )));
            }
            write_extending(&mut bytes, offset, &vec![value as u8; length]);
            artifacts.insert(step.output.clone(), bytes);
        }
        WorkflowStep::Insert(step) => {
            let mut base = artifact(artifacts, &step.base)?.clone();
            let insert = artifact(artifacts, &step.insert)?.clone();
            let offset = step.offset.parse_usize()?;
            write_extending(&mut base, offset, &insert);
            artifacts.insert(step.output.clone(), base);
        }
        WorkflowStep::Merge(step) => {
            let mut output = Vec::new();
            let mut occupied = Vec::<(usize, usize, String)>::new();
            for part in &step.parts {
                let bytes = artifact(artifacts, &part.input)?.clone();
                let offset = part.offset.parse_usize()?;
                let end = offset.checked_add(bytes.len()).ok_or_else(|| {
                    FpwError::Message(format!("{} merge range overflow", step.id))
                })?;
                for (existing_start, existing_end, existing_name) in &occupied {
                    if offset < *existing_end && end > *existing_start {
                        return Err(FpwError::Message(format!(
                            "{} overlaps {} at range [{offset}, {end})",
                            part.input, existing_name
                        )));
                    }
                }
                write_extending(&mut output, offset, &bytes);
                occupied.push((offset, end, part.input.clone()));
            }
            artifacts.insert(step.output.clone(), output);
        }
        WorkflowStep::Crc32(step) => {
            let mut bytes = artifact(artifacts, &step.input)?.clone();
            let range = read_range(
                &bytes,
                step.range.offset.parse_usize()?,
                step.range.length.parse_usize()?,
                &step.id,
            )?;
            let mut hasher = Crc32Hasher::new();
            hasher.update(range);
            let crc = hasher.finalize();
            let crc_bytes = match step.endian {
                Endian::Little => crc.to_le_bytes(),
                Endian::Big => crc.to_be_bytes(),
            };
            write_extending(&mut bytes, step.write_offset.parse_usize()?, &crc_bytes);
            artifacts.insert(step.output.clone(), bytes);
        }
        WorkflowStep::Sha256(step) => {
            let bytes = artifact(artifacts, &step.input)?;
            let source = if let Some(range) = &step.range {
                read_range(
                    bytes,
                    range.offset.parse_usize()?,
                    range.length.parse_usize()?,
                    &step.id,
                )?
            } else {
                bytes.as_slice()
            };
            let digest = Sha256::digest(source).to_vec();
            artifacts.insert(step.output.clone(), digest);
        }
    }
    Ok(())
}

fn artifact<'a>(artifacts: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a Vec<u8>> {
    artifacts
        .get(name)
        .ok_or_else(|| FpwError::Message(format!("missing artifact: {name}")))
}

fn read_range<'a>(
    bytes: &'a [u8],
    offset: usize,
    length: usize,
    step_id: &str,
) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| FpwError::Message(format!("{step_id} range overflow")))?;
    bytes.get(offset..end).ok_or_else(|| {
        FpwError::Message(format!(
            "{step_id} range [{offset}, {end}) is outside input"
        ))
    })
}

fn write_extending(target: &mut Vec<u8>, offset: usize, data: &[u8]) {
    if target.len() < offset {
        target.resize(offset, 0xFF);
    }
    let end = offset + data.len();
    if target.len() < end {
        target.resize(end, 0xFF);
    }
    target[offset..end].copy_from_slice(data);
}

fn resolve_path(base_dir: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn file_report(role: &str, name: &str, path: &Path, bytes: &[u8]) -> FileReport {
    FileReport {
        role: role.to_string(),
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        size_bytes: bytes.len() as u64,
        sha256: sha256_hex(bytes),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn step_kind(step: &WorkflowStep) -> &'static str {
    match step {
        WorkflowStep::Input(_) => "input",
        WorkflowStep::Output(_) => "output",
        WorkflowStep::Fill(_) => "fill",
        WorkflowStep::Insert(_) => "insert",
        WorkflowStep::Merge(_) => "merge",
        WorkflowStep::Crc32(_) => "crc32",
        WorkflowStep::Sha256(_) => "sha256",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ByteRange, Crc32Step, FillStep, InputStep, InsertStep, MergePart, MergeStep, NumberValue,
        OutputStep, Sha256Step,
    };

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fpw-core-{name}-{}", std::process::id()))
    }

    fn write_workflow(root: &Path, workflow: &Workflow) -> PathBuf {
        let path = root.join("workflow.fwp");
        fs::write(&path, serde_json::to_string_pretty(workflow).unwrap()).unwrap();
        path
    }

    fn number(value: u64) -> NumberValue {
        NumberValue::Number(value)
    }

    #[test]
    fn fill_insert_crc32_sha256_execute_end_to_end() {
        let root = test_root("end-to-end");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("input.bin"), vec![0u8; 8]).unwrap();
        fs::write(root.join("patch.bin"), vec![0xAA, 0xBB]).unwrap();

        let workflow = Workflow {
            schema_version: 1,
            name: "end-to-end".to_string(),
            description: None,
            steps: vec![
                WorkflowStep::Input(InputStep {
                    id: "firmware".to_string(),
                    name: "firmware".to_string(),
                    path: Some("input.bin".to_string()),
                }),
                WorkflowStep::Input(InputStep {
                    id: "patch".to_string(),
                    name: "patch".to_string(),
                    path: Some("patch.bin".to_string()),
                }),
                WorkflowStep::Fill(FillStep {
                    id: "fill".to_string(),
                    input: "firmware".to_string(),
                    output: "filled".to_string(),
                    offset: number(2),
                    length: number(3),
                    value: number(0x11),
                }),
                WorkflowStep::Insert(InsertStep {
                    id: "insert".to_string(),
                    base: "filled".to_string(),
                    insert: "patch".to_string(),
                    output: "patched".to_string(),
                    offset: number(6),
                }),
                WorkflowStep::Crc32(Crc32Step {
                    id: "crc".to_string(),
                    input: "patched".to_string(),
                    output: "with_crc".to_string(),
                    range: ByteRange {
                        offset: number(0),
                        length: number(8),
                    },
                    write_offset: number(8),
                    endian: Endian::Little,
                }),
                WorkflowStep::Sha256(Sha256Step {
                    id: "sha".to_string(),
                    input: "with_crc".to_string(),
                    output: "digest".to_string(),
                    range: None,
                }),
                WorkflowStep::Output(OutputStep {
                    id: "out_image".to_string(),
                    input: "with_crc".to_string(),
                    name: "image".to_string(),
                    path: Some("out/image.bin".to_string()),
                }),
                WorkflowStep::Output(OutputStep {
                    id: "out_digest".to_string(),
                    input: "digest".to_string(),
                    name: "digest".to_string(),
                    path: Some("out/image.sha256.bin".to_string()),
                }),
            ],
        };
        let workflow_path = write_workflow(&root, &workflow);

        let report = run_workflow(&workflow_path, &workflow, &RunOptions::default()).unwrap();

        assert_eq!(report.status, ReportStatus::Success);
        let image = fs::read(root.join("out/image.bin")).unwrap();
        assert_eq!(&image[..8], &[0, 0, 0x11, 0x11, 0x11, 0, 0xAA, 0xBB]);
        assert_eq!(image.len(), 12);
        assert_eq!(
            fs::read(root.join("out/image.sha256.bin")).unwrap().len(),
            32
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn merge_rejects_overlapping_parts() {
        let root = test_root("merge-overlap");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.bin"), vec![1, 2, 3, 4]).unwrap();
        fs::write(root.join("b.bin"), vec![5, 6, 7, 8]).unwrap();

        let workflow = Workflow {
            schema_version: 1,
            name: "merge-overlap".to_string(),
            description: None,
            steps: vec![
                WorkflowStep::Input(InputStep {
                    id: "a".to_string(),
                    name: "a".to_string(),
                    path: Some("a.bin".to_string()),
                }),
                WorkflowStep::Input(InputStep {
                    id: "b".to_string(),
                    name: "b".to_string(),
                    path: Some("b.bin".to_string()),
                }),
                WorkflowStep::Merge(MergeStep {
                    id: "merge".to_string(),
                    output: "merged".to_string(),
                    parts: vec![
                        MergePart {
                            input: "a".to_string(),
                            offset: number(0),
                        },
                        MergePart {
                            input: "b".to_string(),
                            offset: number(2),
                        },
                    ],
                }),
            ],
        };
        let workflow_path = write_workflow(&root, &workflow);

        let report = run_workflow(&workflow_path, &workflow, &RunOptions::default()).unwrap();

        assert_eq!(report.status, ReportStatus::Failed);
        assert!(report
            .steps
            .last()
            .and_then(|step| step.message.as_deref())
            .unwrap_or("")
            .contains("overlaps"));

        fs::remove_dir_all(root).unwrap();
    }
}
