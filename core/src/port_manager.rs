use std::{collections::HashSet, net::SocketAddrV4};

use color_eyre::eyre::{eyre, Context};
use serde::{Deserialize, Serialize};

use crate::error::Error;

pub struct PortManager {
    allocated_ports: HashSet<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct PortStatus {
    pub is_in_use: bool,
    pub is_allocated: bool,
}

impl PortManager {
    pub fn new(allocated_ports: HashSet<u32>) -> PortManager {
        PortManager { allocated_ports }
    }
    #[allow(dead_code)]
    pub fn allocate(&mut self, start_port: u32) -> u32 {
        if self.allocated_ports.contains(&start_port) {
            let mut new_port = start_port + 1;
            while self.allocated_ports.contains(&new_port)
                || !port_scanner::local_port_available(new_port as u16)
            {
                new_port += 1;
            }
            self.allocated_ports.insert(new_port);
            new_port
        } else {
            self.allocated_ports.insert(start_port);
            start_port
        }
    }

    pub fn port_status(&self, port: u32) -> PortStatus {
        PortStatus {
            is_in_use: !port_scanner::local_port_available(port as u16),
            is_allocated: self.allocated_ports.contains(&port),
        }
    }

    pub fn add_port(&mut self, port: u32) {
        self.allocated_ports.insert(port);
    }

    pub fn deallocate(&mut self, port: u32) {
        self.allocated_ports.remove(&port);
    }

    pub async fn open_port(&self, port: u16) -> Result<(), Error> {
        tokio::task::spawn_blocking(move || {
            if let Ok(local_ip) = local_ip_address::local_ip() {
                // convert local_ip to a SocketAddrV4
                let local_ip = if let std::net::IpAddr::V4(ipv4) = local_ip {
                    std::net::SocketAddrV4::new(ipv4, port)
                } else {
                    panic!();
                };

                let gateway = igd::aio::search_gateway(Default::default());
                // igd 0.14 switched add_port signature and is now async via aio
                // We'll use a synchronous version for now for compatibility
                let gateway = igd::search_gateway(Default::default())
                    .context("Could not find gateway")?;
                gateway
                    .add_port(
                        igd::PortMappingProtocol::TCP,
                        port,
                        local_ip.ip().clone(),
                        port,
                        0,
                        "Port opened by Lodestone",
                    )
                    .context("Could not open port")?;
                Ok(())
            } else {
                Err(eyre!("Could not find local ip address").into())
            }
        })
        .await
        .unwrap()
    }

    pub async fn open_tcp(&self, port: u16, desc: &str, lease_seconds: u32) -> Result<(), Error> {
        use igd::{PortMappingProtocol, search_gateway};
        use std::thread;
        use std::time::Duration;

        let port = port;
        let desc = desc.to_string();
        let lease = lease_seconds;
        tokio::task::spawn_blocking(move || {
            let mut last_err = None;
            for _ in 0..3 {
                match local_ip_address::local_ip() {
                    Ok(std::net::IpAddr::V4(ipv4)) => {
                        match search_gateway(Default::default()) {
                            Ok(gateway) => {
                                let res = gateway.add_port(
                                    PortMappingProtocol::TCP,
                                    port,
                                    ipv4,
                                    port,
                                    lease,
                                    &desc,
                                );
                                match res {
                                    Ok(_) => return Ok(()),
                                    Err(e) => last_err = Some(e),
                                }
                            }
                            Err(e) => last_err = Some(e),
                        }
                    }
                    _ => last_err = Some(igd::AddPortError::RequestError),
                }
                thread::sleep(Duration::from_millis(350));
            }
            Err(eyre!("Failed to open TCP port: {:?}", last_err).into())
        }).await?
    }

    pub async fn open_udp(&self, port: u16, desc: &str, lease_seconds: u32) -> Result<(), Error> {
        use igd::{PortMappingProtocol, search_gateway};
        use std::thread;
        use std::time::Duration;

        let port = port;
        let desc = desc.to_string();
        let lease = lease_seconds;
        tokio::task::spawn_blocking(move || {
            let mut last_err = None;
            for _ in 0..3 {
                match local_ip_address::local_ip() {
                    Ok(std::net::IpAddr::V4(ipv4)) => {
                        match search_gateway(Default::default()) {
                            Ok(gateway) => {
                                let res = gateway.add_port(
                                    PortMappingProtocol::UDP,
                                    port,
                                    ipv4,
                                    port,
                                    lease,
                                    &desc,
                                );
                                match res {
                                    Ok(_) => return Ok(()),
                                    Err(e) => last_err = Some(e),
                                }
                            }
                            Err(e) => last_err = Some(e),
                        }
                    }
                    _ => last_err = Some(igd::AddPortError::RequestError),
                }
                thread::sleep(Duration::from_millis(350));
            }
            Err(eyre!("Failed to open UDP port: {:?}", last_err).into())
        }).await?
    }

    pub async fn close_tcp(&self, port: u16) -> Result<(), Error> {
        use igd::{PortMappingProtocol, search_gateway};
        tokio::task::spawn_blocking(move || {
            let gateway = search_gateway(Default::default()).context("Could not find gateway")?;
            gateway.remove_port(PortMappingProtocol::TCP, port).context("Failed to close TCP port")?;
            Ok(())
        }).await?
    }

    pub async fn close_udp(&self, port: u16) -> Result<(), Error> {
        use igd::{PortMappingProtocol, search_gateway};
        tokio::task::spawn_blocking(move || {
            let gateway = search_gateway(Default::default()).context("Could not find gateway")?;
            gateway.remove_port(PortMappingProtocol::UDP, port).context("Failed to close UDP port")?;
            Ok(())
        }).await?
    }

    pub async fn external_ip(&self) -> Result<std::net::IpAddr, Error> {
        use igd::search_gateway;
        tokio::task::spawn_blocking(move || {
            let gateway = search_gateway(Default::default()).context("Could not find gateway")?;
            let ip = gateway.get_external_ip().context("Failed to get external IP")?;
            Ok(ip)
        }).await?
    }
}
