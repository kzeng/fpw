use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

use crate::{FpwError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
}

impl Workflow {
    pub fn from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let workflow: Self = serde_json::from_str(&text)?;
        Ok(workflow)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WorkflowStep {
    Input(InputStep),
    Output(OutputStep),
    ImageInput(ImageInputStep),
    ImageOutput(ImageOutputStep),
    ImageExtract(ImageExtractStep),
    ImageOverlay(ImageOverlayStep),
    ImagePatch(ImagePatchStep),
    ImageToBinary(ImageToBinaryStep),
    ImageExtractString(ImageExtractStringStep),
    AssertEqual(AssertEqualStep),
    ImageInsertBinary(ImageInsertBinaryStep),
    NvrGenerate(NvrGenerateStep),
    NvrPatchRegisters(NvrPatchRegistersStep),
    NvrInjectImage(NvrInjectImageStep),
    NvrAppendArchive(NvrAppendArchiveStep),
    #[serde(rename = "imgar-append")]
    ImgArAppend(ImgArAppendStep),
    Fill(FillStep),
    Delete(DeleteStep),
    Insert(InsertStep),
    Merge(MergeStep),
    Crc32(Crc32Step),
    Sha256(Sha256Step),
}

impl WorkflowStep {
    pub fn id(&self) -> &str {
        match self {
            Self::Input(step) => &step.id,
            Self::Output(step) => &step.id,
            Self::ImageInput(step) => &step.id,
            Self::ImageOutput(step) => &step.id,
            Self::ImageExtract(step) => &step.id,
            Self::ImageOverlay(step) => &step.id,
            Self::ImagePatch(step) => &step.id,
            Self::ImageToBinary(step) => &step.id,
            Self::ImageExtractString(step) => &step.id,
            Self::AssertEqual(step) => &step.id,
            Self::ImageInsertBinary(step) => &step.id,
            Self::NvrGenerate(step) => &step.id,
            Self::NvrPatchRegisters(step) => &step.id,
            Self::NvrInjectImage(step) => &step.id,
            Self::NvrAppendArchive(step) => &step.id,
            Self::ImgArAppend(step) => &step.id,
            Self::Fill(step) => &step.id,
            Self::Delete(step) => &step.id,
            Self::Insert(step) => &step.id,
            Self::Merge(step) => &step.id,
            Self::Crc32(step) => &step.id,
            Self::Sha256(step) => &step.id,
        }
    }

    pub fn output_artifact(&self) -> Option<&str> {
        match self {
            Self::Input(step) => Some(&step.name),
            Self::Output(_) => None,
            Self::ImageInput(step) => Some(&step.name),
            Self::ImageOutput(_) => None,
            Self::ImageExtract(step) => Some(&step.output),
            Self::ImageOverlay(step) => Some(&step.output),
            Self::ImagePatch(step) => Some(&step.output),
            Self::ImageToBinary(step) => Some(&step.output),
            Self::ImageExtractString(step) => Some(&step.output),
            Self::AssertEqual(_) => None,
            Self::ImageInsertBinary(step) => Some(&step.output),
            Self::NvrGenerate(step) => Some(&step.output),
            Self::NvrPatchRegisters(step) => Some(&step.output),
            Self::NvrInjectImage(step) => Some(&step.output),
            Self::NvrAppendArchive(step) => Some(&step.output),
            Self::ImgArAppend(step) => Some(&step.output),
            Self::Fill(step) => Some(&step.output),
            Self::Delete(step) => Some(&step.output),
            Self::Insert(step) => Some(&step.output),
            Self::Merge(step) => Some(&step.output),
            Self::Crc32(step) => Some(&step.output),
            Self::Sha256(step) => Some(&step.output),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputStep {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputStep {
    pub id: String,
    pub input: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillStep {
    pub id: String,
    pub input: String,
    pub output: String,
    pub offset: NumberValue,
    pub length: NumberValue,
    #[serde(default = "default_fill_value")]
    pub value: NumberValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertStep {
    pub id: String,
    pub base: String,
    pub insert: String,
    pub output: String,
    pub offset: NumberValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeStep {
    pub id: String,
    pub output: String,
    pub parts: Vec<MergePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePart {
    pub input: String,
    pub offset: NumberValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crc32Step {
    pub id: String,
    pub input: String,
    pub output: String,
    pub range: ByteRange,
    #[serde(rename = "writeOffset")]
    pub write_offset: NumberValue,
    #[serde(default)]
    pub endian: Endian,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sha256Step {
    pub id: String,
    pub input: String,
    pub output: String,
    #[serde(default)]
    pub range: Option<ByteRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByteRange {
    pub offset: NumberValue,
    pub length: NumberValue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Endian {
    #[default]
    Little,
    Big,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInputStep {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageOutputStep {
    pub id: String,
    pub input: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_record_size")]
    pub record_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageExtractStep {
    pub id: String,
    pub input: String,
    pub output: String,
    pub address: NumberValue,
    pub length: NumberValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageOverlayStep {
    pub id: String,
    pub base: String,
    pub overlays: Vec<String>,
    pub output: String,
    #[serde(default)]
    pub overlap: ImageOverlap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePatchStep {
    pub id: String,
    pub input: String,
    pub output: String,
    pub address: NumberValue,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageToBinaryStep {
    pub id: String,
    pub input: String,
    pub output: String,
    pub address: NumberValue,
    pub length: NumberValue,
    #[serde(default = "default_fill_value")]
    pub fill: NumberValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageExtractStringStep {
    pub id: String,
    pub input: String,
    pub output: String,
    pub address: NumberValue,
    pub length: NumberValue,
    #[serde(default)]
    pub trim: StringTrim,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StringTrim {
    None,
    #[default]
    NullSpace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertEqualStep {
    pub id: String,
    pub left: String,
    pub right: String,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInsertBinaryStep {
    pub id: String,
    pub base: String,
    pub input: String,
    pub output: String,
    #[serde(rename = "maxLength")]
    #[serde(default)]
    pub max_length: Option<NumberValue>,
    pub parts: Vec<BinaryImagePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryImagePart {
    #[serde(rename = "sourceOffset")]
    pub source_offset: NumberValue,
    pub address: NumberValue,
    #[serde(default)]
    pub length: Option<NumberValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvrGenerateStep {
    pub id: String,
    pub output: String,
    pub workbook: String,
    pub page: u8,
    pub bank_start: u8,
    pub bank_end: u8,
    #[serde(default = "default_nvr_register_start")]
    pub register_start: u8,
    #[serde(default = "default_nvr_register_end")]
    pub register_end: u8,
    pub base_address: NumberValue,
    pub sheets: Vec<NvrSheet>,
    #[serde(default = "default_nvr_version_sheet")]
    pub version_sheet: String,
    #[serde(default = "default_nvr_version_cell")]
    pub version_cell: String,
    #[serde(default)]
    pub ignore_mask_rule: bool,
    #[serde(default)]
    pub alternate_base: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvrSheet {
    pub name: String,
    pub bank: u8,
    pub row_start: usize,
    pub row_end: usize,
    pub data_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvrPatchRegistersStep {
    pub id: String,
    pub input: String,
    pub output: String,
    pub patches: Vec<NvrRegisterPatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvrRegisterPatch {
    pub bank: u8,
    pub register: u8,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvrInjectImageStep {
    pub id: String,
    pub image: String,
    pub nvr: String,
    pub output: String,
    #[serde(default)]
    pub mirror_offset: Option<NumberValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvrAppendArchiveStep {
    pub id: String,
    pub archive: String,
    pub nvr: String,
    pub output: String,
    pub tool: String,
    #[serde(default = "default_imgar_encryption")]
    pub encryption: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImgArAppendStep {
    pub id: String,
    #[serde(default)]
    pub archive: Option<String>,
    pub input: String,
    pub output: String,
    pub tool: String,
    pub file_type: ImgArFileType,
    #[serde(default = "default_imgar_encryption")]
    pub encryption: String,
    #[serde(default)]
    pub input_file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImgArFileType {
    #[serde(rename = "IMG-A")]
    ImageA,
    #[serde(rename = "IMG-B")]
    ImageB,
    #[serde(rename = "DSP-N-A")]
    DspA,
    #[serde(rename = "DSP-N-B")]
    DspB,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageOverlap {
    #[default]
    Error,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteStep {
    pub id: String,
    pub input: String,
    pub output: String,
    pub range: ByteRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberValue {
    Number(u64),
    String(String),
}

impl NumberValue {
    pub fn parse_u64(&self) -> Result<u64> {
        match self {
            Self::Number(value) => Ok(*value),
            Self::String(value) => parse_number(value),
        }
    }

    pub fn parse_usize(&self) -> Result<usize> {
        let value = self.parse_u64()?;
        usize::try_from(value).map_err(|_| {
            FpwError::Message(format!("number is too large for this platform: {value}"))
        })
    }

    pub fn parse_u32(&self) -> Result<u32> {
        let value = self.parse_u64()?;
        u32::try_from(value)
            .map_err(|_| FpwError::Message(format!("number exceeds 32-bit address space: {value}")))
    }
}

impl Default for NumberValue {
    fn default() -> Self {
        Self::Number(0)
    }
}

pub fn parse_number(value: &str) -> Result<u64> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
            .map_err(|_| FpwError::Message(format!("invalid hex number: {value}")))
    } else {
        trimmed
            .parse::<u64>()
            .map_err(|_| FpwError::Message(format!("invalid number: {value}")))
    }
}

pub fn parse_named_values(values: &[String]) -> Result<BTreeMap<String, String>> {
    let mut parsed = BTreeMap::new();
    for value in values {
        let Some((name, path)) = value.split_once('=') else {
            return Err(FpwError::Message(format!(
                "expected name=path mapping, got: {value}"
            )));
        };
        let name = name.trim();
        let path = path.trim();
        if name.is_empty() || path.is_empty() {
            return Err(FpwError::Message(format!(
                "expected non-empty name=path mapping, got: {value}"
            )));
        }
        parsed.insert(name.to_string(), path.to_string());
    }
    Ok(parsed)
}

fn default_fill_value() -> NumberValue {
    NumberValue::String("0xFF".to_string())
}

fn default_record_size() -> usize {
    16
}

fn default_nvr_register_start() -> u8 {
    128
}
fn default_nvr_register_end() -> u8 {
    255
}
fn default_nvr_version_sheet() -> String {
    "Cover".to_string()
}
fn default_nvr_version_cell() -> String {
    "E3".to_string()
}
fn default_imgar_encryption() -> String {
    "enc0".to_string()
}

pub fn parse_hex_bytes(value: &str) -> Result<Vec<u8>> {
    let compact: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != '_')
        .collect();
    let hex = compact
        .strip_prefix("0x")
        .or_else(|| compact.strip_prefix("0X"))
        .unwrap_or(&compact);
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return Err(FpwError::Message(format!(
            "hex data must contain complete byte pairs: {value}"
        )));
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| FpwError::Message(format!("invalid hex data: {value}")))?;
            u8::from_str_radix(text, 16)
                .map_err(|_| FpwError::Message(format!("invalid hex data: {value}")))
        })
        .collect()
}
