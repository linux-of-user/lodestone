use color_eyre::eyre::{Context, eyre};
use serde_json::Value;
use crate::error::Error;

pub async fn get_purpur_minecraft_versions() -> Result<Vec<String>, Error> {
    let http = reqwest::Client::new();
    let response: Value = serde_json::from_str(
        http.get("https://api.purpurmc.org/v2/purpur")
            .send()
            .await
            .context("Failed to get purpur versions")?
            .text()
            .await
            .context("Failed to get purpur versions")?
            .as_str(),
    )
    .context("Failed to get purpur versions")?;
    let mut versions = response["versions"]
        .as_array()
        .ok_or_else(|| eyre!("Failed to get purpur versions. Response format changed?"))?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();
    versions.reverse();
    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_get_purpur_minecraft_versions() {
        let versions = get_purpur_minecraft_versions().await.unwrap();
        assert!(!versions.is_empty());
    }
}