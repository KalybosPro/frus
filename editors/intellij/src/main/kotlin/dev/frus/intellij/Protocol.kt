// What `frus daemon` says and what a frus application is: the plugin's rules, apart from the IDE
// so that they are tested on their own.

package dev.frus.intellij

import java.io.File

/** A device as `frus daemon` describes it. */
data class Device(
    val id: String,
    val name: String,
    val platform: String,
    /** `desktop`, `mobile` or `web`. */
    val category: String,
    val detail: String,
    val emulator: Boolean,
    val available: Boolean,
    val unavailableReason: String?,
) {
    /** A phone or a tablet simulated in a window of this computer (`frus run -d iphone-15`): the
     *  daemon gives it this computer's platform and category, and an id of its own, where this
     *  computer's id is its platform. */
    val isSimulated: Boolean get() = category == "desktop" && id != platform

    /** An Android emulator that is not running: the daemon lists it among the devices, but it is
     *  started from the emulators, not picked. */
    val isStoppedEmulator: Boolean get() = emulator && platform == "android" && !id.startsWith("emulator-")

    /** A running Android emulator: its id is adb's, `emulator-5554`. */
    val isRunningEmulator: Boolean get() = id.startsWith("emulator-")

    companion object {
        /** The device a `device.added` event's params describe. */
        fun from(params: Map<*, *>): Device? {
            val id = params.str("id") ?: return null
            return Device(
                id = id,
                name = params.str("name") ?: id,
                platform = params.str("platform") ?: "",
                category = params.str("category") ?: "desktop",
                detail = params.str("detail") ?: "",
                emulator = params.bool("emulator"),
                available = params["available"] as? Boolean ?: true,
                unavailableReason = params.str("unavailableReason"),
            )
        }

        private val ORDER = mapOf("desktop" to 0, "mobile" to 1, "web" to 2)

        /** The devices in the picker's order: this computer first, then phones, then browsers,
         *  then the phones and tablets simulated in a window, as `frus devices` lists them. */
        fun sorted(devices: Collection<Device>): List<Device> =
            devices.sortedWith(compareBy<Device>({ if (it.isSimulated) 4 else ORDER[it.category] ?: 3 }, { it.name.lowercase() }))

        /**
         * The device a run goes to when none was chosen: a phone plugged in, then a running
         * emulator, then this computer, then a browser. Never a simulated device or an emulator
         * that is not running: those are run on only when chosen.
         */
        fun preferred(devices: Collection<Device>): Device? {
            val usable = sorted(devices).filter { it.available && !it.isSimulated && !it.isStoppedEmulator }
            return usable.firstOrNull { it.category == "mobile" && !it.emulator }
                ?: usable.firstOrNull { it.isRunningEmulator }
                ?: usable.firstOrNull { it.category == "desktop" }
                ?: usable.firstOrNull()
        }
    }
}

/** An emulator that can be started, as `emulator.getEmulators` lists it. */
data class Emulator(val id: String, val name: String, val platformType: String) {
    companion object {
        fun from(value: Any?): List<Emulator> = (value as? List<*>).orEmpty().mapNotNull {
            val m = it as? Map<*, *> ?: return@mapNotNull null
            val id = m.str("id") ?: return@mapNotNull null
            Emulator(id, m.str("name") ?: id, m.str("platformType") ?: "android")
        }
    }
}

/** Why a platform is not one the application builds for, by platform, from
 *  `daemon.getSupportedPlatforms`; platforms it builds for are not there. */
fun unsupportedReasons(result: Any?): Map<String, String> {
    val types = (result as? Map<*, *>)?.obj("platformTypes") ?: return emptyMap()
    val reasons = HashMap<String, String>()
    for ((platform, support) in types) {
        val s = support as? Map<*, *> ?: continue
        if (s.bool("isSupported")) continue
        val why = ((s["reasons"] as? List<*>)?.firstOrNull() as? Map<*, *>)?.str("reasonText") ?: "not set up"
        reasons[platform.toString()] = why
    }
    return reasons
}

/** When saving a Rust file reloads the running application. */
enum class ReloadOnSave(val label: String) {
    NEVER("Never"),
    MANUAL("When a Rust file is saved by hand (Ctrl+S), not by auto save"),
    MANUAL_IF_DIRTY("When a Rust file with changes is saved by hand"),
    ALL("On every save of a Rust file, auto save included");

    /** Whether a save, by hand or not and of a changed file or not, reloads. */
    fun reloads(manual: Boolean, dirty: Boolean): Boolean = when (this) {
        NEVER -> false
        MANUAL -> manual
        MANUAL_IF_DIRTY -> manual && dirty
        ALL -> true
    }
}

/** Whether `frus run` patches each change into the running application. On is the default,
 *  passed as `--hot`, so that hot reload is on with every frus, those whose own default is off
 *  included. */
enum class HotMode(val label: String, val hint: String) {
    ON(
        "On (--hot)",
        "Each saved change is patched into the running application, every state kept (desktop, web, Android). " +
            "A release build runs without it.",
    ),
    OFF(
        "Off (--no-hot)",
        "A reload rebuilds and restarts the application, which comes back on the same page with the states it keeps.",
    ),
    FRUS(
        "Let frus decide (no flag)",
        "Nothing passed: frus newer than 0.5.0 hot reloads a debug build, frus 0.5.0 and older do not.",
    ),
    ;

    companion object {
        /** A mode saved by name; an unknown name (an older plugin's `DEFAULT`) is on. */
        fun named(name: String?): HotMode = entries.find { it.name == name } ?: ON
    }
}

/**
 * The arguments of the `frus run --machine` an editor drives. `knowsNoHot`: whether this frus
 * takes `--no-hot` (frus 0.5.0 does not, and refuses to run; off is its default anyway).
 */
fun runArguments(
    deviceId: String?,
    release: Boolean,
    hot: HotMode,
    appArgs: List<String>,
    knowsNoHot: Boolean = true,
): List<String> {
    val args = mutableListOf("run", "--machine")
    if (!deviceId.isNullOrBlank()) args += listOf("-d", deviceId)
    if (release) args += "--release"
    when (hot) {
        HotMode.ON -> if (!release) args += "--hot"
        HotMode.OFF -> if (knowsNoHot) args += "--no-hot"
        HotMode.FRUS -> {}
    }
    if (appArgs.isNotEmpty()) args += listOf("--") + appArgs
    return args
}

/** A `Cargo.toml` of a frus application: a package that depends on frus. */
fun isApplicationManifest(text: String): Boolean =
    Regex("""^\s*\[package]""", RegexOption.MULTILINE).containsMatchIn(text) &&
        Regex("""^\s*frus\s*(=|\.)""", RegexOption.MULTILINE).containsMatchIn(text)

/** Whether `dir` holds a frus application. */
fun isApplication(dir: File): Boolean {
    val manifest = File(dir, "Cargo.toml")
    return manifest.isFile && isApplicationManifest(runCatching { manifest.readText() }.getOrDefault(""))
}

/** The frus applications under `root`, nearest first, skipping build output; at most `limit`. */
fun findApplications(root: File, depth: Int = 4, limit: Int = 50): List<File> {
    val found = mutableListOf<File>()
    val skip = setOf("target", "build", "node_modules", ".git", ".idea", ".gradle", ".claude")
    var level = listOf(root)
    for (d in 0..depth) {
        val next = mutableListOf<File>()
        for (dir in level) {
            if (isApplication(dir)) found += dir
            if (found.size >= limit) return found
            dir.listFiles()?.filter { it.isDirectory && it.name !in skip && !it.name.startsWith(".") }
                ?.sortedBy { it.name }?.let(next::addAll)
        }
        level = next
    }
    return found
}

/** The organisation a new application's Android package starts with, unless told. */
const val DEFAULT_ORG = "com.example"

/** Why `name` cannot be a new application's name, or `null` when it can: what `frus create`
 *  accepts (lowercase letters, digits, `_` and `-`), starting with a letter, and not a folder
 *  that is already there with something in it. */
fun nameProblem(name: String, parent: File?): String? {
    if (name.isEmpty()) return "A name, please: it is the folder's and the package's."
    if (name[0] !in 'a'..'z') return "Start with a lowercase letter."
    if (!Regex("^[a-z][a-z0-9_-]*$").matches(name)) return "Lowercase letters, digits, _ and - only, as a Rust package name."
    if (parent != null) {
        val target = File(parent, name)
        if (target.list()?.isNotEmpty() == true) return "$target is already there, and not empty."
    }
    return null
}

/** Why `org` cannot start an Android package name, or `null`: dot-separated parts, each a letter
 *  then letters, digits or `_`. */
fun orgProblem(org: String): String? =
    if (Regex("^[A-Za-z][A-Za-z0-9_]*(\\.[A-Za-z][A-Za-z0-9_]*)+$").matches(org)) {
        null
    } else {
        "A reverse domain such as com.acme: parts separated by dots, each starting with a letter."
    }

/** The arguments of the `frus create` that makes `name` in `parent`. */
fun createArguments(parent: File, name: String, org: String): List<String> {
    val args = mutableListOf("create", File(parent, name).path)
    if (org.isNotEmpty() && org != DEFAULT_ORG) args += listOf("--org", org)
    return args
}
