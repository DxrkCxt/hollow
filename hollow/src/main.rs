// SPDX-License-Identifier: GPL-3.0-only

mod cli;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use hollow::configuration::{LimboConfig, Overlay, env_overlays};
use hollow::server::{limbo_server, log};

fn main() {
    let action = match cli::parse(std::env::args().collect()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {}\n", e.0);
            print!("{}", cli::HELP);
            std::process::exit(2);
        }
    };

    match action {
        cli::Action::PrintHelp => print!("{}", cli::HELP),
        cli::Action::PrintVersion => println!("hollow {}", env!("CARGO_PKG_VERSION")),
        cli::Action::Healthcheck {
            config_path,
            overlays,
        } => std::process::exit(healthcheck(&config_path, merged(overlays))),
        cli::Action::Run {
            config_path,
            overlays,
        } => run(&config_path, merged(overlays)),
    }
}

/// Layer CLI overlays on top of the environment overlays (CLI applied last → wins).
fn merged(cli_overlays: Vec<Overlay>) -> Vec<Overlay> {
    let mut all = env_overlays();
    all.extend(cli_overlays);
    all
}

fn run(config_path: &Path, overlays: Vec<Overlay>) {
    let config = match LimboConfig::load_from(config_path, &overlays) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Cannot start server: {e:?}");
            return;
        }
    };

    let workers = config.worker_group_size.max(1) as usize;
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Cannot start server: {e}");
            return;
        }
    };

    runtime.block_on(async {
        if let Err(e) = limbo_server::run(config).await {
            log::error(format!("Cannot start server: {e}"));
        }
    });
}

/// Probe the configured bind port over TCP. Returns a process exit code: 0 if the
/// server is accepting connections, 1 otherwise. Reads the same config/env as `run`
/// but never writes a default settings file (safe to run against a mounted volume).
fn healthcheck(config_path: &Path, overlays: Vec<Overlay>) -> i32 {
    let config = match LimboConfig::load_readonly(config_path, &overlays) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("healthcheck: config error: {e:?}");
            return 1;
        }
    };

    // A wildcard bind (0.0.0.0 / ::) isn't connectable; probe the matching loopback.
    let addr = config.address;
    let target = if addr.ip().is_unspecified() {
        let loopback = match addr.ip() {
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
        };
        SocketAddr::new(loopback, addr.port())
    } else {
        addr
    };

    match TcpStream::connect_timeout(&target, Duration::from_secs(3)) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("healthcheck: cannot connect to {target}: {e}");
            1
        }
    }
}
