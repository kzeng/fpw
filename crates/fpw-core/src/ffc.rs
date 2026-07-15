use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

use crate::{
    model::{
        ByteRange, Crc32Step, FillStep, InputStep, InsertStep, MergePart, MergeStep, NumberValue,
        OutputStep, Sha256Step, WorkflowStep,
    },
    FpwError, Result, Workflow,
};

#[derive(Debug, Clone)]
pub struct ImportWarning {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub workflow: Workflow,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Deserialize)]
struct FfcWorkflow {
    #[serde(default)]
    settings: FfcSettings,
    #[serde(default)]
    nodes: Vec<FfcNode>,
    #[serde(default)]
    edges: Vec<FfcEdge>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FfcSettings {
    #[serde(default)]
    project_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfcNode {
    id: String,
    kind: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    params: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FfcEdge {
    source: String,
    target: String,
    #[serde(default)]
    target_handle: Option<String>,
    #[serde(default)]
    order: Option<String>,
}

pub fn import_ffc(path: &Path) -> Result<ImportResult> {
    let text = fs::read_to_string(path)?;
    let ffc: FfcWorkflow = serde_json::from_str(&text)?;
    let name = ffc
        .settings
        .project_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "imported-ffc".to_string());

    let node_by_id = ffc
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let ordered_nodes = order_nodes(&ffc, &node_by_id)?;
    let artifact_by_node_id = ffc
        .nodes
        .iter()
        .filter_map(|node| output_artifact_name(node).map(|artifact| (node.id.clone(), artifact)))
        .collect::<BTreeMap<_, _>>();
    let mut warnings = Vec::new();
    let mut steps = Vec::new();

    for node in ordered_nodes {
        match convert_node(node, &ffc.edges, &artifact_by_node_id, &mut warnings)? {
            Some(mut converted) => steps.append(&mut converted),
            None => warnings.push(ImportWarning {
                message: format!(
                    "Skipped unsupported FirmwareFlow step '{}' ({})",
                    node.label_or_id(),
                    node.kind
                ),
            }),
        }
    }

    Ok(ImportResult {
        workflow: Workflow {
            schema_version: 1,
            name,
            description: Some(format!("Imported from {}", path.display())),
            steps,
        },
        warnings,
    })
}

fn order_nodes<'a>(
    workflow: &'a FfcWorkflow,
    node_by_id: &BTreeMap<&str, &'a FfcNode>,
) -> Result<Vec<&'a FfcNode>> {
    let mut incoming = BTreeMap::<&str, usize>::new();
    let mut outgoing = BTreeMap::<&str, Vec<&str>>::new();
    for node in &workflow.nodes {
        incoming.insert(node.id.as_str(), 0);
    }
    for edge in &workflow.edges {
        if node_by_id.contains_key(edge.source.as_str())
            && node_by_id.contains_key(edge.target.as_str())
        {
            outgoing
                .entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
            *incoming.entry(edge.target.as_str()).or_default() += 1;
        }
    }

    let mut ready = workflow
        .nodes
        .iter()
        .filter(|node| incoming.get(node.id.as_str()).copied().unwrap_or(0) == 0)
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>();
    let mut ordered = Vec::new();

    while let Some(node_id) = ready.first().copied() {
        ready.remove(0);
        let node = node_by_id
            .get(node_id)
            .ok_or_else(|| FpwError::Message(format!("missing node {node_id}")))?;
        ordered.push(*node);
        if let Some(targets) = outgoing.get(node_id) {
            for target in targets {
                if let Some(count) = incoming.get_mut(target) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.push(target);
                    }
                }
            }
        }
    }

    if ordered.len() == workflow.nodes.len() {
        Ok(ordered)
    } else {
        Err(FpwError::Message(
            "FirmwareFlow workflow contains a cycle or invalid edge state".to_string(),
        ))
    }
}

fn convert_node(
    node: &FfcNode,
    edges: &[FfcEdge],
    artifact_by_node_id: &BTreeMap<String, String>,
    warnings: &mut Vec<ImportWarning>,
) -> Result<Option<Vec<WorkflowStep>>> {
    let steps = match node.kind.as_str() {
        "input" => vec![WorkflowStep::Input(InputStep {
            id: safe_id(&node.id),
            name: artifact_name(node, "output"),
            path: node.params.get("example").cloned(),
        })],
        "output" => vec![WorkflowStep::Output(OutputStep {
            id: safe_id(&node.id),
            input: param_or_connected(node, "input", edges, artifact_by_node_id)
                .unwrap_or_default(),
            name: output_name(node),
            path: node
                .params
                .get("example")
                .cloned()
                .or_else(|| node.params.get("output").cloned()),
        })],
        "fill" => {
            let (offset, length) = parse_ffc_range(required_param(node, "range")?)?;
            vec![WorkflowStep::Fill(FillStep {
                id: safe_id(&node.id),
                input: param_or_connected(node, "input", edges, artifact_by_node_id)
                    .unwrap_or_default(),
                output: artifact_name(node, "output"),
                offset,
                length,
                value: number_param(node, "value", "0xFF"),
            })]
        }
        "crc" => {
            let (offset, length) = parse_ffc_range(required_param(node, "range")?)?;
            vec![WorkflowStep::Crc32(Crc32Step {
                id: safe_id(&node.id),
                input: param_or_connected(node, "input", edges, artifact_by_node_id)
                    .unwrap_or_default(),
                output: artifact_name(node, "output"),
                range: ByteRange { offset, length },
                write_offset: number_param(node, "address", "0"),
                endian: Default::default(),
            })]
        }
        "sha256-bin" => {
            let (offset, length) = parse_ffc_range(required_param(node, "range")?)?;
            vec![WorkflowStep::Sha256(Sha256Step {
                id: safe_id(&node.id),
                input: param_or_connected(node, "input", edges, artifact_by_node_id)
                    .unwrap_or_default(),
                output: artifact_name(node, "output"),
                range: Some(ByteRange { offset, length }),
            })]
        }
        "sha256" => {
            warnings.push(ImportWarning {
                message: format!(
                    "{}: FirmwareFlow sha256 writes digest into an image; FPW imports it as sha256 + insert.",
                    node.label_or_id()
                ),
            });
            let (offset, length) = parse_ffc_range(required_param(node, "range")?)?;
            let input =
                param_or_connected(node, "input", edges, artifact_by_node_id).unwrap_or_default();
            let digest_output = format!("{}_digest", artifact_name(node, "output"));
            vec![
                WorkflowStep::Sha256(Sha256Step {
                    id: format!("{}_sha256", safe_id(&node.id)),
                    input: input.clone(),
                    output: digest_output.clone(),
                    range: Some(ByteRange { offset, length }),
                }),
                WorkflowStep::Insert(InsertStep {
                    id: format!("{}_insert", safe_id(&node.id)),
                    base: input,
                    insert: digest_output,
                    output: artifact_name(node, "output"),
                    offset: number_param(node, "address", "0"),
                }),
            ]
        }
        "insert" => vec![WorkflowStep::Insert(InsertStep {
            id: safe_id(&node.id),
            base: param_or_connected(node, "base", edges, artifact_by_node_id).unwrap_or_default(),
            insert: param_or_connected(node, "insert", edges, artifact_by_node_id)
                .unwrap_or_else(|| node.params.get("insert").cloned().unwrap_or_default()),
            output: artifact_name(node, "output"),
            offset: number_param(node, "address", "0"),
        })],
        "merge" => {
            warnings.push(ImportWarning {
                message: format!(
                    "{}: FirmwareFlow merge ranges are imported as FPW part offsets; range slicing is not preserved.",
                    node.label_or_id()
                ),
            });
            vec![WorkflowStep::Merge(MergeStep {
                id: safe_id(&node.id),
                output: artifact_name(node, "output"),
                parts: merge_parts(node, edges, artifact_by_node_id)?,
            })]
        }
        _ => return Ok(None),
    };

    Ok(Some(steps))
}

fn merge_parts(
    node: &FfcNode,
    edges: &[FfcEdge],
    artifact_by_node_id: &BTreeMap<String, String>,
) -> Result<Vec<MergePart>> {
    let file1 = param_or_connected(node, "file1", edges, artifact_by_node_id).unwrap_or_default();
    let file2 = param_or_connected(node, "file2", edges, artifact_by_node_id)
        .unwrap_or_else(|| node.params.get("file2").cloned().unwrap_or_default());
    let (offset1, _) = parse_ffc_range(required_param(node, "range1")?)?;
    let (offset2, _) = parse_ffc_range(required_param(node, "range2")?)?;
    let mut parts = vec![
        MergePart {
            input: file1,
            offset: offset1,
        },
        MergePart {
            input: file2,
            offset: offset2,
        },
    ];
    if matches!(
        node.params.get("order").map(String::as_str),
        Some("file2,file1" | "file2-first" | "2,1")
    ) {
        parts.reverse();
    }
    Ok(parts)
}

fn param_or_connected(
    node: &FfcNode,
    key: &str,
    edges: &[FfcEdge],
    artifact_by_node_id: &BTreeMap<String, String>,
) -> Option<String> {
    node.params
        .get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| connected_input_name(node, key, edges, artifact_by_node_id))
}

fn connected_input_name(
    node: &FfcNode,
    key: &str,
    edges: &[FfcEdge],
    artifact_by_node_id: &BTreeMap<String, String>,
) -> Option<String> {
    let mut candidates = edges
        .iter()
        .filter(|edge| edge.target == node.id)
        .collect::<Vec<_>>();
    candidates.sort_by_key(|edge| {
        edge.order
            .as_deref()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    });
    candidates
        .iter()
        .copied()
        .find(|edge| edge.target_handle.as_deref() == Some(key))
        .or_else(|| candidates.first().copied())
        .and_then(|edge| artifact_by_node_id.get(&edge.source).cloned())
}

fn output_artifact_name(node: &FfcNode) -> Option<String> {
    match node.kind.as_str() {
        "output" => None,
        "input" | "fill" | "crc" | "sha256" | "sha256-bin" | "insert" | "merge" => {
            Some(artifact_name(node, "output"))
        }
        _ => None,
    }
}

fn artifact_name(node: &FfcNode, key: &str) -> String {
    node.params
        .get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| safe_id(&node.id))
}

fn output_name(node: &FfcNode) -> String {
    node.params
        .get("output")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| {
            node.params
                .get("order")
                .map(|order| format!("output_{order}"))
        })
        .unwrap_or_else(|| safe_id(&node.id))
}

fn required_param<'a>(node: &'a FfcNode, key: &str) -> Result<&'a str> {
    node.params
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| FpwError::Message(format!("{} requires {key}", node.label_or_id())))
}

fn number_param(node: &FfcNode, key: &str, default: &str) -> NumberValue {
    node.params
        .get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .map(NumberValue::String)
        .unwrap_or_else(|| NumberValue::String(default.to_string()))
}

fn parse_ffc_range(range: &str) -> Result<(NumberValue, NumberValue)> {
    let Some((start, end)) = range.split_once(':') else {
        return Err(FpwError::Message(format!(
            "expected FirmwareFlow range start:end, got {range}"
        )));
    };
    let start_value = crate::model::parse_number(start)?;
    let end_value = crate::model::parse_number(end)?;
    if end_value < start_value {
        return Err(FpwError::Message(format!("invalid range {range}")));
    }
    Ok((
        NumberValue::Number(start_value),
        NumberValue::Number(end_value - start_value + 1),
    ))
}

fn safe_id(value: &str) -> String {
    let mut result = String::new();
    let mut last_was_separator = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            result.push(ch);
            last_was_separator = false;
        } else if !last_was_separator && !result.is_empty() {
            result.push('_');
            last_was_separator = true;
        }
    }
    let result = result.trim_matches('_').to_string();
    if result.is_empty() {
        "step".to_string()
    } else {
        result
    }
}

trait FfcNodeLabel {
    fn label_or_id(&self) -> &str;
}

impl FfcNodeLabel for FfcNode {
    fn label_or_id(&self) -> &str {
        if self.label.trim().is_empty() {
            &self.id
        } else {
            &self.label
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WorkflowStep;

    fn test_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fpw-ffc-{name}-{}", std::process::id()))
    }

    #[test]
    fn imports_supported_steps_and_warns_for_unsupported_steps() {
        let root = test_root("basic");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let ffc_path = root.join("workflow.ffc");
        fs::write(
            &ffc_path,
            r#"{
              "version": 1,
              "settings": { "projectName": "imported" },
              "nodes": [
                {
                  "id": "input-1",
                  "kind": "input",
                  "label": "Input1",
                  "params": { "output": "Input1", "example": "examples/app.bin" }
                },
                {
                  "id": "fill-1",
                  "kind": "fill",
                  "label": "Fill",
                  "params": {
                    "range": "0x10:0x1F",
                    "value": "0xAA",
                    "output": "FILLED"
                  }
                },
                {
                  "id": "sha-1",
                  "kind": "sha256-bin",
                  "label": "SHA",
                  "params": {
                    "range": "0x0:0x1F",
                    "output": "DIGEST"
                  }
                },
                {
                  "id": "rsa-1",
                  "kind": "rsa-sign-pss",
                  "label": "RSA",
                  "params": {}
                }
              ],
              "edges": [
                { "source": "input-1", "target": "fill-1", "targetHandle": "input" },
                { "source": "fill-1", "target": "sha-1", "targetHandle": "input" }
              ]
            }"#,
        )
        .unwrap();

        let imported = import_ffc(&ffc_path).unwrap();

        assert_eq!(imported.workflow.name, "imported");
        assert_eq!(imported.workflow.steps.len(), 3);
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.message.contains("rsa-sign-pss")));

        match &imported.workflow.steps[1] {
            WorkflowStep::Fill(step) => {
                assert_eq!(step.input, "Input1");
                assert_eq!(step.output, "FILLED");
                assert_eq!(step.offset.parse_u64().unwrap(), 0x10);
                assert_eq!(step.length.parse_u64().unwrap(), 16);
            }
            other => panic!("expected fill step, got {other:?}"),
        }

        match &imported.workflow.steps[2] {
            WorkflowStep::Sha256(step) => {
                assert_eq!(step.input, "FILLED");
                assert_eq!(step.output, "DIGEST");
            }
            other => panic!("expected sha256 step, got {other:?}"),
        }

        fs::remove_dir_all(root).unwrap();
    }
}
