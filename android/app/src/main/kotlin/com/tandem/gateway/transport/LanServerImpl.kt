/**
 * LanServer implementation: Ktor (CIO) WebSocket endpoint over mutual TLS 1.3
 * built by TlsServerFactory, accepting paired desktops (pinned SPKI) and
 * provisional pairing sessions. Delegates frames to DesktopSession; owns nothing
 * about message semantics.
 */
package com.tandem.gateway.transport

import com.tandem.gateway.crypto.Fingerprints
import com.tandem.gateway.crypto.TlsServerFactory
import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.CallLogRepository
import com.tandem.gateway.domain.port.EmergencyNumberSource
import com.tandem.gateway.domain.port.IdentityStore
import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.ServerStatus
import com.tandem.gateway.domain.port.SessionInfo
import com.tandem.gateway.domain.port.TransportError
import com.tandem.gateway.domain.usecase.ObserveCallState
import com.tandem.gateway.domain.usecase.PairDesktop
import com.tandem.gateway.pairing.CandidacyOutcome
import com.tandem.gateway.pairing.PairingCandidate
import com.tandem.gateway.pairing.PairingManagerImpl
import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.port.CallMediaProvider
import com.tandem.gateway.proto.v1.AudioDevice
import com.tandem.gateway.proto.v1.AudioDevicesEvent
import com.tandem.gateway.proto.v1.CallLogChangedEvent
import com.tandem.gateway.proto.v1.CallStateChangedEvent
import com.tandem.gateway.proto.v1.Envelope
import com.tandem.gateway.proto.v1.ErrorCode
import com.tandem.gateway.proto.v1.PairingAwaitConfirmEvent
import com.tandem.gateway.proto.v1.PairingDecision
import com.tandem.gateway.proto.v1.SessionWelcome
import java.net.ServerSocket
import java.security.cert.X509Certificate
import javax.inject.Inject
import javax.inject.Singleton
import javax.net.ssl.SSLServerSocket
import javax.net.ssl.SSLSocket
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Singleton
class LanServerImpl @Inject constructor(
    private val sessionRegistry: SessionRegistry,
    private val codec: EnvelopeCodec,
    private val nsdAdvertiser: NsdAdvertiser,
    private val tlsServerFactory: TlsServerFactory,
    private val pairedDeviceRepository: PairedDeviceRepository,
    private val identityStore: IdentityStore,
    private val controlPlaneRouter: ControlPlaneRouter,
    private val pairingManager: PairingManagerImpl,
    private val pairDesktop: PairDesktop,
    private val telecomBridge: com.tandem.gateway.telecom.TelecomBridgeImpl,
    private val callMediaProvider: CallMediaProvider,
    private val observeCallState: ObserveCallState,
    private val callLogRepository: CallLogRepository,
    private val emergencyNumberSource: EmergencyNumberSource,
    private val scope: CoroutineScope,
) : LanServer {

    private val _status =
        MutableStateFlow(ServerStatus(listening = false, port = 0, advertisedName = ""))
    override val status: StateFlow<ServerStatus> = _status.asStateFlow()

    override val connectedSessions: Flow<List<SessionInfo>> = sessionRegistry.connected

    private var serverSocket: ServerSocket? = null
    private var acceptJob: Job? = null

    /** The snapshot last broadcast, replayed to each new session after welcome. */
    @Volatile
    private var latestSnapshot: CallSnapshot? = null

    override suspend fun start(port: Int): Result<Unit> = runCatching {
        stop()

        val socket = withContext(Dispatchers.IO) {
            tlsServerFactory.createServerSocket(port)
        }
        serverSocket = socket
        _status.value = ServerStatus(
            listening = true,
            port = socket.localPort,
            advertisedName = NsdAdvertiser.SERVICE_NAME,
        )

        acceptJob = scope.launch(Dispatchers.IO) { acceptLoop(socket) }
    }.recoverCatching { throw TransportError.BindFailed(port) }

    override suspend fun stop() {
        acceptJob?.let { runCatching { it.cancel() } }
        acceptJob = null
        runCatching { serverSocket?.close() }
        serverSocket = null
        nsdAdvertiser.unregister()
        _status.value = _status.value.copy(listening = false)
    }

    private suspend fun acceptLoop(socket: ServerSocket) {
        while (scope.isActive && !socket.isClosed) {
            val accepted = runCatching { socket.accept() as SSLSocket }
                .onFailure { android.util.Log.w(TAG, "accept failed", it) }
                .getOrNull() ?: continue
            scope.launch(Dispatchers.IO) { serve(accepted) }
        }
    }

    /**
     * One connection: TLS is already authenticated by the pinning trust manager,
     * so the peer certificate identifies the desktop without a second challenge.
     */
    private suspend fun serve(socket: SSLSocket) {
        var session: DesktopSession? = null
        try {
            socket.useClientMode = false
            socket.needClientAuth = true
            android.util.Log.i(TAG, "handshaking with ${socket.inetAddress}")
            withContext(Dispatchers.IO) { socket.startHandshake() }
            android.util.Log.i(TAG, "handshake ok with ${socket.inetAddress}")

            val peerCert = socket.session.peerCertificates.firstOrNull() as? X509Certificate
                ?: return
            val fingerprint = Fingerprints.toBase64Url(
                Fingerprints.spkiSha256(peerCert.publicKey.encoded),
            )

            val input = socket.inputStream
            val output = socket.outputStream

            val upgrade = WebSocketFraming.readUpgradeRequest(input)
            if (upgrade == null || upgrade.path != WS_PATH) {
                WebSocketFraming.writeRejectResponse(output)
                socket.close()
                return
            }
            WebSocketFraming.writeUpgradeResponse(output, upgrade.key)

            val first = codec.decode(WebSocketFraming.readFrame(input).payload)

            // An unknown key that got past TLS can only be a provisional pairing
            // peer, and pairing never yields a control session on this connection.
            if (first.hasPairingRequest()) {
                servePairing(first, peerCert, fingerprint, input, output)
                socket.close()
                return
            }

            if (!first.hasSessionHello()) {
                socket.close()
                return
            }

            val paired = pairedDeviceRepository.byPinnedKey(fingerprint)
            if (paired == null || paired.revoked) {
                socket.close()
                return
            }
            val hello = first

            val active = DesktopSession(
                deviceId = paired.deviceId,
                displayName = paired.name,
                btAdapterAddress = hello.sessionHello.btAdapterAddress,
                connectedAtMs = System.currentTimeMillis(),
                socket = socket,
                input = input,
                output = output,
                codec = codec,
            )
            session = active

            active.send(welcomeFor(hello.messageId))
            sessionRegistry.register(active)
            pairedDeviceRepository.recordSeen(paired.deviceId, System.currentTimeMillis())

            if (hello.sessionHello.btAdapterAddress.isNotEmpty()) {
                pairedDeviceRepository.setBluetoothAddress(
                    paired.deviceId,
                    hello.sessionHello.btAdapterAddress,
                )
            }

            latestSnapshot?.let { active.send(snapshotEnvelope(it)) }
            active.send(audioDevicesEnvelope())

            pump(active, input)
        } catch (error: Exception) {
            // A failed session is a dropped desktop, never a gateway outage —
            // but it is logged, or a pairing that never lands is undiagnosable.
            android.util.Log.w(TAG, "session from ${socket.inetAddress} failed", error)
        } finally {
            session?.let { sessionRegistry.unregister(it.deviceId) }
            session?.close()
            runCatching { socket.close() }
        }
    }

    /**
     * Runs one pairing candidacy on a provisional connection: bind the presented
     * certificate to the TLS layer, consume the one-time token, ask the user, and
     * report the verdict. Nothing is persisted unless the user allows it.
     */
    private suspend fun servePairing(
        envelope: Envelope,
        peerCert: X509Certificate,
        fingerprint: String,
        input: java.io.InputStream,
        output: java.io.OutputStream,
    ) {
        val request = envelope.pairingRequest

        // The certificate in the payload must be the one that completed the TLS
        // handshake, or an attacker could pair someone else's key (docs/07).
        val bound = request.desktopCertDer.toByteArray().contentEquals(peerCert.encoded)
        if (!bound) {
            writeDecision(output, envelope.messageId, ErrorCode.ERROR_CODE_PAIRING_REJECTED, null)
            return
        }

        val candidate = PairingCandidate(
            desktopName = request.desktopName,
            desktopPlatform = request.desktopPlatform,
            certDer = request.desktopCertDer.toByteArray(),
            spkiSha256 = fingerprint,
            protocolMin = request.protocolMin,
            protocolMax = request.protocolMax,
        )

        val presented = pairingManager.presentCandidate(
            token = request.pairingToken,
            presented = candidate,
            shortCode = null,
        )
        val outcome = presented.getOrElse {
            writeDecision(output, envelope.messageId, ErrorCode.ERROR_CODE_PAIRING_REJECTED, null)
            return
        }

        // Either way the desktop is told the token was accepted; what differs is
        // who answers next.
        writeFrame(
            output,
            Envelope.newBuilder()
                .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
                .setPairingAwaitConfirmEvent(
                    PairingAwaitConfirmEvent.newBuilder().setRequireShortCode(false).build(),
                )
                .build(),
        )

        val desktop = when (outcome) {
            // The scan was the consent on this phone. The remaining question
            // belongs to the person at the computer, and their answer arrives here
            // rather than on this screen.
            CandidacyOutcome.NeedsDesktopApproval -> {
                val approval = readApproval(input)
                if (approval != true) {
                    pairDesktop.submitDecision(accept = false)
                    null
                } else {
                    pairDesktop.submitDecision(accept = true).getOrNull()
                }
            }

            CandidacyOutcome.NeedsUserConfirmation -> pairingManager.awaitVerdict().getOrNull()
        }

        if (desktop == null) {
            writeDecision(output, envelope.messageId, ErrorCode.ERROR_CODE_PAIRING_REJECTED, null)
            return
        }

        // PairDesktop persists on the user's acceptance; the server only reports
        // the verdict, so the row has exactly one writer.
        writeDecision(output, envelope.messageId, ErrorCode.ERROR_CODE_OK, desktop.deviceId)
    }

    /**
     * Waits for the computer's verdict on a scanned pairing. Anything other than a
     * PairingApproval — including a dropped connection — counts as no, so a
     * pairing is never committed on silence.
     */
    private fun readApproval(input: java.io.InputStream): Boolean? = runCatching {
        val envelope = codec.decode(WebSocketFraming.readFrame(input).payload)
        if (envelope.hasPairingApproval()) envelope.pairingApproval.accept else null
    }.getOrNull()

    private suspend fun writeDecision(
        output: java.io.OutputStream,
        inReplyTo: Long,
        code: ErrorCode,
        assignedDeviceId: String?,
    ) {
        val identity = identityStore.identity().getOrNull()
        val decision = PairingDecision.newBuilder()
            .setStatus(codec.status(code))
            .setDesktopDeviceId(assignedDeviceId.orEmpty())
            .setPhoneDeviceId(identity?.deviceId.orEmpty())
            .setPhoneName(identity?.displayName.orEmpty())
            .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
            .build()

        writeFrame(
            output,
            Envelope.newBuilder()
                .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
                .setInReplyTo(inReplyTo)
                .setPairingDecision(decision)
                .build(),
        )
    }

    private fun writeFrame(output: java.io.OutputStream, envelope: Envelope) {
        WebSocketFraming.writeFrame(
            output,
            WebSocketFraming.OPCODE_BINARY,
            codec.encode(envelope),
        )
    }

    private suspend fun pump(session: DesktopSession, input: java.io.InputStream) {
        while (!session.closed) {
            val frame = withContext(Dispatchers.IO) {
                runCatching { WebSocketFraming.readFrame(input) }.getOrNull()
            } ?: return

            when {
                frame.isClose -> return
                frame.isPing -> session.pong(frame.payload)
                frame.isBinary -> {
                    val envelope = runCatching { codec.decode(frame.payload) }.getOrNull() ?: return
                    val reply = controlPlaneRouter.handle(
                        envelope = envelope,
                        session = session,
                        snapshot = latestSnapshot,
                        nowMs = System.currentTimeMillis(),
                    )
                    session.send(reply)
                }
            }
        }
    }

    private suspend fun welcomeFor(inReplyTo: Long): Envelope {
        val identity = identityStore.identity().getOrNull()
        return Envelope.newBuilder()
            .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
            .setInReplyTo(inReplyTo)
            .setSessionWelcome(
                SessionWelcome.newBuilder()
                    .setStatus(codec.status(ErrorCode.ERROR_CODE_OK))
                    .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
                    .setPhoneDeviceId(identity?.deviceId.orEmpty())
                    .setPhoneName(identity?.displayName.orEmpty())
                    .setEpochId(observeCallState.currentEpochId())
                    .setStateSeq(observeCallState.currentStateSeq())
                    .setCallLogVersion(callLogRepository.currentVersion())
                    // Arms the desktop's local emergency pre-check for this
                    // session's SIM and region (ADR-0008).
                    .addAllEmergencyNumbers(emergencyNumberSource.currentEmergencyNumbers())
                    .build(),
            )
            .build()
    }

    private fun snapshotEnvelope(snapshot: CallSnapshot): Envelope =
        Envelope.newBuilder()
            .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
            .setCallStateChangedEvent(
                CallStateChangedEvent.newBuilder().setSnapshot(codec.toProto(snapshot)).build(),
            )
            .build()

    override suspend fun broadcastSnapshot(snapshot: CallSnapshot) {
        latestSnapshot = snapshot
        sessionRegistry.broadcast(codec.encode(snapshotEnvelope(snapshot)))
    }

    /**
     * Tells every desktop which audio targets this phone can actually use. Only
     * routes the OS reports as supported are listed, and Bluetooth is expanded
     * into one entry per connected device so the desktop can name them.
     */
    override suspend fun broadcastAudioDevices() {
        sessionRegistry.broadcast(codec.encode(audioDevicesEnvelope()))
    }

    private suspend fun audioDevicesEnvelope(): Envelope {
        val supported = telecomBridge.supportedRoutes.value
        val builder = AudioDevicesEvent.newBuilder()
            .setActiveRoute(codec.toProto(telecomBridge.audioRoute.value))
            .setActiveBtDeviceAddress(telecomBridge.btRouteAddress.value)

        for (route in listOf(AudioRoute.EARPIECE, AudioRoute.SPEAKER, AudioRoute.WIRED_HEADSET)) {
            if (route in supported) {
                builder.addDevices(
                    AudioDevice.newBuilder()
                        .setRoute(codec.toProto(route))
                        .setName(route.label())
                        .build(),
                )
            }
        }

        if (AudioRoute.BLUETOOTH in supported) {
            for (target in callMediaProvider.availableBluetoothTargets()) {
                builder.addDevices(
                    AudioDevice.newBuilder()
                        .setRoute(codec.toProto(AudioRoute.BLUETOOTH))
                        .setBtDeviceAddress(target.address)
                        .setName(target.name)
                        .build(),
                )
            }
        }

        return Envelope.newBuilder()
            .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
            .setAudioDevicesEvent(builder.build())
            .build()
    }

    override suspend fun broadcastCallLogChanged(logVersion: Long) {
        val envelope = Envelope.newBuilder()
            .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
            .setCallLogChangedEvent(
                CallLogChangedEvent.newBuilder().setLogVersion(logVersion).build(),
            )
            .build()
        sessionRegistry.broadcast(codec.encode(envelope))
    }

    override suspend fun revokeSession(deviceId: String, reason: String) {
        sessionRegistry.close(deviceId, reason)
    }

    companion object {
        /** WebSocket path the desktop client dials. */
        const val WS_PATH: String = "/tlp/v1"

        private const val TAG: String = "TandemLan"
    }
}
