/**
 * RFC 6455 handshake and frame codec for the gateway's WebSocket endpoint,
 * written against raw streams so the TLS socket can be created from an
 * SSLContext backed by non-exportable Android Keystore keys (ADR-0006).
 * Pure byte manipulation; no I/O policy and no protocol semantics.
 */
package com.tandem.gateway.transport

import java.io.EOFException
import java.io.InputStream
import java.io.OutputStream
import java.security.MessageDigest

object WebSocketFraming {

    /** Magic GUID from RFC 6455 §1.3, concatenated before hashing. */
    private const val ACCEPT_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

    const val OPCODE_BINARY = 0x2
    const val OPCODE_CLOSE = 0x8
    const val OPCODE_PING = 0x9
    const val OPCODE_PONG = 0xA

    /** Frames larger than this are a protocol violation, not a fragmentation hint. */
    const val MAX_PAYLOAD_BYTES = 256 * 1024

    /** Parsed upgrade request; [key] is echoed back hashed in the response. */
    data class UpgradeRequest(val path: String, val key: String)

    /**
     * Reads the HTTP upgrade request. Returns null when the request is not a
     * well-formed WebSocket upgrade, so the caller can close without guessing.
     */
    fun readUpgradeRequest(input: InputStream): UpgradeRequest? {
        val requestLine = readLine(input) ?: return null
        val parts = requestLine.split(' ')
        if (parts.size < 2 || !parts[0].equals("GET", ignoreCase = true)) return null
        val path = parts[1]

        val headers = mutableMapOf<String, String>()
        while (true) {
            val line = readLine(input) ?: return null
            if (line.isEmpty()) break
            val separator = line.indexOf(':')
            if (separator <= 0) continue
            headers[line.substring(0, separator).trim().lowercase()] =
                line.substring(separator + 1).trim()
        }

        if (!headers["upgrade"].equals("websocket", ignoreCase = true)) return null
        val key = headers["sec-websocket-key"] ?: return null
        return UpgradeRequest(path = path, key = key)
    }

    /** Sec-WebSocket-Accept per RFC 6455: base64(sha1(key + GUID)). */
    fun acceptKey(clientKey: String): String {
        val digest = MessageDigest.getInstance("SHA-1")
            .digest((clientKey + ACCEPT_GUID).toByteArray(Charsets.US_ASCII))
        return android.util.Base64.encodeToString(digest, android.util.Base64.NO_WRAP)
    }

    fun writeUpgradeResponse(output: OutputStream, clientKey: String) {
        val response = buildString {
            append("HTTP/1.1 101 Switching Protocols\r\n")
            append("Upgrade: websocket\r\n")
            append("Connection: Upgrade\r\n")
            append("Sec-WebSocket-Accept: ${acceptKey(clientKey)}\r\n")
            append("\r\n")
        }
        output.write(response.toByteArray(Charsets.US_ASCII))
        output.flush()
    }

    fun writeRejectResponse(output: OutputStream) {
        output.write("HTTP/1.1 400 Bad Request\r\n\r\n".toByteArray(Charsets.US_ASCII))
        output.flush()
    }

    /** One decoded frame. Control frames carry their payload verbatim. */
    data class Frame(val opcode: Int, val payload: ByteArray) {
        val isBinary: Boolean get() = opcode == OPCODE_BINARY
        val isClose: Boolean get() = opcode == OPCODE_CLOSE
        val isPing: Boolean get() = opcode == OPCODE_PING

        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is Frame) return false
            return opcode == other.opcode && payload.contentEquals(other.payload)
        }

        override fun hashCode(): Int = 31 * opcode + payload.contentHashCode()
    }

    /**
     * Reads one frame. Client frames must be masked per RFC 6455 §5.1; an
     * unmasked client frame is a protocol violation and is rejected rather than
     * tolerated.
     */
    fun readFrame(input: InputStream): Frame {
        val first = input.readOrThrow()
        val opcode = first and 0x0F

        val second = input.readOrThrow()
        val masked = (second and 0x80) != 0
        if (!masked) throw ProtocolViolationException("client frame was not masked")

        var length = (second and 0x7F).toLong()
        when (length) {
            126L -> {
                length = 0
                repeat(2) { length = (length shl 8) or input.readOrThrow().toLong() }
            }

            127L -> {
                length = 0
                repeat(8) { length = (length shl 8) or input.readOrThrow().toLong() }
            }
        }

        if (length > MAX_PAYLOAD_BYTES) {
            throw ProtocolViolationException("frame of $length bytes exceeds $MAX_PAYLOAD_BYTES")
        }

        val mask = ByteArray(4) { input.readOrThrow().toByte() }
        val payload = ByteArray(length.toInt())
        var read = 0
        while (read < payload.size) {
            val count = input.read(payload, read, payload.size - read)
            if (count < 0) throw EOFException("stream ended mid-frame")
            read += count
        }
        for (i in payload.indices) {
            payload[i] = (payload[i].toInt() xor mask[i % 4].toInt()).toByte()
        }

        return Frame(opcode, payload)
    }

    /** Writes a server frame. Server frames are never masked (RFC 6455 §5.1). */
    fun writeFrame(output: OutputStream, opcode: Int, payload: ByteArray) {
        val header = ArrayList<Byte>(10)
        header += (0x80 or opcode).toByte()

        when {
            payload.size < 126 -> header += payload.size.toByte()
            payload.size <= 0xFFFF -> {
                header += 126.toByte()
                header += (payload.size shr 8).toByte()
                header += payload.size.toByte()
            }

            else -> {
                header += 127.toByte()
                for (shift in 56 downTo 0 step 8) {
                    header += (payload.size.toLong() shr shift).toByte()
                }
            }
        }

        output.write(header.toByteArray())
        output.write(payload)
        output.flush()
    }

    private fun InputStream.readOrThrow(): Int {
        val value = read()
        if (value < 0) throw EOFException("stream ended")
        return value
    }

    private fun readLine(input: InputStream): String? {
        val builder = StringBuilder()
        while (true) {
            val value = input.read()
            if (value < 0) return if (builder.isEmpty()) null else builder.toString()
            if (value == '\n'.code) return builder.toString().removeSuffix("\r")
            builder.append(value.toChar())
        }
    }
}

class ProtocolViolationException(message: String) : Exception(message)
