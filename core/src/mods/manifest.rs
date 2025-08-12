use std::path::{Path, PathBuf};
use std::io::SeekFrom;
use crate::error::Error;
use crate::mods::types::InstalledMod;
use tokio::fs;
use serde_json;

pub fn manifest_path(instance_path: &Path) -> PathBuf {
    instance_path.join("resources").join("mods").join("installed.json")
}

pub async fn load_manifest(instance_path: &Path) -> Result<Vec<InstalledMod>, Error> {
    let path = manifest_path(instance_path);
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read(&path).await?;
    let mods: Vec<InstalledMod> = serde_json::from_slice(&data)?;
    Ok(mods)
}

pub async fn save_manifest(instance_path: &Path, mods: &Vec<InstalledMod>) -> Result<(), Error> {
    let path = manifest_path(instance_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let tmp_path = path.with_extension("tmp");
    let data = serde_json::to_vec_pretty(mods)?;
    fs::write(&tmp_path, &data).await?;
    fs::rename(&tmp_path, &path).await?;
    Ok(())
}