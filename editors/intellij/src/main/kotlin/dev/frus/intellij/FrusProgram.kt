// The `frus` program: the one the settings name, or the one on the PATH — installed with
// `cargo install frus-cli` when there is none, rather than asking the developer to.
//
// What the plugin runs is a copy of it, so that the installed one is never in use: Windows
// refuses to replace a running program, and the daemon kept open for the IDE would make every
// `cargo install frus-cli` fail with "access denied".

package dev.frus.intellij

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.configurations.PathEnvironmentVariableUtil
import com.intellij.execution.util.ExecUtil
import com.intellij.openapi.application.PathManager
import com.intellij.openapi.diagnostic.logger
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.util.SystemInfo
import java.io.File
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.StandardCopyOption

class FrusException(message: String) : Exception(message)

object FrusProgram {
    private val LOG = logger<FrusProgram>()

    @Volatile
    private var ready: String? = null

    /** The installed program the copy was made from, watched for a new install. */
    @Volatile
    var installed: File? = null
        private set

    /** The program to run, ready now: `null` until [locate] has found it. */
    fun readyOrNull(): String? = ready

    /** Found again next time: the settings changed, or a new install landed. */
    fun forget() {
        ready = null
    }

    /** The program to run, found — and installed first if it has to be. Slow: never on the UI
     *  thread. */
    @Synchronized
    fun locate(indicator: ProgressIndicator?): String {
        ready?.let { return it }
        val configured = FrusSettings.get().path
        val program = configured.ifEmpty { "frus" }
        var file = resolve(program)
        if (file == null) {
            if (configured.isNotEmpty()) throw FrusException("The frus settings name $configured, which does not run.")
            indicator?.text = "Installing frus (cargo install frus-cli, a few minutes the first time)…"
            val cargo = resolve("cargo")
                ?: throw FrusException("frus is not installed, and neither is Rust: install Rust from https://rustup.rs, then try again.")
            val out = ExecUtil.execAndGetOutput(
                GeneralCommandLine(cargo.path, "install", "frus-cli", "--locked").withCharset(StandardCharsets.UTF_8),
            )
            if (out.exitCode != 0) throw FrusException("cargo install frus-cli failed:\n${out.stderr.takeLast(2000)}")
            file = resolve(program) ?: resolve(File(System.getProperty("user.home"), ".cargo/bin/frus").path)
                ?: throw FrusException("frus-cli was installed, but frus is not on the PATH (is ~/.cargo/bin there?).")
        }
        val help = ExecUtil.execAndGetOutput(GeneralCommandLine(file.path, "help", "run").withCharset(StandardCharsets.UTF_8))
        if (help.exitCode != 0 || !help.stdout.contains("--machine")) {
            throw FrusException(
                "$file is older than the editor protocol (frus daemon, frus run --machine): " +
                    "update it with Tools › frus › Update frus, or name a newer one in the frus settings.",
            )
        }
        installed = file
        val copy = try {
            runnableCopy(file, File(PathManager.getSystemPath(), "frus/bin"))
        } catch (e: Exception) {
            LOG.warn("frus: no copy of $file, running it in place", e)
            file
        }
        ready = copy.path
        return copy.path
    }

    /** The file `program` runs: itself when it is a path, else the first match on the PATH (and
     *  in ~/.cargo/bin, which an IDE started from a desktop launcher may not have on its PATH). */
    fun resolve(program: String): File? {
        val extensions = if (SystemInfo.isWindows) listOf("", ".exe", ".cmd", ".bat") else listOf("")
        if (program.contains('/') || program.contains('\\')) {
            return extensions.map { File(program + it) }.firstOrNull { it.isFile }
        }
        PathEnvironmentVariableUtil.findInPath(program)?.let { return it }
        if (SystemInfo.isWindows) {
            for (ext in extensions.drop(1)) PathEnvironmentVariableUtil.findInPath(program + ext)?.let { return it }
        }
        val cargoBin = File(System.getenv("CARGO_HOME") ?: File(System.getProperty("user.home"), ".cargo").path, "bin")
        return extensions.map { File(cargoBin, program + it) }.firstOrNull { it.isFile }
    }

    /** A copy of `program` in `dir` to run instead of it, made if it is not there yet. Named after
     *  the original's size and time, so a new install gets a new copy; older copies are removed
     *  when nothing runs them any more. */
    fun runnableCopy(program: File, dir: File): File {
        val extension = program.name.substringAfterLast('.', "").let { if (it.isEmpty() || it == program.name) "" else ".$it" }
        val name = "frus-${program.lastModified()}-${program.length()}$extension"
        val copy = File(dir, name)
        dir.mkdirs()
        if (!copy.isFile) {
            // Written under another name first: a copy cut short must never be taken for a whole one.
            val partial = File(dir, "$name.${ProcessHandle.current().pid()}.part")
            Files.copy(program.toPath(), partial.toPath(), StandardCopyOption.REPLACE_EXISTING)
            partial.setExecutable(true)
            Files.move(partial.toPath(), copy.toPath(), StandardCopyOption.ATOMIC_MOVE)
        }
        dir.listFiles()?.filter { it.name != name && it.name.startsWith("frus-") }?.forEach {
            // Still running, in another IDE: it goes next time.
            it.delete()
        }
        return copy
    }
}
