// SPDX-License-Identifier: GPL-3.0-only

//! Server bootstrap: tokio runtime, accept loop, keep-alive ticker, graceful shutdown.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpSocket};
use tokio::sync::Notify;

use crate::configuration::LimboConfig;
use crate::connection::PacketSnapshots;
use crate::connection::pipeline;
use hollow_data::TransportType;
use crate::server::{Connections, command_manager, log};
use hollow_world::DimensionRegistry;

pub struct LimboServer {
    pub config: LimboConfig,
    pub dimension_registry: DimensionRegistry,
    pub snapshots: PacketSnapshots,
    pub connections: Connections,
    pub shutdown: Notify,
}

impl LimboServer {
    pub fn request_shutdown(&self) {
        self.shutdown.notify_one();
    }
}

pub async fn run(config: LimboConfig) -> std::io::Result<()> {
    log::set_level(config.debug_level);
    log::info("Starting server...");

    let dimension_registry = DimensionRegistry::load();
    let snapshots = PacketSnapshots::init(&config, &dimension_registry);
    let connections = Connections::new(config.log_players_ip);

    let mut transport = config.transport_type;
    if !transport.is_available() {
        log::debug(format!(
            "Transport type {} is not available! Using NIO.",
            transport.name()
        ));
        transport = TransportType::Nio;
    }
    log::debug(format!("Using {} transport type", transport.name()));

    let address = config.address;
    let server = Arc::new(LimboServer {
        config,
        dimension_registry,
        snapshots,
        connections,
        shutdown: Notify::new(),
    });

    let listener = bind_listener(address)?;

    // Keep-alive broadcast every 5 seconds.
    {
        let server = server.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                for shared in server.connections.all_shared() {
                    shared.send_keep_alive();
                }
            }
        });
    }

    log::info(format!("Server started on {address}"));

    command_manager::start(server.clone());

    // Build the shutdown future once; recreating it inside the accept loop would
    // re-register OS signal handlers on every accepted connection.
    let shutdown = shutdown_signal(&server);
    tokio::pin!(shutdown);

    let mut next_id: u64 = 0;
    loop {
        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, peer)) => {
                        next_id += 1;
                        let id = next_id;
                        let server = server.clone();
                        tokio::spawn(pipeline::handle_connection(stream, peer.to_string(), id, server));
                    }
                    Err(e) => log::error(format!("Accept error: {e}")),
                }
            }
            _ = &mut shutdown => {
                log::info("Stopping server...");
                break;
            }
        }
    }

    log::info("Server stopped, Goodbye!");
    Ok(())
}

/// Bind a listener with an enlarged accept backlog. A bigger backlog lets bursts of
/// simultaneous connections (join/bot floods) queue in the kernel instead of having
/// their SYNs dropped and retransmitted after the OS timeout — the dominant source
/// of connect-latency under load. (tokio's `TcpListener::bind` defaults to 1024;
/// the kernel still clamps to its own maximum, e.g. `net.core.somaxconn` on Linux.)
fn bind_listener(address: SocketAddr) -> std::io::Result<TcpListener> {
    const BACKLOG: u32 = 4096;
    let socket = if address.is_ipv6() {
        TcpSocket::new_v6()?
    } else {
        TcpSocket::new_v4()?
    };
    socket.set_reuseaddr(true)?;
    socket.bind(address)?;
    socket.listen(BACKLOG)
}

async fn shutdown_signal(server: &Arc<LimboServer>) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = term.recv() => {},
            _ = server.shutdown.notified() => {},
        }
    }
    #[cfg(not(unix))]
    {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = server.shutdown.notified() => {},
        }
    }
}
