use axum::{
    extract::State,
    Json,
    routing::{get, post},
    Router,
};
use axum_auth::AuthBearer;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{
    AppState,
    error::{Error, ErrorKind},
    playitgg,
};
use color_eyre::eyre::eyre;

pub fn get_playitgg_routes(state: AppState) -> Router {
    Router::new()
        .route("/playitgg/generate_signup_link", get(generate_signup_link))
        .route("/playitgg/start_cli", post(start_cli_handler))
        .route("/playitgg/stop_cli", post(stop_cli_handler))
        .route("/playitgg/verify_key", post(verify_key))
        .route("/playitgg/cli_is_running", get(cli_is_running))
        .route("/playitgg/get_tunnels", get(get_tunnels))
        .with_state(state)
}

async fn generate_signup_link(State(state): State<AppState>) -> Result<Json<String>, Error> {
    playitgg::generate_signup_link(state).await
}

async fn start_cli_handler(State(state): State<AppState>) -> Result<Json<()>, Error> {
    // Only allow one runner
    if let Some(flag) = &state.playit_keep_running {
        if flag.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(Json(()));
        }
    }
    playitgg::start_cli(Arc::new(Mutex::new(state))).await?;
    Ok(Json(()))
}

async fn stop_cli_handler(State(state): State<AppState>) -> Result<Json<()>, Error> {
    playitgg::stop_cli(Arc::new(Mutex::new(state))).await?;
    Ok(Json(()))
}

async fn verify_key(State(state): State<AppState>) -> Result<Json<bool>, Error> {
    let key = state.playitgg_key.lock().await.clone();
    let valid = match key {
        Some(ref k) => playitgg::is_valid_secret_key(k.clone()).await,
        None => false,
    };
    Ok(Json(valid))
}

async fn cli_is_running(State(state): State<AppState>) -> Result<Json<bool>, Error> {
    Ok(Json(state.playit_keep_running.as_ref().map_or(false, |f| f.load(std::sync::atomic::Ordering::SeqCst))))
}

async fn get_tunnels(State(state): State<AppState>) -> Result<Json<Vec<crate::playitgg::TunnelInfo>>, Error> {
    let key = state.playitgg_key.lock().await.clone();
    let api_url = crate::playitgg::PLAYIT_API_BASE;
    match key {
        Some(ref k) => {
            let client = crate::playitgg::PlayitApiClient::new(api_url.to_string(), k.clone());
            let tunnels = client.get_tunnels().await.map_err(|e| Error {
                kind: ErrorKind::Internal,
                source: eyre!("Failed to get tunnels from playit.gg: {}", e),
            })?;
            Ok(Json(tunnels))
        }
        None => Err(Error { kind: ErrorKind::BadRequest, source: eyre!("No playit.gg key configured") }),
    }
}
