// A `frus run --machine`: its events become console text, and the IDE's buttons become its
// commands (reload, restart, a key, stop).

package dev.frus.intellij

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.process.KillableProcessHandler
import com.intellij.execution.process.ProcessOutputType
import com.intellij.execution.process.ProcessOutputTypes
import com.intellij.openapi.util.Key
import com.intellij.util.concurrency.AppExecutorUtil
import java.nio.charset.StandardCharsets
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

class FrusProcessHandler(commandLine: GeneralCommandLine) : KillableProcessHandler(commandLine) {
    private val stdout = LineBuffer()
    private val stderr = LineBuffer()
    private val nextId = AtomicInteger(1)

    /** Whether the application has started: a reload before then has nothing to reload. */
    @Volatile var started = false
        private set

    init {
        setShouldKillProcessSoftly(false)
    }

    override fun notifyTextAvailable(text: String, outputType: Key<*>) {
        when {
            outputType == ProcessOutputTypes.STDOUT -> stdout.take(text).forEach(::fromFrus)
            // The build's own output (cargo, cargo-apk) and the tool's diagnostics.
            outputType == ProcessOutputTypes.STDERR -> stderr.take(text).forEach { show(it, Shown.TOOL) }
            else -> super.notifyTextAvailable(text, outputType)
        }
    }

    private fun fromFrus(line: String) {
        val said = MachineOutput.translate(line)
        if (said.started) started = true
        said.text?.let { show(it, said.shown) }
    }

    /** Text in the console, as the application's or as the tool's. */
    fun show(text: String, shown: Shown) {
        val type = when (shown) {
            Shown.APP -> ProcessOutputTypes.STDOUT
            Shown.APP_ERROR, Shown.TOOL_ERROR -> ProcessOutputTypes.STDERR
            Shown.TOOL -> ProcessOutputType.SYSTEM
        }
        super.notifyTextAvailable(text + "\n", type)
    }

    /** Patches the change in (hot reload), or rebuilds and comes back on the same page. */
    fun reload() = restart(full = false)

    /** Every state afresh. */
    fun restart() = restart(full = true)

    private fun restart(full: Boolean) {
        if (isProcessTerminated || isProcessTerminating) return
        show(if (full) "Restarting…" else "Reloading…", Shown.TOOL)
        send("app.restart", mapOf("fullRestart" to full))
    }

    /** A key of `frus run`'s: what it does is reported in the log, as in the terminal. */
    fun key(key: String) {
        if (isProcessTerminated || isProcessTerminating || key.length != 1) return
        send("app.key", mapOf("key" to key))
    }

    private fun send(method: String, params: Map<String, Any?>?) {
        val input = processInput ?: return
        try {
            synchronized(this) {
                input.write(MachineOutput.command(nextId.getAndIncrement(), method, params).toByteArray(StandardCharsets.UTF_8))
                input.flush()
            }
        } catch (_: java.io.IOException) {
            // Ending already.
        }
    }

    /** Stop: frus stops the application and ends on its own; if it does not, it is ended. */
    override fun destroyProcessImpl() {
        send("app.stop", null)
        AppExecutorUtil.getAppScheduledExecutorService().schedule({
            if (!isProcessTerminated) super.destroyProcessImpl()
        }, 5, TimeUnit.SECONDS)
    }

    override fun notifyProcessTerminated(exitCode: Int) {
        stdout.rest()?.let(::fromFrus)
        stderr.rest()?.let { show(it, Shown.TOOL) }
        super.notifyProcessTerminated(exitCode)
    }
}
