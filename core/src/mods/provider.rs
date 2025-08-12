use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

use super::types::*;

#[async_trait]
pub trait ModProvider: Send + Sync {
    async fn search(
        &self,
        query: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
    ) -> Result<Vec<ModProject>, anyhow::Error>;

    async fn get_project(&self, id: &str) -> Result<ModProject, anyhow::Error>;

    async fn get_versions(
        &self,
        project_id: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
    ) -> Result<Vec<ModVersion>, anyhow::Error>;

    async fn get_version(&self, version_id: &str) -> Result<ModVersion, anyhow::Error>;

    async fn resolve_required_dependencies(
        &self,
        version_id: &str,
    ) -> Result<Vec<ModVersion>, anyhow::Error>;

    async fn download(
        &self,
        file_url: &str,
        expected_hashes: &HashMap<String, String>,
    ) -> Result<PathBuf, anyhow::Error>;

    fn choose_primary_file<'a>(
        &self,
        version: &'a ModVersion,
        loader: &str,
    ) -> Option<&'a ModFile>;
}