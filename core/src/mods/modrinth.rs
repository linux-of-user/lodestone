use super::provider::ModProvider;
use super::types::*;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::path::{PathBuf};
use std::time::SystemTime;

pub struct ModrinthProvider {
    client: Client,
}

impl ModrinthProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ModProvider for ModrinthProvider {
    async fn search(
        &self,
        query: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
    ) -> Result<Vec<ModProject>, anyhow::Error> {
        let mut facets = vec![r#""project_type:mod""#.to_string()];
        if let Some(loader) = loader {
            facets.push(format!(r#""categories:{}""#, loader));
        }
        if let Some(version) = game_version {
            facets.push(format!(r#""versions:{}""#, version));
        }
        let facets_json = format!("[[{}]]", facets.join(","));
        let url = format!(
            "https://api.modrinth.com/v2/search?q={}&facets={}&limit=25",
            urlencoding::encode(query),
            urlencoding::encode(&facets_json)
        );
        let resp = self.client.get(url).send().await?.error_for_status()?.json::<serde_json::Value>().await?;
        let mut projects = vec![];
        if let Some(results) = resp.get("hits").and_then(|h| h.as_array()) {
            for project in results {
                projects.push(ModProject {
                    id: project.get("project_id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                    slug: project.get("slug").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                    title: project.get("title").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                    description: project.get("description").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                    icon_url: project.get("icon_url").and_then(|x| x.as_str()).map(|s| s.to_string()),
                    loaders: project.get("loaders").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]),
                    game_versions: project.get("game_versions").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]),
                });
            }
        }
        Ok(projects)
    }

    async fn get_project(&self, id: &str) -> Result<ModProject, anyhow::Error> {
        let url = format!("https://api.modrinth.com/v2/project/{}", id);
        let resp = self.client.get(url).send().await?.error_for_status()?.json::<serde_json::Value>().await?;
        Ok(ModProject {
            id: resp.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            slug: resp.get("slug").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            title: resp.get("title").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            description: resp.get("description").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            icon_url: resp.get("icon_url").and_then(|x| x.as_str()).map(|s| s.to_string()),
            loaders: resp.get("loaders").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]),
            game_versions: resp.get("game_versions").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]),
        })
    }

    async fn get_versions(
        &self,
        project_id: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
    ) -> Result<Vec<ModVersion>, anyhow::Error> {
        let url = format!("https://api.modrinth.com/v2/project/{}/version", project_id);
        let resp = self.client.get(url).send().await?.error_for_status()?.json::<Vec<serde_json::Value>>().await?;
        let mut versions = vec![];
        for version in resp {
            let loaders = version.get("loaders").and_then(|l| l.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]);
            let game_versions = version.get("game_versions").and_then(|l| l.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]);
            if let Some(loader_filter) = loader {
                if !loaders.iter().any(|l| l == loader_filter) {
                    continue;
                }
            }
            if let Some(game_version_filter) = game_version {
                if !game_versions.iter().any(|g| g == game_version_filter) {
                    continue;
                }
            }
            let files = version.get("files").and_then(|f| f.as_array()).unwrap_or(&vec![]);
            let mod_files = files
                .iter()
                .map(|file| ModFile {
                    url: file.get("url").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                    filename: file.get("filename").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                    hashes: file.get("hashes").and_then(|h| h.as_object()).map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()).unwrap_or_default(),
                    primary: file.get("primary").and_then(|x| x.as_bool()).unwrap_or(false),
                })
                .collect();
            let dependencies = version
                .get("dependencies")
                .and_then(|d| d.as_array())
                .unwrap_or(&vec![])
                .iter()
                .map(|dep| ModDependency {
                    project_id: dep.get("project_id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                    version_id: dep.get("version_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
                    required: dep.get("dependency_type").and_then(|x| x.as_str()).unwrap_or("") == "required",
                })
                .collect();
            versions.push(ModVersion {
                id: version.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                project_id: project_id.to_string(),
                version_number: version.get("version_number").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                loaders,
                game_versions,
                date_published: version.get("date_published").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                files: mod_files,
                dependencies,
            });
        }
        Ok(versions)
    }

    async fn get_version(&self, version_id: &str) -> Result<ModVersion, anyhow::Error> {
        let url = format!("https://api.modrinth.com/v2/version/{}", version_id);
        let version = self.client.get(url).send().await?.error_for_status()?.json::<serde_json::Value>().await?;
        let loaders = version.get("loaders").and_then(|l| l.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]);
        let game_versions = version.get("game_versions").and_then(|l| l.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or(vec![]);
        let files = version.get("files").and_then(|f| f.as_array()).unwrap_or(&vec![]);
        let mod_files = files
            .iter()
            .map(|file| ModFile {
                url: file.get("url").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                filename: file.get("filename").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                hashes: file.get("hashes").and_then(|h| h.as_object()).map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()).unwrap_or_default(),
                primary: file.get("primary").and_then(|x| x.as_bool()).unwrap_or(false),
            })
            .collect();
        let dependencies = version
            .get("dependencies")
            .and_then(|d| d.as_array())
            .unwrap_or(&vec![])
            .iter()
            .map(|dep| ModDependency {
                project_id: dep.get("project_id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                version_id: dep.get("version_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
                required: dep.get("dependency_type").and_then(|x| x.as_str()).unwrap_or("") == "required",
            })
            .collect();
        Ok(ModVersion {
            id: version.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            project_id: version.get("project_id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            version_number: version.get("version_number").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            loaders,
            game_versions,
            date_published: version.get("date_published").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            files: mod_files,
            dependencies,
        })
    }

    async fn resolve_required_dependencies(
        &self,
        version_id: &str,
    ) -> Result<Vec<ModVersion>, anyhow::Error> {
        let mut resolved = Vec::new();
        let mut stack = vec![version_id.to_string()];
        let mut visited = HashSet::new();

        while let Some(version_id) = stack.pop() {
            if !visited.insert(version_id.clone()) {
                continue;
            }
            let ver = self.get_version(&version_id).await?;
            for dep in &ver.dependencies {
                if dep.required {
                    if let Some(dep_version_id) = &dep.version_id {
                        stack.push(dep_version_id.clone());
                    }
                }
            }
            resolved.push(ver);
        }
        Ok(resolved)
    }

    async fn download(
        &self,
        file_url: &str,
        expected_hashes: &HashMap<String, String>,
    ) -> Result<PathBuf, anyhow::Error> {
        use sha2::{Digest, Sha512};
        use sha1::Sha1;
        use std::io::Write;
        use tempfile::NamedTempFile;

        let resp = self.client.get(file_url).send().await?.error_for_status()?;
        let bytes = resp.bytes().await?;
        // Prefer SHA512, fallback to SHA1
        if let Some(expected_sha512) = expected_hashes.get("sha512") {
            let mut hasher = Sha512::new();
            hasher.update(&bytes);
            let hash = format!("{:x}", hasher.finalize());
            if &hash != expected_sha512 {
                anyhow::bail!("SHA512 mismatch: expected {}, got {}", expected_sha512, hash);
            }
        } else if let Some(expected_sha1) = expected_hashes.get("sha1") {
            let mut hasher = Sha1::new();
            hasher.update(&bytes);
            let hash = format!("{:x}", hasher.finalize());
            if &hash != expected_sha1 {
                anyhow::bail!("SHA1 mismatch: expected {}, got {}", expected_sha1, hash);
            }
        }
        let mut tmpfile = NamedTempFile::new()?;
        tmpfile.write_all(&bytes)?;
        let path = tmpfile.into_temp_path().keep()?;
        Ok(path)
    }

    fn choose_primary_file<'a>(
        &self,
        version: &'a ModVersion,
        loader: &str,
    ) -> Option<&'a ModFile> {
        version.files.iter().find(|f| f.primary)
            .or_else(|| version.files.iter().find(|f| f.filename.ends_with(".jar") && f.filename.contains(loader)))
            .or_else(|| version.files.first())
    }
}