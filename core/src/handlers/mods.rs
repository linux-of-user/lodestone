use axum::{
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json, Router,
};
use axum_auth::AuthBearer;
use serde::Deserialize;

use crate::auth::user::{UserAction, try_auth_or_err};
use crate::mods::modrinth::ModrinthProvider;
use crate::mods::provider::ModProvider;
use crate::mods::types::*;
use crate::mods::manifest::*;
use crate::{AppState, types::InstanceUuid, prelude::GameInstance};
use crate::implementations::minecraft::Flavour;
use chrono::Utc;
use sanitize_filename::sanitize;
use color_eyre::eyre::{eyre, Context, Result};

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub loader: Option<String>,
    pub game_version: Option<String>,
}

#[derive(Deserialize)]
pub struct InstallBody {
    pub project_id: String,
    pub version_id: Option<String>,
}

pub fn get_mods_routes(state: AppState) -> Router {
    Router::new()
        .route("/mods/search", get(search_mods))
        .route("/mods/projects/:id", get(get_project))
        .route("/mods/projects/:id/versions", get(get_project_versions))
        .route("/instances/:uuid/mods", get(list_installed))
        .route("/instances/:uuid/mods/install", post(install_mod))
        .route("/instances/:uuid/mods/:file_name", delete(uninstall_mod))
        .route("/instances/:uuid/mods/updates", get(list_updates))
        .with_state(state)
}

fn loader_and_dir_from_flavour(flavour: &Flavour) -> Result<(&'static str, &'static str)> {
    match flavour {
        Flavour::Fabric { .. } | Flavour::Forge { .. } => Ok(("fabric", "mods")),
        Flavour::Paper { .. } | Flavour::Purpur { .. } | Flavour::Spigot => Ok(("paper", "plugins")),
        Flavour::Quilt { .. } => Ok(("quilt", "mods")),
        Flavour::Vanilla => Err(eyre!("Vanilla does not support mods/plugins").into()),
    }
}

async fn search_mods(
    State(_state): State<AppState>,
    Query(query): Query<SearchParams>
) -> Result<Json<Vec<ModProject>>, crate::error::Error> {
    let provider = ModrinthProvider::new();
    let results = provider
        .search(&query.query, query.loader.as_deref(), query.game_version.as_deref())
        .await
        .map_err(crate::error::Error::from)?;
    Ok(Json(results))
}

async fn get_project(
    State(_state): State<AppState>,
    Path(id): Path<String>
) -> Result<Json<ModProject>, crate::error::Error> {
    let provider = ModrinthProvider::new();
    let project = provider.get_project(&id).await.map_err(crate::error::Error::from)?;
    Ok(Json(project))
}

async fn get_project_versions(
    State(_state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SearchParams>
) -> Result<Json<Vec<ModVersion>>, crate::error::Error> {
    let provider = ModrinthProvider::new();
    let versions = provider.get_versions(
        &id,
        query.loader.as_deref(),
        query.game_version.as_deref()
    ).await.map_err(crate::error::Error::from)?;
    Ok(Json(versions))
}

async fn list_installed(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>
) -> Result<Json<Vec<InstalledMod>>, crate::error::Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::ViewInstance(uuid.clone()), false)?;
    let inst = state.instances.get(&uuid).ok_or_else(|| crate::error::Error::not_found("Instance"))?;
    let instance_path = inst.path().await;
    let mods = load_manifest(&instance_path).await.unwrap_or_default();
    Ok(Json(mods))
}

async fn install_mod(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>,
    Json(body): Json<InstallBody>
) -> Result<Json<Vec<InstalledMod>>, crate::error::Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::WriteInstanceFile(uuid.clone()), false)?;

    let inst = state.instances.get(&uuid).ok_or_else(|| crate::error::Error::not_found("Instance"))?;
    let flavour = inst.flavour().await;
    let game_version = inst.game_version().await;
    let instance_path = inst.path().await;

    let (loader, dir) = loader_and_dir_from_flavour(&flavour)?;

    let provider = ModrinthProvider::new();
    let root_version = if let Some(ver_id) = &body.version_id {
        provider.get_version(ver_id).await.map_err(crate::error::Error::from)?
    } else {
        let versions = provider.get_versions(&body.project_id, Some(loader), Some(&game_version)).await.map_err(crate::error::Error::from)?;
        versions.into_iter().max_by_key(|v| v.date_published.clone()).ok_or_else(|| crate::error::Error::bad_request("No compatible version found"))?
    };
    let required_versions = provider.resolve_required_dependencies(&root_version.id).await.map_err(crate::error::Error::from)?;

    let mut ordered = vec![root_version.clone()];
    ordered.extend(required_versions.clone());

    let target_dir = instance_path.join(dir);
    tokio::fs::create_dir_all(&target_dir).await.map_err(crate::error::Error::from)?;

    let mut installed = Vec::new();
    for version in &ordered {
        let file = provider.choose_primary_file(version, loader).ok_or_else(|| crate::error::Error::bad_request("No primary file"))?;
        let filename = sanitize(&file.filename);
        let dest_path = target_dir.join(&filename);
        let tmp_path = dest_path.with_extension("tmp");
        provider.download(&file.url, &file.hashes, &tmp_path).await.map_err(crate::error::Error::from)?;
        tokio::fs::rename(&tmp_path, &dest_path).await.map_err(crate::error::Error::from)?;

        installed.push(InstalledMod {
            project_id: version.project_id.clone(),
            version_id: version.id.clone(),
            file_name: filename.clone(),
            loaders: version.loaders.clone(),
            game_versions: version.game_versions.clone(),
            installed_at: Utc::now().timestamp(),
            dependencies: version.dependencies.iter().filter_map(|d| d.project_id.clone()).collect(),
            sha1: file.hashes.and_then(|h| h.sha1),
        });
    }
    // Load, append, and save manifest
    let mut manifest = load_manifest(&instance_path).await.unwrap_or_default();
    manifest.extend(installed.clone());
    save_manifest(&instance_path, &manifest).await.map_err(crate::error::Error::from)?;

    Ok(Json(installed))
}

async fn uninstall_mod(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path((uuid, file_name)): Path<(String, String)>
) -> Result<Json<()>, crate::error::Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::WriteInstanceFile(uuid.clone()), false)?;

    let inst = state.instances.get(&uuid).ok_or_else(|| crate::error::Error::not_found("Instance"))?;
    let instance_path = inst.path().await;
    let flavour = inst.flavour().await;
    let (_loader, dir) = loader_and_dir_from_flavour(&flavour)?;
    let target_dir = instance_path.join(dir);

    let mut manifest = load_manifest(&instance_path).await.unwrap_or_default();
    let idx = manifest.iter().position(|m| m.file_name == file_name).ok_or_else(|| crate::error::Error::not_found("Mod"))?;
    let mod_entry = &manifest[idx];

    let dependents: Vec<_> = manifest.iter().filter(|e| e.dependencies.contains(&mod_entry.project_id)).map(|e| e.project_id.clone()).collect();
    if !dependents.is_empty() {
        return Err(crate::error::Error::bad_request(&format!("Cannot remove mod; required by {:?}", dependents)));
    }

    let file_path = target_dir.join(&file_name);
    tokio::fs::remove_file(&file_path).await.map_err(crate::error::Error::from)?;
    manifest.remove(idx);
    save_manifest(&instance_path, &manifest).await.map_err(crate::error::Error::from)?;

    Ok(Json(()))
}

async fn list_updates(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>
) -> Result<Json<Vec<ModUpdateInfo>>, crate::error::Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::ViewInstance(uuid.clone()), false)?;

    let inst = state.instances.get(&uuid).ok_or_else(|| crate::error::Error::not_found("Instance"))?;
    let flavour = inst.flavour().await;
    let game_version = inst.game_version().await;
    let instance_path = inst.path().await;
    let (loader, _) = loader_and_dir_from_flavour(&flavour)?;

    let provider = ModrinthProvider::new();
    let manifest = load_manifest(&instance_path).await.unwrap_or_default();
    let mut updates = Vec::new();
    for entry in manifest {
        let versions = provider.get_versions(&entry.project_id, Some(loader), Some(&game_version)).await.map_err(crate::error::Error::from)?;
        if let Some(latest) = versions.iter().max_by_key(|v| v.date_published.clone()) {
            let has_update = latest.id != entry.version_id;
            updates.push(ModUpdateInfo {
                project_id: entry.project_id.clone(),
                current_version_id: entry.version_id.clone(),
                latest_version_id: latest.id.clone(),
                has_update,
            });
        }
    }
    Ok(Json(updates))
}