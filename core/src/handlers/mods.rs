use axum::{
    extract::{Path, State, Query},
    Json, Router,
};
use axum_auth::AuthBearer;
use std::path::{Path as StdPath, PathBuf};
use serde::Deserialize;

use crate::{
    AppState,
    error::{Error, ErrorKind},
    mods::{modrinth::ModrinthProvider, types::*, manifest::*},
    types::InstanceUuid,
    prelude::GameInstance,
};

#[derive(Deserialize)]
pub struct SearchParams {
    query: String,
    loader: Option<String>,
    game_version: Option<String>,
}

#[derive(Deserialize)]
pub struct InstallBody {
    project_id: String,
    version_id: Option<String>,
}

fn loader_and_dir_from_flavour(flavour: &crate::implementations::minecraft::Flavour) -> Result<(&'static str, &'static str), Error> {
    use crate::implementations::minecraft::Flavour::*;
    match flavour {
        Fabric { .. } => Ok(("fabric", "mods")),
        Forge { .. } => Ok(("forge", "mods")),
        Quilt { .. } => Ok(("quilt", "mods")),
        Paper { .. } => Ok(("paper", "plugins")),
        Purpur { .. } => Ok(("purpur", "plugins")),
        Spigot => Ok(("spigot", "plugins")),
        Vanilla => Err(Error { kind: ErrorKind::BadRequest, source: color_eyre::eyre::eyre!("Vanilla does not support mods/plugins") }),
    }
}

pub fn get_mods_routes(state: AppState) -> Router {
    Router::new()
        .route("/mods/search", axum::routing::get(search_mods))
        .route("/mods/project/:id", axum::routing::get(get_project))
        .route("/mods/project/:id/versions", axum::routing::get(get_versions))
        .route("/instance/:uuid/mods", axum::routing::get(list_installed))
        .route("/instance/:uuid/mods/install", axum::routing::post(install_mod))
        .route("/instance/:uuid/mods/:filename", axum::routing::delete(uninstall_mod))
        .route("/instance/:uuid/mods/updates", axum::routing::get(list_updates))
        .with_state(state)
}

async fn search_mods(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Query(query): Query<SearchParams>,
) -> Result<Json<Vec<ModProject>>, Error> {
    // TODO: Add permission check for global mod browsing if needed
    let provider = ModrinthProvider::new();
    let results = provider
        .search(&query.query, query.loader.as_deref(), query.game_version.as_deref())
        .await?;
    Ok(Json(results))
}

async fn get_project(
    State(_state): State<AppState>,
    AuthBearer(_token): AuthBearer,
    Path(id): Path<String>,
) -> Result<Json<ModProject>, Error> {
    let provider = ModrinthProvider::new();
    let project = provider.get_project(&id).await?;
    Ok(Json(project))
}

async fn get_versions(
    State(_state): State<AppState>,
    AuthBearer(_token): AuthBearer,
    Path(id): Path<String>,
    Query(query): Query<SearchParams>,
) -> Result<Json<Vec<ModVersion>>, Error> {
    let provider = ModrinthProvider::new();
    let versions = provider.get_versions(&id, query.loader.as_deref(), query.game_version.as_deref()).await?;
    Ok(Json(versions))
}

async fn list_installed(
    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(uuid): Path<String>,
) -> Result<Json<Vec<InstalledMod>>, Error> {
    let uuid = InstanceUuid(uuid);
    let user = state.users_manager.read().await.try_auth(&token).ok_or(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Unauthorized") })?;
    let inst = state.instances.get(&uuid).ok_or(Error { kind: ErrorKind::NotFound, source: color_eyre::eyre::eyre!("Instance not found") })?;
    if !user.can_view_instance(&uuid) {
        return Err(Error { kind: ErrorKind::Unauthorized, source: color_eyre::eyre::eyre!("Permission denied") });
    }
    let path = inst.path().await;
    Ok(Json(load_manifest(&path).await?))
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