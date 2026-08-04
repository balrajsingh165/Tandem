//! tandem-daemon entry point: parses CLI flags, loads config, initializes
//! logging, and runs the app supervisor until shutdown signal. No logic beyond
//! bootstrapping app.rs.

// Subsystem surfaces are covered by tests but not yet reached from main; the
// transport and IPC serving loops that use them land with Phase 1.
#![allow(dead_code)]

mod app;
mod config;
mod ipc_service;
mod logging;
mod store;

use std::process::ExitCode;

use tandem_ipc::api::{IpcRequest, IpcResponse};
use tandem_ipc::server::IpcService;
use tandem_ipc::socket::Endpoint;

use crate::app::App;
use crate::config::Config;
use crate::ipc_service::DaemonIpcService;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let config = match Config::default().apply_args(&args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("tandem-daemon: {error}");
            return ExitCode::from(2);
        }
    };

    logging::init(config.log_level);

    let app = App::build(config);
    if !app.health().is_usable() {
        eprintln!("tandem-daemon: control plane failed to start");
        return ExitCode::FAILURE;
    }

    let endpoint = Endpoint::default_for_platform(std::env::var("XDG_RUNTIME_DIR").ok().as_deref());
    let mut service = DaemonIpcService::new(app);

    let Ok(IpcResponse::Status(status)) = service.handle(IpcRequest::Status) else {
        eprintln!("tandem-daemon: status probe failed");
        return ExitCode::FAILURE;
    };

    println!(
        "tandem-daemon ready on {} (control: up, desktop audio: {})",
        endpoint.describe(),
        if status.desktop_audio_available {
            "available"
        } else {
            "unavailable — Tier B-lite"
        }
    );

    ExitCode::SUCCESS
}
