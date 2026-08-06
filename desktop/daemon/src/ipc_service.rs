//! Implements the IpcApi surface over the live controller and subsystems:
//! translates UI method calls into controller commands and streams state events
//! to connected UIs.

use tandem_core::error::CoreError;
use tandem_ipc::api::{
    AudioRoute as IpcAudioRoute, CallState as IpcCallState, CallView, ConnectionStatus, IpcRequest,
    IpcResponse, StatusResult,
};
use tandem_ipc::error::IpcError;
use tandem_ipc::server::IpcService;

use crate::app::App;
use tandem_core::events::UserCommand;
use tandem_core::model::{AudioRoute, Call, CallState};

/// App state shared between the IPC server and the session supervisor. A
/// std mutex is deliberate: every critical section is a short, synchronous
/// mutation, so no lock is ever held across an await.
pub type SharedApp = std::sync::Arc<std::sync::Mutex<App>>;

/// Connection facts one supervisor reports for the UI to render.
#[derive(Debug, Clone, Default)]
pub struct PhoneLink {
    pub connection: Option<ConnectionStatus>,
    pub phone_name: String,
    /// Audio targets as the phone last reported them, so status and events agree.
    pub audio_devices: Vec<tandem_ipc::api::AudioDeviceView>,
    pub active_bt_device_address: String,
}

/// Link state for every paired phone, plus which one the UI is driving. Each
/// phone has its own session, so none of this can be a single global.
#[derive(Debug, Clone, Default)]
pub struct LinkState {
    phones: std::collections::HashMap<String, PhoneLink>,
    selected: String,
}

impl LinkState {
    pub fn of(&self, phone_id: &str) -> PhoneLink {
        self.phones.get(phone_id).cloned().unwrap_or_default()
    }

    pub fn entry(&mut self, phone_id: &str) -> &mut PhoneLink {
        self.phones.entry(phone_id.to_string()).or_default()
    }

    pub fn forget(&mut self, phone_id: &str) {
        self.phones.remove(phone_id);
        if self.selected == phone_id {
            self.selected = self.phones.keys().next().cloned().unwrap_or_default();
        }
    }

    pub fn selected(&self) -> &str {
        &self.selected
    }

    /// Selecting an unknown phone is ignored: the UI must never be able to point
    /// commands at something that was just unpaired.
    pub fn select(&mut self, phone_id: &str) {
        if self.phones.contains_key(phone_id) {
            self.selected = phone_id.to_string();
        }
    }

    /// The first phone to appear becomes the selection, so a single-phone setup
    /// never needs the user to choose.
    pub fn ensure_selected(&mut self, phone_id: &str) {
        self.entry(phone_id);
        if self.selected.is_empty() {
            self.selected = phone_id.to_string();
        }
    }
}

pub type SharedLink = std::sync::Arc<std::sync::Mutex<LinkState>>;

/// The running control sessions, keyed by phone, shared so that whoever pairs,
/// unpairs, or starts the daemon is all talking about the same ones.
pub type SessionTasks =
    std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>>;

/// One command queue per phone: intent for one phone must never be written to
/// another phone's socket.
pub type CommandBuses =
    std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<crate::session_loop::CommandBus>>>>;

/// What the service needs to start a pairing attempt on the user's behalf.
/// Pairing is long-running, so the request returns immediately and progress
/// arrives as `pairingProgress` events.
#[derive(Clone)]
pub struct PairingLauncher {
    pub credentials: tandem_crypto::IdentityCredentials,
    pub events: tandem_ipc::server::EventPublisher,
    pub app: SharedApp,
    pub link: SharedLink,
    pub sessions: SessionTasks,
    pub commands: CommandBuses,
    pub state_path: std::path::PathBuf,
    pub phone_port: u16,
}

/// Brings the control session up for a phone that was just paired. Without this
/// the desktop would stay offline until the next daemon restart, even though the
/// pairing succeeded.
fn start_session_for(launcher: &PairingLauncher, record: &tandem_pairing::flow::PairedPhoneRecord) {
    let phone_id = record.phone_device_id.clone();

    let bus = {
        let mut guard = launcher.commands.lock().expect("bus mutex poisoned");
        guard
            .entry(phone_id.clone())
            .or_insert_with(|| crate::session_loop::CommandBus::new().0)
            .clone()
    };

    launcher
        .link
        .lock()
        .expect("link mutex poisoned")
        .ensure_selected(&phone_id);

    let started = tokio::spawn(crate::session_loop::supervise(
        crate::session_loop::PhoneEndpoint {
            device_id: phone_id.clone(),
            host: String::new(),
            port: launcher.phone_port,
            pin: tandem_transport::tls::PinSource::Paired(record.phone_spki_sha256.clone()),
        },
        launcher.credentials.clone(),
        launcher.app.clone(),
        launcher.link.clone(),
        launcher.events.clone(),
        bus.reset(),
        launcher.state_path.clone(),
    ));

    if let Some(previous) = launcher
        .sessions
        .lock()
        .expect("session mutex poisoned")
        .insert(phone_id, started)
    {
        previous.abort();
    }
}

/// Bridges the UI-facing API to the domain, keeping every policy decision in
/// core rather than in this translation layer.
pub struct DaemonIpcService {
    app: SharedApp,
    link: SharedLink,
    commands: CommandBuses,
    pairing: Option<PairingLauncher>,
    /// The offer currently on screen. Two live offers would race for the phone's
    /// single pairing window, so showing a new code cancels the old one.
    offer_task: Option<tokio::task::JoinHandle<()>>,
    /// Set while a phone waits to be approved on this desktop; taking it is what
    /// releases the pairing exchange.
    pending_approval: PendingApproval,
}

/// The verdict slot the UI fills in and the pairing task waits on.
pub type PendingApproval = std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>;

impl DaemonIpcService {
    pub fn new(app: SharedApp, link: SharedLink, commands: CommandBuses) -> Self {
        Self {
            app,
            link,
            commands,
            pairing: None,
            offer_task: None,
            pending_approval: PendingApproval::default(),
        }
    }

    pub fn with_pairing(mut self, launcher: PairingLauncher) -> Self {
        self.pairing = Some(launcher);
        self
    }

    pub fn shared_app(&self) -> SharedApp {
        self.app.clone()
    }

    /// Parses the QR payload and runs pairing in the background, persisting the
    /// phone identity on success so the pairing survives a restart.
    fn start_pairing(&self, qr_payload: String) -> Result<IpcResponse, IpcError> {
        let launcher = self.pairing.clone().ok_or(IpcError::Internal)?;

        let invitation = tandem_pairing::QrPayload::parse(&qr_payload)
            .map_err(|e| IpcError::PairingFailed(e.to_string()))?;

        tokio::spawn(async move {
            let mut flow = tandem_pairing::flow::PairingFlow::new(invitation);
            let credentials = tandem_pairing::flow::DesktopCredentials {
                name: launcher.credentials.identity.display_name.clone(),
                platform: platform_name().to_string(),
                cert_der: launcher.credentials.identity.cert_der.clone(),
                key_der: launcher.credentials.key_der.clone(),
            };

            let events = launcher.events.clone();
            let outcome = flow
                .run(&credentials, |state| {
                    events.publish(tandem_ipc::api::IpcEvent::PairingProgress {
                        state: describe(state),
                        short_code: short_code_of(state),
                    });
                })
                .await;

            match outcome {
                Ok(record) => {
                    {
                        let mut guard = launcher.app.lock().expect("app mutex poisoned");
                        guard
                            .store()
                            .add_phone(tandem_core::model::PairedPhone {
                                device_id: record.phone_device_id.clone(),
                                name: record.phone_name.clone(),
                                spki_sha256: record.phone_spki_sha256.to_base64url(),
                                bt_address: record.phone_bt_address.clone(),
                            });
                        let _ = guard.store().save(&launcher.state_path);
                    }
                    start_session_for(&launcher, &record);
                    launcher
                        .events
                        .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                            state: "accepted".into(),
                            short_code: None,
                        });
                }
                Err(error) => {
                    launcher
                        .events
                        .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                            state: format!("failed: {error}"),
                            short_code: None,
                        });
                }
            }
        });

        Ok(IpcResponse::Ok)
    }

    /// Forgets the phone and drops the session, telling the phone first so it
    /// revokes this desktop too. Neither side is left trusting a peer the other
    /// has dropped.
    fn unpair(&mut self, phone_id: String) -> Result<IpcResponse, IpcError> {
        let launcher = self.pairing.clone().ok_or(IpcError::Internal)?;
        let phone_id = self.resolve_phone(&phone_id);
        if phone_id.is_empty() {
            return Err(IpcError::Internal);
        }

        if let Some(offer) = self.offer_task.take() {
            offer.abort();
        }

        // Best effort: an offline phone cannot be told, and the desktop still has
        // to be able to forget it. The phone drops trust when told, or on its own.
        if let Some(bus) = self.bus_for(&phone_id) {
            let _ = bus.send(tandem_core::events::OutboundRequest::Unpair);
        }

        let session = launcher
            .sessions
            .lock()
            .expect("session mutex poisoned")
            .remove(&phone_id);
        if let Some(session) = session {
            // The queued frame needs the session alive long enough to be written,
            // but the supervisor must not outlive the pairing or it would retry
            // against a phone that no longer trusts this desktop.
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                session.abort();
            });
        }

        {
            let mut guard = self.app.lock().expect("app mutex poisoned");
            guard.store().remove_phone(&phone_id);
            guard.forget_controller(&phone_id);
            let _ = guard.store().save(&launcher.state_path);
        }
        self.link
            .lock()
            .expect("link mutex poisoned")
            .forget(&phone_id);
        self.commands
            .lock()
            .expect("bus mutex poisoned")
            .remove(&phone_id);

        self.publish_phones(&launcher.events);
        Ok(IpcResponse::Ok)
    }

    /// An empty id from the UI means "whatever is selected", so every command has
    /// a target even before the user has chosen one.
    fn resolve_phone(&self, phone_id: &str) -> String {
        if !phone_id.is_empty() {
            return phone_id.to_string();
        }
        self.link
            .lock()
            .expect("link mutex poisoned")
            .selected()
            .to_string()
    }

    fn bus_for(&self, phone_id: &str) -> Option<std::sync::Arc<crate::session_loop::CommandBus>> {
        self.commands
            .lock()
            .expect("bus mutex poisoned")
            .get(phone_id)
            .cloned()
    }

    fn phone_summaries(&mut self) -> (Vec<tandem_ipc::api::PhoneSummary>, String) {
        let link = self.link.lock().expect("link mutex poisoned").clone();
        let app = self.app.lock().expect("app mutex poisoned");

        let phones: Vec<tandem_core::model::PairedPhone> = app.store_ref().phones().to_vec();
        let summaries = phones
            .iter()
            .map(|phone| {
                let state = link.of(&phone.device_id);
                tandem_ipc::api::PhoneSummary {
                    device_id: phone.device_id.clone(),
                    name: if state.phone_name.is_empty() {
                        phone.name.clone()
                    } else {
                        state.phone_name.clone()
                    },
                    connection: state.connection.unwrap_or(ConnectionStatus::Idle),
                    calls: app
                        .controller_ref(&phone.device_id)
                        .and_then(|c| c.mirror())
                        .map(|m| m.calls.iter().map(call_view).collect())
                        .unwrap_or_default(),
                }
            })
            .collect();

        (summaries, link.selected().to_string())
    }

    fn publish_phones(&mut self, events: &tandem_ipc::server::EventPublisher) {
        let (phones, selected_phone_id) = self.phone_summaries();
        events.publish(tandem_ipc::api::IpcEvent::PhonesChanged {
            phones,
            selected_phone_id,
        });
    }

    /// Mints a pairing offer, returns it for the UI to draw as a QR code, and
    /// runs the exchange in the background so the request answers immediately.
    fn start_offer(&mut self) -> Result<IpcResponse, IpcError> {
        let launcher = self.pairing.clone().ok_or(IpcError::Internal)?;

        if let Some(previous) = self.offer_task.take() {
            previous.abort();
        }

        let offer = tandem_pairing::DesktopOffer::new(
            &launcher.credentials.identity.fingerprint(),
            tandem_pairing::generate_token(),
            launcher.credentials.identity.display_name.clone(),
        );
        let payload = offer.encode();

        let credentials = tandem_pairing::flow::DesktopCredentials {
            name: launcher.credentials.identity.display_name.clone(),
            platform: platform_name().to_string(),
            cert_der: launcher.credentials.identity.cert_der.clone(),
            key_der: launcher.credentials.key_der.clone(),
        };

        let approval = self.pending_approval.clone();
        self.offer_task = Some(tokio::spawn(async move {
            let events = launcher.events.clone();
            let ask = |introduction: tandem_pairing::PhoneIntroduction| {
                let events = events.clone();
                let approval = approval.clone();
                async move {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    *approval.lock().expect("approval mutex poisoned") = Some(sender);
                    events.publish(tandem_ipc::api::IpcEvent::PairingApprovalRequested {
                        phone_name: introduction.phone_name,
                        phone_fingerprint: introduction.phone_fingerprint,
                    });
                    // A dropped sender means the offer was replaced or the UI
                    // went away, which is a refusal rather than an approval.
                    receiver.await.unwrap_or(false)
                }
            };

            let progress_events = events.clone();
            let outcome = tandem_pairing::offer::run(
                &offer,
                &credentials,
                |state| {
                    progress_events.publish(tandem_ipc::api::IpcEvent::PairingProgress {
                        state: describe_offer(state),
                        short_code: None,
                    });
                },
                ask,
            )
            .await;

            match outcome {
                Ok(record) => {
                    {
                        let mut guard = launcher.app.lock().expect("app mutex poisoned");
                        guard
                            .store()
                            .add_phone(tandem_core::model::PairedPhone {
                                device_id: record.phone_device_id.clone(),
                                name: record.phone_name.clone(),
                                spki_sha256: record.phone_spki_sha256.to_base64url(),
                                bt_address: record.phone_bt_address.clone(),
                            });
                        let _ = guard.store().save(&launcher.state_path);
                    }
                    start_session_for(&launcher, &record);
                    launcher
                        .events
                        .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                            state: "accepted".into(),
                            short_code: None,
                        });
                }
                Err(error) => launcher
                    .events
                    .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                        state: format!("failed: {error}"),
                        short_code: None,
                    }),
            }
        }));

        Ok(IpcResponse::Offer(tandem_ipc::api::OfferResult {
            payload,
            desktop_name: self
                .pairing
                .as_ref()
                .map(|p| p.credentials.identity.display_name.clone())
                .unwrap_or_default(),
        }))
    }

    /// Serves history from the local mirror rather than the phone: the UI must
    /// still render recents while the link is down (ADR-0007).
    fn history(&mut self, since_ms: i64, limit: u32) -> IpcResponse {
        let guard = self.app.lock().expect("app mutex poisoned");

        // Recents span every paired phone: the user thinks in calls, not devices.
        let matching: Vec<&tandem_core::model::CallLogRow> = guard
            .store_ref()
            .all_call_log()
            .into_iter()
            .map(|(_, row)| row)
            .filter(|row| row.started_at_ms >= since_ms)
            .collect();

        let capped = limit.max(1) as usize;
        let entries = matching
            .iter()
            .take(capped)
            .map(|row| tandem_ipc::api::HistoryEntry {
                entry_id: row.entry_id.clone(),
                number: row.number.clone(),
                display_name: row.display_name.clone(),
                started_at_ms: row.started_at_ms,
                duration_seconds: row.duration_seconds,
            })
            .collect();

        IpcResponse::History {
            entries,
            has_more: matching.len() > capped,
        }
    }

    /// Status is the selected phone's view plus the roster, so a single-phone
    /// setup reads exactly as it did before the switcher existed.
    fn status(&mut self) -> StatusResult {
        let (phones, selected_phone_id) = self.phone_summaries();

        let app = self.app.lock().expect("app mutex poisoned");
        let state = self
            .link
            .lock()
            .expect("link mutex poisoned")
            .of(&selected_phone_id);

        let desktop_audio_available = app.desktop_audio_available();
        let mirror = app
            .controller_ref(&selected_phone_id)
            .and_then(|c| c.mirror())
            .cloned();

        StatusResult {
            phones,
            selected_phone_id,
            connection: state.connection.unwrap_or(ConnectionStatus::Idle),
            phone_name: state.phone_name,
            calls: mirror
                .as_ref()
                .map(|m| m.calls.iter().map(call_view).collect())
                .unwrap_or_default(),
            audio_route: mirror
                .as_ref()
                .map(|m| audio_route(m.audio_route))
                .unwrap_or(IpcAudioRoute::Earpiece),
            microphone_muted: mirror.as_ref().map(|m| m.microphone_muted).unwrap_or(false),
            desktop_audio_available,
            audio_devices: state.audio_devices,
            active_bt_device_address: state.active_bt_device_address,
        }
    }
}

impl IpcService for DaemonIpcService {
    fn handle(&mut self, request: IpcRequest) -> Result<IpcResponse, IpcError> {
        let command = match request {
            IpcRequest::Status => return Ok(IpcResponse::Status(self.status())),
            IpcRequest::Dial { number, sim_slot } => UserCommand::Dial { number, sim_slot },
            IpcRequest::Answer { call_id } => UserCommand::Answer { call_id },
            IpcRequest::Reject { call_id } => UserCommand::Reject { call_id },
            IpcRequest::End { call_id } => UserCommand::End { call_id },
            IpcRequest::Mute { muted } => UserCommand::SetMuted { muted },
            IpcRequest::Hold { call_id } => UserCommand::Hold { call_id },
            IpcRequest::Unhold { call_id } => UserCommand::Unhold { call_id },
            IpcRequest::Merge {
                call_id,
                other_call_id,
            } => UserCommand::Merge {
                call_id,
                other_call_id,
            },
            IpcRequest::Dtmf { call_id, digits } => UserCommand::SendDtmf { call_id, digits },
            IpcRequest::AudioRoute {
                route,
                bt_device_address,
            } => {
                let audio_available = self
                    .app
                    .lock()
                    .expect("app mutex poisoned")
                    .desktop_audio_available();
                if !audio_available && matches!(route, IpcAudioRoute::Bluetooth) {
                    return Err(IpcError::AudioUnavailable);
                }
                UserCommand::RequestAudioRoute {
                    route: domain_route(route),
                    bt_device_address,
                }
            }
            IpcRequest::Pairing { qr_payload } => return self.start_pairing(qr_payload),
            IpcRequest::PairingOffer => return self.start_offer(),
            IpcRequest::Unpair { phone_id } => return self.unpair(phone_id),
            IpcRequest::SelectPhone { phone_id } => {
                self.link
                    .lock()
                    .expect("link mutex poisoned")
                    .select(&phone_id);
                if let Some(launcher) = self.pairing.clone() {
                    self.publish_phones(&launcher.events);
                }
                return Ok(IpcResponse::Ok);
            }
            IpcRequest::PairingConfirm { accept } => {
                let verdict = self
                    .pending_approval
                    .lock()
                    .expect("approval mutex poisoned")
                    .take();
                match verdict {
                    Some(sender) => {
                        let _ = sender.send(accept);
                        return Ok(IpcResponse::Ok);
                    }
                    None => return Err(IpcError::PairingFailed("nothing to approve".into())),
                }
            }

            IpcRequest::History { since_ms, limit } => return Ok(self.history(since_ms, limit)),
            IpcRequest::Contacts => {
                let guard = self.app.lock().expect("app mutex poisoned");
                let entries = guard
                    .store_ref()
                    .all_contacts()
                    .into_iter()
                    .map(|row| tandem_ipc::api::ContactView {
                        contact_id: row.contact_id.clone(),
                        display_name: row.display_name.clone(),
                        number: row.number.clone(),
                        label: row.label.clone(),
                        starred: row.starred,
                    })
                    .collect();
                return Ok(IpcResponse::Contacts { entries });
            }
            IpcRequest::Settings => return Err(IpcError::Internal),
        };

        // Commands act on the selected phone, and are validated against that
        // phone's mirror rather than a shared one.
        let phone_id = self.resolve_phone("");
        if phone_id.is_empty() {
            return Err(IpcError::PhoneOffline);
        }

        let output = self
            .app
            .lock()
            .expect("app mutex poisoned")
            .controller(&phone_id)
            .apply_user_command(command)
            .map_err(map_core_error)?;

        // Validating intent is only half the job: the request still has to reach
        // the phone, and the supervisor owns the socket.
        if let tandem_core::events::ControllerOutput::SendRequest(request) = output {
            self.bus_for(&phone_id)
                .ok_or(IpcError::PhoneOffline)?
                .send(request)
                .map_err(|()| IpcError::PhoneOffline)?;
        }

        Ok(IpcResponse::Ok)
    }
}

/// Core failures keep their meaning across the IPC boundary so the UI can show
/// the right guidance — the emergency refusal especially (ADR-0008).
fn map_core_error(error: CoreError) -> IpcError {
    match error {
        CoreError::CallNotFound(id) => IpcError::CallNotFound(id),
        CoreError::InvalidCallState { .. } | CoreError::EmergencyCallActive => {
            IpcError::InvalidCallState
        }
        CoreError::EmergencyBlocked { number } => IpcError::EmergencyBlocked { number },
        CoreError::StaleEpoch { .. } | CoreError::NotSynchronized => IpcError::PhoneOffline,
    }
}

fn call_view(call: &Call) -> CallView {
    CallView {
        call_id: call.call_id.clone(),
        state: call_state(call.state),
        remote_number: call.remote_number.clone(),
        remote_display_name: call.remote_display_name.clone(),
        started_at_ms: call.started_at_ms,
        is_conference: call.is_conference,
        can_hold: call.can_hold,
        can_merge: call.can_merge,
        is_emergency: call.is_emergency,
    }
}

fn call_state(state: CallState) -> IpcCallState {
    match state {
        CallState::Connecting => IpcCallState::Connecting,
        CallState::Dialing => IpcCallState::Dialing,
        CallState::Ringing => IpcCallState::Ringing,
        CallState::Active => IpcCallState::Active,
        CallState::Holding => IpcCallState::Holding,
        CallState::Disconnecting => IpcCallState::Disconnecting,
        CallState::Disconnected => IpcCallState::Disconnected,
    }
}

fn audio_route(route: AudioRoute) -> IpcAudioRoute {
    match route {
        AudioRoute::Earpiece => IpcAudioRoute::Earpiece,
        AudioRoute::Speaker => IpcAudioRoute::Speaker,
        AudioRoute::WiredHeadset => IpcAudioRoute::WiredHeadset,
        AudioRoute::Bluetooth => IpcAudioRoute::Bluetooth,
    }
}

/// Reported to the phone so its paired-devices list can label this machine.
fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn describe(state: &tandem_pairing::flow::PairingState) -> String {
    use tandem_pairing::flow::PairingState as S;
    match state {
        S::Scanned => "scanned".into(),
        S::Connecting => "connecting".into(),
        S::AwaitingConfirmation { .. } => "awaitingConfirmation".into(),
        S::Accepted(_) => "accepted".into(),
        S::Failed(error) => format!("failed: {error}"),
    }
}

fn describe_offer(state: &tandem_pairing::OfferState) -> String {
    use tandem_pairing::OfferState as S;
    match state {
        S::Waiting => "waitingForScan".into(),
        S::Retrying { reason } => format!("retrying: {reason}"),
        S::Connecting { phone_name } => format!("connecting:{phone_name}"),
        S::AwaitingLocalApproval(phone) => format!("approve:{}", phone.phone_name),
        S::AwaitingConfirmation => "awaitingConfirmation".into(),
        S::Accepted(_) => "accepted".into(),
        S::Failed(error) => format!("failed: {error}"),
    }
}

fn short_code_of(state: &tandem_pairing::flow::PairingState) -> Option<String> {
    match state {
        tandem_pairing::flow::PairingState::AwaitingConfirmation { short_code } => {
            short_code.as_ref().map(|c| c.as_str().to_string())
        }
        _ => None,
    }
}

fn domain_route(route: IpcAudioRoute) -> AudioRoute {
    match route {
        IpcAudioRoute::Earpiece => AudioRoute::Earpiece,
        IpcAudioRoute::Speaker => AudioRoute::Speaker,
        IpcAudioRoute::WiredHeadset => AudioRoute::WiredHeadset,
        IpcAudioRoute::Bluetooth => AudioRoute::Bluetooth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tandem_bluetooth::backends::BackendKind;

    /// The receiver has to stay alive for the length of the test: dropping it
    /// closes the queue, and an accepted command would then look like an offline
    /// phone.
    struct Harness {
        service: DaemonIpcService,
        commands: crate::session_loop::CommandReceiver,
    }

    fn service() -> Harness {
        service_with_link(live_link())
    }

    const TEST_PHONE: &str = "phone-under-test";

    fn service_with_link(link: LinkState) -> Harness {
        let mut app = App::build(Config {
            bluetooth_backend: BackendKind::Null,
            ..Config::default()
        });
        app.adopt_emergency_numbers(TEST_PHONE, vec!["911".into(), "112".into()]);
        app.store().add_phone(tandem_core::model::PairedPhone {
            device_id: TEST_PHONE.into(),
            name: "Pixel".into(),
            spki_sha256: "pin".into(),
            bt_address: String::new(),
        });

        let (bus, receiver) = crate::session_loop::CommandBus::new();
        let buses: CommandBuses = Default::default();
        buses
            .lock()
            .expect("bus mutex poisoned")
            .insert(TEST_PHONE.into(), bus);

        Harness {
            service: DaemonIpcService::new(
                std::sync::Arc::new(std::sync::Mutex::new(app)),
                std::sync::Arc::new(std::sync::Mutex::new(link)),
                buses,
            ),
            commands: receiver,
        }
    }

    /// The link state a single-phone test wants: one phone, selected, live.
    fn live_link() -> LinkState {
        let mut link = LinkState::default();
        link.ensure_selected(TEST_PHONE);
        let entry = link.entry(TEST_PHONE);
        entry.connection = Some(ConnectionStatus::Live);
        entry.phone_name = "Pixel".into();
        link
    }

    #[test]
    fn status_reports_media_availability_so_the_ui_can_explain_itself() {
        let mut h = service_with_link(live_link());
        let response = h.service.handle(IpcRequest::Status).unwrap();

        match response {
            IpcResponse::Status(status) => {
                assert_eq!(status.connection, ConnectionStatus::Live);
                assert_eq!(status.phone_name, "Pixel");
                assert!(!status.desktop_audio_available);
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    /// The refusal must survive translation with its number intact, since the UI
    /// copy names the number it refused.
    #[test]
    fn an_emergency_dial_is_refused_with_the_number_preserved() {
        let mut h = service();
        let err = h
            .service
            .handle(IpcRequest::Dial {
                number: "911".into(),
                sim_slot: -1,
            })
            .unwrap_err();
        assert_eq!(
            err,
            IpcError::EmergencyBlocked {
                number: "911".into()
            }
        );
        assert_eq!(err.code(), tandem_ipc::error::IPC_EMERGENCY_BLOCKED);
    }

    #[test]
    fn an_ordinary_dial_passes_through() {
        let mut h = service();
        assert_eq!(
            h.service
                .handle(IpcRequest::Dial {
                    number: "+14155550123".into(),
                    sim_slot: -1,
                })
                .unwrap(),
            IpcResponse::Ok
        );

        // Accepting the request is not enough: it has to be queued for the phone,
        // or the call would never be placed.
        assert_eq!(
            h.commands.try_recv().expect("the dial must be queued"),
            tandem_core::events::OutboundRequest::Dial {
                number: "+14155550123".into(),
                sim_slot: -1,
            }
        );
    }

    /// Routing to Bluetooth on a Tier B-lite build must fail with a specific
    /// reason rather than a generic error.
    #[test]
    fn bluetooth_routing_is_refused_when_the_build_has_no_audio_path() {
        let mut h = service();
        assert_eq!(
            h.service
                .handle(IpcRequest::AudioRoute {
                route: IpcAudioRoute::Bluetooth,
                    bt_device_address: "AA:BB".into(),
                })
                .unwrap_err(),
            IpcError::AudioUnavailable
        );
    }

    #[test]
    fn commands_against_an_unsynced_mirror_report_the_phone_offline() {
        let mut h = service();
        assert_eq!(
            h.service
                .handle(IpcRequest::Answer {
                    call_id: "c1".into()
                })
                .unwrap_err(),
            IpcError::PhoneOffline
        );
    }
}
