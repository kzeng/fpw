use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::Result;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProjects {
    #[serde(default)]
    pub projects: Vec<RecentProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub updated_at_unix_ms: u128,
}

pub fn app_data_dir() -> PathBuf {
    if let Some(fpw_config_home) = env::var_os("FPW_CONFIG_HOME") {
        return PathBuf::from(fpw_config_home);
    }

    if cfg!(windows) {
        if let Some(appdata) = env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("fpw");
        }
    }

    if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config_home).join("fpw");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config").join("fpw");
    }

    PathBuf::from(".fpw")
}

pub fn recent_projects_path() -> PathBuf {
    app_data_dir().join("recent-projects.json")
}

pub fn load_recent_projects(path: Option<&Path>) -> Result<RecentProjects> {
    let path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(recent_projects_path);
    if !path.is_file() {
        return Ok(RecentProjects::default());
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

pub fn save_recent_projects(path: Option<&Path>, recent: &RecentProjects) -> Result<()> {
    let path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(recent_projects_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(recent)?)?;
    Ok(())
}

pub fn touch_recent_project(
    path: Option<&Path>,
    workflow_path: &Path,
    name: &str,
    updated_at_unix_ms: u128,
) -> Result<RecentProjects> {
    let mut recent = load_recent_projects(path)?;
    let workflow_path = workflow_path.to_string_lossy().to_string();
    recent
        .projects
        .retain(|project| project.path != workflow_path);
    recent.projects.insert(
        0,
        RecentProject {
            path: workflow_path,
            name: name.to_string(),
            updated_at_unix_ms,
        },
    );
    recent.projects.truncate(20);
    save_recent_projects(path, &recent)?;
    Ok(recent)
}
