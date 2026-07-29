use crc32fast::Hasher as Crc32Hasher;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    image::SparseImage,
    model::{
        parse_hex_bytes, Endian, ImageOverlap, ImgArFileType, StringTrim, Workflow, WorkflowStep,
    },
    nvr::{self, NvrBlock},
    report::{unix_ms_now, ExecutionReport, FileReport, ReportStatus, StepReport},
    validate_workflow, FpwError, Result,
};

#[derive(Debug, Clone)]
enum Artifact {
    Binary(Vec<u8>),
    Image(SparseImage),
    Text(String),
    Nvr(NvrBlock),
}

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
            WorkflowStep::ImageInput(step) => {
                format!("image input {} <- {:?}", step.name, step.path)
            }
            WorkflowStep::ImageOutput(step) => {
                format!("Intel HEX output {} -> {:?}", step.name, step.path)
            }
            WorkflowStep::ImageExtract(step) => format!(
                "extract image {} at 0x{:08X} length 0x{:X} -> {}",
                step.input,
                step.address.parse_u32().unwrap_or_default(),
                step.length.parse_u64().unwrap_or_default(),
                step.output
            ),
            WorkflowStep::ImageOverlay(step) => format!(
                "overlay {} images onto {} -> {}",
                step.overlays.len(),
                step.base,
                step.output
            ),
            WorkflowStep::ImagePatch(step) => format!(
                "patch image {} at 0x{:08X} -> {}",
                step.input,
                step.address.parse_u32().unwrap_or_default(),
                step.output
            ),
            WorkflowStep::ImageToBinary(step) => {
                format!("convert image {} -> binary {}", step.input, step.output)
            }
            WorkflowStep::ImageExtractString(step) => format!(
                "extract string from {} at 0x{:08X} -> {}",
                step.input,
                step.address.parse_u32().unwrap_or_default(),
                step.output
            ),
            WorkflowStep::AssertEqual(step) => {
                format!("assert {} equals {}", step.left, step.right)
            }
            WorkflowStep::ImageInsertBinary(step) => format!(
                "insert binary {} into image {} using {} parts -> {}",
                step.input,
                step.base,
                step.parts.len(),
                step.output
            ),
            WorkflowStep::NvrGenerate(step) => format!(
                "generate NVR page {} banks {}..{} from {} -> {}",
                step.page, step.bank_start, step.bank_end, step.workbook, step.output
            ),
            WorkflowStep::NvrPatchRegisters(step) => format!(
                "apply {} register patches to NVR {} -> {}",
                step.patches.len(),
                step.input,
                step.output
            ),
            WorkflowStep::NvrInjectImage(step) => format!(
                "inject NVR {} into image {} -> {}",
                step.nvr, step.image, step.output
            ),
            WorkflowStep::NvrAppendArchive(step) => format!(
                "append NVR {} to archive {} with imgAr -> {}",
                step.nvr, step.archive, step.output
            ),
            WorkflowStep::ImgArAppend(step) => format!(
                "append {} {} to imgAr archive -> {}",
                img_ar_file_type(&step.file_type),
                step.input,
                step.output
            ),
            WorkflowStep::Fill(step) => format!("fill {} -> {}", step.input, step.output),
            WorkflowStep::Delete(step) => format!(
                "delete range from {} -> {} (preserve length)",
                step.input, step.output
            ),
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
    let mut artifacts: BTreeMap<String, Artifact> = BTreeMap::new();
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
    artifacts: &mut BTreeMap<String, Artifact>,
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
            artifacts.insert(step.name.clone(), Artifact::Binary(bytes));
        }
        WorkflowStep::Output(step) => {
            let bytes = binary_artifact(artifacts, &step.input)?.clone();
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
        WorkflowStep::ImageInput(step) => {
            let resolved = if let Some(path) = options.inputs.get(&step.name) {
                resolve_path(Path::new("."), path)
            } else {
                let path = step.path.clone().ok_or_else(|| {
                    FpwError::Message(format!("image input {} requires a path", step.name))
                })?;
                resolve_path(base_dir, &path)
            };
            let source = fs::read_to_string(&resolved)?;
            let image = SparseImage::from_intel_hex(&source)?;
            file_reports.push(file_report(
                "image-input",
                &step.name,
                &resolved,
                source.as_bytes(),
            ));
            artifacts.insert(step.name.clone(), Artifact::Image(image));
        }
        WorkflowStep::ImageOutput(step) => {
            let source = image_artifact(artifacts, &step.input)?.to_intel_hex(step.record_size)?;
            let resolved = if let Some(path) = options.outputs.get(&step.name) {
                resolve_path(Path::new("."), path)
            } else {
                let path = step.path.clone().ok_or_else(|| {
                    FpwError::Message(format!("image output {} requires a path", step.name))
                })?;
                resolve_path(base_dir, &path)
            };
            if let Some(parent) = resolved.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&resolved, source.as_bytes())?;
            file_reports.push(file_report(
                "image-output",
                &step.name,
                &resolved,
                source.as_bytes(),
            ));
        }
        WorkflowStep::ImageExtract(step) => {
            let image = image_artifact(artifacts, &step.input)?
                .extract(step.address.parse_u32()?, step.length.parse_usize()?)?;
            artifacts.insert(step.output.clone(), Artifact::Image(image));
        }
        WorkflowStep::ImageOverlay(step) => {
            let mut image = image_artifact(artifacts, &step.base)?.clone();
            for overlay in &step.overlays {
                image.overlay(
                    image_artifact(artifacts, overlay)?,
                    matches!(step.overlap, ImageOverlap::Replace),
                )?;
            }
            artifacts.insert(step.output.clone(), Artifact::Image(image));
        }
        WorkflowStep::ImagePatch(step) => {
            let mut image = image_artifact(artifacts, &step.input)?.clone();
            image.insert(
                step.address.parse_u32()?,
                &parse_hex_bytes(&step.data)?,
                true,
            )?;
            artifacts.insert(step.output.clone(), Artifact::Image(image));
        }
        WorkflowStep::ImageToBinary(step) => {
            let fill = step.fill.parse_u64()? as u8;
            let bytes = image_artifact(artifacts, &step.input)?.to_binary(
                step.address.parse_u32()?,
                step.length.parse_usize()?,
                fill,
            )?;
            artifacts.insert(step.output.clone(), Artifact::Binary(bytes));
        }
        WorkflowStep::ImageExtractString(step) => {
            let bytes = image_artifact(artifacts, &step.input)?
                .read_exact(step.address.parse_u32()?, step.length.parse_usize()?)?;
            let text = std::str::from_utf8(&bytes).map_err(|_| {
                FpwError::Message(format!("{} extracted value is not valid UTF-8", step.id))
            })?;
            if !text.is_ascii() {
                return Err(FpwError::Message(format!(
                    "{} extracted value is not ASCII",
                    step.id
                )));
            }
            let text = match step.trim {
                StringTrim::None => text.to_string(),
                StringTrim::NullSpace => text
                    .trim_matches(|character| character == '\0' || character == ' ')
                    .to_string(),
            };
            artifacts.insert(step.output.clone(), Artifact::Text(text));
        }
        WorkflowStep::AssertEqual(step) => {
            let left = text_artifact(artifacts, &step.left)?;
            let right = text_artifact(artifacts, &step.right)?;
            if left != right {
                return Err(FpwError::Message(step.message.clone().unwrap_or_else(
                    || {
                        format!(
                            "{} assertion failed: {} ({left:?}) != {} ({right:?})",
                            step.id, step.left, step.right
                        )
                    },
                )));
            }
        }
        WorkflowStep::ImageInsertBinary(step) => {
            let binary = binary_artifact(artifacts, &step.input)?.clone();
            if let Some(max_length) = &step.max_length {
                let maximum = max_length.parse_usize()?;
                if binary.len() > maximum {
                    return Err(FpwError::Message(format!(
                        "{} input {} is {} bytes, maximum is {} bytes",
                        step.id,
                        step.input,
                        binary.len(),
                        maximum
                    )));
                }
            }
            let mut image = image_artifact(artifacts, &step.base)?.clone();
            for part in &step.parts {
                let source_offset = part.source_offset.parse_usize()?;
                if source_offset > binary.len() {
                    return Err(FpwError::Message(format!(
                        "{} sourceOffset {} exceeds input length {}",
                        step.id,
                        source_offset,
                        binary.len()
                    )));
                }
                let available = binary.len() - source_offset;
                let requested = match &part.length {
                    Some(length) => length.parse_usize()?,
                    None => available,
                };
                let copy_length = available.min(requested);
                if copy_length == 0 {
                    continue;
                }
                image.insert(
                    part.address.parse_u32()?,
                    &binary[source_offset..source_offset + copy_length],
                    true,
                )?;
            }
            artifacts.insert(step.output.clone(), Artifact::Image(image));
        }
        WorkflowStep::NvrGenerate(step) => {
            let path = resolve_path(base_dir, &step.workbook);
            let block = nvr::generate(&path, step)?;
            artifacts.insert(step.output.clone(), Artifact::Nvr(block));
        }
        WorkflowStep::NvrPatchRegisters(step) => {
            let mut block = nvr_artifact(artifacts, &step.input)?.clone();
            for patch in &step.patches {
                block.patch(patch.bank, patch.register, &parse_hex_bytes(&patch.data)?)?;
            }
            artifacts.insert(step.output.clone(), Artifact::Nvr(block));
        }
        WorkflowStep::NvrInjectImage(step) => {
            let block = nvr_artifact(artifacts, &step.nvr)?.clone();
            let mut image = image_artifact(artifacts, &step.image)?.clone();
            image.insert(block.address, &block.data, true)?;
            if let Some(offset) = &step.mirror_offset {
                let mirror_address =
                    block
                        .address
                        .checked_add(offset.parse_u32()?)
                        .ok_or_else(|| {
                            FpwError::Message(format!("{} mirror address overflow", step.id))
                        })?;
                image.insert(mirror_address, &block.data, true)?;
            }
            artifacts.insert(step.output.clone(), Artifact::Image(image));
        }
        WorkflowStep::NvrAppendArchive(step) => {
            let archive = binary_artifact(artifacts, &step.archive)?.clone();
            let block = nvr_artifact(artifacts, &step.nvr)?.clone();
            let temp_dir = base_dir.join(format!(".fpw-nvr-{}-{}", std::process::id(), step.id));
            fs::create_dir_all(&temp_dir)?;
            let archive_path = temp_dir.join("archive.bin");
            let nvr_path = temp_dir.join(block.file_name());
            fs::write(&archive_path, archive)?;
            fs::write(&nvr_path, &block.data)?;
            let tool = fs::canonicalize(resolve_path(base_dir, &step.tool)).map_err(|error| {
                FpwError::Message(format!(
                    "{} cannot resolve imgAr executable {}: {error}",
                    step.id, step.tool
                ))
            })?;
            let (date, time) = current_utc_imgar_timestamp();
            let result = Command::new(&tool)
                .current_dir(&temp_dir)
                .arg("archive.bin")
                .arg(&step.encryption)
                .arg("NVR-REG")
                .arg(date)
                .arg(time)
                .arg(block.file_name())
                .output()
                .map_err(|error| {
                    FpwError::Message(format!(
                        "{} cannot start {}: {error}",
                        step.id,
                        tool.display()
                    ))
                })?;
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            let legacy_success = result.status.code() == Some(1)
                && archive_path.is_file()
                && (stderr.contains("write to ReleaseBin")
                    || stdout.contains("write to ReleaseBin"));
            if !result.status.success() && !legacy_success {
                return Err(FpwError::Message(format!(
                    "{} imgAr failed with {}: {}{}",
                    step.id, result.status, stdout, stderr
                )));
            }
            let bytes = fs::read(&archive_path)?;
            let _ = fs::remove_dir_all(&temp_dir);
            artifacts.insert(step.output.clone(), Artifact::Binary(bytes));
        }
        WorkflowStep::ImgArAppend(step) => {
            let temp_dir = base_dir.join(format!(".fpw-imgar-{}-{}", std::process::id(), step.id));
            if temp_dir.exists() {
                fs::remove_dir_all(&temp_dir)?;
            }
            fs::create_dir_all(&temp_dir)?;
            let archive_path = temp_dir.join("archive.bin");
            if let Some(archive) = &step.archive {
                fs::write(&archive_path, binary_artifact(artifacts, archive)?)?;
            }
            let input_name = match step.file_type {
                ImgArFileType::ImageA | ImgArFileType::ImageB => step
                    .input_file_name
                    .clone()
                    .unwrap_or_else(|| "input.hex".to_string()),
                ImgArFileType::DspA | ImgArFileType::DspB => {
                    step.input_file_name.clone().ok_or_else(|| {
                        FpwError::Message(format!(
                            "{} requires inputFileName such as dsp_vE000F200_ig1_A.bin",
                            step.id
                        ))
                    })?
                }
            };
            if Path::new(&input_name)
                .file_name()
                .and_then(|value| value.to_str())
                != Some(&input_name)
            {
                return Err(FpwError::Message(format!(
                    "{} inputFileName must be a file name without directories",
                    step.id
                )));
            }
            let input_path = temp_dir.join(&input_name);
            match step.file_type {
                ImgArFileType::ImageA | ImgArFileType::ImageB => {
                    fs::write(
                        &input_path,
                        image_artifact(artifacts, &step.input)?.to_intel_hex(16)?,
                    )?;
                }
                ImgArFileType::DspA | ImgArFileType::DspB => {
                    let dsp = binary_artifact(artifacts, &step.input)?;
                    if dsp.len() < 0x7000 {
                        return Err(FpwError::Message(format!(
                            "{} DSP input is {} bytes; legacy imgAr requires at least 0x7000 bytes",
                            step.id,
                            dsp.len()
                        )));
                    }
                    fs::write(&input_path, dsp)?;
                }
            }
            let tool = fs::canonicalize(resolve_path(base_dir, &step.tool)).map_err(|error| {
                FpwError::Message(format!(
                    "{} cannot resolve imgAr executable {}: {error}",
                    step.id, step.tool
                ))
            })?;
            let (date, time) = current_utc_imgar_timestamp();
            let result = Command::new(&tool)
                .current_dir(&temp_dir)
                .arg("archive.bin")
                .arg(&step.encryption)
                .arg(img_ar_file_type(&step.file_type))
                .arg(date)
                .arg(time)
                .arg(&input_name)
                .output()
                .map_err(|error| {
                    FpwError::Message(format!(
                        "{} cannot start {}: {error}",
                        step.id,
                        tool.display()
                    ))
                })?;
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            let legacy_success = result.status.code() == Some(1)
                && archive_path.is_file()
                && (stderr.contains("write to ReleaseBin")
                    || stdout.contains("write to ReleaseBin"));
            if !result.status.success() && !legacy_success {
                return Err(FpwError::Message(format!(
                    "{} imgAr failed with {}: {}{}",
                    step.id, result.status, stdout, stderr
                )));
            }
            let bytes = fs::read(&archive_path)?;
            let _ = fs::remove_dir_all(&temp_dir);
            artifacts.insert(step.output.clone(), Artifact::Binary(bytes));
        }
        WorkflowStep::Fill(step) => {
            let mut bytes = binary_artifact(artifacts, &step.input)?.clone();
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
            artifacts.insert(step.output.clone(), Artifact::Binary(bytes));
        }
        WorkflowStep::Delete(step) => {
            let mut bytes = binary_artifact(artifacts, &step.input)?.clone();
            let offset = step.range.offset.parse_usize()?;
            let length = step.range.length.parse_usize()?;
            let end = offset
                .checked_add(length)
                .ok_or_else(|| FpwError::Message(format!("{} range overflow", step.id)))?;
            let clamped_start = offset.min(bytes.len());
            let clamped_end = end.min(bytes.len());
            bytes[clamped_start..clamped_end].fill(0xFF);
            artifacts.insert(step.output.clone(), Artifact::Binary(bytes));
        }
        WorkflowStep::Insert(step) => {
            let mut base = binary_artifact(artifacts, &step.base)?.clone();
            let insert = binary_artifact(artifacts, &step.insert)?.clone();
            let offset = step.offset.parse_usize()?;
            write_extending(&mut base, offset, &insert);
            artifacts.insert(step.output.clone(), Artifact::Binary(base));
        }
        WorkflowStep::Merge(step) => {
            let mut output = Vec::new();
            let mut occupied = Vec::<(usize, usize, String)>::new();
            for part in &step.parts {
                let bytes = binary_artifact(artifacts, &part.input)?.clone();
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
            artifacts.insert(step.output.clone(), Artifact::Binary(output));
        }
        WorkflowStep::Crc32(step) => {
            let mut bytes = binary_artifact(artifacts, &step.input)?.clone();
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
            artifacts.insert(step.output.clone(), Artifact::Binary(bytes));
        }
        WorkflowStep::Sha256(step) => {
            let bytes = binary_artifact(artifacts, &step.input)?;
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
            artifacts.insert(step.output.clone(), Artifact::Binary(digest));
        }
    }
    Ok(())
}

fn current_utc_imgar_timestamp() -> (String, String) {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let day_seconds = seconds % 86_400;
    // Gregorian conversion from days since 1970-01-01.
    let era_days = days + 719_468;
    let era = if era_days >= 0 {
        era_days
    } else {
        era_days - 146_096
    } / 146_097;
    let day_of_era = era_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (
        format!("{year:04}{month:02}{day:02}"),
        format!(
            "{:02}:{:02}:{:02}",
            day_seconds / 3_600,
            (day_seconds % 3_600) / 60,
            day_seconds % 60
        ),
    )
}

fn img_ar_file_type(file_type: &ImgArFileType) -> &'static str {
    match file_type {
        ImgArFileType::ImageA => "IMG-A",
        ImgArFileType::ImageB => "IMG-B",
        ImgArFileType::DspA => "DSP-N-A",
        ImgArFileType::DspB => "DSP-N-B",
    }
}

fn binary_artifact<'a>(
    artifacts: &'a BTreeMap<String, Artifact>,
    name: &str,
) -> Result<&'a Vec<u8>> {
    match artifacts.get(name) {
        Some(Artifact::Binary(bytes)) => Ok(bytes),
        Some(Artifact::Nvr(block)) => Ok(&block.data),
        Some(Artifact::Image(_)) => Err(FpwError::Message(format!(
            "artifact {name} is an image, expected binary"
        ))),
        Some(Artifact::Text(_)) => Err(FpwError::Message(format!(
            "artifact {name} is text, expected binary"
        ))),
        None => Err(FpwError::Message(format!("missing artifact: {name}"))),
    }
}

fn image_artifact<'a>(
    artifacts: &'a BTreeMap<String, Artifact>,
    name: &str,
) -> Result<&'a SparseImage> {
    match artifacts.get(name) {
        Some(Artifact::Image(image)) => Ok(image),
        Some(Artifact::Binary(_)) => Err(FpwError::Message(format!(
            "artifact {name} is binary, expected image"
        ))),
        Some(Artifact::Text(_)) => Err(FpwError::Message(format!(
            "artifact {name} is text, expected image"
        ))),
        Some(Artifact::Nvr(_)) => Err(FpwError::Message(format!(
            "artifact {name} is NVR data, expected image"
        ))),
        None => Err(FpwError::Message(format!("missing artifact: {name}"))),
    }
}

fn text_artifact<'a>(artifacts: &'a BTreeMap<String, Artifact>, name: &str) -> Result<&'a String> {
    match artifacts.get(name) {
        Some(Artifact::Text(text)) => Ok(text),
        Some(Artifact::Binary(_)) => Err(FpwError::Message(format!(
            "artifact {name} is binary, expected text"
        ))),
        Some(Artifact::Image(_)) => Err(FpwError::Message(format!(
            "artifact {name} is an image, expected text"
        ))),
        Some(Artifact::Nvr(_)) => Err(FpwError::Message(format!(
            "artifact {name} is NVR data, expected text"
        ))),
        None => Err(FpwError::Message(format!("missing artifact: {name}"))),
    }
}

fn nvr_artifact<'a>(artifacts: &'a BTreeMap<String, Artifact>, name: &str) -> Result<&'a NvrBlock> {
    match artifacts.get(name) {
        Some(Artifact::Nvr(block)) => Ok(block),
        Some(_) => Err(FpwError::Message(format!(
            "artifact {name} is not NVR data"
        ))),
        None => Err(FpwError::Message(format!("missing artifact: {name}"))),
    }
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
        WorkflowStep::ImageInput(_) => "image-input",
        WorkflowStep::ImageOutput(_) => "image-output",
        WorkflowStep::ImageExtract(_) => "image-extract",
        WorkflowStep::ImageOverlay(_) => "image-overlay",
        WorkflowStep::ImagePatch(_) => "image-patch",
        WorkflowStep::ImageToBinary(_) => "image-to-binary",
        WorkflowStep::ImageExtractString(_) => "image-extract-string",
        WorkflowStep::AssertEqual(_) => "assert-equal",
        WorkflowStep::ImageInsertBinary(_) => "image-insert-binary",
        WorkflowStep::NvrGenerate(_) => "nvr-generate",
        WorkflowStep::NvrPatchRegisters(_) => "nvr-patch-registers",
        WorkflowStep::NvrInjectImage(_) => "nvr-inject-image",
        WorkflowStep::NvrAppendArchive(_) => "nvr-append-archive",
        WorkflowStep::ImgArAppend(_) => "imgar-append",
        WorkflowStep::Fill(_) => "fill",
        WorkflowStep::Delete(_) => "delete",
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
        ByteRange, Crc32Step, DeleteStep, FillStep, InputStep, InsertStep, MergePart, MergeStep,
        NumberValue, OutputStep, Sha256Step,
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
                WorkflowStep::Delete(DeleteStep {
                    id: "delete".to_string(),
                    input: "filled".to_string(),
                    output: "deleted".to_string(),
                    range: ByteRange {
                        offset: number(3),
                        length: number(3),
                    },
                }),
                WorkflowStep::Insert(InsertStep {
                    id: "insert".to_string(),
                    base: "deleted".to_string(),
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
        assert_eq!(&image[..8], &[0, 0, 0x11, 0xFF, 0xFF, 0xFF, 0xAA, 0xBB]);
        assert_eq!(image.len(), 12);
        assert_eq!(
            fs::read(root.join("out/image.sha256.bin")).unwrap().len(),
            32
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_preserves_length_and_ignores_range_past_end() {
        let root = test_root("delete-range");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("input.bin"), [0x10, 0x20, 0x30, 0x40]).unwrap();

        let workflow = Workflow {
            schema_version: 1,
            name: "delete-range".to_string(),
            description: None,
            steps: vec![
                WorkflowStep::Input(InputStep {
                    id: "input".to_string(),
                    name: "firmware".to_string(),
                    path: Some("input.bin".to_string()),
                }),
                WorkflowStep::Delete(DeleteStep {
                    id: "delete".to_string(),
                    input: "firmware".to_string(),
                    output: "deleted".to_string(),
                    range: ByteRange {
                        offset: number(2),
                        length: number(8),
                    },
                }),
                WorkflowStep::Output(OutputStep {
                    id: "output".to_string(),
                    input: "deleted".to_string(),
                    name: "image".to_string(),
                    path: Some("out.bin".to_string()),
                }),
            ],
        };
        let workflow_path = write_workflow(&root, &workflow);

        let report = run_workflow(&workflow_path, &workflow, &RunOptions::default()).unwrap();

        assert_eq!(report.status, ReportStatus::Success);
        assert_eq!(
            fs::read(root.join("out.bin")).unwrap(),
            [0x10, 0x20, 0xFF, 0xFF]
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

    #[test]
    fn postbuild_mcu_workflow_merges_reference_hex_images() {
        let root = test_root("postbuild-mcu");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let workflow_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/postbuild-mcu-merge.fwp");
        let workflow = Workflow::from_path(&workflow_path).unwrap();
        let hex_output = root.join("postbuild-mcu.hex");
        let bin_output = root.join("postbuild-mcu.bin");
        let mut options = RunOptions::default();
        let postbuild_inputs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Postbuild/Input");
        options.inputs.insert(
            "gboot".to_string(),
            postbuild_inputs
                .join("GungnirS_gboot.hex")
                .to_string_lossy()
                .to_string(),
        );
        options.inputs.insert(
            "image_a".to_string(),
            postbuild_inputs
                .join("GungnirS_imageA.hex")
                .to_string_lossy()
                .to_string(),
        );
        options.inputs.insert(
            "image_b".to_string(),
            postbuild_inputs
                .join("GungnirS_imageB.hex")
                .to_string_lossy()
                .to_string(),
        );
        options.outputs.insert(
            "jlink_hex".to_string(),
            hex_output.to_string_lossy().to_string(),
        );
        options.outputs.insert(
            "jlink_bin".to_string(),
            bin_output.to_string_lossy().to_string(),
        );

        let report = run_workflow(&workflow_path, &workflow, &options).unwrap();

        assert_eq!(report.status, ReportStatus::Success);
        let output_image =
            SparseImage::from_intel_hex(&fs::read_to_string(&hex_output).unwrap()).unwrap();
        assert_eq!(output_image.start_address(), Some(0x0800_1187));
        let binary = fs::read(&bin_output).unwrap();
        assert_eq!(binary.len(), 0x20_0000);
        assert_eq!(&binary[0x10250..0x10258], b"1.00.04\0");
        assert_eq!(&binary[0x110250..0x110258], b"1.00.04\0");
        assert_eq!(&binary[0x10C000..0x10C002], &[0, 0]);

        let dsp_path = root.join("dsp_vE000F200.bin");
        let dsp: Vec<u8> = (0..0x80020).map(|index| (index % 251) as u8).collect();
        fs::write(&dsp_path, &dsp).unwrap();
        let dsp_workflow_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/postbuild-dsp-inject.fwp");
        let dsp_workflow = Workflow::from_path(&dsp_workflow_path).unwrap();
        let dsp_hex_output = root.join("postbuild-mcu-dsp.hex");
        let dsp_bin_output = root.join("postbuild-mcu-dsp.bin");
        let mut dsp_options = RunOptions::default();
        dsp_options.inputs.insert(
            "mcu_hex".to_string(),
            hex_output.to_string_lossy().to_string(),
        );
        dsp_options
            .inputs
            .insert("dsp".to_string(), dsp_path.to_string_lossy().to_string());
        dsp_options.outputs.insert(
            "jlink_dsp_hex".to_string(),
            dsp_hex_output.to_string_lossy().to_string(),
        );
        dsp_options.outputs.insert(
            "jlink_dsp_bin".to_string(),
            dsp_bin_output.to_string_lossy().to_string(),
        );
        let dsp_report = run_workflow(&dsp_workflow_path, &dsp_workflow, &dsp_options).unwrap();
        assert_eq!(dsp_report.status, ReportStatus::Success);
        let dsp_binary = fs::read(dsp_bin_output).unwrap();
        assert_eq!(&dsp_binary[0x80000..0x100000], &dsp[..0x80000]);
        assert_eq!(&dsp_binary[0x180000..0x180020], &dsp[0x80000..]);

        fs::write(&dsp_path, vec![0; 0x93001]).unwrap();
        let oversized_report =
            run_workflow(&dsp_workflow_path, &dsp_workflow, &dsp_options).unwrap();
        assert_eq!(oversized_report.status, ReportStatus::Failed);
        assert!(oversized_report
            .steps
            .last()
            .and_then(|step| step.message.as_deref())
            .unwrap_or("")
            .contains("maximum"));

        let image_b_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Postbuild/Input/GungnirS_imageB.hex");
        let mut mismatched_image_b =
            SparseImage::from_intel_hex(&fs::read_to_string(image_b_path).unwrap()).unwrap();
        mismatched_image_b
            .insert(0x0811_0250, b"9.99.99", true)
            .unwrap();
        let mismatched_path = root.join("image-b-mismatch.hex");
        fs::write(
            &mismatched_path,
            mismatched_image_b.to_intel_hex(16).unwrap(),
        )
        .unwrap();
        let mut mismatch_options = options.clone();
        mismatch_options.inputs.insert(
            "image_b".to_string(),
            mismatched_path.to_string_lossy().to_string(),
        );
        let mismatch_report = run_workflow(&workflow_path, &workflow, &mismatch_options).unwrap();
        assert_eq!(mismatch_report.status, ReportStatus::Failed);
        assert_eq!(
            mismatch_report
                .steps
                .last()
                .and_then(|step| step.message.as_deref()),
            Some("Image A and Image B firmware versions differ")
        );

        fs::remove_dir_all(root).unwrap();
    }
}
