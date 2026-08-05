/**
 * CameraX ImageAnalysis.Analyzer that decodes QR codes from the preview stream
 * with ZXing, reading the luminance plane directly so no bitmap is allocated per
 * frame. Reports the first successful decode once and then ignores the rest.
 */
package com.tandem.gateway.pairing

import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader
import java.util.concurrent.atomic.AtomicBoolean

class QrImageAnalyzer(private val onDecoded: (String) -> Unit) : ImageAnalysis.Analyzer {

    private val reader = QRCodeReader()
    private val hints = mapOf(
        DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE),
        DecodeHintType.TRY_HARDER to true,
    )
    private val done = AtomicBoolean(false)

    override fun analyze(image: ImageProxy) {
        if (done.get()) {
            image.close()
            return
        }
        try {
            decode(image)?.let { text ->
                if (done.compareAndSet(false, true)) onDecoded(text)
            }
        } catch (_: Exception) {
            // A frame that does not contain a readable code is the common case.
        } finally {
            reader.reset()
            image.close()
        }
    }

    private fun decode(image: ImageProxy): String? {
        val plane = image.planes.firstOrNull() ?: return null
        val buffer = plane.buffer
        val bytes = ByteArray(buffer.remaining())
        buffer.get(bytes)

        val source = PlanarYUVLuminanceSource(
            bytes,
            plane.rowStride,
            image.height,
            0,
            0,
            minOf(plane.rowStride, image.width),
            image.height,
            false,
        )
        return reader.decode(BinaryBitmap(HybridBinarizer(source)), hints).text
    }
}
