/**
 * ViewModel behind DialpadScreen: holds the dial string, offers recent numbers that
 * narrow as the user types, and places the call through PlaceCall so the emergency
 * policy and dialer-role checks apply.
 */
package com.tandem.gateway.ui.dialpad

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.domain.model.CallLogEntry
import com.tandem.gateway.domain.port.CallLogRepository
import com.tandem.gateway.domain.usecase.PlaceCall
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

@HiltViewModel
class DialpadViewModel @Inject constructor(
    private val placeCall: PlaceCall,
    private val callLogRepository: CallLogRepository,
) : ViewModel() {

    private val _dialString = MutableStateFlow("")
    val dialString: StateFlow<String> = _dialString.asStateFlow()

    private val _failure = MutableStateFlow<String?>(null)
    val failure: StateFlow<String?> = _failure.asStateFlow()

    private val recents = MutableStateFlow<List<CallLogEntry>>(emptyList())

    /**
     * Shown above the keypad so someone can redial without opening a second
     * screen, and narrowed as they type against both digits and saved names.
     */
    val suggestions: StateFlow<List<CallLogEntry>> =
        combine(recents, _dialString) { history, typed ->
            val matching = if (typed.isEmpty()) {
                history
            } else {
                val digits = typed.filter(Char::isDigit)
                history.filter { entry ->
                    (digits.isNotEmpty() &&
                        entry.number.filter(Char::isDigit).contains(digits)) ||
                        entry.displayName.contains(typed, ignoreCase = true)
                }
            }
            matching.distinctBy { it.number }.take(MAX_SUGGESTIONS)
        }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), emptyList())

    init {
        viewModelScope.launch {
            recents.value = callLogRepository
                .page(0, RECENTS_FOR_SUGGESTIONS, 0)
                .getOrNull()
                ?.entries
                .orEmpty()
        }
    }

    fun setInitial(number: String) {
        if (_dialString.value.isEmpty()) _dialString.value = number
    }

    fun append(digit: String) {
        _dialString.value += digit
        _failure.value = null
    }

    fun backspace() {
        _dialString.value = _dialString.value.dropLast(1)
    }

    fun choose(number: String) {
        _dialString.value = number
        _failure.value = null
    }

    /**
     * fromDesktop is false here: this is the handset, the sanctioned path for
     * emergency calls, so the guard must not block it (ADR-0008).
     */
    fun call() = viewModelScope.launch {
        val number = _dialString.value
        if (number.isEmpty()) return@launch

        placeCall(number, simSlot = -1, fromDesktop = false)
            .onSuccess { _dialString.value = "" }
            .onFailure { cause -> _failure.value = cause.message }
    }

    fun callNow(number: String) = viewModelScope.launch {
        placeCall(number, simSlot = -1, fromDesktop = false)
            .onFailure { cause -> _failure.value = cause.message }
    }

    private companion object {
        const val MAX_SUGGESTIONS = 4
        const val RECENTS_FOR_SUGGESTIONS = 60
    }
}
