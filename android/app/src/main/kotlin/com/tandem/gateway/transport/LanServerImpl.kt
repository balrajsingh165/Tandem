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
import com.tandem.gateway.proto.v1.CallLogChangedEvent
import com.tandem.gateway.proto.v1.CallStateChangedEvent
import com.tandem.gateway.proto.v1.Envelope
import com.tandem.gateway.proto.v1.ErrorCode
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
            val accepted = runCatching { socket.accept() as SSLSocket }.getOrNull() ?: continue
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
            withContext(Dispatchers.IO) { socket.startHandshake() }

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

            val hello = codec.decode(WebSocketFraming.readFrame(input).payload)
            if (!hello.hasSessionHello()) {
                socket.close()
                return
            }

            val paired = pairedDeviceRepository.byPinnedKey(fingerprint)
            if (paired == null || paired.revoked) {
                // An unknown key that got past TLS is a provisional pairing peer;
                // it is not a control session and must not receive call state.
                socket.close()
                return
            }

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

            pump(active, input)
        } catch (_: Exception) {
            // A failed session is a dropped desktop, never a gateway outage.
        } finally {
            session?.let { sessionRegistry.unregister(it.deviceId) }
            session?.close()
            runCatching { socket.close() }
        }
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

    override suspend fun claimCall(callId: String, deviceId: String): Boolean =
        sessionRegistry.claim(callId, deviceId)

    companion object {
        /** WebSocket path the desktop client dials. */
        const val WS_PATH: String = "/tlp/v1"
    }
}
