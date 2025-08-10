use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProject {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModVersion {
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
    pub date_published: String,
    pub files: Vec<ModFile>,
    pub dependencies: Vec<ModDependency>,
}

// Deduped ModUpdateInfo (only one definition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModUpdateInfo {
    pub project_id: String,
    pub current_version_id: String,
    pub latest_version_id: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModUpdateInfo {
    pub project_id: String,
    pub current_version_id: String,
    pub latest_version_id: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFile {
    pub url: String,
    pub filename: String,
    pub hashes: HashMap<String, String>,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDependency {
    pub project_id: String,
    pub version_id: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    pub project_id: String,
    pub version_id: String,
    pub filename: String,
    pub path: String,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
    pub installed_at: i64,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModUpdateInfo {
    pub project_id: String,
    pub current_version_id: String,
    pub latest_version_id: Option<String>,
    pub has_update: bool,
}