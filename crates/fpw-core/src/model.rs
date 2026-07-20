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
