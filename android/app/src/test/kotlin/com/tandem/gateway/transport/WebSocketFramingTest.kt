/**
 * Unit tests for the RFC 6455 handshake and frame codec: upgrade parsing, the
 * Sec-WebSocket-Accept derivation, masked client frames, extended payload
 * lengths, and the protocol violations the gateway must refuse.
 */
package com.tandem.gateway.transport

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class WebSocketFramingTest {

    private fun maskedClientFrame(opcode: Int, payload: ByteArray): ByteArray {
        val mask = byteArrayOf(0x37, 0xFA.toByte(), 0x21, 0x3D)
        val out = ByteArrayOutputStream()
        out.write(0x80 or opcode)

        when {
            payload.size < 126 -> out.write(0x80 or payload.size)
            payload.size <= 0xFFFF -> {
                out.write(0x80 or 126)
                out.write(payload.size shr 8)
                out.write(payload.size and 0xFF)
            }
            else -> {
                out.write(0x80 or 127)
                for (shift in 56 downTo 0 step 8) {
                    out.write(((payload.size.toLong() shr shift) and 0xFF).toInt())
                }
            }
        }

        out.write(mask)
        payload.forEachIndexed { index, byte ->
            out.write((byte.toInt() xor mask[index % 4].toInt()) and 0xFF)
        }
        return out.toByteArray()
    }

    @Test
    fun `parses a well formed upgrade request`() {
        val request = buildString {
            append("GET /tlp/v1 HTTP/1.1\r\n")
            append("Host: phone.local\r\n")
            append("Upgrade: websocket\r\n")
            append("Connection: Upgrade\r\n")
            append("Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n")
            append("\r\n")
        }

        val parsed = WebSocketFraming.readUpgradeRequest(
            ByteArrayInputStream(request.toByteArray()),
        )

        assertEquals("/tlp/v1", parsed?.path)
        assertEquals("dGhlIHNhbXBsZSBub25jZQ==", parsed?.key)
    }

    @Test
    fun `rejects a request that is not a websocket upgrade`() {
        val plain = "GET /tlp/v1 HTTP/1.1\r\nHost: phone.local\r\n\r\n"
        assertNull(WebSocketFraming.readUpgradeRequest(ByteArrayInputStream(plain.toByteArray())))
    }

    /** The canonical example from RFC 6455 section 1.3. */
    @Test
    fun `derives the accept key from the rfc example`() {
        assertEquals(
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
            WebSocketFraming.acceptKey("dGhlIHNhbXBsZSBub25jZQ=="),
        )
    }

    @Test
    fun `round trips a binary payload`() {
        val payload = "envelope-bytes".toByteArray()
        val frame = WebSocketFraming.readFrame(
            ByteArrayInputStream(maskedClientFrame(WebSocketFraming.OPCODE_BINARY, payload)),
        )

        assertTrue(frame.isBinary)
        assertArrayEquals(payload, frame.payload)
    }

    @Test
    fun `handles the sixteen bit extended length`() {
        val payload = ByteArray(1000) { (it % 251).toByte() }
        val frame = WebSocketFraming.readFrame(
            ByteArrayInputStream(maskedClientFrame(WebSocketFraming.OPCODE_BINARY, payload)),
        )
        assertArrayEquals(payload, frame.payload)
    }

    @Test
    fun `handles the sixty four bit extended length`() {
        val payload = ByteArray(70_000) { (it % 251).toByte() }
        val frame = WebSocketFraming.readFrame(
            ByteArrayInputStream(maskedClientFrame(WebSocketFraming.OPCODE_BINARY, payload)),
        )
        assertArrayEquals(payload, frame.payload)
    }

    /** RFC 6455 section 5.1: an unmasked client frame must be refused. */
    @Test
    fun `refuses an unmasked client frame`() {
        val unmasked = byteArrayOf(0x82.toByte(), 0x02, 0x01, 0x02)
        assertThrows(ProtocolViolationException::class.java) {
            WebSocketFraming.readFrame(ByteArrayInputStream(unmasked))
        }
    }

    @Test
    fun `refuses a frame larger than the envelope cap`() {
        val out = ByteArrayOutputStream()
        out.write(0x80 or WebSocketFraming.OPCODE_BINARY)
        out.write(0x80 or 127)
        val oversized = (WebSocketFraming.MAX_PAYLOAD_BYTES + 1).toLong()
        for (shift in 56 downTo 0 step 8) {
            out.write(((oversized shr shift) and 0xFF).toInt())
        }

        assertThrows(ProtocolViolationException::class.java) {
            WebSocketFraming.readFrame(ByteArrayInputStream(out.toByteArray()))
        }
    }

    @Test
    fun `recognizes control frames`() {
        val close = WebSocketFraming.readFrame(
            ByteArrayInputStream(maskedClientFrame(WebSocketFraming.OPCODE_CLOSE, ByteArray(0))),
        )
        assertTrue(close.isClose)

        val ping = WebSocketFraming.readFrame(
            ByteArrayInputStream(maskedClientFrame(WebSocketFraming.OPCODE_PING, byteArrayOf(1))),
        )
        assertTrue(ping.isPing)
    }

    /** Server frames must never be masked, and must be readable back verbatim. */
    @Test
    fun `writes unmasked server frames`() {
        val out = ByteArrayOutputStream()
        WebSocketFraming.writeFrame(out, WebSocketFraming.OPCODE_BINARY, byteArrayOf(9, 8, 7))
        val written = out.toByteArray()

        assertEquals((0x80 or WebSocketFraming.OPCODE_BINARY).toByte(), written[0])
        assertEquals(3.toByte(), written[1])
        assertArrayEquals(byteArrayOf(9, 8, 7), written.copyOfRange(2, 5))
    }

    @Test
    fun `writes the sixteen bit length header for medium payloads`() {
        val out = ByteArrayOutputStream()
        WebSocketFraming.writeFrame(out, WebSocketFraming.OPCODE_BINARY, ByteArray(300))
        val written = out.toByteArray()

        assertEquals(126.toByte(), written[1])
        assertEquals(1.toByte(), written[2])
        assertEquals(44.toByte(), written[3])
    }
}
