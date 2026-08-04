/**
 * Use-case: merge two calls into a conference via TelecomBridge, honoring
 * can_merge. Maps telecom conference semantics onto the single is_conference flag
 * desktops render.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.domain.port.TelecomError
import javax.inject.Inject

class MergeCalls @Inject constructor(
    private val telecomBridge: TelecomBridge,
) {
    /**
     * An empty [otherCallId] means "the single held call", which is what the
     * desktop UI offers when exactly one other call exists.
     */
    suspend operator fun invoke(
        callId: String,
        otherCallId: String,
        snapshot: CallSnapshot?,
    ): Result<Unit> {
        val call = snapshot?.call(callId)
        if (call != null && !call.canMerge) {
            return Result.failure(TelecomError.InvalidCallState(callId, "merge"))
        }

        val resolved = otherCallId.ifEmpty {
            val others = snapshot?.calls.orEmpty().filter { it.callId != callId && !it.isTerminal }
            if (others.size != 1) {
                return Result.failure(TelecomError.InvalidCallState(callId, "merge"))
            }
            others.first().callId
        }
        return telecomBridge.merge(callId, resolved)
    }
}
