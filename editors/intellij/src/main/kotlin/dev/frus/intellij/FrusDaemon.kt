// `frus daemon`, kept running for the IDE: the devices, as they come and go, the emulators that
// can be started or made, and what each application builds for.

package dev.frus.intellij

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.Disposable
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.Service
import com.intellij.openapi.diagnostic.logger
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.ide.ActivityTracker
import com.intellij.util.concurrency.AppExecutorUtil
import java.io.File
import java.io.OutputStream
import java.nio.charset.StandardCharsets
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

@Service(Service.Level.APP)
class FrusDaemon : Disposable {
    private val log = logger<FrusDaemon>()
    private val devices = ConcurrentHashMap<String, Device>()
    private val pending = ConcurrentHashMap<Int, CompletableFuture<Any?>>()
    private val nextId = AtomicInteger(1)
    private val changeListeners = CopyOnWriteArrayList<() -> Unit>()
    private val messageListeners = CopyOnWriteArrayList<(level: String, message: String) -> Unit>()
    private val awaited = CopyOnWriteArrayList<Pair<String, (Device) -> Unit>>()

    @Volatile private var process: Process? = null
    @Volatile private var input: OutputStream? = null
    @Volatile private var starting = false
    @Volatile private var watch: ScheduledFuture<*>? = null

    /** Why the daemon is not running, when it could not start. */
    @Volatile var problem: String? = null
        private set

    /** The emulators that can be started, as last asked. */
    @Volatile var emulators: List<Emulator> = emptyList()
        private set

    /** Why each platform is not one the application builds for, by application directory. */
    private val support = ConcurrentHashMap<String, Map<String, String>>()

    val isRunning: Boolean get() = process?.isAlive == true

    /** Starts the daemon unless it runs or is starting: finding (or installing) `frus` first, in
     *  the background. */
    fun ensureStarted() {
        if (isRunning || starting) return
        starting = true
        val task = object : Task.Backgroundable(null, "Starting frus", false) {
            override fun run(indicator: ProgressIndicator) {
                try {
                    start(FrusProgram.locate(indicator))
                    problem = null
                } catch (e: FrusException) {
                    problem = e.message
                    notify(NotificationType.ERROR, e.message ?: "frus did not start")
                } catch (e: Exception) {
                    problem = e.message
                    log.warn("frus daemon did not start", e)
                } finally {
                    starting = false
                    changed()
                }
            }
        }
        ProgressManager.getInstance().run(task)
    }

    private fun start(program: String) {
        val daemon = ProcessBuilder(program, "daemon")
            .redirectError(ProcessBuilder.Redirect.DISCARD)
            .start()
        process = daemon
        input = daemon.outputStream
        val reader = Thread({
            daemon.inputStream.bufferedReader(StandardCharsets.UTF_8).useLines { lines ->
                lines.forEach { line -> (Json.parseOrNull(line) as? Map<*, *>)?.let(::take) }
            }
            if (process === daemon) {
                process = null
                input = null
                devices.clear()
                changed()
            }
            pending.values.forEach { it.completeExceptionally(FrusException("frus daemon stopped")) }
            pending.clear()
        }, "frus daemon")
        reader.isDaemon = true
        reader.start()
        send(nextId.getAndIncrement(), "device.enable", null)
        refreshEmulators()
        watchInstall()
    }

    /** A new install of `frus` (Tools › frus › Update frus, or `cargo install` by hand): the
     *  daemon starts again from it, and runs use it from then on. */
    private fun watchInstall() {
        if (watch != null) return
        var seen = FrusProgram.installed?.let { it.lastModified() to it.length() }
        var settledAt = 0L
        watch = AppExecutorUtil.getAppScheduledExecutorService().scheduleWithFixedDelay({
            val file = FrusProgram.installed ?: return@scheduleWithFixedDelay
            if (!file.isFile) return@scheduleWithFixedDelay // Gone for a moment, mid-replacement.
            val now = file.lastModified() to file.length()
            if (now != seen) {
                seen = now
                settledAt = System.currentTimeMillis() + 2000
            } else if (settledAt != 0L && System.currentTimeMillis() >= settledAt) {
                settledAt = 0L
                log.info("frus: a new install of $file, restarting the daemon")
                restart()
            }
        }, 2, 2, TimeUnit.SECONDS)
    }

    /** Stops the daemon running and starts it again from a new `frus`. */
    fun restart() {
        stop()
        FrusProgram.forget()
        devices.clear()
        changed()
        ensureStarted()
    }

    private fun stop() {
        val daemon = process ?: return
        process = null
        runCatching { send(nextId.getAndIncrement(), "daemon.shutdown", null, daemon.outputStream) }
        AppExecutorUtil.getAppScheduledExecutorService().schedule({ if (daemon.isAlive) daemon.destroy() }, 2, TimeUnit.SECONDS)
        input = null
    }

    /** Asks the daemon, and gives its answer when it comes. */
    fun request(method: String, params: Map<String, Any?> = emptyMap()): CompletableFuture<Any?> {
        val future = CompletableFuture<Any?>()
        if (!isRunning) {
            future.completeExceptionally(FrusException(problem ?: "frus daemon is not running"))
            return future
        }
        val id = nextId.getAndIncrement()
        pending[id] = future
        try {
            send(id, method, params)
        } catch (e: Exception) {
            pending.remove(id)
            future.completeExceptionally(e)
        }
        return future
    }

    @Synchronized
    private fun send(id: Int, method: String, params: Map<String, Any?>?, to: OutputStream? = input) {
        val out = to ?: throw FrusException("frus daemon is not running")
        out.write(MachineOutput.command(id, method, params).toByteArray(StandardCharsets.UTF_8))
        out.flush()
    }

    /** A message of the daemon's. */
    internal fun take(message: Map<*, *>) {
        val id = (message["id"] as? Number)?.toInt()
        if (id != null && message["event"] == null) {
            val waiting = pending.remove(id) ?: return
            val error = message["error"]
            if (error != null) waiting.completeExceptionally(FrusException(error.toString())) else waiting.complete(message["result"])
            return
        }
        val params = message.obj("params") ?: emptyMap<String, Any?>()
        when (message.str("event")) {
            "daemon.logMessage", "daemon.showMessage" -> {
                val level = params.str("level") ?: "info"
                val text = params.str("message").orEmpty()
                log.info("frus daemon: $text")
                messageListeners.forEach { it(level, text) }
                if (message.str("event") == "daemon.showMessage") {
                    notify(
                        when (level) {
                            "error" -> NotificationType.ERROR
                            "warning" -> NotificationType.WARNING
                            else -> NotificationType.INFORMATION
                        },
                        text,
                    )
                }
            }
            "device.added" -> Device.from(params)?.let {
                devices[it.id] = it
                arrived(it)
                changed()
                refreshEmulators()
            }
            "device.removed" -> params.str("id")?.let {
                devices.remove(it)
                changed()
                refreshEmulators()
            }
        }
    }

    /** An emulator started from the IDE is selected once it is up: by its name, which a running
     *  emulator takes from its AVD. */
    fun awaitEmulator(name: String, up: (Device) -> Unit) {
        awaited += name to up
    }

    private fun arrived(device: Device) {
        if (!device.isRunningEmulator || !device.available) return
        for (entry in awaited) {
            if (entry.first == device.name || entry.first.replace('_', ' ') == device.name) {
                awaited.remove(entry)
                ApplicationManager.getApplication().invokeLater { entry.second(device) }
            }
        }
    }

    private fun refreshEmulators() {
        request("emulator.getEmulators").thenAccept {
            emulators = Emulator.from(it)
            changed()
        }
    }

    /** Starts an emulator; it arrives as a device once it is up. */
    fun launchEmulator(id: String): CompletableFuture<Any?> =
        request("emulator.launch", mapOf("emulatorId" to id, "coldBoot" to false))

    /** Makes a new Android emulator, installing what it needs: its name, once made. */
    fun createEmulator(name: String?): CompletableFuture<String> =
        request("emulator.create", if (name.isNullOrBlank()) emptyMap() else mapOf("name" to name)).thenApply {
            val result = it as? Map<*, *> ?: throw FrusException("no answer")
            if (!result.bool("success")) throw FrusException(result.str("error") ?: "the emulator was not created")
            result.str("emulatorName") ?: throw FrusException("the emulator has no name")
        }

    /** Why each platform is not one the application in `root` builds for, as last asked; asks
     *  again in the background. */
    fun unsupported(root: File): Map<String, String> {
        val key = root.path
        request("daemon.getSupportedPlatforms", mapOf("projectRoot" to key)).thenAccept {
            val now = unsupportedReasons(it)
            if (support.put(key, now) != now) changed()
        }
        return support[key].orEmpty()
    }

    /** The devices now, the desktop first, then phones, then browsers. */
    fun list(): List<Device> = Device.sorted(devices.values)

    fun get(id: String): Device? = devices[id]

    fun onChange(parent: Disposable, listener: () -> Unit) {
        changeListeners += listener
        com.intellij.openapi.util.Disposer.register(parent) { changeListeners -= listener }
    }

    fun onMessage(listener: (level: String, message: String) -> Unit): () -> Unit {
        messageListeners += listener
        return { messageListeners -= listener }
    }

    private fun changed() {
        changeListeners.forEach { it() }
        // The toolbar's device selector shows the change without waiting for the next update.
        ActivityTracker.getInstance().inc()
    }

    override fun dispose() {
        watch?.cancel(false)
        stop()
    }

    companion object {
        fun get(): FrusDaemon = ApplicationManager.getApplication().getService(FrusDaemon::class.java)

        fun notify(type: NotificationType, message: String, project: com.intellij.openapi.project.Project? = null) {
            NotificationGroupManager.getInstance().getNotificationGroup("frus")
                .createNotification("frus", message, type)
                .notify(project)
        }
    }
}
