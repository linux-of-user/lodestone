use color_eyre::eyre::{eyre, Context};
use serde_json::Value;

use crate::error::Error;

pub async fn get_quilt_minecraft_versions() -> Result<Vec<String>, Error> {
    let http = reqwest::Client::new();

    let response: Value = serde_json::from_str(
        http.get("https://meta.quiltmc.org/v3/versions/game")
            .send()
            .await
            .context("Failed to get quilt versions")?
            .text()
            .await
            .context("Failed to get quilt versions")?
            .as_str(),
    )
    .context("Failed to get quilt versions")?;

    let versions = response
        .as_array()
        .ok_or_else(|| eyre!("Failed to get quilt versions. Response format changed?"))?
        .iter()
        .filter_map(|v| v.get("version").and_then(|id| id.as_str()))
        .map(|id| id.to_string())
        .collect::<Vec<String>>();

    Ok(versions)
}

pub async fn get_quilt_loader_versions() -> Result<Vec<String>, Error> {
    let http = reqwest::Client::new();

    let response: Value = serde_json::from_str(
        http.get("https://meta.quiltmc.org/v3/versions/loader")
            .send()
            .await
            .context("Failed to get quilt loader versions")?
            .text()
            .await
            .context("Failed to get quilt loader versions")?
            .as_str(),
    )
    .context("Failed to get quilt loader versions")?;

    let versions = response
        .as_array()
        .ok_or_else(|| eyre!("Failed to get quilt loader versions. Response format changed?"))?
        .iter()
        .filter_map(|v| v.get("version").and_then(|s| s.as_str()))
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    Ok(versions)
}

pub async fn get_quilt_installer_versions() -> Result<Vec<String>, Error> {
    let http = reqwest::Client::new();

    let response: Value = serde_json::from_str(
        http.get("https://meta.quiltmc.org/v3/versions/installer")
            .send()
            .await
            .context("Failed to get quilt installer versions")?
            .text()
            .await
            .context("Failed to get quilt installer versions")?
            .as_str(),
    )
    .context("Failed to get quilt installer versions")?;

    let versions = response
        .as_array()
        .ok_or_else(|| eyre!("Failed to get quilt installer versions. Response format changed?"))?
        .iter()
        .filter_map(|v| v.get("version").and_then(|s| s.as_str()))
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    Ok(versions)
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_get_quilt_minecraft_versions() {
        let versions = get_quilt_minecraft_versions().await.unwrap();
        assert!(!versions.is_empty());
    }
}