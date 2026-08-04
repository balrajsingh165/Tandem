/**
 * State machine for one pairing candidacy: TokenPresented, AwaitingConfirm (with
 * optional short-code comparison), Accepted, Rejected, Expired. Emits the
 * PairingAwaitConfirmEvent and PairingDecision payloads.
 */
package com.tandem.gateway.pairing

import com.tandem.gateway.domain.port.PairingError

/**
 * One candidate's lifecycle. A token is consumed by the first request that
 * presents it, success or failure, so a leaked code cannot be replayed.
 */
class PairingSession(
    private val token: String,
    private val expiresAtMs: Long,
    private val requireShortCode: Boolean,
) {
    var phase: Phase = Phase.WAITING
        private set

    private var tokenConsumed = false

    fun presentToken(candidateToken: String, nowMs: Long): Result<Unit> {
        if (nowMs > expiresAtMs) {
            phase = Phase.EXPIRED
            return Result.failure(PairingError.TokenExpired)
        }
        if (tokenConsumed) {
            return Result.failure(PairingError.TokenAlreadyUsed)
        }
        tokenConsumed = true
        if (candidateToken != token) {
            phase = Phase.FAILED
            return Result.failure(PairingError.TokenMismatch)
        }
        phase = Phase.AWAITING_CONFIRMATION
        return Result.success(Unit)
    }

    fun requiresShortCode(): Boolean = requireShortCode

    fun accept(): Result<Unit> {
        if (phase != Phase.AWAITING_CONFIRMATION) {
            return Result.failure(PairingError.WindowBusy)
        }
        phase = Phase.ACCEPTED
        return Result.success(Unit)
    }

    fun reject(): Result<Unit> {
        phase = Phase.REJECTED
        return Result.failure(PairingError.RejectedByUser)
    }

    fun expire() {
        if (phase == Phase.WAITING || phase == Phase.AWAITING_CONFIRMATION) {
            phase = Phase.EXPIRED
        }
    }

    enum class Phase {
        WAITING,
        AWAITING_CONFIRMATION,
        ACCEPTED,
        REJECTED,
        EXPIRED,
        FAILED,
    }
}
