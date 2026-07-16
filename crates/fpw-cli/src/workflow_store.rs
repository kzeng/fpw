use fpw_core::{validate_workflow, FpwError, Result, Workflow};
use serde::Serialize;
use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    pub path: String,
    pub name: String,
    pub description: Option<String>,
    pub step_count: usize,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone)]
pub struct WorkflowStore {
    root: PathBuf,
}

impl Default for WorkflowStore {
    fn default() -> Self {
        let root = env::var_os("FPW_WORKFLOW_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("workflows"));
        Self::new(root)
    }
}

impl WorkflowStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list(&self) -> Result<Vec<WorkflowSummary>> {
        fs::create_dir_all(&self.root)?;
        let mut workflows = Vec::new();
        self.collect_workflows(&self.root, &mut workflows)?;
        workflows.sort_by(|left, right| {
            right
                .updated_at_unix_ms
                .cmp(&left.updated_at_unix_ms)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(workflows)
    }

    pub fn open(&self, relative_path: &str) -> Result<Workflow> {
        Workflow::from_path(&self.resolve(relative_path)?)
    }

    pub fn create(&self, relative_path: &str, workflow: &Workflow) -> Result<WorkflowSummary> {
        validate_workflow(workflow)?;
        let path = self.resolve(relative_path)?;
        if path.exists() {
            return Err(FpwError::Message(format!(
                "workflow already exists: {relative_path}"
            )));
        }
        self.write(&path, workflow)?;
        self.summary(&path, workflow)
    }

    pub fn save(&self, relative_path: &str, workflow: &Workflow) -> Result<WorkflowSummary> {
        validate_workflow(workflow)?;
        let path = self.resolve(relative_path)?;
        if !path.is_file() {
            return Err(FpwError::Message(format!(
                "workflow does not exist: {relative_path}"
            )));
        }
        self.write(&path, workflow)?;
        self.summary(&path, workflow)
    }

    pub fn duplicate(&self, source_path: &str, target_path: &str) -> Result<WorkflowSummary> {
        let workflow = self.open(source_path)?;
        self.create(target_path, &workflow)
    }

    pub fn archive(&self, relative_path: &str, timestamp_ms: u128) -> Result<String> {
        let source = self.resolve(relative_path)?;
        if !source.is_file() {
            return Err(FpwError::Message(format!(
                "workflow does not exist: {relative_path}"
            )));
        }
        let trash = self.root.join(".trash");
        fs::create_dir_all(&trash)?;
        let stem = source
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("workflow");
        let archived_name = format!("{stem}-{timestamp_ms}.fwp");
        let archived = trash.join(&archived_name);
        fs::rename(source, &archived)?;
        Ok(format!(".trash/{archived_name}"))
    }

    pub fn import_fwp(&self, source_path: &Path, target_path: &str) -> Result<WorkflowSummary> {
        let workflow = Workflow::from_path(source_path)?;
        self.create(target_path, &workflow)
    }

    pub fn import_ffc(
        &self,
        source_path: &Path,
        target_path: &str,
    ) -> Result<(WorkflowSummary, Vec<String>)> {
        let imported = fpw_core::ffc::import_ffc(source_path)?;
        let warnings = imported
            .warnings
            .into_iter()
            .map(|warning| warning.message)
            .collect();
        let summary = self.create(target_path, &imported.workflow)?;
        Ok((summary, warnings))
    }

    fn collect_workflows(&self, directory: &Path, output: &mut Vec<WorkflowSummary>) -> Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some(".trash") {
                    self.collect_workflows(&path, output)?;
                }
            } else if path.extension().and_then(|value| value.to_str()) == Some("fwp") {
                if let Ok(workflow) = Workflow::from_path(&path) {
                    output.push(self.summary(&path, &workflow)?);
                }
            }
        }
        Ok(())
    }

    fn resolve(&self, relative_path: &str) -> Result<PathBuf> {
        let relative = Path::new(relative_path.trim());
        if relative.as_os_str().is_empty()
            || relative.is_absolute()
            || relative.extension().and_then(|value| value.to_str()) != Some("fwp")
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(FpwError::Message(
                "workflow path must be a relative .fwp path without '..'".to_string(),
            ));
        }
        Ok(self.root.join(relative))
    }

    fn write(&self, path: &Path, workflow: &Workflow) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(workflow)?)?;
        Ok(())
    }

    fn summary(&self, path: &Path, workflow: &Workflow) -> Result<WorkflowSummary> {
        let updated_at_unix_ms = fs::metadata(path)?
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let relative = path
            .strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        Ok(WorkflowSummary {
            path: relative,
            name: workflow.name.clone(),
            description: workflow.description.clone(),
            step_count: workflow.steps.len(),
            updated_at_unix_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fpw_core::model::{InputStep, OutputStep, WorkflowStep};

    fn workflow(name: &str) -> Workflow {
        Workflow {
            schema_version: 1,
            name: name.to_string(),
            description: Some("managed workflow".to_string()),
            steps: vec![
                WorkflowStep::Input(InputStep {
                    id: "input".to_string(),
                    name: "firmware".to_string(),
                    path: Some("input.bin".to_string()),
                }),
                WorkflowStep::Output(OutputStep {
                    id: "output".to_string(),
                    input: "firmware".to_string(),
                    name: "image".to_string(),
                    path: Some("out.bin".to_string()),
                }),
            ],
        }
    }

    #[test]
    fn create_open_duplicate_and_archive_workflow() {
        let root = std::env::temp_dir().join(format!("fpw-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let store = WorkflowStore::new(root.clone());

        store.create("release.fwp", &workflow("release")).unwrap();
        assert_eq!(store.open("release.fwp").unwrap().name, "release");
        store.duplicate("release.fwp", "release-copy.fwp").unwrap();
        assert_eq!(store.list().unwrap().len(), 2);

        let archived = store.archive("release-copy.fwp", 123).unwrap();
        assert_eq!(archived, ".trash/release-copy-123.fwp");
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(root.join(archived).is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_paths_outside_workflow_root() {
        let store = WorkflowStore::new(std::env::temp_dir().join("fpw-store-safe"));
        assert!(store.create("../outside.fwp", &workflow("bad")).is_err());
        assert!(store.create("absolute.txt", &workflow("bad")).is_err());
    }
}
