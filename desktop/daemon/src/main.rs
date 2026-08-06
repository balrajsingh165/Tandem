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
mod session_loop;
mod store;

use std::process::ExitCode;

use tandem_crypto::{FileSecretStore, IdentityCredentials};
use tandem_ipc::server::{EventPublisher, IpcServer};
use tandem_ipc::socket::Endpoint;

use crate::app::App;
use crate::config::Config;
use crate::ipc_service::{DaemonIpcService, LinkState, PairingLauncher, SharedApp, SharedLink};
use crate::session_loop::PhoneEndpoint;

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

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("tandem-daemon: could not start the async runtime: {error}");
            return ExitCode::FAILURE;
        }
    };

    runtime.block_on(run(config))
}

async fn run(config: Config) -> ExitCode {
    let identity = match load_identity(&config) {
        Ok(identity) => identity,
        Err(error) => {
            eprintln!("tandem-daemon: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut app = App::build(config.clone());
    if !app.health().is_usable() {
        eprintln!("tandem-daemon: control plane failed to start");
        return ExitCode::FAILURE;
    }

    // A previous pairing is what makes the daemon useful on start; without it
    // there is nothing to connect to until the UI runs pairing.
    let state_path = state_file(&config);
    *app.store() = crate::store::Store::load(&state_path);

    let desktop_audio = app.desktop_audio_available();
    let endpoint = ipc_endpoint(&config);

    let shared_app: SharedApp = std::sync::Arc::new(std::sync::Mutex::new(app));
    let link: SharedLink = std::sync::Arc::new(std::sync::Mutex::new(LinkState::default()));
    let events = EventPublisher::new();
    let sessions: crate::ipc_service::SessionTasks = Default::default();
    let commands: crate::ipc_service::CommandBuses = Default::default();
    let server = IpcServer::new(
        DaemonIpcService::new(shared_app.clone(), link.clone(), commands.clone())
            .with_pairing(PairingLauncher {
                credentials: identity.clone(),
                events: events.clone(),
                app: shared_app.clone(),
                link: link.clone(),
                sessions: sessions.clone(),
                commands: commands.clone(),
                state_path: state_path.clone(),
                phone_port: config.phone_port,
            }),
        events.clone(),
    );

    // One supervisor per paired phone: both stay connected, and the UI chooses
    // which one a command goes to. Until any phone is paired the daemon still
    // serves the UI so pairing can be started from it.
    let mut endpoints = paired_phones(&config, &shared_app);
    if endpoints.is_empty() {
        endpoints.extend(configured_phone(&config));
    }

    for phone in endpoints {
        let phone_id = phone.device_id.clone();
        let bus = {
            let mut guard = commands.lock().expect("bus mutex poisoned");
            guard
                .entry(phone_id.clone())
                .or_insert_with(|| session_loop::CommandBus::new().0)
                .clone()
        };
        link.lock()
            .expect("link mutex poisoned")
            .ensure_selected(&phone_id);

        let handle = tokio::spawn(session_loop::supervise(
            phone,
            identity.clone(),
            shared_app.clone(),
            link.clone(),
            events.clone(),
            bus.reset(),
            state_path.clone(),
        ));
        sessions
            .lock()
            .expect("session mutex poisoned")
            .insert(phone_id, handle);
    }

    println!(
        "tandem-daemon ready on {} (identity {}, desktop audio: {})",
        endpoint.describe(),
        &identity.identity.device_id[..12.min(identity.identity.device_id.len())],
        if desktop_audio {
            "available"
        } else {
            "unavailable â€” Tier B-lite"
        }
    );

    // Serving runs until the process is signalled; a bind failure is fatal
    // because the UI would have nothing to talk to.
    tokio::select! {
        result = server.serve(&endpoint) => {
            if let Err(error) = result {
                eprintln!("tandem-daemon: IPC endpoint unavailable: {error}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        _ = shutdown_signal() => {
            println!("tandem-daemon: shutting down");
            ExitCode::SUCCESS
        }
    }
}

/// Loads or creates this desktop's identity. Without it there is nothing to
/// present in a TLS handshake, so pairing and every session would fail later
/// with a confusing error.
fn load_identity(config: &Config) -> Result<IdentityCredentials, String> {
    let store = FileSecretStore::new(secrets_directory(config));
    tandem_crypto::identity::load_or_create(&store, &config.desktop_display_name)
        .map_err(|e| format!("could not open the device identity: {e}"))
}

fn secrets_directory(config: &Config) -> std::path::PathBuf {
    config
        .state_directory
        .clone()
        .unwrap_or_else(default_state_directory)
        .join("secrets")
}

fn default_state_directory() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("tandem")
}

fn ipc_endpoint(config: &Config) -> Endpoint {
    match &config.ipc_socket_override {
        Some(path) if cfg!(windows) => Endpoint::WindowsPipe(path.clone()),
        Some(path) => Endpoint::UnixSocket(std::path::PathBuf::from(path)),
        None => Endpoint::default_for_platform(std::env::var("XDG_RUNTIME_DIR").ok().as_deref()),
    }
}

fn state_file(config: &Config) -> std::path::PathBuf {
    config
        .state_directory
        .clone()
        .unwrap_or_else(default_state_directory)
        .join("tandem-cache.json")
}

/// The phone this desktop paired with previously. Its host still comes from
/// configuration until mDNS discovery is wired, but the pin comes from the
/// persisted pairing rather than the command line.
fn paired_phones(config: &Config, app: &SharedApp) -> Vec<PhoneEndpoint> {
    let stored = match app.lock() {
        Ok(guard) => guard.store_ref().phones().to_vec(),
        Err(_) => return Vec::new(),
    };

    stored
        .into_iter()
        .filter_map(|phone| {
            let fingerprint =
                tandem_crypto::SpkiFingerprint::from_base64url(&phone.spki_sha256).ok()?;
            Some(PhoneEndpoint {
                device_id: phone.device_id,
                // A configured host only makes sense for a single phone; with more
                // than one, discovery is the only way to tell them apart.
                host: config.phone_host.clone().unwrap_or_default(),
                port: config.phone_port,
                pin: tandem_transport::tls::PinSource::Paired(fingerprint),
            })
        })
        .collect()
}

/// A phone endpoint supplied entirely on the command line, for bring-up before a
/// pairing exists.
fn configured_phone(config: &Config) -> Option<PhoneEndpoint> {
    let host = config.phone_host.clone()?;
    let pin = config.phone_pin.as_ref()?;
    let fingerprint = tandem_crypto::SpkiFingerprint::from_base64url(pin).ok()?;

    Some(PhoneEndpoint {
        device_id: String::new(),
        host,
        port: config.phone_port,
        pin: tandem_transport::tls::PinSource::Paired(fingerprint),
    })
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
