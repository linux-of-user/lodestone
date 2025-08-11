use axum::{    extract::{Path, State, Query},
    routing::{get, put},    Json, Router,
};use axum_auth::AuthBearer;
use crate::{AppState, error::{Error, ErrorKind}};use color_eyre::eyre::eyre;
pub fn get_docker_routes(state: AppState) -> Router {
    Router::new()        .route("/docker/containers", get(list_containers))
        .route("/docker/containers/:id/start", put(start_container))        .route("/docker/containers/:id/stop", put(stop_container))
        .route("/docker/containers/:id/restart", put(restart_container))        .route("/docker/containers/:id/kill", put(kill_container))
        .route("/docker/containers/:id/logs", get(get_logs))        .with_state(state)
}
async fn list_containers(    State(state): State<AppState>,
    AuthBearer(token): AuthBearer,) -> Result<Json<Vec<crate::traits::InstanceInfo>>, Error> {
    let requester = state.users_manager.read().await.try_auth(&token).ok_or(Error {        kind: ErrorKind::Unauthorized,
        source: eyre!("Unauthorized"),    })?;
    if !requester.is_owner {        return Err(Error { kind: ErrorKind::Unauthorized, source: eyre!("Owner only") });
    }    let containers = state.docker_bridge.list_containers().await?;
    Ok(Json(containers))}
async fn start_container(
    State(state): State<AppState>,    AuthBearer(token): AuthBearer,
    Path(id): Path<String>,) -> Result<Json<()>, Error> {
    let requester = state.users_manager.read().await.try_auth(&token).ok_or(Error {        kind: ErrorKind::Unauthorized,
        source: eyre!("Unauthorized"),    })?;
    if !requester.is_owner {        return Err(Error { kind: ErrorKind::Unauthorized, source: eyre!("Owner only") });
    }    state.docker_bridge.start_container(&id.into()).await?;
    Ok(Json(()))}
async fn stop_container(
    State(state): State<AppState>,    AuthBearer(token): AuthBearer,
    Path(id): Path<String>,) -> Result<Json<()>, Error> {
    let requester = state.users_manager.read().await.try_auth(&token).ok_or(Error {        kind: ErrorKind::Unauthorized,
        source: eyre!("Unauthorized"),    })?;
    if !requester.is_owner {        return Err(Error { kind: ErrorKind::Unauthorized, source: eyre!("Owner only") });
    }    state.docker_bridge.stop_container(&id.into()).await?;
    Ok(Json(()))}
async fn restart_container(
    State(state): State<AppState>,    AuthBearer(token): AuthBearer,
    Path(id): Path<String>,) -> Result<Json<()>, Error> {
    let requester = state.users_manager.read().await.try_auth(&token).ok_or(Error {        kind: ErrorKind::Unauthorized,
        source: eyre!("Unauthorized"),    })?;
    if !requester.is_owner {        return Err(Error { kind: ErrorKind::Unauthorized, source: eyre!("Owner only") });
    }    state.docker_bridge.restart_container(&id.into()).await?;
    Ok(Json(()))}
async fn kill_container(
    State(state): State<AppState>,    AuthBearer(token): AuthBearer,
    Path(id): Path<String>,) -> Result<Json<()>, Error> {
    let requester = state.users_manager.read().await.try_auth(&token).ok_or(Error {        kind: ErrorKind::Unauthorized,
        source: eyre!("Unauthorized"),    })?;
    if !requester.is_owner {        return Err(Error { kind: ErrorKind::Unauthorized, source: eyre!("Owner only") });
    }    state.docker_bridge.kill_container(&id.into()).await?;
    Ok(Json(()))}
#[derive(serde::Deserialize)]
pub struct LogsQuery {    tail: Option<u64>,
}
async fn get_logs(    State(_state): State<AppState>,
    Path(_id): Path<String>,    Query