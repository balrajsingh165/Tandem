/**
 * ViewModel for DialpadScreen: dial-string editing and PlaceCall dispatch. Note
 * the emergency guard applies only to desktop-originated dials; handset dials
 * pass through.
 */
package com.tandem.gateway.ui.dialpad

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.domain.usecase.PlaceCall
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@HiltViewModel
class DialpadViewModel @Inject constructor(
    private val placeCall: PlaceCall,
) : ViewModel() {

    private val _dialString = MutableStateFlow("")
    val dialString: StateFlow<String> = _dialString.asStateFlow()

    private val _failure = MutableStateFlow<String?>(null)
    val failure: StateFlow<String?> = _failure.asStateFlow()

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
}
