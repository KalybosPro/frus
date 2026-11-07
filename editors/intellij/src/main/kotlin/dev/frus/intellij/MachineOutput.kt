// What a line of `frus run --machine` shows in the Run console.

package dev.frus.intellij

/** How a piece of console text is shown: the application's output, its errors, or what the
 *  tool says (the build, the steps it takes). */
enum class Shown { APP, APP_ERROR, TOOL, TOOL_ERROR }

/** What one line of `frus run --machine` means for the console. */
data class Said(val text: String?, val shown: Shown, val stopped: Boolean = false, val started: Boolean = false)

object MachineOutput {
    /** The console text for a line of standard output: an event, or a line that is not JSON
     *  (shown as the tool's). `null` text for an event the console does not show. */
    fun translate(line: String): Said {
        val message = Json.parseOrNull(line) as? Map<*, *> ?: return Said(line, Shown.TOOL)
        val params = message.obj("params") ?: emptyMap<String, Any?>()
        return when (message.str("event")) {
            "app.log" -> Said(params.str("log").orEmpty(), if (params.bool("error")) Shown.APP_ERROR else Shown.APP)
            "app.progress" -> Said(params.str("message").orEmpty(), if (params.bool("error")) Shown.TOOL_ERROR else Shown.TOOL)
            "app.start" -> Said("Running on ${params.str("deviceName") ?: "the device"}…", Shown.TOOL)
            "app.started" -> Said("The application is running. Saving a Rust file reloads it.", Shown.TOOL, started = true)
            "app.restarted" -> {
                val what = if (params.bool("fullRestart")) "Restarted" else "Reloaded"
                val seconds = (params["seconds"] as? Number)?.toDouble()
                Said(if (seconds != null) "$what in ${"%.1f".format(java.util.Locale.ROOT, seconds)} s." else "$what.", Shown.TOOL)
            }
            "app.stop" -> Said(params.str("error"), Shown.TOOL_ERROR, stopped = true)
            else -> Said(null, Shown.TOOL)
        }
    }

    /** A command to `frus run --machine`, as the line to write. */
    fun command(id: Int, method: String, params: Map<String, Any?>? = null): String {
        val message = linkedMapOf<String, Any?>("id" to id, "method" to method)
        if (params != null) message["params"] = params
        return Json.write(message) + "\n"
    }
}

/**
 * How a run ended, for the last line of its console: every end is shown, with its exit code, and
 * an end that is not a success with the last error seen, so that a run never stops in silence.
 */
class RunEnding {
    private var lastError: String? = null
    private var stopSaid = false

    /** A piece of console text, as shown: an error is remembered. */
    fun saw(text: String?, shown: Shown, stopped: Boolean = false) {
        if (stopped) stopSaid = true
        if (!text.isNullOrBlank() && (shown == Shown.TOOL_ERROR || shown == Shown.APP_ERROR)) lastError = text.trim()
    }

    /** A line of the tool's standard error (the build, the tool's own diagnostics): one that
     *  reads as an error is remembered. */
    fun sawStderr(line: String) {
        val text = line.trim()
        if (ERROR_LINE.containsMatchIn(text)) lastError = text
    }

    /** The console's last line for an exit with `code`. */
    fun message(code: Int): Said {
        if (code == 0) {
            val how = if (stopSaid) "" else ", without saying it was stopping"
            return Said("frus run ended (exit code 0$how).", Shown.TOOL)
        }
        val why = lastError?.let { " The last error: $it" } ?: " It said no error: the lines above are all it printed."
        return Said("frus run failed (exit code $code).$why", Shown.TOOL_ERROR)
    }

    private companion object {
        /** `error: …`, `error[E0425]: …`, `Error: …`, a panic. */
        val ERROR_LINE = Regex("""^(error(\[\w+])?:|Error:|thread '.*' panicked)|\bpanicked at\b""")
    }
}

/** Splits text that arrives in pieces into whole lines. */
class LineBuffer {
    private val pending = StringBuilder()

    /** The whole lines `chunk` completes, without their ends. */
    fun take(chunk: String): List<String> {
        pending.append(chunk)
        val lines = mutableListOf<String>()
        while (true) {
            val at = pending.indexOf("\n")
            if (at < 0) break
            lines += pending.substring(0, at).removeSuffix("\r")
            pending.delete(0, at + 1)
        }
        return lines
    }

    /** What is left without an end, once the stream is over. */
    fun rest(): String? = pending.toString().takeIf { it.isNotEmpty() }.also { pending.setLength(0) }
}
