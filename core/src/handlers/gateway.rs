use axum::{extract::Path, routing::put, Json, Router};
use axum_auth::AuthBearer;

use color_eyre::eyre::eyre;

use crate::{
    error::{Error, ErrorKind},
    AppState,
};

pub async fn open_port(
    axum::extract::State(state): axum::extract::State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(port): Path<u16>,
) -> Result<Json<()>, crate::error::Error> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&token)
        .ok_or_else(|| crate::error::Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Token error"),
        })?;
    if !requester.is_owner {
        return Err(crate::error::Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Only owners can open ports"),
        });
    }

    state.port_manager.lock().await.open_port(port).await?;
    Ok(Json(()))
}

pub async fn open_tcp(
    axum::extract::State(state): axum::extract::State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(port): Path<u16>,
) -> Result<Json<()>, Error> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&token)
        .ok_or_else(|| Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Token error"),
        })?;
    if !requester.is_owner {
        return Err(Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Only owners can open ports"),
        });
    }
    state.port_manager.lock().await.open_tcp(port, "Lodestone TCP", 0).await?;
    Ok(Json(()))
}

pub async fn open_udp(
    axum::extract::State(state): axum::extract::State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(port): Path<u16>,
) -> Result<Json<()>, Error> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&token)
        .ok_or_else(|| Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Token error"),
        })?;
    if !requester.is_owner {
        return Err(Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Only owners can open ports"),
        });
    }
    state.port_manager.lock().await.open_udp(port, "Lodestone UDP", 0).await?;
    Ok(Json(()))
}

pub async fn close_tcp(
    axum::extract::State(state): axum::extract::State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(port): Path<u16>,
) -> Result<Json<()>, Error> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&token)
        .ok_or_else(|| Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Token error"),
        })?;
    if !requester.is_owner {
        return Err(Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Only owners can close ports"),
        });
    }
    state.port_manager.lock().await.close_tcp(port).await?;
    Ok(Json(()))
}

pub async fn close_udp(
    axum::extract::State(state): axum::extract::State<AppState>,
    AuthBearer(token): AuthBearer,
    Path(port): Path<u16>,
) -> Result<Json<()>, Error> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&token)
        .ok_or_else(|| Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Token error"),
        })?;
    if !requester.is_owner {
        return Err(Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Only owners can close ports"),
        });
    }
    state.port_manager.lock().await.close_udp(port).await?;
    Ok(Json(()))
}

pub async fn external_ip(
    axum::extract::State(state): axum::extract::State<AppState>,
    AuthBearer(token): AuthBearer,
) -> Result<Json<std::net::IpAddr>, Error> {
    let requester = state
        .users_manager
        .read()
        .await
        .try_auth(&token)
        .ok_or_else(|| Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Token error"),
        })?;
    if !requester.is_owner {
        return Err(Error {
            kind: ErrorKind::Unauthorized,
            source: eyre!("Only owners can get external IP"),
        });
    }
    let ip = state.port_manager.lock().await.external_ip().await?;
    Ok(Json(ip))
}

pub fn get_gateway_routes(state: AppState) -> Router {
    Router::new()
        .route("/gateway/open_port/:port", put(open_port))
        .route("/gateway/open/tcp/:port", put(open_tcp))
        .route("/gateway/open/udp/:port", put(open_udp))
        .route("/gateway/close/tcp/:port", put(close_tcp))
        .route("/gateway/close/udp/:port", put(close_udp))
        .route("/gateway/external_ip", get(external_ip))
        .with_state(state)
}
