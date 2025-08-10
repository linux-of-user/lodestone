use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::{
    auth::user::{UserAction, UserToken},
    mod_manager::service::ModManager,
    mod_manager::types::{InstalledEntry, ModUpdateInfo, ProjectCard, Project, ProjectVersion},
    types::InstanceUuid,
    AppState,
};
use chrono::Utc;
use sanitize_filename::sanitize;
use color_eyre::eyre::{eyre, Context};
use tracing::{error, info};

#[derive(Deserialize)]
pub struct SearchParams {
    query: String,
    loader: Option<String>,
    game_version: Option<String>,
}

// Helper: derive loader and install_dir from flavour
fn loader_and_dir_from_flavour(flavour: &str) -> (&str, &str) {
    match flavour.to_lowercase().as_str() {
        "fabric" | "forge" | "quilt" => ("fabric", "mods"),
        "paper" | "purpur" | "spigot" => ("paper", "plugins"),
        _ => ("fabric", "mods"),
    }
}

pub fn get_mods_routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/mods/search", axum::routing::get(search_mods))
        .route("/mods/project/:id", axum::routing::get(get_project))
        .route("/mods/project/:id/versions", axum::routing::get(get_project_versions))
        .route("/instance/:uuid/mods", axum::routing::get(list_installed))
        .route("/instance/:uuid/mods/install", axum::routing::post(install_mod))
        .route("/instance/:uuid/mods/:file_name", axum::routing::delete(uninstall_mod))
        .route("/instance/:uuid/mods/updates", axum::routing::get(list_updates))
        .with_state(shared_state)
}

async fn search_mods(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
    UserToken(user): UserToken,
) -> Result<Json<Vec<ProjectCard>>, axum::response::Response> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&user)
        .ok_or_else(|| axum::response::IntoResponse::into_response("Unauthorized"))?;
    // No instance-specific permission needed for search
    let provider = ModManager::new_modrinth();
    let results = provider
        .provider
        .search(&params.query, params.loader.as_deref(), params.game_version.as_deref())
        .await
        .map_err(axum::response::IntoResponse::into_response)?;
    Ok(Json(results))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    UserToken(user): UserToken,
) -> Result<Json<Project>, axum::response::Response> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&user)
        .ok_or_else(|| axum::response::IntoResponse::into_response("Unauthorized"))?;
    let provider = ModManager::new_modrinth();
    let project = provider
        .provider
        .get_project(&id)
        .await
        .map_err(axum::response::IntoResponse::into_response)?;
    Ok(Json(project))
}

async fn get_project_versions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<SearchParams>,
    UserToken(user): UserToken,
) -> Result<Json<Vec<ProjectVersion>>, axum::response::Response> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&user)
        .ok_or_else(|| axum::response::IntoResponse::into_response("Unauthorized"))?;
    let provider = ModManager::new_modrinth();
    let versions = provider
        .provider
        .get_project_versions(&id)
        .await
        .map_err(axum::response::IntoResponse::into_response)?;
    Ok(Json(versions))
}

async fn list_installed(
    State(state): State<Arc<AppState>>,
    Path(uuid): Path<String>,
    UserToken(user): UserToken,
) -> Result<Json<Vec<InstalledEntry>>, axum::response::Response> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&user)
        .ok_or_else(|| axum::response::IntoResponse::into_response("Unauthorized"))?;
    requester.try_action(
        &UserAction::ViewInstance(uuid.clone()),
        state.global_settings.lock().await.safe_mode(),
    )?;
    let instance_dir = PathBuf::from(format!("./instances/{}", uuid));
    let provider = ModManager::new_modrinth();
    let entries = provider
        .list_installed(instance_dir)
        .await
        .map_err(axum::response::IntoResponse::into_response)?;
    Ok(Json(entries))
}

#[derive(Deserialize)]
pub struct InstallBody {
    pub project_id: String,
    pub version_id: Option<String>,
}

async fn install_mod(
    State(state): State<Arc<AppState>>,
    Path(uuid): Path<String>,
    Json(body): Json<InstallBody>,
    UserToken(user): UserToken,
) -> Result<Json<Vec<InstalledEntry>>, axum::response::Response> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&user)
        .ok_or_else(|| axum::response::IntoResponse::into_response("Unauthorized"))?;
    requester.try_action(
        &UserAction::WriteInstanceFile(uuid.clone()),
        state.global_settings.lock().await.safe_mode(),
    )?;

    // Load instance config to infer loader/game_version/path
    let instances = &state.instances;
    let instance = instances
        .get(&InstanceUuid(uuid.clone()))
        .ok_or_else(|| axum::response::IntoResponse::into_response("Instance not found"))?;
    let (flavour, game_version) = (instance.flavour().await, instance.game_version().await);
    let instance_path = instance.instance_path().await;
    let (loader, install_dir) = loader_and_dir_from_flavour(&flavour.to_string());

    let provider = ModManager::new_modrinth();
    let project_id = &body.project_id;
    let version_id = body.version_id.as_deref();

    // Find root version
    let root_version = if let Some(version_id) = version_id {
        provider
            .provider
            .get_project_versions(project_id)
            .await
            .map_err(axum::response::IntoResponse::into_response)?
            .into_iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| axum::response::IntoResponse::into_response("Version not found"))?
    } else {
        let versions = provider
            .provider
            .get_project_versions(project_id)
            .await
            .map_err(axum::response::IntoResponse::into_response)?;
        versions
            .iter()
            .filter(|v| v.loaders.iter().any(|l| l == loader) && v.game_versions.iter().any(|g| g == &game_version))
            .max_by_key(|v| v.date_published.clone())
            .cloned()
            .ok_or_else(|| axum::response::IntoResponse::into_response("No compatible version found"))?
    };

    // Resolve required dependencies
    let mut to_install = vec![root_version.clone()];
    let mut visited = HashSet::new();
    visited.insert(root_version.id.clone());
    let mut stack = root_version
        .dependencies
        .iter()
        .filter_map(|d| d.project_id.clone())
        .collect::<Vec<_>>();

    while let Some(dep_proj_id) = stack.pop() {
        if !visited.insert(dep_proj_id.clone()) {
            continue;
        }
        let dep_versions = provider
            .provider
            .get_project_versions(&dep_proj_id)
            .await
            .map_err(axum::response::IntoResponse::into_response)?;
        if let Some(dep_version) = dep_versions
            .iter()
            .filter(|v| v.loaders.iter().any(|l| l == loader) && v.game_versions.iter().any(|g| g == &game_version))
            .max_by_key(|v| v.date_published.clone())
        {
            to_install.push(dep_version.clone());
            for d in dep_version.dependencies.iter().filter_map(|d| d.project_id.clone()) {
                stack.push(d);
            }
        }
    }

    let mut installed = vec![];
    let mut moved_paths = vec![];
    for version in &to_install {
        let file = provider
            .provider
            .get_primary_file_info(version)
            .ok_or_else(|| axum::response::IntoResponse::into_response("No primary file"))?;
        let tmp_path = {
            let resp = reqwest::get(&file.url).await.map_err(axum::response::IntoResponse::into_response)?;
            let bytes = resp.bytes().await.map_err(axum::response::IntoResponse::into_response)?;
            let tmp_path = std::env::temp_dir().join(format!("lodestone_mod_{}", sanitize(&file.filename)));
            tokio::fs::write(&tmp_path, &bytes).await.map_err(axum::response::IntoResponse::into_response)?;
            // Optionally hash check can go here
            tmp_path
        };
        let filename = sanitize(&file.filename);
        let dest_dir = instance_path.join(install_dir);
        tokio::fs::create_dir_all(&dest_dir).await.map_err(axum::response::IntoResponse::into_response)?;
        let dest_path = dest_dir.join(&filename);
        if !dest_path.starts_with(&dest_dir) {
            return Err(axum::response::IntoResponse::into_response("Unsafe mod path"));
        }
        tokio::fs::rename(&tmp_path, &dest_path).await.map_err(axum::response::IntoResponse::into_response)?;
        moved_paths.push(dest_path.clone());

        installed.push(InstalledEntry {
            project_id: version.project_id.clone(),
            version_id: version.id.clone(),
            file_name: filename.clone(),
            loaders: version.loaders.clone(),
            game_versions: version.game_versions.clone(),
            installed_at: Utc::now().timestamp(),
            dependencies: version
                .dependencies
                .iter()
                .filter_map(|d| d.project_id.clone())
                .collect(),
            sha1: file.hashes.and_then(|h| h.sha1),
        });
    }

    // Save manifest
    let mut manifest = provider.load_manifest(&instance_path).await.unwrap_or_default();
    manifest.extend(installed.clone());
    provider.save_manifest(&instance_path, &manifest).await.map_err(axum::response::IntoResponse::into_response)?;

    // Emit event
    // (example: state.event_broadcaster.send(...); not shown here)

    Ok(Json(installed))
}

async fn uninstall_mod(
    State(state): State<Arc<AppState>>,
    Path((uuid, file_name)): Path<(String, String)>,
    UserToken(user): UserToken,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&user)
        .ok_or_else(|| axum::response::IntoResponse::into_response("Unauthorized"))?;
    requester.try_action(
        &UserAction::WriteInstanceFile(uuid.clone()),
        state.global_settings.lock().await.safe_mode(),
    )?;
    let instances = &state.instances;
    let instance = instances
        .get(&InstanceUuid(uuid.clone()))
        .ok_or_else(|| axum::response::IntoResponse::into_response("Instance not found"))?;
    let flavour = instance.flavour().await;
    let instance_path = instance.instance_path().await;
    let (loader, install_dir) = loader_and_dir_from_flavour(&flavour.to_string());
    let provider = ModManager::new_modrinth();

    let mut manifest = provider.load_manifest(&instance_path).await.unwrap_or_default();
    let idx = manifest.iter().position(|e| e.file_name == file_name);
    let entry = match idx {
        Some(i) => &manifest[i],
        None => return Err(axum::response::IntoResponse::into_response("Mod not found")),
    };
    // Check for dependents
    let dependents: Vec<_> = manifest
        .iter()
        .filter(|e| e.dependencies.contains(&entry.project_id))
        .collect();
    if !dependents.is_empty() {
        let names = dependents
            .iter()
            .map(|e| e.file_name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(axum::response::IntoResponse::into_response(format!(
            "Cannot remove: other installed mods depend on this ({})",
            names
        )));
    }
    // Remove file
    let dest_path = instance_path.join(install_dir).join(&file_name);
    let _ = tokio::fs::remove_file(&dest_path).await;
    // Remove from manifest
    manifest.remove(idx.unwrap());
    provider.save_manifest(&instance_path, &manifest).await.map_err(axum::response::IntoResponse::into_response)?;

    // Emit event (optional)

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_updates(
    State(state): State<Arc<AppState>>,
    Path(uuid): Path<String>,
    UserToken(user): UserToken,
) -> Result<Json<Vec<ModUpdateInfo>>, axum::response::Response> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&user)
        .ok_or_else(|| axum::response::IntoResponse::into_response("Unauthorized"))?;
    requester.try_action(
        &UserAction::ViewInstance(uuid.clone()),
        state.global_settings.lock().await.safe_mode(),
    )?;
    let instances = &state.instances;
    let instance = instances
        .get(&InstanceUuid(uuid.clone()))
        .ok_or_else(|| axum::response::IntoResponse::into_response("Instance not found"))?;
    let (flavour, game_version) = (instance.flavour().await, instance.game_version().await);
    let (loader, _install_dir) = loader_and_dir_from_flavour(&flavour.to_string());
    let instance_path = instance.instance_path().await;
    let provider = ModManager::new_modrinth();
    let manifest = provider.load_manifest(&instance_path).await.unwrap_or_default();

    let mut updates = vec![];
    for entry in &manifest {
        let versions = provider
            .provider
            .get_project_versions(&entry.project_id)
            .await
            .unwrap_or_default();
        let newest = versions
            .iter()
            .filter(|v| v.loaders.iter().any(|l| l == loader) && v.game_versions.iter().any(|g| g == &game_version))
            .max_by_key(|v| v.date_published.clone());
        let has_update = match newest {
            Some(newest) => newest.id != entry.version_id,
            None => false,
        };
        updates.push(ModUpdateInfo {
            project_id: entry.project_id.clone(),
            current_version_id: entry.version_id.clone(),
            latest_version_id: newest.map(|v| v.id.clone()),
            has_update,
        });
    }
    Ok(Json(updates))
}

async fn install_mod(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>,
    Json(body): Json<InstallBody>,
) -> Result<Json<Vec<InstalledMod>>, Error> {
    let uuid = InstanceUuid(uuid);
    let user = state.users_manager.read().await.try_auth(&token).ok_or(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Unauthorized") })?;
    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: color_eyre::eyre::eyre!("Instance not found") })?;
    if !user.can_write_instance_file(&uuid) {
        return Err(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Permission denied") });
    }
    let (loader, dir) = match inst {
        GameInstance::MinecraftInstance(ref mi) => {
            let cfg = mi.get_config().await;
            loader_and_dir_from_flavour(&cfg.flavour)?
        }
        _ => return Err(Error { kind: ErrorKind::BadRequest, source: color_eyre::eyre::eyre!("Not a Minecraft instance") }),
    };
    let path = inst.path().await;
    // TODO: Implement the recursive dependency download, file move, manifest update, event emission as described in the plan above.
    // For now, stub:
    Ok(Json(vec![]))
}

async fn uninstall_mod(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path((uuid, filename)): Path<(String, String)>,
) -> Result<Json<()>, Error> {
    let uuid = InstanceUuid(uuid);
    let user = state.users_manager.read().await.try_auth(&token).ok_or(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Unauthorized") })?;
    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: color_eyre::eyre::eyre!("Instance not found") })?;
    if !user.can_write_instance_file(&uuid) {
        return Err(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Permission denied") });
    }
    let (loader, dir) = match inst {
        GameInstance::MinecraftInstance(ref mi) => {
            let cfg = mi.get_config().await;
            loader_and_dir_from_flavour(&cfg.flavour)?
        }
        _ => return Err(Error { kind: ErrorKind::BadRequest, source: color_eyre::eyre::eyre!("Not a Minecraft instance") }),
    };
    let path = inst.path().await;
    // TODO: Load manifest, check for dependency, remove file and manifest entry, emit event.
    Ok(Json(()))
}

async fn list_updates(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>,
) -> Result<Json<Vec<ModUpdateInfo>>, Error> {
    let uuid = InstanceUuid(uuid);
    let user = state.users_manager.read().await.try_auth(&token).ok_or(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Unauthorized") })?;
    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: color_eyre::eyre::eyre!("Instance not found") })?;
    if !user.can_view_instance(&uuid) {
        return Err(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Permission denied") });
    }
    let (loader, _dir) = match inst {
        GameInstance::MinecraftInstance(ref mi) => {
            let cfg = mi.get_config().await;
            loader_and_dir_from_flavour(&cfg.flavour)?
        }
        _ => return Err(Error { kind: ErrorKind::BadRequest, source: color_eyre::eyre::eyre!("Not a Minecraft instance") }),
    };
    let path = inst.path().await;
    // TODO: Load manifest, query ModrinthProvider for updates, return update info.
    Ok(Json(vec![]))
}