use std::collections::{BTreeMap, BTreeSet};

use crate::{
    model::{parse_hex_bytes, ImgArFileType, Workflow, WorkflowStep},
    FpwError, Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtifactKind {
    Binary,
    Image,
    Text,
    Nvr,
}

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
    let mut artifacts = BTreeMap::new();

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
                artifacts.insert(input.name.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::Output(output) => {
                require_binary_like(&artifacts, &output.input, &output.id)?;
                if output.name.trim().is_empty() {
                    return Err(FpwError::Message(format!("{} requires name", output.id)));
                }
            }
            WorkflowStep::ImageInput(input) => {
                if input.name.trim().is_empty() {
                    return Err(FpwError::Message(format!("{} requires name", input.id)));
                }
                artifacts.insert(input.name.clone(), ArtifactKind::Image);
            }
            WorkflowStep::ImageOutput(output) => {
                require_artifact(&artifacts, &output.input, &output.id, ArtifactKind::Image)?;
                if output.name.trim().is_empty() {
                    return Err(FpwError::Message(format!("{} requires name", output.id)));
                }
                if !(1..=255).contains(&output.record_size) {
                    return Err(FpwError::Message(format!(
                        "{} recordSize must be between 1 and 255",
                        output.id
                    )));
                }
            }
            WorkflowStep::ImageExtract(extract) => {
                require_artifact(&artifacts, &extract.input, &extract.id, ArtifactKind::Image)?;
                extract.address.parse_u32()?;
                extract.length.parse_usize()?;
                artifacts.insert(extract.output.clone(), ArtifactKind::Image);
            }
            WorkflowStep::ImageOverlay(overlay) => {
                require_artifact(&artifacts, &overlay.base, &overlay.id, ArtifactKind::Image)?;
                if overlay.overlays.is_empty() {
                    return Err(FpwError::Message(format!(
                        "{} requires overlays",
                        overlay.id
                    )));
                }
                for input in &overlay.overlays {
                    require_artifact(&artifacts, input, &overlay.id, ArtifactKind::Image)?;
                }
                artifacts.insert(overlay.output.clone(), ArtifactKind::Image);
            }
            WorkflowStep::ImagePatch(patch) => {
                require_artifact(&artifacts, &patch.input, &patch.id, ArtifactKind::Image)?;
                patch.address.parse_u32()?;
                parse_hex_bytes(&patch.data)?;
                artifacts.insert(patch.output.clone(), ArtifactKind::Image);
            }
            WorkflowStep::ImageToBinary(convert) => {
                require_artifact(&artifacts, &convert.input, &convert.id, ArtifactKind::Image)?;
                convert.address.parse_u32()?;
                convert.length.parse_usize()?;
                validate_byte(convert.fill.parse_u64()?, &convert.id, "fill")?;
                artifacts.insert(convert.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::ImageExtractString(extract) => {
                require_artifact(&artifacts, &extract.input, &extract.id, ArtifactKind::Image)?;
                extract.address.parse_u32()?;
                extract.length.parse_usize()?;
                artifacts.insert(extract.output.clone(), ArtifactKind::Text);
            }
            WorkflowStep::AssertEqual(assertion) => {
                require_artifact(
                    &artifacts,
                    &assertion.left,
                    &assertion.id,
                    ArtifactKind::Text,
                )?;
                require_artifact(
                    &artifacts,
                    &assertion.right,
                    &assertion.id,
                    ArtifactKind::Text,
                )?;
            }
            WorkflowStep::ImageInsertBinary(insert) => {
                require_artifact(&artifacts, &insert.base, &insert.id, ArtifactKind::Image)?;
                require_artifact(&artifacts, &insert.input, &insert.id, ArtifactKind::Binary)?;
                if insert.parts.is_empty() {
                    return Err(FpwError::Message(format!("{} requires parts", insert.id)));
                }
                if let Some(max_length) = &insert.max_length {
                    max_length.parse_usize()?;
                }
                for part in &insert.parts {
                    part.source_offset.parse_usize()?;
                    part.address.parse_u32()?;
                    if let Some(length) = &part.length {
                        length.parse_usize()?;
                    }
                }
                artifacts.insert(insert.output.clone(), ArtifactKind::Image);
            }
            WorkflowStep::NvrGenerate(generate) => {
                if generate.workbook.trim().is_empty() {
                    return Err(FpwError::Message(format!(
                        "{} requires workbook",
                        generate.id
                    )));
                }
                if generate.bank_start > generate.bank_end {
                    return Err(FpwError::Message(format!(
                        "{} bankStart exceeds bankEnd",
                        generate.id
                    )));
                }
                if generate.register_start < 128 || generate.register_end < 128 {
                    return Err(FpwError::Message(format!(
                        "{} registers must be in range 128..255",
                        generate.id
                    )));
                }
                if generate.sheets.is_empty() {
                    return Err(FpwError::Message(format!(
                        "{} requires sheets",
                        generate.id
                    )));
                }
                generate.base_address.parse_u32()?;
                artifacts.insert(generate.output.clone(), ArtifactKind::Nvr);
            }
            WorkflowStep::NvrPatchRegisters(patch) => {
                require_artifact(&artifacts, &patch.input, &patch.id, ArtifactKind::Nvr)?;
                if patch.patches.is_empty() {
                    return Err(FpwError::Message(format!("{} requires patches", patch.id)));
                }
                for item in &patch.patches {
                    if item.register < 128 {
                        return Err(FpwError::Message(format!(
                            "{} register must be in range 128..255",
                            patch.id
                        )));
                    }
                    parse_hex_bytes(&item.data)?;
                }
                artifacts.insert(patch.output.clone(), ArtifactKind::Nvr);
            }
            WorkflowStep::NvrInjectImage(inject) => {
                require_artifact(&artifacts, &inject.image, &inject.id, ArtifactKind::Image)?;
                require_artifact(&artifacts, &inject.nvr, &inject.id, ArtifactKind::Nvr)?;
                if let Some(offset) = &inject.mirror_offset {
                    offset.parse_u32()?;
                }
                artifacts.insert(inject.output.clone(), ArtifactKind::Image);
            }
            WorkflowStep::NvrAppendArchive(append) => {
                require_artifact(
                    &artifacts,
                    &append.archive,
                    &append.id,
                    ArtifactKind::Binary,
                )?;
                require_artifact(&artifacts, &append.nvr, &append.id, ArtifactKind::Nvr)?;
                if append.tool.trim().is_empty() {
                    return Err(FpwError::Message(format!("{} requires tool", append.id)));
                }
                if append.encryption != "enc0" && append.encryption != "enc1" {
                    return Err(FpwError::Message(format!(
                        "{} encryption must be enc0 or enc1",
                        append.id
                    )));
                }
                artifacts.insert(append.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::ImgArAppend(append) => {
                if let Some(archive) = &append.archive {
                    require_artifact(&artifacts, archive, &append.id, ArtifactKind::Binary)?;
                }
                let input_kind = match append.file_type {
                    ImgArFileType::ImageA | ImgArFileType::ImageB => ArtifactKind::Image,
                    ImgArFileType::DspA | ImgArFileType::DspB => ArtifactKind::Binary,
                };
                require_artifact(&artifacts, &append.input, &append.id, input_kind)?;
                if append.tool.trim().is_empty() {
                    return Err(FpwError::Message(format!("{} requires tool", append.id)));
                }
                if append.encryption != "enc0" && append.encryption != "enc1" {
                    return Err(FpwError::Message(format!(
                        "{} encryption must be enc0 or enc1",
                        append.id
                    )));
                }
                if matches!(append.file_type, ImgArFileType::DspA | ImgArFileType::DspB) {
                    let name = append.input_file_name.as_deref().unwrap_or("");
                    let suffix = if matches!(append.file_type, ImgArFileType::DspA) {
                        "_A.bin"
                    } else {
                        "_B.bin"
                    };
                    let format_ok = name.len() == 23
                        && name.starts_with("dsp_v")
                        && name.ends_with(suffix)
                        && &name[13..16] == "_ig"
                        && matches!(name.as_bytes()[16], b'0' | b'1')
                        && name[5..13].bytes().all(|byte| byte.is_ascii_hexdigit());
                    if !format_ok {
                        return Err(FpwError::Message(format!(
                            "{} inputFileName must match dsp_vXXXXXXXX_igN{}",
                            append.id, suffix
                        )));
                    }
                }
                artifacts.insert(append.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::Fill(fill) => {
                require_artifact(&artifacts, &fill.input, &fill.id, ArtifactKind::Binary)?;
                validate_byte(fill.value.parse_u64()?, &fill.id, "value")?;
                artifacts.insert(fill.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::Delete(delete) => {
                require_artifact(&artifacts, &delete.input, &delete.id, ArtifactKind::Binary)?;
                delete.range.offset.parse_usize()?;
                delete.range.length.parse_usize()?;
                artifacts.insert(delete.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::Insert(insert) => {
                require_artifact(&artifacts, &insert.base, &insert.id, ArtifactKind::Binary)?;
                require_artifact(&artifacts, &insert.insert, &insert.id, ArtifactKind::Binary)?;
                artifacts.insert(insert.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::Merge(merge) => {
                if merge.parts.is_empty() {
                    return Err(FpwError::Message(format!("{} requires parts", merge.id)));
                }
                for part in &merge.parts {
                    require_artifact(&artifacts, &part.input, &merge.id, ArtifactKind::Binary)?;
                }
                artifacts.insert(merge.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::Crc32(crc) => {
                require_artifact(&artifacts, &crc.input, &crc.id, ArtifactKind::Binary)?;
                artifacts.insert(crc.output.clone(), ArtifactKind::Binary);
            }
            WorkflowStep::Sha256(sha) => {
                require_artifact(&artifacts, &sha.input, &sha.id, ArtifactKind::Binary)?;
                artifacts.insert(sha.output.clone(), ArtifactKind::Binary);
            }
        }
    }

    Ok(())
}

fn require_artifact(
    artifacts: &BTreeMap<String, ArtifactKind>,
    name: &str,
    step_id: &str,
    expected: ArtifactKind,
) -> Result<()> {
    match artifacts.get(name) {
        Some(actual) if *actual == expected => Ok(()),
        Some(actual) => Err(FpwError::Message(format!(
            "{step_id} requires a {} artifact for {name}, found {}",
            artifact_kind_name(expected),
            artifact_kind_name(*actual)
        ))),
        None => Err(FpwError::Message(format!(
            "{step_id} references unknown artifact: {name}"
        ))),
    }
}

fn require_binary_like(
    artifacts: &BTreeMap<String, ArtifactKind>,
    name: &str,
    step_id: &str,
) -> Result<()> {
    match artifacts.get(name) {
        Some(ArtifactKind::Binary | ArtifactKind::Nvr) => Ok(()),
        Some(actual) => Err(FpwError::Message(format!(
            "{step_id} requires binary or NVR data for {name}, found {}",
            artifact_kind_name(*actual)
        ))),
        None => Err(FpwError::Message(format!(
            "{step_id} references unknown artifact: {name}"
        ))),
    }
}

fn artifact_kind_name(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Binary => "binary",
        ArtifactKind::Image => "image",
        ArtifactKind::Text => "text",
        ArtifactKind::Nvr => "NVR data",
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
