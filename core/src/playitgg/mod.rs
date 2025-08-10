/*
Copyright 2022 Developed Methods LLC

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/

pub mod helper;
pub mod tcp_client;
pub mod utils;

mod playit_secret;
use playit_agent::client::{PlayitApiClient, PlayitClient, PlayitConnectionConfig};
use playit_agent_protocol::api_types::TunnelInfo;
use playit_secret::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

const PLAYIT_API_BASE: &str = "https://api.playit.gg";
const PLAYIT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn is_running_flag(flag: &Arc<AtomicBool>) -> bool {
    flag.load(Ordering::SeqCst)
}

pub async fn start_cli(state: Arc<Mutex<crate::AppState>>) -> Result<(), crate::error::Error> {
    let mut state = state.lock().await;
    if let Some(running_flag) = &state.playit_keep_running {
        if is_running_flag(running_flag) {
            return Ok(());
        }
    }
    let running_flag = Arc::new(AtomicBool::new(true));
    state.playit_keep_running = Some(running_flag.clone());

    let playitgg_key = state.playitgg_key.lock().await.clone();
    let event_broadcaster = state.event_broadcaster.clone();

    tokio::spawn(async move {
        event_broadcaster.send(crate::events::PlayitggRunnerEvent::loading());
        let mut backoff = 1;
        loop {
            if !is_running_flag(&running_flag) {
                event_broadcaster.send(crate::events::PlayitggRunnerEvent::stopped());
                break;
            }
            if let Some(ref key) = playitgg_key {
                let config = PlayitConnectionConfig {
                    api_url: PLAYIT_API_BASE.to_string(),
                    secret_key: key.clone(),
                };
                match PlayitClient::connect(config).await {
                    Ok(mut client) => {
                        event_broadcaster.send(crate::events::PlayitggRunnerEvent::started());
                        while is_running_flag(&running_flag) {
                            if let Err(e) = client.poll().await {
                                event_broadcaster.send(crate::events::PlayitggRunnerEvent::stopped_with_error(e.to_string()));
                                break;
                            }
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                    }
                    Err(e) => {
                        event_broadcaster.send(crate::events::PlayitggRunnerEvent::stopped_with_error(format!("Playit connect failed: {}", e)));
                        tokio::time::sleep(tokio::time::Duration::from_secs(backoff)).await;
                        backoff = (backoff * 2).min(60);
                    }
                }
            } else {
                event_broadcaster.send(crate::events::PlayitggRunnerEvent::stopped_with_error("No playitgg key found".into()));
                break;
            }
        }
    });

    Ok(())
}

pub async fn stop_cli(state: Arc<Mutex<crate::AppState>>) -> Result<(), crate::error::Error> {
    let mut state = state.lock().await;
    if let Some(flag) = &state.playit_keep_running {
        flag.store(false, Ordering::SeqCst);
    }
    Ok(())
}
    let response = api
        .tunnels_list_json(ReqTunnelsList {
            tunnel_id: None,
            agent_id: None,
        })
        .await;
    if let Ok(response) = response {
        let tunnels_value = response.get("tunnels");
        if let Some(tunnels_value) = tunnels_value {
            let tunnels = tunnels_value.as_array();
            if let Some(tunnels) = tunnels {
                let mut res: Vec<PlayitTunnelInfo> = vec![];
                for tunnel in tunnels {
                    let id_value = tunnel.get("id");
                    let name_value = tunnel.get("name");
                    let active_value = tunnel.get("active");

                    if !((id_value.is_some() && id_value.unwrap().as_str().is_some())
                        && (name_value.is_some() && name_value.unwrap().as_str().is_some())
                        && (active_value.is_some() && active_value.unwrap().as_bool().is_some()))
                    {
                        return Err(Error {
                            kind: ErrorKind::Internal,
                            source: eyre!("Got malformed response from Playit"),
                        });
                    }

                    let id = id_value.unwrap().as_str().unwrap().to_string();
                    let name = name_value.unwrap().as_str().unwrap().to_string();
                    let active = active_value.unwrap().as_bool().unwrap();

                    if !((tunnel.get("alloc").is_some()
                        && tunnel.get("alloc").unwrap().get("data").is_some())
                        && (tunnel.get("origin").is_some()
                            && tunnel.get("origin").unwrap().get("data").is_some()))
                    {
                        return Err(Error {
                            kind: ErrorKind::Internal,
                            source: eyre!("Got malformed response from Playit"),
                        });
                    }

                    let alloc_data = tunnel.get("alloc").unwrap().get("data").unwrap();
                    let origin_data = tunnel.get("origin").unwrap().get("data").unwrap();

                    let local_port_value = origin_data.get("local_port");
                    let local_ip_value = origin_data.get("local_ip");
                    let assigned_domain_value = alloc_data.get("assigned_domain");
                    let assigned_port_value = alloc_data.get("port_start");

                    if !(local_port_value.is_some()
                        && local_ip_value.is_some()
                        && assigned_domain_value.is_some())
                    {
                        return Err(Error {
                            kind: ErrorKind::Internal,
                            source: eyre!("Got malformed response from Playit"),
                        });
                    }

                    let local_port = local_port_value.unwrap().as_i64();
                    let local_ip = local_ip_value.unwrap().as_str();
                    let assigned_domain = assigned_domain_value.unwrap().as_str();
                    let assigned_port = assigned_port_value.unwrap().as_i64();

                    if !(local_port.is_some() && local_ip.is_some() && assigned_domain.is_some()) {
                        return Err(Error {
                            kind: ErrorKind::Internal,
                            source: eyre!("Got malformed response from Playit"),
                        });
                    }

                    res.push(PlayitTunnelInfo {
                        local_ip: local_ip.unwrap().to_string(),
                        local_port: local_port.unwrap() as u16,
                        name,
                        tunnel_id: TunnelUuid(id),
                        active,
                        server_address: format!(
                            "{}:{}",
                            assigned_domain.unwrap(),
                            assigned_port.unwrap()
                        ),
                    });
                }
                Ok(Json(res))
            } else {
                Err(Error {
                    kind: ErrorKind::Internal,
                    source: eyre!("Got malformed response from Playit"),
                })
            }
        } else {
            Err(Error {
                kind: ErrorKind::Internal,
                source: eyre!("Got malformed response from Playit"),
            })
        }
    } else {
        Err(Error {
            kind: ErrorKind::Internal,
            source: eyre!("Couldn't connect to Playit"),
        })
    }
}
