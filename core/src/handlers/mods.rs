use axum::{
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json, Router,
};
use axum_auth::AuthBearer;
use serde::Deserialize;
use std::collections::{HashSet, HashMap};
use std::path::{PathBuf};
use chrono::Utc;
use sanitize_filename::sanitize;
use color_eyre::eyre::{eyre, Context};

use crate::{
    AppState,
    error::{Error, ErrorKind},
    mods::{modrinth::ModrinthProvider, provider::ModProvider, types::*, manifest::*},
    types::InstanceUuid,
    prelude::GameInstance,
    auth::user::{UserAction, try_auth_or_err},
    implementations::minecraft::Flavour,
};

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

fn loader_and_dir_from_flavour(flavour: &Flavour) -> Result<(&'static str, &'static str), Error> {
    match flavour {
        Flavour::Fabric { .. } => Ok(("fabric", "mods")),
        Flavour::Forge { .. } => Ok(("forge", "mods")),
        Flavour::Paper { .. } => Ok(("paper", "plugins")),
        Flavour::Purpur { .. } => Ok(("purpur", "plugins")),
        Flavour::Spigot => Ok(("spigot", "plugins")),
        Flavour::Vanilla => Err(Error { kind: ErrorKind::BadRequest, source: eyre!("Vanilla does not support mods/plugins") }),
    }
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

async fn search_mods(
    State(_state): State<AppState>,
    Query(query): Query<SearchParams>
) -> Result<Json<Vec<ModProject>>, Error> {
    let provider = ModrinthProvider::new();
    let results = provider
        .search(&query.query, query.loader.as_deref(), query.game_version.as_deref())
        .await
        .context("search_mods failed")?;
    Ok(Json(results))
}

async fn get_project(
    State(_state): State<AppState>,
    Path(id): Path<String>
) -> Result<Json<ModProject>, Error> {
    let provider = ModrinthProvider::new();
    let project = provider.get_project(&id).await.context("get_project failed")?;
    Ok(Json(project))
}

async fn get_project_versions(
    State(_state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SearchParams>
) -> Result<Json<Vec<ModVersion>>, Error> {
    let provider = ModrinthProvider::new();
    let versions = provider
        .get_versions(&id, query.loader.as_deref(), query.game_version.as_deref())
        .await
        .context("get_project_versions failed")?;
    Ok(Json(versions))
}

async fn list_installed(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>
) -> Result<Json<Vec<InstalledMod>>, Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::ViewInstance(uuid.clone()), state.global_settings.lock().await.safe_mode())?;
    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: eyre!("Instance not found") })?;
    let path = inst.path().await;
    Ok(Json(load_manifest(&path).await?))
}

async fn install_mod(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>,
    Json(body): Json<InstallBody>
) -> Result<Json<Vec<InstalledMod>>, Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::WriteInstanceFile(uuid.clone()), state.global_settings.lock().await.safe_mode())?;

    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: eyre!("Instance not found") })?;
    let flavour = inst.flavour().await;
    let game_version = inst.game_version().await;
    let instance_path = inst.path().await;
    let (loader, install_dir) = loader_and_dir_from_flavour(&flavour)?;

    let provider = ModrinthProvider::new();
    let root_version = if let Some(ver_id) = &body.version_id {
        provider.get_version(ver_id).await.context("get_version failed")?
    } else {
        let mut versions = provider.get_versions(&body.project_id, Some(loader), Some(&game_version)).await.context("get_versions failed")?;
        versions
            .into_iter()
            .max_by_key(|v| v.date_published.clone())
            .ok_or(Error { kind: ErrorKind::BadRequest, source: eyre!("No compatible version found") })?
    };

    let mut to_install = vec![root_version.clone()];
    let mut visited = HashSet::new();
    visited.insert(root_version.id.clone());
    let dep_versions = provider.resolve_required_dependencies(&root_version.id).await.context("resolve_required_dependencies failed")?;
    for dep in dep_versions {
        if visited.insert(dep.id.clone()) {
            to_install.push(dep);
        }
    }

    let dest_dir = instance_path.join(install_dir);
    tokio::fs::create_dir_all(&dest_dir).await.context("create_dir_all failed")?;

    let mut installed = Vec::new();
    let mut moved_files = Vec::new();
    for version in &to_install {
        let file = provider.choose_primary_file(version, loader).ok_or(Error { kind: ErrorKind::BadRequest, source: eyre!("No primary file found") })?;
        let tmp_path = provider.download(&file.url, &file.hashes).await.context("download failed")?;
        let filename = sanitize(&file.filename);
        let dest_path = dest_dir.join(&filename);
        if !dest_path.starts_with(&dest_dir) {
            return Err(Error { kind: ErrorKind::BadRequest, source: eyre!("Unsafe mod path for install") });
        }
        tokio::fs::rename(&tmp_path, &dest_path).await.context("rename mod file failed")?;
        moved_files.push(dest_path.clone());
        installed.push(InstalledMod {
            project_id: version.id.clone(),
            version_id: version.version_number.clone(),
            filename: filename.clone(),
            path: dest_path.to_string_lossy().to_string(),
            loaders: version.loaders.clone(),
            game_versions: version.game_versions.clone(),
            installed_at: Utc::now().timestamp(),
            dependencies: if &version.id == &root_version.id {
                to_install.iter().skip(1).map(|v| v.id.clone()).collect()
            } else {
                Vec::new()
            },
        });
    }

    // Load, append, and save manifest
    let mut manifest = load_manifest(&instance_path).await.unwrap_or_default();
    manifest.extend(installed.clone());
    save_manifest(&instance_path, &manifest).await.context("save_manifest failed")?;

    Ok(Json(installed))
}

async fn uninstall_mod(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path((uuid, file_name)): Path<(String, String)>
) -> Result<Json<()>, Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::WriteInstanceFile(uuid.clone()), state.global_settings.lock().await.safe_mode())?;

    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: eyre!("Instance not found") })?;
    let instance_path = inst.path().await;
    let flavour = inst.flavour().await;
    let (_loader, dir) = loader_and_dir_from_flavour(&flavour)?;
    let target_dir = instance_path.join(dir);

    let mut manifest = load_manifest(&instance_path).await?;
    let idx = manifest.iter().position(|m| m.filename == file_name).ok_or(Error { kind: ErrorKind::NotFound, source: eyre!("Mod not found") })?;
    let mod_entry = &manifest[idx];

    let dependents: Vec<_> = manifest.iter().filter(|e| e.dependencies.contains(&mod_entry.project_id)).map(|e| e.project_id.clone()).collect();
    if !dependents.is_empty() {
        return Err(Error { kind: ErrorKind::BadRequest, source: eyre!("Cannot remove mod: required by {:?}", dependents) });
    }

    let file_path = target_dir.join(&file_name);
    let _ = tokio::fs::remove_file(&file_path).await;
    manifest.remove(idx);
    save_manifest(&instance_path, &manifest).await?;

    Ok(Json(()))
}

async fn list_updates(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>
) -> Result<Json<Vec<ModUpdateInfo>>, Error> {
    let uuid = InstanceUuid(uuid);
    let requester = state.users_manager.read().await.try_auth_or_err(&token)?;
    requester.try_action(&UserAction::ViewInstance(uuid.clone()), state.global_settings.lock().await.safe_mode())?;

    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: eyre!("Instance not found") })?;
    let flavour = inst.flavour().await;
    let game_version = inst.game_version().await;
    let instance_path = inst.path().await;
    let (loader, _) = loader_and_dir_from_flavour(&flavour)?;
    let provider = ModrinthProvider::new();
    let manifest = load_manifest(&instance_path).await.unwrap_or_default();
    let mut updates = Vec::new();
    for entry in &manifest {
        let versions = provider.get_versions(&entry.project_id, Some(loader), Some(&game_version)).await.unwrap_or_default();
        let newest = versions.iter().max_by_key(|v| v.date_published.clone());
        let has_update = match newest {
            Some(newest) => newest.version_number != entry.version_id,
            None => false,
        };
        updates.push(ModUpdateInfo {
            project_id: entry.project_id.clone(),
            current_version_id: entry.version_id.clone(),
            latest_version_id: newest.map(|v| v.version_number.clone()),
            has_update,
        });
    }
    Ok(Json(updates))
}