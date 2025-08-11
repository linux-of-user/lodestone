use super::{provider::ModProvider, types::*};
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

pub struct CurseForgeProvider {
    client: Client,
    api_key: String,
}

impl CurseForgeProvider {
    pub fn new() -> Result<Self, anyhow::Error> {
        let api_key = dotenv::var("CURSEFORGE_API_KEY").or_else(|_| env::var("CURSEFORGE_API_KEY"))
            .map_err(|_| anyhow::anyhow!("CURSEFORGE_API_KEY missing"))?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }
}

#[async_trait]
impl ModProvider for CurseForgeProvider {
    async fn search(
        &self,
        query: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
    ) -> Result<Vec<ModProject>, anyhow::Error> {
        let mut url = format!("https://api.curseforge.com/v1/mods/search?gameId=432&searchFilter={}", urlencoding::encode(query));
        if let Some(loader) = loader {
            url.push_str(&format!("&modLoaderType={}", match loader.to_lowercase().as_str() {
                "fabric" => "4",
                "forge" => "1",
                "quilt" => "6",
                "paper" | "purpur" | "spigot" => "0", // 0 = any modloader
                _ => "0"
            }));
        }
        if let Some(game_version) = game_version {
            url.push_str(&format!("&gameVersion={}", urlencoding::encode(game_version)));
        }
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let mut results = Vec::new();
        if let Some(mods) = resp.get("data").and_then(|d| d.as_array()) {
            for m in mods {
                results.push(ModProject {
                    id: m.get("id").and_then(|x| x.as_u64()).unwrap_or(0).to_string(),
                    slug: m.get("slug").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    title: m.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    description: m.get("summary").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    icon_url: m.get("logo").and_then(|l| l.get("url")).and_then(|x| x.as_str()).map(|s| s.to_string()),
                    loaders: vec![], // Not always available in search, can refine
                    game_versions: vec![], // Not always present
                });
            }
        }
        Ok(results)
    }

    async fn get_project(&self, id: &str) -> Result<ModProject, anyhow::Error> {
        let url = format!("https://api.curseforge.com/v1/mods/{}", id);
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let m = resp.get("data").ok_or_else(|| anyhow::anyhow!("No project"))?;
        Ok(ModProject {
            id: m.get("id").and_then(|x| x.as_u64()).unwrap_or(0).to_string(),
            slug: m.get("slug").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            title: m.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            description: m.get("summary").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            icon_url: m.get("logo").and_then(|l| l.get("url")).and_then(|x| x.as_str()).map(|s| s.to_string()),
            loaders: vec![], // Could extract from latestFilesIndexes if present
            game_versions: vec![],
        })
    }

    async fn get_versions(
        &self,
        project_id: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
    ) -> Result<Vec<ModVersion>, anyhow::Error> {
        let mut url = format!("https://api.curseforge.com/v1/mods/{}/files", project_id);
        if let Some(game_version) = game_version {
            url.push_str(&format!("?gameVersion={}", urlencoding::encode(game_version)));
        }
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let mut versions = Vec::new();
        if let Some(files) = resp.get("data").and_then(|d| d.as_array()) {
            for f in files {
                // Filter by modloader if desired (not always present)
                let files_game_versions: Vec<String> = f.get("gameVersions")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                if let Some(loader) = loader {
                    let loader_match = files_game_versions.iter().any(|v| v.to_lowercase().contains(&loader.to_lowercase()));
                    if !loader_match { continue; }
                }
                let dep_vec = f.get("dependencies").and_then(|d| d.as_array()).map(|arr| arr.iter().filter_map(|d| d.get("modId").and_then(|id| id.as_u64()).map(|id| ModDependency { project_id: id.to_string(), version_id: None, required: d.get("relationType").and_then(|t| t.as_u64()) == Some(3) })).collect()).unwrap_or_default();
                let mod_files = vec![ModFile {
                    url: f.get("downloadUrl").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    filename: f.get("fileName").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    hashes: HashMap::new(), // CurseForge does not expose hashes directly in file info
                    primary: true,
                }];
                versions.push(ModVersion {
                    id: f.get("id").and_then(|x| x.as_u64()).unwrap_or(0).to_string(),
                    project_id: project_id.to_string(),
                    version_number: f.get("displayName").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    loaders: files_game_versions.clone(),
                    game_versions: files_game_versions,
                    date_published: f.get("fileDate").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    files: mod_files,
                    dependencies: dep_vec,
                });
            }
        }
        Ok(versions)
    }

    async fn get_version(&self, version_id: &str) -> Result<ModVersion, anyhow::Error> {
        let url = format!("https://api.curseforge.com/v1/files/{}", version_id);
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let f = resp.get("data").ok_or_else(|| anyhow::anyhow!("No file"))?;
        let files_game_versions: Vec<String> = f.get("gameVersions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        let dep_vec = f.get("dependencies").and_then(|d| d.as_array()).map(|arr| arr.iter().filter_map(|d| d.get("modId").and_then(|id| id.as_u64()).map(|id| ModDependency { project_id: id.to_string(), version_id: None, required: d.get("relationType").and_then(|t| t.as_u64()) == Some(3) })).collect()).unwrap_or_default();
        let mod_files = vec![ModFile {
            url: f.get("downloadUrl").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            filename: f.get("fileName").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            hashes: HashMap::new(),
            primary: true,
        }];
        Ok(ModVersion {
            id: f.get("id").and_then(|x| x.as_u64()).unwrap_or(0).to_string(),
            project_id: f.get("modId").and_then(|x| x.as_u64()).map(|id| id.to_string()).unwrap_or_default(),
            version_number: f.get("displayName").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            loaders: files_game_versions.clone(),
            game_versions: files_game_versions,
            date_published: f.get("fileDate").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            files: mod_files,
            dependencies: dep_vec,
        })
    }

    async fn resolve_required_dependencies(
        &self,
        version_id: &str,
    ) -> Result<Vec<ModVersion>, anyhow::Error> {
        // Not implemented for now; would require recursive lookup by modId and fileId.
        Ok(vec![])
    }

    async fn download(
        &self,
        file_url: &str,
        _expected_hashes: &HashMap<String, String>,
    ) -> Result<PathBuf, anyhow::Error> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let resp = self.client.get(file_url).send().await?.error_for_status()?;
        let bytes = resp.bytes().await?;
        let mut tmpfile = NamedTempFile::new()?;
        tmpfile.write_all(&bytes)?;
        let path = tmpfile.into_temp_path().keep()?;
        Ok(path)
    }

    fn choose_primary_file<'a>(
        &self,
        version: &'a ModVersion,
        _loader: &str,
    ) -> Option<&'a ModFile> {
        version.files.iter().find(|f| f.primary)
            .or_else(|| version.files.first())
    }
}